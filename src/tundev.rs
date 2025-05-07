use log::info;
use tokio::io::AsyncReadExt;
use tun::Configuration;

pub struct TunDev {
    pub dev: tun::AsyncDevice,
}

impl TunDev {
    #[allow(unused_variables)]
    pub fn new(tun_name: String, netmask: &str, destination: &str, ip_addr: &str) -> Self {
        info!("TUN initialized for {}", ip_addr);

        let mut config = Configuration::default();
        config
            .address(ip_addr)
            .netmask(netmask)
            .destination(destination)
            .mtu(1500);
        #[cfg(target_os = "linux")]
        config.platform_config(|config| {
            config.ensure_root_privileges(true);
        });

        #[cfg(not(target_os = "macos"))]
        config.tun_name(tun_name);

        config.up();
        let dev = tun::create_as_async(&config).expect("Cannot create TUN device");

        TunDev { dev }
    }

    pub async fn send(self: &mut Self, buf: &[u8], nbytes: usize) -> std::io::Result<usize> {
        return self.dev.send(&buf[..nbytes]).await;
    }

    pub async fn read(self: &mut Self, buffer: &mut [u8]) -> Result<usize, std::io::Error> {
        return self.dev.read(buffer).await;
    }
}
