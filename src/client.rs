use anyhow::{Result, anyhow};
use crypto::load_auth_key;
use dotenv::dotenv;
use log::{debug, error, info};
use reqwest::Client;
use std::env;
use tokio::net::UdpSocket;
use tokio::signal::unix::{SignalKind, signal};
//use tray_item::{TrayItem, IconSource};
use axum::http::StatusCode;
use common::{HandshakeRep, HandshakeReq};
pub mod common;
pub mod crypto;
pub mod packet;
pub mod tundev;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessError(u32);

async fn send_tun(dev: &mut tundev::TunDev, buf: &[u8], nbytes: usize) {
    match dev.send(&buf[..nbytes], nbytes).await {
        Ok(_) => {}
        Err(err) => error!("{}", err),
    }
}

#[derive(Debug)]
struct StartParams {
    netmask: String,
    destination: String,
    ip_addr: String,
}

async fn handshake(
    auth_key: String,
    server_addr: String,
    socket: &UdpSocket,
) -> Result<StartParams> {
    info!("Starting handshake with {}", server_addr);
    socket.connect(server_addr.clone()).await?;

    let handshake = HandshakeReq::new(&auth_key);
    socket.send(&handshake.serialize()?).await?;

    let mut socket_buf = [0; 1500];
    loop {
        let (amt, _) = socket.recv_from(&mut socket_buf).await?;
        if let Ok(handshake) = HandshakeRep::deserialize(&socket_buf[..amt]) {
            return Ok(StartParams {
                netmask: handshake.netmask,
                destination: handshake.destination,
                ip_addr: handshake.sdn_ip_addr,
            });
        } else {
            error!("Initialization failed. Keep trying");
        }
    }
}

pub async fn run(tun_dev: String, server_addr: String) -> Result<()> {
    info!("Starting client");
    let auth_key = load_auth_key()?;

    let socket = UdpSocket::bind("0.0.0.0:0")
        .await
        .expect("Cannot open socket");

    info!(
        "Client bound to {:?}",
        socket.local_addr().expect("Cannot get the local addr")
    );

    // let icon_raw = include_bytes!("../icons/icon-red.ico");
    // let connected_icon_raw = include_bytes!("../icons/icon-green.ico");
    // let mut tray = TrayItem::new("Tray Example", IconSource::Data { height: 64, width: 64, data: Vec::from(icon_raw) }).unwrap();
    // tray.add_label("Tray Label").unwrap();
    // tray.add_menu_item("Hello", || {
    //     println!("Hello!");
    // }).unwrap();
    // let mut inner = tray.inner_mut();
    // inner.add_quit_item("Quit");
    // inner.display();

    let start_params = match handshake(auth_key, server_addr, &socket).await {
        Ok(p) => {
            info!("Handshake successfully finished {:?}", p);
            p
        }
        Err(err) => {
            error!("Handshake failed: {}", err);
            std::process::exit(1)
        }
    };

    let mut socket_buf = [0; 1500];
    let mut tun_buf = [0; 1500];

    let mut dev = tundev::TunDev::new(
        tun_dev,
        start_params.netmask.as_str(),
        start_params.destination.as_str(),
        start_params.ip_addr.as_str(),
    );

    loop {
        tokio::select! {
            result = socket.recv_from(&mut socket_buf) => {
                match result {
                    Ok((amt, _)) => {
                        // if let Some(header) = packet::parse_ipv4_header(&socket_buf[..amt]) {
                        //     debug!(
                        //         "{} {} {}",
                        //         header.src_ip, header.dst_ip, header.total_length
                        //     );
                        // }
                        send_tun(&mut dev, &socket_buf, amt).await;
                    },
                    Err(_) => todo!()
                }
            },
            tun_ret = dev.read(&mut tun_buf) => {
                match tun_ret {
                    Ok(amt) => {
                        let mut is_loopback = false;
                        if let Some(header) = packet::parse_ipv4_header(&tun_buf[..amt]) {
                            debug!("{} {}", header.src_ip, header.dst_ip);
                            is_loopback =
                            header.src_ip == header.dst_ip && header.src_port == header.dst_port;
                        }
                        if is_loopback {
                            send_tun(&mut dev, &tun_buf, amt).await;
                        } else {
                            match socket.send(&tun_buf[..amt]).await {
                                Ok(bytes_sent) => {
                                    if bytes_sent != amt {
                                        error!("Less bytes sent than expected to socket");
                                    }
                                },
                                Err(err) => error!("{}´", err)
                            }
                        }
                    }
                    Err(err) => {
                        info!("{}", err);
                    }
                }
            }
        }
    }
}

fn echo_syntax(args: &Vec<String>) {
    println!("Use {} [tun_dev] [server_ip] [--auth=link]", args[0]);
}

async fn auth_client(arg: String) -> Result<String> {
    let parts: Vec<&str> = arg.split("=").collect();
    if parts.len() != 2 {
        return Err(anyhow!("Invalid auth argument"));
    }

    let (public_key, _) = crate::crypto::try_load_crypto_keys("public.key", "private.key")?;
    let payload = crate::common::AuthClientRequest { public_key };

    let res = Client::new()
        .post(parts[1].to_string())
        .json(&payload)
        .send()
        .await?;

    match res.status() {
        StatusCode::OK => {
            let auth_key = res.text().await?;
            Ok(auth_key)
        }
        _ => {
            if let Ok(text) = res.text().await {
                Err(anyhow!(format!("Auth failed: {}", text)))
            } else {
                Err(anyhow!(format!("Auth failed: Unknown error")))
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
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

    let args: Vec<String> = env::args().collect();
    if args.len() >= 3 {
        if let Err(err) = crypto::try_generate_crypto_keys("public.key", "private.key") {
            if err.kind() != std::io::ErrorKind::AlreadyExists {
                return Err(anyhow!(err));
            }
        }
        if args.len() == 4 {
            let auth_key = auth_client(args[3].clone()).await?;
            println!("{}", auth_key);
            std::fs::write("auth.key", auth_key)?;
        }
        let _ = run(args[1].clone(), args[2].clone()).await?;
    } else {
        echo_syntax(&args);
        std::process::exit(1);
    }
    Ok(())
}
