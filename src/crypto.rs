use anyhow::{Result, anyhow};
use std::fs::write;
use std::path::Path;
use base64::{engine::general_purpose, Engine};
use ed25519_dalek::{SigningKey, VerifyingKey, SECRET_KEY_LENGTH, PUBLIC_KEY_LENGTH};
use rand::rngs::OsRng;

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
