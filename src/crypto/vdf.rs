//! Verifiable Delay Function implementation
//!
//! Uses the `vdf` crate (Pietrzak/Wesolowski) when feature `vdf-crypto` is enabled.
//! Requires GMP: `apt-get install libgmp-dev` or `dnf install gmp-devel`.

use crate::error::{ChaincraftError, Result};

/// Verifiable Delay Function wrapper
#[derive(Debug, Clone)]
pub struct VerifiableDelayFunction {
    num_bits: u16,
}

impl VerifiableDelayFunction {
    pub fn new() -> Self {
        Self::with_bits(2048)
    }

    pub fn with_bits(num_bits: u16) -> Self {
        Self { num_bits }
    }

    /// Solve the VDF: compute output for challenge after `iterations` steps
    pub fn solve(&self, challenge: &[u8], iterations: u64) -> Result<Vec<u8>> {
        #[cfg(feature = "vdf-crypto")]
        {
            use vdf::{PietrzakVDFParams, VDFParams, VDF};
            let vdf = PietrzakVDFParams(self.num_bits).new();
            vdf.solve(challenge, iterations).map_err(|e| {
                ChaincraftError::Crypto(crate::error::CryptoError::VdfError {
                    reason: format!("{e:?}"),
                })
            })
        }
        #[cfg(not(feature = "vdf-crypto"))]
        {
            let _ = (challenge, iterations);
            Err(ChaincraftError::Crypto(crate::error::CryptoError::VdfError {
                reason: "VDF requires feature 'vdf-crypto'. Enable with: chaincraft = { version = \"..\", features = [\"vdf-crypto\"] }".to_string(),
            }))
        }
    }

    /// Verify a VDF solution
    pub fn verify(&self, challenge: &[u8], iterations: u64, solution: &[u8]) -> Result<bool> {
        #[cfg(feature = "vdf-crypto")]
        {
            use vdf::{PietrzakVDFParams, VDFParams, VDF};
            let vdf = PietrzakVDFParams(self.num_bits).new();
            vdf.verify(challenge, iterations, solution)
                .map(|_| true)
                .map_err(|e| {
                    ChaincraftError::Crypto(crate::error::CryptoError::VdfError {
                        reason: format!("{e:?}"),
                    })
                })
        }
        #[cfg(not(feature = "vdf-crypto"))]
        {
            let _ = (challenge, iterations, solution);
            Err(ChaincraftError::Crypto(crate::error::CryptoError::VdfError {
                reason: "VDF requires feature 'vdf-crypto'".to_string(),
            }))
        }
    }
}

impl Default for VerifiableDelayFunction {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(not(feature = "vdf-crypto"))]
    #[test]
    fn test_vdf_without_feature_returns_error() {
        let vdf = VerifiableDelayFunction::new();
        let err = vdf.solve(b"challenge", 10);
        assert!(err.is_err());
        assert!(err.unwrap_err().to_string().contains("vdf-crypto"));
    }

    #[cfg(feature = "vdf-crypto")]
    #[test]
    fn test_vdf_solve_and_verify() {
        let vdf = VerifiableDelayFunction::with_bits(1024);
        let challenge = b"test";
        let iterations = 66u64; // Pietrzak requires at least 66 iterations
        let solution = vdf.solve(challenge, iterations).expect("solve");
        assert!(!solution.is_empty());
        assert!(vdf.verify(challenge, iterations, &solution).unwrap());
    }
}
