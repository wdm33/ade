// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! RED seed-epoch consensus-inputs WAL provenance append
//! (PHASE4-N-F-A A3a).
//!
//! The single shared helper that appends the closed
//! `WalEntry::SeedEpochConsensusInputsImported` provenance entry
//! after the verified-bootstrap composition site has durably
//! `put` the seed-epoch sidecar. It is RED because it touches the
//! `WalStore` (I/O); the entry it writes — and its codec/replay —
//! are BLUE in `ade_ledger::wal`.
//!
//! Ordering / commit point (the load-bearing invariant): the
//! composition site `put`s the sidecar (durable) and THEN calls
//! this helper to append the WAL entry (durable = the commit
//! point). A crash between the two leaves the sidecar present but
//! no provenance entry — replay (A3a) yields no
//! `RecoveredBootstrapProvenance`, so warm-start (A3b) treats the
//! import as "not imported" and fails closed. The provenance is
//! never observed half-written.
//!
//! `sidecar_hash` is `blake2b_256` of the EXACT A1 canonical
//! sidecar bytes the composer just `put` — the same `&[u8]`,
//! never a re-encode (CN-CINPUT-01: A1 is the sole encoder).
//!
//! Containment (CN-CINPUT-02, extended): the call to
//! `append_seed_epoch_provenance` is allowed only at the two
//! verified-bootstrap composition sites (`genesis_bootstrap.rs`,
//! `mithril_bootstrap.rs`); the forge-time path may not reference
//! it. `ci/ci_check_consensus_input_provenance.sh` enforces this.

use ade_crypto::blake2b_256;
use ade_ledger::wal::{WalEntry, WalError, WalStore};
use ade_types::{EpochNo, Hash32};

/// Append the bootstrap-provenance WAL entry for a just-persisted
/// seed-epoch consensus-input sidecar.
///
/// MUST be called only AFTER the sidecar bytes have been durably
/// `put` through the anchor-keyed `SnapshotStore` surface;
/// `sidecar_bytes` MUST be the exact A1 canonical bytes that were
/// written (the hash binds the WAL fact to those bytes). The WAL
/// append is the commit point for the import.
pub fn append_seed_epoch_provenance(
    wal: &mut dyn WalStore,
    anchor_fp: &Hash32,
    epoch_no: EpochNo,
    sidecar_bytes: &[u8],
) -> Result<(), WalError> {
    let sidecar_hash = blake2b_256(sidecar_bytes);
    wal.append(WalEntry::SeedEpochConsensusInputsImported {
        anchor_fp: anchor_fp.clone(),
        sidecar_hash,
        epoch_no,
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::*;
    use ade_ledger::wal::{replay_from_anchor, RecoveredBootstrapProvenance};
    use std::collections::BTreeMap;

    /// Minimal in-memory `WalStore` double for unit coverage.
    struct VecWal {
        entries: Vec<WalEntry>,
    }
    impl VecWal {
        fn new() -> Self {
            Self {
                entries: Vec::new(),
            }
        }
    }
    impl WalStore for VecWal {
        fn append(&mut self, entry: WalEntry) -> Result<(), WalError> {
            self.entries.push(entry);
            Ok(())
        }
        fn read_all(&self) -> Result<Vec<WalEntry>, WalError> {
            Ok(self.entries.clone())
        }
    }

    #[test]
    fn append_then_replay_surfaces_provenance_with_bound_hash() {
        let anchor_fp = Hash32([0x01; 32]);
        let sidecar_bytes = vec![0xDE, 0xAD, 0xBE, 0xEF];
        let mut wal = VecWal::new();

        append_seed_epoch_provenance(&mut wal, &anchor_fp, EpochNo(576), &sidecar_bytes)
            .expect("append");

        // read_all includes the provenance entry.
        let entries = wal.read_all().expect("read_all");
        assert_eq!(entries.len(), 1);

        // Replay reconstructs the view; sidecar_hash is the
        // blake2b_256 of the exact bytes we passed.
        let bb = BTreeMap::new();
        let out = replay_from_anchor(&anchor_fp, &entries, &bb).expect("replay");
        assert_eq!(
            out.provenance,
            Some(RecoveredBootstrapProvenance {
                anchor_fp: Hash32([0x01; 32]),
                sidecar_hash: blake2b_256(&sidecar_bytes),
                epoch_no: EpochNo(576),
            })
        );
        assert_eq!(out.admit_count, 0);
    }
}
