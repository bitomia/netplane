use anyhow::Result;
use dashmap::DashMap;
use netplane_common::packet::is_multicast_or_broadcast;
use netplane_common::transport::{Transport, UdpTransport};
use netplane_common::{
    MessageType, P2PHandshakeInit, P2PHandshakeResp, PeerAnnounce, PeerEventType, PeerInfo,
    PeerList, PeerState, RelayPacket, UDPHeartbeat, get_message_type,
};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::net::{Ipv4Addr, SocketAddr};
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{error, info, trace, warn};

use crate::db;
use crate::peers::PeersVec;
use crate::peers::UdpPeersRouting;
use crate::peers::*;
use crate::server::*;
use crate::source::*;
use crate::trafficlog::*;

pub struct UdpServer {
    server: Server<SocketAddr>,
    traffic_logger: Option<TrafficLogger>,
    replay_file: Option<String>,
    replay_delay: Option<u64>,
}

impl UdpServer {
    pub fn new(
        db: Arc<db::Db>,
        stats: Arc<ServerStats>,
        dump_file: Option<String>,
        replay_file: Option<String>,
        replay_delay: Option<u64>,
        dynamic_clients_key: Option<String>,
    ) -> Self {
        let peers: Peers<SocketAddr> = Peers::new(DashMap::new());

        if let Some(traffic_logger_path) = dump_file {
            let traffic_logger = TrafficLogger::new(&traffic_logger_path).ok();
            Self {
                server: Server {
                    peers,
                    db,
                    stats,
                    dynamic_clients_key,
                },
                traffic_logger,
                replay_file,
                replay_delay,
            }
        } else if replay_file.is_some() {
            Self {
                server: Server {
                    peers,
                    db,
                    stats,
                    dynamic_clients_key,
                },
                traffic_logger: None,
                replay_file,
                replay_delay,
            }
        } else {
            Self {
                server: Server {
                    peers,
                    db,
                    stats,
                    dynamic_clients_key,
                },
                traffic_logger: None,
                replay_file: None,
                replay_delay: None,
            }
        }
    }

    pub async fn start(&mut self) -> Result<()> {
        let server_addr = std::env::var("SERVER").unwrap_or("0.0.0.0:5000".to_string());
        info!("Netplane UDP server listening on {}", server_addr);

        let mut transport = UdpTransport::bind(&server_addr).await.unwrap();
        let mut buf = [0; 1500];

        // Channel for replay injection
        let (replay_tx, mut replay_rx) = mpsc::unbounded_channel::<(Vec<u8>, String)>();

        // Start replay task if configured
        if let (Some(replay_file), Some(replay_delay)) =
            (self.replay_file.clone(), self.replay_delay)
        {
            tokio::spawn(async move {
                info!("Waiting {} seconds before starting replay...", replay_delay);
                tokio::time::sleep(Duration::from_secs(replay_delay)).await;
                info!("Starting traffic replay");

                let file = match File::open(&replay_file) {
                    Ok(f) => f,
                    Err(e) => {
                        error!("Failed to open replay file: {}", e);
                        return;
                    }
                };
                let reader = BufReader::new(file);

                for line in reader.lines() {
                    let line = match line {
                        Ok(l) => l,
                        Err(e) => {
                            error!("Failed to read line: {}", e);
                            continue;
                        }
                    };

                    if line.trim().is_empty() {
                        continue;
                    }

                    match ReplayEntry::parse(&line) {
                        Ok(entry) => {
                            let _ = replay_tx.send((entry.data, entry.sdn_addr));
                            tokio::time::sleep(Duration::from_millis(10)).await;
                        }
                        Err(e) => {
                            warn!("Failed to parse replay line: {} - {}", line, e);
                        }
                    }
                }

                info!("Replay completed");
            });
        }

        loop {
            self.server.peers.retain(|addr, peer| {
                let udp_peer = peer.as_any().downcast_ref::<UdpPeer>().unwrap();
                if udp_peer.get_state() == PeerState::HandshakeDone
                    && udp_peer.is_heartbeat_expired(Duration::from_secs(15))
                {
                    self.server.stats.dec_clients();
                    info!(
                        "Removing expired peer sdn_addr={} addr={}",
                        udp_peer.get_sdn_addr(),
                        addr,
                    );
                    true
                } else {
                    false
                }
            });

            let (amt, src) = tokio::select! {
                Some((data, addr)) = replay_rx.recv() => {
                    let addr = Ipv4Addr::from_str(&addr)?;
                    buf[..data.len()].copy_from_slice(&data);
                    (data.len(), SourceAddr::Replay(addr))
                },
                result = transport.recv(&mut buf) => {
                    let (amt, addr) = result?;
                    (amt, SourceAddr::Network(addr))
                }
            };

            match src {
                SourceAddr::Replay(src) => self.replay(&src, &mut transport, &buf, amt).await?,
                SourceAddr::Network(src) => {
                    self.network(&src, &mut transport, &buf, amt, self.server.stats.clone())
                        .await?
                }
            }
        }
    }

    async fn replay(
        &mut self,
        src: &Ipv4Addr,
        transport: &mut UdpTransport,
        buf: &[u8; 1500],
        amt: usize,
    ) -> Result<()> {
        let peers_vec = self.server.peers.to_vec().await;
        for (dst_peer_addr, dst_peer_sdn_addr) in peers_vec {
            if *src == dst_peer_sdn_addr {
                transport.send(&buf[..amt], Some(&dst_peer_addr)).await?;
                break;
            }
        }
        Ok(())
    }

    /// Broadcast a message to all connected peers except the specified one
    async fn broadcast_to_peers(
        &self,
        transport: &mut UdpTransport,
        msg: &[u8],
        exclude: Option<&SocketAddr>,
    ) {
        // Collect addresses first to avoid holding lock across await
        let addrs: Vec<SocketAddr> = {
            self.server
                .peers
                .iter()
                .filter(|peer| {
                    peer.get_state() == PeerState::HandshakeDone && (exclude != Some(peer.key()))
                })
                .map(|peer| *peer.key())
                .collect()
        };

        for addr in addrs {
            if let Err(e) = transport.send(msg, Some(&addr)).await {
                error!("Failed to broadcast to {:?}: {}", addr, e);
            }
        }
    }

    /// Send peer list to a specific peer
    async fn send_peer_list(&self, transport: &mut UdpTransport, to: &SocketAddr) {
        let peer_list = self
            .server
            .peers
            .get_peer_list()
            .await
            .into_iter()
            .filter(|p| Ipv4Addr::from_str(&p.sdn_ip).ok() != Some(Ipv4Addr::UNSPECIFIED))
            .collect();

        let msg = PeerList::new(peer_list);
        if let Ok(serialized) = msg.serialize()
            && let Err(e) = transport.send(&serialized, Some(to)).await
        {
            error!("Failed to send peer list to {:?}: {}", to, e);
        }
    }

    async fn network(
        &mut self,
        src: &SocketAddr,
        transport: &mut UdpTransport,
        buf: &[u8; 1500],
        amt: usize,
        stats: Arc<ServerStats>,
    ) -> Result<()> {
        self.server.stats.add_in_bytes(amt);

        let peer_state = {
            let peer = self
                .server
                .peers
                .entry(*src)
                .or_insert(UdpPeer::new(PeerData {
                    sdn_addr: Ipv4Addr::UNSPECIFIED,
                    state: PeerState::HandshakePending,
                    client_public_key: None,
                }));
            peer.get_state()
        };

        match peer_state {
            PeerState::HandshakeDone => {
                // Route based on message type
                match get_message_type(&buf[..amt]) {
                    MessageType::RelayPacket(relay) => {
                        self.handle_relay_packet(transport, src, &relay, &buf[..amt])
                            .await?;
                    }
                    MessageType::P2PHandshakeInit(init) => {
                        self.handle_p2p_handshake_init(transport, &init, &buf[..amt])
                            .await?;
                    }
                    MessageType::P2PHandshakeResp(resp) => {
                        self.handle_p2p_handshake_resp(transport, &resp, &buf[..amt])
                            .await?;
                    }
                    MessageType::Heartbeat(_) => {
                        trace!("Heartbeat received from {:?}", src);
                        if let Some(mut peer) = self.server.peers.get_mut(src) {
                            let udp_peer = peer.as_any_mut().downcast_mut::<UdpPeer>().unwrap();
                            udp_peer.update_last_heartbeat();
                        }
                        let reply = UDPHeartbeat::new();
                        if let Ok(data) = reply.serialize()
                            && let Err(e) = transport.send(&data, Some(src)).await
                        {
                            error!("Failed to send heartbeat reply to {:?}: {}", src, e);
                        }
                    }
                    _ => {
                        warn!("Unknown or unexpected message type from {:?}", src);
                    }
                }
            }
            PeerState::HandshakePending => {
                if let MessageType::HandshakeReq(handshake) = get_message_type(&buf[..amt]) {
                    match Server::<SocketAddr>::process_handshake(
                        handshake,
                        &self.server.db,
                        src.ip(),
                    )
                    .await
                    {
                        HandshakeResult::Success(reply, sdn_client_ip, client_pub_key) => {
                            // Send handshake reply
                            match transport.send(&reply.serialize()?, Some(src)).await {
                                Ok(_) => {
                                    info!("User {} connected with SDN IP {}", src, sdn_client_ip);
                                    stats.inc_clients();

                                    // Create peer info for announcements
                                    let peer_info = PeerInfo::new(&sdn_client_ip, &client_pub_key);

                                    // Update peer state
                                    self.server
                                        .peers
                                        .update(
                                            *src,
                                            sdn_client_ip.clone(),
                                            client_pub_key.clone(),
                                            PeerState::HandshakeDone,
                                        )
                                        .await;

                                    // Send current peer list to new peer
                                    self.send_peer_list(transport, src).await;

                                    // Broadcast new peer to existing peers
                                    let announce =
                                        PeerAnnounce::new(PeerEventType::Connected, peer_info);
                                    if let Ok(serialized) = announce.serialize() {
                                        self.broadcast_to_peers(transport, &serialized, Some(src))
                                            .await;
                                    }
                                }
                                Err(err) => {
                                    error!("Send handshake reply failed: {}", err);
                                }
                            }
                        }
                        HandshakeResult::Error(error_response) => {
                            if let Err(err) = transport
                                .send(&error_response.serialize()?, Some(src))
                                .await
                            {
                                error!("Failed to send error response: {}", err);
                            } else {
                                info!("Authorization failed, error response sent to {}", src);
                            }
                        }
                    }
                } else {
                    error!("Expected HandshakeReq from {:?}", src);
                }
            }
        }
        Ok(())
    }

    /// Handle E2E encrypted relay packet - just forward based on destination SDN IP
    async fn handle_relay_packet(
        &self,
        transport: &mut UdpTransport,
        _src: &SocketAddr,
        relay: &RelayPacket,
        raw_data: &[u8],
    ) -> Result<()> {
        let dst_sdn_ip = Ipv4Addr::from_str(&relay.dst_ip)?;

        // Log traffic if configured
        if let Some(ref logger) = self.traffic_logger {
            logger.log_packet(&relay.dst_ip, raw_data).await;
        }

        // Multicast/broadcast: fan out to all peers except the sender
        if is_multicast_or_broadcast(&dst_sdn_ip) {
            let src_sdn_ip = Ipv4Addr::from_str(&relay.src_ip)?;
            let addrs = self.server.peers.find_all_addrs_except(&src_sdn_ip).await;
            for addr in addrs {
                if let Err(e) = transport.send(raw_data, Some(&addr)).await {
                    error!("Failed to relay multicast packet to {:?}: {}", addr, e);
                } else {
                    self.server.stats.add_out_bytes(raw_data.len());
                }
            }
            return Ok(());
        }

        // Find destination peer and forward
        if let Some(dst_addr) = self.server.peers.find_by_sdn_ip(&dst_sdn_ip).await {
            if let Err(e) = transport.send(raw_data, Some(&dst_addr)).await {
                error!("Failed to relay packet to {:?}: {}", dst_addr, e);
            } else {
                self.server.stats.add_out_bytes(raw_data.len());
            }
        } else {
            trace!("Destination {} not found for relay", relay.dst_ip);
        }

        Ok(())
    }

    /// Handle P2P handshake init - route to responder
    async fn handle_p2p_handshake_init(
        &self,
        transport: &mut UdpTransport,
        init: &P2PHandshakeInit,
        raw_data: &[u8],
    ) -> Result<()> {
        let responder_sdn_ip = Ipv4Addr::from_str(&init.responder_sdn_ip)?;

        info!(
            "Routing P2P handshake init from {} to {}",
            init.initiator_sdn_ip, init.responder_sdn_ip
        );

        if let Some(dst_addr) = self.server.peers.find_by_sdn_ip(&responder_sdn_ip).await {
            if let Err(e) = transport.send(raw_data, Some(&dst_addr)).await {
                error!("Failed to route P2P handshake init: {}", e);
            }
        } else {
            warn!(
                "P2P handshake responder {} not found",
                init.responder_sdn_ip
            );
        }

        Ok(())
    }

    /// Handle P2P handshake response - route to initiator
    async fn handle_p2p_handshake_resp(
        &self,
        transport: &mut UdpTransport,
        resp: &P2PHandshakeResp,
        raw_data: &[u8],
    ) -> Result<()> {
        let initiator_sdn_ip = Ipv4Addr::from_str(&resp.initiator_sdn_ip)?;

        info!(
            "Routing P2P handshake response from {} to {}",
            resp.responder_sdn_ip, resp.initiator_sdn_ip
        );

        if let Some(dst_addr) = self.server.peers.find_by_sdn_ip(&initiator_sdn_ip).await {
            if let Err(e) = transport.send(raw_data, Some(&dst_addr)).await {
                error!("Failed to route P2P handshake response: {}", e);
            }
        } else {
            warn!(
                "P2P handshake initiator {} not found",
                resp.initiator_sdn_ip
            );
        }

        Ok(())
    }
}
