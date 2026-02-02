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
use desktop::VpnManager;
#[cfg(mobile)]
use mobile::VpnManager;

/// Extensions to [`tauri::App`], [`tauri::AppHandle`] and [`tauri::Window`] to access the vpn-manager APIs.
pub trait VpnManagerExt<R: Runtime> {
    fn vpn_manager(&self) -> &VpnManager<R>;
}

impl<R: Runtime, T: Manager<R>> crate::VpnManagerExt<R> for T {
    fn vpn_manager(&self) -> &VpnManager<R> {
        self.state::<VpnManager<R>>().inner()
    }
}

/// Initializes the plugin.
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("netplane")
        .invoke_handler(tauri::generate_handler![commands::ping])
        .setup(|app, api| {
            #[cfg(target_os = "android")]
            let handle = api.register_android_plugin("com.bitomia.netplane", "VpnManagerPlugin")?;

            #[cfg(mobile)]
            let vpn_manager = mobile::init(app, api)?;
            #[cfg(desktop)]
            let vpn_manager = desktop::init(app, api)?;
            app.manage(vpn_manager);

            Ok(())
        })
        .build()
}
