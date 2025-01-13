use log::info;
use std::collections::HashSet;
use std::net::{SocketAddr, UdpSocket};

use crate::common::HANDSHAKE;

pub fn run() -> std::io::Result<()> {
    info!("Starting server");
    let socket = UdpSocket::bind("0.0.0.0:5000")?;
    info!("UDP server listening on 0.0.0.0:5000");

    let mut clients: HashSet<SocketAddr> = HashSet::new();
    let mut buf = [0; 1500];
    loop {
        let (amt, src) = socket.recv_from(&mut buf)?;
        if amt == HANDSHAKE.len() && buf[..3] == HANDSHAKE {
            info!("Handshake from {}", src);
            clients.insert(src);
            continue;
        }
        for client in &clients {
            if src != *client {
                socket.send_to(&buf, &client)?;
            }
        }
    }
}
