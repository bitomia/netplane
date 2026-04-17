use anyhow::{anyhow, Result};
use log::{info, warn};
use std::net::Ipv4Addr;

pub struct ConsumerRoutes {
    tun_dev: String,
}

pub struct ExitNodeState {
    sdn_cidr: String,
    tun_dev: String,
    previous_ip_forward: Option<String>,
}

#[cfg(target_os = "linux")]
fn run(cmd: &str, args: &[&str]) -> Result<String> {
    let output = std::process::Command::new(cmd).args(args).output()?;
    if !output.status.success() {
        return Err(anyhow!(
            "{} {} failed: {}",
            cmd,
            args.join(" "),
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

#[cfg(target_os = "macos")]
fn run(cmd: &str, args: &[&str]) -> Result<String> {
    let output = std::process::Command::new(cmd).args(args).output()?;
    if !output.status.success() {
        return Err(anyhow!(
            "{} {} failed: {}",
            cmd,
            args.join(" "),
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Install the two 0.0.0.0/1 and 128.0.0.0/1 "split default" routes
/// so all internet-bound traffic is funneled into the TUN device.
#[cfg(target_os = "linux")]
pub fn enable_consumer(tun_dev: &str) -> Result<ConsumerRoutes> {
    run("ip", &["route", "add", "0.0.0.0/1", "dev", tun_dev])?;
    run("ip", &["route", "add", "128.0.0.0/1", "dev", tun_dev])?;
    info!("Installed exit-node consumer routes via {}", tun_dev);
    Ok(ConsumerRoutes {
        tun_dev: tun_dev.to_string(),
    })
}

#[cfg(target_os = "macos")]
pub fn enable_consumer(tun_dev: &str) -> Result<ConsumerRoutes> {
    run("route", &["-n", "add", "-net", "0.0.0.0/1", "-interface", tun_dev])?;
    run(
        "route",
        &["-n", "add", "-net", "128.0.0.0/1", "-interface", tun_dev],
    )?;
    info!("Installed exit-node consumer routes via {}", tun_dev);
    Ok(ConsumerRoutes {
        tun_dev: tun_dev.to_string(),
    })
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
pub fn enable_consumer(tun_dev: &str) -> Result<ConsumerRoutes> {
    info!(
        "Exit-node consumer mode on this platform relies on TUN default route ({})",
        tun_dev
    );
    Ok(ConsumerRoutes {
        tun_dev: tun_dev.to_string(),
    })
}

#[cfg(target_os = "linux")]
pub fn disable_consumer(state: &ConsumerRoutes) {
    let _ = run("ip", &["route", "del", "0.0.0.0/1", "dev", &state.tun_dev]);
    let _ = run(
        "ip",
        &["route", "del", "128.0.0.0/1", "dev", &state.tun_dev],
    );
    info!("Removed exit-node consumer routes");
}

#[cfg(target_os = "macos")]
pub fn disable_consumer(state: &ConsumerRoutes) {
    let _ = run(
        "route",
        &["-n", "delete", "-net", "0.0.0.0/1", "-interface", &state.tun_dev],
    );
    let _ = run(
        "route",
        &["-n", "delete", "-net", "128.0.0.0/1", "-interface", &state.tun_dev],
    );
    info!("Removed exit-node consumer routes");
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
pub fn disable_consumer(_state: &ConsumerRoutes) {}

/// Configure this host as an exit node: enable ip_forward and install
/// an nftables MASQUERADE rule for the SDN subnet.
#[cfg(target_os = "linux")]
pub fn enable_exit_node(sdn_cidr: &str, tun_dev: &str) -> Result<ExitNodeState> {
    let prev = run("sysctl", &["-n", "net.ipv4.ip_forward"]).ok();
    run("sysctl", &["-w", "net.ipv4.ip_forward=1"])?;

    let _ = run("nft", &["delete", "table", "inet", "netplane"]);
    run("nft", &["add", "table", "inet", "netplane"])?;
    run(
        "nft",
        &[
            "add",
            "chain",
            "inet",
            "netplane",
            "postrouting",
            "{ type nat hook postrouting priority 100 ; }",
        ],
    )?;
    let rule = format!(
        "ip saddr {} oifname != \"{}\" masquerade",
        sdn_cidr, tun_dev
    );
    run("nft", &["add", "rule", "inet", "netplane", "postrouting", &rule])?;

    info!(
        "Exit-node mode enabled: MASQUERADE for {} via non-{} iface",
        sdn_cidr, tun_dev
    );
    Ok(ExitNodeState {
        sdn_cidr: sdn_cidr.to_string(),
        tun_dev: tun_dev.to_string(),
        previous_ip_forward: prev,
    })
}

#[cfg(not(target_os = "linux"))]
pub fn enable_exit_node(_sdn_cidr: &str, _tun_dev: &str) -> Result<ExitNodeState> {
    Err(anyhow!(
        "Exit-node server mode is only supported on Linux in this release"
    ))
}

#[cfg(target_os = "linux")]
pub fn disable_exit_node(state: &ExitNodeState) {
    let _ = run("nft", &["delete", "table", "inet", "netplane"]);
    if let Some(prev) = &state.previous_ip_forward {
        let _ = run("sysctl", &["-w", &format!("net.ipv4.ip_forward={}", prev)]);
    }
    info!(
        "Exit-node mode disabled (sdn_cidr={}, tun={})",
        state.sdn_cidr, state.tun_dev
    );
}

#[cfg(not(target_os = "linux"))]
pub fn disable_exit_node(_state: &ExitNodeState) {}

/// CIDR string for the SDN subnet built from a network address and a dotted-quad netmask.
pub fn sdn_cidr(network: &str, netmask: &str) -> Result<String> {
    let mask: Ipv4Addr = netmask
        .parse()
        .map_err(|_| anyhow!("invalid netmask: {}", netmask))?;
    let bits = u32::from(mask).count_ones();
    if u32::from(mask).leading_ones() != bits {
        warn!("Non-contiguous netmask {} — using bit count {}", netmask, bits);
    }
    Ok(format!("{}/{}", network, bits))
}
