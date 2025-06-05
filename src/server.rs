use anyhow::Result;
use common::{HandshakeRep, HandshakeReq, HandshakeStatus};
use dotenv::dotenv;
use log::{debug, error, info};
use std::collections::{HashMap, HashSet};
use std::net::{Ipv4Addr, SocketAddr};
use std::str::FromStr;
use std::sync::Arc;
use tokio::signal::unix::{SignalKind, signal};

pub mod common;
pub mod crypto;
pub mod db;
pub mod packet;
pub mod tundev;
pub mod webserver;

use crate::packet::parse_ipv4_header;
use crate::webserver::WebServer;

#[derive(Eq, Hash, PartialEq)]
struct Client {
    pub src: SocketAddr,
    pub sdn_ip_addr: Ipv4Addr,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessError(u32);

async fn reticula_server(db: Arc<db::Db>) -> Result<()> {
    let server_addr = std::env::var("SERVER").unwrap_or("0.0.0.0:5000".to_string());
    let socket = tokio::net::UdpSocket::bind(server_addr).await?;
    let mut clients: HashSet<Client> = HashSet::new();
    let mut buf = [0; 1500];
    let mut clients_status: HashMap<SocketAddr, HandshakeStatus> = HashMap::new();

    loop {
        let (amt, src) = socket.recv_from(&mut buf).await?;
        debug!("BYTES received {}", amt);
        let status = clients_status
            .entry(src)
            .or_insert(HandshakeStatus::Pending);
        match status {
            HandshakeStatus::Initialized => {
                if let Some(header) = parse_ipv4_header(&buf[..amt]) {
                    debug!(
                        "{} {} {}",
                        header.src_ip, header.dst_ip, header.total_length
                    );

                    for client in &clients {
                        debug!("Sending data to {}", src);
                        if src != client.src {
                            // TODO this is broadcasting, parse IP header and send only to target
                            debug!("...relying");
                            socket
                                .send_to(&buf[..amt], &client.src).await?;

                        }
                    }
                } else {
                    error!("Packet not supported");
                }
            }
            HandshakeStatus::Pending => {
                match HandshakeReq::deserialize(&buf[..amt]) {
                    Ok(handshake) => {
                        info!("HandshakeReq received {}", src);
                        match crate::crypto::verify_signed_key(handshake.auth_key) {
                            Ok(auth_client) => {
                                if let Ok(client) = db.get_client(&auth_client.client_id).await {
                                    clients.insert(Client {
                                        src,
                                        sdn_ip_addr: Ipv4Addr::from_str(
                                            &client.sdn_client_ip.as_str(),
                                        )?,
                                    });
                                    info!("Client connected {} {}", src, client.sdn_client_ip);
                                    let netmask = String::from("255.255.255.0");
                                    let destination = String::from("12.0.0.0");
                                    let reply = HandshakeRep::new(
                                        &netmask,
                                        &destination,
                                        &client.sdn_client_ip,
                                    );
                                    match socket.send_to(&reply.serialize()?, &src).await {
                                        Ok(_) => {
                                            clients_status.insert(src, HandshakeStatus::Initialized);
                                        },
                                        Err(_) => {
                                            // TODO
                                        }
                                    }
                                } else {
                                    error!("Ignoring Unknown user {}", src.ip());
                                }
                            }
                            Err(error) => {
                                error!("Unexpected verifying key error: {} {}", src.ip(), error);
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

#[tokio::main]
async fn main() -> Result<(), ProcessError> {
    let mut sigterm = signal(SignalKind::terminate()).expect("Failed to bind SIGTERM handler");
    let mut sigint = signal(SignalKind::interrupt()).expect("Failed to bind SIGINT handler");

    tokio::spawn(async move {
        tokio::select! {
            _ = sigterm.recv() => {}
            _ = sigint.recv() => {}
        }
        info!("Shutting down...");
        // TODO shutdown gracefully
        std::process::exit(0);
    });

    env_logger::init();
    dotenv().ok();
    std::env::var("SECRET_KEY").expect("SECRET_KEY env var not found");

    let db = Arc::new(db::Db::new().await);
    let web_server = WebServer::new(db.clone()).await;

    info!("Starting reticula server");
    let reticula_server = tokio::spawn(reticula_server(db.clone()));
    info!("UDP server listening on 0.0.0.0:5000");

    tokio::select! {
        _ = web_server => info!("Web server stopped"),
        _ = reticula_server => info!("Reticula server stopped")
    }
    Ok(())
}
