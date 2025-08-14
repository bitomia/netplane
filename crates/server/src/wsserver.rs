use anyhow::Result;
use log::{error, info};
use netplane_common::packet::{parse_ipv4_header, validate_packet};
use netplane_common::transport::{Transport, WebSocketTransport};
use netplane_common::{HandshakeReq, PeerState};
use std::collections::HashMap;
use std::net::{Ipv4Addr, SocketAddr};
use std::str::FromStr;
use std::sync::Arc;
use std::sync::atomic::{AtomicI32, Ordering};
use tokio::sync::Mutex;
use tokio::sync::mpsc;

use crate::db;
use crate::peers::*;
use crate::server::*;

pub struct WebSocketServer(Server<i32>);

impl WebSocketServer {
    pub fn new(db: Arc<db::Db>, stats: Arc<ServerStats>) -> Self {
        let peers: Peers<i32> = Peers::new(Mutex::new(HashMap::new()));
        Self(Server { peers, db, stats })
    }

    pub async fn start(&mut self) -> Result<()> {
        let listen_addr = std::env::var("SERVER").unwrap_or("0.0.0.0:5000".to_string());
        info!("Netplane websocket server listening on {}", listen_addr);

        WebSocketTransport::bind(&listen_addr, {
            let db = Arc::clone(&self.0.db);
            let peers = Arc::clone(&self.0.peers);
            let next_peer_id = Arc::new(AtomicI32::new(0));
            let next_peer_id_clone = Arc::clone(&next_peer_id);

            move |socket, addr| {
                let peer_id = next_peer_id_clone.fetch_add(1, Ordering::SeqCst);

                WebSocketServer::ws_handle_connection(
                    peer_id,
                    socket,
                    addr,
                    Arc::clone(&db),
                    Arc::clone(&peers),
                )
            }
        })
        .await;
        Ok(())
    }

    async fn ws_handle_connection(
        peer_id: i32,
        socket: WebSocketTransport,
        addr: SocketAddr,
        db: Arc<db::Db>,
        peers: Peers<i32>,
    ) {
        info!("Connection started {} {:?}", peer_id, addr);

        let (tx, mut rx): (Tx, Rx) = mpsc::unbounded_channel();
        let mut send_socket = socket.clone();
        let send_task = tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                send_socket.send(msg.as_ref(), None).await.unwrap();
            }
        });

        let mut recv_socket = socket.clone();
        let forward_task = tokio::spawn(async move {
            let mut buf = [0; 1500];

            loop {
                let (amt, _) = recv_socket.recv(&mut buf).await.unwrap();

                let status = {
                    let mut peers_guard = peers.lock().await;
                    let state = PeerState::HandshakePending;
                    peers_guard.entry(peer_id).or_insert(TcpPeer::new(
                        PeerData {
                            sdn_addr: Ipv4Addr::UNSPECIFIED,
                            state: state.clone(),
                        },
                        tx.clone(),
                    ));
                    state
                };

                match status {
                    PeerState::HandshakeDone => {
                        if validate_packet(&buf[..amt]) {
                            if let Some(header) = parse_ipv4_header(&buf[..amt]) {
                                let peers_guard = peers.lock().await;
                                for (&_peer_id, peer) in peers_guard.iter() {
                                    if peer.get_sdn_addr().to_string() == header.dst_ip.to_string()
                                    {
                                        if let Some(peer) = peer.as_any().downcast_ref::<TcpPeer>()
                                        {
                                            peer.tx
                                                .send(bytes::Bytes::copy_from_slice(&buf[..amt]))
                                                .unwrap();
                                        }
                                        break;
                                    }
                                }
                            }
                        }
                    }
                    PeerState::HandshakePending => {
                        match HandshakeReq::deserialize(&buf[..amt]) {
                            Ok(handshake) => {
                                match Server::<i32>::process_handshake(handshake, &db, addr.ip())
                                    .await
                                {
                                    Ok((reply, sdn_client_ip)) => {
                                        {
                                            let mut peers_guard = peers.lock().await;
                                            peers_guard.entry(peer_id).and_modify(|p| {
                                                p.set_sdn_addr(
                                                    &Ipv4Addr::from_str(&sdn_client_ip).unwrap(),
                                                );
                                            });
                                        }
                                        let mut reply_socket = socket.clone();
                                        match reply_socket
                                            .send(&reply.serialize().unwrap(), None)
                                            .await
                                        {
                                            Ok(_) => {
                                                let mut peers_guard = peers.lock().await;
                                                peers_guard.entry(peer_id).and_modify(|p| {
                                                    p.set_state(PeerState::HandshakeDone);
                                                });
                                            }
                                            Err(_) => {
                                                error!("Send handhsake reply failed");
                                            }
                                        }
                                    }
                                    Err(_) => {
                                        // Error already logged in process_handshake
                                    }
                                }
                            }
                            Err(err) => {
                                error!("HandshakeReq failed: {}", err);
                            }
                        }
                    }
                }
            }
        });
        tokio::select! {
            _ = forward_task => {},
            _ = send_task => {},
        }
    }
}
