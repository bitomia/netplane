use anyhow::anyhow;
use dotenv::dotenv;
use log::info;

use netplane_client::client;

#[cfg(target_os = "android")]
use netplane_client::{client::create_transport, client::StartParams, fd::PlatformFd};
#[cfg(target_os = "android")]
use tauri_plugin_netplane_vpn_manager::NetplaneVpnManagerExt;

use std::{path::PathBuf, sync::Mutex};
use tauri::Manager;
use tauri_plugin_log::{Target, TargetKind};
use tokio_util::sync::CancellationToken;

#[cfg(not(target_os = "android"))]
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
};

struct AppState {
    disconnect_token: Mutex<CancellationToken>,
}

#[tauri::command]
async fn client(
    app: tauri::AppHandle,
    app_state: tauri::State<'_, AppState>,
    server: &str,
    auth: &str,
    transport: &str,
) -> tauri::Result<()> {
    let key_directory = if cfg!(target_os = "android") {
        app.path().app_data_dir()?
    } else {
        PathBuf::new()
    };

    let authkey_path = key_directory
        .join("auth.key")
        .into_os_string()
        .into_string()
        .expect("auth.key path should not be empty");

    let public_filepath = key_directory
        .join("public.key")
        .into_os_string()
        .into_string()
        .expect("public.key path should not be empty");

    let private_filepath = key_directory
        .join("private.key")
        .into_os_string()
        .into_string()
        .expect("private.key path should not be empty");

    dotenv().ok();

    if server.is_empty() {
        return Err(anyhow!("ERROR: No hay servidor").into());
    }

    if let Err(err) =
        netplane_common::crypto::try_generate_crypto_keys(&public_filepath, &private_filepath)
    {
        if err.kind() != std::io::ErrorKind::AlreadyExists {
            return Err(err.into());
        }
    }

    let host = server.to_string();
    let mut transport_type: Option<String> = None;
    let tun_dev = "tun0".to_string();
    let port: Option<u16> = Some(5000);
    let auth_port: Option<u16> = Some(8000);
    let loopback_relay = false;
    let no_encryption = false;

    if !transport.is_empty() {
        transport_type = Some(transport.to_string());
    }

    if !auth.is_empty() {
        let link_code = auth;

        client::auth_client(
            &authkey_path,
            &public_filepath,
            &private_filepath,
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

    #[cfg(not(target_os = "android"))]
    {
        client::run(
            tun_dev,
            host,
            port,
            transport_type,
            loopback_relay,
            no_encryption,
            &authkey_path,
            &public_filepath,
            &private_filepath,
            Some(cloned_token),
        )
        .await?;
    }
    #[cfg(target_os = "android")]
    {
        let tunnel_fd = app
            .netplane_vpn_manager()
            .get_tunnel_fd()
            .map_err(|e| anyhow!("Failed to get tunnel fd: {}", e))?;
        let fd = PlatformFd::Unix(tunnel_fd.fd);
        let start_params = StartParams {
            netmask: "255.255.255.0".to_string(),
            destination: "192.168.1.37".to_string(),
            ip_addr: "10.0.0.3".to_string(),
        };

        let control_addr = format!("{}:{}", host, port.unwrap_or(5000));

        let mut transport = create_transport(&control_addr, transport_type).await?;

        client::run_from_fd(
            fd,
            &start_params,
            transport,
            loopback_relay,
            no_encryption,
            &public_filepath,
            &private_filepath,
        )
        .await?;
    }

    Ok(())
}

#[tauri::command]
async fn stop_update(token_state: tauri::State<'_, AppState>) -> tauri::Result<()> {
    token_state.disconnect_token.lock().unwrap().cancel();

    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(
            tauri_plugin_log::Builder::new()
                .targets([Target::new(TargetKind::Stdout)])
                .build(),
        )
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_netplane_vpn_manager::init())
        .manage(AppState {
            disconnect_token: Mutex::new(CancellationToken::new()),
        })
        .setup(|app| {
            client::init_logger();
            info!("Netplane app starting");

            #[cfg(not(target_os = "android"))]
            {
                let show = MenuItem::with_id(app, "show", "Show", true, None::<&str>)?;
                let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
                let menu = Menu::with_items(app, &[&show, &quit])?;

                TrayIconBuilder::with_id("main-tray")
                    .icon(app.default_window_icon().unwrap().clone())
                    .menu(&menu)
                    .show_menu_on_left_click(false)
                    .on_menu_event(|app, event| match event.id.as_ref() {
                        "show" => {
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                        "quit" => {
                            app.exit(0);
                        }
                        _ => {}
                    })
                    .on_tray_icon_event(|tray, event| {
                        if let tauri::tray::TrayIconEvent::Click {
                            button: tauri::tray::MouseButton::Left,
                            button_state: tauri::tray::MouseButtonState::Up,
                            ..
                        } = event
                        {
                            let app = tray.app_handle();
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                    })
                    .build(app)?;
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![client, stop_update])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
