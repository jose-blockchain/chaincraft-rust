//! Cryptographic primitives for blockchain operations

pub mod address;
pub mod ecdsa;
pub mod encrypt;
pub mod hash;
pub mod pow;
pub mod vdf;
pub mod vrf;

use crate::error::{ChaincraftError, CryptoError, Result};
use async_trait::async_trait;
use ed25519_dalek::{Signer, Verifier};
use k256::ecdsa::signature::Verifier as K256Verifier;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Trait for keyless cryptographic primitives (hashing, proof of work, etc.)
#[async_trait]
pub trait KeylessCryptoPrimitive: Send + Sync {
    type Input;
    type Output;
    type Challenge;
    type Proof;

    async fn compute(&self, input: Self::Input) -> Result<Self::Output>;
    async fn create_proof(&self, challenge: Self::Challenge) -> Result<Self::Proof>;
    async fn verify_proof(&self, challenge: Self::Challenge, proof: Self::Proof) -> Result<bool>;
}

/// Trait for keyed cryptographic primitives (signatures, VRF, etc.)
#[async_trait]
pub trait KeyedCryptoPrimitive: Send + Sync {
    type PrivateKey;
    type PublicKey;
    type Input;
    type Output;
    type Message;
    type Signature;

    async fn generate_keypair(&self) -> Result<(Self::PrivateKey, Self::PublicKey)>;
    async fn compute(&self, key: &Self::PrivateKey, input: Self::Input) -> Result<Self::Output>;
    async fn verify(
        &self,
        key: &Self::PublicKey,
        input: Self::Input,
        output: &Self::Output,
    ) -> Result<bool>;
    async fn sign(
        &self,
        private_key: &Self::PrivateKey,
        message: &Self::Message,
    ) -> Result<Self::Signature>;
}

/// Public key types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PublicKey {
    Ed25519(ed25519_dalek::VerifyingKey),
    Secp256k1(k256::PublicKey),
}

impl Serialize for PublicKey {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            PublicKey::Ed25519(key) => serializer.serialize_str(&hex::encode(key.as_bytes())),
            PublicKey::Secp256k1(key) => {
                serializer.serialize_str(&hex::encode(key.to_sec1_bytes()))
            },
        }
    }
}

impl<'de> Deserialize<'de> for PublicKey {
    fn deserialize<D>(deserializer: D) -> std::result::Result<PublicKey, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let bytes = hex::decode(&s).map_err(serde::de::Error::custom)?;

        if bytes.len() == 32 {
            // Try Ed25519
            let key = ed25519_dalek::VerifyingKey::from_bytes(&bytes.try_into().unwrap())
                .map_err(serde::de::Error::custom)?;
            Ok(PublicKey::Ed25519(key))
        } else if bytes.len() == 33 {
            // Try Secp256k1
            let key = k256::PublicKey::from_sec1_bytes(&bytes).map_err(serde::de::Error::custom)?;
            Ok(PublicKey::Secp256k1(key))
        } else {
            Err(serde::de::Error::custom("Invalid public key length"))
        }
    }
}

impl PublicKey {
    /// Get the key as bytes
    pub fn as_bytes(&self) -> Vec<u8> {
        match self {
            PublicKey::Ed25519(key) => key.as_bytes().to_vec(),
            PublicKey::Secp256k1(key) => key.to_sec1_bytes().to_vec(),
        }
    }

    /// Convert to hex string
    pub fn to_hex(&self) -> String {
        hex::encode(self.as_bytes())
    }

    /// Create from hex string
    pub fn from_hex(hex_str: &str, key_type: KeyType) -> Result<Self> {
        let bytes = hex::decode(hex_str).map_err(|_| {
            ChaincraftError::Crypto(crate::error::CryptoError::InvalidPublicKey {
                reason: "Invalid hex encoding".to_string(),
            })
        })?;

        match key_type {
            KeyType::Ed25519 => {
                let key_bytes: [u8; 32] = bytes.try_into().map_err(|_| {
                    ChaincraftError::Crypto(crate::error::CryptoError::InvalidPublicKey {
                        reason: "Invalid key length for Ed25519".to_string(),
                    })
                })?;
                let key = ed25519_dalek::VerifyingKey::from_bytes(&key_bytes).map_err(|_| {
                    ChaincraftError::Crypto(crate::error::CryptoError::InvalidPublicKey {
                        reason: "Invalid Ed25519 key".to_string(),
                    })
                })?;
                Ok(PublicKey::Ed25519(key))
            },
            KeyType::Secp256k1 => {
                let key = k256::PublicKey::from_sec1_bytes(&bytes).map_err(|_| {
                    ChaincraftError::Crypto(crate::error::CryptoError::InvalidPublicKey {
                        reason: "Invalid Secp256k1 key".to_string(),
                    })
                })?;
                Ok(PublicKey::Secp256k1(key))
            },
        }
    }

    pub fn algorithm(&self) -> &'static str {
        match self {
            PublicKey::Ed25519(_) => "Ed25519",
            PublicKey::Secp256k1(_) => "Secp256k1",
        }
    }

    pub fn verify(&self, message: &[u8], signature: &Signature) -> Result<bool> {
        match (self, signature) {
            (PublicKey::Ed25519(pk), Signature::Ed25519(sig)) => {
                Ok(pk.verify(message, sig).is_ok())
            },
            (PublicKey::Secp256k1(pk), Signature::Secp256k1(sig)) => {
                let verifying_key = k256::ecdsa::VerifyingKey::from(pk);
                Ok(verifying_key.verify(message, sig).is_ok())
            },
            _ => Err(ChaincraftError::Crypto(CryptoError::InvalidSignature)),
        }
    }
}

/// Private key types
#[derive(Debug, Clone)]
pub enum PrivateKey {
    Ed25519(ed25519_dalek::SigningKey),
    Secp256k1(k256::SecretKey),
}

impl Serialize for PrivateKey {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            PrivateKey::Ed25519(key) => serializer.serialize_str(&hex::encode(key.as_bytes())),
            PrivateKey::Secp256k1(key) => serializer.serialize_str(&hex::encode(key.to_bytes())),
        }
    }
}

impl<'de> Deserialize<'de> for PrivateKey {
    fn deserialize<D>(deserializer: D) -> std::result::Result<PrivateKey, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let bytes = hex::decode(&s).map_err(serde::de::Error::custom)?;

        if bytes.len() == 32 {
            // Try Ed25519 first
            let bytes_array: [u8; 32] = bytes
                .clone()
                .try_into()
                .map_err(|_| serde::de::Error::custom("Invalid byte length conversion"))?;
            let key = ed25519_dalek::SigningKey::from_bytes(&bytes_array);
            Ok(PrivateKey::Ed25519(key))
        } else {
            Err(serde::de::Error::custom("Invalid private key length"))
        }
    }
}

impl PrivateKey {
    /// Get the corresponding public key
    pub fn public_key(&self) -> PublicKey {
        match self {
            PrivateKey::Ed25519(key) => PublicKey::Ed25519(key.verifying_key()),
            PrivateKey::Secp256k1(key) => PublicKey::Secp256k1(key.public_key()),
        }
    }

    /// Convert to hex string (be careful with this!)
    pub fn to_hex(&self) -> String {
        match self {
            PrivateKey::Ed25519(key) => hex::encode(key.as_bytes()),
            PrivateKey::Secp256k1(key) => hex::encode(key.to_bytes()),
        }
    }

    /// Create from hex string
    pub fn from_hex(hex_str: &str, key_type: KeyType) -> Result<Self> {
        let bytes = hex::decode(hex_str).map_err(|_| {
            ChaincraftError::Crypto(CryptoError::InvalidPrivateKey {
                reason: "Invalid hex encoding".to_string(),
            })
        })?;

        match key_type {
            KeyType::Ed25519 => {
                let key_bytes: [u8; 32] = bytes.try_into().map_err(|_| {
                    ChaincraftError::Crypto(CryptoError::InvalidPrivateKey {
                        reason: "Invalid key length for Ed25519".to_string(),
                    })
                })?;
                let key = ed25519_dalek::SigningKey::from_bytes(&key_bytes);
                Ok(PrivateKey::Ed25519(key))
            },
            KeyType::Secp256k1 => {
                let key = k256::SecretKey::from_slice(&bytes).map_err(|_| {
                    ChaincraftError::Crypto(CryptoError::InvalidPrivateKey {
                        reason: "Invalid Secp256k1 key".to_string(),
                    })
                })?;
                Ok(PrivateKey::Secp256k1(key))
            },
        }
    }

    pub fn algorithm(&self) -> &'static str {
        match self {
            PrivateKey::Ed25519(_) => "Ed25519",
            PrivateKey::Secp256k1(_) => "Secp256k1",
        }
    }

    pub fn sign(&self, message: &[u8]) -> Result<Signature> {
        match self {
            PrivateKey::Ed25519(key) => {
                let signature = key.sign(message);
                Ok(Signature::Ed25519(signature))
            },
            PrivateKey::Secp256k1(key) => {
                use k256::ecdsa::SigningKey;
                let signing_key = SigningKey::from(key);
                use k256::ecdsa::signature::Signer;
                let signature = signing_key.sign(message);
                Ok(Signature::Secp256k1(signature))
            },
        }
    }
}

/// Signature types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Signature {
    Ed25519(ed25519_dalek::Signature),
    Secp256k1(k256::ecdsa::Signature),
}

impl Serialize for Signature {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Signature::Ed25519(sig) => serializer.serialize_str(&hex::encode(sig.to_bytes())),
            Signature::Secp256k1(sig) => serializer.serialize_str(&hex::encode(sig.to_bytes())),
        }
    }
}

impl<'de> Deserialize<'de> for Signature {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Signature, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let bytes = hex::decode(&s).map_err(serde::de::Error::custom)?;

        if bytes.len() == 64 {
            // Try Ed25519
            let bytes_array: [u8; 64] = bytes
                .clone()
                .try_into()
                .map_err(|_| serde::de::Error::custom("Invalid byte length conversion"))?;
            let sig = ed25519_dalek::Signature::from_bytes(&bytes_array);
            Ok(Signature::Ed25519(sig))
        } else if bytes.len() == 65 || bytes.len() == 71 {
            // Try Secp256k1
            match k256::ecdsa::Signature::from_der(&bytes)
                .or_else(|_| k256::ecdsa::Signature::from_slice(&bytes))
            {
                Ok(sig) => Ok(Signature::Secp256k1(sig)),
                Err(_) => Err(serde::de::Error::custom("Invalid Secp256k1 signature")),
            }
        } else {
            Err(serde::de::Error::custom("Invalid signature length"))
        }
    }
}

impl Signature {
    /// Convert to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            Signature::Ed25519(sig) => sig.to_bytes().to_vec(),
            Signature::Secp256k1(sig) => sig.to_bytes().to_vec(),
        }
    }

    /// Convert to hex string
    pub fn to_hex(&self) -> String {
        hex::encode(self.to_bytes())
    }

    /// Create from hex string
    pub fn from_hex(hex_str: &str, sig_type: KeyType) -> Result<Self> {
        let bytes = hex::decode(hex_str)
            .map_err(|_| ChaincraftError::Crypto(crate::error::CryptoError::InvalidSignature))?;

        match sig_type {
            KeyType::Ed25519 => {
                let sig_bytes: [u8; 64] = bytes.try_into().map_err(|_| {
                    ChaincraftError::Crypto(crate::error::CryptoError::InvalidSignature)
                })?;
                let sig = ed25519_dalek::Signature::from_bytes(&sig_bytes);
                Ok(Signature::Ed25519(sig))
            },
            KeyType::Secp256k1 => {
                let sig = k256::ecdsa::Signature::from_slice(&bytes).map_err(|_| {
                    ChaincraftError::Crypto(crate::error::CryptoError::InvalidSignature)
                })?;
                Ok(Signature::Secp256k1(sig))
            },
        }
    }

    pub fn algorithm(&self) -> &'static str {
        match self {
            Signature::Ed25519(_) => "Ed25519",
            Signature::Secp256k1(_) => "Secp256k1",
        }
    }
}

/// Key types supported by the system
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KeyType {
    Ed25519,
    Secp256k1,
}

impl KeyType {
    /// Get the string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            KeyType::Ed25519 => "ed25519",
            KeyType::Secp256k1 => "secp256k1",
        }
    }
}

impl std::str::FromStr for KeyType {
    type Err = ();

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "ed25519" => Ok(KeyType::Ed25519),
            "secp256k1" => Ok(KeyType::Secp256k1),
            _ => Err(()),
        }
    }
}

/// Utility functions for cryptographic operations
pub mod utils {
    use super::*;
    use rand::rngs::OsRng;

    /// Generate a new keypair for the specified key type
    pub fn generate_keypair(key_type: KeyType) -> Result<(PrivateKey, PublicKey)> {
        let mut rng = OsRng;

        match key_type {
            KeyType::Ed25519 => {
                let private_key = ed25519_dalek::SigningKey::generate(&mut rng);
                let public_key = private_key.verifying_key();
                Ok((PrivateKey::Ed25519(private_key), PublicKey::Ed25519(public_key)))
            },
            KeyType::Secp256k1 => {
                let private_key = k256::SecretKey::random(&mut rng);
                let public_key = private_key.public_key();
                Ok((PrivateKey::Secp256k1(private_key), PublicKey::Secp256k1(public_key)))
            },
        }
    }

    /// Sign a message with a private key
    pub fn sign_message(private_key: &PrivateKey, message: &[u8]) -> Result<Signature> {
        match private_key {
            PrivateKey::Ed25519(key) => {
                let signature = key.sign(message);
                Ok(Signature::Ed25519(signature))
            },
            PrivateKey::Secp256k1(key) => {
                use k256::ecdsa::{signature::Signer, SigningKey};
                let signing_key = SigningKey::from(key);
                let signature = signing_key.sign(message);
                Ok(Signature::Secp256k1(signature))
            },
        }
    }

    /// Verify a signature with a public key
    pub fn verify_signature(
        public_key: &PublicKey,
        message: &[u8],
        signature: &Signature,
    ) -> Result<bool> {
        match (public_key, signature) {
            (PublicKey::Ed25519(pk), Signature::Ed25519(sig)) => {
                use ed25519_dalek::Verifier;
                Ok(pk.verify(message, sig).is_ok())
            },
            (PublicKey::Secp256k1(pk), Signature::Secp256k1(sig)) => {
                use k256::ecdsa::{signature::Verifier, VerifyingKey};
                let verifying_key = VerifyingKey::from(pk);
                Ok(verifying_key.verify(message, sig).is_ok())
            },
            _ => Err(ChaincraftError::Crypto(CryptoError::InvalidSignature)),
        }
    }
}

// Utility functions for ed25519 signatures
pub mod ed25519_utils {
    use crate::error::{ChaincraftError, CryptoError, Result};
    use ed25519_dalek::Signature as Ed25519Signature;

    // Workaround for signature creation
    pub fn create_signature(bytes: &[u8; 64]) -> Result<Ed25519Signature> {
        // In ed25519_dalek 2.0, from_bytes returns a Signature directly, not a Result
        Ok(Ed25519Signature::from_bytes(bytes))
    }
}

// Re-export commonly used types
pub use address::Address;
pub use ecdsa::EcdsaSignature;
pub use encrypt::SymmetricEncryption;
pub use hash::*;
pub use pow::ProofOfWork;
pub use vdf::VerifiableDelayFunction;
pub use vrf::{VerifiableRandomFunction, ECDSAVRF};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::{ChaincraftError, CryptoError, Result};

    #[test]
    fn test_key_generation() -> Result<()> {
        // ... existing code ...
        Ok(())
    }
}
