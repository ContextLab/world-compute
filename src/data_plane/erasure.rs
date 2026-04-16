//! RS(10,18) erasure coding per FR-071 (T068).
//!
//! k=10 data shards, n=18 total (8 parity). Storage overhead: 1.80x.
//! Survives any 8 simultaneous shard losses.

use crate::error::{ErrorCode, WcError};
use reed_solomon_erasure::galois_8::ReedSolomon;

/// Default erasure coding parameters per research/04-storage.md.
pub const DATA_SHARDS: usize = 10;
pub const PARITY_SHARDS: usize = 8;
pub const TOTAL_SHARDS: usize = DATA_SHARDS + PARITY_SHARDS;

/// Encode data into RS(10,18) shards.
/// Input data is split into 10 equal-sized data shards, then 8 parity shards
/// are computed. Returns all 18 shards.
pub fn encode(data: &[u8]) -> Result<Vec<Vec<u8>>, WcError> {
    let rs = ReedSolomon::new(DATA_SHARDS, PARITY_SHARDS)
        .map_err(|e| WcError::new(ErrorCode::Internal, format!("RS init: {e}")))?;

    // Pad data to be divisible by DATA_SHARDS
    let shard_size = data.len().div_ceil(DATA_SHARDS);
    let mut padded = data.to_vec();
    padded.resize(shard_size * DATA_SHARDS, 0);

    // Split into data shards
    let mut shards: Vec<Vec<u8>> = padded.chunks(shard_size).map(|c| c.to_vec()).collect();

    // Add empty parity shards
    for _ in 0..PARITY_SHARDS {
        shards.push(vec![0u8; shard_size]);
    }

    // Compute parity
    rs.encode(&mut shards)
        .map_err(|e| WcError::new(ErrorCode::Internal, format!("RS encode: {e}")))?;

    Ok(shards)
}

/// Reconstruct original data from at least 10 of 18 shards.
/// Missing shards should be passed as None.
pub fn reconstruct(shards: &mut [Option<Vec<u8>>]) -> Result<Vec<u8>, WcError> {
    if shards.len() != TOTAL_SHARDS {
        return Err(WcError::new(
            ErrorCode::Internal,
            format!("Expected {TOTAL_SHARDS} shard slots, got {}", shards.len()),
        ));
    }

    let present = shards.iter().filter(|s| s.is_some()).count();
    if present < DATA_SHARDS {
        return Err(WcError::new(
            ErrorCode::Internal,
            format!("Only {present} shards available, need at least {DATA_SHARDS}"),
        ));
    }

    let rs = ReedSolomon::new(DATA_SHARDS, PARITY_SHARDS)
        .map_err(|e| WcError::new(ErrorCode::Internal, format!("RS init: {e}")))?;

    rs.reconstruct(shards)
        .map_err(|e| WcError::new(ErrorCode::Internal, format!("RS reconstruct: {e}")))?;

    // Reassemble data from the first 10 (data) shards
    let mut data = Vec::new();
    for s in shards.iter().take(DATA_SHARDS).flatten() {
        data.extend_from_slice(s);
    }

    Ok(data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_produces_correct_shard_count() {
        let data = b"hello world compute erasure test data that is long enough";
        let shards = encode(data).unwrap();
        assert_eq!(shards.len(), TOTAL_SHARDS);
    }

    #[test]
    fn full_round_trip_no_loss() {
        let data = b"hello world compute erasure coding round trip test";
        let shards = encode(data).unwrap();
        let mut shard_opts: Vec<Option<Vec<u8>>> = shards.into_iter().map(Some).collect();
        let recovered = reconstruct(&mut shard_opts).unwrap();
        // Recovered may have padding; check prefix matches
        assert!(recovered.starts_with(data));
    }

    #[test]
    fn survives_8_shard_loss() {
        let data = b"critical data that must survive 8 shard losses per RS(10,18)";
        let shards = encode(data).unwrap();
        let mut shard_opts: Vec<Option<Vec<u8>>> = shards.into_iter().map(Some).collect();

        // Remove 8 shards (the maximum we can lose)
        for shard in shard_opts.iter_mut().take(8) {
            *shard = None;
        }

        let recovered = reconstruct(&mut shard_opts).unwrap();
        assert!(recovered.starts_with(data));
    }

    #[test]
    fn fails_with_9_shard_loss() {
        let data = b"data that cannot survive 9 losses";
        let shards = encode(data).unwrap();
        let mut shard_opts: Vec<Option<Vec<u8>>> = shards.into_iter().map(Some).collect();

        // Remove 9 shards — one more than RS(10,18) can handle
        for shard in shard_opts.iter_mut().take(9) {
            *shard = None;
        }

        assert!(reconstruct(&mut shard_opts).is_err());
    }

    #[test]
    fn storage_overhead_is_correct() {
        let data = vec![42u8; 10000];
        let shards = encode(&data).unwrap();
        let total_shard_bytes: usize = shards.iter().map(|s| s.len()).sum();
        let overhead = total_shard_bytes as f64 / data.len() as f64;
        // RS(10,18) overhead should be 1.8x
        assert!((overhead - 1.8).abs() < 0.01, "Overhead {overhead} should be ~1.8x");
    }
}
