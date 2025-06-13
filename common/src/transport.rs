use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use std::net::SocketAddr;
use tokio::net::{TcpListener, TcpStream, UdpSocket};
use tokio_tungstenite::accept_async;
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream, connect_async, tungstenite::protocol::Message,
};

use tungstenite::Bytes;

#[derive(Debug)]
pub struct TransportError {}

pub trait Transport: Sized {
    fn send(
        &mut self,
        buf: &[u8],
        addr: Option<&SocketAddr>,
    ) -> impl std::future::Future<Output = tokio::io::Result<usize>> + Send;
    fn recv(
        &mut self,
        buf: &mut [u8],
    ) -> impl std::future::Future<Output = tokio::io::Result<(usize, SocketAddr)>> + Send;
}

pub struct WebSocketTransport {
    write: SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
    read: SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
}

impl WebSocketTransport {
    pub async fn connect(addr: &str) -> Result<Self, TransportError> {
        let (ws_stream, _) = match connect_async(addr).await {
            Ok(val) => val,
            Err(_) => {
                return Err(TransportError {});
            }
        };
        let (write, read) = ws_stream.split();
        Ok(WebSocketTransport { write, read })
    }

    pub async fn bind<F, Fut>(addr: &str, callback: F)
    where
        F: Fn(WebSocketTransport, SocketAddr) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let listener = TcpListener::bind(&addr).await.expect("Failed to bind");

        loop {
            if let Ok((stream, addr)) = listener.accept().await {
                let ws_stream = match accept_async(MaybeTlsStream::Plain(stream)).await {
                    Ok(ws) => ws,
                    Err(e) => {
                        eprintln!("WebSocket handshake error: {}", e);
                        return;
                    }
                };
                let (write, read) = ws_stream.split();
                tokio::spawn(callback(WebSocketTransport { write, read }, addr));
            }
        }
    }
}

impl Transport for WebSocketTransport {
    async fn send(&mut self, buf: &[u8], _addr: Option<&SocketAddr>) -> tokio::io::Result<usize> {
        match self
            .write
            .send(Message::Binary(Bytes::copy_from_slice(buf)))
            .await
        {
            Ok(_) => Ok(buf.len()),
            Err(e) => Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                e.to_string(),
            )),
        }
    }

    async fn recv(&mut self, buf: &mut [u8]) -> tokio::io::Result<(usize, SocketAddr)> {
        if let Some(message) = self.read.next().await {
            match message {
                Ok(msg) => {
                    if let Message::Binary(msg) = msg {
                        let len = msg.len();
                        buf[..len].copy_from_slice(&msg);
                        return Ok((len, "0.0.0.0:0".parse().unwrap()));
                    } else {
                        let len = msg.len();
                        buf[..len].copy_from_slice(msg.into_text().unwrap().as_bytes());
                        return Ok((len, "0.0.0.0:0".parse().unwrap()));
                    }
                }
                Err(e) => {
                    eprintln!("Error processing message {}", e);
                    Err(tokio::io::Error::new(
                        tokio::io::ErrorKind::Other,
                        "Failed to handle received message",
                    ))
                }
            }
        } else {
            Err(tokio::io::Error::new(
                tokio::io::ErrorKind::Other,
                "Failed to receive message",
            ))
        }
    }
}

pub struct UdpTransport {
    socket: UdpSocket,
}

impl UdpTransport {
    pub async fn connect(&self, addr: &str) -> Result<(), TransportError> {
        let addr = addr.parse::<SocketAddr>().unwrap();
        match self.socket.connect(addr).await {
            Ok(val) => val,
            Err(_) => {
                return Err(TransportError {});
            }
        };
        Ok(())
    }

    pub async fn bind(addr: &str) -> Result<Self, TransportError> {
        let socket = match tokio::net::UdpSocket::bind(addr).await {
            Ok(val) => val,
            Err(_) => {
                return Err(TransportError {});
            }
        };
        Ok(UdpTransport { socket })
    }
}

impl Transport for UdpTransport {
    async fn send(&mut self, buf: &[u8], addr: Option<&SocketAddr>) -> tokio::io::Result<usize> {
        if let Some(addr) = addr {
            self.socket.send_to(buf, addr).await
        } else {
            self.socket.send(buf).await
        }
    }

    async fn recv(&mut self, buf: &mut [u8]) -> tokio::io::Result<(usize, SocketAddr)> {
        self.socket.recv_from(buf).await
    }
}
