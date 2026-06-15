use anyhow::Error;
use dotenv::dotenv;
use netplane_common::crypto;
use netplane_common::transport::TransportMode;
use std::sync::Arc;
#[cfg(unix)]
use tokio::signal::unix::{SignalKind, signal};
use tokio::task::JoinHandle;
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;

mod db;
mod dnsserver;
mod handlers;
mod peers;
mod server;
mod source;
mod trafficlog;
mod udpserver;
mod webserver;
mod wsserver;

use crate::dnsserver::DnsServer;
use crate::server::ProcessError;
use crate::webserver::WebServer;

fn echo_syntax(msg: Option<String>, args: &[String]) {
    if let Some(msg) = msg {
        print!("{}. ", msg);
    }
    println!(
        "Use {} [--migrate] [--create-user=<email>] [--transport=udp|websocket] [--dump=<file>] [--replay=<file> --replay-delay=<seconds>] [--dynamic-clients=<link-key>]",
        args[0]
    );
    println!("\nOptions:");
    println!("  --migrate              Run database migrations");
    println!("  --create-user=<email>  Create or update a user");
    println!("  --transport=<mode>     Transport mode: udp or websocket (default: udp)");
    println!("  --dump=<file>          Dump traffic to file");
    println!("  --replay=<file>        Replay traffic from dump file (requires --replay-delay)");
    println!(
        "  --replay-delay=<sec>   Seconds to wait before replaying (allows clients to connect)"
    );
    println!(
        "  --dynamic-clients=<auth_key>  Allow clients with this auth key to be assigned an IP dynamically (default: disabled)"
    );
}

fn try_start_dns_server(db: Arc<crate::db::Db>) -> Option<JoinHandle<Result<(), Error>>> {
    let dns_address: Result<String, _> = std::env::var("DNS_ADDRESS");
    if dns_address.is_ok() {
        let mut dns_server = DnsServer::new(Arc::clone(&db));
        let dns_server_task =
            tokio::spawn(async move { dns_server.start(dns_address.unwrap()).await });

        Some(dns_server_task)
    } else {
        None
    }
}

#[tokio::main]
async fn main() -> Result<(), ProcessError> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();
    info!("Netplane server rev {}", netplane_common::git_rev_main!());

    dotenv().ok();
    crypto::check_env();

    #[cfg(unix)]
    {
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
    }

    #[cfg(not(unix))]
    {
        tokio::spawn(async move {
            match tokio::signal::ctrl_c().await {
                Ok(()) => {
                    info!("Shutting down...");
                    // TODO shutdown gracefully
                    std::process::exit(0);
                }
                Err(err) => {
                    eprintln!("Unable to listen for shutdown signal: {}", err);
                }
            }
        });
    }

    let args: Vec<String> = std::env::args().collect();
    let mut transport_mode =
        TransportMode::from_string(std::env::var("TRANSPORT").unwrap_or("UDP".to_string()))
            .expect("Invalid transport mode");
    let mut should_migrate = false;
    let mut create_user_email: Option<String> = None;
    let mut dump_file: Option<String> = None;
    let mut replay_file: Option<String> = None;
    let mut replay_delay: Option<u64> = None;
    let mut dynamic_clients_key: Option<String> = None;

    // Parse command line arguments
    for arg in &args[1..] {
        if arg == "--migrate" {
            should_migrate = true;
        } else if arg.starts_with("--create-user=") {
            create_user_email = Some(arg.split('=').nth(1).unwrap_or("").to_string());
        } else if arg.starts_with("--transport=") {
            transport_mode =
                TransportMode::from_string(arg.split('=').nth(1).unwrap_or("UDP").to_string())
                    .expect("Invalid transport mode");
        } else if arg.starts_with("--dump=") {
            dump_file = Some(arg.split('=').nth(1).unwrap_or("").to_string());
        } else if arg.starts_with("--replay=") {
            replay_file = Some(arg.split('=').nth(1).unwrap_or("").to_string());
        } else if arg.starts_with("--replay-delay=") {
            let delay_str = arg.split('=').nth(1).unwrap_or("0");
            replay_delay = Some(delay_str.parse().expect("Invalid replay delay value"));
        } else if arg.starts_with("--dynamic-clients=") {
            dynamic_clients_key = Some(arg.split('=').nth(1).unwrap_or("").to_string());
        } else if arg.starts_with("--help") {
            echo_syntax(None, &args);
            std::process::exit(1);
        } else {
            echo_syntax(Some("Argument not recognized".to_string()), &args);
            std::process::exit(1);
        }
    }

    if should_migrate {
        db::do_migrate().await;
        return Ok(());
    }

    if let Some(email) = create_user_email {
        if email.is_empty() {
            echo_syntax(Some("Error: Email cannot be empty".to_string()), &args);
            std::process::exit(1);
        }

        let password = rpassword::prompt_password(format!("Enter password for {}: ", email))
            .expect("Failed to read password");

        if password.is_empty() {
            println!("Error: Password cannot be empty");
            std::process::exit(1);
        }

        let confirm_password = rpassword::prompt_password("Confirm password: ".to_string())
            .expect("Failed to read password confirmation");

        if password != confirm_password {
            println!("Error: Passwords do not match");
            std::process::exit(1);
        }

        let db = db::Db::new().await;
        let password_hash =
            bcrypt::hash(password, bcrypt::DEFAULT_COST).expect("Failed to hash password");
        match db.create_or_update_user(&email, &password_hash).await {
            Ok(is_new_user) => {
                if is_new_user {
                    info!("User created successfully: {}", email);
                    println!("User created successfully:");
                } else {
                    info!("User password updated successfully: {}", email);
                    println!("User password updated successfully:");
                }
                println!("Email: {}", email);
            }
            Err(e) => {
                info!("Failed to create/update user: {}", e);
                println!("Failed to create/update user: {}", e);
                std::process::exit(1);
            }
        }
        return Ok(());
    }

    if let Err(err) =
        netplane_common::crypto::try_generate_crypto_keys("server_public.key", "server_private.key")
    {
        warn!("Crypto keys generation: {}", err.to_string());
    }

    if dynamic_clients_key.is_some() {
        info!("Dynamic linking of clients enabled")
    }

    let db = Arc::new(db::Db::new().await);
    let server_stats = Arc::new(server::ServerStats::new(transport_mode.clone()));
    let dnsserver = try_start_dns_server(Arc::clone(&db));

    tokio::select! {
        _ = async { dnsserver.unwrap().await }, if dnsserver.is_some() => info!("DNS server stopped"),
        _ = WebServer::new(Arc::clone(&db), Arc::clone(&server_stats), dynamic_clients_key.clone()).await => info!("Web server stopped"),
        _ = server::run(Arc::clone(&db), Arc::clone(&server_stats), transport_mode, dump_file, replay_file, replay_delay, dynamic_clients_key) => info!("Netplane server stopped")
    }
    Ok(())
}
