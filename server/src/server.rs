use anyhow::Result;
use common::packet::parse_ipv4_header;
use common::transport::{Transport, WebSocketTransport};
use common::{HandshakeRep, HandshakeReq, HandshakeStatus};
use dotenv::dotenv;
use log::{debug, error, info};
use sqlx::sqlite::SqlitePoolOptions;
use std::collections::{HashMap, HashSet};
use std::net::{Ipv4Addr, SocketAddr};
use std::path::Path as FilePath;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use tokio::signal::unix::{SignalKind, signal};

mod db;
mod webserver;

use crate::webserver::WebServer;

#[derive(Clone, Eq, Hash, PartialEq)]
struct Client {
    pub src: SocketAddr,
    pub sdn_ip_addr: Ipv4Addr,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessError(u32);

struct Server {
    clients: Arc<Mutex<HashSet<Client>>>,
    clients_status: Arc<Mutex<HashMap<SocketAddr, HandshakeStatus>>>,
    db: Arc<db::Db>,
}

impl Server {
    pub fn new(db: Arc<db::Db>) -> Server {
        Server {
            clients: Arc::new(Mutex::new(HashSet::<Client>::new())),
            clients_status: Arc::new(Mutex::new(HashMap::<SocketAddr, HandshakeStatus>::new())),
            db,
        }
    }

    pub async fn start(self: &Self) -> Result<()> {
        let listen_addr = std::env::var("SERVER").unwrap_or("0.0.0.0:5000".to_string());
        WebSocketTransport::bind(&listen_addr, {
            let db = Arc::clone(&self.db);
            let clients = Arc::clone(&self.clients);
            let clients_status = Arc::clone(&self.clients_status);

            move |socket, addr| {
                Server::handle_connection(
                    socket,
                    addr,
                    Arc::clone(&db),
                    Arc::clone(&clients),
                    Arc::clone(&clients_status),
                )
            }
        })
        .await;
        Ok(())
    }

    async fn handle_connection(
        mut socket: WebSocketTransport,
        addr: SocketAddr,
        db: Arc<db::Db>,
        clients: Arc<Mutex<HashSet<Client>>>,
        clients_status: Arc<Mutex<HashMap<SocketAddr, HandshakeStatus>>>,
    ) {
        let mut buf = [0; 1500];

        loop {
            let (amt, _) = socket.recv(&mut buf).await.unwrap();

            let status = {
                let mut status_map = clients_status.lock().unwrap();
                status_map
                    .entry(addr)
                    .or_insert(HandshakeStatus::Pending)
                    .clone()
            };

            match status {
                HandshakeStatus::Initialized => {
                    if let Some(header) = parse_ipv4_header(&buf[..amt]) {
                        debug!(
                            "{} {} {}",
                            header.src_ip, header.dst_ip, header.total_length
                        );

                        let clients = {
                            let clients = clients.lock().unwrap();
                            clients.clone()
                        };
                        for client in clients {
                            debug!("Sending data to {} {}", header.dst_ip, client.sdn_ip_addr);
                            if addr.to_string() == header.dst_ip.to_string() {
                                debug!("...relying");
                                let _ = socket.send(&buf[..amt], None).await;
                            }
                        }
                    } else {
                        error!("Packet not supported");
                    }
                }
                HandshakeStatus::Pending => {
                    match HandshakeReq::deserialize(&buf[..amt]) {
                        Ok(handshake) => {
                            info!("HandshakeReq received {}", addr);
                            match common::crypto::verify_signed_key(handshake.auth_key) {
                                Ok(auth_client) => {
                                    let client = db.get_client(&auth_client.client_id).await;
                                    if let Ok(client) = client {
                                        clients.lock().unwrap().insert(Client {
                                            src: addr,
                                            sdn_ip_addr: Ipv4Addr::from_str(
                                                &client.sdn_client_ip.as_str(),
                                            )
                                            .unwrap(),
                                        });
                                        info!("Client connected {} {}", addr, client.sdn_client_ip);
                                        let netmask = String::from("255.255.255.0");
                                        let destination = String::from("12.0.0.0");
                                        let reply = HandshakeRep::new(
                                            &netmask,
                                            &destination,
                                            &client.sdn_client_ip,
                                        );
                                        match socket.send(&reply.serialize().unwrap(), None).await {
                                            Ok(_) => {
                                                clients_status
                                                    .lock()
                                                    .unwrap()
                                                    .insert(addr, HandshakeStatus::Initialized);
                                            }
                                            Err(_) => {
                                                // TODO
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
                    }
                }
            }
        }
    }
}

async fn do_migrate() {
    let db_file_path = std::env::var("DATABASE_URL").unwrap();
    let db_path = db_file_path.replace("sqlite://", "");

    if !FilePath::new(&db_path).exists() {
        info!("Database file not found, creating...");
        std::fs::File::create(&db_path).expect("Failed to create SQLite file");
    }
    let pool = SqlitePoolOptions::new()
        .connect(&db_file_path)
        .await
        .expect("Cannot connect to database");
    sqlx::migrate!("./src/migrations")
        .run(&pool)
        .await
        .expect("Migration failed");
    println!("Migration successfully finished");
}

fn echo_syntax(args: &Vec<String>) {
    println!("Use {} [--migrate]", args[0]);
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

    let args: Vec<String> = std::env::args().collect();
    if args.len() == 2 {
        if args[1] == "--migrate" {
            do_migrate().await;
            return Ok(());
        } else {
            echo_syntax(&args);
            std::process::exit(1);
        }
    }

    let db = Arc::new(db::Db::new().await);

    info!("Starting reticula server");
    let listen_addr = std::env::var("SERVER").unwrap_or("0.0.0.0:5000".to_string());
    let reticula_server = Server::new(Arc::clone(&db));
    let reticula_server = tokio::spawn(async move { reticula_server.start().await });
    info!("TCP server listening on {}", listen_addr);

    let is_webserver_enabled = std::env::var("WEBSERVER_ENABLED").unwrap_or("true".to_string());
    if is_webserver_enabled == "true" {
        let web_server = WebServer::new(db.clone()).await;
        tokio::select! {
            _ = web_server => info!("Web server stopped"),
            _ = reticula_server => info!("Reticula server stopped")
        }
    } else {
        tokio::select! {
            _ = reticula_server => info!("Reticula server stopped")
        }
    }
    Ok(())
}
