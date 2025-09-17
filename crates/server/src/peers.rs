use bytes::Bytes;
use netplane_common::PeerState;
use std::any::Any;
use std::collections::HashMap;
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tokio::sync::mpsc;

pub type Tx = mpsc::UnboundedSender<Bytes>;
pub type Rx = mpsc::UnboundedReceiver<Bytes>;

pub trait Peer: Send + Any {
    fn get_sdn_addr(&self) -> Ipv4Addr;
    fn set_sdn_addr(&mut self, addr: &Ipv4Addr);
    fn get_state(&self) -> PeerState;
    fn set_state(&mut self, state: PeerState);
    fn update_last_heartbeat(&mut self);
    fn is_heartbeat_expired(&self, timeout: Duration) -> bool;
}

impl dyn Peer {
    pub fn as_any(&self) -> &dyn Any {
        self
    }
}

#[derive(Clone)]
pub struct PeerData {
    pub sdn_addr: Ipv4Addr,
    pub state: PeerState,
    pub last_heartbeat: Instant,
}

#[derive(Clone)]
pub struct UdpPeer {
    data: PeerData,
}

impl UdpPeer {
    pub fn new(data: PeerData) -> Box<dyn Peer> {
        Box::new(UdpPeer { data })
    }
}

impl Peer for UdpPeer {
    fn get_sdn_addr(&self) -> Ipv4Addr {
        self.data.sdn_addr
    }

    fn set_sdn_addr(&mut self, addr: &Ipv4Addr) {
        self.data.sdn_addr = addr.clone();
    }

    fn get_state(&self) -> PeerState {
        self.data.state.clone()
    }

    fn set_state(&mut self, state: PeerState) {
        self.data.state = state;
    }

    fn update_last_heartbeat(&mut self) {
        self.data.last_heartbeat = Instant::now();
    }

    fn is_heartbeat_expired(&self, timeout: Duration) -> bool {
        self.data.last_heartbeat.elapsed() > timeout
    }
}

pub struct TcpPeer {
    data: PeerData,
    pub tx: Tx,
}

impl TcpPeer {
    pub fn new(data: PeerData, tx: Tx) -> Box<dyn Peer> {
        Box::new(TcpPeer { data, tx })
    }
}

impl Peer for TcpPeer {
    fn get_sdn_addr(&self) -> Ipv4Addr {
        self.data.sdn_addr
    }

    fn set_sdn_addr(&mut self, addr: &Ipv4Addr) {
        self.data.sdn_addr = addr.clone();
    }

    fn get_state(&self) -> PeerState {
        self.data.state.clone()
    }

    fn set_state(&mut self, state: PeerState) {
        self.data.state = state;
    }

    fn update_last_heartbeat(&mut self) {
        self.data.last_heartbeat = Instant::now();
    }

    fn is_heartbeat_expired(&self, timeout: Duration) -> bool {
        self.data.last_heartbeat.elapsed() > timeout
    }
}

pub type Peers<Key> = Arc<Mutex<HashMap<Key, Box<dyn Peer>>>>;

pub trait PeersVec<Key, Value> {
    async fn to_vec(&self) -> Vec<(Key, Value)>;
}

impl PeersVec<SocketAddr, Ipv4Addr> for Peers<SocketAddr> {
    async fn to_vec(&self) -> Vec<(SocketAddr, Ipv4Addr)> {
        let peers = self.lock().await;
        peers
            .iter()
            .map(|(addr, peer)| (*addr, peer.get_sdn_addr()))
            .collect()
    }
}

impl PeersVec<Tx, Ipv4Addr> for Peers<i32> {
    async fn to_vec(&self) -> Vec<(Tx, Ipv4Addr)> {
        let peers = self.lock().await;
        peers
            .values()
            .filter_map(|peer| {
                if let Some(tcp_peer) = peer.as_any().downcast_ref::<TcpPeer>() {
                    Some((tcp_peer.tx.clone(), tcp_peer.get_sdn_addr()))
                } else {
                    None
                }
            })
            .collect()
    }
}
