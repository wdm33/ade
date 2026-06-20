//! EPOCH-CONSENSUS-VIEW S3f-2 (DC-EVIEW-08) — the window driver: advance the reduced
//! UTxO checkpoint + the cert/delegation state forward over a window of blocks, then
//! aggregate per-pool stake.
//!
//! This is the orchestration that ties the proven pieces into one pass over a real
//! epoch's blocks:
//!   1. per block: `reduced_block_delta` (== reduce(track_utxo), proven) -> the
//!      checkpoint's `apply_block_delta`; and `advance_cert_state`
//!      (== `process_block_certificates`) to carry the delegation/reward state forward;
//!   2. once: `sum_base_credential_stake` (the per-base-credential UTxO coin sums) ->
//!      `aggregate_pool_stake` over the advanced delegation map.
//!
//! The starting cert state is the manifest-bound bootstrap cert state (DC-EVIEW-09), so
//! the advanced delegation map includes PRE-bootstrap delegators -- the window does NOT
//! start from an empty map. The starting UTxO is the bootstrap reduced checkpoint. So the
//! aggregate is over Ade's OWN complete state, not a cardano-node import.
//!
//! RED orchestration: it reads/writes the durable redb checkpoint (I/O) and clones a
//! `LedgerState` per call (a transient window operation, never the hot live path). The
//! per-step transforms it sequences are deterministic and individually proven.

use super::reduced_utxo_checkpoint::{ReducedCheckpointError, ReducedUtxoCheckpoint};
use ade_ledger::error::LedgerError;
use ade_ledger::reduced_advance::{advance_cert_state, reduced_block_delta};
use ade_ledger::reduced_aggregate::{aggregate_pool_stake, AggregateError, StakeByPool};
use ade_ledger::state::LedgerState;
use ade_types::shelley::block::ShelleyBlock;
use ade_types::CardanoEra;

/// Fail-closed reasons the window drive cannot produce an aggregate. Every variant aborts
/// the window without producing a partial / wrong stake distribution.
#[derive(Debug)]
pub enum WindowDriverError {
    /// The reduced-UTxO checkpoint store rejected an apply / read.
    Checkpoint(ReducedCheckpointError),
    /// A block's reduced delta or cert advance failed (authority-fatal ledger error).
    Ledger(LedgerError),
    /// The per-pool aggregation overflowed (`checked_add` guard).
    Aggregate(AggregateError),
}

/// Drive the reduced checkpoint + cert state forward over `blocks`, then aggregate.
///
/// `checkpoint` must already be `build_from` the bootstrap reduced UTxO; `bootstrap_state`
/// must carry the manifest-bound bootstrap cert state (DC-EVIEW-09). The blocks are the
/// window's ordered blocks (bootstrap -> the boundary). Mirrors the ledger's per-block
/// cert application (`rules.rs`: `(cert_state, gov_state) = process_block_certificates`)
/// exactly. Returns the per-pool `StakeByPool` for the window's end state, or a
/// fail-closed error.
pub fn drive_window_aggregate(
    checkpoint: &ReducedUtxoCheckpoint,
    bootstrap_state: &LedgerState,
    blocks: &[ShelleyBlock],
    era: CardanoEra,
) -> Result<StakeByPool, WindowDriverError> {
    let mut state = bootstrap_state.clone();
    for block in blocks {
        // UTxO side: the reduced block delta (== reduce(track_utxo)) into the checkpoint.
        let delta = reduced_block_delta(block, era).map_err(WindowDriverError::Ledger)?;
        checkpoint
            .apply_block_delta(&delta.spent, &delta.produced)
            .map_err(WindowDriverError::Checkpoint)?;
        // Cert side: advance the delegation/reward (+ gov) state, exactly as the ledger does.
        let (cert_state, gov_state) =
            advance_cert_state(block, era, &state).map_err(WindowDriverError::Ledger)?;
        state.cert_state = cert_state;
        state.gov_state = gov_state;
    }
    let cred_utxo_stake = checkpoint
        .sum_base_credential_stake()
        .map_err(WindowDriverError::Checkpoint)?;
    aggregate_pool_stake(&cred_utxo_stake, &state.cert_state.delegation)
        .map_err(WindowDriverError::Aggregate)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ade_codec::cbor::envelope::decode_block_envelope;
    use ade_ledger::reduced_utxo::ReducedStakeRef;
    use ade_types::shelley::cert::StakeCredential;
    use ade_types::tx::{Coin, PoolId, TxIn};
    use ade_types::{Hash28, Hash32};
    use std::collections::BTreeMap;

    const RAW_CONWAY_BLOCK: &[u8] =
        include_bytes!("../../../ade_node/tests/fixtures/raw_era_block_conway.cbor");

    fn temp_checkpoint() -> (ReducedUtxoCheckpoint, tempfile::TempDir) {
        let dir = tempfile::tempdir().expect("tempdir");
        let cp = ReducedUtxoCheckpoint::open(&dir.path().join("reduced.redb")).expect("open");
        (cp, dir)
    }

    fn cred(b: u8) -> StakeCredential {
        StakeCredential::KeyHash(Hash28([b; 28]))
    }
    fn pool(b: u8) -> PoolId {
        PoolId(Hash28([b; 28]))
    }
    fn txin(b: u8) -> TxIn {
        TxIn { tx_hash: Hash32([b; 32]), index: 0 }
    }

    /// Bootstrap-only window (zero blocks): the aggregate is the bootstrap reduced UTxO
    /// grouped by the bootstrap cert state's delegation map -- proving the
    /// checkpoint->sum->aggregate wiring + that the PRE-bootstrap delegators (present in
    /// the bootstrap cert state) are counted.
    #[test]
    fn empty_window_aggregates_bootstrap_state() {
        let (cp, _dir) = temp_checkpoint();
        // bootstrap reduced UTxO: two base creds with coin.
        let mut reduced: BTreeMap<TxIn, (Coin, ReducedStakeRef)> = BTreeMap::new();
        reduced.insert(txin(1), (Coin(1000), ReducedStakeRef::Base(cred(0xA))));
        reduced.insert(txin(2), (Coin(2000), ReducedStakeRef::Base(cred(0xB))));
        reduced.insert(txin(3), (Coin(9999), ReducedStakeRef::NonContributing));
        cp.build_from(&reduced).expect("build_from");

        // bootstrap cert state: both creds delegate to the same pool (a pre-bootstrap fact).
        let mut state = LedgerState::new(CardanoEra::Conway);
        state.cert_state.delegation.delegations.insert(cred(0xA), pool(0x1));
        state.cert_state.delegation.delegations.insert(cred(0xB), pool(0x1));

        let agg = drive_window_aggregate(&cp, &state, &[], CardanoEra::Conway).expect("drive");
        // pool 0x1 = 1000 + 2000; the NonContributing 9999 is excluded.
        assert_eq!(agg.pool_stakes.get(&pool(0x1)).copied(), Some(Coin(3000)));
        assert_eq!(agg.total_active_stake, Coin(3000));
    }

    /// Drive over a REAL conway block: the driver's result equals the SAME pieces composed
    /// by hand (apply the block's reduced delta to the checkpoint + advance the cert state
    /// + aggregate). Proves the loop sequences the proven pieces correctly on real data.
    #[test]
    fn real_conway_block_drive_equals_composed_pieces() {
        let env = decode_block_envelope(RAW_CONWAY_BLOCK).expect("envelope");
        let inner = &RAW_CONWAY_BLOCK[env.block_start..env.block_end];
        let block = ade_codec::conway::decode_conway_block(inner)
            .expect("decode conway block")
            .decoded()
            .clone();
        let era = CardanoEra::Conway;

        // a bootstrap reduced UTxO with a delegated base cred (so the aggregate is non-trivial).
        let mut reduced: BTreeMap<TxIn, (Coin, ReducedStakeRef)> = BTreeMap::new();
        reduced.insert(txin(7), (Coin(5_000_000), ReducedStakeRef::Base(cred(0xC))));
        let mut state = LedgerState::new(era);
        state.cert_state.delegation.delegations.insert(cred(0xC), pool(0x9));

        // (1) the driver.
        let (cp_drv, _d1) = temp_checkpoint();
        cp_drv.build_from(&reduced).expect("build");
        let driven =
            drive_window_aggregate(&cp_drv, &state, std::slice::from_ref(&block), era).expect("drive");

        // (2) the same pieces composed by hand.
        let (cp_ref, _d2) = temp_checkpoint();
        cp_ref.build_from(&reduced).expect("build");
        let delta = reduced_block_delta(&block, era).expect("delta");
        cp_ref.apply_block_delta(&delta.spent, &delta.produced).expect("apply");
        let (cert_state, _gov) = advance_cert_state(&block, era, &state).expect("advance");
        let cred_utxo = cp_ref.sum_base_credential_stake().expect("sum");
        let composed = aggregate_pool_stake(&cred_utxo, &cert_state.delegation).expect("agg");

        assert_eq!(driven, composed, "the window driver == the proven pieces composed in order");
    }
}
