use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use anyhow::Error;
use tokio::task::JoinHandle;
use tracing::{error, info};

use netplane_common::{HandshakeError, HandshakeRep, HandshakeReq, transport::TransportMode};

use crate::db;
use crate::peers::*;
use crate::udpserver::UdpServer;
use crate::wsserver::WebSocketServer;

#[derive(Debug)]
pub enum HandshakeResult {
    /// Success with reply, SDN client IP, and client public key
    Success(HandshakeRep, String, String),
    Error(HandshakeError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub struct ProcessError(u32);

#[derive(Debug)]
pub struct ServerStats {
    #[allow(dead_code)]
    pub transport_mode: TransportMode,
    pub in_bytes: AtomicUsize,
    pub out_bytes: AtomicUsize,
}

impl ServerStats {
    pub fn new(transport_mode: TransportMode) -> ServerStats {
        ServerStats {
            transport_mode,
            in_bytes: AtomicUsize::new(0),
            out_bytes: AtomicUsize::new(0),
        }
    }

    pub fn add_in_bytes(&self, nbytes: usize) -> usize {
        self.in_bytes.fetch_add(nbytes, Ordering::Relaxed)
    }

    pub fn add_out_bytes(&self, nbytes: usize) -> usize {
        self.out_bytes.fetch_add(nbytes, Ordering::Relaxed)
    }
}

pub struct Server<PeerKey> {
    pub peers: Peers<PeerKey>,
    pub db: Arc<db::Db>,
    pub stats: Arc<ServerStats>,
    pub dynamic_clients_key: Option<String>,
}

impl<PeerKey> Server<PeerKey> {
    /// Process client handshake - verify auth and return network config
    pub async fn process_handshake(
        handshake_req: HandshakeReq,
        db: &db::Db,
        client_addr: std::net::IpAddr,
    ) -> HandshakeResult {
        match netplane_common::crypto::verify_signed_key(handshake_req.auth_key) {
            Ok(auth_client) => {
                if let Ok(client) = db.get_client(&auth_client.client_id).await {
                    info!("Client verified {} {}", client_addr, client.sdn_client_ip);

                    // Client must provide public key for E2E encryption with other clients
                    if let Some(client_pub_key) = handshake_req.client_public_key {
                        let reply = HandshakeRep::new(
                            &client.netmask,
                            &client.network,
                            &client.sdn_client_ip,
                        );
                        HandshakeResult::Success(reply, client.sdn_client_ip, client_pub_key)
                    } else {
                        error!("Client did not provide public key - required for E2E encryption");
                        HandshakeResult::Error(HandshakeError::new(
                            "Client must provide public key for E2E encryption",
                        ))
                    }
                } else {
                    error!("Authorization failed: Unknown user {}", client_addr);
                    HandshakeResult::Error(HandshakeError::new(
                        "Authorization failed: Unknown user",
                    ))
                }
            }
            Err(error) => {
                error!(
                    "Authorization failed: Key verification error: {} {}",
                    client_addr, error
                );
                HandshakeResult::Error(HandshakeError::new(&format!(
                    "Authorization failed: {}",
                    error
                )))
            }
        }
    }
}

pub fn run(
    db: Arc<crate::db::Db>,
    server_stats: Arc<ServerStats>,
    transport_mode: TransportMode,
    dump_file: Option<String>,
    replay_file: Option<String>,
    replay_delay: Option<u64>,
    dynamic_clients_key: Option<String>,
) -> JoinHandle<Result<(), Error>> {
    tokio::spawn(async move {
        match transport_mode {
            TransportMode::WebSocket => {
                WebSocketServer::new(
                    Arc::clone(&db),
                    Arc::clone(&server_stats),
                    dynamic_clients_key,
                )
                .start()
                .await
            }
            _ => {
                UdpServer::new(
                    Arc::clone(&db),
                    Arc::clone(&server_stats),
                    dump_file,
                    replay_file,
                    replay_delay,
                    dynamic_clients_key,
                )
                .start()
                .await
            }
        }
    })
}
