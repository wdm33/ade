// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! EPOCH-CONSENSUS-VIEW S3a (DC-EVIEW-03) — pre-Conway pointer RESOLUTION.
//!
//! A pointer address's decoded coordinates (`ade_codec::address::Ptr`, the
//! era-parameterized decoder) resolve to the stake credential REGISTERED by the
//! `StakeRegistration` certificate at exactly that `(slot, txIx, certIx)` chain
//! position. This is the per-output resolution ALGORITHM over a [`PointerMap`]; the
//! map is POPULATED from registration certs by the windowed cert accumulation (S3b)
//! — S3a provides the type + the algorithm, tested against a synthetic map.
//!
//! Pre-Conway ONLY: at Conway (PV9+) pointer stake is retired, so the Slice-2
//! classifier yields `Null` and no resolution occurs. Fail-closed: an unregistered
//! coordinate resolves to `None` (no stake) — never a fabricated credential.

use std::collections::btree_map::{BTreeMap, Entry};

use ade_codec::address::Ptr;
use ade_types::shelley::cert::StakeCredential;

/// `(slot, txIx, certIx)` → the credential registered by the cert at that position.
/// Keyed on the cardano-ledger stored pointer shape (u32 slot, u16 txIx, u16 certIx).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PointerMap {
    map: BTreeMap<(u32, u16, u16), StakeCredential>,
}

impl PointerMap {
    pub fn new() -> Self {
        Self { map: BTreeMap::new() }
    }

    /// Record the credential registered by a `StakeRegistration` cert at its chain
    /// position `(slot, txIx, certIx)`. A coordinate is the position of exactly ONE
    /// certificate, so a duplicate coordinate is a malformed input — fail-closed:
    /// returns `false` and does NOT overwrite (the caller treats it as an error).
    pub fn insert(
        &mut self,
        slot: u32,
        tx_index: u16,
        cert_index: u16,
        cred: StakeCredential,
    ) -> bool {
        match self.map.entry((slot, tx_index, cert_index)) {
            Entry::Vacant(v) => {
                v.insert(cred);
                true
            }
            Entry::Occupied(_) => false,
        }
    }

    /// Resolve a decoded pointer to its registered credential. Fail-closed: an
    /// unregistered coordinate yields `None` (no stake), never a fabricated
    /// credential. (Conway-retired pointers never reach here — they classify to
    /// `Null` upstream.)
    pub fn resolve(&self, ptr: &Ptr) -> Option<StakeCredential> {
        self.map
            .get(&(ptr.slot, ptr.tx_index, ptr.cert_index))
            .cloned()
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ade_types::Hash28;

    fn key_cred(fill: u8) -> StakeCredential {
        StakeCredential::KeyHash(Hash28([fill; 28]))
    }
    fn script_cred(fill: u8) -> StakeCredential {
        StakeCredential::ScriptHash(Hash28([fill; 28]))
    }

    #[test]
    fn resolves_a_registered_pointer() {
        let mut m = PointerMap::new();
        assert!(m.insert(100, 2, 3, key_cred(0xAB)));
        assert_eq!(
            m.resolve(&Ptr { slot: 100, tx_index: 2, cert_index: 3 }),
            Some(key_cred(0xAB))
        );
    }

    #[test]
    fn unregistered_pointer_is_none_fail_closed() {
        let mut m = PointerMap::new();
        m.insert(100, 2, 3, key_cred(0xAB));
        // a different coordinate has no registration -> None (no stake), not a guess.
        assert_eq!(m.resolve(&Ptr { slot: 100, tx_index: 2, cert_index: 4 }), None);
        assert_eq!(m.resolve(&Ptr { slot: 101, tx_index: 2, cert_index: 3 }), None);
        // an empty map resolves nothing.
        assert_eq!(PointerMap::new().resolve(&Ptr { slot: 1, tx_index: 1, cert_index: 1 }), None);
    }

    #[test]
    fn duplicate_position_is_rejected_fail_closed() {
        let mut m = PointerMap::new();
        assert!(m.insert(5, 0, 0, key_cred(0x11)));
        // the SAME coordinate registered twice is malformed -> false, no overwrite.
        assert!(!m.insert(5, 0, 0, key_cred(0x22)));
        assert_eq!(
            m.resolve(&Ptr { slot: 5, tx_index: 0, cert_index: 0 }),
            Some(key_cred(0x11)),
            "the first registration is kept; the duplicate does not overwrite"
        );
    }

    #[test]
    fn distinct_coordinates_resolve_independently() {
        let mut m = PointerMap::new();
        m.insert(1, 0, 0, key_cred(0x01));
        m.insert(1, 0, 1, script_cred(0x02));
        m.insert(2, 0, 0, key_cred(0x03));
        assert_eq!(m.resolve(&Ptr { slot: 1, tx_index: 0, cert_index: 0 }), Some(key_cred(0x01)));
        assert_eq!(m.resolve(&Ptr { slot: 1, tx_index: 0, cert_index: 1 }), Some(script_cred(0x02)));
        assert_eq!(m.resolve(&Ptr { slot: 2, tx_index: 0, cert_index: 0 }), Some(key_cred(0x03)));
        assert_eq!(m.len(), 3);
    }

    #[test]
    fn resolution_is_deterministic() {
        let mut m = PointerMap::new();
        m.insert(7, 1, 2, key_cred(0x55));
        let p = Ptr { slot: 7, tx_index: 1, cert_index: 2 };
        assert_eq!(m.resolve(&p), m.resolve(&p));
    }
}
