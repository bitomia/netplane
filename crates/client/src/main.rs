use std::env;

use anyhow::{Result, anyhow};
use dotenv::dotenv;
use tracing::info;

pub mod client;
pub mod client_manager;
mod fd;
mod http_client;
mod routes;
mod tundev;

const AUTH_KEY_FILENAME: &str = "auth.key";
const DYNAMIC_KEY_FILENAME: &str = "dynamic.key";

#[cfg(not(any(target_os = "android", target_os = "ios")))]
#[allow(dead_code)]
fn echo_syntax(msg: Option<String>, args: &[String]) {
    if let Some(msg) = msg {
        print!("{}. ", msg);
    }
    println!(
        "Use {} [server] [--port=5000] [--tun=device] [--link=link_code|--dynamic-link=dynamic_link_code] [--auth-port=8000] [--transport=udp|websocket] [--exit-node=sdn_ip] [--loopback-relay] [--no-encryption]",
        args[0]
    );
}

#[cfg(not(any(target_os = "android", target_os = "ios")))]
#[allow(dead_code)]
fn main() -> Result<()> {
    let log_format = env::args()
        .find_map(|a| a.strip_prefix("--log-format=").map(str::to_string))
        .as_deref()
        .map(|v| match v {
            "json" => client::LogFormat::Json,
            "logfmt" => client::LogFormat::Logfmt,
            _ => client::LogFormat::Pretty,
        })
        .unwrap_or(client::LogFormat::Pretty);
    client::init_logger(log_format);
    info!("Netplane client rev {}", netplane_common::git_rev_main!());

    dotenv().ok();

    let args: Vec<String> = env::args().collect();
    if args.len() >= 2 {
        if let Err(err) =
            netplane_common::crypto::try_generate_crypto_keys("public.key", "private.key")
            && err.kind() != std::io::ErrorKind::AlreadyExists
        {
            return Err(anyhow!(err));
        }

        let mut link_code = None;
        let mut dynamic_link_code = None;
        let mut transport_type = None;
        let mut tun_dev = "tun0".to_string();
        let mut port = None;
        let mut auth_port = None;
        let mut loopback_relay = false;
        let mut no_encryption = false;
        let mut exit_node: Option<String> = None;

        if args[1] == "--help" {
            echo_syntax(None, &args);
            std::process::exit(1);
        }
        // Parse optional arguments
        for arg in &args[2..] {
            if arg.starts_with("--dynamic-link") {
                let parts: Vec<&str> = arg.split("=").collect();
                if parts.len() != 2 {
                    return Err(anyhow!("Invalid dynamic-link argument"));
                }
                dynamic_link_code = Some(parts[1]);
            } else if arg.starts_with("--link=") {
                let parts: Vec<&str> = arg.split("=").collect();
                if parts.len() != 2 {
                    return Err(anyhow!("Invalid link argument"));
                }
                link_code = Some(parts[1]);
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
            } else if arg.starts_with("--exit-node=") {
                exit_node = arg.split('=').nth(1).map(|s| s.to_string());
            } else if arg.starts_with("--help") {
                echo_syntax(None, &args);
                std::process::exit(1);
            } else {
                echo_syntax(Some("Argument not recognized".to_string()), &args);
                std::process::exit(1);
            }
        }

        if dynamic_link_code.is_some() && link_code.is_some() {
            println!("Cannot set both dynamic-link and link options");
            return Err(anyhow!("Cannot set both dynamic-link and link options"));
        }

        let rt = tokio::runtime::Runtime::new()?;
        rt.block_on(async {
            let host = args[1].clone();

            let (authkey_filepath, link_code, is_dynamic_link): (&str, Option<String>, bool) =
                if let Some(dynamic_link_code) = dynamic_link_code {
                    (
                        DYNAMIC_KEY_FILENAME,
                        Some(dynamic_link_code.to_string()),
                        true,
                    )
                } else if let Some(link_code) = link_code {
                    (AUTH_KEY_FILENAME, Some(link_code.to_string()), false)
                } else if std::fs::exists(DYNAMIC_KEY_FILENAME)? {
                    (DYNAMIC_KEY_FILENAME, None, true)
                } else if std::fs::exists(AUTH_KEY_FILENAME)? {
                    (AUTH_KEY_FILENAME, None, false)
                } else {
                    echo_syntax(Some("Client not linked".to_string()), &args);
                    std::process::exit(1);
                };

            if let Some(link_code) = link_code {
                client::auth_client(
                    authkey_filepath,
                    "public.key",
                    "private.key",
                    &host,
                    link_code.as_str(),
                    is_dynamic_link,
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
                is_dynamic_link,
                authkey_filepath,
                "public.key",
                "private.key",
                None,
                None,
                exit_node,
            )
            .await?
            .await?;

            Ok::<(), anyhow::Error>(())
        })?;
    } else {
        echo_syntax(None, &args);
        std::process::exit(1);
    }
    Ok(())
}
