use flutter_rust_bridge::frb;
use once_cell::sync::Lazy;
use std::io;
use std::sync::{Mutex, Once};
use tracing::info;

use crate::frb_generated::StreamSink;
use tokio::runtime::Runtime;
use tokio_util::sync::CancellationToken;

use netplane_client::client::{
    self, auth_client, create_transport, handshake, run_from_fd, run_with_named_tun, LogFormat,
    StartParams,
};
use netplane_client::fd::PlatformFd;
use netplane_common::crypto::{load_auth_key, try_generate_crypto_keys};
use netplane_common::transport::AnyTransport;

/// Dedicated Tokio runtime. The FRB-generated functions run on FRB's worker
/// threads, but the client's networking + `tokio::spawn` need a Tokio context,
/// so all async work is driven here
static RT: Lazy<Runtime> = Lazy::new(|| Runtime::new().expect("failed to create Tokio runtime"));

/// Cancellation token for the connection, if any
static CANCEL: Lazy<Mutex<Option<CancellationToken>>> = Lazy::new(|| Mutex::new(None));

/// Live transport + handshake result produced by [`prepare_tunnel`] and consumed
/// by [`connect_fd`]. On platforms where the host owns the TUN device (Android's
/// `VpnService`), the handshake must run *before* the fd exists — the assigned IP
/// is needed to configure the tunnel — so the connected transport is parked here
/// between the two calls.
static PENDING: Lazy<Mutex<Option<PendingTunnel>>> = Lazy::new(|| Mutex::new(None));

struct PendingTunnel {
    transport: Box<AnyTransport>,
    start_params: StartParams,
}

static LOGGER_INIT: Once = Once::new();

#[frb(init)]
pub fn init_app() {
    flutter_rust_bridge::setup_default_user_utils();
}

/// Connection parameters
pub struct NetplaneConfig {
    /// Relay server host (no scheme), e.g. `relay.example.com`
    pub host: String,
    /// Relay control port (handshake/transport). `0` uses the client default
    pub port: u16,
    /// `"udp"` or `"websocket"`
    pub transport: String,
    /// One-time link code used by [`authenticate`]
    pub link_code: String,
    /// TUN device name to create
    pub tun_dev: String,
    /// Path where the auth key is stored
    pub authkey_path: String,
    pub public_key_path: String,
    pub private_key_path: String,
    /// HTTP auth port. `0` uses the client default (8000)
    pub auth_port: u16,
    pub loopback_relay: bool,
    pub no_encryption: bool,
}

/// Status events emitted during a [`connect`] session.
pub enum ConnectionEvent {
    Connecting,
    Connected { ip_addr: String, netmask: String },
    Disconnected,
    Error(String),
}

/// Initialize the tracing logger. `0 = Pretty, 1 = Json, 2 = Logfmt`
pub fn init_logger(log_format: i32) {
    LOGGER_INIT.call_once(|| {
        let fmt = match log_format {
            1 => LogFormat::Json,
            2 => LogFormat::Logfmt,
            _ => LogFormat::Pretty,
        };
        client::init_logger(fmt);
    });
}

/// Generate the client's crypto keypair if it does not already exist.
/// A pre-existing keypair is treated as success
pub fn generate_keys(public_key_path: String, private_key_path: String) -> anyhow::Result<()> {
    info!("generate_keys");

    match try_generate_crypto_keys(&public_key_path, &private_key_path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => Ok(()),
        Err(e) => Err(e.into()),
    }
}

/// Exchange the link code for an auth key and persist it to `authkey_path`.
/// Blocking network call; runs on an FRB worker thread
pub fn authenticate(config: NetplaneConfig) -> anyhow::Result<()> {
    info!("authenticate");

    let auth_port = if config.auth_port == 0 {
        None
    } else {
        Some(config.auth_port)
    };

    RT.block_on(auth_client(
        &config.authkey_path,
        &config.public_key_path,
        &config.private_key_path,
        &config.host,
        &config.link_code,
        false,
        auth_port,
    ))
}

/// Establish the tunnel and stream status until it ends or [`disconnect`] is
/// called.
/// Returns immediately after spawning the session; the caller consumes
/// the returned Dart stream. Requires privileges to create the TUN device
pub fn connect(config: NetplaneConfig, sink: StreamSink<ConnectionEvent>) -> anyhow::Result<()> {
    info!("connect");

    // Replace any previous session's token and register the new one
    let token = CancellationToken::new();
    {
        let mut guard = CANCEL.lock().unwrap();
        if let Some(previous) = guard.take() {
            previous.cancel();
        }
        *guard = Some(token.clone());
    }

    RT.spawn(async move {
        let _ = sink.add(ConnectionEvent::Connecting);

        let outcome: anyhow::Result<()> = async {
            let port = if config.port == 0 { 5000 } else { config.port };
            let control_addr = format!("{}:{}", config.host, port);

            let auth_key = load_auth_key(config.authkey_path.clone())?;
            let mut transport =
                create_transport(&control_addr, Some(config.transport.clone())).await?;

            let (start_params, _client_pub) = handshake(
                auth_key,
                &config.public_key_path,
                &config.private_key_path,
                control_addr,
                &mut transport,
                false,
            )
            .await?;

            let _ = sink.add(ConnectionEvent::Connected {
                ip_addr: start_params.ip_addr.clone(),
                netmask: start_params.netmask.clone(),
            });

            // Create the named TUN and start the packet loop
            let mut handle = run_with_named_tun(
                config.tun_dev.clone(),
                &start_params,
                transport,
                config.loopback_relay,
                config.no_encryption,
                &config.public_key_path,
                &config.private_key_path,
                None,
                None,
            )
            .await?;

            // `update_loop`'s internal token handling busy-spins on cancel, so
            // drive cancellation here by aborting the task instead
            tokio::select! {
                res = &mut handle => match res {
                    Ok(io_err) => Err(anyhow::anyhow!("client loop ended: {io_err}")),
                    Err(join_err) => Err(anyhow::anyhow!("client task failed: {join_err}")),
                },
                _ = token.cancelled() => {
                    handle.abort();
                    Ok(())
                }
            }
        }
        .await;

        match outcome {
            Ok(()) => {
                let _ = sink.add(ConnectionEvent::Disconnected);
            }
            Err(e) => {
                let _ = sink.add(ConnectionEvent::Error(e.to_string()));
            }
        }
    });

    Ok(())
}

/// Tunnel parameters assigned by the relay during the handshake. The host uses
/// these to configure its TUN device before handing the fd back to [`connect_fd`].
pub struct TunnelParams {
    pub ip_addr: String,
    pub netmask: String,
    pub destination: String,
}

/// Connect the transport and run the handshake, parking the live transport for a
/// subsequent [`connect_fd`]. Returns the IP/netmask/destination the relay
/// assigned so the caller can build the TUN device (Android `VpnService.Builder`
/// needs the address before `establish()` can hand back the fd).
///
/// Blocking network call; runs on an FRB worker thread. Must be followed by
/// [`connect_fd`] (or [`disconnect`] to discard the parked transport).
pub fn prepare_tunnel(config: NetplaneConfig) -> anyhow::Result<TunnelParams> {
    info!("prepare_tunnel");

    RT.block_on(async move {
        let port = if config.port == 0 { 5000 } else { config.port };
        let control_addr = format!("{}:{}", config.host, port);

        let auth_key = load_auth_key(config.authkey_path.clone())?;
        let mut transport =
            create_transport(&control_addr, Some(config.transport.clone())).await?;

        let (start_params, _client_pub) = handshake(
            auth_key,
            &config.public_key_path,
            &config.private_key_path,
            control_addr,
            &mut transport,
            false,
        )
        .await?;

        let params = TunnelParams {
            ip_addr: start_params.ip_addr.clone(),
            netmask: start_params.netmask.clone(),
            destination: start_params.destination.clone(),
        };

        *PENDING.lock().unwrap() = Some(PendingTunnel {
            transport,
            start_params,
        });

        Ok(params)
    })
}

/// Run the packet loop over a host-provided TUN file descriptor, streaming status
/// until it ends or [`disconnect`] is called. Consumes the transport parked by the
/// most recent [`prepare_tunnel`]; errors immediately if none is pending.
///
/// The fd is owned by the caller (e.g. the Android `VpnService`'s
/// `ParcelFileDescriptor`); this side does not close it on drop.
pub fn connect_fd(
    config: NetplaneConfig,
    fd: i32,
    sink: StreamSink<ConnectionEvent>,
) -> anyhow::Result<()> {
    info!("connect_fd");

    let pending = match PENDING.lock().unwrap().take() {
        Some(p) => p,
        None => anyhow::bail!("connect_fd called without a pending prepare_tunnel"),
    };

    // Replace any previous session's token and register the new one
    let token = CancellationToken::new();
    {
        let mut guard = CANCEL.lock().unwrap();
        if let Some(previous) = guard.take() {
            previous.cancel();
        }
        *guard = Some(token.clone());
    }

    RT.spawn(async move {
        let PendingTunnel {
            transport,
            start_params,
        } = pending;

        let outcome: anyhow::Result<()> = async {
            let _ = sink.add(ConnectionEvent::Connected {
                ip_addr: start_params.ip_addr.clone(),
                netmask: start_params.netmask.clone(),
            });

            let mut handle = run_from_fd(
                PlatformFd::from_raw_fd(fd),
                &start_params,
                transport,
                config.loopback_relay,
                config.no_encryption,
                &config.public_key_path,
                &config.private_key_path,
                None,
            )
            .await?;

            // `update_loop`'s internal token handling busy-spins on cancel, so
            // drive cancellation here by aborting the task instead
            tokio::select! {
                res = &mut handle => match res {
                    Ok(io_err) => Err(anyhow::anyhow!("client loop ended: {io_err}")),
                    Err(join_err) => Err(anyhow::anyhow!("client task failed: {join_err}")),
                },
                _ = token.cancelled() => {
                    handle.abort();
                    Ok(())
                }
            }
        }
        .await;

        match outcome {
            Ok(()) => {
                let _ = sink.add(ConnectionEvent::Disconnected);
            }
            Err(e) => {
                let _ = sink.add(ConnectionEvent::Error(e.to_string()));
            }
        }
    });

    Ok(())
}

/// Cancel the active connection, if any. Idempotent.
pub fn disconnect() {
    info!("disconnect");

    if let Some(token) = CANCEL.lock().unwrap().take() {
        token.cancel();
    }

    // Drop a transport parked by `prepare_tunnel` that was never handed to
    // `connect_fd` (e.g. the user cancelled before the fd was established).
    let _ = PENDING.lock().unwrap().take();
}
