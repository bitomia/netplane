use anyhow::Result;
use common::packet::parse_ipv4_header;
use common::transport::{Transport, UdpTransport, WebSocketTransport};
use common::{HandshakeRep, HandshakeReq, HandshakeStatus};
use dotenv::dotenv;
use log::{debug, error, info};
use sqlx::sqlite::SqlitePoolOptions;
use std::collections::{HashMap, HashSet};
use std::net::{Ipv4Addr, SocketAddr};
use std::path::Path as FilePath;
use std::str::FromStr;
use std::sync::Arc;
use tokio::signal::unix::{SignalKind, signal};

mod db;
mod webserver;

use crate::webserver::WebServer;

#[derive(Eq, Hash, PartialEq)]
struct Client {
    pub src: SocketAddr,
    pub sdn_ip_addr: Ipv4Addr,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessError(u32);

async fn reticula_server(listen_addr: String, db: Arc<db::Db>) -> Result<()> {
    WebSocketTransport::bind(&listen_addr, {
        let db = Arc::clone(&db);
        move |socket, addr| handle_connection(socket, addr, Arc::clone(&db))
    })
    .await;
    Ok(())
}

async fn handle_connection(mut socket: WebSocketTransport, addr: SocketAddr, db: Arc<db::Db>) {
    let mut clients: HashSet<Client> = HashSet::new();
    let mut buf = [0; 1500];
    let mut clients_status: HashMap<SocketAddr, HandshakeStatus> = HashMap::new();

    loop {
        let (amt, src) = socket.recv(&mut buf).await.unwrap();

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
                            socket.send(&buf[..amt], Some(&client.src)).await;
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
                        match common::crypto::verify_signed_key(handshake.auth_key) {
                            Ok(auth_client) => {
                                if let Ok(client) = db.get_client(&auth_client.client_id).await {
                                    clients.insert(Client {
                                        src,
                                        sdn_ip_addr: Ipv4Addr::from_str(
                                            &client.sdn_client_ip.as_str(),
                                        )
                                        .unwrap(),
                                    });
                                    info!("Client connected {} {}", src, client.sdn_client_ip);
                                    let netmask = String::from("255.255.255.0");
                                    let destination = String::from("12.0.0.0");
                                    let reply = HandshakeRep::new(
                                        &netmask,
                                        &destination,
                                        &client.sdn_client_ip,
                                    );
                                    match socket.send(&reply.serialize().unwrap(), Some(&src)).await
                                    {
                                        Ok(_) => {
                                            clients_status
                                                .insert(src, HandshakeStatus::Initialized);
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
    let reticula_server = tokio::spawn(reticula_server(listen_addr.clone(), db.clone()));
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
