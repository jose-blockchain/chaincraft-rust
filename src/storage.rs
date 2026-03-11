//! Storage implementation for chain data

use crate::error::{Result, StorageError};
use async_trait::async_trait;
use std::collections::HashMap;
use std::path::Path;

#[cfg(feature = "persistent")]
use sled;

/// Trait for key-value storage backends
#[async_trait]
pub trait Storage: Send + Sync {
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>>;
    async fn put(&self, key: &str, value: Vec<u8>) -> Result<()>;
    async fn delete(&self, key: &str) -> Result<()>;
    async fn exists(&self, key: &str) -> Result<bool>;
    async fn clear(&self) -> Result<()>;
    async fn initialize(&self) -> Result<()>;
    /// Return the number of stored keys (best-effort for non-in-memory backends).
    async fn len(&self) -> Result<usize>;

    /// Return true if storage has no keys.
    async fn is_empty(&self) -> Result<bool> {
        self.len().await.map(|n| n == 0)
    }
}

/// In-memory storage implementation
#[derive(Debug, Default)]
pub struct MemoryStorage {
    data: tokio::sync::RwLock<HashMap<String, Vec<u8>>>,
}

impl MemoryStorage {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl Storage for MemoryStorage {
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>> {
        let data = self.data.read().await;
        Ok(data.get(key).cloned())
    }

    async fn put(&self, key: &str, value: Vec<u8>) -> Result<()> {
        let mut data = self.data.write().await;
        data.insert(key.to_string(), value);
        Ok(())
    }

    async fn delete(&self, key: &str) -> Result<()> {
        let mut data = self.data.write().await;
        data.remove(key);
        Ok(())
    }

    async fn exists(&self, key: &str) -> Result<bool> {
        let data = self.data.read().await;
        Ok(data.contains_key(key))
    }

    async fn clear(&self) -> Result<()> {
        let mut data = self.data.write().await;
        data.clear();
        Ok(())
    }

    async fn initialize(&self) -> Result<()> {
        // In-memory storage doesn't need initialization
        Ok(())
    }

    async fn len(&self) -> Result<usize> {
        let data = self.data.read().await;
        Ok(data.len())
    }
}

/// On-disk storage implementation using `sled`.
///
/// This roughly corresponds to the Python version's dbm-based persistent storage.
#[cfg(feature = "persistent")]
#[derive(Debug)]
pub struct SledStorage {
    db: sled::Db,
}

#[cfg(feature = "persistent")]
impl SledStorage {
    /// Open or create a sled database at the given path.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let db = sled::open(path).map_err(|e| StorageError::DatabaseOperation {
            reason: e.to_string(),
        })?;
        Ok(Self { db })
    }
}

#[cfg(feature = "persistent")]
#[async_trait]
impl Storage for SledStorage {
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>> {
        let res = self
            .db
            .get(key.as_bytes())
            .map_err(|e| StorageError::DatabaseOperation {
                reason: e.to_string(),
            })?;
        Ok(res.map(|ivec| ivec.to_vec()))
    }

    async fn put(&self, key: &str, value: Vec<u8>) -> Result<()> {
        self.db
            .insert(key.as_bytes(), value)
            .map_err(|e| StorageError::DatabaseOperation {
                reason: e.to_string(),
            })?;
        self.db
            .flush()
            .map_err(|e| StorageError::DatabaseOperation {
                reason: e.to_string(),
            })?;
        Ok(())
    }

    async fn delete(&self, key: &str) -> Result<()> {
        self.db
            .remove(key.as_bytes())
            .map_err(|e| StorageError::DatabaseOperation {
                reason: e.to_string(),
            })?;
        self.db
            .flush()
            .map_err(|e| StorageError::DatabaseOperation {
                reason: e.to_string(),
            })?;
        Ok(())
    }

    async fn exists(&self, key: &str) -> Result<bool> {
        let res =
            self.db
                .contains_key(key.as_bytes())
                .map_err(|e| StorageError::DatabaseOperation {
                    reason: e.to_string(),
                })?;
        Ok(res)
    }

    async fn clear(&self) -> Result<()> {
        self.db
            .clear()
            .map_err(|e| StorageError::DatabaseOperation {
                reason: e.to_string(),
            })?;
        self.db
            .flush()
            .map_err(|e| StorageError::DatabaseOperation {
                reason: e.to_string(),
            })?;
        Ok(())
    }

    async fn initialize(&self) -> Result<()> {
        // sled opens lazily; nothing special required
        Ok(())
    }

    async fn len(&self) -> Result<usize> {
        Ok(self.db.len())
    }
}
