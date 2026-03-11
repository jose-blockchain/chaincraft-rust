//! Keypair Generation Example
//!
//! This example demonstrates how to generate and use cryptographic keypairs
//! for different signature schemes supported by ChainCraft.

use chaincraft_rust::{
    crypto::address::Address,
    crypto::{utils, KeyType, PrivateKey, PublicKey},
    Result,
};

fn main() -> Result<()> {
    println!("ChainCraft Keypair Generation Example");
    println!("=====================================\n");

    // Generate Ed25519 keypair
    println!("1. Ed25519 Keypair:");
    let (ed25519_private, ed25519_public) = utils::generate_keypair(KeyType::Ed25519)?;
    println!("   Private Key: {}", ed25519_private.to_hex());
    println!("   Public Key:  {}", ed25519_public.to_hex());

    let ed25519_address = Address::from_public_key(&ed25519_public);
    println!("   Address:     {ed25519_address}\n");

    // Generate secp256k1 keypair
    println!("2. secp256k1 Keypair:");
    let (secp256k1_private, secp256k1_public) = utils::generate_keypair(KeyType::Secp256k1)?;
    println!("   Private Key: {}", secp256k1_private.to_hex());
    println!("   Public Key:  {}", secp256k1_public.to_hex());

    let secp256k1_address = Address::from_public_key(&secp256k1_public);
    println!("   Address:     {secp256k1_address}\n");

    // Demonstrate signing and verification
    println!("3. Signing and Verification:");
    let message = b"Hello, ChainCraft!";
    println!("   Message: {}", String::from_utf8_lossy(message));

    // Sign with Ed25519
    let ed25519_signature = ed25519_private.sign(message)?;
    println!("   Ed25519 Signature: {}", ed25519_signature.to_hex());

    let ed25519_valid = ed25519_public.verify(message, &ed25519_signature)?;
    println!("   Ed25519 Verification: {ed25519_valid}");

    // Sign with secp256k1
    let secp256k1_signature = secp256k1_private.sign(message)?;
    println!("   secp256k1 Signature: {}", secp256k1_signature.to_hex());

    let secp256k1_valid = secp256k1_public.verify(message, &secp256k1_signature)?;
    println!("   secp256k1 Verification: {secp256k1_valid}");

    // Demonstrate key serialization/deserialization
    println!("\n4. Key Serialization:");
    let private_hex = ed25519_private.to_hex();
    let public_hex = ed25519_public.to_hex();

    println!("   Serialized Private Key: {private_hex}");
    println!("   Serialized Public Key:  {public_hex}");

    // Deserialize keys
    let restored_private = PrivateKey::from_hex(&private_hex, KeyType::Ed25519)?;
    let restored_public = PublicKey::from_hex(&public_hex, KeyType::Ed25519)?;

    println!("   Keys restored successfully!");

    // Verify restored keys work
    let test_signature = restored_private.sign(message)?;
    let test_valid = restored_public.verify(message, &test_signature)?;
    println!("   Restored key verification: {test_valid}");

    Ok(())
}
