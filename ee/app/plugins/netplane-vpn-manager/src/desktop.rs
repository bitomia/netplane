use serde::de::DeserializeOwned;
use tauri::{plugin::PluginApi, AppHandle, Runtime};

use crate::models::*;

pub fn init<R: Runtime, C: DeserializeOwned>(
    app: &AppHandle<R>,
    _api: PluginApi<R, C>,
) -> crate::Result<NetplaneVpnManager<R>> {
    Ok(NetplaneVpnManager(app.clone()))
}

/// Access to the netplane-vpn-manager APIs.
pub struct NetplaneVpnManager<R: Runtime>(AppHandle<R>);

impl<R: Runtime> NetplaneVpnManager<R> {
    pub fn ping(&self, payload: PingRequest) -> crate::Result<PingResponse> {
        Ok(PingResponse {
            value: payload.value,
        })
    }

    pub fn request_vpn_permission(&self) -> crate::Result<VpnPermissionResponse> {
        Ok(VpnPermissionResponse { granted: true })
    }

    pub fn start_vpn(&self, _payload: StartVpnRequest) -> crate::Result<StartVpnResponse> {
        Ok(StartVpnResponse { fd: -1 })
    }

    pub fn stop_vpn(&self) -> crate::Result<()> {
        Ok(())
    }

    pub fn get_tunnel_fd(&self) -> crate::Result<TunnelFdResponse> {
        Ok(TunnelFdResponse { fd: -1 })
    }
}
