use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{Future, SinkExt, StreamExt};
use serde::Serialize;
use std::net::{SocketAddr, ToSocketAddrs};
use std::sync::Arc;
use thiserror::Error;
use tokio::net::{TcpListener, TcpStream, UdpSocket};
use tokio::sync::Mutex;
use tokio_tungstenite::accept_async;
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream, connect_async, tungstenite::protocol::Message,
};
use tungstenite::Bytes;

use crate::noise_session::NoiseSession;

#[derive(Debug, Error)]
pub enum TransportError {
    #[error("UDP error: {0}")]
    UDP(#[from] tokio::io::Error),
    #[error("Websocket error: {0}")]
    WebSocket(#[from] tungstenite::Error),
}

#[derive(Debug, Serialize, Clone)]
#[repr(C)]
pub enum TransportMode {
    UDP = 0,
    WebSocket,
}

impl TransportMode {
    pub fn from_string(mode: String) -> Option<TransportMode> {
        match mode.trim().to_lowercase().as_str() {
            "udp" => Some(TransportMode::UDP),
            "ws" | "websocket" => Some(TransportMode::WebSocket),
            _ => None,
        }
    }

    pub fn as_string(&self) -> String {
        match self {
            TransportMode::UDP => "udp".to_string(),
            TransportMode::WebSocket => "websocket".to_string(),
        }
    }
}

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

#[derive(Clone)]
pub struct WebSocketTransport {
    write: Arc<Mutex<SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>>>,
    read: Arc<Mutex<SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>>>,
}

impl WebSocketTransport {
    pub async fn connect(addr: &str) -> Result<Self, TransportError> {
        let (ws_stream, _) = match connect_async(addr).await {
            Ok(val) => val,
            Err(err) => {
                return Err(TransportError::WebSocket(err));
            }
        };
        let (write, read) = ws_stream.split();
        Ok(WebSocketTransport {
            write: Arc::new(Mutex::new(write)),
            read: Arc::new(Mutex::new(read)),
        })
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
                        continue;
                    }
                };

                let (write, read) = ws_stream.split();
                tokio::spawn(callback(
                    WebSocketTransport {
                        write: Arc::new(Mutex::new(write)),
                        read: Arc::new(Mutex::new(read)),
                    },
                    addr,
                ));
            }
        }
    }
}

impl Transport for WebSocketTransport {
    async fn send(&mut self, buf: &[u8], _addr: Option<&SocketAddr>) -> tokio::io::Result<usize> {
        let mut write = self.write.lock().await;

        match write
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
        let mut read = self.read.lock().await;

        if let Some(message) = read.next().await {
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
        let mut addr = addr.to_socket_addrs().expect("Invalid server address");
        let addr = addr
            .find(|addr| addr.is_ipv4())
            .expect("Invalid server address");

        match self.socket.connect(addr).await {
            Ok(val) => val,
            Err(err) => {
                return Err(TransportError::UDP(err));
            }
        };

        Ok(())
    }

    pub async fn bind(addr: &str) -> Result<Self, TransportError> {
        let socket = match tokio::net::UdpSocket::bind(addr).await {
            Ok(val) => val,
            Err(err) => {
                return Err(TransportError::UDP(err));
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

pub enum AnyTransport {
    WebSocket(WebSocketTransport),
    Udp(UdpTransport),
    EncryptedWebSocket(EncryptedTransport<WebSocketTransport>),
    EncryptedUdp(EncryptedTransport<UdpTransport>),
}

impl Transport for AnyTransport {
    async fn send(&mut self, buf: &[u8], addr: Option<&SocketAddr>) -> tokio::io::Result<usize> {
        match self {
            AnyTransport::WebSocket(ws) => ws.send(buf, addr).await,
            AnyTransport::Udp(udp) => udp.send(buf, addr).await,
            AnyTransport::EncryptedWebSocket(enc_ws) => enc_ws.send(buf, addr).await,
            AnyTransport::EncryptedUdp(enc_udp) => enc_udp.send(buf, addr).await,
        }
    }

    async fn recv(&mut self, buf: &mut [u8]) -> tokio::io::Result<(usize, SocketAddr)> {
        match self {
            AnyTransport::WebSocket(ws) => ws.recv(buf).await,
            AnyTransport::Udp(udp) => udp.recv(buf).await,
            AnyTransport::EncryptedWebSocket(enc_ws) => enc_ws.recv(buf).await,
            AnyTransport::EncryptedUdp(enc_udp) => enc_udp.recv(buf).await,
        }
    }
}

pub struct EncryptedTransport<T: Transport> {
    inner: T,
    noise_session: Option<NoiseSession>,
}

impl<T: Transport> EncryptedTransport<T> {
    pub fn new(transport: T) -> Self {
        Self {
            inner: transport,
            noise_session: None,
        }
    }

    pub fn with_noise_session(transport: T, session: NoiseSession) -> Self {
        Self {
            inner: transport,
            noise_session: Some(session),
        }
    }

    pub fn set_noise_session(&mut self, session: NoiseSession) {
        self.noise_session = Some(session);
    }

    pub fn is_encrypted(&self) -> bool {
        self.noise_session.is_some()
    }
}

impl<T: Transport + Send> Transport for EncryptedTransport<T> {
    async fn send(&mut self, buf: &[u8], addr: Option<&SocketAddr>) -> tokio::io::Result<usize> {
        let noise_session = self.noise_session.as_ref().ok_or_else(|| {
            tokio::io::Error::new(
                tokio::io::ErrorKind::NotConnected,
                "No noise session established - encryption required",
            )
        })?;
        let encrypted = noise_session
            .encrypt(buf)
            .await
            .map_err(|e| tokio::io::Error::new(tokio::io::ErrorKind::Other, e.to_string()))?;
        self.inner.send(&encrypted, addr).await
    }

    async fn recv(&mut self, buf: &mut [u8]) -> tokio::io::Result<(usize, SocketAddr)> {
        let (amt, addr) = self.inner.recv(buf).await?;

        let noise_session = self.noise_session.as_ref().ok_or_else(|| {
            tokio::io::Error::new(
                tokio::io::ErrorKind::NotConnected,
                "No noise session established - encryption required",
            )
        })?;
        let decrypted = noise_session
            .decrypt(&buf[..amt])
            .await
            .map_err(|e| tokio::io::Error::new(tokio::io::ErrorKind::Other, e.to_string()))?;

        let len = decrypted.len().min(buf.len());
        buf[..len].copy_from_slice(&decrypted[..len]);
        Ok((len, addr))
    }
}
