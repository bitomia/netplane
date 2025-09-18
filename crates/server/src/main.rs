use anyhow::Error;
use axum::{Router, serve::Serve};
use dotenv::dotenv;
use env_logger::Env;
use log::info;
use netplane_common::transport::TransportMode;
use std::sync::Arc;
use tokio::signal::unix::{SignalKind, signal};
use tokio::task::JoinHandle;

mod db;
mod dnsserver;
mod handlers;
mod peers;
mod server;
mod udpserver;
mod webserver;
mod wsserver;

use crate::dnsserver::DnsServer;
use crate::server::ProcessError;
use crate::udpserver::UdpServer;
use crate::webserver::WebServer;
use crate::wsserver::WebSocketServer;

fn echo_syntax(args: &Vec<String>) {
    println!("Use {} [--migrate] [--transport=udp|websocket]", args[0]);
}

fn try_start_dns_server(db: Arc<crate::db::Db>) -> Option<JoinHandle<Result<(), Error>>> {
    let dns_address: Result<String, _> = std::env::var("DNS_ADDRESS");
    if dns_address.is_ok() {
        let mut dns_server = DnsServer::new(Arc::clone(&db));
        let dns_server_task =
            tokio::spawn(async move { dns_server.start(dns_address.unwrap()).await });

        return Some(dns_server_task);
    } else {
        return None;
    }
}

async fn try_start_web_server(
    db: Arc<crate::db::Db>,
    server_stats: Arc<crate::server::ServerStats>,
) -> Option<Serve<tokio::net::TcpListener, Router, Router>> {
    let is_web_server_enabled = std::env::var("WEBSERVER_ENABLED").unwrap_or("true".to_string());
    if is_web_server_enabled == "true" {
        let web_server = WebServer::new(Arc::clone(&db), Arc::clone(&server_stats));
        Some(web_server.await)
    } else {
        None
    }
}

fn start_netplane_server(
    db: Arc<crate::db::Db>,
    server_stats: Arc<server::ServerStats>,
    transport_mode: TransportMode,
) -> JoinHandle<Result<(), Error>> {
    let netplane_server = tokio::spawn(async move {
        match transport_mode {
            TransportMode::WebSocket => {
                WebSocketServer::new(Arc::clone(&db), Arc::clone(&server_stats))
                    .start()
                    .await
            }
            _ => {
                UdpServer::new(Arc::clone(&db), Arc::clone(&server_stats))
                    .start()
                    .await
            }
        }
    });
    netplane_server
}

#[tokio::main]
async fn main() -> Result<(), ProcessError> {
    dotenv().ok();
    env_logger::Builder::from_env(Env::default().default_filter_or("debug")).init();

    info!(
        "Starting netplane server ({})",
        netplane_common::git_rev_main!()
    );

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

    let args: Vec<String> = std::env::args().collect();
    let mut transport_mode =
        TransportMode::from_string(std::env::var("TRANSPORT").unwrap_or("UDP".to_string()))
            .expect("Invalid transport mode");
    let mut should_migrate = false;

    // Parse command line arguments
    for arg in &args[1..] {
        if arg == "--migrate" {
            should_migrate = true;
        } else if arg.starts_with("--transport=") {
            transport_mode =
                TransportMode::from_string(arg.split('=').nth(1).unwrap_or("UDP").to_string())
                    .expect("Invalid transport mode");
        } else if arg.starts_with("--") {
            echo_syntax(&args);
            std::process::exit(1);
        }
    }

    if should_migrate {
        db::do_migrate().await;
        return Ok(());
    }

    let db = Arc::new(db::Db::new().await);
    let server_stats = Arc::new(server::ServerStats::new(transport_mode.clone()));

    let webserver = try_start_web_server(Arc::clone(&db), Arc::clone(&server_stats)).await;
    let dnsserver = try_start_dns_server(Arc::clone(&db));

    tokio::select! {
        _ = async { webserver.unwrap().await }, if webserver.is_some() => { info!("Web server stopped") }
        _ = async { dnsserver.unwrap().await }, if dnsserver.is_some() => { info!("DNS server stopped") }
        _ = start_netplane_server(Arc::clone(&db), Arc::clone(&server_stats), transport_mode) => info!("Netplane server stopped")
    }
    Ok(())
}
