use anyhow::{Result, anyhow};
use dotenv::dotenv;
use http_post::http_post_json;
use log::{error, info, trace};
use netplane_common::crypto::load_auth_key;
use netplane_common::packet::{parse_ipv4_header, validate_packet};
use netplane_common::transport::{UdpTransport, WebSocketTransport};
use netplane_common::{
    HandshakeRep, HandshakeReq, UDPHeartbeat, transport::AnyTransport, transport::Transport,
};
use std::env;
use tokio::time::{Duration, interval};
//use tray_item::{TrayItem, IconSource};

pub mod http_post;
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
    transport: &mut AnyTransport,
) -> Result<StartParams> {
    info!("Starting handshake with {}", server_addr);

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
        } else {
            error!("Initialization failed. Keep trying");
        }
    }
}

async fn create_transport(
    control_addr: &str,
    transport_type: Option<String>,
) -> Result<AnyTransport> {
    let transport_type = transport_type
        .or_else(|| env::var("TRANSPORT").ok())
        .unwrap_or_else(|| "udp".to_string());

    match transport_type.to_lowercase().as_str() {
        "websocket" | "ws" => {
            info!("Starting websocket connection");
            //            let control_addr = format!("ws://{}", control_addr);
            let transport = WebSocketTransport::connect(control_addr).await?;
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
    control_addr: String,
    transport_type: Option<String>,
) -> Result<()> {
    info!("Starting client");
    let auth_key = load_auth_key()?;

    let mut transport = create_transport(&control_addr, transport_type).await?;

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
    let mut dev = tundev::TunDev::new(
        tun_dev,
        start_params.netmask.as_str(),
        start_params.destination.as_str(),
        start_params.ip_addr.as_str(),
    );

    let mut heartbeat_interval = interval(Duration::from_secs(5));

    loop {
        tokio::select! {
            result = transport.recv(&mut socket_buf) => {
                match result {
                    Ok((amt, _)) => {
                        if validate_packet(&socket_buf[..amt]) {
                            send_tun(&mut dev, &socket_buf, amt).await;
                        } else {
                            trace!("Ignoring non-ipv4 packet")
                        }
                    },
                    Err(_) => todo!()
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
                                trace!("Heartbeat sent to server");
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

fn echo_syntax(args: &Vec<String>) {
    println!(
        "Use {} [tun_dev] [server_ip] [--auth=link] [--transport=udp|websocket]",
        args[0]
    );
}

async fn auth_client(arg: String) -> Result<String> {
    let parts: Vec<&str> = arg.split("=").collect();
    if parts.len() != 2 {
        return Err(anyhow!("Invalid auth argument"));
    }

    let (public_key, _) =
        netplane_common::crypto::try_load_crypto_keys("public.key", "private.key")?;
    let payload = netplane_common::AuthClientRequest { public_key };

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
    println!("netplane client ({})", netplane_common::git_rev_main!());

    env_logger::init();
    dotenv().ok();

    let args: Vec<String> = env::args().collect();
    if args.len() >= 3 {
        if let Err(err) =
            netplane_common::crypto::try_generate_crypto_keys("public.key", "private.key")
        {
            if err.kind() != std::io::ErrorKind::AlreadyExists {
                return Err(anyhow!(err));
            }
        }

        let mut auth_arg = None;
        let mut transport_type = None;

        // Parse optional arguments
        for arg in &args[3..] {
            if arg.starts_with("--auth=") {
                auth_arg = Some(arg.clone());
            } else if arg.starts_with("--transport=") {
                transport_type = arg.split('=').nth(1).map(|s| s.to_string());
            }
        }

        if let Some(auth_arg) = auth_arg {
            let auth_key = auth_client(auth_arg).await?;
            std::fs::write("auth.key", auth_key)?;
        }

        let _ = run(args[1].clone(), args[2].clone(), transport_type).await?;
    } else {
        echo_syntax(&args);
        std::process::exit(1);
    }
    Ok(())
}
