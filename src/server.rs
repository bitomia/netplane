use crate::client::Client;
use crate::common::{handshake_deserialize, HANDSHAKE_HEADER, HANDSHAKE_SIZE};
use crate::db;
use crate::packet::parse_ipv4_header;
use anyhow::Result;
use libp2p::{*, futures::AsyncReadExt, futures::AsyncWriteExt, futures::StreamExt};
use libp2p::swarm::SwarmEvent;
use libp2p_stream as stream;
use std::net::Ipv4Addr;
use std::{collections::HashSet, time::Duration};

const RETICULA_PROTOCOL: StreamProtocol = StreamProtocol::new("/reticula");

async fn handle_server(mut incoming_streams: libp2p_stream::IncomingStreams) {
    while let Some((peer, mut stream)) = incoming_streams.next().await {
        log::info!("Client connected {peer:?}");

        let mut clients: HashSet<Client> = HashSet::new();
        // TODO check peer registered
        clients.insert(Client {
            peer,
            ipv4_addr: Ipv4Addr::new(0, 0, 0, 0),
        });

        tokio::spawn(async move {
            let mut buf: [u8; 1500] = [0; 1500];
            loop {
                match stream.read(&mut buf).await {
                    Ok(amt) => {
                        log::info!("Received {} bytes", amt);
                        //if amt == HANDSHAKE_SIZE && buf[..3] == HANDSHAKE_HEADER {
                        //    let handshake = handshake_deserialize(&buf);
                        //    if db
                        //        .check_client(&src.ip().to_string(), &handshake.ipv4_addr.to_string())
                        //    .await
                        //    == true
                        //    {
                        //        clients.insert(Client {
                        //            src,
                        //            ipv4_addr: handshake.ipv4_addr,
                        //        });
                        //        log::info!("Client connected {} {}", src, handshake.ipv4_addr);
                        //    } else {
                        //        log::error!(
                        //            "Ignoring Unknown user pair {} {}",
                        //            src.ip(),
                        //            handshake.ipv4_addr
                        //        );
                        //    }
                        //    continue;
                        //}
                        if let Some(header) = parse_ipv4_header(&buf[..amt]) {
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
pub async fn start() -> Result<()> {
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

    tokio::spawn(handle_server(incoming_streams));
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
