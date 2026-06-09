use anyhow::{Result, anyhow};
use bincode::{Decode, Encode, config};
use serde::{Deserialize, Serialize};
use std::net::Ipv4Addr;

pub mod crypto;
pub mod git;
pub mod noise_session;
pub mod packet;
pub mod transport;

// Internal protocol headers
pub const HANDSHAKE_REQUEST_HEADER: [u8; 3] = [0, 1, 2];
pub const HANDSHAKE_REPLY_HEADER: [u8; 3] = [3, 4, 5];
pub const HANDSHAKE_ERROR_HEADER: [u8; 3] = [9, 10, 11];
pub const HEARTBEAT_HEADER: [u8; 3] = [6, 7, 8];
pub const NOISE_HANDSHAKE_INIT_HEADER: [u8; 3] = [0x4E, 0x48, 0x49]; // "NHI"
pub const NOISE_HANDSHAKE_RESP_HEADER: [u8; 3] = [0x4E, 0x48, 0x52]; // "NHR"

#[derive(Serialize, Deserialize)]
pub struct AuthClientRequest {
    pub public_key: String,
    pub dynamic_link: bool,
}

#[derive(PartialEq, Clone, Debug)]
pub enum PeerState {
    HandshakePending,
    HandshakeDone,
}

#[derive(Encode, Decode, PartialEq, Debug)]
pub struct HandshakeReq {
    pub header: [u8; 3],
    pub auth_key: String,
    pub client_public_key: Option<String>,
}

impl HandshakeReq {
    pub fn new(auth_key: &str, _is_dynamic: bool) -> Self {
        Self {
            header: HANDSHAKE_REQUEST_HEADER,
            auth_key: auth_key.to_string(),
            client_public_key: None,
        }
    }

    pub fn new_with_crypto(auth_key: &str, client_public_key: &str, _is_dynamic: bool) -> Self {
        Self {
            header: HANDSHAKE_REQUEST_HEADER,
            auth_key: auth_key.to_string(),
            client_public_key: Some(client_public_key.to_string()),
        }
    }
    pub fn serialize(&self) -> Result<Vec<u8>> {
        match bincode::encode_to_vec(self, config::standard()) {
            Ok(v) => Ok(v),
            Err(err) => Err(anyhow!(err)),
        }
    }
    pub fn deserialize(buf: &[u8]) -> Result<Self> {
        match bincode::decode_from_slice::<Self, _>(buf, config::standard()) {
            Ok((v, _)) => Ok(v),
            Err(err) => Err(anyhow!(err)),
        }
    }
    pub fn size(&self) -> Result<usize> {
        Ok(self.serialize()?.len())
    }
}

#[derive(Encode, Decode, PartialEq, Debug)]
pub struct HandshakeRep {
    pub header: [u8; 3],
    pub netmask: String,
    pub network: String,
    pub sdn_ip_addr: String,
    pub server_public_key: Option<String>,
}

impl HandshakeRep {
    pub fn new(netmask: &str, network: &str, sdn_ip_addr: &str) -> Self {
        Self {
            header: HANDSHAKE_REPLY_HEADER,
            netmask: netmask.to_string(),
            network: network.to_string(),
            sdn_ip_addr: sdn_ip_addr.to_string(),
            server_public_key: None,
        }
    }

    pub fn new_with_crypto(
        netmask: &str,
        network: &str,
        sdn_ip_addr: &str,
        server_public_key: &str,
    ) -> Self {
        Self {
            header: HANDSHAKE_REPLY_HEADER,
            netmask: netmask.to_string(),
            network: network.to_string(),
            sdn_ip_addr: sdn_ip_addr.to_string(),
            server_public_key: Some(server_public_key.to_string()),
        }
    }
    pub fn serialize(&self) -> Result<Vec<u8>> {
        match bincode::encode_to_vec(self, config::standard()) {
            Ok(v) => Ok(v),
            Err(err) => Err(anyhow!(err)),
        }
    }
    pub fn deserialize(buf: &[u8]) -> Result<Self> {
        match bincode::decode_from_slice::<Self, _>(buf, config::standard()) {
            Ok((v, _)) => Ok(v),
            Err(err) => Err(anyhow!(err)),
        }
    }
    pub fn size(&self) -> Result<usize> {
        Ok(self.serialize()?.len())
    }
}

#[derive(Encode, Decode, PartialEq, Debug)]
pub struct HandshakeError {
    pub header: [u8; 3],
    pub error_message: String,
}

impl HandshakeError {
    pub fn new(error_message: &str) -> Self {
        Self {
            header: HANDSHAKE_ERROR_HEADER,
            error_message: error_message.to_string(),
        }
    }
    pub fn serialize(&self) -> Result<Vec<u8>> {
        match bincode::encode_to_vec(self, config::standard()) {
            Ok(v) => Ok(v),
            Err(err) => Err(anyhow!(err)),
        }
    }
    pub fn deserialize(buf: &[u8]) -> Result<Self> {
        match bincode::decode_from_slice::<Self, _>(buf, config::standard()) {
            Ok((v, _)) => Ok(v),
            Err(err) => Err(anyhow!(err)),
        }
    }
    pub fn size(&self) -> Result<usize> {
        Ok(self.serialize()?.len())
    }
}

pub fn calculate_network_address(ip_str: &str, netmask_str: &str) -> Result<Ipv4Addr, String> {
    let ip: Ipv4Addr = ip_str.parse().map_err(|_| "Invalid IP address format")?;
    let netmask: Ipv4Addr = netmask_str.parse().map_err(|_| "Invalid netmask format")?;

    let ip_octets = ip.octets();
    let netmask_octets = netmask.octets();

    let network_octets = [
        ip_octets[0] & netmask_octets[0],
        ip_octets[1] & netmask_octets[1],
        ip_octets[2] & netmask_octets[2],
        ip_octets[3] & netmask_octets[3],
    ];

    Ok(Ipv4Addr::from(network_octets))
}

#[derive(Encode, Decode, PartialEq, Debug)]
pub struct UDPHeartbeat {
    pub header: [u8; 3],
}

impl Default for UDPHeartbeat {
    fn default() -> Self {
        Self::new()
    }
}

impl UDPHeartbeat {
    pub fn new() -> Self {
        Self {
            header: HEARTBEAT_HEADER,
        }
    }

    pub fn serialize(&self) -> Result<Vec<u8>> {
        match bincode::encode_to_vec(self, config::standard()) {
            Ok(v) => Ok(v),
            Err(err) => Err(anyhow!(err)),
        }
    }

    pub fn deserialize(buf: &[u8]) -> Result<Self> {
        match bincode::decode_from_slice::<Self, _>(buf, config::standard()) {
            Ok((v, _)) => Ok(v),
            Err(err) => Err(anyhow!(err)),
        }
    }

    pub fn size(&self) -> Result<usize> {
        Ok(self.serialize()?.len())
    }
}

#[derive(Encode, Decode, PartialEq, Debug)]
pub struct NoiseHandshakeInit {
    pub header: [u8; 3],
    pub payload: Vec<u8>,
}

impl NoiseHandshakeInit {
    pub fn new(payload: Vec<u8>) -> Self {
        Self {
            header: NOISE_HANDSHAKE_INIT_HEADER,
            payload,
        }
    }

    pub fn serialize(&self) -> Result<Vec<u8>> {
        match bincode::encode_to_vec(self, config::standard()) {
            Ok(v) => Ok(v),
            Err(err) => Err(anyhow!(err)),
        }
    }

    pub fn deserialize(buf: &[u8]) -> Result<Self> {
        match bincode::decode_from_slice::<Self, _>(buf, config::standard()) {
            Ok((v, _)) => Ok(v),
            Err(err) => Err(anyhow!(err)),
        }
    }
}

#[derive(Encode, Decode, PartialEq, Debug)]
pub struct NoiseHandshakeResp {
    pub header: [u8; 3],
    pub payload: Vec<u8>,
}

impl NoiseHandshakeResp {
    pub fn new(payload: Vec<u8>) -> Self {
        Self {
            header: NOISE_HANDSHAKE_RESP_HEADER,
            payload,
        }
    }

    pub fn serialize(&self) -> Result<Vec<u8>> {
        match bincode::encode_to_vec(self, config::standard()) {
            Ok(v) => Ok(v),
            Err(err) => Err(anyhow!(err)),
        }
    }

    pub fn deserialize(buf: &[u8]) -> Result<Self> {
        match bincode::decode_from_slice::<Self, _>(buf, config::standard()) {
            Ok((v, _)) => Ok(v),
            Err(err) => Err(anyhow!(err)),
        }
    }
}

// P2P E2E encryption headers
pub const PEER_LIST_HEADER: [u8; 3] = [0x50, 0x4C, 0x53]; // "PLS"
pub const PEER_ANNOUNCE_HEADER: [u8; 3] = [0x50, 0x41, 0x4E]; // "PAN"
pub const P2P_HANDSHAKE_INIT_HEADER: [u8; 3] = [0x50, 0x48, 0x49]; // "PHI"
pub const P2P_HANDSHAKE_RESP_HEADER: [u8; 3] = [0x50, 0x48, 0x52]; // "PHR"
pub const RELAY_PACKET_HEADER: [u8; 3] = [0x52, 0x4C, 0x59]; // "RLY"

#[derive(Encode, Decode, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum PeerEventType {
    Connected,
    Disconnected,
}

#[derive(Encode, Decode, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct PeerInfo {
    pub sdn_ip: String,
    pub public_key: String,
}

impl PeerInfo {
    pub fn new(sdn_ip: &str, public_key: &str) -> Self {
        Self {
            sdn_ip: sdn_ip.to_string(),
            public_key: public_key.to_string(),
        }
    }
}

#[derive(Encode, Decode, PartialEq, Debug)]
pub struct PeerList {
    pub header: [u8; 3],
    pub peers: Vec<PeerInfo>,
}

impl PeerList {
    pub fn new(peers: Vec<PeerInfo>) -> Self {
        Self {
            header: PEER_LIST_HEADER,
            peers,
        }
    }

    pub fn serialize(&self) -> Result<Vec<u8>> {
        match bincode::encode_to_vec(self, config::standard()) {
            Ok(v) => Ok(v),
            Err(err) => Err(anyhow!(err)),
        }
    }

    pub fn deserialize(buf: &[u8]) -> Result<Self> {
        match bincode::decode_from_slice::<Self, _>(buf, config::standard()) {
            Ok((v, _)) => Ok(v),
            Err(err) => Err(anyhow!(err)),
        }
    }
}

#[derive(Encode, Decode, PartialEq, Debug)]
pub struct PeerAnnounce {
    pub header: [u8; 3],
    pub event_type: PeerEventType,
    pub peer: PeerInfo,
}

impl PeerAnnounce {
    pub fn new(event_type: PeerEventType, peer: PeerInfo) -> Self {
        Self {
            header: PEER_ANNOUNCE_HEADER,
            event_type,
            peer,
        }
    }

    pub fn serialize(&self) -> Result<Vec<u8>> {
        match bincode::encode_to_vec(self, config::standard()) {
            Ok(v) => Ok(v),
            Err(err) => Err(anyhow!(err)),
        }
    }

    pub fn deserialize(buf: &[u8]) -> Result<Self> {
        match bincode::decode_from_slice::<Self, _>(buf, config::standard()) {
            Ok((v, _)) => Ok(v),
            Err(err) => Err(anyhow!(err)),
        }
    }
}

#[derive(Encode, Decode, PartialEq, Debug)]
pub struct P2PHandshakeInit {
    pub header: [u8; 3],
    pub initiator_sdn_ip: String,
    pub responder_sdn_ip: String,
    pub noise_payload: Vec<u8>,
}

impl P2PHandshakeInit {
    pub fn new(initiator_sdn_ip: &str, responder_sdn_ip: &str, noise_payload: Vec<u8>) -> Self {
        Self {
            header: P2P_HANDSHAKE_INIT_HEADER,
            initiator_sdn_ip: initiator_sdn_ip.to_string(),
            responder_sdn_ip: responder_sdn_ip.to_string(),
            noise_payload,
        }
    }

    pub fn serialize(&self) -> Result<Vec<u8>> {
        match bincode::encode_to_vec(self, config::standard()) {
            Ok(v) => Ok(v),
            Err(err) => Err(anyhow!(err)),
        }
    }

    pub fn deserialize(buf: &[u8]) -> Result<Self> {
        match bincode::decode_from_slice::<Self, _>(buf, config::standard()) {
            Ok((v, _)) => Ok(v),
            Err(err) => Err(anyhow!(err)),
        }
    }
}

#[derive(Encode, Decode, PartialEq, Debug)]
pub struct P2PHandshakeResp {
    pub header: [u8; 3],
    pub initiator_sdn_ip: String,
    pub responder_sdn_ip: String,
    pub noise_payload: Vec<u8>,
}

impl P2PHandshakeResp {
    pub fn new(initiator_sdn_ip: &str, responder_sdn_ip: &str, noise_payload: Vec<u8>) -> Self {
        Self {
            header: P2P_HANDSHAKE_RESP_HEADER,
            initiator_sdn_ip: initiator_sdn_ip.to_string(),
            responder_sdn_ip: responder_sdn_ip.to_string(),
            noise_payload,
        }
    }

    pub fn serialize(&self) -> Result<Vec<u8>> {
        match bincode::encode_to_vec(self, config::standard()) {
            Ok(v) => Ok(v),
            Err(err) => Err(anyhow!(err)),
        }
    }

    pub fn deserialize(buf: &[u8]) -> Result<Self> {
        match bincode::decode_from_slice::<Self, _>(buf, config::standard()) {
            Ok((v, _)) => Ok(v),
            Err(err) => Err(anyhow!(err)),
        }
    }
}

#[derive(Encode, Decode, PartialEq, Debug)]
pub struct RelayPacket {
    pub header: [u8; 3],
    pub src_sdn_ip: String,
    pub dst_sdn_ip: String,
    pub encrypted_payload: Vec<u8>,
}

impl RelayPacket {
    pub fn new(src_sdn_ip: &str, dst_sdn_ip: &str, encrypted_payload: Vec<u8>) -> Self {
        Self {
            header: RELAY_PACKET_HEADER,
            src_sdn_ip: src_sdn_ip.to_string(),
            dst_sdn_ip: dst_sdn_ip.to_string(),
            encrypted_payload,
        }
    }

    pub fn serialize(&self) -> Result<Vec<u8>> {
        match bincode::encode_to_vec(self, config::standard()) {
            Ok(v) => Ok(v),
            Err(err) => Err(anyhow!(err)),
        }
    }

    pub fn deserialize(buf: &[u8]) -> Result<Self> {
        match bincode::decode_from_slice::<Self, _>(buf, config::standard()) {
            Ok((v, _)) => Ok(v),
            Err(err) => Err(anyhow!(err)),
        }
    }
}

#[derive(Debug)]
pub enum MessageType {
    HandshakeReq(HandshakeReq),
    HandshakeRep(HandshakeRep),
    HandshakeError(HandshakeError),
    Heartbeat(UDPHeartbeat),
    PeerList(PeerList),
    PeerAnnounce(PeerAnnounce),
    P2PHandshakeInit(P2PHandshakeInit),
    P2PHandshakeResp(P2PHandshakeResp),
    RelayPacket(RelayPacket),
    Unknown,
}

pub fn get_message_type(buf: &[u8]) -> MessageType {
    if buf.len() < 3 {
        return MessageType::Unknown;
    }

    let header: [u8; 3] = [buf[0], buf[1], buf[2]];

    match header {
        HANDSHAKE_REQUEST_HEADER => {
            HandshakeReq::deserialize(buf).map_or(MessageType::Unknown, MessageType::HandshakeReq)
        }
        HANDSHAKE_REPLY_HEADER => {
            HandshakeRep::deserialize(buf).map_or(MessageType::Unknown, MessageType::HandshakeRep)
        }
        HANDSHAKE_ERROR_HEADER => HandshakeError::deserialize(buf)
            .map_or(MessageType::Unknown, MessageType::HandshakeError),
        HEARTBEAT_HEADER => {
            UDPHeartbeat::deserialize(buf).map_or(MessageType::Unknown, MessageType::Heartbeat)
        }
        PEER_LIST_HEADER => {
            PeerList::deserialize(buf).map_or(MessageType::Unknown, MessageType::PeerList)
        }
        PEER_ANNOUNCE_HEADER => {
            PeerAnnounce::deserialize(buf).map_or(MessageType::Unknown, MessageType::PeerAnnounce)
        }
        P2P_HANDSHAKE_INIT_HEADER => P2PHandshakeInit::deserialize(buf)
            .map_or(MessageType::Unknown, MessageType::P2PHandshakeInit),
        P2P_HANDSHAKE_RESP_HEADER => P2PHandshakeResp::deserialize(buf)
            .map_or(MessageType::Unknown, MessageType::P2PHandshakeResp),
        RELAY_PACKET_HEADER => {
            RelayPacket::deserialize(buf).map_or(MessageType::Unknown, MessageType::RelayPacket)
        }
        _ => MessageType::Unknown,
    }
}
