use anyhow::{Result, anyhow};
use dotenv::dotenv;
use env_logger::Env;
use log::{debug, error, info};
use std::env;
use std::str::FromStr;
use tokio::time::{Duration, interval};

use netplane_common::crypto::load_auth_key;
use netplane_common::packet::{parse_ipv4_header, validate_packet};
use netplane_common::transport::{UdpTransport, WebSocketTransport};
use netplane_common::{
    HandshakeError, HandshakeRep, HandshakeReq, UDPHeartbeat, transport::AnyTransport,
    transport::Transport,
};

#[path = "http_post.rs"]
mod http_post;
#[path = "tray.rs"]
mod tray;
#[path = "tundev.rs"]
mod tundev;

use http_post::http_post_json;

// Re-export fd module when this file is used as a binary
// When included as a module in lib.rs, this will be a child module
#[path = "fd.rs"]
pub mod fd;

use fd::PlatformFd;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessError(u32);

async fn send_tun(dev: &mut tundev::TunDev, buf: &[u8], nbytes: usize) {
    match dev.send(&buf[..nbytes], nbytes).await {
        Ok(_) => {}
        Err(err) => error!("{}", err),
    }
}

#[derive(Debug)]
pub struct StartParams {
    pub netmask: String,
    pub destination: String,
    pub ip_addr: String,
}

pub async fn handshake(auth_key: String, transport: &mut AnyTransport) -> Result<StartParams> {
    info!("Starting handshake");

    let handshake = HandshakeReq::new(&auth_key);
    transport.send(&handshake.serialize()?, None).await?;

    let mut socket_buf = [0; 1500];
    loop {
        let (amt, _) = transport.recv(&mut socket_buf).await?;
        if let Ok(handshake) = HandshakeRep::deserialize(&socket_buf[..amt]) {
            info!("Successful handshake {:?}", handshake);
            return Ok(StartParams {
                netmask: handshake.netmask,
                destination: handshake.network,
                ip_addr: handshake.sdn_ip_addr,
            });
        } else if let Ok(error_response) = HandshakeError::deserialize(&socket_buf[..amt]) {
            error!("Authorization failed: {}", error_response.error_message);
            return Err(anyhow!(
                "Authorization failed: {}",
                error_response.error_message
            ));
        } else {
            error!("Initialization failed. Keep trying");
        }
    }
}

pub async fn create_transport(
    control_addr: &str,
    transport_type: Option<String>,
) -> Result<AnyTransport> {
    let transport_type = transport_type
        .or_else(|| env::var("TRANSPORT").ok())
        .unwrap_or_else(|| "udp".to_string());

    match transport_type.to_lowercase().as_str() {
        "websocket" | "ws" => {
            info!("Starting websocket connection");
            let control_addr = format!("ws://{}", control_addr);
            let transport = WebSocketTransport::connect(control_addr.as_str()).await?;
            Ok(AnyTransport::WebSocket(transport))
        }
        "udp" => {
            info!("Starting UDP connection");
            let transport = UdpTransport::bind("0.0.0.0:0")
                .await
                .map_err(|_| anyhow!("Cannot bind UDP socket"))?;
            transport
                .connect(control_addr)
                .await
                .map_err(|_| anyhow!("Cannot connect UDP socket"))?;
            Ok(AnyTransport::Udp(transport))
        }
        _ => Err(anyhow!(
            "Unsupported transport type: {}. Use 'websocket' or 'udp'",
            transport_type
        )),
    }
}

pub async fn run(
    tun_dev: String,
    host: String,
    port: Option<u16>,
    transport_type: Option<String>,
) -> Result<()> {
    info!("Starting client");
    let authkey_path = String::from_str("auth.key")?;
    let auth_key = load_auth_key(authkey_path)?;
    let control_addr = format!("{}:{}", host, port.unwrap_or(5000));
    let mut transport = create_transport(&control_addr, transport_type).await?;

    let start_params = match handshake(auth_key, &mut transport).await {
        Ok(p) => {
            info!("Handshake successfully finished {:?}", p);
            p
        }
        Err(err) => {
            error!("Handshake failed: {}", err);
            std::process::exit(1)
        }
    };

    let mut dev = tundev::TunDev::new(
        tun_dev,
        start_params.netmask.as_str(),
        start_params.destination.as_str(),
        start_params.ip_addr.as_str(),
    )?;

    update_loop(&mut dev, &mut transport).await
}

pub async fn run_from_fd(
    tun_fd: PlatformFd,
    start_params: &StartParams,
    mut transport: &mut AnyTransport,
) -> Result<()> {
    info!("Starting client with fd");

    let mut dev = tundev::TunDev::new_from_fd(
        tun_fd,
        start_params.netmask.as_str(),
        start_params.destination.as_str(),
        start_params.ip_addr.as_str(),
    )?;

    update_loop(&mut dev, &mut transport).await
}

async fn update_loop(dev: &mut tundev::TunDev, transport: &mut AnyTransport) -> Result<()> {
    let mut heartbeat_interval = interval(Duration::from_secs(5));
    let mut socket_buf = [0; 1500];
    let mut tun_buf = [0; 1500];

    loop {
        tokio::select! {
            result = transport.recv(&mut socket_buf) => {
                match result {
                    Ok((amt, _)) => {
                        if validate_packet(&socket_buf[..amt]) {
                            send_tun(dev, &socket_buf, amt).await;
                        } else {
                            error!("Ignoring non-ipv4 packet")
                        }
                    },
                    Err(err) => error!("{}", err)
                }
            },
            tun_ret = dev.read(&mut tun_buf) => {
                match tun_ret {
                    Ok(amt) => {
                        let mut is_loopback = false;
                        if let Some(header) = parse_ipv4_header(&tun_buf[..amt]) {
                            is_loopback = header.src_ip == header.dst_ip;
                        }
                        if is_loopback {
                            send_tun(dev, &tun_buf, amt).await;
                        } else {
                            match transport.send(&tun_buf[..amt], None).await {
                                Ok(bytes_sent) => {
                                    if bytes_sent != amt {
                                        error!("Less bytes sent than expected to socket");
                                    }
                                },
                                Err(err) => error!("{}", err)
                            }
                        }
                    }
                    Err(err) => {
                        error!("{}", err);
                    }
                }
            },
            _ = heartbeat_interval.tick() => {
                let heartbeat = UDPHeartbeat::new();
                match heartbeat.serialize() {
                    Ok(heartbeat_data) => {
                        match transport.send(&heartbeat_data, None).await {
                            Ok(_) => {
                                debug!("Heartbeat sent to server");
                            },
                            Err(err) => {
                                error!("Failed to send heartbeat: {}", err);
                            }
                        }
                    },
                    Err(err) => {
                        error!("Failed to serialize heartbeat: {}", err);
                    }
                }
            }
        }
    }
}

#[cfg(not(any(target_os = "android", target_os = "ios")))]
#[allow(dead_code)]
fn echo_syntax(args: &Vec<String>) {
    println!(
        "Use {} [server] [--port=5000] [--tun=device] [--auth=link_code] [--auth-port=8000] [--transport=udp|websocket]",
        args[0]
    );
}

pub async fn auth_client(
    authkey_filepath: &str,
    publickey_filepath: &str,
    privatekey_filepath: &str,
    host: &str,
    link_code: &str,
    auth_port: Option<u16>,
) -> Result<()> {
    let port = auth_port.unwrap_or(8000);
    let auth_url = format!("http://{}:{}/auth/{}", host, port, link_code);
    let (public_key, _) =
        netplane_common::crypto::try_load_crypto_keys(publickey_filepath, privatekey_filepath)?;

    let payload = netplane_common::AuthClientRequest { public_key };
    let res = http_post_json(&auth_url, &payload)?;
    match res.status_code {
        axum::http::StatusCode::OK => {
            let auth_key = res.payload;
            std::fs::write(authkey_filepath, auth_key)?;
            Ok(())
        }
        _ => Err(anyhow!(format!("Auth failed: {}", res.payload))),
    }
}

pub fn init_logger() {
    #[cfg(target_os = "android")]
    android_logger::init_once(
        android_logger::Config::default()
            .with_max_level(log::LevelFilter::Trace)
            .with_tag("netplane"),
    );

    #[cfg(not(target_os = "android"))]
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
}

#[cfg(not(any(target_os = "android", target_os = "ios")))]
#[allow(dead_code)]
fn main() -> Result<()> {
    init_logger();
    info!("Netplane client rev {}", netplane_common::git_rev_main!());

    dotenv().ok();

    let args: Vec<String> = env::args().collect();
    if args.len() >= 2 {
        if let Err(err) =
            netplane_common::crypto::try_generate_crypto_keys("public.key", "private.key")
        {
            if err.kind() != std::io::ErrorKind::AlreadyExists {
                return Err(anyhow!(err));
            }
        }

        let mut auth_arg = None;
        let mut transport_type = None;
        let mut tun_dev = "tun0".to_string();
        let mut port = None;
        let mut auth_port = None;

        // Parse optional arguments
        for arg in &args[2..] {
            if arg.starts_with("--auth=") {
                auth_arg = Some(arg.clone());
            } else if arg.starts_with("--transport=") {
                transport_type = arg.split('=').nth(1).map(|s| s.to_string());
            } else if arg.starts_with("--tun=") {
                tun_dev = arg.split('=').nth(1).unwrap_or("tun0").to_string();
            } else if arg.starts_with("--port=") {
                if let Some(port_str) = arg.split('=').nth(1) {
                    port = port_str.parse::<u16>().ok();
                }
            } else if arg.starts_with("--auth-port=") {
                if let Some(auth_port_str) = arg.split('=').nth(1) {
                    auth_port = auth_port_str.parse::<u16>().ok();
                }
            }
        }

        let host = args[1].clone();

        // Run tokio runtime in a separate thread to keep main thread for tray
        #[cfg(all(feature = "tray", target_os = "macos"))]
        {
            // Run the client in a tokio runtime on a background thread
            std::thread::spawn(move || {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async {
                    if let Some(auth_arg) = auth_arg {
                        let parts: Vec<&str> = auth_arg.split("=").collect();
                        if parts.len() == 2 {
                            let link_code = parts[1];
                            let _ = auth_client(
                                "auth.key",
                                "public.key",
                                "private.key",
                                &host,
                                link_code,
                                auth_port,
                            )
                            .await;
                        }
                    }

                    let _ = run(tun_dev, host, port, transport_type).await;
                });
            });

            // Initialize and display tray on main thread (blocks)
            tray::init_tray_and_display()?;
        }

        #[cfg(not(all(feature = "tray", target_os = "macos")))]
        {
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(async {
                // Initialize tray icon after validating arguments
                #[cfg(all(feature = "tray", any(target_os = "windows", target_os = "linux")))]
                let tray_rx = tray::init_tray().ok();

                if let Some(auth_arg) = auth_arg {
                    let parts: Vec<&str> = auth_arg.split("=").collect();
                    if parts.len() != 2 {
                        return Err(anyhow!("Invalid auth argument"));
                    }
                    let link_code = parts[1];
                    auth_client(
                        "auth.key",
                        "public.key",
                        "private.key",
                        &host,
                        link_code,
                        auth_port,
                    )
                    .await?;
                }

                // Spawn tray message handler on supported platforms
                #[cfg(all(feature = "tray", any(target_os = "windows", target_os = "linux")))]
                if let Some(rx) = tray_rx {
                    tokio::spawn(async move {
                        loop {
                            if let Ok(msg) = rx.try_recv() {
                                match msg {
                                    tray::TrayMessage::Quit => {
                                        info!("Quit requested from tray");
                                        std::process::exit(0);
                                    }
                                }
                            }
                            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                        }
                    });
                    info!("Tray message handler spawned");
                }

                run(tun_dev, host, port, transport_type).await
            })?;
        }
    } else {
        echo_syntax(&args);
        std::process::exit(1);
    }
    Ok(())
}
