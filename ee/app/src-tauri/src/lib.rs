use log::info;
use anyhow::{Result, anyhow};
use netplane_client::client;
use dotenv::dotenv;
use tauri::Error;

type TauriResult<T> = Result<T, Error>;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn client(server: &str, auth: &str, transport: &str) -> TauriResult<()> {
    dotenv().ok();

    if let Err(err) =
        netplane_common::crypto::try_generate_crypto_keys("public.key", "private.key")
    {
        if err.kind() != std::io::ErrorKind::AlreadyExists {
            return Err(anyhow::Error::from(err).into());
        }
    }

    let host = server.to_string();
    let mut auth_arg: Option<String> = None;
    let mut transport_type: Option<String> = None;
    let mut tun_dev = "tun0".to_string();
    let mut port: Option<u16> = Some(5000);
    let mut auth_port: Option<u16> = Some(8000);
    let mut loopback_relay = false;
    let mut no_encryption = false;

    if !auth.is_empty() {
        auth_arg = Some(format!("--auth={}",auth).to_string());
    } 

    if !transport.is_empty() {
        transport_type = Some(transport.to_string());
    }

   #[cfg(not(all(feature = "tray", target_os = "macos")))]
    {
        let rt = tokio::runtime::Runtime::new()?;
        rt.block_on(async {
            #[cfg(all(feature = "tray", any(target_os = "windows", target_os = "linux")))]
            let tray_rx = tray::init_tray().ok();

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

            client::run(
                tun_dev,
                host,
                port,
                transport_type,
                loopback_relay,
                no_encryption,
            )
            .await
        })?;
    }
    
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    client::init_logger();

    info!("Netplane app starting");

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![client])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
