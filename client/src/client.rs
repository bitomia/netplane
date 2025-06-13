use anyhow::{Result, anyhow};
use common::{crypto::load_auth_key, transport::WebSocketTransport};
use dotenv::dotenv;
use http_post::http_post_json;
use log::{debug, error, info};
use std::env;

use tokio::signal::unix::{SignalKind, signal};
//use tray_item::{TrayItem, IconSource};
use common::{HandshakeRep, HandshakeReq, transport::Transport};

pub mod http_post;
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

async fn handshake<T: Transport>(
    auth_key: String,
    server_addr: String,
    transport: &mut T,
) -> Result<StartParams> {
    info!("Starting handshake with {}", server_addr);

    let handshake = HandshakeReq::new(&auth_key);
    transport.send(&handshake.serialize()?, None).await?;

    let mut socket_buf = [0; 1500];
    loop {
        let (amt, _) = transport.recv(&mut socket_buf).await?;
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

pub async fn run(tun_dev: String, control_addr: String) -> Result<()> {
    info!("Starting client");
    let auth_key = load_auth_key()?;

    let mut transport = WebSocketTransport::connect(control_addr.as_str())
        .await
        .expect("Cannot open WebSocket");

    info!("Client connected to control");

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

    let start_params = match handshake(auth_key, control_addr, &mut transport).await {
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

    debug!("{:?}", start_params);
    let mut dev = tundev::TunDev::new(
        tun_dev,
        start_params.netmask.as_str(),
        start_params.destination.as_str(),
        start_params.ip_addr.as_str(),
    );

    loop {
        tokio::select! {
            result = transport.recv(&mut socket_buf) => {
                match result {
                    Ok(amt) => {
                        if let Some(header) = packet::parse_ipv4_header(&socket_buf[..amt]) {
                            debug!(
                                "{} {} {}",
                                header.src_ip, header.dst_ip, header.total_length
                            );
                        }
                        send_tun(&mut dev, &socket_buf, amt.0).await;
                    },
                    Err(_) => todo!()
                }
            },
            tun_ret = dev.read(&mut tun_buf) => {
                match tun_ret {
                    Ok(amt) => {
                        let mut is_loopback = false;
                        if let Some(header) = packet::parse_ipv4_header(&tun_buf[..amt]) {
                            is_loopback = header.src_ip == header.dst_ip;
                            debug!("> {} {} lo={:?}", header.src_ip, header.dst_ip, is_loopback);
                        }
                        if is_loopback {
                            send_tun(&mut dev, &tun_buf, amt).await;
                        } else {
                            match transport.send(&tun_buf[..amt], None).await {
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

    let (public_key, _) = common::crypto::try_load_crypto_keys("public.key", "private.key")?;
    let payload = common::AuthClientRequest { public_key };

    let res = http_post_json(parts[1], &payload)?;
    match res.status_code {
        axum::http::StatusCode::OK => {
            let auth_key = res.payload;
            Ok(auth_key)
        }
        _ => Err(anyhow!(format!("Auth failed: {}", res.payload))),
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
        if let Err(err) = common::crypto::try_generate_crypto_keys("public.key", "private.key") {
            if err.kind() != std::io::ErrorKind::AlreadyExists {
                return Err(anyhow!(err));
            }
        }
        if args.len() == 4 {
            let auth_key = auth_client(args[3].clone()).await?;
            std::fs::write("auth.key", auth_key)?;
        }
        let _ = run(args[1].clone(), args[2].clone()).await?;
    } else {
        echo_syntax(&args);
        std::process::exit(1);
    }
    Ok(())
}
