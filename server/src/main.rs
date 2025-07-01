use dotenv::dotenv;
use log::info;
use sqlx::sqlite::SqlitePoolOptions;
use std::path::Path as FilePath;
use std::sync::Arc;
use tokio::signal::unix::{SignalKind, signal};

mod db;
mod handlers;
mod peers;
mod server;
mod webserver;

use crate::server::{ProcessError, Server};
use crate::webserver::WebServer;

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
    let mut reticula_server = Server::new(
        Arc::clone(&db),
        &std::env::var("TRANSPORT").unwrap_or("UDP".to_string()),
    );
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
