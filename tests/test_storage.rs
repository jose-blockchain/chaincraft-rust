use chaincraft::{
    storage::{MemoryStorage, Storage},
    Result,
};

#[tokio::test]
async fn test_memory_storage_basic_operations() -> Result<()> {
    let storage = MemoryStorage::new();

    // Initialize storage
    storage.initialize().await?;

    let key = "test_key";
    let value = b"test_value".to_vec();

    // Test that key doesn't exist initially
    assert!(!storage.exists(key).await?);
    assert!(storage.get(key).await?.is_none());

    // Put a value
    storage.put(key, value.clone()).await?;

    // Test that key now exists
    assert!(storage.exists(key).await?);

    // Get the value back
    let retrieved = storage.get(key).await?;
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap(), value);

    // Delete the key
    storage.delete(key).await?;

    // Test that key no longer exists
    assert!(!storage.exists(key).await?);
    assert!(storage.get(key).await?.is_none());

    Ok(())
}

#[tokio::test]
async fn test_memory_storage_multiple_keys() -> Result<()> {
    let storage = MemoryStorage::new();
    storage.initialize().await?;

    let keys_values = vec![
        ("key1", b"value1".to_vec()),
        ("key2", b"value2".to_vec()),
        ("key3", b"value3".to_vec()),
    ];

    // Store multiple key-value pairs
    for (key, value) in &keys_values {
        storage.put(key, value.clone()).await?;
    }

    // Verify all keys exist and have correct values
    for (key, expected_value) in &keys_values {
        assert!(storage.exists(key).await?);
        let retrieved = storage.get(key).await?;
        assert!(retrieved.is_some());
        assert_eq!(&retrieved.unwrap(), expected_value);
    }

    // Clear all data
    storage.clear().await?;

    // Verify all keys are gone
    for (key, _) in &keys_values {
        assert!(!storage.exists(key).await?);
        assert!(storage.get(key).await?.is_none());
    }

    Ok(())
}

#[tokio::test]
async fn test_memory_storage_overwrite() -> Result<()> {
    let storage = MemoryStorage::new();
    storage.initialize().await?;

    let key = "test_key";
    let value1 = b"first_value".to_vec();
    let value2 = b"second_value".to_vec();

    // Store first value
    storage.put(key, value1.clone()).await?;
    let retrieved = storage.get(key).await?;
    assert_eq!(retrieved.unwrap(), value1);

    // Overwrite with second value
    storage.put(key, value2.clone()).await?;
    let retrieved = storage.get(key).await?;
    assert_eq!(retrieved.unwrap(), value2);

    Ok(())
}

#[tokio::test]
async fn test_memory_storage_empty_value() -> Result<()> {
    let storage = MemoryStorage::new();
    storage.initialize().await?;

    let key = "empty_key";
    let empty_value = Vec::new();

    // Store empty value
    storage.put(key, empty_value.clone()).await?;

    // Verify it exists and is retrievable
    assert!(storage.exists(key).await?);
    let retrieved = storage.get(key).await?;
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap(), empty_value);

    Ok(())
}
