use crate::client::Client;
use crate::common::{deserialize_handshake, HANDSHAKE_HEADER};
use crate::common;
use crate::db;
use crate::packet::parse_ipv4_header;
use crate::webserver::WebServer;
use anyhow::Result;
use lazy_static::lazy_static;
use libp2p::{*, futures::AsyncReadExt, futures::AsyncWriteExt, futures::StreamExt, multiaddr::Protocol};
use libp2p::swarm::SwarmEvent;
use libp2p_stream::{self as stream};
use std::collections::HashMap;
use std::net::Ipv4Addr;
use std::time::Duration;
use std::sync::Arc;

const RETICULA_PROTOCOL: StreamProtocol = StreamProtocol::new("/reticula");

lazy_static! {
    static ref streams: tokio::sync::Mutex<HashMap<PeerId, Arc<tokio::sync::Mutex<libp2p::swarm::Stream>>>> = tokio::sync::Mutex::new(HashMap::new());
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
    while let Some((peer, stream)) = incoming_streams.next().await {
        log::info!("Client connected {peer:?}");

        //let db = Arc::clone(&db);
        let stream = Arc::new(tokio::sync::Mutex::new(stream));
        {
            streams.lock().await.insert(peer, stream.clone());
        }
        let control = Arc::clone(&control);
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

                            let current_clients = clients.lock().await;
                            for client in current_clients.iter() {
                                if *client.0 == peer {
                                    continue;
                                }
                                let s = control.lock().await.open_stream(*client.0, RETICULA_PROTOCOL).await;
                                let _ = s.unwrap().write_all(&buf[..amt]).await;
                            }
                        }
                    }
                    Err(_) => {
                        log::info!("Unknown error");
                        drop(locked_stream);
                        continue;
                    }
                }
            }
        });
    }
}

async fn remove_client(peer: PeerId) {
    let mut current_clients = clients.lock().await;
    current_clients.remove(&peer);
    let mut current_streams = streams.lock().await;
    current_streams.remove(&peer);
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
                log::info!("Connection established {peer_id:?} {addr:?}");

                let mut current_clients = clients.lock().await;
                current_clients.insert(peer_id, Client {
                    addr: addr.clone(),
                    peer: peer_id,
                    sdn_ip_addr: Ipv4Addr::new(0, 0, 0, 0),
                });
            }
            SwarmEvent::ConnectionClosed { peer_id, connection_id, endpoint, num_established, cause } => {
                log::info!("Connection closed {peer_id:?} {connection_id:?} {endpoint:?} {num_established:?} {cause:?}");
                remove_client(peer_id).await;
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
    let identity = common::load_identity();
    let mut swarm = SwarmBuilder::with_existing_identity(identity)
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
