use crate::common::{handshake_deserialize, HANDSHAKE_HEADER, HANDSHAKE_SIZE};
use crate::packet::parse_ipv4_header;
use axum::{response::Html, routing::get, Router};
use log::{debug, error, info};
use std::collections::HashSet;
use std::net::{Ipv4Addr, SocketAddr, UdpSocket};

#[derive(Eq, Hash, PartialEq)]
struct Client {
    pub src: SocketAddr,
    pub ipv4_addr: Ipv4Addr,
}

async fn index() -> Html<&'static str> {
    Html(std::include_str!("../index.html"))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessError(u32);

async fn reticula_server() {
    let socket = UdpSocket::bind("0.0.0.0:5000").expect("Cannot open socket");
    let mut clients: HashSet<Client> = HashSet::new();
    let mut buf = [0; 1500];
    loop {
        let (amt, src) = socket.recv_from(&mut buf).expect("Cannot receive");
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
                    socket
                        .send_to(&buf[..amt], &client.src)
                        .expect("Cannot send");
                }
            }
        } else {
            error!("Packet not supported");
        }
    }
}

pub async fn run() -> Result<(), ProcessError> {
    info!("Starting web server");
    let app = Router::new().route("/", get(index));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    let web_server = axum::serve(listener, app);

    info!("Starting reticula server");
    let reticula_server = tokio::spawn(reticula_server());
    info!("UDP server listening on 0.0.0.0:5000");

    tokio::select! {
        _ = web_server => info!("Web server stopped"),
        _ = reticula_server => info!("Reticula server stopped")
    }
    Ok(())
}
