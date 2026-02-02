use serde::de::DeserializeOwned;
use tauri::{plugin::PluginApi, AppHandle, Runtime};

use crate::models::*;

pub fn init<R: Runtime, C: DeserializeOwned>(
  app: &AppHandle<R>,
  _api: PluginApi<R, C>,
) -> crate::Result<VpnManager<R>> {
  Ok(VpnManager(app.clone()))
}

/// Access to the vpn-manager APIs.
pub struct VpnManager<R: Runtime>(AppHandle<R>);

impl<R: Runtime> VpnManager<R> {
  pub fn ping(&self, payload: PingRequest) -> crate::Result<PingResponse> {
    Ok(PingResponse {
      value: payload.value,
    })
  }
}
