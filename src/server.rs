use crate::common::{handshake_deserialize, HANDSHAKE_HEADER, HANDSHAKE_SIZE};
use crate::packet::parse_ipv4_header;
use log::{debug, error, info};
use std::collections::HashSet;
use std::net::{Ipv4Addr, SocketAddr, UdpSocket};

#[derive(Eq, Hash, PartialEq)]
struct Client {
    pub src: SocketAddr,
    pub ipv4_addr: Ipv4Addr,
}

pub fn run() -> std::io::Result<()> {
    info!("Starting server");
    let socket = UdpSocket::bind("0.0.0.0:5000")?;
    info!("UDP server listening on 0.0.0.0:5000");

    let mut clients: HashSet<Client> = HashSet::new();
    let mut buf = [0; 1500];
    loop {
        let (amt, src) = socket.recv_from(&mut buf)?;
        if amt == HANDSHAKE_SIZE && buf[..3] == HANDSHAKE_HEADER {
            let handshake = handshake_deserialize(&buf);
            info!("Client connected {} {}", src, handshake.ipv4_addr);
            clients.insert(Client {
                src,
                ipv4_addr: handshake.ipv4_addr,
            });
            continue;
        }
        if let Some(header) = parse_ipv4_header(&buf[..amt]) {
            debug!(
                "{} {} {}",
                header.src_ip, header.dst_ip, header.total_length
            );

            for client in &clients {
                debug!("Sending data to {}", src);
                if src != client.src {
                    // TODO this is broadcasting, parse IP header and send only to target
                    debug!("...relying");
                    socket.send_to(&buf[..amt], &client.src)?;
                }
            }
        } else {
            error!("Packet not supported");
        }
    }
}
