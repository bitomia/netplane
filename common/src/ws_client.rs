pub mod transport;

use common::transport::{Transport, WebSocketTransport};

#[tokio::main]
async fn main() {
    let addr = "ws://127.0.0.1:9001";
    let mut c = WebSocketTransport::connect(&addr).await.unwrap();

    let _ = c.send("hello world".as_bytes(), None).await;

    let mut buf: [u8; 1500] = [0; 1500];
    match c.recv(&mut buf).await {
        Ok((size, _)) => {
            println!("> {}", String::from_utf8(buf[..size].to_vec()).unwrap());
        }
        _ => {}
    }
}
