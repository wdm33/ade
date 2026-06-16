// Core Contract:
// - Deterministic: same inputs => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Canonical serialization for all persisted/hashed data

//! The v2 UTxO set commitment -- a Ristretto255 ECMH (elliptic-curve multiset
//! hash). NAMED + domain-separated; part of Ade's INTERNAL replay contract
//! (MEM-OPT-UTXO-DISK S1.5 / DC-MEM-10). NOT peer-facing, NOT Cardano consensus.
//!
//!   entry(bytes) = RistrettoPoint::from_uniform_bytes(blake2b_512(DOMAIN_ENTRY || bytes))
//!   commitment   = identity + Σ entry(e) over all live e   -- commutative; O(1) add/remove
//!   digest       = blake2b_256(DOMAIN_DIGEST || commitment.compress())  -- 32-byte Hash32
//!
//! The commitment is a commutative GROUP accumulator: `add` and `remove` are
//! exact inverses (point + / -), and the digest is independent of insertion
//! order (a real multiset commitment, NOT a naive XOR/sum -- those are insecure).
//! Deterministic + constant-time (no float, no platform-dependence) -- BLUE-safe.
//!
//! Used by `ade_ledger::fingerprint::fingerprint_utxo_v2` (the S1.5a full-recompute
//! oracle) and, in S1.5b, by per-block incremental maintenance.

use ade_types::Hash32;
use blake2::digest::Digest;
use curve25519_dalek::ristretto::RistrettoPoint;
use curve25519_dalek::traits::Identity;

use crate::blake2b::blake2b_256;

/// Entry-commitment domain tag. FROZEN -- changing it is a new fingerprint version.
pub const DOMAIN_ENTRY: &[u8] = b"ade.utxo.fp.v2.entry";
/// Digest domain tag. FROZEN.
pub const DOMAIN_DIGEST: &[u8] = b"ade/fp/utxo/v2";

/// Hash-to-Ristretto of one entry's domain-separated canonical bytes. The 64
/// uniform bytes (Blake2b-512) map to a Ristretto point via the Elligator map.
fn entry_point(canonical_entry_bytes: &[u8]) -> RistrettoPoint {
    let mut hasher = blake2::Blake2b::<blake2::digest::consts::U64>::new();
    hasher.update(DOMAIN_ENTRY);
    hasher.update(canonical_entry_bytes);
    let wide: [u8; 64] = hasher.finalize().into();
    RistrettoPoint::from_uniform_bytes(&wide)
}

/// A commutative UTxO set commitment -- an accumulating Ristretto point.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UtxoSetCommitment(RistrettoPoint);

impl UtxoSetCommitment {
    /// The empty set's commitment (the group identity).
    pub fn empty() -> Self {
        UtxoSetCommitment(RistrettoPoint::identity())
    }

    /// Add a live entry (produce an output). Commutative with every other add.
    pub fn add(&mut self, canonical_entry_bytes: &[u8]) {
        self.0 += entry_point(canonical_entry_bytes);
    }

    /// Remove an entry (spend an input). The exact inverse of `add`.
    pub fn remove(&mut self, canonical_entry_bytes: &[u8]) {
        self.0 -= entry_point(canonical_entry_bytes);
    }

    /// The 32-byte digest: Blake2b-256 over the domain-separated compressed point.
    pub fn digest(&self) -> Hash32 {
        let compressed = self.0.compress();
        let mut buf = Vec::with_capacity(DOMAIN_DIGEST.len() + 32);
        buf.extend_from_slice(DOMAIN_DIGEST);
        buf.extend_from_slice(compressed.as_bytes());
        blake2b_256(&buf)
    }
}

impl Default for UtxoSetCommitment {
    fn default() -> Self {
        Self::empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- structural invariants (no hardcoded values needed) ----

    #[test]
    fn order_independent() {
        // The multiset commitment is independent of insertion order.
        let mut a = UtxoSetCommitment::empty();
        a.add(b"alpha");
        a.add(b"beta");
        a.add(b"gamma");
        let mut b = UtxoSetCommitment::empty();
        b.add(b"gamma");
        b.add(b"alpha");
        b.add(b"beta");
        assert_eq!(a, b);
        assert_eq!(a.digest(), b.digest());
    }

    #[test]
    fn add_remove_is_exact_inverse() {
        let mut c = UtxoSetCommitment::empty();
        let empty_digest = c.digest();
        c.add(b"entry-x");
        c.add(b"entry-y");
        assert_ne!(c.digest(), empty_digest);
        c.remove(b"entry-y");
        c.remove(b"entry-x");
        assert_eq!(c, UtxoSetCommitment::empty());
        assert_eq!(c.digest(), empty_digest);
    }

    #[test]
    fn deterministic() {
        let build = || {
            let mut c = UtxoSetCommitment::empty();
            c.add(b"one");
            c.add(b"two");
            c
        };
        assert_eq!(build().digest(), build().digest());
    }

    #[test]
    fn distinct_sets_distinct_digests() {
        let mut a = UtxoSetCommitment::empty();
        a.add(b"x");
        let mut b = UtxoSetCommitment::empty();
        b.add(b"y");
        assert_ne!(a.digest(), b.digest());
    }

    #[test]
    fn binds_value_not_just_key() {
        // Two entries that share a prefix (the "key") but differ in suffix (the
        // "value") MUST commit differently -- the commitment binds the full bytes.
        let mut a = UtxoSetCommitment::empty();
        a.add(b"txin-0|coin=100");
        let mut b = UtxoSetCommitment::empty();
        b.add(b"txin-0|coin=200");
        assert_ne!(a.digest(), b.digest());
    }

    // ---- GOLDEN VECTORS (FROZEN -- regenerating silently is a contract break) ----

    #[test]
    fn golden_empty_digest() {
        assert_eq!(
            format!("{}", UtxoSetCommitment::empty().digest()),
            "70b2faf838d2fe2cdf7d2d54a10491cbb5f572ba61e17768b7ddf8f7fd466ac4"
        );
    }

    #[test]
    fn golden_single_entry_digest() {
        let mut c = UtxoSetCommitment::empty();
        c.add(b"ade.utxo.fp.v2.golden.entry.1");
        assert_eq!(
            format!("{}", c.digest()),
            "84ddb1dd89b50f55a6443c9086007ad248baace25d12113a79ceeefcecddf151"
        );
    }

    #[test]
    fn golden_two_entry_digest() {
        let mut c = UtxoSetCommitment::empty();
        c.add(b"ade.utxo.fp.v2.golden.entry.1");
        c.add(b"ade.utxo.fp.v2.golden.entry.2");
        assert_eq!(
            format!("{}", c.digest()),
            "a72f15a7646926f3c2c135335d463ca47139fbf50f3f07e64564202b65461fbd"
        );
    }
}
