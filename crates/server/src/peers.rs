use bytes::Bytes;
use netplane_common::{PeerInfo, PeerState};
use std::any::Any;
use std::collections::HashMap;
use std::hash::Hash;
use std::net::{Ipv4Addr, SocketAddr};
use std::str::FromStr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tokio::sync::Mutex;

pub type Tx = mpsc::UnboundedSender<Bytes>;
pub type Rx = mpsc::UnboundedReceiver<Bytes>;

pub trait Peer: Send + Any {
    fn get_sdn_addr(&self) -> Ipv4Addr;
    fn set_sdn_addr(&mut self, addr: &Ipv4Addr);
    fn get_state(&self) -> PeerState;
    fn set_state(&mut self, state: PeerState);
    fn get_client_public_key(&self) -> Option<String>;
    fn set_client_public_key(&mut self, key: Option<String>);
    fn get_is_exit_node(&self) -> bool;
    fn set_is_exit_node(&mut self, is_exit_node: bool);
}

impl dyn Peer {
    pub fn as_any(&self) -> &dyn Any {
        self
    }

    pub fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[derive(Clone)]
pub struct PeerData {
    pub sdn_addr: Ipv4Addr,
    pub state: PeerState,
    pub client_public_key: Option<String>,
    pub is_exit_node: bool,
}

#[derive(Clone)]
pub struct UdpPeer {
    data: PeerData,
    last_heartbeat: Instant,
}

impl UdpPeer {
    pub fn new(data: PeerData) -> Box<dyn Peer> {
        Box::new(UdpPeer {
            data,
            last_heartbeat: Instant::now(),
        })
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

    fn get_client_public_key(&self) -> Option<String> {
        self.data.client_public_key.clone()
    }

    fn set_client_public_key(&mut self, key: Option<String>) {
        self.data.client_public_key = key;
    }

    fn get_is_exit_node(&self) -> bool {
        self.data.is_exit_node
    }

    fn set_is_exit_node(&mut self, is_exit_node: bool) {
        self.data.is_exit_node = is_exit_node;
    }
}

impl UdpPeer {
    pub fn update_last_heartbeat(&mut self) {
        self.last_heartbeat = Instant::now();
    }

    pub fn is_heartbeat_expired(&self, timeout: Duration) -> bool {
        self.last_heartbeat.elapsed() > timeout
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

    fn get_client_public_key(&self) -> Option<String> {
        self.data.client_public_key.clone()
    }

    fn set_client_public_key(&mut self, key: Option<String>) {
        self.data.client_public_key = key;
    }

    fn get_is_exit_node(&self) -> bool {
        self.data.is_exit_node
    }

    fn set_is_exit_node(&mut self, is_exit_node: bool) {
        self.data.is_exit_node = is_exit_node;
    }
}

pub type Peers<Key> = Arc<Mutex<HashMap<Key, Box<dyn Peer>>>>;

pub trait PeersRouting<T> {
    /// Get list of all connected peers as PeerInfo for broadcasting
    async fn get_peer_list(&self) -> Vec<PeerInfo>;

    async fn update(
        &self,
        peer_id: T,
        sdn_client_ip: String,
        client_pub_key: String,
        state: PeerState,
        is_exit_node: bool,
    );
}

impl<T: Eq + Hash> PeersRouting<T> for Peers<T> {
    async fn get_peer_list(&self) -> Vec<PeerInfo> {
        self.lock()
            .await
            .values()
            .filter(|peer| peer.get_state() == PeerState::HandshakeDone)
            .map(|peer| {
                PeerInfo::new(
                    &peer.get_sdn_addr().to_string(),
                    &peer.get_client_public_key().unwrap_or_default(),
                    peer.get_is_exit_node(),
                )
            })
            .collect()
    }

    async fn update(
        &self,
        peer_id: T,
        sdn_client_ip: String,
        client_pub_key: String,
        state: PeerState,
        is_exit_node: bool,
    ) {
        let mut peers = self.lock().await;
        peers.retain(|_, v| v.get_sdn_addr().to_string() != sdn_client_ip);

        if let Some(peer) = peers.get_mut(&peer_id) {
            peer.set_sdn_addr(&Ipv4Addr::from_str(&sdn_client_ip).expect("Invalid SDN client IP"));
            peer.set_client_public_key(Some(client_pub_key));
            peer.set_state(state);
            peer.set_is_exit_node(is_exit_node);
        }
    }
}

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

/// Trait for peer routing operations
pub trait UdpPeersRouting: PeersRouting<SocketAddr> {
    /// Find peer address by SDN IP
    async fn find_by_sdn_ip(&self, sdn_ip: &Ipv4Addr) -> Option<SocketAddr>;
    /// Find all HandshakeDone peer addresses except the sender
    async fn find_all_addrs_except(&self, src_sdn_ip: &Ipv4Addr) -> Vec<SocketAddr>;
}

impl UdpPeersRouting for Peers<SocketAddr> {
    async fn find_by_sdn_ip(&self, sdn_ip: &Ipv4Addr) -> Option<SocketAddr> {
        let peers = self.lock().await;
        for (addr, peer) in peers.iter() {
            if peer.get_state() == PeerState::HandshakeDone && peer.get_sdn_addr() == *sdn_ip {
                return Some(*addr);
            }
        }
        None
    }

    async fn find_all_addrs_except(&self, src_sdn_ip: &Ipv4Addr) -> Vec<SocketAddr> {
        let peers = self.lock().await;
        peers
            .iter()
            .filter(|(_, peer)| {
                peer.get_state() == PeerState::HandshakeDone && peer.get_sdn_addr() != *src_sdn_ip
            })
            .map(|(addr, _)| *addr)
            .collect()
    }
}

/// Trait for WebSocket peer routing operations
pub trait TcpPeersRouting {
    /// Find peer TX channel by SDN IP
    async fn find_tx_by_sdn_ip(&self, sdn_ip: &Ipv4Addr) -> Option<Tx>;

    /// Get all TX channels for connected peers (for broadcasting)
    async fn get_all_tx(&self) -> Vec<Tx>;
    /// Get all TX channels except the sender (for multicast/broadcast fan-out)
    async fn get_all_tx_except(&self, src_sdn_ip: &Ipv4Addr) -> Vec<Tx>;
}

impl TcpPeersRouting for Peers<i32> {
    async fn find_tx_by_sdn_ip(&self, sdn_ip: &Ipv4Addr) -> Option<Tx> {
        let peers = self.lock().await;
        for peer in peers.values() {
            if peer.get_state() == PeerState::HandshakeDone && peer.get_sdn_addr() == *sdn_ip {
                if let Some(tcp_peer) = peer.as_any().downcast_ref::<TcpPeer>() {
                    return Some(tcp_peer.tx.clone());
                }
            }
        }
        None
    }

    async fn get_all_tx(&self) -> Vec<Tx> {
        let peers = self.lock().await;
        peers
            .values()
            .filter(|peer| peer.get_state() == PeerState::HandshakeDone)
            .filter_map(|peer| {
                peer.as_any()
                    .downcast_ref::<TcpPeer>()
                    .map(|tcp_peer| tcp_peer.tx.clone())
            })
            .collect()
    }

    async fn get_all_tx_except(&self, src_sdn_ip: &Ipv4Addr) -> Vec<Tx> {
        let peers = self.lock().await;
        peers
            .values()
            .filter(|peer| {
                peer.get_state() == PeerState::HandshakeDone && peer.get_sdn_addr() != *src_sdn_ip
            })
            .filter_map(|peer| {
                peer.as_any()
                    .downcast_ref::<TcpPeer>()
                    .map(|tcp_peer| tcp_peer.tx.clone())
            })
            .collect()
    }
}
