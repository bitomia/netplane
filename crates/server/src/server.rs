use anyhow::Result;
use log::{error, info};
use netplane_common::packet::{parse_ipv4_header, validate_packet};
use netplane_common::transport::{Transport, UdpTransport, WebSocketTransport};
use netplane_common::{HandshakeRep, HandshakeReq, HandshakeStatus};
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
    async fn process_handshake(
        handshake_req: HandshakeReq,
        db: &db::Db,
        client_addr: std::net::IpAddr,
    ) -> Result<(HandshakeRep, String)> {
        match netplane_common::crypto::verify_signed_key(handshake_req.auth_key) {
            Ok(auth_client) => {
                if let Ok(client) = db.get_client(&auth_client.client_id).await {
                    info!("Client connected {} {}", client_addr, client.sdn_client_ip);
                    let reply =
                        HandshakeRep::new(&client.netmask, &client.network, &client.sdn_client_ip);
                    Ok((reply, client.sdn_client_ip))
                } else {
                    error!("Ignoring Unknown user {}", client_addr);
                    Err(anyhow::anyhow!("Unknown user"))
                }
            }
            Err(error) => {
                error!("Unexpected verifying key error: {} {}", client_addr, error);
                Err(anyhow::anyhow!("Key verification failed: {}", error))
            }
        }
    }

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
                    if let Some(header) = parse_ipv4_header(&buf[..amt]) {
                        for (peer_addr, sdn_addr) in peers.clone() {
                            if src == peer_addr {
                                continue;
                            }
                            if header.dst_ip == sdn_addr.sdn_ip_addr.to_string()
                                || peer_addr.ip().is_multicast()
                            {
                                transport.send(&buf[..amt], Some(&peer_addr)).await?;
                            }
                        }
                    }
                }
                HandshakeStatus::Pending => {
                    match HandshakeReq::deserialize(&buf[..amt]) {
                        Ok(handshake) => {
                            match Self::process_handshake(handshake, &self.db, src.ip()).await {
                                Ok((reply, sdn_client_ip)) => {
                                    peer.sdn_ip_addr = Ipv4Addr::from_str(&sdn_client_ip)?;
                                    match transport.send(&reply.serialize()?, Some(&src)).await {
                                        Ok(_) => {
                                            peer.status = HandshakeStatus::Initialized;
                                        }
                                        Err(_) => {
                                            // TODO
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
                        if validate_packet(&buf[..amt]) {
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
                            }
                        }
                    }
                    HandshakeStatus::Pending => match HandshakeReq::deserialize(&buf[..amt]) {
                        Ok(handshake) => {
                            match Self::process_handshake(handshake, &db, addr.ip()).await {
                                Ok((reply, sdn_client_ip)) => {
                                    {
                                        let mut peers_guard = peers.lock().unwrap();
                                        peers_guard.entry(peer_id).and_modify(|p| {
                                            p.sdn_ip_addr =
                                                Ipv4Addr::from_str(&sdn_client_ip).unwrap()
                                        });
                                    }
                                    let mut reply_socket = socket.clone();
                                    match reply_socket.send(&reply.serialize().unwrap(), None).await
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
                                }
                                Err(_) => {
                                    // Error already logged in process_handshake
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
