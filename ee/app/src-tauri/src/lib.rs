use anyhow::anyhow;
use dotenv::dotenv;
use log::info;
use netplane_client::client;
use std::sync::Mutex;
use tokio_util::sync::CancellationToken;

struct AppState {
    disconnect_token: Mutex<CancellationToken>,
}

#[tauri::command]
async fn client(
    app_state: tauri::State<'_, AppState>,
    server: &str,
    auth: &str,
    transport: &str,
) -> tauri::Result<()> {
    dotenv().ok();

    if server.is_empty() {
        return Err(anyhow!("ERROR: No hay servidor").into())
    }

    if let Err(err) = netplane_common::crypto::try_generate_crypto_keys("public.key", "private.key")
    {
        if err.kind() != std::io::ErrorKind::AlreadyExists {
            return Err(err.into());
        }
    }

    let host = server.to_string();
    let mut transport_type: Option<String> = None;
    let mut tun_dev = "tun0".to_string();
    let mut port: Option<u16> = Some(5000);
    let mut auth_port: Option<u16> = Some(8000);
    let mut loopback_relay = false;
    let mut no_encryption = false;

    if !transport.is_empty() {
        transport_type = Some(transport.to_string());
    }

    if !auth.is_empty() {
        let link_code = auth;

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

    let cloned_token = {
        let mut token = match app_state.disconnect_token.lock() {
            Ok(t) => t,
            Err(e) => return Err(anyhow!("{}", e).into()),
        };

        if (*token).is_cancelled() {
            *token = CancellationToken::new();
        }

        (*token).clone()
    };

    client::run(
        tun_dev,
        host,
        port,
        transport_type,
        loopback_relay,
        no_encryption,
        Some(cloned_token),
    )
    .await?;

    Ok(())
}

#[tauri::command]
async fn stop_update(token_state: tauri::State<'_, AppState>) -> tauri::Result<()> {
    token_state.disconnect_token.lock().unwrap().cancel();

    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    client::init_logger();

    info!("Netplane app starting");

    tauri::Builder::default()
        .manage(AppState {
            disconnect_token: Mutex::new(CancellationToken::new()),
        })
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![client, stop_update])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
