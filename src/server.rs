use crate::common::{handshake_deserialize, HANDSHAKE_HEADER, HANDSHAKE_SIZE};
use crate::db;
use crate::packet::parse_ipv4_header;
use crate::webserver::WebServer;
use log::{debug, error, info};
use std::collections::HashSet;
use std::net::{Ipv4Addr, SocketAddr, UdpSocket};
use std::sync::Arc;

#[derive(Eq, Hash, PartialEq)]
struct Client {
    pub src: SocketAddr,
    pub ipv4_addr: Ipv4Addr,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessError(u32);

async fn reticula_server(db: Arc<db::Db>) {
    let socket = UdpSocket::bind("0.0.0.0:5000").expect("Cannot open socket");
    let mut clients: HashSet<Client> = HashSet::new();
    let mut buf = [0; 1500];
    loop {
        let (amt, src) = socket.recv_from(&mut buf).expect("Cannot receive");
        if amt == HANDSHAKE_SIZE && buf[..3] == HANDSHAKE_HEADER {
            let handshake = handshake_deserialize(&buf);
            if db
                .check_client(&src.ip().to_string(), &handshake.ipv4_addr.to_string())
                .await
                == true
            {
                clients.insert(Client {
                    src,
                    ipv4_addr: handshake.ipv4_addr,
                });
                info!("Client connected {} {}", src, handshake.ipv4_addr);
            } else {
                error!(
                    "Ignoring Unknown user pair {} {}",
                    src.ip(),
                    handshake.ipv4_addr
                );
            }
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
    let db = Arc::new(db::Db::new().await);
    let web_server = WebServer::new("0.0.0.0:3000", db.clone()).await;

    info!("Starting reticula server");
    let reticula_server = tokio::spawn(reticula_server(db.clone()));
    info!("UDP server listening on 0.0.0.0:5000");

    tokio::select! {
        _ = web_server => info!("Web server stopped"),
        _ = reticula_server => info!("Reticula server stopped")
    }
    Ok(())
}
