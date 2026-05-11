use dotenv::dotenv;
use tracing::info;
use netplane_client::client;

#[cfg(target_os = "android")]
use anyhow::anyhow;
#[cfg(target_os = "android")]
use netplane_client::{client::create_transport, client::StartParams, fd::PlatformFd};
#[cfg(target_os = "android")]
use tauri_plugin_netplane_vpn_manager::NetplaneVpnManagerExt;

use std::path::{Path, PathBuf};
use tokio_util::sync::CancellationToken;

#[cfg(not(target_os = "android"))]
use tauri::{
    image::Image,
    menu::{Menu, MenuItem},
    Emitter, Manager,
};

#[cfg(target_os = "android")]
use tauri::{Emitter, Manager};

use crate::error::AppError;

#[tauri::command]
pub fn get_version() -> String {
    let rev = include_str!("../../../../.git/refs/heads/main");
    rev.chars().take(8).collect()
}

#[tauri::command]
pub async fn start_client(
    app_handle: tauri::AppHandle,
    app_state: tauri::State<'_, crate::AppState>,
    server: &str,
    auth: &str,
    transport: &str,
) -> tauri::Result<()> {
    let key_directory = if cfg!(target_os = "android") {
        app_handle.path().app_data_dir()?
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

    app_handle
        .emit("connecting", ())
        .expect("start connecting emit error");

    dotenv().ok();

    if server.is_empty() {
        tracing::error!("No server");
        app_handle
            .emit("connect_error", AppError::NoServer.to_string())
            .expect("no server emit error");
        return Ok(());
    }

    if let Err(err) =
        netplane_common::crypto::try_generate_crypto_keys(&public_filepath, &private_filepath)
    {
        if err.kind() != std::io::ErrorKind::AlreadyExists {
            tracing::error!("crypto_keys error: {:?}", err);
            app_handle
                .emit("connect_error", AppError::GenericError.to_string())
                .expect("crypto_keys error emit error");
            return Ok(());
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

        if let Err(err) = client::auth_client(
            &authkey_path,
            &public_filepath,
            &private_filepath,
            &host,
            &link_code,
            auth_port,
        )
        .await
        {
            //TODO: Change literal str for client error types
            match err.to_string().as_str() {
                "Auth failed: \"Couldn't open file\"" => {
                    tracing::error!("Could not open auth.key");
                    app_handle
                        .emit("connect_error", AppError::AuthKey.to_string())
                        .expect("auth emit error");
                }
                "Link failed" => {
                    tracing::error!("Invalid link code");
                    app_handle
                        .emit("connect_error", AppError::AuthCode.to_string())
                        .expect("auth emit error");
                }
                _ => {}
            }
            return Ok(());
        }
    }

    let cloned_token = {
        let mut token = match app_state.disconnect_token.lock() {
            Ok(t) => t,
            Err(err) => {
                tracing::error!("Token could not be adquired: {:?}", err);
                app_handle
                    .emit("connect_error", AppError::GenericError.to_string())
                    .expect("cloned_token emit error");
                return Ok(());
            }
        };

        if (*token).is_cancelled() {
            *token = CancellationToken::new();
        }

        (*token).clone()
    };

    let state_tx = app_state
        .state_tx
        .lock()
        .expect("state_tx lock poisoned")
        .clone();

    #[cfg(not(target_os = "android"))]
    {
        if let Err(err) = client::run(
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
            state_tx,
        )
        .await
        {
            tracing::error!("Error when executing run: {:?}", err);
            app_handle
                .emit("connect_error", AppError::GenericError.to_string())
                .expect("run emit error");
            return Ok(());
        }
    }
    #[cfg(target_os = "android")]
    {
        use tauri_plugin_netplane_vpn_manager::StartVpnRequest;

        let vpn_permission = app_handle
            .netplane_vpn_manager()
            .request_vpn_permission()
            .map_err(|e| anyhow!("Failed to request VPN permission: {}", e))?;

        if !vpn_permission.granted {
            tracing::error!("VPN permission not granted");
            app_handle
                .emit("connect_error", AppError::GenericError.to_string())
                .expect("vpn permission emit error");
            return Ok(());
        }

        let start_params = StartParams {
            netmask: "255.255.255.0".to_string(),
            destination: "192.168.1.37".to_string(),
            ip_addr: "10.0.0.3".to_string(),
        };

        let prefix_length = 24;
        let vpn_ip: std::net::Ipv4Addr = start_params
            .ip_addr
            .parse()
            .map_err(|e| anyhow!("Invalid VPN IP: {}", e))?;
        let mask = if prefix_length == 0 {
            0u32
        } else {
            !0u32 << (32 - prefix_length)
        };
        let network_ip = std::net::Ipv4Addr::from(u32::from(vpn_ip) & mask);

        let vpn_response = app_handle
            .netplane_vpn_manager()
            .start_vpn(StartVpnRequest {
                address: start_params.ip_addr.clone(),
                route_address: network_ip.to_string(),
                prefix_length,
            })
            .map_err(|e| anyhow!("Failed to start VPN: {}", e))?;

        if vpn_response.fd < 0 {
            tracing::error!("VPN tunnel fd is invalid: {}", vpn_response.fd);
            app_handle
                .emit("connect_error", AppError::GenericError.to_string())
                .expect("vpn fd emit error");
            return Ok(());
        }

        let fd = PlatformFd::Unix(vpn_response.fd);

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
            state_tx,
        )
        .await?;
    }

    #[cfg(not(target_os = "android"))]
    if let Some(tray) = app_handle.tray_by_id("main-tray") {
        let connected_icon = match Image::from_path(Path::new("icons/connected/connected.png")) {
            Ok(image) => image,
            Err(err) => {
                tracing::error!("connected.png not found: {:?}", err);
                app_handle
                    .emit("connect_error", AppError::GenericError.to_string())
                    .expect("connected_icon emit error");
                return Ok(());
            }
        };

        if let Err(err) = tray.set_icon(Some(connected_icon)) {
            tracing::error!("Could not set icon: {:?}", err);
            app_handle
                .emit("connect_error", AppError::GenericError.to_string())
                .expect("set_icon connect emit error");
            return Ok(());
        };

        let show_item = match MenuItem::with_id(&app_handle, "show", "Show", true, None::<&str>) {
            Ok(menu_item) => menu_item,
            Err(err) => {
                tracing::error!("Could not create show item: {:?}", err);
                app_handle
                    .emit("connect_error", AppError::GenericError.to_string())
                    .expect("show_item connect emit error");
                return Ok(());
            }
        };

        let quit_item = match MenuItem::with_id(&app_handle, "quit", "Quit", true, None::<&str>) {
            Ok(menu_item) => menu_item,
            Err(err) => {
                tracing::error!("Could not create quit item: {:?}", err);
                app_handle
                    .emit("connect_error", AppError::GenericError.to_string())
                    .expect("quit_item connect emit error");
                return Ok(());
            }
        };

        let disconnect_item =
            match MenuItem::with_id(&app_handle, "disconnect", "Disconnect", true, None::<&str>) {
                Ok(menu_item) => menu_item,
                Err(err) => {
                    tracing::error!("Could not create disconnect item: {:?}", err);
                    app_handle
                        .emit("connect_error", AppError::GenericError.to_string())
                        .expect("disconnect_item connect emit error");
                    return Ok(());
                }
            };

        let menu = match Menu::with_items(&app_handle, &[&show_item, &quit_item, &disconnect_item])
        {
            Ok(menu) => menu,
            Err(err) => {
                tracing::error!("Could not create connect menu: {:?}", err);
                app_handle
                    .emit("connect_error", AppError::GenericError.to_string())
                    .expect("menu connect emit error");
                return Ok(());
            }
        };

        if let Err(err) = tray.set_menu(Some(menu)) {
            tracing::error!("Could not set connect menu: {:?}", err);
            app_handle
                .emit("disconnect_error", AppError::GenericError.to_string())
                .expect("set_menu connect emit error");
            return Ok(());
        }
    }

    info!("Connecting to dashboard");
    app_handle
        .emit("connected", ())
        .expect("finish connecting emit error");

    Ok(())
}

#[tauri::command]
pub async fn stop_update(app_handle: tauri::AppHandle) {
    app_handle
        .emit("disconnecting", ())
        .expect("start disconnecting emit error");
    disconnect(app_handle).await
}

pub async fn disconnect(app_handle: tauri::AppHandle) {
    let app_state = app_handle.state::<crate::AppState>();

    match app_state.disconnect_token.lock() {
        Ok(lock) => lock.cancel(),
        Err(err) => {
            tracing::error!("Token could not be adquired: {:?}", err);
            app_handle
                .emit("disconnect_error", AppError::GenericError.to_string())
                .expect("disconnect_token emit error");
            return;
        }
    };

    #[cfg(target_os = "android")]
    {
        use tauri_plugin_netplane_vpn_manager::NetplaneVpnManagerExt;
        if let Err(err) = app_handle.netplane_vpn_manager().stop_vpn() {
            tracing::error!("Failed to stop VPN: {:?}", err);
        }
    }

    #[cfg(not(target_os = "android"))]
    if let Some(tray) = app_handle.tray_by_id("main-tray") {
        let disconnected_icon =
            match Image::from_path(Path::new("icons/disconnected/disconnected.ico")) {
                Ok(image) => image,
                Err(err) => {
                    tracing::error!("disconnected.png not found: {:?}", err);
                    app_handle
                        .emit("disconnect_error", AppError::GenericError.to_string())
                        .expect("disconnected_icon emit error");
                    return;
                }
            };

        if let Err(err) = tray.set_icon(Some(disconnected_icon)) {
            tracing::error!("Could not set icon: {:?}", err);
            app_handle
                .emit("disconnect_error", AppError::GenericError.to_string())
                .expect("set_icon disconnect emit error");
            return;
        }

        let show_item = match MenuItem::with_id(&app_handle, "show", "Show", true, None::<&str>) {
            Ok(menu_item) => menu_item,
            Err(err) => {
                tracing::error!("Could not create show item: {:?}", err);
                app_handle
                    .emit("disconnect_error", AppError::GenericError.to_string())
                    .expect("show_item disconnect emit error");
                return;
            }
        };

        let quit_item = match MenuItem::with_id(&app_handle, "quit", "Quit", true, None::<&str>) {
            Ok(menu_item) => menu_item,
            Err(err) => {
                tracing::error!("Could not create quit item: {:?}", err);
                app_handle
                    .emit("disconnect_error", AppError::GenericError.to_string())
                    .expect("quit_item disconnect emit error");
                return;
            }
        };

        let menu = match Menu::with_items(&app_handle, &[&show_item, &quit_item]) {
            Ok(menu) => menu,
            Err(err) => {
                tracing::error!("Could not create disconnect menu: {:?}", err);
                app_handle
                    .emit("disconnect_error", AppError::GenericError.to_string())
                    .expect("menu disconnect emit error");
                return;
            }
        };

        if let Err(err) = tray.set_menu(Some(menu)) {
            tracing::error!("Could not set disconnect menu: {:?}", err);
            app_handle
                .emit("disconnect_error", AppError::GenericError.to_string())
                .expect("set_menu disconnect emit error");
            return;
        }
    }

    info!("Disconnecting from dashboard");
    app_handle
        .emit("disconnected", ())
        .expect("finish disconnecting emit error");
}
