use dotenv::dotenv;
use log::info;
use netplane_client::client;
use std::path::Path;
use std::sync::Mutex;
use tauri::{
    image::Image,
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    Emitter, Manager,
};
use tokio_util::sync::CancellationToken;

struct AppState {
    disconnect_token: Mutex<CancellationToken>,
}

#[tauri::command]
async fn client(app_handle: tauri::AppHandle, server: String, auth: String, transport: String) {
    app_handle
        .emit("connecting", ())
        .expect("start connecting emit error");

    let app_state = app_handle.state::<AppState>();

    dotenv().ok();

    if server.is_empty() {
        app_handle
            .emit("connect_error", "No server".to_string())
            .expect("no server emit error");
        log::error!("no server emit error");
        return;
    }

    if let Err(err) = netplane_common::crypto::try_generate_crypto_keys("public.key", "private.key")
    {
        if err.kind() != std::io::ErrorKind::AlreadyExists {
            app_handle
                .emit("connect_error", err.to_string())
                .expect("alreadyExists crypto_keys emit error");
            log::error!("alreadyExists crypto_keys emit error");
            return;
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
            "auth.key",
            "public.key",
            "private.key",
            &host,
            &link_code,
            auth_port,
        )
        .await
        {
            app_handle
                .emit("connect_error", err.to_string())
                .expect("auth emit error");
            log::error!("auth emit error");
            return;
        }
    }

    let cloned_token = {
        let mut token = match app_state.disconnect_token.lock() {
            Ok(t) => t,
            Err(err) => {
                app_handle
                    .emit("connect_error", err.to_string())
                    .expect("cloned_token emit error");
                log::error!("cloned_token emit error");
                return;
            }
        };

        if (*token).is_cancelled() {
            *token = CancellationToken::new();
        }

        (*token).clone()
    };

    if let Err(err) = client::run(
        tun_dev,
        host,
        port,
        transport_type,
        loopback_relay,
        no_encryption,
        Some(cloned_token),
    )
    .await
    {
        app_handle
            .emit("connect_error", err.to_string())
            .expect("run emit error");
        log::error!("run emit error");
        return;
    }

    if let Some(tray) = app_handle.tray_by_id("main-tray") {
        let connected_icon = match Image::from_path(Path::new("icons/connected/connected.png")) {
            Ok(image) => image,
            Err(err) => {
                app_handle
                    .emit("connect_error", err.to_string())
                    .expect("connected_icon emit error");
                log::error!("connected_icon emit error");
                return;
            }
        };

        if let Err(err) = tray.set_icon(Some(connected_icon)) {
            app_handle
                .emit("connect_error", err.to_string())
                .expect("set_icon connect emit error");
            log::error!("set_icon connect emit error");
            return;
        };

        let show_item = match MenuItem::with_id(&app_handle, "show", "Show", true, None::<&str>) {
            Ok(menu_item) => menu_item,
            Err(err) => {
                app_handle
                    .emit("connect_error", err.to_string())
                    .expect("show_item connect emit error");
                log::error!("show_item connect emit error");
                return;
            }
        };

        let quit_item = match MenuItem::with_id(&app_handle, "quit", "Quit", true, None::<&str>) {
            Ok(menu_item) => menu_item,
            Err(err) => {
                app_handle
                    .emit("connect_error", err.to_string())
                    .expect("quit_item connect emit error");
                log::error!("quit_item connect emit error");
                return;
            }
        };

        let disconnect_item =
            match MenuItem::with_id(&app_handle, "disconnect", "Disconnect", true, None::<&str>) {
                Ok(menu_item) => menu_item,
                Err(err) => {
                    app_handle
                        .emit("connect_error", err.to_string())
                        .expect("disconnect_item connect emit error");
                    log::error!("disconnect_item connect emit error");
                    return;
                }
            };

        let menu = match Menu::with_items(&app_handle, &[&show_item, &quit_item, &disconnect_item])
        {
            Ok(menu) => menu,
            Err(err) => {
                app_handle
                    .emit("connect_error", err.to_string())
                    .expect("menu connect emit error");
                log::error!("menu connect emit error");
                return;
            }
        };

        if let Err(err) = tray.set_menu(Some(menu)) {
            app_handle
                .emit("disconnect_error", err.to_string())
                .expect("set_menu connect emit error");
            log::error!("set_menu connect emit error");
            return;
        }
    }

    info!("Connecting to dashboard");
    app_handle
        .emit("connected", ())
        .expect("finish connecting emit error");
}

#[tauri::command]
async fn stop_update(app_handle: tauri::AppHandle) {
    app_handle
        .emit("disconnecting", ())
        .expect("start disconnecting emit error");
    disconnect(app_handle).await
}

async fn disconnect(app_handle: tauri::AppHandle) {
    let app_state = app_handle.state::<AppState>();

    match app_state.disconnect_token.lock() {
        Ok(lock) => lock.cancel(),
        Err(err) => {
            app_handle
                .emit("disconnect_error", err.to_string())
                .expect("disconnect_token emit error");
            log::error!("disconnect_token emit error");
            return;
        }
    };

    if let Some(tray) = app_handle.tray_by_id("main-tray") {
        let disconnected_icon =
            match Image::from_path(Path::new("icons/disconnected/disconnected.ico")) {
                Ok(image) => image,
                Err(err) => {
                    app_handle
                        .emit("disconnect_error", err.to_string())
                        .expect("disconnected_icon emit error");
                    log::error!("disconnected_icon emit error");
                    return;
                }
            };

        if let Err(err) = tray.set_icon(Some(disconnected_icon)) {
            app_handle
                .emit("disconnect_error", err.to_string())
                .expect("set_icon disconnect emit error");
            log::error!("set_icon disconnect emit error");
            return;
        }

        let show_item = match MenuItem::with_id(&app_handle, "show", "Show", true, None::<&str>) {
            Ok(menu_item) => menu_item,
            Err(err) => {
                app_handle
                    .emit("disconnect_error", err.to_string())
                    .expect("show_item disconnect emit error");
                log::error!("show_item disconnect emit error");
                return;
            }
        };

        let quit_item = match MenuItem::with_id(&app_handle, "quit", "Quit", true, None::<&str>) {
            Ok(menu_item) => menu_item,
            Err(err) => {
                app_handle
                    .emit("disconnect_error", err.to_string())
                    .expect("quit_item disconnect emit error");
                log::error!("quit_item disconnect emit error");
                return;
            }
        };

        let menu = match Menu::with_items(&app_handle, &[&show_item, &quit_item]) {
            Ok(menu) => menu,
            Err(err) => {
                app_handle
                    .emit("disconnect_error", err.to_string())
                    .expect("menu disconnect emit error");
                log::error!("menu disconnect emit error");
                return;
            }
        };

        if let Err(err) = tray.set_menu(Some(menu)) {
            app_handle
                .emit("disconnect_error", err.to_string())
                .expect("set_menu disconnect emit error");
            log::error!("set_menu disconnect emit error");
            return;
        }
    }

    info!("Disconnecting from dashboard");
    app_handle
        .emit("disconnected", ())
        .expect("finish disconnecting emit error");
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
            let show_item = MenuItem::with_id(app, "show", "Show", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

            let menu = Menu::with_items(app, &[&show_item, &quit_item])?;

            let disconnected_icon =
                Image::from_path(Path::new("icons/disconnected/disconnected.ico"))?;

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
                        tauri::async_runtime::spawn(async move {
                            disconnect(app_handle).await;
                        });
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
