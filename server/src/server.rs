use anyhow::Result;
use common::packet::parse_ipv4_header;
use common::transport::{Transport, UdpTransport, WebSocketTransport};
use common::{HandshakeRep, HandshakeReq, HandshakeStatus};
use log::{error, info};
use std::collections::HashMap;
use std::net::{Ipv4Addr, SocketAddr};
use std::str::FromStr;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

use crate::db;
use crate::peers::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessError(u32);

pub struct Server {
    peers: Peers,
    db: Arc<db::Db>,
    transport: String,
}

impl Server {
    pub fn new(db: Arc<db::Db>, transport: &String) -> Server {
        let mut transport = transport.clone();
        transport.make_ascii_lowercase();

        let peers = match transport.as_str() {
            "websocket" => {
                info!("Initializing with WebSocket transport");
                Peers::TcpPeers(Arc::new(Mutex::new(HashMap::new())))
            }
            _ => {
                info!("Initializing with UDP transport");
                Peers::UdpPeers(HashMap::new())
            }
        };

        Server {
            peers,
            db,
            transport,
        }
    }

    pub async fn start(&mut self) -> Result<()> {
        match self.transport.as_str() {
            "websocket" => self.ws_start().await,
            _ => self.udp_start().await,
        }
    }

    async fn udp_start(&mut self) -> Result<()> {
        let server_addr = std::env::var("SERVER").unwrap_or("0.0.0.0:5000".to_string());
        let mut transport = UdpTransport::bind(&server_addr).await.unwrap();
        let mut buf = [0; 1500];

        loop {
            let (amt, src) = transport.recv(&mut buf).await?;

            let peers = try_get_udp(&mut self.peers).unwrap();
            let peer = peers.entry(src).or_insert(UdpPeer {
                sdn_ip_addr: Ipv4Addr::UNSPECIFIED,
                status: HandshakeStatus::Pending,
            });

            match peer.status {
                HandshakeStatus::Initialized => {
                    if let Some(_header) = parse_ipv4_header(&buf[..amt]) {
                        for (other_peer_addr, _) in peers.clone() {
                            if src != other_peer_addr {
                                // TODO this is broadcasting, parse IP header and send only to target
                                transport.send(&buf[..amt], Some(&other_peer_addr)).await?;
                            }
                        }
                    } else {
                        error!("Packet not supported");
                    }
                }
                HandshakeStatus::Pending => {
                    match HandshakeReq::deserialize(&buf[..amt]) {
                        Ok(handshake) => {
                            match common::crypto::verify_signed_key(handshake.auth_key) {
                                Ok(auth_client) => {
                                    if let Ok(client) =
                                        self.db.get_client(&auth_client.client_id).await
                                    {
                                        peer.sdn_ip_addr =
                                            Ipv4Addr::from_str(&client.sdn_client_ip.as_str())?;

                                        info!("Client connected {} {}", src, client.sdn_client_ip);

                                        let reply = HandshakeRep::new(
                                            &client.netmask,
                                            &client.network,
                                            &client.sdn_client_ip,
                                        );
                                        match transport.send(&reply.serialize()?, Some(&src)).await
                                        {
                                            Ok(_) => {
                                                peer.status = HandshakeStatus::Initialized;
                                            }
                                            Err(_) => {
                                                // TODO
                                            }
                                        }
                                    } else {
                                        error!("Ignoring Unknown user {}", src.ip());
                                    }
                                }
                                Err(error) => {
                                    error!(
                                        "Unexpected verifying key error: {} {}",
                                        src.ip(),
                                        error
                                    );
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
    }

    async fn ws_start(&mut self) -> Result<()> {
        let listen_addr = std::env::var("SERVER").unwrap_or("0.0.0.0:5000".to_string());
        let peers = try_get_tcp(&mut self.peers).unwrap();

        WebSocketTransport::bind(&listen_addr, {
            let db = Arc::clone(&self.db);
            let peers = Arc::clone(peers);
            let next_peer_id = Arc::new(AtomicI32::new(0));
            let next_peer_id_clone = Arc::clone(&next_peer_id);

            move |socket, addr| {
                let peer_id = next_peer_id_clone.fetch_add(1, Ordering::SeqCst);

                Server::ws_handle_connection(
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
        peers: Arc<Mutex<HashMap<i32, TcpPeer>>>,
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
                    let mut peers_guard = peers.lock().unwrap();
                    let peer = peers_guard.entry(peer_id).or_insert(TcpPeer {
                        sdn_ip_addr: Ipv4Addr::UNSPECIFIED,
                        status: HandshakeStatus::Pending,
                        tx: tx.clone(),
                    });
                    peer.status.clone()
                };

                match status {
                    HandshakeStatus::Initialized => {
                        if let Some(header) = parse_ipv4_header(&buf[..amt]) {
                            let peers_guard = peers.lock().unwrap();
                            for (&_peer_id, peer) in peers_guard.iter() {
                                if peer.sdn_ip_addr.to_string() == header.dst_ip.to_string() {
                                    peer.tx
                                        .send(bytes::Bytes::copy_from_slice(&buf[..amt]))
                                        .unwrap();
                                    break;
                                }
                            }
                        } else {
                            error!("Packet not supported");
                        }
                    }
                    HandshakeStatus::Pending => match HandshakeReq::deserialize(&buf[..amt]) {
                        Ok(handshake) => {
                            match common::crypto::verify_signed_key(handshake.auth_key) {
                                Ok(auth_client) => {
                                    let client = db.get_client(&auth_client.client_id).await;
                                    if let Ok(client) = client {
                                        {
                                            let mut peers_guard = peers.lock().unwrap();
                                            peers_guard.entry(peer_id).and_modify(|p| {
                                                p.sdn_ip_addr = Ipv4Addr::from_str(
                                                    &client.sdn_client_ip.as_str(),
                                                )
                                                .unwrap()
                                            });
                                        }
                                        info!("Client connected {} {}", addr, client.sdn_client_ip);
                                        let reply = HandshakeRep::new(
                                            &client.netmask,
                                            &client.network,
                                            &client.sdn_client_ip,
                                        );
                                        let mut reply_socket = socket.clone();
                                        match reply_socket
                                            .send(&reply.serialize().unwrap(), None)
                                            .await
                                        {
                                            Ok(_) => {
                                                let mut peers_guard = peers.lock().unwrap();
                                                peers_guard.entry(peer_id).and_modify(|p| {
                                                    p.status = HandshakeStatus::Initialized
                                                });
                                            }
                                            Err(_) => {
                                                error!("Send handhsake reply failed");
                                            }
                                        }
                                    } else {
                                        error!("Ignoring Unknown user {}", addr.ip());
                                    }
                                }
                                Err(error) => {
                                    error!(
                                        "Unexpected verifying key error: {} {}",
                                        addr.ip(),
                                        error
                                    );
                                }
                            }
                        }
                        Err(err) => {
                            error!("HandshakeReq failed: {}", err);
                        }
                    },
                }
            }
        });
        tokio::select! {
            _ = forward_task => {},
            _ = send_task => {},
        }
    }
}
