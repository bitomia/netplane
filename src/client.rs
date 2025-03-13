use crate::common::{serialize_handshake, Handshake, HANDSHAKE_HEADER};
use crate::packet::parse_ipv4_header;
use crate::tundev;
use anyhow::{anyhow, Result};
use futures::{AsyncReadExt, AsyncWriteExt, StreamExt};
use libp2p::{multiaddr::Protocol, *};
use libp2p_stream as stream;
use std::time::Duration;
use std::sync::Arc;
use std::fs;
use crate::common;
use std::net::Ipv4Addr;

const RETICULA_PROTOCOL: StreamProtocol = StreamProtocol::new("/reticula");

pub struct Client {
    pub peer: PeerId,
    pub sdn_ip_addr: Ipv4Addr,
    pub addr: libp2p::multiaddr::Multiaddr,
}

pub async fn start(
    tun_name: &str,
    destination: &str,
    netmask: &str,
    sdn_ip_addr: &str,
    addr: &str,
) -> Result<()> {
    let dev = tundev::TunDev::new(tun_name, netmask, destination, sdn_ip_addr);
    let mut swarm = SwarmBuilder::with_new_identity()
        .with_tokio()
        .with_quic()
        .with_behaviour(|_| stream::Behaviour::new())?
        .with_swarm_config(|cfg| cfg.with_idle_connection_timeout(Duration::from_secs(10)))
        .build();

    let server_addr: Multiaddr = addr.parse().expect("Invalid Multiaddr");
    swarm.dial(server_addr.clone())?;

    let Some(Protocol::P2p(server_peer)) = server_addr.iter().last() else {
        anyhow::bail!("Provided address does not end in `/p2p`");
    };

    let identity = common::load_identity();
    let control = swarm.behaviour().new_control();
    tokio::spawn(client_connection_handler(dev, control, identity, server_peer, sdn_ip_addr.to_string()));
    loop {
        let event = swarm.next().await.expect("never terminates");
        match event {
            libp2p::swarm::SwarmEvent::NewListenAddr { address, .. } => {
                let listen_address = address.with_p2p(*swarm.local_peer_id()).unwrap();
                log::info!("{listen_address:?}");
            }
            libp2p::swarm::SwarmEvent::ConnectionClosed { .. } => {
                log::info!("Connection closed");
                break;
            }
            _event => log::info!("event"),
        }
    }
    Ok(())
}

pub async fn send_tun(mut dev: tundev::TunDev, buf: &[u8], nbytes: usize) {
    match dev.send(&buf[..nbytes], nbytes).await {
        Ok(_) => {
            log::info!("=> Tun write {}", nbytes);
        }
        Err(err) => log::error!("send_tun {}", err),
    }
}

async fn handle_client(mut dev: tundev::TunDev, mut stream: libp2p::Stream, mut incoming_streams: libp2p_stream::IncomingStreams) {
    let mut buf = [0; 1500];
    loop {
        tokio::select! {
            t = dev.read(&mut buf) => {
                if let Ok(amt) = t {
                    log::debug!("Tun read");
                    let mut is_loopback = false;
                    if let Some(header) = parse_ipv4_header(&buf[..amt]) {
                        is_loopback = header.src_ip == header.dst_ip && header.src_port == header.dst_port;
                    }
                    if is_loopback {
                        log::debug!("Loopback packet");
                        send_tun(dev.clone(), &buf, amt).await;
                    } else {
                        match stream.write_all(&buf[..amt]).await {
                            Ok(_) => {
                                log::debug!("Stream write");
                            }
                            Err(err) => log::error!("{}", err),
                        }
                    }
                }
            }
                s = incoming_streams.next() => {
                    if let Some((peer, mut stream)) = s {
                        log::debug!("Incoming stream from {}", peer);
                        let mut buf = [0; 1500];
                        match stream.read(&mut buf).await {
                            Ok(amt) => {
                                log::debug!("Stream read {}", amt);
                                send_tun(dev.clone(), &buf, amt).await;
                                log::debug!("Stream read sent to tun {}", amt);
                            }
                            Err(err) => {
                                log::error!("{}", err);
                            }
                        }
                    }
                }
        }
    }
}

async fn client_connection_handler(
    dev: tundev::TunDev,
    mut control: stream::Control,
    identity: identity::Keypair,
    server_peer: PeerId,
    sdn_ip_addr: String
) -> Result<()> {
    let incoming_streams = control.accept(RETICULA_PROTOCOL).unwrap();
    let mut stream = match control.open_stream(server_peer, RETICULA_PROTOCOL).await {
        Ok(stream) => stream,
        Err(err) => return Err(anyhow!("{}", err)),
    };
    let handshake = Handshake {
        header: HANDSHAKE_HEADER,
        sdn_ip_addr: sdn_ip_addr.parse().expect("Invalid IP address"),
        identity,
    };
    log::info!("Sending handshake");
    stream.write_all(&serialize_handshake(&handshake)).await?;

    let handler = tokio::spawn(handle_client(dev, stream, incoming_streams));
    tokio::select! {
        _ = handler => {
            log::info!("Handler finished");
        }
    }
    Ok(())
}
