//! Proof of Work Example
//!
//! Demonstrates the Proof of Work cryptographic primitive used in blockchain consensus.
//! Ported from concepts in the Python chaincraft examples (blockchain.py, randomness_beacon.py).
//!
//! Features:
//! - Mining with configurable difficulty
//! - Hash verification
//! - Async parallel mining
//!
//! Run with: `cargo run --example proof_of_work_example`

use chaincraft_rust::crypto::{pow::PoWChallenge, KeylessCryptoPrimitive, ProofOfWork};
use chaincraft_rust::error::Result;
use std::time::Instant;

#[tokio::main]
async fn main() -> Result<()> {
    println!("Chaincraft Proof of Work Example");
    println!("=================================\n");

    let challenge = PoWChallenge::new("blockchain_challenge_001");
    let pow = ProofOfWork::with_difficulty(2);

    println!("Mining with difficulty 2 (2 leading zeros)...");
    let start = Instant::now();
    let proof = pow.create_proof(challenge.clone()).await?;
    let elapsed = start.elapsed();

    println!("  Nonce: {}", proof.nonce);
    println!("  Hash:  {}...", &proof.hash[..proof.hash.len().min(16)]);
    println!("  Time:  {elapsed:?}\n");

    let verified = pow.verify_proof(challenge, proof).await?;
    println!("Verification: {}", if verified { "PASSED" } else { "FAILED" });

    Ok(())
}
