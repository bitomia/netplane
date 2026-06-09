use anyhow::{Result, anyhow};
use base64::{Engine as _, engine::general_purpose};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::fs::write;
use std::io::{Error, ErrorKind};
use std::path::Path;

use crate::noise_session::*;

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthData {
    pub client_id: String,
}

pub fn load_auth_key(authkey_path: String) -> Result<String> {
    let key = std::fs::read_to_string(authkey_path.as_str()).map_err(|err| {
        std::io::Error::new(err.kind(), format!("Opening auth.key file failed: {}", err))
    })?;
    Ok(key)
}

pub fn try_generate_crypto_keys(
    public_filepath: &str,
    private_filepath: &str,
) -> Result<(), std::io::Error> {
    if Path::new(private_filepath).exists() || Path::new(public_filepath).exists() {
        return Err(Error::new(
            ErrorKind::AlreadyExists,
            "key files already exist",
        ));
    }

    let parsed_pattern = match NOISE_PARAMS.parse() {
        Ok(value) => value,
        Err(_) => {
            return Err(Error::other("Error parsing pattern"));
        }
    };
    let keypair = match snow::Builder::new(parsed_pattern).generate_keypair() {
        Ok(value) => value,
        Err(_) => {
            return Err(Error::other("Error generating keypair"));
        }
    };

    let public_b64 = general_purpose::URL_SAFE_NO_PAD.encode(keypair.public);
    let private_b64 = general_purpose::URL_SAFE_NO_PAD.encode(keypair.private);

    write(public_filepath, public_b64)?;
    write(private_filepath, private_b64)?;

    Ok(())
}

pub fn try_load_crypto_keys(
    public_filepath: &str,
    private_filepath: &str,
) -> Result<(String, String)> {
    let pub_b64 = std::fs::read_to_string(public_filepath)?;
    let priv_b64 = std::fs::read_to_string(private_filepath)?;

    Ok((pub_b64, priv_b64))
}

pub fn sign_key(key: &[u8]) -> String {
    let secret_key = std::env::var("AUTH_SECRET").expect("AUTH_SECRET env var not found");

    let mut mac =
        HmacSha256::new_from_slice(secret_key.as_bytes()).expect("HMAC can take key of any size");
    mac.update(key);

    let signature = mac.finalize().into_bytes();
    let signature_b64 = general_purpose::URL_SAFE_NO_PAD.encode(signature);
    let key_b64 = general_purpose::URL_SAFE_NO_PAD.encode(key);
    format!("{}.{}", key_b64, signature_b64)
}

pub fn verify_signed_key(signed_key: String) -> Result<AuthData> {
    let parts: Vec<&str> = signed_key.split(".").collect();
    if parts.len() != 2 {
        return Err(anyhow!("Malformed key"));
    }
    let signature = general_purpose::URL_SAFE_NO_PAD.decode(parts[1])?;
    let secret_key = std::env::var("AUTH_SECRET").expect("AUTH_SECRET env var not found");

    let mut mac =
        HmacSha256::new_from_slice(secret_key.as_bytes()).expect("HMAC can take key of any size");
    mac.update(
        general_purpose::URL_SAFE_NO_PAD
            .decode(parts[0])
            .unwrap()
            .as_slice(),
    );

    match mac.verify_slice(&signature[..]) {
        Ok(_) => {
            let key_b64 = general_purpose::URL_SAFE_NO_PAD.decode(parts[0])?;
            let auth_data = serde_json::from_slice::<AuthData>(key_b64.as_slice())?;
            Ok(auth_data)
        }
        Err(err) => Err(anyhow!(err)),
    }
}

pub fn check_env() {
    std::env::var("AUTH_SECRET").expect("AUTH_SECRET env var not found");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_noise() {
        let _ = std::fs::remove_file("public_test");
        let _ = std::fs::remove_file("private_test");
        assert!(
            try_generate_crypto_keys("public_test", "private_test").is_ok(),
            "cannot generate auth key files"
        );
        let keys = try_load_crypto_keys("public_test", "private_test");
        assert!(keys.is_ok(), "cannot load keys");
        assert!(snow_test().is_ok(), "snow test failed");
    }

    #[tokio::test]
    async fn test_noise_session_encrypt_decrypt() {
        // Generate two sets of keys for client and server
        let client_keypair = snow::Builder::new(NOISE_PARAMS.parse().unwrap())
            .generate_keypair()
            .unwrap();
        let server_keypair = snow::Builder::new(NOISE_PARAMS.parse().unwrap())
            .generate_keypair()
            .unwrap();

        // Create handshake states
        let client =
            create_noise_initiator(&client_keypair.private, &server_keypair.public).unwrap();
        let server =
            create_noise_responder(&server_keypair.private, &client_keypair.public).unwrap();

        // Perform handshake
        let (client_session, server_session) = perform_noise_handshake(client, server).unwrap();

        // Test data encryption/decryption
        let test_data = b"Hello, encrypted world!";

        // Encrypt with client, decrypt with server
        let encrypted = client_session.encrypt(test_data).await.unwrap();
        let decrypted = server_session.decrypt(&encrypted).await.unwrap();

        assert_eq!(
            test_data,
            decrypted.as_slice(),
            "Failed to decrypt data encrypted by client"
        );

        // Encrypt with server, decrypt with client
        let encrypted = server_session.encrypt(test_data).await.unwrap();
        let decrypted = client_session.decrypt(&encrypted).await.unwrap();

        assert_eq!(
            test_data,
            decrypted.as_slice(),
            "Failed to decrypt data encrypted by server"
        );
    }

    #[test]
    fn test_sign_key() {
        unsafe {
            std::env::set_var("AUTH_SECRET", "secret_key");
        }
        let client_id = "1234".to_string();
        let auth_data = crate::crypto::AuthData { client_id };
        let auth_data = serde_json::json!(auth_data).to_string();
        let signed_key = sign_key(auth_data.as_bytes());

        assert!(!signed_key.is_empty(), "empty signed key");
        assert!(
            verify_signed_key(signed_key).is_ok(),
            "verify signed key failed"
        );
    }

    fn snow_test() -> Result<(), anyhow::Error> {
        let client_keypair = snow::Builder::new(NOISE_PARAMS.parse()?).generate_keypair()?;
        let server_keypair = snow::Builder::new(NOISE_PARAMS.parse()?).generate_keypair()?;

        let mut client = snow::Builder::new(NOISE_PARAMS.parse()?)
            .local_private_key(&client_keypair.private)
            .remote_public_key(&server_keypair.public)
            .build_initiator()?;
        let mut server = snow::Builder::new(NOISE_PARAMS.parse()?)
            .local_private_key(&server_keypair.private)
            .remote_public_key(&client_keypair.public)
            .build_responder()?;

        let (mut read_buf, mut first_msg, mut second_msg) = ([0u8; 1024], [0u8; 1024], [0u8; 1024]);

        let mut buf = [0u8; 1024];

        // -> e
        let auth_key = "auth_key";
        let len = client.write_message(auth_key.as_bytes(), &mut first_msg)?;

        // respond processes the first message...
        server.read_message(&first_msg[..len], &mut read_buf)?;

        // <- e, ee
        let len = server.write_message(&[], &mut second_msg)?;

        // initiator processes the response...
        client.read_message(&second_msg[..len], &mut read_buf)?;

        // NN handshake complete, transition into transport mode.
        let mut initiator = client.into_transport_mode()?;
        let mut responder = server.into_transport_mode()?;

        let len = initiator.write_message(b"test\0", &mut buf).unwrap();

        let mut read_buf = [0u8; 1024];
        responder.read_message(&buf[..len], &mut read_buf).unwrap();
        println!("client said: {}", String::from_utf8_lossy(&read_buf[..len]));

        Ok(())
    }
}
