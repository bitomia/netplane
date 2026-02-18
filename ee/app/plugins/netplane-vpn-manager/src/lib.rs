use tauri::{
    plugin::{Builder, TauriPlugin},
    Manager, Runtime,
};

pub use models::*;

#[cfg(desktop)]
mod desktop;
#[cfg(mobile)]
mod mobile;

mod commands;
mod error;
mod models;

pub use error::{Error, Result};

#[cfg(desktop)]
use desktop::NetplaneVpnManager;
#[cfg(mobile)]
use mobile::NetplaneVpnManager;

/// Extensions to [`tauri::App`], [`tauri::AppHandle`] and [`tauri::Window`] to access the netplane-vpn-manager APIs.
pub trait NetplaneVpnManagerExt<R: Runtime> {
    fn netplane_vpn_manager(&self) -> &NetplaneVpnManager<R>;
}

impl<R: Runtime, T: Manager<R>> crate::NetplaneVpnManagerExt<R> for T {
    fn netplane_vpn_manager(&self) -> &NetplaneVpnManager<R> {
        self.state::<NetplaneVpnManager<R>>().inner()
    }
}

/// Initializes the plugin.
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("netplane-vpn-manager")
        .invoke_handler(tauri::generate_handler![
            commands::ping,
            commands::request_vpn_permission,
            commands::start_vpn,
            commands::stop_vpn,
            commands::get_tunnel_fd,
        ])
        .setup(|app, api| {
            #[cfg(mobile)]
            let netplane_vpn_manager = mobile::init(app, api)?;
            #[cfg(desktop)]
            let netplane_vpn_manager = desktop::init(app, api)?;
            app.manage(netplane_vpn_manager);
            Ok(())
        })
        .build()
}
