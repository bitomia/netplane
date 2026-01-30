use anyhow::anyhow;
use dotenv::dotenv;
use log::info;
use netplane_client::client;
use std::sync::Mutex;
use std::path::Path;
use tauri::{
    Emitter, Manager, image::Image, menu::{Menu, MenuItem}, tray::TrayIconBuilder
};
use tokio_util::sync::CancellationToken;

struct AppState {
    disconnect_token: Mutex<CancellationToken>,
}

#[tauri::command]
async fn client(
    app_handle: tauri::AppHandle,
    server: &str,
    auth: &str,
    transport: &str,
) -> tauri::Result<()> {
    let app_state = app_handle.state::<AppState>();

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

    if let Some(tray) = app_handle.tray_by_id("main-tray") {
        let connected_icon = Image::from_path(Path::new("icons/connected.ico")).unwrap();
        tray.set_icon(Some(connected_icon))?;

        let show = MenuItem::with_id(&app_handle, "show", "Show", true, None::<&str>)?;
        let quit = MenuItem::with_id(&app_handle, "quit", "Quit", true, None::<&str>)?;
        let disconnect = MenuItem::with_id(&app_handle, "disconnect", "Disconnect", true, None::<&str>)?;
        let menu = Menu::with_items(&app_handle, &[&show, &quit, &disconnect])?;

        tray.set_menu(Some(menu))?;

    }

    Ok(())
}

#[tauri::command]
async fn stop_update(app_handle: tauri::AppHandle) -> tauri::Result<()> {
    inner_stop_update(app_handle).await
}

async fn inner_stop_update(app_handle: tauri::AppHandle) -> tauri::Result<()> {
    let app_state = app_handle.state::<AppState>();

    app_state.disconnect_token.lock().unwrap().cancel();

    if let Some(tray) = app_handle.tray_by_id("main-tray") {
        let disconnected_icon = Image::from_path(Path::new("icons/disconnected.ico")).unwrap();
        tray.set_icon(Some(disconnected_icon))?;

        let show = MenuItem::with_id(&app_handle, "show", "Show", true, None::<&str>)?;
        let quit = MenuItem::with_id(&app_handle, "quit", "Quit", true, None::<&str>)?;

        let menu = Menu::with_items(&app_handle, &[&show, &quit])?;

        tray.set_menu(Some(menu))?;
    }

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
        .setup(|app| {

            let show = MenuItem::with_id(app, "show", "Show", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

            let menu = Menu::with_items(app, &[&show, &quit])?;

            let disconnected_icon = Image::from_path(Path::new("icons/disconnected.ico")).unwrap();

            TrayIconBuilder::with_id("main-tray")
                .icon(disconnected_icon)
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
                    "disconnect" => {

                        let app_handle = app.clone();
                        tauri::async_runtime::spawn( async move {
                           inner_stop_update(app_handle).await;
                        });
                        app.emit("disconnect", ()).expect("Can't disconnect");
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

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![client, stop_update])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
