use std::net::{Ipv4Addr, SocketAddr};

pub enum SourceAddr {
    Replay(Ipv4Addr),
    Network(SocketAddr),
}
