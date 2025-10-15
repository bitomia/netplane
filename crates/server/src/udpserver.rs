use anyhow::Result;
use log::{error, info, trace, warn};
use netplane_common::packet::{parse_ipv4_header, validate_packet};
use netplane_common::transport::{Transport, UdpTransport};
use netplane_common::{HandshakeReq, PeerState, UDPHeartbeat};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::net::{Ipv4Addr, SocketAddr};
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::sync::mpsc;
use tokio::time;

use crate::db;
use crate::peers::PeersVec;
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
    ) -> Self {
        let peers: Peers<SocketAddr> = Peers::new(Mutex::new(HashMap::new()));

        if let Some(traffic_logger_path) = dump_file {
            let traffic_logger = TrafficLogger::new(&traffic_logger_path).ok();
            Self {
                server: Server { peers, db, stats },
                traffic_logger,
                replay_file,
                replay_delay,
            }
        } else if let Some(_) = replay_file {
            Self {
                server: Server { peers, db, stats },
                traffic_logger: None,
                replay_file,
                replay_delay,
            }
        } else {
            Self {
                server: Server { peers, db, stats },
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

        // Start heartbeat timeout cleanup task
        let peers_for_cleanup = self.server.peers.clone();
        tokio::spawn(async move {
            let mut interval = time::interval(Duration::from_secs(5)); // Check interval
            let timeout = Duration::from_secs(5); // Hearbeat timeout

            loop {
                interval.tick().await;

                let mut peers = peers_for_cleanup.lock().await;
                let mut expired_peers = Vec::new();

                for (addr, peer) in peers.iter() {
                    let peer = peer.as_any().downcast_ref::<UdpPeer>().unwrap();
                    if peer.get_state() == PeerState::HandshakeDone
                        && peer.is_heartbeat_expired(timeout)
                    {
                        expired_peers.push(*addr);
                    }
                }

                for addr in expired_peers {
                    warn!("Removing peer {:?} due to heartbeat timeout", addr);
                    peers.remove(&addr);
                }
            }
        });

        loop {
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
                SourceAddr::Network(src) => self.network(&src, &mut transport, &buf, amt).await?,
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

    async fn network(
        &mut self,
        src: &SocketAddr,
        transport: &mut UdpTransport,
        buf: &[u8; 1500],
        amt: usize,
    ) -> Result<()> {
        self.server.stats.add_in_bytes(amt);

        let peer_state = {
            let mut peers = self.server.peers.lock().await;
            let peer = peers.entry(*src).or_insert(UdpPeer::new(PeerData {
                sdn_addr: Ipv4Addr::UNSPECIFIED,
                state: PeerState::HandshakePending,
            }));
            peer.get_state()
        };

        match peer_state {
            PeerState::HandshakeDone => {
                if validate_packet(&buf[..amt]) {
                    if let Some(header) = parse_ipv4_header(&buf[..amt]) {
                        if let Some(ref logger) = self.traffic_logger {
                            logger
                                .log_packet(&header.dst_ip.to_string(), &buf[..amt])
                                .await;
                        }

                        let peers_vec = self.server.peers.to_vec().await;
                        for (dst_peer_addr, dst_peer_sdn_addr) in peers_vec {
                            if *src == dst_peer_addr {
                                continue;
                            }

                            if header.dst_ip == dst_peer_sdn_addr.to_string()
                                || dst_peer_addr.ip().is_multicast()
                            {
                                transport.send(&buf[..amt], Some(&dst_peer_addr)).await?;
                                self.server.stats.add_out_bytes(amt);
                            }
                        }
                    }
                } else if let Ok(_) = UDPHeartbeat::deserialize(&buf[..amt]) {
                    trace!("Heartbeat received from {:?}", src);

                    let mut peers = self.server.peers.lock().await;
                    if let Some(peer) = peers.get_mut(src) {
                        let peer = peer.as_any_mut().downcast_mut::<UdpPeer>().unwrap();
                        peer.update_last_heartbeat();
                    }
                } else {
                    error!("Unknown packet");
                }
            }
            PeerState::HandshakePending => match HandshakeReq::deserialize(&buf[..amt]) {
                Ok(handshake) => {
                    match Server::<SocketAddr>::process_handshake(
                        handshake,
                        &self.server.db,
                        src.ip(),
                    )
                    .await
                    {
                        HandshakeResult::Success(reply, sdn_client_ip) => {
                            let mut peers = self.server.peers.lock().await;

                            match peers.get_mut(&src) {
                                Some(peer) => {
                                    peer.set_sdn_addr(&Ipv4Addr::from_str(&sdn_client_ip)?);

                                    match transport.send(&reply.serialize()?, Some(&src)).await {
                                        Ok(_) => {
                                            info!("User successfully connected");
                                            peer.set_state(PeerState::HandshakeDone);
                                        }
                                        Err(err) => {
                                            error!("Send handshake reply failed: {}", err);
                                        }
                                    }
                                }
                                None => info!("Peer unknown"),
                            }
                        }
                        HandshakeResult::Error(error_response) => {
                            match transport
                                .send(&error_response.serialize()?, Some(&src))
                                .await
                            {
                                Ok(_) => {
                                    info!("Authorization failed, error response sent to {}", src);
                                }
                                Err(err) => {
                                    error!("Failed to send error response: {}", err);
                                }
                            }
                        }
                    }
                }
                Err(err) => error!("HandshakeReq failed: {}", err),
            },
        }
        Ok(())
    }
}
