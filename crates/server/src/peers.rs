use bytes::Bytes;
use netplane_common::HandshakeStatus;
use std::collections::HashMap;
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

pub type Tx = mpsc::UnboundedSender<Bytes>;
pub type Rx = mpsc::UnboundedReceiver<Bytes>;

pub struct TcpPeer {
    pub sdn_ip_addr: Ipv4Addr,
    pub status: HandshakeStatus,
    pub tx: Tx,
}

#[derive(Clone)]
pub struct UdpPeer {
    pub sdn_ip_addr: Ipv4Addr,
    pub status: HandshakeStatus,
}

pub enum Peers {
    UdpPeers(HashMap<SocketAddr, UdpPeer>),
    TcpPeers(Arc<Mutex<HashMap<i32, TcpPeer>>>),
}

pub fn try_get_tcp(peers: &mut Peers) -> Option<&mut Arc<Mutex<HashMap<i32, TcpPeer>>>> {
    if let Peers::TcpPeers(peers) = peers {
        Some(peers)
    } else {
        None
    }
}

pub fn try_get_udp(peers: &mut Peers) -> Option<&mut HashMap<SocketAddr, UdpPeer>> {
    if let Peers::UdpPeers(peers) = peers {
        Some(peers)
    } else {
        None
    }
}
