//! Deterministic content hashing for project-bundle artifacts.
//!
//! Mirrors the FNV-1a fingerprint used by `core-snapshot` / `core-scene` so a
//! durable artifact's hash is stable across runs and platforms and is legible in
//! manifests/diagnostics. The hash is rendered as fixed 16-digit lowercase hex.

const FNV_OFFSET: u64 = 14_695_981_039_346_656_037;
const FNV_PRIME: u64 = 1_099_511_628_211;

/// A 64-bit content hash of an artifact's bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BundleHash(pub u64);

impl BundleHash {
    /// FNV-1a over `bytes`.
    pub fn of(bytes: &[u8]) -> Self {
        let mut h = FNV_OFFSET;
        for &b in bytes {
            h ^= b as u64;
            h = h.wrapping_mul(FNV_PRIME);
        }
        BundleHash(h)
    }

    /// FNV-1a over a string's UTF-8 bytes (the common case — artifacts are text).
    pub fn of_str(s: &str) -> Self {
        Self::of(s.as_bytes())
    }

    /// Fixed-width lowercase hex (16 digits), the manifest's on-disk form.
    pub fn to_hex(self) -> String {
        format!("{:016x}", self.0)
    }

    /// Parse the 16-digit lowercase-hex form produced by [`BundleHash::to_hex`].
    pub fn parse_hex(s: &str) -> Option<Self> {
        if s.len() != 16 || !s.bytes().all(|b| b.is_ascii_hexdigit()) {
            return None;
        }
        u64::from_str_radix(s, 16).ok().map(BundleHash)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_is_deterministic_and_hex_round_trips() {
        let a = BundleHash::of_str("voxelchunk 1\n");
        let b = BundleHash::of_str("voxelchunk 1\n");
        assert_eq!(a, b);
        assert_eq!(a.to_hex().len(), 16);
        assert_eq!(BundleHash::parse_hex(&a.to_hex()), Some(a));
    }

    #[test]
    fn different_bytes_hash_differently() {
        assert_ne!(BundleHash::of_str("a"), BundleHash::of_str("b"));
    }

    #[test]
    fn parse_hex_rejects_malformed() {
        assert_eq!(BundleHash::parse_hex("xyz"), None);
        assert_eq!(BundleHash::parse_hex("0123456789abcdeg"), None); // 'g'
        assert_eq!(BundleHash::parse_hex("0123456789abcde"), None); // 15 digits
    }
}
