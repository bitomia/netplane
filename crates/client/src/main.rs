use anyhow::{Result, anyhow};
use dotenv::dotenv;
use log::info;

use std::env;

pub mod client;
pub mod client_manager;
mod fd;
mod http_client;
mod tundev;

#[cfg(not(any(target_os = "android", target_os = "ios")))]
#[allow(dead_code)]
fn echo_syntax(args: &Vec<String>) {
    println!(
        "Use {} [server] [--port=5000] [--tun=device] [--auth=link_code] [--auth-port=8000] [--transport=udp|websocket] [--loopback-relay] [--no-encryption]",
        args[0]
    );
}

#[cfg(not(any(target_os = "android", target_os = "ios")))]
#[allow(dead_code)]
fn main() -> Result<()> {
    client::init_logger();
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
        let mut loopback_relay = false;
        let mut no_encryption = false;

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
            } else if arg == "--loopback-relay" {
                loopback_relay = true;
            } else if arg == "--no-encryption" {
                no_encryption = true;
            }
        }

        let host = args[1].clone();

        let rt = tokio::runtime::Runtime::new()?;
        rt.block_on(async {
            if let Some(auth_arg) = auth_arg {
                let parts: Vec<&str> = auth_arg.split("=").collect();
                if parts.len() != 2 {
                    return Err(anyhow!("Invalid auth argument"));
                }
                let link_code = parts[1];
                client::auth_client(
                    "auth.key",
                    "public.key",
                    "private.key",
                    &host,
                    link_code,
                    auth_port,
                )
                .await?;
            }

            client::run(
                tun_dev,
                host,
                port,
                transport_type,
                loopback_relay,
                no_encryption,
                "auth.key",
                "public.key",
                "private.key",
                None,
            )
            .await?
            .await?;

            Ok(())
        })?;
    } else {
        echo_syntax(&args);
        std::process::exit(1);
    }
    Ok(())
}
