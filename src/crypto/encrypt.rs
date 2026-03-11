//! Symmetric encryption (Fernet), matching Python crypto_primitives/encrypt.py

use crate::error::{ChaincraftError, CryptoError, Result};
use fernet::Fernet;
use std::str;

/// Symmetric encryption using Fernet (Python cryptography.fernet compatible)
#[derive(Clone)]
pub struct SymmetricEncryption {
    fernet: Fernet,
    key: String,
}

impl SymmetricEncryption {
    /// Create with generated key, or use provided key (base64 URL-safe encoded)
    pub fn new(key: Option<&str>) -> Result<Self> {
        let key = match key {
            None => Fernet::generate_key(),
            Some(k) => k.to_string(),
        };
        let fernet = Fernet::new(&key).ok_or_else(|| {
            ChaincraftError::Crypto(CryptoError::EncryptionFailed {
                reason: "Invalid Fernet key".to_string(),
            })
        })?;
        Ok(Self { fernet, key })
    }

    /// Generate a new key and use it
    pub fn generate_key(&mut self) -> Result<String> {
        self.key = Fernet::generate_key();
        self.fernet = Fernet::new(&self.key).ok_or_else(|| {
            ChaincraftError::Crypto(CryptoError::KeyGenerationFailed {
                reason: "Fernet key generation failed".to_string(),
            })
        })?;
        Ok(self.key.clone())
    }

    /// Encrypt data (acts as "sign" in KeyCryptoPrimitive: encrypt-then-verify)
    pub fn sign(&self, data: &[u8]) -> Result<Vec<u8>> {
        let token = self.fernet.encrypt(data);
        Ok(token.into_bytes())
    }

    /// Verify by decrypting and comparing to original data
    pub fn verify(&self, data: &[u8], signature: &[u8]) -> Result<bool> {
        let token = str::from_utf8(signature).map_err(|_| {
            ChaincraftError::Crypto(CryptoError::DecryptionFailed {
                reason: "Invalid UTF-8 token".to_string(),
            })
        })?;
        let decrypted = self.fernet.decrypt(token).map_err(|_| {
            ChaincraftError::Crypto(CryptoError::DecryptionFailed {
                reason: "Decryption failed".to_string(),
            })
        })?;
        Ok(decrypted == data)
    }

    /// Encrypt plaintext string, return base64 token as string
    pub fn encrypt(&self, plaintext: &str) -> Result<String> {
        let token = self.fernet.encrypt(plaintext.as_bytes());
        Ok(token)
    }

    /// Decrypt ciphertext token, return plaintext string
    pub fn decrypt(&self, ciphertext: &str) -> Result<String> {
        let bytes = self.fernet.decrypt(ciphertext).map_err(|_| {
            ChaincraftError::Crypto(CryptoError::DecryptionFailed {
                reason: "Decryption failed".to_string(),
            })
        })?;
        String::from_utf8(bytes).map_err(|e| {
            ChaincraftError::Crypto(CryptoError::DecryptionFailed {
                reason: format!("Invalid UTF-8: {e}"),
            })
        })
    }

    /// Return the key as string (base64)
    pub fn get_key(&self) -> &str {
        &self.key
    }
}

impl Default for SymmetricEncryption {
    fn default() -> Self {
        Self::new(None).expect("Fernet default keygen")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let enc = SymmetricEncryption::new(None).unwrap();
        let msg = "Hello, Chaincraft!";
        let ciphertext = enc.encrypt(msg).unwrap();
        let decrypted = enc.decrypt(&ciphertext).unwrap();
        assert_eq!(decrypted, msg);
    }

    #[test]
    fn test_sign_verify() {
        let enc = SymmetricEncryption::new(None).unwrap();
        let data = b"secret bytes";
        let sig = enc.sign(data).unwrap();
        assert!(enc.verify(data, &sig).unwrap());
        assert!(!enc.verify(b"wrong", &sig).unwrap());
    }

    #[test]
    fn test_with_provided_key() {
        let key = Fernet::generate_key();
        let enc1 = SymmetricEncryption::new(Some(&key)).unwrap();
        let enc2 = SymmetricEncryption::new(Some(&key)).unwrap();
        let ct = enc1.encrypt("test").unwrap();
        let pt = enc2.decrypt(&ct).unwrap();
        assert_eq!(pt, "test");
    }
}
