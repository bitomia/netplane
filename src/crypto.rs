use anyhow::{Result, anyhow};
use std::fs::write;
use std::path::Path;
use ed25519_dalek::{SigningKey, VerifyingKey, PUBLIC_KEY_LENGTH, SECRET_KEY_LENGTH};
use rand::rngs::OsRng;
use base64::{Engine as _, engine::general_purpose};
use hmac::{Hmac, Mac};
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


fn snow_test() -> Result<(), anyhow::Error>
{
    let signing_key= "VJC7IabdEbYQvb88ulpjx6VrtCTYuuT+t6ww6/31XkA=";
    let signing_key = general_purpose::STANDARD.decode(signing_key).unwrap();
    let signing_key: [u8;32] = signing_key.try_into().unwrap();
    let signing_key: SigningKey = SigningKey::from_bytes(&signing_key);
    
    static PATTERN: &'static str = "Noise_IK_25519_ChaChaPoly_BLAKE2s";
    let initiator_keypair = snow::Builder::new(PATTERN.parse()?)
        .generate_keypair()?;

    let responder_keypair = snow::Builder::new(PATTERN.parse()?)
        .generate_keypair()?;

    // ====
    
    let mut initiator = snow::Builder::new(PATTERN.parse()?)
        .local_private_key(&initiator_keypair.private)
        .remote_public_key(&responder_keypair.public)
        .build_initiator()?;
    
    let mut responder = snow::Builder::new(PATTERN.parse()?)
        .local_private_key(&responder_keypair.private)
        .remote_public_key(&initiator_keypair.public)
        .build_responder()?;

    let (mut read_buf, mut first_msg, mut second_msg) =
        ([0u8; 1024], [0u8; 1024], [0u8; 1024]);

    let mut buf = [0u8; 1024];
    
    // -> e
    let len = initiator.write_message(&[], &mut first_msg)?;
    
    // responder processes the first message...
    responder.read_message(&first_msg[..len], &mut read_buf)?;

    // <- e, ee
    let len = responder.write_message(&[], &mut second_msg)?;

    // initiator processes the response...
    initiator.read_message(&second_msg[..len], &mut read_buf)?;

    // NN handshake complete, transition into transport mode.
    let mut initiator = initiator.into_transport_mode()?;
    let mut responder = responder.into_transport_mode()?;

    let len = initiator.write_message(b"test\0", &mut buf).unwrap();
    println!("{}", len);
    
    let mut read_buf = [0u8; 1024];
    responder.read_message(&buf[..len], &mut read_buf).unwrap();
    println!("client said: {}", String::from_utf8_lossy(&read_buf[..len]));

    Ok(())
}

pub fn sign_key(pubkey: &[u8]) -> String {
    let secret_key = std::env::var("SECRET_KEY").unwrap();
    
    let mut mac = HmacSha256::new_from_slice(secret_key.as_bytes()).expect("HMAC can take key of any size");
    mac.update(pubkey);
    
    let signature = mac.finalize().into_bytes();
    let signature_b64 = general_purpose::URL_SAFE_NO_PAD.encode(signature);
    let key_b64 = general_purpose::URL_SAFE_NO_PAD.encode(pubkey);
    format!("{}.{}", key_b64, signature_b64)
}

pub fn verify_signed_key(signed_pubkey: String) -> bool {
    let parts: Vec<&str> = signed_pubkey.split(".").collect();
    if parts.len() != 2 {
        return false;
    }
    println!("Parts: {} {}", parts[0], parts[1]);
    
    let signature = general_purpose::URL_SAFE_NO_PAD.decode(parts[1]).unwrap();

    let secret_key = std::env::var("SECRET_KEY").unwrap();
    let mut  mac = HmacSha256::new_from_slice(secret_key.as_bytes()).expect("HMAC can take key of any size");

    mac.update(general_purpose::URL_SAFE_NO_PAD.decode(parts[0]).unwrap().as_slice());
    mac.verify_slice(&signature[..]).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_noise() {
        assert!(snow_test().is_ok(), "snow test failed");
    }

    #[test]
    fn test_sign_key() {
        unsafe {
            std::env::set_var("SECRET_KEY", "secret_key");
        }
        let key = String::from("test123");
        let signed_key = sign_key(key.as_bytes());

        assert!(signed_key.len() != 0, "empty signed key");
        assert!(verify_signed_key(signed_key), "verify signed key failed");
    }
}
