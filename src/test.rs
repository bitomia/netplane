use tokio::net::UdpSocket;
use std::env;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let args: Vec<String> = env::args().collect();
    
    // let addr = format!("0.0.0.0:{}", args[1]);
    // let dst = "137.74.167.51:12345";

    let addr = args[1].as_str();
    let dst = args[2].as_str();
    
    let socket = UdpSocket::bind(addr)
        .await
        .expect("Cannot open socket");

    println!(
        "Client bound to {:?}",
        socket.local_addr().expect("Cannot get the local addr")
    );

    println!("Sending handshake {}", dst);
    socket
        .connect(dst)
        .await
        .expect("Cannot connect");
    socket
        .send("test".as_bytes())
        .await
        .expect("Cannot send handshake");

    let mut socket_buf = [0; 1500];
    loop {
        tokio::select! {
            result = socket.recv_from(&mut socket_buf) => {
                match result {
                    Ok((amt, from)) => {
                        println!("=> Server sent {} from {}", amt, from);
                        // if let Some(header) = packet::parse_ipv4_header(&socket_buf[..amt]) {
                        //     debug!(
                        //         "{} {} {}",
                        //         header.src_ip, header.dst_ip, header.total_length
                        //     );
                        // }
                    }
                    Err(_
                    ) => todo!()
                }
            }
        }
    }
}
