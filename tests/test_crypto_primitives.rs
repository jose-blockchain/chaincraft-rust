use chaincraft::Result;

#[tokio::test]
async fn test_hashing() -> Result<()> {
    use sha2::{Digest, Sha256};

    let data = b"Hello, Chaincraft!";
    let mut hasher = Sha256::new();
    hasher.update(data);
    let hash = hasher.finalize();

    // Verify hash is 32 bytes (256 bits)
    assert_eq!(hash.len(), 32);

    // Verify consistent hashing
    let mut hasher2 = Sha256::new();
    hasher2.update(data);
    let hash2 = hasher2.finalize();
    assert_eq!(hash, hash2);

    Ok(())
}

#[tokio::test]
async fn test_basic_crypto_placeholder() -> Result<()> {
    // This is a placeholder test for crypto primitives
    // The actual crypto module needs to be implemented properly
    // before we can test the full crypto functionality

    println!("Crypto primitives test placeholder - TODO: implement full crypto tests");
    // This test passes by not throwing exceptions

    Ok(())
}
