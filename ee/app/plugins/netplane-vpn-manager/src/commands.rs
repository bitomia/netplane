use tauri::{command, AppHandle, Runtime};

use crate::models::*;
use crate::NetplaneVpnManagerExt;
use crate::Result;

#[command]
pub(crate) async fn ping<R: Runtime>(
    app: AppHandle<R>,
    payload: PingRequest,
) -> Result<PingResponse> {
    app.netplane_vpn_manager().ping(payload)
}

#[command]
pub(crate) async fn request_vpn_permission<R: Runtime>(
    app: AppHandle<R>,
) -> Result<VpnPermissionResponse> {
    app.netplane_vpn_manager().request_vpn_permission()
}

#[command]
pub(crate) async fn start_vpn<R: Runtime>(
    app: AppHandle<R>,
    payload: StartVpnRequest,
) -> Result<StartVpnResponse> {
    app.netplane_vpn_manager().start_vpn(payload)
}

#[command]
pub(crate) async fn stop_vpn<R: Runtime>(app: AppHandle<R>) -> Result<()> {
    app.netplane_vpn_manager().stop_vpn()
}

#[command]
pub(crate) async fn get_tunnel_fd<R: Runtime>(app: AppHandle<R>) -> Result<TunnelFdResponse> {
    app.netplane_vpn_manager().get_tunnel_fd()
}
