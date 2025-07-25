use log::error;

pub struct IpHeader {
    pub version_ihl: u8,
    pub version: u8,
    pub ihl: u8,
    pub total_length: u16,
    pub protocol: u8,
    pub src_ip: String,
    pub src_port: u16,
    pub dst_ip: String,
    pub dst_port: u16,
}

pub fn parse_ipv4_header(packet: &[u8]) -> Option<IpHeader> {
    if packet.len() < 20 {
        println!("Packet too small to contain an IPv4 header.");
        return None;
    }

    let version_ihl = packet[0];
    let ihl = (version_ihl & 0x0F) * 4;
    let protocol = packet[9];
    let mut src_port: u16 = 0;
    let mut dst_port: u16 = 0;
    if (protocol == 6 || protocol == 17) && packet.len() >= ihl as usize + 4 {
        src_port = u16::from_be_bytes([packet[ihl as usize], packet[ihl as usize + 1]]);
        dst_port = u16::from_be_bytes([packet[ihl as usize + 2], packet[ihl as usize + 3]]);
        // if protocol == 6 { "TCP" } else { "UDP" };
    }

    Some(IpHeader {
        version_ihl,
        version: version_ihl >> 4,
        ihl,
        total_length: u16::from_be_bytes([packet[2], packet[3]]),
        protocol,
        src_ip: format!(
            "{}.{}.{}.{}",
            packet[12], packet[13], packet[14], packet[15]
        ),
        src_port,
        dst_ip: format!(
            "{}.{}.{}.{}",
            packet[16], packet[17], packet[18], packet[19]
        ),
        dst_port,
    })
}

pub fn validate_packet(buf: &[u8]) -> bool {
    if parse_ipv4_header(buf).is_some() {
        true
    } else {
        error!("Packet not supported");
        false
    }
}
