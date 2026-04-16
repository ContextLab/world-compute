//! CIDv1 content-addressed object store per FR-070 (T026).
//!
//! Provides put/get/has/delete with SHA-256 hashing.

use cid::Cid;
use multihash::Multihash;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Multihash code for SHA2-256.
const SHA2_256: u64 = 0x12;
/// CID codec for raw binary data.
const RAW_CODEC: u64 = 0x55;

/// In-memory CID-addressed object store.
/// Production will use a disk-backed store with LRU eviction.
#[derive(Debug, Clone)]
pub struct CidStore {
    objects: Arc<RwLock<HashMap<Cid, Vec<u8>>>>,
}

impl CidStore {
    pub fn new() -> Self {
        Self { objects: Arc::new(RwLock::new(HashMap::new())) }
    }

    /// Store data and return its CID.
    pub fn put(&self, data: &[u8]) -> Result<Cid, crate::error::WcError> {
        let cid = compute_cid(data)?;
        self.objects.write().unwrap().insert(cid, data.to_vec());
        Ok(cid)
    }

    /// Retrieve data by CID.
    pub fn get(&self, cid: &Cid) -> Option<Vec<u8>> {
        self.objects.read().unwrap().get(cid).cloned()
    }

    /// Check if a CID exists in the store.
    pub fn has(&self, cid: &Cid) -> bool {
        self.objects.read().unwrap().contains_key(cid)
    }

    /// Delete an object by CID.
    pub fn delete(&self, cid: &Cid) -> bool {
        self.objects.write().unwrap().remove(cid).is_some()
    }

    /// Number of objects in the store.
    pub fn len(&self) -> usize {
        self.objects.read().unwrap().len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Default for CidStore {
    fn default() -> Self {
        Self::new()
    }
}

/// Compute a CIDv1 (raw codec, SHA2-256) for the given data.
pub fn compute_cid(data: &[u8]) -> Result<Cid, crate::error::WcError> {
    let hash = Sha256::digest(data);
    let mh = Multihash::<64>::wrap(SHA2_256, &hash)
        .map_err(|e| crate::error::WcError::Serialization(e.to_string()))?;
    Ok(Cid::new_v1(RAW_CODEC, mh))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn put_get_round_trip() {
        let store = CidStore::new();
        let data = b"hello world compute";
        let cid = store.put(data).unwrap();
        assert!(store.has(&cid));
        assert_eq!(store.get(&cid).unwrap(), data);
    }

    #[test]
    fn same_data_produces_same_cid() {
        let cid1 = compute_cid(b"test data").unwrap();
        let cid2 = compute_cid(b"test data").unwrap();
        assert_eq!(cid1, cid2);
    }

    #[test]
    fn different_data_produces_different_cid() {
        let cid1 = compute_cid(b"data A").unwrap();
        let cid2 = compute_cid(b"data B").unwrap();
        assert_ne!(cid1, cid2);
    }

    #[test]
    fn delete_removes_object() {
        let store = CidStore::new();
        let cid = store.put(b"ephemeral").unwrap();
        assert!(store.delete(&cid));
        assert!(!store.has(&cid));
    }
}
