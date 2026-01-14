use anyhow::Result;
use dotenv::dotenv;
use log::info;
use netplane_client::client;
use tauri::Error;

type TauriResult<T> = Result<T, Error>;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn client(server: &str, auth: &str, transport: &str) -> TauriResult<()> {
    dotenv().ok();

    if let Err(err) = netplane_common::crypto::try_generate_crypto_keys("public.key", "private.key")
    {
        if err.kind() != std::io::ErrorKind::AlreadyExists {
            return Err(anyhow::Error::from(err).into());
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

    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
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
