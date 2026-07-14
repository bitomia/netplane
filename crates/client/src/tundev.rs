use tokio::io::AsyncReadExt;
use tracing::info;
use tun::Configuration;

use super::fd::PlatformFd;

pub struct TunDev {
    pub dev: tun::AsyncDevice,
}

impl TunDev {
    #[allow(unused_variables)]
    pub fn new(
        tun_dev: String,
        netmask: &str,
        destination: &str,
        ip_addr: &str,
    ) -> anyhow::Result<Self> {
        info!("TUN initialized for {}", ip_addr);

        let mut config = Configuration::default();
        config
            .address(ip_addr)
            .netmask(netmask)
            .destination(destination)
            .mtu(1400);

        #[cfg(target_os = "linux")]
        config.platform_config(|config| {
            config.ensure_root_privileges(true);
        });

        #[cfg(not(target_os = "macos"))]
        config.tun_name(tun_dev);

        config.up();

        let dev = tun::create_as_async(&config)
            .map_err(|e| anyhow::anyhow!("Cannot create TUN device: {}", e))?;
        Ok(TunDev { dev })
    }

    pub fn new_from_fd(
        fd: PlatformFd,
        netmask: &str,
        destination: &str,
        ip_addr: &str,
    ) -> anyhow::Result<Self> {
        #[cfg(unix)]
        let raw_fd = fd.as_raw_fd();
        #[cfg(windows)]
        let raw_fd = fd.as_raw_handle();

        info!("TUN initialized from FD {:?} for {}", fd, ip_addr);

        let mut config = Configuration::default();
        #[cfg(unix)]
        config
            .address(ip_addr)
            .netmask(netmask)
            .destination(destination)
            .raw_fd(raw_fd)
            // The fd is owned by the host (macOS NetworkExtension created the utun);
            // don't let the tun device close it on drop, or a stop/reconnect can
            // double-close and hit an unrelated, reused descriptor.
            .close_fd_on_drop(false)
            .mtu(1400);
        #[cfg(windows)]
        config
            .address(ip_addr)
            .netmask(netmask)
            .destination(destination)
            .raw_handle(raw_fd)
            .mtu(1400);
        let dev = tun::create_as_async(&config)
            .map_err(|e| anyhow::anyhow!("Cannot create TUN device from FD: {}", e))?;

        Ok(TunDev { dev })
    }

    pub async fn send(&mut self, buf: &[u8], nbytes: usize) -> std::io::Result<usize> {
        return self.dev.send(&buf[..nbytes]).await;
    }

    pub async fn read(&mut self, buffer: &mut [u8]) -> Result<usize, std::io::Error> {
        self.dev.read(buffer).await
    }
}
