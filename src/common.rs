use std::net::Ipv4Addr;
use std::str::FromStr;
use anyhow::{anyhow, Result};

pub const HANDSHAKE_REQUEST_HEADER: [u8; 3] = [0, 1, 2];
const HANDSHAKE_REQUEST_SIZE: usize = 7;

pub enum HandshakeStatus {
    Pending,
    WaitingReply,
    Initialized,
}

pub struct HandshakeReq {
    pub header: [u8; 3],
    pub ipv4_addr: Ipv4Addr,
}

impl HandshakeReq {
    pub fn new(ip_addr: String) -> Result<HandshakeReq> {
        Ok(HandshakeReq {
            header: HANDSHAKE_REQUEST_HEADER,
            ipv4_addr: Ipv4Addr::from_str(ip_addr.as_str())?
        })
    }
    pub fn serialize(self: &Self) -> Vec<u8> {
        return [&self.header, &self.ipv4_addr.octets()[..4]].concat();
    }
    pub fn deserialize(buf: &[u8]) -> Result<Self> {
        if buf.len() != HANDSHAKE_REQUEST_SIZE {
            return Err(anyhow!("Invalid handshake request"));
        }
        Ok(HandshakeReq {
            header: HANDSHAKE_REQUEST_HEADER,
            ipv4_addr: Ipv4Addr::new(buf[3], buf[4], buf[5], buf[6]),
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
    pub fn new() -> Result<HandshakeRep> {
        Ok(HandshakeRep {
            header: HANDSHAKE_REPLY_HEADER,
        })
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
