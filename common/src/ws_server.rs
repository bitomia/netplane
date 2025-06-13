pub mod transport;

use common::transport::{Transport, WebSocketTransport};
use std::net::SocketAddr;

#[tokio::main]
async fn main() {
    let addr = "127.0.0.1:9001";
    WebSocketTransport::bind(addr, handle_connection).await;
}

async fn handle_connection(mut socket: WebSocketTransport, addr: SocketAddr) {
    println!("WebSocket connection established: {}", addr);

    loop {
        let mut buf: [u8; 1500] = [0; 1500];
        match socket.recv(&mut buf).await {
            Ok((size, _)) => {
                println!("{}", String::from_utf8(buf[..size].to_vec()).unwrap());
                if let Err(_) = socket.send(&buf, Some(&addr)).await {
                    break;
                }
            }
            Err(_) => {
                break;
            }
        }
    }

    println!("Connection closed: {}", addr);
}
