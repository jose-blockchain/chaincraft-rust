//! ECDSA signature implementation

use crate::crypto::{KeyType, KeyedCryptoPrimitive, PrivateKey, PublicKey, Signature};
use crate::error::{ChaincraftError, CryptoError, Result};
use async_trait::async_trait;
use base64::{engine::general_purpose, Engine as _};
use serde::{Deserialize, Serialize};

/// ECDSA signature provider
#[derive(Debug, Clone)]
pub struct EcdsaSignature {
    pub key_type: KeyType,
}

impl EcdsaSignature {
    pub fn new(key_type: KeyType) -> Self {
        Self { key_type }
    }

    pub fn ed25519() -> Self {
        Self::new(KeyType::Ed25519)
    }

    pub fn secp256k1() -> Self {
        Self::new(KeyType::Secp256k1)
    }
}

impl Default for EcdsaSignature {
    fn default() -> Self {
        Self::ed25519()
    }
}

#[async_trait]
impl KeyedCryptoPrimitive for EcdsaSignature {
    type PublicKey = PublicKey;
    type PrivateKey = PrivateKey;
    type Input = Vec<u8>;
    type Output = Vec<u8>;
    type Message = Vec<u8>;
    type Signature = Signature;

    async fn generate_keypair(&self) -> Result<(Self::PrivateKey, Self::PublicKey)> {
        crate::crypto::utils::generate_keypair(self.key_type)
    }

    async fn compute(&self, key: &Self::PrivateKey, input: Self::Input) -> Result<Self::Output> {
        // For ECDSA, compute is the same as signing
        let signature = self.sign(key, &input).await?;
        Ok(signature.to_bytes())
    }

    async fn sign(
        &self,
        private_key: &Self::PrivateKey,
        message: &Self::Message,
    ) -> Result<Self::Signature> {
        crate::crypto::utils::sign_message(private_key, message)
    }

    async fn verify(
        &self,
        key: &Self::PublicKey,
        input: Self::Input,
        output: &Self::Output,
    ) -> Result<bool> {
        // For ed25519-dalek v1.0.1, the API is different
        match key {
            PublicKey::Ed25519(pk) => {
                // For Ed25519, we need a 64-byte signature
                if output.len() != 64 {
                    return Err(ChaincraftError::Crypto(CryptoError::InvalidSignature));
                }

                // With v1.0.1, the signature requires a direct array conversion
                let mut sig_bytes = [0u8; 64];
                sig_bytes.copy_from_slice(&output[0..64]);

                // Direct creation (v1.0.1 has a different API)
                let signature = ed25519_dalek::Signature::from(sig_bytes);

                // Verify the signature
                use ed25519_dalek::Verifier;
                match pk.verify(&input, &signature) {
                    Ok(_) => Ok(true),
                    Err(_) => Ok(false), // This is a verification failure, not an error
                }
            },
            PublicKey::Secp256k1(pk) => {
                // For Secp256k1, parse the signature and verify
                let signature = match k256::ecdsa::Signature::from_slice(output.as_slice()) {
                    Ok(s) => s,
                    Err(_) => return Err(ChaincraftError::Crypto(CryptoError::InvalidSignature)),
                };

                use k256::ecdsa::{signature::Verifier, VerifyingKey};
                let verifying_key = VerifyingKey::from(pk);
                Ok(verifying_key.verify(&input, &signature).is_ok())
            },
        }
    }
}

/// High-level ECDSA signature wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ECDSASignature {
    data: Vec<u8>,
}

impl ECDSASignature {
    pub fn new(data: Vec<u8>) -> Self {
        Self { data }
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        Ok(Self {
            data: bytes.to_vec(),
        })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        self.data.clone()
    }
}

/// High-level ECDSA signer
#[derive(Debug)]
pub struct ECDSASigner {
    private_key: PrivateKey,
    public_key: PublicKey,
    provider: EcdsaSignature,
}

impl ECDSASigner {
    pub fn new() -> Result<Self> {
        // Use async context if available, otherwise create a simple fallback
        let provider = EcdsaSignature::ed25519();
        let (private_key, public_key) = futures::executor::block_on(provider.generate_keypair())?;

        Ok(Self {
            private_key,
            public_key,
            provider,
        })
    }

    pub fn sign(&self, message: &[u8]) -> Result<ECDSASignature> {
        let signature =
            futures::executor::block_on(self.provider.sign(&self.private_key, &message.to_vec()))?;
        Ok(ECDSASignature::new(signature.to_bytes()))
    }

    pub fn get_public_key_pem(&self) -> Result<String> {
        match &self.public_key {
            PublicKey::Ed25519(pk) => {
                // Convert to PEM-like format
                let bytes = pk.to_bytes();
                let b64 = general_purpose::STANDARD.encode(bytes);
                Ok(format!("-----BEGIN PUBLIC KEY-----\n{b64}\n-----END PUBLIC KEY-----"))
            },
            PublicKey::Secp256k1(pk) => {
                // For secp256k1, use the compressed format
                use k256::elliptic_curve::sec1::ToEncodedPoint;
                let point = pk.to_encoded_point(true);
                let b64 = general_purpose::STANDARD.encode(point.as_bytes());
                Ok(format!("-----BEGIN PUBLIC KEY-----\n{b64}\n-----END PUBLIC KEY-----"))
            },
        }
    }
}

/// High-level ECDSA verifier
#[derive(Debug, Clone)]
pub struct ECDSAVerifier {
    provider: EcdsaSignature,
}

impl ECDSAVerifier {
    pub fn new() -> Self {
        Self {
            provider: EcdsaSignature::ed25519(),
        }
    }

    pub fn verify(
        &self,
        message: &[u8],
        signature: &ECDSASignature,
        public_key_pem: &str,
    ) -> Result<bool> {
        // Parse PEM format
        let public_key = self.parse_public_key_pem(public_key_pem)?;

        futures::executor::block_on(self.provider.verify(
            &public_key,
            message.to_vec(),
            &signature.data,
        ))
    }

    fn parse_public_key_pem(&self, pem: &str) -> Result<PublicKey> {
        // Remove PEM headers and whitespace
        let cleaned_pem = pem
            .replace("-----BEGIN PUBLIC KEY-----", "")
            .replace("-----END PUBLIC KEY-----", "")
            .replace(['\n', '\r', ' '], "");

        // Decode base64
        let key_bytes = general_purpose::STANDARD
            .decode(cleaned_pem)
            .map_err(|_| ChaincraftError::Crypto(CryptoError::InvalidSignature))?;

        // Try to parse as Ed25519 first (32 bytes)
        if key_bytes.len() == 32 {
            let mut array = [0u8; 32];
            array.copy_from_slice(&key_bytes);

            match ed25519_dalek::VerifyingKey::from_bytes(&array) {
                Ok(pk) => Ok(PublicKey::Ed25519(pk)),
                Err(_) => Err(ChaincraftError::Crypto(CryptoError::InvalidSignature)),
            }
        } else if key_bytes.len() == 33 {
            // Try secp256k1 compressed format
            match k256::PublicKey::from_sec1_bytes(&key_bytes) {
                Ok(pk) => Ok(PublicKey::Secp256k1(pk)),
                Err(_) => Err(ChaincraftError::Crypto(CryptoError::InvalidSignature)),
            }
        } else {
            Err(ChaincraftError::Crypto(CryptoError::InvalidSignature))
        }
    }
}

impl Default for ECDSAVerifier {
    fn default() -> Self {
        Self::new()
    }
}
