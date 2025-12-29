use anyhow::{Result, anyhow};
use base64::{Engine as _, engine::general_purpose};
use log::{debug, info};
use std::collections::HashMap;

use std::net::Ipv4Addr;
use std::str::FromStr;

use netplane_common::PeerInfo;
use netplane_common::noise_session::{
    NoiseSession, create_noise_initiator, create_noise_responder,
};
use netplane_common::{P2PHandshakeInit, P2PHandshakeResp};

/// Manages P2P noise sessions with other peers
pub struct PeerSessionManager {
    /// Our own SDN IP
    own_sdn_ip: Ipv4Addr,
    /// Our private key bytes
    private_key: Vec<u8>,
    /// Our public key (base64)
    public_key: String,
    /// Known peers (SDN IP -> PeerInfo)
    known_peers: HashMap<Ipv4Addr, PeerInfo>,
    /// Established P2P sessions (SDN IP -> NoiseSession) - used for decryption
    sessions: HashMap<Ipv4Addr, NoiseSession>,
    /// Loopback session for encrypting to self (initiator session)
    /// We need separate sessions because Noise is directional
    loopback_session: Option<NoiseSession>,
    /// Pending handshakes where we are the initiator (SDN IP -> HandshakeState)
    pending_initiator: HashMap<Ipv4Addr, snow::HandshakeState>,
    /// Queued packets waiting for handshake completion (SDN IP -> packets)
    pending_packets: HashMap<Ipv4Addr, Vec<Vec<u8>>>,
}

impl PeerSessionManager {
    pub fn new(own_sdn_ip: Ipv4Addr, private_key: Vec<u8>, public_key: String) -> Self {
        // Add ourselves to known_peers for loopback support
        let mut known_peers = HashMap::new();
        known_peers.insert(
            own_sdn_ip,
            PeerInfo::new(&own_sdn_ip.to_string(), &public_key),
        );

        Self {
            own_sdn_ip,
            private_key,
            public_key,
            known_peers,
            sessions: HashMap::new(),
            loopback_session: None,
            pending_initiator: HashMap::new(),
            pending_packets: HashMap::new(),
        }
    }

    /// Add or update a known peer
    pub fn add_peer(&mut self, peer: PeerInfo) {
        if let Ok(ip) = Ipv4Addr::from_str(&peer.sdn_ip) {
            info!(
                "Added peer: {} (key: {}...)",
                peer.sdn_ip,
                &peer.public_key[..8.min(peer.public_key.len())]
            );
            self.known_peers.insert(ip, peer);
        }
    }

    /// Remove a peer and its session
    pub fn remove_peer(&mut self, sdn_ip: &Ipv4Addr) {
        self.known_peers.remove(sdn_ip);
        self.sessions.remove(sdn_ip);
        self.pending_initiator.remove(sdn_ip);
        self.pending_packets.remove(sdn_ip);
        info!("Removed peer: {}", sdn_ip);
    }

    /// Check if we know about this peer
    pub fn knows_peer(&self, sdn_ip: &Ipv4Addr) -> bool {
        self.known_peers.contains_key(sdn_ip)
    }

    /// Check if we have an established session with this peer
    pub fn has_session(&self, sdn_ip: &Ipv4Addr) -> bool {
        // For self-loopback, check loopback_session (for encryption)
        // The responder session in self.sessions is for decryption
        if *sdn_ip == self.own_sdn_ip {
            self.loopback_session.is_some()
        } else {
            self.sessions.contains_key(sdn_ip)
        }
    }

    /// Check if handshake is in progress with this peer
    pub fn handshake_in_progress(&self, sdn_ip: &Ipv4Addr) -> bool {
        self.pending_initiator.contains_key(sdn_ip)
    }

    /// Initiate P2P handshake with a peer
    pub fn initiate_handshake(&mut self, dst_sdn_ip: &Ipv4Addr) -> Result<P2PHandshakeInit> {
        let peer = self
            .known_peers
            .get(dst_sdn_ip)
            .ok_or_else(|| anyhow!("Unknown peer: {}", dst_sdn_ip))?;

        let peer_pub_key = general_purpose::URL_SAFE_NO_PAD
            .decode(&peer.public_key)
            .map_err(|e| anyhow!("Failed to decode peer public key: {}", e))?;

        let mut initiator = create_noise_initiator(&self.private_key, &peer_pub_key)?;

        let mut buf = [0u8; 1024];
        let len = initiator
            .write_message(&[], &mut buf)
            .map_err(|e| anyhow!("Failed to write handshake init: {}", e))?;

        self.pending_initiator.insert(*dst_sdn_ip, initiator);

        info!("Initiating P2P handshake with {}", dst_sdn_ip);

        Ok(P2PHandshakeInit::new(
            &self.own_sdn_ip.to_string(),
            &dst_sdn_ip.to_string(),
            buf[..len].to_vec(),
        ))
    }

    /// Handle incoming P2P handshake init (we are the responder)
    pub fn handle_handshake_init(&mut self, init: &P2PHandshakeInit) -> Result<P2PHandshakeResp> {
        let initiator_ip = Ipv4Addr::from_str(&init.initiator_sdn_ip)
            .map_err(|e| anyhow!("Invalid initiator IP: {}", e))?;

        let peer = self
            .known_peers
            .get(&initiator_ip)
            .ok_or_else(|| anyhow!("Unknown peer: {}", initiator_ip))?;

        let peer_pub_key = general_purpose::URL_SAFE_NO_PAD
            .decode(&peer.public_key)
            .map_err(|e| anyhow!("Failed to decode peer public key: {}", e))?;

        let mut responder = create_noise_responder(&self.private_key, &peer_pub_key)?;

        let mut buf = [0u8; 1024];

        // Process init message
        responder
            .read_message(&init.noise_payload, &mut buf)
            .map_err(|e| anyhow!("Failed to read handshake init: {}", e))?;

        // Create response
        let len = responder
            .write_message(&[], &mut buf)
            .map_err(|e| anyhow!("Failed to write handshake response: {}", e))?;

        // Convert to transport mode and store session
        let transport_state = responder
            .into_transport_mode()
            .map_err(|e| anyhow!("Failed to convert to transport mode: {}", e))?;

        let session = NoiseSession::new(transport_state);
        self.sessions.insert(initiator_ip, session);

        info!(
            "P2P session established with {} (as responder)",
            initiator_ip
        );

        Ok(P2PHandshakeResp::new(
            &init.initiator_sdn_ip,
            &self.own_sdn_ip.to_string(),
            buf[..len].to_vec(),
        ))
    }

    /// Handle P2P handshake response (we were the initiator)
    pub fn handle_handshake_resp(&mut self, resp: &P2PHandshakeResp) -> Result<()> {
        let responder_ip = Ipv4Addr::from_str(&resp.responder_sdn_ip)
            .map_err(|e| anyhow!("Invalid responder IP: {}", e))?;

        let mut initiator = self
            .pending_initiator
            .remove(&responder_ip)
            .ok_or_else(|| anyhow!("No pending handshake with {}", responder_ip))?;

        let mut buf = [0u8; 1024];
        initiator
            .read_message(&resp.noise_payload, &mut buf)
            .map_err(|e| anyhow!("Failed to read handshake response: {}", e))?;

        let transport_state = initiator
            .into_transport_mode()
            .map_err(|e| anyhow!("Failed to convert to transport mode: {}", e))?;

        let session = NoiseSession::new(transport_state);

        // For self-loopback, store initiator session separately (for encryption)
        // The responder session (for decryption) was already stored in handle_handshake_init
        if responder_ip == self.own_sdn_ip {
            self.loopback_session = Some(session);
            info!("Loopback session established (as initiator)");
        } else {
            self.sessions.insert(responder_ip, session);
            info!(
                "P2P session established with {} (as initiator)",
                responder_ip
            );
        }

        Ok(())
    }

    /// Encrypt data for a specific peer
    pub async fn encrypt_for(&self, dst: &Ipv4Addr, data: &[u8]) -> Result<Vec<u8>> {
        // For self-loopback, use the initiator session (loopback_session)
        if *dst == self.own_sdn_ip {
            let session = self
                .loopback_session
                .as_ref()
                .ok_or_else(|| anyhow!("No loopback session"))?;
            session.encrypt(data).await
        } else {
            let session = self
                .sessions
                .get(dst)
                .ok_or_else(|| anyhow!("No session with {}", dst))?;
            session.encrypt(data).await
        }
    }

    /// Decrypt data from a specific peer
    pub async fn decrypt_from(&self, src: &Ipv4Addr, data: &[u8]) -> Result<Vec<u8>> {
        let session = self
            .sessions
            .get(src)
            .ok_or_else(|| anyhow!("No session with {}", src))?;
        session.decrypt(data).await
    }

    /// Queue a packet for later sending (while waiting for handshake)
    pub fn queue_packet(&mut self, dst: &Ipv4Addr, packet: Vec<u8>) {
        self.pending_packets
            .entry(*dst)
            .or_insert_with(Vec::new)
            .push(packet);
        debug!("Queued packet for {} (waiting for handshake)", dst);
    }

    /// Get and clear pending packets for a peer
    pub fn take_pending_packets(&mut self, dst: &Ipv4Addr) -> Vec<Vec<u8>> {
        self.pending_packets.remove(dst).unwrap_or_default()
    }
}
