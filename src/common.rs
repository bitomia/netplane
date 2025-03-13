use libp2p::identity;
use serde::{Deserialize, Serialize};
use bincode;
use base64::prelude::*;
use std::net::Ipv4Addr;
use std::fs;
use crate::common;

pub const HANDSHAKE_HEADER: [u8; 3] = [0, 1, 2];

#[derive(Serialize, Deserialize)]
pub struct Handshake {
    pub header: [u8; 3],
    pub sdn_ip_addr: Ipv4Addr,
    #[serde(serialize_with = "serialize_identity", deserialize_with = "deserialize_identity")]
    pub identity: identity::Keypair,
}

fn serialize_identity<S>(identity: &identity::Keypair, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    let encoded = identity.to_protobuf_encoding().map_err(serde::ser::Error::custom)?;
    serializer.serialize_bytes(&encoded)
}

fn deserialize_identity<'de, D>(deserializer: D) -> Result<identity::Keypair, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let bytes: Vec<u8> = serde::Deserialize::deserialize(deserializer)?;
    identity::Keypair::from_protobuf_encoding(&bytes).map_err(serde::de::Error::custom)
}

pub fn serialize_handshake(handshake: &Handshake) -> Vec<u8> {
    bincode::serialize(handshake).expect("Failed to serialize handshake")
}

pub fn deserialize_handshake(data: &[u8]) -> Handshake {
    bincode::deserialize(data).expect("Failed to deserialize handshake")
}

pub fn identity_to_base64(identity: &identity::Keypair) -> String {
  BASE64_STANDARD.encode(&identity.to_protobuf_encoding().unwrap())
}

pub fn base64_to_identity(encoded: &str) -> identity::Keypair {
    let decoded = BASE64_STANDARD.decode(encoded).unwrap();
    identity::Keypair::from_protobuf_encoding(&decoded).unwrap()
}

pub fn load_identity() -> identity::Keypair {
    let key_file = "identity.json";
    if let Ok(key_data) = fs::read_to_string(key_file) {
        return common::base64_to_identity(key_data.as_str());
    }
    log::info!("Generating new identity...");
    let keypair = identity::Keypair::generate_ed25519();
    let base64_encoded = common::identity_to_base64(&keypair);
    fs::write(key_file, base64_encoded).expect("Failed to save identity");
    keypair
}


