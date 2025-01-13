use libc::{setsockopt, IPPROTO_IP, IP_HDRINCL};
use log::info;
use nix::sys::socket::{socket, AddressFamily, SockFlag, SockProtocol, SockType};
use std::io::Read;
use std::mem;
use std::os::fd::AsRawFd;
use tun::Configuration;

pub struct TunDev {
    dev: tun::Device,
}

impl TunDev {
    #[allow(unused_variables)]
    pub fn new(tun_name: String, ip_addr: String) -> Self {
        info!("TUN initialized for {}", ip_addr);

        let raw_socket = socket(
            AddressFamily::Inet,
            SockType::Raw,
            SockFlag::empty(),
            SockProtocol::Raw,
        )
        .expect("Failed to create raw socket");
        let option_value: libc::c_int = 1; // Enable IP_HDRINCL (1 = true)
        let ret = unsafe {
            setsockopt(
                raw_socket.as_raw_fd(), // Socket descriptor
                IPPROTO_IP,             // Protocol level
                IP_HDRINCL,             // Option name
                &option_value as *const _ as *const libc::c_void,
                mem::size_of_val(&option_value) as libc::socklen_t,
            )
        };
        if ret == -1 {
            std::process::exit(1)
        }

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
