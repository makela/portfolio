//! Content hash for cache-busting asset URLs. Shared by `build.rs` (release
//! table) and `assets.rs` (live hashing in debug builds), so both always
//! produce the same `?v=` value for the same file.

/// FNV-1a, formatted as 16 hex digits: tiny, dependency-free and stable across
/// Rust versions, which is all a cache-busting key needs (not for security).
pub(crate) fn content_hash(bytes: &[u8]) -> String {
    let hash = bytes.iter().fold(0xcbf2_9ce4_8422_2325_u64, |hash, &b| {
        (hash ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3)
    });
    format!("{hash:016x}")
}
