use anyhow::{Result, anyhow};
use std::sync::Arc;
use tokio::sync::Mutex;

pub static NOISE_PARAMS: &'static str = "Noise_IK_25519_ChaChaPoly_BLAKE2s";

#[derive(Clone)]
pub struct NoiseSession {
    transport: Arc<Mutex<snow::TransportState>>,
}

impl NoiseSession {
    pub fn new(transport: snow::TransportState) -> Self {
        Self {
            transport: Arc::new(Mutex::new(transport)),
        }
    }

    pub async fn encrypt(&self, data: &[u8]) -> Result<Vec<u8>> {
        let mut transport = self.transport.lock().await;
        let mut encrypted_buf = vec![0u8; data.len() + 16]; // Extra space for tag
        let len = transport
            .write_message(data, &mut encrypted_buf)
            .map_err(|e| anyhow!("Encryption failed: {}", e))?;
        encrypted_buf.truncate(len);
        Ok(encrypted_buf)
    }

    pub async fn decrypt(&self, encrypted_data: &[u8]) -> Result<Vec<u8>> {
        let mut transport = self.transport.lock().await;
        let mut decrypted_buf = vec![0u8; encrypted_data.len()];
        let len = transport
            .read_message(encrypted_data, &mut decrypted_buf)
            .map_err(|e| anyhow!("Decryption failed: {}", e))?;
        decrypted_buf.truncate(len);
        Ok(decrypted_buf)
    }
}

pub fn create_noise_initiator(
    local_private_key: &[u8],
    remote_public_key: &[u8],
) -> Result<snow::HandshakeState> {
    let parsed_pattern = NOISE_PARAMS
        .parse()
        .map_err(|e| anyhow!("Failed to parse noise pattern: {}", e))?;

    let builder = snow::Builder::new(parsed_pattern)
        .local_private_key(local_private_key)
        .remote_public_key(remote_public_key);

    builder
        .build_initiator()
        .map_err(|e| anyhow!("Failed to create noise initiator: {}", e))
}

pub fn create_noise_responder(
    local_private_key: &[u8],
    remote_public_key: &[u8],
) -> Result<snow::HandshakeState> {
    let parsed_pattern = NOISE_PARAMS
        .parse()
        .map_err(|e| anyhow!("Failed to parse noise pattern: {}", e))?;

    let builder = snow::Builder::new(parsed_pattern)
        .local_private_key(local_private_key)
        .remote_public_key(remote_public_key);

    builder
        .build_responder()
        .map_err(|e| anyhow!("Failed to create noise responder: {}", e))
}

pub fn perform_noise_handshake(
    mut initiator: snow::HandshakeState,
    mut responder: snow::HandshakeState,
) -> Result<(NoiseSession, NoiseSession)> {
    let mut buf = [0u8; 1024];
    let mut first_msg = [0u8; 1024];
    let mut second_msg = [0u8; 1024];

    // Step 1: Initiator -> Responder (e)
    let len = initiator
        .write_message(&[], &mut first_msg)
        .map_err(|e| anyhow!("Handshake step 1 failed: {}", e))?;

    // Step 2: Responder processes first message
    responder
        .read_message(&first_msg[..len], &mut buf)
        .map_err(|e| anyhow!("Handshake step 2 failed: {}", e))?;

    // Step 3: Responder -> Initiator (e, ee)
    let len = responder
        .write_message(&[], &mut second_msg)
        .map_err(|e| anyhow!("Handshake step 3 failed: {}", e))?;

    // Step 4: Initiator processes response
    initiator
        .read_message(&second_msg[..len], &mut buf)
        .map_err(|e| anyhow!("Handshake step 4 failed: {}", e))?;

    // Convert to transport mode
    let initiator_transport = initiator
        .into_transport_mode()
        .map_err(|e| anyhow!("Failed to convert initiator to transport: {}", e))?;
    let responder_transport = responder
        .into_transport_mode()
        .map_err(|e| anyhow!("Failed to convert responder to transport: {}", e))?;

    Ok((
        NoiseSession::new(initiator_transport),
        NoiseSession::new(responder_transport),
    ))
}

use crate::transport::{AnyTransport, Transport};
use crate::{NoiseHandshakeInit, NoiseHandshakeResp};

/// Perform noise handshake as initiator (client) over the transport
pub async fn perform_noise_handshake_initiator(
    mut initiator: snow::HandshakeState,
    transport: &mut AnyTransport,
) -> Result<NoiseSession> {
    let mut buf = [0u8; 1024];

    // Step 1: Send initiator message (-> e, es, s, ss)
    let len = initiator
        .write_message(&[], &mut buf)
        .map_err(|e| anyhow!("Failed to create initiator message: {}", e))?;

    let init_msg = NoiseHandshakeInit::new(buf[..len].to_vec());
    transport
        .send(&init_msg.serialize()?, None)
        .await
        .map_err(|e| anyhow!("Failed to send noise init: {}", e))?;

    // Step 2: Receive responder message (<- e, ee, se)
    let mut recv_buf = [0u8; 1500];
    let (amt, _) = transport
        .recv(&mut recv_buf)
        .await
        .map_err(|e| anyhow!("Failed to receive noise response: {}", e))?;

    let resp = NoiseHandshakeResp::deserialize(&recv_buf[..amt])
        .map_err(|e| anyhow!("Failed to deserialize noise response: {}", e))?;

    // Process the response
    initiator
        .read_message(&resp.payload, &mut buf)
        .map_err(|e| anyhow!("Failed to process responder message: {}", e))?;

    // Convert to transport mode
    let transport_state = initiator
        .into_transport_mode()
        .map_err(|e| anyhow!("Failed to convert to transport mode: {}", e))?;

    Ok(NoiseSession::new(transport_state))
}

/// Process noise handshake init and create responder session
pub fn process_noise_handshake_init(
    mut responder: snow::HandshakeState,
    init_payload: &[u8],
) -> Result<(NoiseHandshakeResp, NoiseSession)> {
    let mut buf = [0u8; 1024];

    // Process initiator message
    responder
        .read_message(init_payload, &mut buf)
        .map_err(|e| anyhow!("Failed to process initiator message: {}", e))?;

    // Create response message
    let len = responder
        .write_message(&[], &mut buf)
        .map_err(|e| anyhow!("Failed to create responder message: {}", e))?;

    let resp = NoiseHandshakeResp::new(buf[..len].to_vec());

    // Convert to transport mode
    let transport_state = responder
        .into_transport_mode()
        .map_err(|e| anyhow!("Failed to convert responder to transport mode: {}", e))?;

    Ok((resp, NoiseSession::new(transport_state)))
}
