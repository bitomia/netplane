use std::sync::Arc;
use tokio::sync::Mutex;
use log::info;
use tokio::io::AsyncReadExt;
use tun::Configuration;

pub struct TunDev {
    pub dev: Arc<Mutex<tun::AsyncDevice>>,
}

impl TunDev {
    #[allow(unused_variables)]
    pub fn new(tun_name: &str, netmask: &str, destination: &str, ip_addr: &str) -> Self {
        info!("TUN initialized for {}", ip_addr);

        let mut config = Configuration::default();
        config
            .address(ip_addr.clone())
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
        
        TunDev {
            dev: Arc::new(Mutex::new(dev)),
        }
    }

    pub async fn send(&self, buf: &[u8], nbytes: usize) -> std::io::Result<usize> {
        let dev = self.dev.lock().await;
        dev.send(&buf[..nbytes]).await
    }

    pub async fn read(&self, buffer: &mut [u8]) -> Result<usize, std::io::Error> {
        let mut dev = self.dev.lock().await;
        dev.read(buffer).await
    }
}

impl Clone for TunDev {
    fn clone(&self) -> Self {
        TunDev {
            dev: Arc::clone(&self.dev),
        }
    }
}

