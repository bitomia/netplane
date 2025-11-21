#[cfg(all(feature = "tray", any(target_os = "windows", target_os = "macos", target_os = "linux")))]
use anyhow::Result;
#[cfg(all(feature = "tray", any(target_os = "windows", target_os = "macos", target_os = "linux")))]
use log::info;

#[cfg(all(feature = "tray", any(target_os = "windows", target_os = "macos", target_os = "linux")))]
#[allow(dead_code)]
pub enum TrayMessage {
    Quit,
}

// macOS-specific tray initialization that runs display() on main thread
// This function blocks until the app quits
#[cfg(all(feature = "tray", target_os = "macos"))]
#[allow(dead_code)]
pub fn init_tray_and_display() -> Result<()> {
    use tray_item::{IconSource, TrayItem};

    let icon_raw = include_bytes!("../assets/connected.ico");
    let icon_green = IconSource::Data {
        height: 64,
        width: 64,
        data: icon_raw.to_vec(),
    };
    let mut tray = TrayItem::new("Netplane", icon_green)?;
    tray.add_label("Netplane Client").ok();

    tray.add_menu_item("Quit", move || {
        info!("Quit clicked from tray");
        std::process::exit(0);
    })
    .ok();

    info!("Tray indicator initialized");

    // Display the tray (this blocks on main thread, which is required for macOS)
    let inner = tray.inner_mut();
    inner.display();

    Ok(())
}

// Non-macOS tray initialization
#[cfg(all(feature = "tray", any(target_os = "windows", target_os = "linux")))]
pub fn init_tray() -> Result<std::sync::mpsc::Receiver<TrayMessage>> {
    use tray_item::{IconSource, TrayItem};

    let (tx, rx) = std::sync::mpsc::channel();

    let icon_raw = include_bytes!("../assets/icon-red.ico");
    let icon_green = IconSource::Data {
        height: 64,
        width: 64,
        data: icon_raw.to_vec(),
    };
    let mut tray = TrayItem::new("Netplane", icon_green)?;
    tray.add_label("Netplane Client").ok();

    let quit_tx = tx.clone();
    tray.add_menu_item("Quit", move || {
        info!("Quit clicked from tray");
        quit_tx.send(TrayMessage::Quit).ok();
    })
    .ok();

    info!("Tray indicator initialized");

    // Keep the tray alive for the lifetime of the application
    Box::leak(Box::new(tray));

    Ok(rx)
}

#[cfg(not(all(feature = "tray", any(target_os = "windows", target_os = "macos", target_os = "linux"))))]
pub enum TrayMessage {}

#[cfg(not(all(feature = "tray", any(target_os = "windows", target_os = "macos", target_os = "linux"))))]
pub fn init_tray() -> anyhow::Result<std::sync::mpsc::Receiver<TrayMessage>> {
    Err(anyhow::anyhow!("Tray not supported on this platform or tray feature not enabled"))
}
