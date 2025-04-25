use tokio::net::UdpSocket;
use tokio::time;
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

    let mut interval = time::interval(time::Duration::from_millis(100));
    let mut socket_buf = [0; 1500];
    loop {
        tokio::select! {
            _ = interval.tick() => {
                println!(".");
                socket
                .send_to("test".as_bytes(), dst)
                .await
                .expect("Cannot send handshake");
            },
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
                    Err(e) => println!("{}", e)
                }
            }
        }
    }
}
