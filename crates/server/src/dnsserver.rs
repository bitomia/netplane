use anyhow::Result;
use log::{error, info};
use std::net::{Ipv4Addr, UdpSocket};
use std::str;
use std::sync::Arc;
use std::thread;

fn u16_from_be(bytes: &[u8]) -> u16 {
    ((bytes[0] as u16) << 8) | (bytes[1] as u16)
}

fn parse_qname(packet: &[u8], mut pos: usize) -> Option<(String, usize)> {
    // Parse labels until zero length; returns dotted name and new position after the terminating 0
    let mut labels = Vec::new();
    loop {
        if pos >= packet.len() {
            return None;
        }
        let len = packet[pos] as usize;
        if len == 0 {
            pos += 1; // skip the zero
            break;
        }
        pos += 1;
        if pos + len > packet.len() {
            return None;
        }
        match str::from_utf8(&packet[pos..pos + len]) {
            Ok(s) => labels.push(s.to_string()),
            Err(_) => return None,
        }
        pos += len;
    }
    Some((labels.join("."), pos))
}

fn build_response(query: &[u8], answer_ip: Option<String>) -> Option<Vec<u8>> {
    if query.len() < 12 {
        return None;
    }
    let id = &query[0..2];
    let qdcount = u16_from_be(&query[4..6]);
    if qdcount == 0 {
        return None;
    }
    let (_qname, pos_after_qname) = parse_qname(query, 12)?;
    if pos_after_qname + 4 > query.len() {
        return None;
    }
    let qtype = u16_from_be(&query[pos_after_qname..pos_after_qname + 2]);
    let qclass = u16_from_be(&query[pos_after_qname + 2..pos_after_qname + 4]);

    let valid_query = (qtype == 1) && (qclass == 1);

    let mut resp = Vec::new();
    resp.extend_from_slice(id);

    if valid_query && answer_ip.is_some() {
        resp.extend_from_slice(&[0x81, 0x80]); // Standard response, no error
    } else {
        resp.extend_from_slice(&[0x81, 0x83]); // Standard response, NXDOMAIN (RCODE=3)
    }

    resp.extend_from_slice(&query[4..6]); // QDCOUNT

    if valid_query && answer_ip.is_some() {
        resp.extend_from_slice(&[0x00, 0x01]);
    } else {
        resp.extend_from_slice(&[0x00, 0x00]);
    }

    resp.extend_from_slice(&[0x00, 0x00]); // NSCOUNT
    resp.extend_from_slice(&[0x00, 0x00]); // ARCOUNT

    resp.extend_from_slice(&query[12..pos_after_qname + 4]);

    if valid_query && answer_ip.is_some() {
        resp.extend_from_slice(&[0xC0, 0x0C]); // Name pointer
        resp.extend_from_slice(&[0x00, 0x01]); // TYPE A
        resp.extend_from_slice(&[0x00, 0x01]); // CLASS IN
        resp.extend_from_slice(&[0x00, 0x00, 0x00, 0x3C]); // TTL
        resp.extend_from_slice(&[0x00, 0x04]); // RDLENGTH

        let ipv4: Ipv4Addr = answer_ip.unwrap().parse().ok()?;
        resp.extend_from_slice(&ipv4.octets());
    }

    Some(resp)
}

pub struct DnsServer {
    db: Arc<crate::db::Db>,
}

impl DnsServer {
    pub fn new(db: Arc<crate::db::Db>) -> DnsServer {
        DnsServer { db }
    }

    pub async fn start(&mut self, bind_addr: String) -> Result<()> {
        info!("Starting DNS server {}", bind_addr);

        let socket = UdpSocket::bind(bind_addr)?;
        socket.set_nonblocking(false)?;

        loop {
            let mut buf = [0u8; 512];
            match socket.recv_from(&mut buf) {
                Ok((len, src)) => {
                    let packet = buf[..len].to_vec();
                    let sock = socket.try_clone()?;
                    let qname = parse_qname(&packet, 12);

                    match qname {
                        Some((qname, _)) => {
                            let answer_ip = self.db.get_hostname(&qname).await;
                            thread::spawn(move || match answer_ip {
                                Ok(answer_ip) => match build_response(&packet, answer_ip.clone()) {
                                    Some(resp) => {
                                        if let Err(e) = sock.send_to(&resp, src) {
                                            eprintln!("Failed to send response to {}: {}", src, e);
                                        }
                                    }
                                    None => {
                                        eprintln!(
                                            "Failed to build response for query from {}",
                                            src
                                        );
                                    }
                                },
                                Err(e) => error!("Error querying DNS entry from database: {}", e),
                            });
                        }
                        None => {
                            eprintln!("Failed to build response for query from {}", src);
                        }
                    }
                }
                Err(e) => eprintln!("recv_from failed: {}", e),
            }
        }
    }
}
