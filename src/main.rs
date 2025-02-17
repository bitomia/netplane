use dotenv::dotenv;
use log::info;
use std::env;
use tokio::signal::unix::{signal, SignalKind};

pub mod client;
pub mod common;
pub mod db;
pub mod packet;
pub mod server;
pub mod tundev;
pub mod webserver;

fn echo_syntax(args: &Vec<String>) {
    println!(
        "Use {} [server|client] [tun_name] [destination] [netmask] [ip] [server_ip]",
        args[0]
    );
}

#[tokio::main]
async fn main() {
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

    colog::init();
    dotenv().ok();

    let args: Vec<String> = env::args().collect();
    if args.len() == 2 && args[1] == "server" {
        let _ = server::run().await;
    } else if args.len() == 7 && args[1] == "client" {
        let _ = client::run(
            args[2].clone(),
            args[3].clone(),
            args[4].clone(),
            args[5].clone(),
            args[6].clone(),
        )
        .await;
    } else {
        echo_syntax(&args);
        std::process::exit(1);
    }
}
