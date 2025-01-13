use log::info;
use std::io::Read;
use tun::Configuration;

pub struct TunDev {
    dev: tun::Device,
}

impl TunDev {
    #[allow(unused_variables)]
    pub fn new(tun_name: String, ip_addr: String) -> Self {
        info!("TUN initialized for {}", ip_addr);

        let mut config = Configuration::default();
        config
            .address(ip_addr.clone())
            .netmask("255.255.255.0")
            .destination("10.0.0.0")
            .mtu(1500);
        #[cfg(not(target_os = "macos"))]
        config.tun_name(tun_name);
        config.up();
        let tun = tun::create(&config).unwrap();
        tun.set_nonblock().unwrap();
        TunDev { dev: tun }
    }

    pub fn send(self: &mut Self, buf: &[u8]) {
        self.dev.send(buf).unwrap();
    }

    pub fn read(self: &mut Self, buffer: &mut [u8]) -> std::io::Result<usize> {
        return self.dev.read(buffer);
    }
}
