use std::net::Ipv4Addr;
use std::str::FromStr;
use anyhow::{anyhow, Result};
use log::{debug};

pub const HANDSHAKE_REQUEST_HEADER: [u8; 3] = [0, 1, 2];
const HANDSHAKE_REQUEST_SIZE: usize = 35;

#[derive(PartialEq)]
pub enum HandshakeStatus {
    Pending,
    Initialized,
}

pub struct HandshakeReq {
    pub header: [u8; 3],
    pub client_key: [u8; 32],
}

impl HandshakeReq {
    pub fn new(client_key: &Vec<u8>) -> Result<HandshakeReq> {
        Ok(HandshakeReq {
            header: HANDSHAKE_REQUEST_HEADER,
            client_key: client_key[..32].try_into()?
        })
    }
    pub fn serialize(self: &Self) -> Vec<u8> {
        return [&self.header, &self.client_key[..]].concat();
    }
    pub fn deserialize(buf: &[u8]) -> Result<Self> {
        debug!("{}", buf.len());
        if buf.len() != HANDSHAKE_REQUEST_SIZE {
            return Err(anyhow!("Invalid handshake request"));
        }
        Ok(HandshakeReq {
            header: HANDSHAKE_REQUEST_HEADER,
            client_key: buf[3..32].try_into()?,
        })
    }
    pub fn size() -> usize {
        return HANDSHAKE_REQUEST_SIZE;
    }
}

pub const HANDSHAKE_REPLY_HEADER: [u8; 3] = [3, 4, 5];
const HANDSHAKE_REPLY_SIZE: usize = 3;

pub struct HandshakeRep {
    pub header: [u8; 3],
}
impl HandshakeRep {
    pub fn new() -> HandshakeRep {
        HandshakeRep {
            header: HANDSHAKE_REPLY_HEADER,
        }
    }
    pub fn serialize(self: &Self) -> Vec<u8> {
        return self.header.to_vec();
    }
    pub fn deserialize(buf: &[u8]) -> Result<Self> {
        if buf.len() != HANDSHAKE_REPLY_SIZE {
            return Err(anyhow!("Invalid handshake reply"));
        }
        Ok(HandshakeRep {
            header: HANDSHAKE_REPLY_HEADER,
        })
    }
    pub fn size() -> usize {
        return HANDSHAKE_REPLY_SIZE;
    }
}
