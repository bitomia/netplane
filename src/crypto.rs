use anyhow::{Result, anyhow};
use std::fs::write;
use std::path::Path;
use base64::{engine::general_purpose, Engine};
use ed25519_dalek::{ed25519::signature::SignerMut, SigningKey, VerifyingKey, PUBLIC_KEY_LENGTH, SECRET_KEY_LENGTH};
use rand::rngs::OsRng;
use hmac::{Hmac, Mac}; // Mac is a trait, Hmac is the struct
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

pub fn try_generate_auth_keys() -> Result<()> {
    if Path::new("private.key").exists() || Path::new("public.key").exists() {
        return Err(anyhow!("auth key files already exist"));
    }

    let signing_key: SigningKey = SigningKey::generate(&mut OsRng);
    let verify_key = VerifyingKey::from(&signing_key);

    let private_key_b64 = general_purpose::STANDARD.encode(signing_key.as_bytes());
    let public_key_b64 = general_purpose::STANDARD.encode(verify_key.as_bytes());
    write("private.key", private_key_b64)?;
    write("public.key", public_key_b64)?;
    
    Ok(())
}

pub fn try_load_auth_keys() -> Result<(SigningKey, VerifyingKey)> {
    let priv_b64 = std::fs::read_to_string("private.key")?;
    let pub_b64 = std::fs::read_to_string("public.key")?;

    let private_key = general_purpose::STANDARD.decode(priv_b64.trim())?;
    let public_key = general_purpose::STANDARD.decode(pub_b64.trim())?;

    let private = SigningKey::from_bytes(&<[u8; SECRET_KEY_LENGTH]>::try_from(private_key.as_slice())?);
    let public = VerifyingKey::from_bytes(&<[u8; PUBLIC_KEY_LENGTH]>::try_from(public_key.as_slice())?)?;
    
    Ok((private, public))
}

pub fn sign_key(key: String) -> Vec<u8> {
    let secret_key = std::env::var("SECRET_KEY").unwrap();
    let mut mac = HmacSha256::new_from_slice(secret_key.as_bytes()).expect("HMAC can take key of any size");

    mac.update(key.as_bytes());
    mac.finalize().into_bytes().to_vec()
}

pub fn verify_signed_key(key: String) -> bool {
    let secret_key = std::env::var("SECRET_KEY").unwrap();
    let mac = HmacSha256::new_from_slice(secret_key.as_bytes()).expect("HMAC can take key of any size");
    mac.verify_slice(key.as_bytes()).is_ok()
}
