use netplane_client::client::ClientState;
use std::sync::Mutex;
use tauri::Emitter;
#[cfg(not(target_os = "android"))]
use tauri::{
    image::Image,
    menu::{Menu, MenuItem},
    path::BaseDirectory,
    tray::TrayIconBuilder,
    Manager,
};
use tokio_util::sync::CancellationToken;
use tracing_subscriber::EnvFilter;

mod commands;
mod error;

pub struct AppState {
    disconnect_token: Mutex<CancellationToken>,
    #[allow(dead_code)]
    client_state: ClientState,
    state_tx: Mutex<Option<tokio::sync::watch::Sender<ClientState>>>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_opener::init());

    #[cfg(any(target_os = "android", target_os = "ios"))]
    let builder = builder.plugin(tauri_plugin_netplane_vpn_manager::init());

    let (state_tx, mut state_rx) = tokio::sync::watch::channel(ClientState {});

    builder
        .manage(AppState {
            disconnect_token: Mutex::new(CancellationToken::new()),
            client_state: ClientState {},
            state_tx: Mutex::new(Some(state_tx)),
        })
        .setup(|app| {
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                while state_rx.changed().await.is_ok() {
                    let state = state_rx.borrow().clone();
                    app_handle.emit("state-updated", state).unwrap();
                }
            });

            #[cfg(not(target_os = "android"))]
            {
                let show_item = MenuItem::with_id(app, "show", "Show", true, None::<&str>)?;
                let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

                let menu = Menu::with_items(app, &[&show_item, &quit_item])?;
                let disconnected_icon_path = app.path().resolve(
                    "icons/disconnected/disconnected.ico",
                    BaseDirectory::Resource,
                )?;
                let disconnected_icon = Image::from_path(disconnected_icon_path)?;

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
                                commands::disconnect(app_handle).await;
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
            }

            Ok(())
        })
        .on_window_event(|window, event| if let tauri::WindowEvent::CloseRequested { api, .. } = event {
            window.hide().unwrap();
            api.prevent_close();
        })
        .invoke_handler(tauri::generate_handler![
            commands::start_client,
            commands::stop_update,
            commands::get_version
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
