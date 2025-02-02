use std::net::Ipv4Addr;

pub const HANDSHAKE_HEADER: [u8; 3] = [0, 1, 2];
pub const HANDSHAKE_SIZE: usize = 7;

pub struct Handshake {
    pub header: [u8; 3],
    pub ipv4_addr: Ipv4Addr,
}

pub fn handshake_serialize(handshake: &Handshake) -> Vec<u8> {
    return [&handshake.header, &handshake.ipv4_addr.octets()[..4]].concat();
}

pub fn handshake_deserialize(buf: &[u8]) -> Handshake {
    Handshake {
        header: HANDSHAKE_HEADER,
        ipv4_addr: Ipv4Addr::new(buf[3], buf[4], buf[5], buf[6]),
    }
}
