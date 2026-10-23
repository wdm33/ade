// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

use ::sha3::{Digest, Sha3_256};
use ade_types::Hash32;

/// SHA3-256 (FIPS 202) of arbitrary data. Pure and infallible.
///
/// Cardano uses it in one place Ade needs: the key hash of a bootstrap witness,
/// `Blake2b-224(SHA3-256(prefix ‖ vkey ‖ chain_code ‖ attributes))`.
pub fn sha3_256(data: &[u8]) -> Hash32 {
    let mut out = [0u8; 32];
    out.copy_from_slice(&Sha3_256::digest(data));
    Hash32(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(h: &Hash32) -> String {
        h.0.iter().map(|b| format!("{b:02x}")).collect()
    }

    #[test]
    fn fips_202_vectors() {
        assert_eq!(
            hex(&sha3_256(b"")),
            "a7ffc6f8bf1ed76651c14756a061d662f580ff4de43b49fa82d80a4b80f8434a"
        );
        assert_eq!(
            hex(&sha3_256(b"abc")),
            "3a985da74fe225b2045c172d6bd390bd855f086e3e9d525b46bfe24511431532"
        );
    }
}
