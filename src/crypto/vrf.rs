//! ECDSA-based VRF primitive (matches Python ECDSAVRFPrimitive)
//!
//! Simplified VRF using ECDSA secp256k1:
//! - Proof = ECDSA signature on input
//! - VRF output = SHA256(proof), used as randomness
//!
//! This is a mocked VRF approach, not a real production VRF.

use crate::error::{ChaincraftError, CryptoError, Result};
use k256::ecdsa::{signature::Signer, SigningKey};
use rand_core::OsRng;
use sha2::{Digest, Sha256};
use std::fmt;

/// ECDSA-based VRF: proof = ECDSA sign, output = hash(proof)
#[derive(Debug, Clone)]
pub struct ECDSAVRF {
    signing_key: SigningKey,
}

impl ECDSAVRF {
    /// Generate new keypair (secp256k1)
    pub fn new() -> Result<Self> {
        let signing_key = SigningKey::random(&mut OsRng);
        Ok(Self { signing_key })
    }

    /// Create from existing signing key bytes
    pub fn from_signing_key_bytes(bytes: &[u8]) -> Result<Self> {
        let signing_key = SigningKey::from_slice(bytes).map_err(|_| {
            ChaincraftError::Crypto(CryptoError::InvalidPrivateKey {
                reason: "Invalid secp256k1 key".to_string(),
            })
        })?;
        Ok(Self { signing_key })
    }

    /// Sign data (VRF input) to produce proof
    pub fn prove(&self, data: &[u8]) -> Result<Vec<u8>> {
        let signature: k256::ecdsa::Signature = self.signing_key.sign(data);
        Ok(signature.to_bytes().to_vec())
    }

    /// Verify proof and return VRF output (hash of proof) if valid
    pub fn verify(&self, data: &[u8], proof: &[u8]) -> Result<Vec<u8>> {
        use k256::ecdsa::{signature::Verifier, VerifyingKey};
        let sig = k256::ecdsa::Signature::from_slice(proof)
            .map_err(|_| ChaincraftError::Crypto(CryptoError::InvalidSignature))?;
        let vk = VerifyingKey::from(&self.signing_key);
        vk.verify(data, &sig)
            .map_err(|_| ChaincraftError::Crypto(CryptoError::VrfVerificationFailed))?;
        Ok(Self::vrf_output(proof))
    }

    /// VRF output = SHA256(proof)
    pub fn vrf_output(proof: &[u8]) -> Vec<u8> {
        Sha256::digest(proof).to_vec()
    }

    /// Public key as bytes (sec1 compressed)
    pub fn public_key_bytes(&self) -> Vec<u8> {
        use k256::elliptic_curve::sec1::ToEncodedPoint;
        self.signing_key
            .verifying_key()
            .to_encoded_point(true)
            .as_bytes()
            .to_vec()
    }

    /// Verify proof using public key bytes
    pub fn verify_with_public_key(
        public_key_bytes: &[u8],
        data: &[u8],
        proof: &[u8],
    ) -> Result<Vec<u8>> {
        use k256::ecdsa::{signature::Verifier, VerifyingKey};
        let pk = k256::PublicKey::from_sec1_bytes(public_key_bytes).map_err(|_| {
            ChaincraftError::Crypto(CryptoError::InvalidPublicKey {
                reason: "Invalid secp256k1 public key".to_string(),
            })
        })?;
        let vk = VerifyingKey::from(pk);
        let sig = k256::ecdsa::Signature::from_slice(proof)
            .map_err(|_| ChaincraftError::Crypto(CryptoError::InvalidSignature))?;
        vk.verify(data, &sig)
            .map_err(|_| ChaincraftError::Crypto(CryptoError::VrfVerificationFailed))?;
        Ok(Self::vrf_output(proof))
    }
}

impl Default for ECDSAVRF {
    fn default() -> Self {
        Self::new().expect("VRF keygen")
    }
}

/// Legacy alias for backward compatibility
#[derive(Debug, Clone, Default)]
pub struct VerifiableRandomFunction(ECDSAVRF);

impl VerifiableRandomFunction {
    pub fn new() -> Result<Self> {
        ECDSAVRF::new().map(Self)
    }
}

impl fmt::Display for VerifiableRandomFunction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "VerifiableRandomFunction(ECDSA)")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vrf_prove_verify() {
        let vrf = ECDSAVRF::new().unwrap();
        let data = b"vrf_input";
        let proof = vrf.prove(data).unwrap();
        assert!(!proof.is_empty());
        let output = vrf.verify(data, &proof).unwrap();
        assert_eq!(output.len(), 32);
        assert_eq!(output, ECDSAVRF::vrf_output(&proof));
    }

    #[test]
    fn test_vrf_verify_with_public_key() {
        let vrf = ECDSAVRF::new().unwrap();
        let pk = vrf.public_key_bytes();
        let data = b"test";
        let proof = vrf.prove(data).unwrap();
        let output = ECDSAVRF::verify_with_public_key(&pk, data, &proof).unwrap();
        assert_eq!(output, ECDSAVRF::vrf_output(&proof));
    }

    #[test]
    fn test_vrf_invalid_proof_fails() {
        let vrf = ECDSAVRF::new().unwrap();
        let data = b"input";
        let bad_proof = vec![0u8; 64];
        assert!(vrf.verify(data, &bad_proof).is_err());
    }
}
