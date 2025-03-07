use crate::common::{handshake_serialize, Handshake, HANDSHAKE_HEADER};
use crate::packet::parse_ipv4_header;
use crate::tundev;

use log::{debug, error, info};
use std::net::Ipv4Addr;
use std::str::FromStr;
use tokio::net::UdpSocket;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessError(u32);

pub async fn send_tun(dev: &mut crate::tundev::TunDev, buf: &[u8], nbytes: usize) {
    match dev.send(&buf[..nbytes], nbytes).await {
        Ok(_) => {}
        Err(err) => error!("{}", err),
    }
}

//pub async fn run(
//    tun_name: String,
//    destination: String,
//    netmask: String,
//    ip_addr: String,
//    server_addr: String,
//) -> Result<(), ProcessError> {
//    info!("Starting client");
//    let socket = UdpSocket::bind("0.0.0.0:0")
//        .await
//        .expect("Cannot open socket");
//
//    info!(
//        "Client bound to {:?}",
//        socket.local_addr().expect("Cannot get the local addr")
//    );
//
//    let mut dev = tundev::TunDev::new(tun_name, netmask, destination, ip_addr.clone());
//
//    let ipv4_addr = match Ipv4Addr::from_str(ip_addr.as_str()) {
//        Ok(addr) => addr,
//        Err(_) => return Err(ProcessError(1)),
//    };
//    let handshake = Handshake {
//        header: HANDSHAKE_HEADER,
//        ipv4_addr,
//    };
//    info!("Sending handshake {}", server_addr.clone());
//    socket
//        .connect(server_addr.clone())
//        .await
//        .expect("Cannot connect");
//    socket
//        .send(&handshake_serialize(&handshake))
//        .await
//        .expect("Cannot send handshake");
//
//    let mut socket_buf = [0; 1500];
//    let mut tun_buf = [0; 1500];
//    loop {
//        tokio::select! {
//            result = socket.recv_from(&mut socket_buf) => {
//                match result {
//                    Ok((amt, from)) => {
//                        debug!("=> Server sent {} from {}", amt, from);
//                        if let Some(header) = parse_ipv4_header(&socket_buf[..amt]) {
//                            debug!(
//                                "{} {} {}",
//                                header.src_ip, header.dst_ip, header.total_length
//                            );
//                        }
//                        send_tun(&mut dev, &socket_buf, amt).await;
//                    }
//                    Err(_) => todo!()
//                }
//            }
//            tun_ret = dev.read(&mut tun_buf) => {
//                match tun_ret {
//                    Ok(amt) => {
//                        debug!("<= Tun read {}", amt);
//
//                        let mut is_loopback = false;
//                        if let Some(header) = parse_ipv4_header(&tun_buf[..amt]) {
//                            debug!("{} {}", header.src_ip, header.dst_ip);
//                            is_loopback =
//                                header.src_ip == header.dst_ip && header.src_port == header.dst_port;
//                        }
//                        if is_loopback {
//                            send_tun(&mut dev, &tun_buf, amt).await;
//                        } else {
//                            match socket.send(&tun_buf[..amt]).await {
//                                Ok(bytes_sent) => {
//                                    if bytes_sent != amt {
//                                        error!("Less bytes sent than expected to socket");
//                                    }
//                                },
//                                Err(err) => error!("{}´", err)
//                            }
//                        }
//                    }
//                    Err(err) => {
//                        info!("{}", err);
//                    }
//                }
//            }
//        }
//    }
//}
