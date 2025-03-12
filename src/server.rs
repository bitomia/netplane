use crate::client::Client;
use crate::common::{deserialize_handshake, HANDSHAKE_HEADER};
use crate::common;
use crate::db;
use crate::packet::parse_ipv4_header;
use crate::webserver::WebServer;
use anyhow::Result;
use autonat::v2::client;
use lazy_static::lazy_static;
use libp2p::{*, futures::AsyncReadExt, futures::AsyncWriteExt, futures::StreamExt, multiaddr::Protocol};
use libp2p::swarm::SwarmEvent;
use libp2p_stream::{self as stream, OpenStreamError};
use std::borrow::Borrow;
use std::fmt::Debug;
use std::collections::HashMap;
use std::net::Ipv4Addr;
use std::{collections::HashSet, time::Duration};
use std::sync::Arc;
use std::sync::Mutex;

const RETICULA_PROTOCOL: StreamProtocol = StreamProtocol::new("/reticula");

lazy_static! {
    static ref streams: tokio::sync::Mutex<Vec<Arc<tokio::sync::Mutex<libp2p::swarm::Stream>>>> = tokio::sync::Mutex::new(Vec::new());
    static ref clients: tokio::sync::Mutex<HashMap<PeerId, Client>> = {
        let m = HashMap::new();
        tokio::sync::Mutex::new(m)
    };
}

fn extract_ipv4(multiaddr: &Multiaddr) -> Option<std::net::Ipv4Addr> {
    for protocol in multiaddr.iter() {
        if let Protocol::Ip4(ipv4) = protocol {
            return Some(ipv4);
        }
    }
    None
}

async fn handle_server(db: Arc<db::Db>, control: Arc<tokio::sync::Mutex<libp2p_stream::Control>>, mut incoming_streams: libp2p_stream::IncomingStreams) {
    while let Some((peer, mut stream)) = incoming_streams.next().await {
        log::info!("Client connected {peer:?}");

        let db = Arc::clone(&db);
        let stream = Arc::new(tokio::sync::Mutex::new(stream));
        streams.lock().await.push(stream.clone());

        tokio::spawn(async move {
            let mut buf: [u8; 1500] = [0; 1500];
            loop {
                let mut locked_stream = stream.lock().await;
                match locked_stream.read(&mut buf).await {
                    Ok(amt) => {
                        drop(locked_stream);
                        
                        log::info!("Received {} bytes", amt);
                        if buf[..3] == HANDSHAKE_HEADER {
                            let handshake = deserialize_handshake(&buf);
                            let base64_identity = common::identity_to_base64(&handshake.identity);
                            log::info!("Received handshake from {peer} with identity {base64_identity}");
                            //if db.check_client(&base64_identity, &handshake.sdn_ip_addr.to_string()).await
                            {
                                log::info!("Client connected {}", peer);
                                let mut current_clients = clients.lock().await;
                                let client_addr = current_clients.get(&peer).unwrap().addr.clone();
                                current_clients.insert(peer, Client {
                                    peer,
                                    sdn_ip_addr: handshake.sdn_ip_addr,
                                    addr: client_addr,
                                });
                            } 
                            //else {
                            //    log::error!("Ignoring unknown user pair {}", peer);
                            //    break;
                            //}
                        } else if let Some(header) = parse_ipv4_header(&buf[..amt]) {
                            log::info!(
                                "{} {} {}",
                                header.src_ip,
                                header.dst_ip,
                                header.total_length
                            );
                            for stream in streams.lock().await.iter() {
                                println!("relay 1");
                                stream.lock().await.write_all(&buf[..amt]).await.unwrap();
                                println!("relay 2");
                            }
                        }
                    }
                    Err(_) => {
                        continue;
                    }
                }
            }
        });
    }
}

async fn read_header(mut stream: libp2p::Stream) -> std::io::Result<Vec<u8>> {
    let mut buffer = vec![0; 1500];
    let n = stream.read(&mut buffer).await?;
    buffer.truncate(n); // Resize buffer to actual read size
    Ok(buffer)
}

pub async fn server_swarm_loop(db: Arc<db::Db>, mut swarm: Swarm<stream::Behaviour>) {
    loop {
        match swarm.select_next_some().await {
            SwarmEvent::NewListenAddr { address, .. } => {
                let listen_address = address.with_p2p(*swarm.local_peer_id()).unwrap();
                log::info!("Listening on {listen_address:?}")
            }
            SwarmEvent::ConnectionEstablished { peer_id, endpoint, .. } => {
                let addr = endpoint.get_remote_address();
                let mut current_clients = clients.lock().await;
                current_clients.insert(peer_id, Client {
                    addr: addr.clone(),
                    peer: peer_id,
                    sdn_ip_addr: Ipv4Addr::new(0, 0, 0, 0),
                });
            }
            SwarmEvent::Behaviour(event) => log::info!("{event:?}"),
            _ => {}
        }
    }
}

pub async fn start() -> Result<()> {
    let db = Arc::new(db::Db::new().await);
    log::info!("Starting reticula server");
    let web_server = WebServer::new("127.0.0.1:3000", db.clone()).await;
    let mut swarm = SwarmBuilder::with_new_identity()
        .with_tokio()
        .with_quic()
        .with_behaviour(|_| stream::Behaviour::new())?
        .with_swarm_config(|cfg| cfg.with_idle_connection_timeout(Duration::from_secs(10)))
        .build();
    swarm
        .listen_on("/ip4/127.0.0.1/udp/5000/quic-v1".parse()?)
        .unwrap();
    let mut control = swarm
        .behaviour()
        .new_control();
    let incoming_streams = control
        .accept(RETICULA_PROTOCOL)
        .unwrap();
    let server = tokio::spawn(handle_server(db.clone(), Arc::new(tokio::sync::Mutex::new(control)), incoming_streams));
    let server_swarm = tokio::spawn(server_swarm_loop(db.clone(), swarm));
    tokio::select! {
        _ = server => log::info!("Server exited"),
        _ = server_swarm => log::info!("Server swarm exited"),
        _ = web_server => log::info!("Web server exited"),
    }
    Ok(())
}
