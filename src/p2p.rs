use crate::common::{handshake_deserialize, HANDSHAKE_HEADER, HANDSHAKE_SIZE};
use crate::db;
use crate::packet::parse_ipv4_header;
use crate::tundev;
use anyhow::{Context, Result};
use futures::{AsyncReadExt, AsyncWriteExt, StreamExt};
use libp2p::{multiaddr::Protocol, swarm::SwarmEvent, *};
use libp2p_stream as stream;
use rand::RngCore;
use std::any::Any;
use std::net::{Ipv4Addr, SocketAddr, UdpSocket};
use std::{collections::HashSet, error::Error, io::*, time::Duration};

const RETICULA_PROTOCOL: StreamProtocol = StreamProtocol::new("/reticula");

#[derive(Eq, Hash, PartialEq)]
struct Client {
    pub src: SocketAddr,
    pub ipv4_addr: Ipv4Addr,
}

async fn send(mut stream: Stream) -> std::io::Result<()> {
    let num_bytes: usize = 10;

    let mut bytes = vec![0; num_bytes];
    rand::thread_rng().fill_bytes(&mut bytes);

    stream.write_all(&bytes).await?;

    let mut buf = vec![0; num_bytes];
    stream.read_exact(&mut buf).await?;
    if bytes != buf {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "incorrect echo",
        ));
    }

    stream.close().await?;
    log::info!("SENT!");
    Ok(())
}
async fn echo(mut stream: Stream) -> std::io::Result<usize> {
    let mut total: usize = 0;
    let mut buf: [u8; 100] = [0u8; 100];

    loop {
        let read: usize = stream.read(&mut buf).await?;
        if read == 0 {
            return Ok(total);
        }

        total += read;
        stream.write_all(&buf[..read]).await?;
    }
}

pub async fn p2p_server() -> Result<()> {
    let mut swarm = SwarmBuilder::with_new_identity()
        .with_tokio()
        .with_quic()
        .with_behaviour(|_| stream::Behaviour::new())?
        .with_swarm_config(|cfg| cfg.with_idle_connection_timeout(Duration::from_secs(10)))
        .build();

    swarm
        .listen_on("/ip4/127.0.0.1/udp/5000/quic-v1".parse()?)
        .unwrap();
    let mut incoming_streams = swarm
        .behaviour()
        .new_control()
        .accept(RETICULA_PROTOCOL)
        .unwrap();
    let mut clients: HashSet<Client> = HashSet::new();

    while let Some((peer, mut stream)) = incoming_streams.next().await {
        log::info!("Client connected {peer:?}");
        // TODO check peer registered
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
                            log::debug!(
                                "{} {} {}",
                                header.src_ip,
                                header.dst_ip,
                                header.total_length
                            );

                            for client in &clients {
                                log::debug!("Sending data to {}", peer);
                                //                            if src != client.src
                                {
                                    // TODO this is broadcasting, parse IP header and send only to target
                                    log::debug!("...relying");
                                    //    socket
                                    //        .send_to(&buf[..amt], &client.src)
                                    //        .expect("Cannot send");
                                    //
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
    });
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

pub async fn p2p_client(
    tun_name: &str,
    destination: &str,
    netmask: &str,
    ip_addr: &str,
    addr: &str,
) -> Result<()> {
    let dev = tundev::TunDev::new(tun_name, netmask, destination, ip_addr);
    let mut swarm = SwarmBuilder::with_new_identity()
        .with_tokio()
        .with_quic()
        .with_behaviour(|_| stream::Behaviour::new())?
        .with_swarm_config(|cfg| cfg.with_idle_connection_timeout(Duration::from_secs(10)))
        .build();

    let server_addr: Multiaddr = addr.parse().expect("Invalid Multiaddr");
    swarm.dial(server_addr.clone())?;

    let Some(Protocol::P2p(peer)) = server_addr.iter().last() else {
        anyhow::bail!("Provided address does not end in `/p2p`");
    };

    let control = swarm.behaviour().new_control();
    tokio::spawn(client_connection_handler(dev, peer, control));
    loop {
        let event = swarm.next().await.expect("never terminates");
        match event {
            libp2p::swarm::SwarmEvent::NewListenAddr { address, .. } => {
                let listen_address = address.with_p2p(*swarm.local_peer_id()).unwrap();
                log::info!("{listen_address:?}");
            }
            _event => log::info!("event"),
        }
    }
}

pub async fn send_tun(dev: &mut crate::tundev::TunDev, buf: &[u8], nbytes: usize) {
    match dev.send(&buf[..nbytes], nbytes).await {
        Ok(_) => {}
        Err(err) => log::error!("send_tun {}", err),
    }
}

async fn client_connection_handler(
    mut dev: tundev::TunDev,
    peer: PeerId,
    mut control: stream::Control,
) {
    let mut stream = match control.open_stream(peer, RETICULA_PROTOCOL).await {
        Ok(stream) => stream,
        Err(err) => {
            log::error!("{}", err);
            return;
        }
    };
    let mut tun_buf = [0; 1500];
    loop {
        let tun_ret = dev.read(&mut tun_buf).await;
        match tun_ret {
            Ok(amt) => {
                log::info!("<= Tun read {}", amt);

                let mut is_loopback = false;
                if let Some(header) = parse_ipv4_header(&tun_buf[..amt]) {
                    is_loopback =
                        header.src_ip == header.dst_ip && header.src_port == header.dst_port;
                    is_loopback = false; // TODO remove this line
                }
                if is_loopback {
                    send_tun(&mut dev, &tun_buf, amt).await;
                } else {
                    match stream.write_all(&tun_buf[..amt]).await {
                        Ok(_) => {}
                        Err(err) => log::error!("{}", err),
                    }
                }
            }
            Err(err) => {
                log::info!("{}", err);
            }
        }
    }
}
