use serde::de::DeserializeOwned;
use tauri::{
    plugin::{PluginApi, PluginHandle},
    AppHandle, Runtime,
};

use crate::models::*;

#[cfg(target_os = "ios")]
tauri::ios_plugin_binding!(init_plugin_netplane_vpn_manager);

// initializes the Kotlin or Swift plugin classes
pub fn init<R: Runtime, C: DeserializeOwned>(
    _app: &AppHandle<R>,
    api: PluginApi<R, C>,
) -> crate::Result<NetplaneVpnManager<R>> {
    #[cfg(target_os = "android")]
    let handle = api.register_android_plugin("com.bitomia.netplane.vpn", "NetplanePlugin")?;
    #[cfg(target_os = "ios")]
    let handle = api.register_ios_plugin(init_plugin_netplane_vpn_manager)?;
    Ok(NetplaneVpnManager(handle))
}

/// Access to the netplane-vpn-manager APIs.
pub struct NetplaneVpnManager<R: Runtime>(PluginHandle<R>);

impl<R: Runtime> NetplaneVpnManager<R> {
    pub fn ping(&self, payload: PingRequest) -> crate::Result<PingResponse> {
        self.0
            .run_mobile_plugin("ping", payload)
            .map_err(Into::into)
    }

    pub fn request_vpn_permission(&self) -> crate::Result<VpnPermissionResponse> {
        self.0
            .run_mobile_plugin("requestVpnPermission", ())
            .map_err(Into::into)
    }

    pub fn start_vpn(&self, payload: StartVpnRequest) -> crate::Result<StartVpnResponse> {
        self.0
            .run_mobile_plugin("startVpn", payload)
            .map_err(Into::into)
    }

    pub fn stop_vpn(&self) -> crate::Result<()> {
        self.0.run_mobile_plugin("stopVpn", ()).map_err(Into::into)
    }

    pub fn get_tunnel_fd(&self) -> crate::Result<TunnelFdResponse> {
        self.0
            .run_mobile_plugin("getTunnelFd", ())
            .map_err(Into::into)
    }
}
