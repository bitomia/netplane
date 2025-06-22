use anyhow::{Result, anyhow};
use bincode::{Decode, Encode, config};
use serde::{Deserialize, Serialize};
use std::net::Ipv4Addr;

pub mod crypto;
pub mod packet;
pub mod transport;

#[derive(Serialize, Deserialize)]
pub struct AuthClientRequest {
    pub public_key: String,
}

pub const HANDSHAKE_REQUEST_HEADER: [u8; 3] = [0, 1, 2];

#[derive(PartialEq, Clone)]
pub enum HandshakeStatus {
    Pending,
    Initialized,
}

#[derive(Encode, Decode, PartialEq, Debug)]
pub struct HandshakeReq {
    pub header: [u8; 3],
    pub auth_key: String,
}

impl HandshakeReq {
    pub fn new(auth_key: &String) -> Self {
        Self {
            header: HANDSHAKE_REQUEST_HEADER,
            auth_key: auth_key.clone(),
        }
    }
    pub fn serialize(self: &Self) -> Result<Vec<u8>> {
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
    pub fn size(self: &Self) -> Result<usize> {
        Ok(self.serialize()?.len())
    }
}

pub const HANDSHAKE_REPLY_HEADER: [u8; 3] = [3, 4, 5];

#[derive(Encode, Decode, PartialEq, Debug)]
pub struct HandshakeRep {
    pub header: [u8; 3],
    pub netmask: String,
    pub destination: String,
    pub sdn_ip_addr: String,
}

impl HandshakeRep {
    pub fn new(netmask: &String, destination: &String, sdn_ip_addr: &String) -> Self {
        Self {
            header: HANDSHAKE_REPLY_HEADER,
            netmask: netmask.clone(),
            destination: destination.clone(),
            sdn_ip_addr: sdn_ip_addr.clone(),
        }
    }
    pub fn serialize(self: &Self) -> Result<Vec<u8>> {
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
    pub fn size(self: &Self) -> Result<usize> {
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
