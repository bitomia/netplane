use anyhow::{Result, anyhow};
use base64::{Engine as _, engine::general_purpose};
use serde::Serialize;
use std::env;
use std::io;
use std::net::Ipv4Addr;
use std::str::FromStr;
use tokio::time::{Duration, interval};
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, warn};

use netplane_common::crypto::{load_auth_key, try_load_crypto_keys};
use netplane_common::packet::{is_multicast_or_broadcast, parse_ipv4_header, validate_packet};
use netplane_common::transport::{AnyTransport, Transport, UdpTransport, WebSocketTransport};
use netplane_common::{
    HandshakeError, HandshakeRep, HandshakeReq, MessageType, PeerEventType, RelayPacket,
    UDPHeartbeat, get_message_type,
};

use crate::client_manager::ClientManager;
use crate::fd::PlatformFd;
use crate::http_client;
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

#[derive(Debug, Clone, Serialize)]
pub struct ClientState {}

/// Handshake with the relay server
pub async fn handshake(
    auth_key: String,
    public_filepath: &str,
    private_filepath: &str,
    server_addr: String,
    transport: &mut AnyTransport,
    is_dynamic_link: bool,
) -> Result<(StartParams, String)> {
    info!("Starting handshake with relay server {}", server_addr);

    // Load client crypto keys - needed for E2E encryption with other clients
    let (client_pub, _) = try_load_crypto_keys(public_filepath, private_filepath)
        .map_err(|e| anyhow!("Failed to load crypto keys: {}", e))?;

    let handshake = HandshakeReq::new_with_crypto(&auth_key, &client_pub, is_dynamic_link);

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

#[allow(clippy::too_many_arguments)]
pub async fn run(
    tun_dev: String,
    host: String,
    port: Option<u16>,
    transport_type: Option<String>,
    loopback_relay: bool,
    no_encryption: bool,
    is_dynamic_link: bool,
    authkey_path: &str,
    public_filepath: &str,
    private_filepath: &str,
    option_token: Option<CancellationToken>,
    state_tx: Option<tokio::sync::watch::Sender<ClientState>>,
) -> Result<tokio::task::JoinHandle<std::io::Error>, anyhow::Error> {
    info!("Starting client");

    let auth_key = load_auth_key(authkey_path.to_string())?;
    let control_addr = format!("{}:{}", host, port.unwrap_or(5000));
    let mut transport = create_transport(&control_addr, transport_type).await?;

    info!("Client connected to relay server");

    let (start_params, _client_pub) = match handshake(
        auth_key,
        public_filepath,
        private_filepath,
        control_addr,
        &mut transport,
        is_dynamic_link,
    )
    .await
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
    let (own_sdn_ip, client_manager) =
        create_p2p_session(&start_params, public_filepath, private_filepath)?;

    let dev = tundev::TunDev::new(
        tun_dev,
        start_params.netmask.as_str(),
        start_params.destination.as_str(),
        start_params.ip_addr.as_str(),
    )?;

    Ok(update_loop(
        dev,
        transport,
        client_manager,
        own_sdn_ip,
        loopback_relay,
        no_encryption,
        option_token,
        state_tx,
    ))
}

#[allow(clippy::too_many_arguments)]
pub async fn run_from_fd(
    tun_fd: PlatformFd,
    start_params: &StartParams,
    transport: Box<AnyTransport>,
    loopback_relay: bool,
    no_encryption: bool,
    public_filepath: &str,
    private_filepath: &str,
    state_tx: Option<tokio::sync::watch::Sender<ClientState>>,
) -> Result<tokio::task::JoinHandle<std::io::Error>, anyhow::Error> {
    info!("Starting client with fd");

    let (own_sdn_ip, client_manager) =
        create_p2p_session(start_params, public_filepath, private_filepath)?;

    let dev = tundev::TunDev::new_from_fd(
        tun_fd,
        start_params.netmask.as_str(),
        start_params.destination.as_str(),
        start_params.ip_addr.as_str(),
    )?;

    Ok(update_loop(
        dev,
        transport,
        client_manager,
        own_sdn_ip,
        loopback_relay,
        no_encryption,
        None,
        state_tx,
    ))
}

fn create_p2p_session(
    start_params: &StartParams,
    public_filepath: &str,
    private_filepath: &str,
) -> Result<(Ipv4Addr, ClientManager), anyhow::Error> {
    let (client_pub, client_priv) = try_load_crypto_keys(public_filepath, private_filepath)?;
    let client_priv_bytes = general_purpose::URL_SAFE_NO_PAD.decode(&client_priv)?;
    let own_sdn_ip = Ipv4Addr::from_str(&start_params.ip_addr)?;
    let client_manager = ClientManager::new(own_sdn_ip, client_priv_bytes, client_pub);

    Ok((own_sdn_ip, client_manager))
}

#[allow(clippy::too_many_arguments)]
fn update_loop(
    mut dev: tundev::TunDev,
    mut transport: Box<AnyTransport>,
    mut client_manager: ClientManager,
    own_sdn_ip: Ipv4Addr,
    loopback_relay: bool,
    no_encryption: bool,
    option_token: Option<CancellationToken>,
    state_tx: Option<tokio::sync::watch::Sender<ClientState>>,
) -> tokio::task::JoinHandle<std::io::Error> {
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
                                &mut client_manager,
                                &own_sdn_ip,
                                no_encryption,
                                &state_tx,
                            ).await;
                        },
                        Err(err) => {
                            error!("Receive error: {}", err);
                            return err;
                        }
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
                                &mut client_manager,
                                &own_sdn_ip,
                                loopback_relay,
                                no_encryption,
                            ).await;
                        }
                        Err(err) => {
                            error!("TUN read error: {}", err);
                            return err;
                        }
                    }
                },

                // Send heartbeat to relay server
                _ = heartbeat_interval.tick() => {
                    let heartbeat = UDPHeartbeat::new();
                    let data = heartbeat.serialize().unwrap();
                    match transport.send(&data, None).await {
                        Ok(_) => debug!("Heartbeat sent"),
                        Err(err) => {
                            error!("Failed to send heartbeat: {}", err);
                            return err;
                        }
                    }
                },

                Some(_) = async {
                    match &option_token {
                        Some(t) => {
                            let _: () = t.cancelled().await;
                            Some(())
                        },
                        None => None,
                    }
                } => {
                    info!("Update loop stopped");
                }
            }
        }
    })
}

async fn handle_relay_server_message(
    data: &[u8],
    transport: &mut AnyTransport,
    dev: &mut tundev::TunDev,
    client_manager: &mut ClientManager,
    own_sdn_ip: &Ipv4Addr,
    no_encryption: bool,
    state_tx: &Option<tokio::sync::watch::Sender<ClientState>>,
) {
    match get_message_type(data) {
        MessageType::PeerList(list) => {
            info!("Received peer list with {} peers", list.peers.len());
            for peer in list.peers {
                client_manager.add_peer(peer);
            }
        }

        MessageType::PeerAnnounce(announce) => match announce.event_type {
            PeerEventType::Connected => {
                info!("Peer connected: {}", announce.peer.sdn_ip);
                client_manager.add_peer(announce.peer);
            }
            PeerEventType::Disconnected => {
                info!("Peer disconnected: {}", announce.peer.sdn_ip);
                if let Ok(ip) = Ipv4Addr::from_str(&announce.peer.sdn_ip) {
                    client_manager.remove_peer(&ip);
                }
            }
        },

        MessageType::P2PHandshakeInit(init) => {
            debug!("Received P2P handshake init from {}", init.initiator_sdn_ip);
            match client_manager.handle_handshake_init(&init) {
                Ok(resp) => {
                    if let Ok(data) = resp.serialize()
                        && let Err(e) = transport.send(&data, None).await
                    {
                        error!("Failed to send P2P handshake response: {}", e);
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
            match client_manager.handle_handshake_resp(&resp) {
                Ok(()) => {
                    // Flush any queued packets
                    if let Ok(responder_ip) = Ipv4Addr::from_str(&resp.responder_sdn_ip) {
                        let pending = client_manager.take_pending_packets(&responder_ip);
                        for packet in pending {
                            if let Some(header) = parse_ipv4_header(&packet)
                                && let Ok(dst_ip) = Ipv4Addr::from_str(&header.dst_ip)
                                && let Ok(encrypted) =
                                    client_manager.encrypt_for(&dst_ip, &packet).await
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
                    match client_manager
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
            if let Some(tx) = state_tx {
                let _ = tx.send(ClientState {});
            }
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
    client_manager: &mut ClientManager,
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

    // Handle multicast/broadcast
    if is_multicast_or_broadcast(&dst_ip) {
        if no_encryption {
            // No-encryption mode: send single RelayPacket with multicast dst,
            // server will fan out to all peers
            let relay = RelayPacket::new(
                &own_sdn_ip.to_string(),
                &dst_ip.to_string(),
                packet.to_vec(),
            );
            if let Ok(data) = relay.serialize()
                && let Err(e) = transport.send(&data, None).await
            {
                error!("Failed to send multicast relay packet: {}", e);
            }
        } else {
            // Encryption mode: encrypt separately for each peer with an established session
            let peers = client_manager.get_all_session_peers();
            for peer_ip in peers {
                match client_manager.encrypt_for(&peer_ip, packet).await {
                    Ok(encrypted) => {
                        let relay = RelayPacket::new(
                            &own_sdn_ip.to_string(),
                            &peer_ip.to_string(),
                            encrypted,
                        );
                        if let Ok(data) = relay.serialize()
                            && let Err(e) = transport.send(&data, None).await
                        {
                            error!("Failed to send multicast relay to {}: {}", peer_ip, e);
                        }
                    }
                    Err(e) => {
                        error!("Failed to encrypt multicast for {}: {}", peer_ip, e);
                    }
                }
            }
        }
        return;
    }

    // No encryption mode - send plaintext through relay
    if no_encryption {
        let relay = RelayPacket::new(
            &own_sdn_ip.to_string(),
            &dst_ip.to_string(),
            packet.to_vec(),
        );
        if let Ok(data) = relay.serialize()
            && let Err(e) = transport.send(&data, None).await
        {
            error!("Failed to send relay packet: {}", e);
        }
        return;
    }

    // Check if we have a session with this peer
    if client_manager.has_session(&dst_ip) {
        // Encrypt and send
        match client_manager.encrypt_for(&dst_ip, packet).await {
            Ok(encrypted) => {
                let relay =
                    RelayPacket::new(&own_sdn_ip.to_string(), &dst_ip.to_string(), encrypted);
                if let Ok(data) = relay.serialize()
                    && let Err(e) = transport.send(&data, None).await
                {
                    error!("Failed to send relay packet: {}", e);
                }
            }
            Err(e) => {
                error!("Failed to encrypt packet for {}: {}", dst_ip, e);
            }
        }
    } else if client_manager.knows_peer(&dst_ip) {
        // Queue packet and initiate handshake if not already in progress
        client_manager.queue_packet(&dst_ip, packet.to_vec());

        if !client_manager.handshake_in_progress(&dst_ip) {
            match client_manager.initiate_handshake(&dst_ip) {
                Ok(init) => {
                    if let Ok(data) = init.serialize()
                        && let Err(e) = transport.send(&data, None).await
                    {
                        error!("Failed to send P2P handshake init: {}", e);
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
    dynamic_link: bool,
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
                    _ => return Err(anyhow!("Auth failed: \"Couldn't open file\"")),
                }
            }
        }
    };

    let auth_url = format!("http://{}:{}/auth/{}", host, port, link_code);

    let (public_key, _) =
        netplane_common::crypto::try_load_crypto_keys(publickey_filepath, privatekey_filepath)?;

    let payload = netplane_common::AuthClientRequest {
        public_key,
        dynamic_link,
    };
    let res = http_client::http_post_json(&auth_url, &payload)?;
    match res.status_code {
        axum::http::StatusCode::OK => {
            let auth_key = res.payload;
            std::fs::write(authkey_filepath, auth_key)?;
            Ok(())
        }
        err => Err(anyhow!("{}:{}", err, res.payload)),
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub enum LogFormat {
    #[default]
    Pretty,
    Json,
    Logfmt,
}

#[cfg(not(target_os = "android"))]
mod json_fmt {
    use std::fmt;
    use tracing::{Event, Subscriber};
    use tracing_subscriber::fmt::{FmtContext, FormatEvent, FormatFields, format::Writer};
    use tracing_subscriber::registry::LookupSpan;

    pub struct JsonFormatter;

    impl<S, N> FormatEvent<S, N> for JsonFormatter
    where
        S: Subscriber + for<'a> LookupSpan<'a>,
        N: for<'a> FormatFields<'a> + 'static,
    {
        fn format_event(
            &self,
            _ctx: &FmtContext<'_, S, N>,
            mut writer: Writer<'_>,
            event: &Event<'_>,
        ) -> fmt::Result {
            let ts = chrono::Local::now().to_rfc3339_opts(chrono::SecondsFormat::Nanos, false);
            writer.write_char('{')?;
            write!(writer, "\"time\":\"{}\"", ts)?;
            write!(writer, ",\"level\":\"{}\"", event.metadata().level())?;

            let mut visitor = JsonVisitor {
                writer: &mut writer,
                result: Ok(()),
            };
            event.record(&mut visitor);
            visitor.result?;

            writer.write_char('}')?;
            writeln!(writer)
        }
    }

    struct JsonVisitor<'a, 'b> {
        writer: &'a mut Writer<'b>,
        result: fmt::Result,
    }

    impl<'a, 'b> tracing::field::Visit for JsonVisitor<'a, 'b> {
        fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn fmt::Debug) {
            if self.result.is_err() {
                return;
            }
            let rendered = format!("{:?}", value);
            let unquoted = strip_debug_quotes(&rendered).to_string();
            self.result = write_string(self.writer, Self::name_or_msg(field), &unquoted);
        }

        fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
            if self.result.is_err() {
                return;
            }
            self.result = write_string(self.writer, Self::name_or_msg(field), value);
        }

        fn record_i64(&mut self, field: &tracing::field::Field, value: i64) {
            if self.result.is_err() {
                return;
            }
            self.result = write_raw(self.writer, Self::name_or_msg(field), &value.to_string());
        }

        fn record_u64(&mut self, field: &tracing::field::Field, value: u64) {
            if self.result.is_err() {
                return;
            }
            self.result = write_raw(self.writer, Self::name_or_msg(field), &value.to_string());
        }

        fn record_f64(&mut self, field: &tracing::field::Field, value: f64) {
            if self.result.is_err() {
                return;
            }
            let v = if value.is_finite() {
                value.to_string()
            } else {
                format!("\"{}\"", value)
            };
            self.result = write_raw(self.writer, Self::name_or_msg(field), &v);
        }

        fn record_bool(&mut self, field: &tracing::field::Field, value: bool) {
            if self.result.is_err() {
                return;
            }
            self.result = write_raw(
                self.writer,
                Self::name_or_msg(field),
                if value { "true" } else { "false" },
            );
        }
    }

    impl<'a, 'b> JsonVisitor<'a, 'b> {
        fn name_or_msg(field: &tracing::field::Field) -> &str {
            if field.name() == "message" {
                "msg"
            } else {
                field.name()
            }
        }
    }

    fn strip_debug_quotes(s: &str) -> &str {
        if s.len() >= 2 && s.starts_with('"') && s.ends_with('"') {
            &s[1..s.len() - 1]
        } else {
            s
        }
    }

    fn write_raw(writer: &mut Writer<'_>, key: &str, raw: &str) -> fmt::Result {
        writer.write_char(',')?;
        write_json_string(writer, key)?;
        writer.write_char(':')?;
        writer.write_str(raw)
    }

    fn write_string(writer: &mut Writer<'_>, key: &str, value: &str) -> fmt::Result {
        writer.write_char(',')?;
        write_json_string(writer, key)?;
        writer.write_char(':')?;
        write_json_string(writer, value)
    }

    fn write_json_string(writer: &mut Writer<'_>, s: &str) -> fmt::Result {
        writer.write_char('"')?;
        for c in s.chars() {
            match c {
                '"' => writer.write_str("\\\"")?,
                '\\' => writer.write_str("\\\\")?,
                '\n' => writer.write_str("\\n")?,
                '\r' => writer.write_str("\\r")?,
                '\t' => writer.write_str("\\t")?,
                '\x08' => writer.write_str("\\b")?,
                '\x0c' => writer.write_str("\\f")?,
                c if (c as u32) < 0x20 => write!(writer, "\\u{:04x}", c as u32)?,
                c => writer.write_char(c)?,
            }
        }
        writer.write_char('"')
    }
}

#[cfg(not(target_os = "android"))]
mod logfmt {
    use std::fmt;
    use tracing::{Event, Subscriber};
    use tracing_subscriber::fmt::{FmtContext, FormatEvent, FormatFields, format::Writer};
    use tracing_subscriber::registry::LookupSpan;

    pub struct LogfmtFormatter;

    impl<S, N> FormatEvent<S, N> for LogfmtFormatter
    where
        S: Subscriber + for<'a> LookupSpan<'a>,
        N: for<'a> FormatFields<'a> + 'static,
    {
        fn format_event(
            &self,
            ctx: &FmtContext<'_, S, N>,
            mut writer: Writer<'_>,
            event: &Event<'_>,
        ) -> fmt::Result {
            let ts = chrono::Local::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, false);
            write!(writer, "time={} level={}", ts, event.metadata().level())?;

            let mut visitor = LogfmtVisitor {
                writer: &mut writer,
                result: Ok(()),
            };
            event.record(&mut visitor);
            visitor.result?;

            if let Some(scope) = ctx.event_scope() {
                for span in scope.from_root() {
                    write!(writer, " span={}", span.name())?;
                }
            }

            writeln!(writer)
        }
    }

    struct LogfmtVisitor<'a, 'b> {
        writer: &'a mut Writer<'b>,
        result: fmt::Result,
    }

    impl<'a, 'b> tracing::field::Visit for LogfmtVisitor<'a, 'b> {
        fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn fmt::Debug) {
            if self.result.is_err() {
                return;
            }
            let rendered = format!("{:?}", value);
            let unquoted = strip_debug_quotes(&rendered);
            self.result = write_kv(self.writer, field.name(), unquoted);
        }

        fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
            if self.result.is_err() {
                return;
            }
            self.result = write_kv(self.writer, field.name(), value);
        }
    }

    fn strip_debug_quotes(s: &str) -> &str {
        if s.len() >= 2 && s.starts_with('"') && s.ends_with('"') {
            &s[1..s.len() - 1]
        } else {
            s
        }
    }

    fn write_kv(writer: &mut Writer<'_>, key: &str, value: &str) -> fmt::Result {
        let key = if key == "message" { "msg" } else { key };
        let needs_quote = value.is_empty()
            || value
                .chars()
                .any(|c| c == ' ' || c == '"' || c == '=' || c.is_control());
        if needs_quote {
            write!(writer, " {}=\"", key)?;
            for c in value.chars() {
                match c {
                    '"' => writer.write_str("\\\"")?,
                    '\\' => writer.write_str("\\\\")?,
                    '\n' => writer.write_str("\\n")?,
                    '\r' => writer.write_str("\\r")?,
                    '\t' => writer.write_str("\\t")?,
                    c => writer.write_char(c)?,
                }
            }
            writer.write_char('"')
        } else {
            write!(writer, " {}={}", key, value)
        }
    }
}

pub fn init_logger(format: LogFormat) {
    #[cfg(target_os = "android")]
    {
        use tracing_subscriber::layer::SubscriberExt;
        use tracing_subscriber::util::SubscriberInitExt;
        let _ = format;
        let android_layer = tracing_android::layer("netplane").expect("init tracing-android");
        tracing_subscriber::registry()
            .with(tracing_subscriber::EnvFilter::new("trace"))
            .with(android_layer)
            .init();
    }

    #[cfg(not(target_os = "android"))]
    {
        use tracing_subscriber::EnvFilter;
        let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
        match format {
            LogFormat::Json => {
                let _ = tracing_subscriber::fmt()
                    .event_format(json_fmt::JsonFormatter)
                    .with_env_filter(filter)
                    .try_init();
            }
            LogFormat::Pretty => {
                let _ = tracing_subscriber::fmt().with_env_filter(filter).try_init();
            }
            LogFormat::Logfmt => {
                let _ = tracing_subscriber::fmt()
                    .event_format(logfmt::LogfmtFormatter)
                    .with_env_filter(filter)
                    .try_init();
            }
        }
    }
}
