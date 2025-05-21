use anyhow::{anyhow, Result};
use bincode::{config, Decode, Encode};

pub const HANDSHAKE_REQUEST_HEADER: [u8; 3] = [0, 1, 2];

#[derive(PartialEq)]
pub enum HandshakeStatus {
    Pending,
    Initialized,
}

#[derive(Encode, Decode, PartialEq, Debug)]
pub struct HandshakeReq {
    pub header: [u8; 3],
    pub public_key: String,
}

impl HandshakeReq {
    pub fn new(public_key: &String) -> Self {
        Self {
            header: HANDSHAKE_REQUEST_HEADER,
            public_key: public_key.clone(),
        }
    }
    pub fn serialize(self: &Self) -> Result<Vec<u8>> {
        match bincode::encode_to_vec(self, config::standard()) {
            Ok(v) => Ok(v),
            Err(err) => Err(anyhow!(err))
        }
    }
    pub fn deserialize(buf: &[u8]) -> Result<Self> {
        match bincode::decode_from_slice::<Self, _>(buf, config::standard()) {
            Ok((v, _)) => Ok(v),
            Err(err) => Err(anyhow!(err))
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
            Err(err) => Err(anyhow!(err))
        }
    }
    pub fn deserialize(buf: &[u8]) -> Result<Self> {
        match bincode::decode_from_slice::<Self, _>(buf, config::standard()) {
            Ok((v, _)) => Ok(v),
            Err(err) => Err(anyhow!(err))
        }
    }
    pub fn size(self: &Self) -> Result<usize> {
        Ok(self.serialize()?.len())
    }
}
