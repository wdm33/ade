// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! BLUE recovered anchor-point load + fail-closed verify (PHASE4-N-AK AK-S1).
//!
//! Companion to `bootstrap::resolve_live_follow_start` (DC-NODE-31). The recover
//! path (`ade_node::node_lifecycle::warm_start_recovery`) calls
//! [`load_recovered_anchor_point`] once it has discovered the recovered
//! `anchor_fp`; the returned [`ChainTip`] becomes the canonical
//! [`crate::bootstrap::BootstrapInputs::recovered_anchor`] input, which
//! `bootstrap_initial_state` then resolves into the live-follow start tip.
//!
//! Kept OUT of `bootstrap.rs` so that module stays the single-`pub fn` bootstrap
//! authority (CN-NODE-01, `ci/ci_check_bootstrap_closure.sh`): the load is a
//! recovery-time store read + verify, distinct from the bootstrap transition
//! itself. The single `SnapshotStore` read is RED I/O of a BLUE-authoritative
//! record; the decode (via the sole codec) + the `anchor_fp` binding check are
//! BLUE. Mirrors `bootstrap::restore_seed_epoch_consensus_inputs`.

use ade_ledger::recovered_anchor_point::decode_recovered_anchor_point;
use ade_types::Hash32;

use crate::bootstrap::BootstrapError;
use crate::chaindb::{ChainTip, SnapshotStore};

/// BLUE load + fail-closed verify of the persisted recovered anchor-point
/// record (PHASE4-N-AK AK-S1, DC-NODE-31). The recover path
/// (`warm_start_recovery`) calls this once it has discovered the recovered
/// `anchor_fp`; the returned [`ChainTip`] becomes the canonical
/// [`crate::bootstrap::BootstrapInputs::recovered_anchor`] input.
///
/// This is only called on the recover path, where the store is definitively
/// non-Origin (a seed-epoch anchor lineage was discovered) — so a missing /
/// malformed / fingerprint-mismatched record is a deterministic fail-closed
/// halt, never a silent Origin fallback.
pub fn load_recovered_anchor_point<S>(
    snapshot_store: &S,
    expected_anchor_fp: &Hash32,
) -> Result<ChainTip, BootstrapError>
where
    S: SnapshotStore + ?Sized,
{
    // 1. Record bytes for the recovered anchor (absent => fail closed).
    let bytes = snapshot_store
        .get_recovered_anchor_point(expected_anchor_fp)
        .map_err(BootstrapError::ChainDb)?
        .ok_or(BootstrapError::RecoveredAnchorPointMissing {
            anchor_fp: expected_anchor_fp.clone(),
        })?;

    // 2. Decode via the sole codec (malformed / unknown version / non-canonical
    //    / trailing bytes all fail here).
    let record = decode_recovered_anchor_point(&bytes)
        .map_err(BootstrapError::RecoveredAnchorPointDecode)?;

    // 3. Binding: the record must describe THIS recovered anchor lineage.
    if record.anchor_fp != *expected_anchor_fp {
        return Err(BootstrapError::RecoveredAnchorPointBindingMismatch {
            expected_anchor_fp: expected_anchor_fp.clone(),
            actual_anchor_fp: record.anchor_fp,
        });
    }

    Ok(ChainTip {
        slot: record.slot,
        hash: record.block_hash,
    })
}
