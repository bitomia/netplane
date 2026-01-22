use anyhow::{Result, anyhow};
use base64::{Engine as _, engine::general_purpose};
use env_logger::Env;
use log::{debug, error, info, warn};
use std::env;
use std::io;
use std::net::Ipv4Addr;
use std::str::FromStr;
use tokio::time::{Duration, interval};
use tokio_util::sync::CancellationToken;

use netplane_common::crypto::{load_auth_key, try_load_crypto_keys};
use netplane_common::packet::{parse_ipv4_header, validate_packet};
use netplane_common::transport::{AnyTransport, Transport, UdpTransport, WebSocketTransport};
use netplane_common::{
    HandshakeError, HandshakeRep, HandshakeReq, MessageType, PeerEventType, RelayPacket,
    UDPHeartbeat, get_message_type,
};

use crate::fd::PlatformFd;
use crate::http_client;
use crate::peer_session::PeerSessionManager;
use crate::tundev;

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

/// Handshake with the relay server
pub async fn handshake(
    auth_key: String,
    server_addr: String,
    transport: &mut AnyTransport,
) -> Result<(StartParams, String)> {
    info!("Starting handshake with relay server {}", server_addr);

    // Load client crypto keys - needed for E2E encryption with other clients
    let (client_pub, _) = try_load_crypto_keys("public.key", "private.key")
        .map_err(|e| anyhow!("Failed to load crypto keys: {}", e))?;

    let handshake = HandshakeReq::new_with_crypto(&auth_key, &client_pub);

    transport.send(&handshake.serialize()?, None).await?;

    let mut socket_buf = [0; 1500];
    loop {
        let (amt, _) = transport.recv(&mut socket_buf).await?;

        if let Ok(handshake_rep) = HandshakeRep::deserialize(&socket_buf[..amt]) {
            info!("Successful handshake with relay server {:?}", handshake_rep);

            return Ok((
                StartParams {
                    netmask: handshake_rep.netmask,
                    destination: handshake_rep.network,
                    ip_addr: handshake_rep.sdn_ip_addr,
                },
                client_pub,
            ));
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
) -> Result<Box<AnyTransport>> {
    let transport_type = transport_type
        .or_else(|| env::var("TRANSPORT").ok())
        .unwrap_or_else(|| "udp".to_string());

    match transport_type.to_lowercase().as_str() {
        "websocket" | "ws" => {
            info!("Starting websocket connection {}", control_addr);

            let control_addr = format!("ws://{}", control_addr);
            let transport = WebSocketTransport::connect(control_addr.as_str()).await?;

            Ok(Box::new(AnyTransport::WebSocket(transport)))
        }
        "udp" => {
            info!("Starting UDP connection {}", control_addr);

            let transport = UdpTransport::bind("0.0.0.0:0")
                .await
                .map_err(|_| anyhow!("Cannot bind UDP socket"))?;
            transport
                .connect(control_addr)
                .await
                .map_err(|_| anyhow!("Cannot connect UDP socket"))?;

            Ok(Box::new(AnyTransport::Udp(transport)))
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
    loopback_relay: bool,
    no_encryption: bool,
    option_token: Option<CancellationToken>,
) -> Result<tokio::task::JoinHandle<()>, anyhow::Error> {
    info!("Starting client");

    let authkey_path = String::from_str("auth.key")?;
    let auth_key = load_auth_key(authkey_path)?;
    let control_addr = format!("{}:{}", host, port.unwrap_or(5000));
    let mut transport = create_transport(&control_addr, transport_type).await?;

    info!("Client connected to relay server");

    let (start_params, _client_pub) = match handshake(auth_key, control_addr, &mut transport).await
    {
        Ok((p, pub_key)) => {
            info!("Handshake successfully finished {:?}", p);
            (p, pub_key)
        }
        Err(err) => {
            error!("Handshake failed: {}", err);
            std::process::exit(1)
        }
    };
    let (own_sdn_ip, peer_manager) = create_p2p_session(&start_params)?;

    let dev = tundev::TunDev::new(
        tun_dev,
        start_params.netmask.as_str(),
        start_params.destination.as_str(),
        start_params.ip_addr.as_str(),
    )?;

    Ok(update_loop(
        dev,
        transport,
        peer_manager,
        own_sdn_ip,
        loopback_relay,
        no_encryption,
        option_token,
    ))
}

pub async fn run_from_fd(
    tun_fd: PlatformFd,
    start_params: &StartParams,
    transport: Box<AnyTransport>,
    loopback_relay: bool,
    no_encryption: bool,
) -> Result<tokio::task::JoinHandle<()>, anyhow::Error> {
    info!("Starting client with fd");

    let (own_sdn_ip, peer_manager) = create_p2p_session(start_params)?;

    let dev = tundev::TunDev::new_from_fd(
        tun_fd,
        start_params.netmask.as_str(),
        start_params.destination.as_str(),
        start_params.ip_addr.as_str(),
    )?;

    Ok(update_loop(
        dev,
        transport,
        peer_manager,
        own_sdn_ip,
        loopback_relay,
        no_encryption,
        None,
    ))
}

fn create_p2p_session(
    start_params: &StartParams,
) -> Result<(Ipv4Addr, PeerSessionManager), anyhow::Error> {
    let (client_pub, client_priv) = try_load_crypto_keys("public.key", "private.key")?;
    let client_priv_bytes = general_purpose::URL_SAFE_NO_PAD.decode(&client_priv)?;
    let own_sdn_ip = Ipv4Addr::from_str(&start_params.ip_addr)?;
    let peer_manager = PeerSessionManager::new(own_sdn_ip, client_priv_bytes, client_pub);

    Ok((own_sdn_ip, peer_manager))
}

fn update_loop(
    mut dev: tundev::TunDev,
    mut transport: Box<AnyTransport>,
    mut peer_manager: PeerSessionManager,
    own_sdn_ip: Ipv4Addr,
    loopback_relay: bool,
    no_encryption: bool,
    option_token: Option<CancellationToken>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut heartbeat_interval = interval(Duration::from_secs(5));
        let mut socket_buf = [0; 1500];
        let mut tun_buf = [0; 1500];
        loop {
            tokio::select! {
                // Receive from relay server
                result = transport.recv(&mut socket_buf) => {
                    match result {
                        Ok((amt, _)) => {
                            handle_relay_server_message(
                                &socket_buf[..amt],
                                &mut transport,
                                &mut dev,
                                &mut peer_manager,
                                &own_sdn_ip,
                                no_encryption,
                            ).await;
                        },
                        Err(err) => error!("Receive error: {}", err)
                    }
                },

                // Receive from TUN device
                tun_ret = dev.read(&mut tun_buf) => {
                    match tun_ret {
                        Ok(amt) => {
                            handle_outgoing_packet(
                                &tun_buf[..amt],
                                &mut transport,
                                &mut dev,
                                &mut peer_manager,
                                &own_sdn_ip,
                                loopback_relay,
                                no_encryption,
                            ).await;
                        }
                        Err(err) => error!("TUN read error: {}", err)
                    }
                },

                // Send heartbeat to relay server
                _ = heartbeat_interval.tick() => {
                    let heartbeat = UDPHeartbeat::new();
                    if let Ok(data) = heartbeat.serialize() {
                        if let Err(err) = transport.send(&data, None).await {
                            error!("Failed to send heartbeat: {}", err);
                        } else {
                            debug!("Heartbeat sent");
                        }
                    }
                },

                Some(_) = async {
                    match &option_token {
                        Some(t) => Some(t.cancelled().await),
                        None => None,
                    }
                } => {
                    info!("Update loop stopped");
                    break;
                }
            }
        }
    })
}

async fn handle_relay_server_message(
    data: &[u8],
    transport: &mut AnyTransport,
    dev: &mut tundev::TunDev,
    peer_manager: &mut PeerSessionManager,
    own_sdn_ip: &Ipv4Addr,
    no_encryption: bool,
) {
    match get_message_type(data) {
        MessageType::PeerList(list) => {
            info!("Received peer list with {} peers", list.peers.len());
            for peer in list.peers {
                peer_manager.add_peer(peer);
            }
        }

        MessageType::PeerAnnounce(announce) => match announce.event_type {
            PeerEventType::Connected => {
                info!("Peer connected: {}", announce.peer.sdn_ip);
                peer_manager.add_peer(announce.peer);
            }
            PeerEventType::Disconnected => {
                info!("Peer disconnected: {}", announce.peer.sdn_ip);
                if let Ok(ip) = Ipv4Addr::from_str(&announce.peer.sdn_ip) {
                    peer_manager.remove_peer(&ip);
                }
            }
        },

        MessageType::P2PHandshakeInit(init) => {
            debug!("Received P2P handshake init from {}", init.initiator_sdn_ip);
            match peer_manager.handle_handshake_init(&init) {
                Ok(resp) => {
                    if let Ok(data) = resp.serialize() {
                        if let Err(e) = transport.send(&data, None).await {
                            error!("Failed to send P2P handshake response: {}", e);
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to handle P2P handshake init: {}", e);
                }
            }
        }

        MessageType::P2PHandshakeResp(resp) => {
            debug!(
                "Received P2P handshake response from {}",
                resp.responder_sdn_ip
            );
            match peer_manager.handle_handshake_resp(&resp) {
                Ok(()) => {
                    // Flush any queued packets
                    if let Ok(responder_ip) = Ipv4Addr::from_str(&resp.responder_sdn_ip) {
                        let pending = peer_manager.take_pending_packets(&responder_ip);
                        for packet in pending {
                            if let Some(header) = parse_ipv4_header(&packet) {
                                if let Ok(dst_ip) = Ipv4Addr::from_str(&header.dst_ip) {
                                    if let Ok(encrypted) =
                                        peer_manager.encrypt_for(&dst_ip, &packet).await
                                    {
                                        let relay = RelayPacket::new(
                                            &own_sdn_ip.to_string(),
                                            &dst_ip.to_string(),
                                            encrypted,
                                        );
                                        if let Ok(data) = relay.serialize() {
                                            if let Err(e) = transport.send(&data, None).await {
                                                error!("Failed to send queued packet: {}", e);
                                            } else {
                                                debug!("Sent queued packet to {}", dst_ip);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to handle P2P handshake response: {}", e);
                }
            }
        }

        MessageType::RelayPacket(relay) => {
            if no_encryption {
                // No encryption mode - payload is plaintext
                if validate_packet(&relay.encrypted_payload) {
                    send_tun(dev, &relay.encrypted_payload, relay.encrypted_payload.len()).await;
                } else {
                    warn!("Received invalid IPv4 packet (no-encryption mode)");
                }
            } else {
                // E2E encrypted packet from another peer
                if let Ok(src_ip) = Ipv4Addr::from_str(&relay.src_sdn_ip) {
                    match peer_manager
                        .decrypt_from(&src_ip, &relay.encrypted_payload)
                        .await
                    {
                        Ok(decrypted) => {
                            if validate_packet(&decrypted) {
                                send_tun(dev, &decrypted, decrypted.len()).await;
                            } else {
                                warn!("Decrypted packet is not valid IPv4");
                            }
                        }
                        Err(e) => {
                            warn!("Failed to decrypt packet from {}: {}", src_ip, e);
                        }
                    }
                }
            }
        }

        MessageType::Heartbeat(_) => {
            debug!("Heartbeat acknowledgment received");
        }

        _ => {
            if validate_packet(data) {
                warn!("Received unencrypted packet");
            } else {
                debug!("Unknown message type received");
            }
        }
    }
}

/// Handle an outgoing packet from the TUN device
async fn handle_outgoing_packet(
    packet: &[u8],
    transport: &mut AnyTransport,
    dev: &mut tundev::TunDev,
    peer_manager: &mut PeerSessionManager,
    own_sdn_ip: &Ipv4Addr,
    loopback_relay: bool,
    no_encryption: bool,
) {
    let header = match parse_ipv4_header(packet) {
        Some(h) => h,
        None => {
            warn!("Invalid IPv4 packet from TUN");
            return;
        }
    };

    let dst_ip = match Ipv4Addr::from_str(&header.dst_ip) {
        Ok(ip) => ip,
        Err(_) => {
            warn!("Invalid destination IP: {}", header.dst_ip);
            return;
        }
    };

    // Handle loopback - direct path unless --loopback-relay is enabled
    if dst_ip == *own_sdn_ip && !loopback_relay {
        send_tun(dev, packet, packet.len()).await;
        return;
    }

    // No encryption mode - send plaintext through relay
    if no_encryption {
        let relay = RelayPacket::new(
            &own_sdn_ip.to_string(),
            &dst_ip.to_string(),
            packet.to_vec(),
        );
        if let Ok(data) = relay.serialize() {
            if let Err(e) = transport.send(&data, None).await {
                error!("Failed to send relay packet: {}", e);
            }
        }
        return;
    }

    // Check if we have a session with this peer
    if peer_manager.has_session(&dst_ip) {
        // Encrypt and send
        match peer_manager.encrypt_for(&dst_ip, packet).await {
            Ok(encrypted) => {
                let relay =
                    RelayPacket::new(&own_sdn_ip.to_string(), &dst_ip.to_string(), encrypted);
                if let Ok(data) = relay.serialize() {
                    if let Err(e) = transport.send(&data, None).await {
                        error!("Failed to send relay packet: {}", e);
                    }
                }
            }
            Err(e) => {
                error!("Failed to encrypt packet for {}: {}", dst_ip, e);
            }
        }
    } else if peer_manager.knows_peer(&dst_ip) {
        // Queue packet and initiate handshake if not already in progress
        peer_manager.queue_packet(&dst_ip, packet.to_vec());

        if !peer_manager.handshake_in_progress(&dst_ip) {
            match peer_manager.initiate_handshake(&dst_ip) {
                Ok(init) => {
                    if let Ok(data) = init.serialize() {
                        if let Err(e) = transport.send(&data, None).await {
                            error!("Failed to send P2P handshake init: {}", e);
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to initiate handshake with {}: {}", dst_ip, e);
                }
            }
        }
    } else {
        // Unknown peer - drop packet
        debug!("Dropping packet to unknown peer: {}", dst_ip);
    }
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

    match load_auth_key(authkey_filepath.to_string()) {
        Ok(key) => {
            let auth_url = format!("http://{}:{}/auth", host, port);
            let res = http_client::http_get(&auth_url, &key)?;

            if res.status_code == axum::http::StatusCode::OK {
                warn!("Client already authenticated");
                return Ok(());
            }
        }
        Err(err) => {
            if let Some(io_err) = err.downcast_ref::<io::Error>() {
                match io_err.kind() {
                    io::ErrorKind::NotFound => (),
                    _ => return Err(anyhow!(format!("Auth failed: \"Couldn't open file\""))),
                }
            }
        }
    };

    let auth_url = format!("http://{}:{}/auth/{}", host, port, link_code);
    let (public_key, _) =
        netplane_common::crypto::try_load_crypto_keys(publickey_filepath, privatekey_filepath)?;

    let payload = netplane_common::AuthClientRequest { public_key };
    let res = http_client::http_post_json(&auth_url, &payload)?;
    match res.status_code {
        axum::http::StatusCode::OK => {
            let auth_key = res.payload;
            std::fs::write(authkey_filepath, auth_key)?;
            Ok(())
        }
        _ => Err(anyhow!(format!("Link failed"))),
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
