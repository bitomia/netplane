use anyhow::{Context, Result};
use net_route::{Handle, Route};
use std::ffi::CString;
use std::net::IpAddr;
use tracing::{info, warn};

async fn resolve_host(host: &str) -> Result<IpAddr> {
    if let Ok(ip) = host.parse::<IpAddr>() {
        return Ok(ip);
    }
    let mut addrs = tokio::net::lookup_host(format!("{}:0", host)).await?;
    addrs
        .next()
        .map(|sa| sa.ip())
        .context("could not resolve relay host")
}

pub struct RouteGuard {
    host_pin: Option<Route>,
}

impl RouteGuard {
    pub async fn restore(self) {
        info!("Restoring routes");
        let Some(pin) = self.host_pin else { return };
        let handle = match Handle::new() {
            Ok(h) => h,
            Err(e) => {
                warn!("route restore: cannot open netlink handle: {}", e);
                return;
            }
        };
        if let Err(e) = handle.delete(&pin).await {
            warn!("route restore: failed to remove host pin: {}", e);
        } else {
            info!("Removed relay host pin");
        }
    }
}

pub async fn install_exit_node_routes(tun_name: &str, relay_host: &str) -> Result<RouteGuard> {
    let handle = Handle::new()?;
    let cname = CString::new(tun_name)?;
    let tun_idx = unsafe { libc::if_nametoindex(cname.as_ptr()) };
    if tun_idx == 0 {
        return Err(anyhow::anyhow!(
            "if_nametoindex({}) failed: {}",
            tun_name,
            std::io::Error::last_os_error()
        ));
    }
    let server_ip = resolve_host(relay_host).await?;
    let existing = handle.list().await?;
    let original_default = existing.into_iter().find(|r| {
        r.prefix == 0
            && match r.destination {
                IpAddr::V4(v4) => v4.is_unspecified(),
                IpAddr::V6(v6) => v6.is_unspecified(),
            }
            && r.gateway.is_some()
    });

    let mut host_pin: Option<Route> = None;
    if let Some(def) = &original_default {
        if let Some(gw) = def.gateway {
            let host_prefix = if server_ip.is_ipv4() { 32 } else { 128 };
            let pin = Route::new(server_ip, host_prefix).with_gateway(gw);
            info!("Pinning relay {}/{} via {}", server_ip, host_prefix, gw);
            match handle.add(&pin).await {
                Ok(_) => host_pin = Some(pin),
                Err(e) => warn!("host pin add returned: {} (may already exist)", e),
            }
        }
    } else {
        warn!("No existing default route found; relay traffic may loop");
    }

    let lower = Route::new("0.0.0.0".parse().unwrap(), 1).with_ifindex(tun_idx);
    let upper = Route::new("128.0.0.0".parse().unwrap(), 1).with_ifindex(tun_idx);
    info!(
        "Adding split-default routes 0.0.0.0/1 and 128.0.0.0/1 via {} (ifindex {})",
        tun_name, tun_idx
    );
    handle.add(&lower).await?;
    handle.add(&upper).await?;

    Ok(RouteGuard { host_pin })
}
