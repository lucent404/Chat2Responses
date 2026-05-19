use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use anyhow::{anyhow, Result};
use argon2::{
    password_hash::{
        rand_core::{OsRng, RngCore},
        PasswordHash, PasswordHasher, PasswordVerifier, SaltString,
    },
    Argon2,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};

type HmacSha256 = Hmac<Sha256>;

#[derive(Clone)]
pub struct Crypto {
    cipher: Aes256Gcm,
    hmac_key: [u8; 32],
}

impl Crypto {
    pub fn new(secret: &str) -> Self {
        let digest = Sha256::digest(secret.as_bytes());
        let mut key = [0_u8; 32];
        key.copy_from_slice(&digest);
        Self {
            cipher: Aes256Gcm::new_from_slice(&key).expect("valid AES-256 key"),
            hmac_key: key,
        }
    }

    pub fn encrypt(&self, plaintext: &str) -> Result<String> {
        let mut nonce_bytes = [0_u8; 12];
        let mut rng = OsRng;
        rng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);
        let ciphertext = self
            .cipher
            .encrypt(nonce, plaintext.as_bytes())
            .map_err(|_| anyhow!("failed to encrypt secret"))?;
        Ok(format!(
            "{}.{}",
            URL_SAFE_NO_PAD.encode(nonce_bytes),
            URL_SAFE_NO_PAD.encode(ciphertext)
        ))
    }

    pub fn decrypt(&self, value: &str) -> Result<String> {
        let (nonce, ciphertext) = value
            .split_once('.')
            .ok_or_else(|| anyhow!("invalid encrypted secret format"))?;
        let nonce = URL_SAFE_NO_PAD.decode(nonce)?;
        let ciphertext = URL_SAFE_NO_PAD.decode(ciphertext)?;
        let plaintext = self
            .cipher
            .decrypt(Nonce::from_slice(&nonce), ciphertext.as_ref())
            .map_err(|_| anyhow!("failed to decrypt secret"))?;
        String::from_utf8(plaintext).map_err(Into::into)
    }

    pub fn hash_api_key(&self, key: &str) -> String {
        let mut mac =
            <HmacSha256 as Mac>::new_from_slice(&self.hmac_key).expect("HMAC accepts any key");
        mac.update(key.as_bytes());
        hex::encode(mac.finalize().into_bytes())
    }
}

pub fn generate_api_key() -> String {
    let mut bytes = [0_u8; 32];
    let mut rng = OsRng;
    rng.fill_bytes(&mut bytes);
    format!("cr_{}", URL_SAFE_NO_PAD.encode(bytes))
}

pub fn generate_session_token() -> String {
    let mut bytes = [0_u8; 32];
    let mut rng = OsRng;
    rng.fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

pub fn hash_password(password: &str) -> Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    Ok(Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| anyhow!("failed to hash password: {e}"))?
        .to_string())
}

pub fn verify_password(password: &str, hash: &str) -> bool {
    let Ok(parsed) = PasswordHash::new(hash) else {
        return false;
    };
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encryption_round_trips() {
        let crypto = Crypto::new("test-secret");
        let encrypted = crypto.encrypt("sk-provider").unwrap();
        assert_ne!(encrypted, "sk-provider");
        assert_eq!(crypto.decrypt(&encrypted).unwrap(), "sk-provider");
    }

    #[test]
    fn api_key_hash_is_stable_and_secret_dependent() {
        let a = Crypto::new("one");
        let b = Crypto::new("two");
        assert_eq!(a.hash_api_key("cr_test"), a.hash_api_key("cr_test"));
        assert_ne!(a.hash_api_key("cr_test"), b.hash_api_key("cr_test"));
    }

    #[test]
    fn password_hash_verifies() {
        let hash = hash_password("correct horse").unwrap();
        assert!(verify_password("correct horse", &hash));
        assert!(!verify_password("wrong", &hash));
    }
}
