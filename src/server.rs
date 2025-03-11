use crate::client::Client;
use crate::common::{handshake_deserialize, HANDSHAKE_HEADER, HANDSHAKE_SIZE};
use crate::db;
use crate::packet::parse_ipv4_header;
use crate::webserver::WebServer;
use anyhow::Result;
use libp2p::{*, futures::AsyncReadExt, futures::AsyncWriteExt, futures::StreamExt};
use libp2p::swarm::SwarmEvent;
use libp2p_stream as stream;
use std::{collections::HashSet, time::Duration};
use std::sync::Arc;

const RETICULA_PROTOCOL: StreamProtocol = StreamProtocol::new("/reticula");

async fn handle_server(db: Arc<db::Db>, mut incoming_streams: libp2p_stream::IncomingStreams) {
    while let Some((peer, mut stream)) = incoming_streams.next().await {
        log::info!("Client connected {peer:?}");

        let mut clients: HashSet<Client> = HashSet::new();
        let db = Arc::clone(&db);
        tokio::spawn(async move {
            let mut buf: [u8; 1500] = [0; 1500];
            loop {
                match stream.read(&mut buf).await {
                    Ok(amt) => {
                        log::info!("Received {} bytes", amt);
                        if amt == HANDSHAKE_SIZE && buf[..3] == HANDSHAKE_HEADER {
                            let handshake = handshake_deserialize(&buf);
                            if db.check_client(&peer.to_string(), &handshake.ipv4_addr.to_string()).await {
                                clients.insert(Client {
                                    peer,
                                    ipv4_addr: handshake.ipv4_addr,
                                });
                                log::info!("Client connected {} {}", peer, handshake.ipv4_addr);
                            } else {
                                log::error!(
                                    "Ignoring unknown user pair {} {}",
                                    peer,
                                    handshake.ipv4_addr
                                );
                            }
                            continue;
                        }
                        else if let Some(header) = parse_ipv4_header(&buf[..amt]) {
                            log::info!(
                                "{} {} {}",
                                header.src_ip,
                                header.dst_ip,
                                header.total_length
                            );
                            for client in &clients {
                                log::info!("Sending data to {}", peer);
                                if peer != client.peer {
                                    let _ = stream.write_all(&buf[..amt]).await;
                                }
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

pub async fn server_swarm_loop(mut swarm: Swarm<stream::Behaviour>) {
    loop {
        match swarm.select_next_some().await {
            SwarmEvent::NewListenAddr { address, .. } => {
                let listen_address = address.with_p2p(*swarm.local_peer_id()).unwrap();
                log::info!("Listening on {listen_address:?}")
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
    let incoming_streams = swarm
        .behaviour()
        .new_control()
        .accept(RETICULA_PROTOCOL)
        .unwrap();
    let server = tokio::spawn(handle_server(db.clone(), incoming_streams));
    let server_swarm = tokio::spawn(server_swarm_loop(swarm));
    tokio::select! {
        _ = server => log::info!("Server exited"),
        _ = server_swarm => log::info!("Server swarm exited"),
        _ = web_server => log::info!("Web server exited"),
    }
    Ok(())
}
