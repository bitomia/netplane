use anyhow::Result;
use log::{error, info, trace};
use netplane_common::packet::{parse_ipv4_header, validate_packet};
use netplane_common::transport::{Transport, UdpTransport};
use netplane_common::{HandshakeReq, PeerState, UDPHeartbeat};
use std::collections::HashMap;
use std::net::{Ipv4Addr, SocketAddr};
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::db;
use crate::peers::PeersVec;
use crate::peers::*;
use crate::server::*;

pub struct UdpServer(Server<SocketAddr>);

impl UdpServer {
    pub fn new(db: Arc<db::Db>, stats: Arc<ServerStats>) -> Self {
        let peers: Peers<SocketAddr> = Peers::new(Mutex::new(HashMap::new()));
        Self(Server { peers, db, stats })
    }

    pub async fn start(&mut self) -> Result<()> {
        let server_addr = std::env::var("SERVER").unwrap_or("0.0.0.0:5000".to_string());
        info!("Netplane UDP server listening on {}", server_addr);

        let mut transport = UdpTransport::bind(&server_addr).await.unwrap();
        let mut buf = [0; 1500];

        loop {
            let (amt, src) = transport.recv(&mut buf).await?;
            self.0.stats.add_in_bytes(amt);

            let peer_state = {
                let mut peers = self.0.peers.lock().await;
                let peer = peers.entry(src).or_insert(UdpPeer::new(PeerData {
                    sdn_addr: Ipv4Addr::UNSPECIFIED,
                    state: PeerState::HandshakePending,
                }));
                peer.get_state()
            };

            match peer_state {
                PeerState::HandshakeDone => {
                    if validate_packet(&buf[..amt]) {
                        if let Some(header) = parse_ipv4_header(&buf[..amt]) {
                            let peers_vec = self.0.peers.to_vec().await;
                            for (dst_peer_addr, dst_peer_sdn_addr) in peers_vec {
                                if src == dst_peer_addr {
                                    continue;
                                }
                                if header.dst_ip == dst_peer_sdn_addr.to_string()
                                    || dst_peer_addr.ip().is_multicast()
                                {
                                    transport.send(&buf[..amt], Some(&dst_peer_addr)).await?;
                                    self.0.stats.add_out_bytes(amt);
                                }
                            }
                        }
                    } else if let Ok(_) = UDPHeartbeat::deserialize(&buf[..amt]) {
                        trace!("Heartbeat received from {:?}", src);
                    } else {
                        error!("Unknown packet");
                    }
                }
                PeerState::HandshakePending => match HandshakeReq::deserialize(&buf[..amt]) {
                    Ok(handshake) => {
                        match Server::<SocketAddr>::process_handshake(
                            handshake,
                            &self.0.db,
                            src.ip(),
                        )
                        .await
                        {
                            Ok((reply, sdn_client_ip)) => {
                                let mut peers = self.0.peers.lock().await;

                                match peers.get_mut(&src) {
                                    Some(peer) => {
                                        peer.set_sdn_addr(&Ipv4Addr::from_str(&sdn_client_ip)?);

                                        match transport.send(&reply.serialize()?, Some(&src)).await
                                        {
                                            Ok(_) => {
                                                peer.set_state(PeerState::HandshakeDone);
                                            }
                                            Err(err) => {
                                                error!("handshake response failed: {}", err)
                                            }
                                        }
                                    }
                                    None => info!("Peer unknown"),
                                }
                            }
                            Err(err) => error!("handshake failed: {}", err),
                        }
                    }
                    Err(err) => {
                        error!("HandshakeReq failed: {}", err);
                    }
                },
            }
        }
    }
}
