use log::{error, info};
use netplane_common::{
    transport::TransportMode, HandshakeError, HandshakeRep, HandshakeReq, PeerList,
};
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use crate::db;
use crate::peers::*;

#[derive(Debug)]
pub enum HandshakeResult {
    /// Success with reply, SDN client IP, client public key, and is_exit_node
    Success(HandshakeRep, String, String, bool),
    Error(HandshakeError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessError(u32);

#[derive(Debug)]
pub struct ServerStats {
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

    pub fn add_in_bytes(self: &Self, nbytes: usize) -> usize {
        self.in_bytes.fetch_add(nbytes, Ordering::Relaxed)
    }

    pub fn add_out_bytes(self: &Self, nbytes: usize) -> usize {
        self.out_bytes.fetch_add(nbytes, Ordering::Relaxed)
    }
}

pub struct Server<PeerKey> {
    pub peers: Peers<PeerKey>,
    pub db: Arc<db::Db>,
    pub stats: Arc<ServerStats>,
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
                        let exit_node_sdn_ip = db
                            .get_exit_node_sdn_ip(&client.id)
                            .await
                            .unwrap_or(None);
                        let reply = HandshakeRep::new(
                            &client.netmask,
                            &client.network,
                            &client.sdn_client_ip,
                        )
                        .with_exit_node(client.is_exit_node, exit_node_sdn_ip);
                        HandshakeResult::Success(
                            reply,
                            client.sdn_client_ip,
                            client_pub_key,
                            client.is_exit_node,
                        )
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
