use anyhow::Result;
use dashmap::DashMap;
use netplane_common::packet::is_multicast_or_broadcast;
use netplane_common::transport::{Transport, WebSocketTransport};
use netplane_common::{
    MessageType, P2PHandshakeInit, P2PHandshakeResp, PeerAnnounce, PeerEventType, PeerInfo,
    PeerList, PeerState, RelayPacket, get_message_type,
};
use std::net::{Ipv4Addr, SocketAddr};
use std::str::FromStr;
use std::sync::Arc;
use std::sync::atomic::{AtomicI32, Ordering};
use tokio::sync::mpsc;
use tracing::{debug, error, info, trace, warn};

use crate::db;
use crate::peers::TcpPeersRouting;
use crate::peers::*;
use crate::server::*;

pub struct WebSocketServer(Server<i32>);

impl WebSocketServer {
    pub fn new(
        db: Arc<db::Db>,
        stats: Arc<ServerStats>,
        dynamic_clients_key: Option<String>,
    ) -> Self {
        let peers: Peers<i32> = Peers::new(DashMap::new());
        Self(Server {
            peers,
            db,
            stats,
            dynamic_clients_key,
        })
    }

    pub async fn start(&mut self) -> tokio::io::Result<()> {
        let listen_addr = std::env::var("SERVER").unwrap_or("0.0.0.0:5000".to_string());
        info!("Netplane websocket server listening on {}", listen_addr);

        let db = Arc::clone(&self.0.db);
        let peers = Arc::clone(&self.0.peers);
        let stats = Arc::clone(&self.0.stats);
        let next_peer_id = Arc::new(AtomicI32::new(0));
        let dynamic_clients_key = self.0.dynamic_clients_key.clone();

        WebSocketTransport::bind(&listen_addr, move |socket, addr| {
            info!("New client connection");

            WebSocketServer::ws_handle_connection(
                next_peer_id.fetch_add(1, Ordering::SeqCst),
                socket,
                addr,
                Arc::clone(&db),
                Arc::clone(&peers),
                Arc::clone(&stats),
                dynamic_clients_key.clone(),
            )
        })
        .await
    }

    async fn ws_handle_connection(
        peer_id: i32,
        socket: WebSocketTransport,
        addr: SocketAddr,
        db: Arc<db::Db>,
        peers: Peers<i32>,
        stats: Arc<ServerStats>,
        _dynamic_clients_key: Option<String>,
    ) {
        info!(
            "Starting handler for new client connection (connections={})",
            peers.len()
        );
        stats.inc_clients();

        let (tx, mut rx): (Tx, Rx) = mpsc::unbounded_channel();
        let mut send_socket = socket.clone();
        let send_task = tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                send_socket.send(msg.as_ref(), None).await.unwrap_or(0);
            }
        });

        let mut recv_socket = socket.clone();
        let peers_clone = peers.clone();
        let stats_clone = stats.clone();
        let forward_task = tokio::spawn(async move {
            let mut buf = [0; 1500];

            loop {
                let amt = match recv_socket.recv(&mut buf).await {
                    Ok((amt, _)) => amt,
                    Err(err) => {
                        warn!("Removing peer {} due to socket error {:?}", peer_id, err);

                        // Get peer info before removing for disconnect broadcast
                        let peer_info = {
                            if let Some(peer) = peers.get(&peer_id) {
                                if peer.get_state() == PeerState::HandshakeDone {
                                    Some(PeerInfo::new(
                                        &peer.get_sdn_addr().to_string(),
                                        &peer.get_client_public_key().unwrap_or_default(),
                                    ))
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        };

                        // Remove peer
                        peers.remove(&peer_id);

                        // Broadcast disconnect if peer was connected
                        if let Some(info) = peer_info {
                            let announce = PeerAnnounce::new(PeerEventType::Disconnected, info);
                            if let Ok(serialized) = announce.serialize() {
                                let all_tx = peers.get_all_tx().await;
                                for peer_tx in all_tx {
                                    let _ = peer_tx.send(bytes::Bytes::from(serialized.clone()));
                                }
                            }
                        }

                        return;
                    }
                };
                stats_clone.add_in_bytes(amt);

                let peer_state = {
                    let peer = peers.entry(peer_id).or_insert(TcpPeer::new(
                        PeerData {
                            sdn_addr: Ipv4Addr::UNSPECIFIED,
                            state: PeerState::HandshakePending,
                            client_public_key: None,
                        },
                        tx.clone(),
                    ));
                    peer.get_state()
                };

                match peer_state {
                    PeerState::HandshakeDone => {
                        // Route based on message type
                        match get_message_type(&buf[..amt]) {
                            MessageType::RelayPacket(relay) => {
                                debug!("RelayPacket");
                                if let Err(e) = handle_ws_relay_packet(
                                    &peers,
                                    &stats_clone,
                                    &relay,
                                    &buf[..amt],
                                )
                                .await
                                {
                                    error!("Failed to handle relay packet: {}", e);
                                }
                            }
                            MessageType::P2PHandshakeInit(init) => {
                                debug!("P2PHandshakeInit");
                                if let Err(e) =
                                    handle_ws_p2p_handshake_init(&peers, &init, &buf[..amt]).await
                                {
                                    error!("Failed to handle P2P handshake init: {}", e);
                                }
                            }
                            MessageType::P2PHandshakeResp(resp) => {
                                debug!("P2PHandshakeResp");
                                if let Err(e) =
                                    handle_ws_p2p_handshake_resp(&peers, &resp, &buf[..amt]).await
                                {
                                    error!("Failed to handle P2P handshake response: {}", e);
                                }
                            }
                            MessageType::Heartbeat(_) => {
                                debug!("Heartbeat received from peer {}", peer_id);
                                // WebSocket doesn't need heartbeat tracking like UDP
                            }
                            _ => {
                                warn!("Unknown or unexpected message type from peer {}", peer_id);
                            }
                        }
                    }
                    PeerState::HandshakePending => {
                        if let MessageType::HandshakeReq(handshake) = get_message_type(&buf[..amt])
                        {
                            match Server::<i32>::process_handshake(handshake, &db, addr.ip()).await
                            {
                                HandshakeResult::Success(reply, sdn_client_ip, client_pub_key) => {
                                    let mut reply_socket = socket.clone();
                                    match reply_socket.send(&reply.serialize().unwrap(), None).await
                                    {
                                        Ok(_) => {
                                            info!(
                                                "User {} connected with SDN IP {}",
                                                addr, sdn_client_ip
                                            );

                                            // Create peer info for announcements
                                            let peer_info =
                                                PeerInfo::new(&sdn_client_ip, &client_pub_key);

                                            // Update peer state
                                            peers
                                                .update(
                                                    peer_id,
                                                    sdn_client_ip.clone(),
                                                    client_pub_key.clone(),
                                                    PeerState::HandshakeDone,
                                                )
                                                .await;

                                            // Send current peer list to new peer (excluding self)
                                            let peer_list: Vec<PeerInfo> = peers
                                                .get_peer_list()
                                                .await
                                                .into_iter()
                                                .filter(|p| p.sdn_ip != sdn_client_ip)
                                                .collect();
                                            let list_msg = PeerList::new(peer_list);
                                            if let Ok(serialized) = list_msg.serialize() {
                                                let _ = tx.send(bytes::Bytes::from(serialized));
                                            }

                                            // Broadcast new peer to existing peers
                                            let announce = PeerAnnounce::new(
                                                PeerEventType::Connected,
                                                peer_info,
                                            );
                                            if let Ok(serialized) = announce.serialize() {
                                                let all_tx = peers.get_all_tx().await;
                                                for peer_tx in all_tx {
                                                    if !std::ptr::eq(&peer_tx, &tx) {
                                                        let _ = peer_tx.send(bytes::Bytes::from(
                                                            serialized.clone(),
                                                        ));
                                                    }
                                                }
                                            }
                                        }
                                        Err(err) => {
                                            error!("Send handshake reply failed: {}", err);
                                        }
                                    }
                                }
                                HandshakeResult::Error(error_response) => {
                                    let mut error_socket = socket.clone();
                                    if let Err(err) = error_socket
                                        .send(&error_response.serialize().unwrap(), None)
                                        .await
                                    {
                                        error!("Failed to send error response: {}", err);
                                    } else {
                                        info!(
                                            "Authorization failed, error response sent to {}",
                                            addr
                                        );
                                    }
                                }
                            }
                        } else {
                            error!("Expected HandshakeReq from peer {}", peer_id);
                        }
                    }
                }
            }
        });

        tokio::select! {
            _ = forward_task => {
                info!("Forward task finished");
            },
            _ = send_task => {
                info!("Send task finished");
            },
        }

        // Cleanup: broadcast disconnect on task completion
        let peer_info = {
            if let Some(peer) = peers_clone.get(&peer_id) {
                if peer.get_state() == PeerState::HandshakeDone {
                    Some(PeerInfo::new(
                        &peer.get_sdn_addr().to_string(),
                        &peer.get_client_public_key().unwrap_or_default(),
                    ))
                } else {
                    None
                }
            } else {
                None
            }
        };

        if let Some(info) = peer_info {
            let announce = PeerAnnounce::new(PeerEventType::Disconnected, info);
            if let Ok(serialized) = announce.serialize() {
                let all_tx = peers_clone.get_all_tx().await;
                for peer_tx in all_tx {
                    let _ = peer_tx.send(bytes::Bytes::from(serialized.clone()));
                }
            }
        }

        // Remove peer from list
        stats.dec_clients();
        peers_clone.remove(&peer_id);
        info!("Peer removed peer_id={}", peer_id);
    }
}

/// Handle E2E encrypted relay packet - just forward based on destination SDN IP
async fn handle_ws_relay_packet(
    peers: &Peers<i32>,
    stats: &Arc<ServerStats>,
    relay: &RelayPacket,
    raw_data: &[u8],
) -> Result<()> {
    let dst_sdn_ip = Ipv4Addr::from_str(&relay.dst_ip)?;

    // Multicast/broadcast: fan out to all peers except the sender
    if is_multicast_or_broadcast(&dst_sdn_ip) {
        let src_sdn_ip = Ipv4Addr::from_str(&relay.src_ip)?;
        let txs = peers.get_all_tx_except(&src_sdn_ip).await;
        let data = bytes::Bytes::from(raw_data.to_vec());
        for tx in txs {
            if let Err(e) = tx.send(data.clone()) {
                error!("Failed to relay multicast packet: {}", e);
            } else {
                stats.add_out_bytes(raw_data.len());
            }
        }
        return Ok(());
    }

    if let Some(dst_tx) = peers.find_tx_by_sdn_ip(&dst_sdn_ip).await {
        if let Err(e) = dst_tx.send(bytes::Bytes::from(raw_data.to_vec())) {
            error!("Failed to relay packet: {}", e);
        } else {
            stats.add_out_bytes(raw_data.len());
        }
    } else {
        trace!("Destination {} not found for relay", relay.dst_ip);
    }

    Ok(())
}

/// Handle P2P handshake init - route to responder
async fn handle_ws_p2p_handshake_init(
    peers: &Peers<i32>,
    init: &P2PHandshakeInit,
    raw_data: &[u8],
) -> Result<()> {
    let responder_sdn_ip = Ipv4Addr::from_str(&init.responder_sdn_ip)?;

    info!(
        "Routing P2P handshake init from {} to {}",
        init.initiator_sdn_ip, init.responder_sdn_ip
    );

    if let Some(dst_tx) = peers.find_tx_by_sdn_ip(&responder_sdn_ip).await {
        if let Err(e) = dst_tx.send(bytes::Bytes::from(raw_data.to_vec())) {
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
async fn handle_ws_p2p_handshake_resp(
    peers: &Peers<i32>,
    resp: &P2PHandshakeResp,
    raw_data: &[u8],
) -> Result<()> {
    let initiator_sdn_ip = Ipv4Addr::from_str(&resp.initiator_sdn_ip)?;

    info!(
        "Routing P2P handshake response from {} to {}",
        resp.responder_sdn_ip, resp.initiator_sdn_ip
    );

    if let Some(dst_tx) = peers.find_tx_by_sdn_ip(&initiator_sdn_ip).await {
        if let Err(e) = dst_tx.send(bytes::Bytes::from(raw_data.to_vec())) {
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
