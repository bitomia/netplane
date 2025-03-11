use std::env;
use tokio::signal::unix::{signal, SignalKind};

pub mod client;
pub mod common;
pub mod db;
pub mod packet;
pub mod server;
pub mod tundev;
pub mod webserver;

fn echo_syntax(args: &[String]) {
    println!(
        "Use {} [server|client] [tun_name] [destination] [netmask] [ip] [server_ip]",
        args[0]
    );
}

#[tokio::main]
async fn main() {
    env_logger::init();

    let mut sigterm = signal(SignalKind::terminate()).expect("Failed to bind SIGTERM handler");
    let mut sigint = signal(SignalKind::interrupt()).expect("Failed to bind SIGINT handler");
    tokio::spawn(async move {
        tokio::select! {
            _ = sigterm.recv() => {
                std::process::exit(0);
            },
                _ = sigint.recv() => {
                std::process::exit(0);
            }
        }
    });

    let args: Vec<String> = env::args().collect();
    if args.len() == 2 && args[1] == "server" {
        let _ = server::start().await;
    } else if args.len() == 7 && args[1] == "client" {
        let _ = client::start(&args[2], &args[3], &args[4], &args[5], &args[6]).await;
    } else {
        echo_syntax(&args);
        std::process::exit(1);
    }
}

