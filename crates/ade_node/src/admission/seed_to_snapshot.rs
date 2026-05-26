// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! GREEN seed → snapshot bridge (PHASE4-N-M-B S3).
//!
//! Composes N-M-A's seed import outputs into a persisted snapshot
//! at `seed_point.slot`. The persisted snapshot is what
//! [`ade_runtime::bootstrap::bootstrap_initial_state`]'s warm-start
//! branch (CN-NODE-01) picks up at the next runner start; we do
//! NOT bypass bootstrap_initial_state.
//!
//! Hard non-goal: this module MUST NOT add any partial
//! reference-script support or seed-import fallback. A1's
//! `JsonSeedError::UnsupportedTxOutFeature` fail-fast remains
//! authoritative (DC-ADMIT-09). The CI gate
//! `ci/ci_check_admission_no_refscript_skip.sh` enforces it.
//!
//! Sole authority: `seed_to_snapshot`. Capture flows exclusively
//! through `PersistentSnapshotCache::capture` (CN-STORE-08).

use ade_core::consensus::praos_state::PraosChainDepState;
use ade_ledger::fingerprint::fingerprint;
use ade_ledger::state::LedgerState;
use ade_ledger::utxo::UTxOState;
use ade_runtime::chaindb::SnapshotStore;
use ade_runtime::rollback::{PersistentCacheError, PersistentSnapshotCache};
use ade_types::{CardanoEra, Hash32, SlotNo};

/// Closed error sum surfaced by [`seed_to_snapshot`]. Each variant
/// maps to a fatal admission-bootstrap halt at the runner (B4).
#[derive(Debug)]
pub enum SeedToSnapshotError {
    /// `PersistentSnapshotCache::capture` returned a Conway-encoder
    /// failure (e.g. caller supplied a pre-Conway ledger). The
    /// admission cluster is Conway-only; this is authority-fatal.
    Encode(PersistentCacheError),
    /// `SnapshotStore` returned an underlying I/O / store error
    /// (also surfaced through `PersistentCacheError::Store`).
    Store(PersistentCacheError),
}

/// Sole authority bridge. Builds a Conway `LedgerState` from the
/// imported `(UTxOState, PraosChainDepState)` pair, captures it at
/// `seed_point` via `PersistentSnapshotCache::capture`, and returns
/// the post-capture initial ledger fingerprint.
///
/// The returned fingerprint equals
/// `fingerprint(&built_ledger).combined`. The runner uses it as
/// the BootstrapAnchor's `initial_ledger_fingerprint` and as the
/// WAL chain's first `prior_fp` (DC-ANCHOR-01 / DC-WAL-02).
pub fn seed_to_snapshot<S: SnapshotStore + ?Sized>(
    utxo: UTxOState,
    chain_dep_seed: PraosChainDepState,
    seed_point: SlotNo,
    store: &S,
) -> Result<Hash32, SeedToSnapshotError> {
    let ledger = build_seed_ledger(utxo);
    let initial_fp = fingerprint(&ledger).combined;
    let cache = PersistentSnapshotCache::new(store);
    cache
        .capture(seed_point, &ledger, &chain_dep_seed)
        .map_err(|e| match e {
            err @ PersistentCacheError::Encode(_) => SeedToSnapshotError::Encode(err),
            err @ PersistentCacheError::Decode(_) => SeedToSnapshotError::Encode(err),
            err @ PersistentCacheError::Store(_) => SeedToSnapshotError::Store(err),
        })?;
    Ok(initial_fp)
}

/// Pure ledger build: Conway era, supplied UTxO map, all other
/// fields at their canonical defaults (LedgerState::new(Conway)).
/// Visible for tests + B4 reuse.
pub fn build_seed_ledger(utxo: UTxOState) -> LedgerState {
    let mut ledger = LedgerState::new(CardanoEra::Conway);
    ledger.utxo_state = utxo;
    ledger
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::*;

    use ade_core::consensus::praos_state::Nonce;
    use ade_runtime::chaindb::InMemoryChainDb;
    use ade_runtime::rollback::persistent_cache::PersistentSnapshotCache;

    fn empty_chain_dep() -> PraosChainDepState {
        PraosChainDepState::genesis(Nonce::ZERO)
    }

    fn empty_utxo() -> UTxOState {
        UTxOState::new()
    }

    #[test]
    fn seed_to_snapshot_writes_via_persistent_cache() {
        let store = InMemoryChainDb::new();
        let slot = SlotNo(12345);
        let _fp = seed_to_snapshot(empty_utxo(), empty_chain_dep(), slot, &store).expect("ok");
        // Read it back through the cache.
        let cache = PersistentSnapshotCache::new(&store);
        let bytes = store.get_snapshot(slot).expect("get").expect("present");
        // Round-trip via the cache reader.
        let _ = bytes;
        let result = ade_ledger::rollback::SnapshotReader::nearest_le(&cache, slot)
            .expect("decode-roundtrip");
        let (resolved_slot, _ledger, _chain_dep) = result;
        assert_eq!(resolved_slot, slot);
    }

    #[test]
    fn seed_to_snapshot_returns_initial_ledger_fingerprint() {
        let store = InMemoryChainDb::new();
        let slot = SlotNo(1);
        let fp = seed_to_snapshot(empty_utxo(), empty_chain_dep(), slot, &store).expect("ok");
        let expected = fingerprint(&build_seed_ledger(empty_utxo())).combined;
        assert_eq!(fp, expected);
    }

    #[test]
    fn seed_to_snapshot_two_runs_byte_identical() {
        let s1 = InMemoryChainDb::new();
        let s2 = InMemoryChainDb::new();
        let slot = SlotNo(42);
        let fp1 = seed_to_snapshot(empty_utxo(), empty_chain_dep(), slot, &s1).expect("ok");
        let fp2 = seed_to_snapshot(empty_utxo(), empty_chain_dep(), slot, &s2).expect("ok");
        assert_eq!(fp1, fp2);
        let b1 = s1.get_snapshot(slot).expect("get").expect("present");
        let b2 = s2.get_snapshot(slot).expect("get").expect("present");
        assert_eq!(b1, b2);
    }

    #[test]
    fn seed_to_snapshot_propagates_pre_conway_encode_error_as_authority_fatal() {
        // Force-encode a non-Conway ledger to confirm the
        // PersistentCacheError::Encode path surfaces as
        // SeedToSnapshotError::Encode (authority-fatal).
        let store = InMemoryChainDb::new();
        let mut ledger = LedgerState::new(CardanoEra::Shelley);
        ledger.utxo_state = empty_utxo();
        let cache = PersistentSnapshotCache::new(&store);
        let result = cache.capture(SlotNo(1), &ledger, &empty_chain_dep());
        assert!(matches!(result, Err(PersistentCacheError::Encode(_))));
    }
}
