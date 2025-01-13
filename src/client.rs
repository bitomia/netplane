use log::{debug, error, info};
use std::io;
use std::net::UdpSocket;

use crate::common::HANDSHAKE;
use crate::tundev;

pub fn run(tun_name: String, ip_addr: String, server_addr: String) -> std::io::Result<()> {
    info!("Starting client");
    let socket = UdpSocket::bind("0.0.0.0:0")?;
    socket.set_nonblocking(true)?;

    info!("Client bound to {}", socket.local_addr()?);

    let mut dev = tundev::TunDev::new(tun_name, ip_addr);

    socket.send_to(&HANDSHAKE, server_addr.clone())?;

    loop {
        let mut buf = [0; 1500];
        loop {
            match socket.recv_from(&mut buf) {
                Ok((amt, from)) => {
                    debug!("=> Tun send {} from {}", amt, from);
                    dev.send(&buf[..amt]);
                }
                Err(ref err) if err.kind() != io::ErrorKind::WouldBlock => {
                    error!("Something went wrong: {}", err)
                }
                _ => {}
            }
            match dev.read(&mut buf) {
                Ok(amt) => {
                    debug!("<= Tun read {}", amt);
                    socket.send_to(&buf[..amt], server_addr.clone())?;
                }
                Err(ref err) if err.kind() != io::ErrorKind::WouldBlock => {
                    error!("Something went wrong: {}", err)
                }
                _ => {}
            }
        }
    }
}
