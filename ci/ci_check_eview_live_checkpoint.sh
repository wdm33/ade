#!/usr/bin/env bash
set -uo pipefail

# EPOCH-CONSENSUS-VIEW S3f-4d-mat (DC-EPOCH-11): the live reduced-UTxO checkpoint. -mat-1
# (this gate): build the authoritative reduced checkpoint from the seed UTxO at bootstrap,
# BEFORE the UTxO is dropped. It is a deterministic projection of the ledger UTxO (replay-
# equivalent), disk-backed (redb), and GATED on the EVIEW cert-state package so non-EVIEW
# bootstrap stays BYTE-IDENTICAL. Fail-closed on a build failure. (Per-block advance, reorg
# re-materialize, fail-closed gating, and the shadow-derivation proof are owed sub-slices.)

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"; cd "$REPO_ROOT"
FAILED=0; fail() { echo "FAIL: $1"; FAILED=1; }
B=crates/ade_node/src/admission/bootstrap.rs

# (1) the bootstrap build reuses the proven reduce + checkpoint machinery (NOT a new stake system).
grep -qE 'fn build_live_reduced_checkpoint' "$B" || fail "build_live_reduced_checkpoint missing"
grep -qF 'reduce_txout(txout)' "$B" || fail "the build does not reduce via reduce_txout (DC-EVIEW-04)"
grep -qF 'ReducedUtxoCheckpoint::open' "$B" || fail "the build does not open the durable reduced checkpoint"
grep -qF '.build_from(&reduced)' "$B" || fail "the build does not build_from the reduced map (DC-EVIEW-10 machinery)"

# (2) disk-backed durable path (redb in the snapshot dir).
grep -qF 'reduced-checkpoint.redb' "$B" || fail "the reduced checkpoint is not a disk-backed redb in the snapshot dir"

# (3) BYTE-IDENTICAL until -wire: the build is GATED on the EVIEW cert-state package, so a
#     non-EVIEW bootstrap (empty cert state) builds nothing and is unchanged.
grep -qF 'ledger.cert_state.delegation.delegations.is_empty()' "$B" \
    || fail "the build is not gated on the EVIEW cert-state -- non-EVIEW bootstrap must stay byte-identical"

# (4) the build happens BEFORE the UTxO is dropped (so the seed UTxO is still resident).
awk '/build_live_reduced_checkpoint\(&snapshot_dir, &utxo/{b=NR} /drop\(utxo\);/{d=NR} END{exit !(b>0 && d>0 && b<d)}' "$B" \
    || fail "the reduced checkpoint must be built BEFORE drop(utxo)"
# the build SEALS the immutable bootstrap baseline + records the seed slot (advancer resumes
# from seed_slot+1, anchor not re-applied; a reorg rollback re-materializes from this baseline).
grep -qF 'checkpoint.seal_bootstrap(seed_slot)' "$B" || fail "the build does not seal the bootstrap baseline (seal_bootstrap)"

# (5) fail-closed on a build failure.
grep -qE 'AdmissionBootstrapError::ReducedCheckpoint' "$B" || fail "a reduced-checkpoint build failure is not fail-closed"

# (6) no resident full UTxO is retained by the build (the reduced map is transient, freed; the
#     existing drop(utxo) + track_utxo=false path is preserved).
grep -qF 'drop(utxo);' "$B" || fail "the seed UTxO drop (track_utxo=false steady state) was removed"

# (7) the proof.
grep -qE 'fn live_reduced_checkpoint_builds_durable_deterministic' "$B" || fail "the -mat-1 durable/deterministic proof is missing"

# (8) -mat-2 primitive: the per-block advance records its slot ATOMICALLY (the lockstep
#     cursor the live ChainDB replay drives) -- a durable marker, not best-effort.
CP=crates/ade_runtime/src/chaindb/reduced_utxo_checkpoint.rs
grep -qE 'pub fn advance_block' "$CP" || fail "advance_block (the per-block advance) missing"
grep -qE 'pub fn last_advanced_slot' "$CP" || fail "last_advanced_slot (the lockstep cursor) missing"
grep -qF 'LAST_SLOT_KEY' "$CP" || fail "the durable last-advanced-slot marker is missing"
grep -qE 'fn advance_block_applies_delta_and_records_slot' "$CP" || fail "the -mat-2 advance/slot proof is missing"

# (9) -mat-2b: the live ChainDB-replay advancer reads ONLY the durable ChainDB (selected
#     chain), in slot order, fail-closed -- not peer/network/clock.
WD=crates/ade_runtime/src/chaindb/reduced_window_driver.rs
grep -qE 'pub fn advance_reduced_checkpoint_over_chaindb' "$WD" || fail "the live ChainDB-replay advancer missing"
grep -qF '.iter_from_slot(from)' "$WD" || fail "the advancer does not replay the durable ChainDB in order"
grep -qF 'reduced_block_delta(&block, era)' "$WD" || fail "the advancer does not reduce via DC-EVIEW-04"
grep -qF '.advance_block(stored.slot' "$WD" || fail "the advancer does not advance_block (lockstep slot)"
grep -qE 'enum CheckpointAdvanceError' "$WD" || fail "the advancer is not fail-closed (CheckpointAdvanceError)"
grep -qE 'fn advance_over_chaindb_replays_durable_blocks' "$WD" || fail "the -mat-2b advancer proof is missing"

# (10) -mat-2c: the relay loop opens the checkpoint ONLY when it exists (= EVIEW configured)
#      and advances it to the durable tip after each admit -- byte-identical when absent.
NL=crates/ade_node/src/node_lifecycle.rs
grep -qF 'if reduced_checkpoint_path.exists()' "$NL" || fail "the relay-loop open is not gated on the checkpoint existing (EVIEW-only)"
grep -qF 'advance_reduced_checkpoint_to_durable_tip(reduced_checkpoint, chaindb)' "$NL" || fail "the loop does not advance the checkpoint after the admit"
grep -qF 'advance_reduced_checkpoint_over_chaindb(' "$NL" || fail "the loop helper does not call the durable-ChainDB advancer"
# the helper no-ops when EVIEW is not configured (None) -> the follow/forge path is byte-identical.
grep -qF 'let Some(cp) = reduced_checkpoint else {' "$NL" || fail "the advance is not a no-op when EVIEW is unconfigured (byte-identical)"

# (11) -mat-3: reorg re-materialize. The checkpoint seals an IMMUTABLE bootstrap baseline; a
#      rollback (advanced past the durable tip) re-materializes the live table from it.
grep -qE 'pub fn seal_bootstrap' "$CP" || fail "seal_bootstrap (the immutable bootstrap baseline) missing"
grep -qE 'pub fn reset_to_bootstrap' "$CP" || fail "reset_to_bootstrap (the reorg re-materialize) missing"
grep -qF 'BOOTSTRAP_TABLE' "$CP" || fail "the immutable bootstrap table is missing"
grep -qF 'cp.reset_to_bootstrap()' "$NL" || fail "the loop does not re-materialize the checkpoint on a rollback"
grep -qF 'if advanced.0 > tip.slot.0 {' "$NL" || fail "the loop does not DETECT a rollback (advanced past the durable tip)"
grep -qE 'fn reset_to_bootstrap_re_materializes_seed_state' "$CP" || fail "the -mat-3 re-materialize proof is missing"

# (12) -mat-4: the fail-closed readiness gate -- blocks view production on a missing/corrupt/
#      lagging/wrong-lineage/overshot checkpoint; admits ONLY an exact-slot, matching-lineage one.
grep -qE 'pub fn verify_ready_at' "$CP" || fail "verify_ready_at (the readiness gate) missing"
grep -qE 'enum CheckpointReadinessError' "$CP" || fail "the readiness reject enum missing"
grep -qF 'CheckpointReadinessError::Lagging' "$CP" || fail "the lagging reject missing (fail-closed on behind)"
grep -qF 'CheckpointReadinessError::SeedMismatch' "$CP" || fail "the lineage reject missing (fail-closed on wrong seed)"
grep -qF 'CheckpointReadinessError::Ahead' "$CP" || fail "the overshoot reject missing (fail-closed on past-required)"
grep -qE 'fn verify_ready_at_fails_closed_unless_exact_and_lineage_bound' "$CP" || fail "the -mat-4 readiness proof missing"

# (13) -mat-shadow: the live derive -- the checkpoint's per-pool stake via the delegation state
#      (the candidate view's stake-by-pool, compared to the oracle in the live shadow proof).
grep -qE 'pub fn derive_stake_by_pool' "$CP" || fail "derive_stake_by_pool (the live derive) missing"
grep -qF 'aggregate_pool_stake(&cred_stake, delegation)' "$CP" || fail "the derive does not aggregate via the delegation state"
grep -qE 'fn derive_stake_by_pool_aggregates_via_delegation' "$CP" || fail "the -mat-shadow derive proof missing"

# (14) -wire-1: the live source-window extraction (the AUTHORITATIVE replay path's input).
#      Reads ONLY the durable selected-chain ChainDB, explicit bounds (no wall clock), validates.
EW=crates/ade_node/src/epoch_wire.rs
grep -qE 'pub fn extract_source_window' "$EW" || fail "extract_source_window (the durable-window input) missing"
grep -qF '.iter_from_slot(source_window_start)' "$EW" || fail "the extraction does not read the durable ChainDB by the explicit start bound"
grep -qF 'validate_source_window(&window, &window_blocks)' "$EW" || fail "the extraction does not validate the window (fail-closed)"
grep -qE 'fn extract_source_window_pins_and_validates_a_durable_block' "$EW" || fail "the -wire-1 extraction proof missing"

# (15) -wire-2a: the RUNTIME readiness witness (advanced-THROUGH >= required, not exact-at-B --
#      the live checkpoint is non-authoritative) + the fresh seed-state replay checkpoint (the
#      authoritative window replay advances a SEPARATE redb, never the live one).
grep -qE 'pub fn verify_advanced_through' "$CP" || fail "verify_advanced_through (runtime readiness witness) missing"
grep -qE 'pub fn materialize_bootstrap_into' "$CP" || fail "materialize_bootstrap_into (fresh replay seed-state) missing"
grep -qE 'fn verify_advanced_through_admits_at_or_beyond_required' "$CP" || fail "the -wire-2a readiness proof missing"
grep -qE 'fn materialize_bootstrap_into_yields_fresh_independent_seed_state' "$CP" || fail "the -wire-2a materialize proof missing"

# (16) -wire-2b: the orchestration -- the live readiness WITNESS (advanced-through the window
#      end + the terminal ChainDB lineage commitment) + the SOLE authoritative derive (durable
#      window replay over a FRESH seed-state checkpoint; the live checkpoint is NOT the derive).
grep -qE 'pub fn verify_live_readiness' "$EW" || fail "verify_live_readiness (the readiness witness) missing"
grep -qF 'live.verify_advanced_through(window.source_window_end' "$EW" || fail "readiness does not check advanced-through the window end"
grep -qF '.get_block_by_hash(&window.lineage_pin)' "$EW" || fail "readiness does not check the terminal ChainDB lineage commitment"
grep -qE 'pub fn derive_authoritative_candidate' "$EW" || fail "derive_authoritative_candidate (the SOLE authoritative derive) missing"
grep -qF 'materialize_bootstrap_into(scratch_path)' "$EW" || fail "the authoritative derive does not replay over a FRESH seed-state checkpoint"
grep -qE 'fn verify_live_readiness_requires_advanced_through_and_terminal_present' "$EW" || fail "the -wire-2b readiness proof missing"

if (( FAILED == 0 )); then
    echo "OK: live reduced checkpoint -mat-1 (DC-EPOCH-11; build from seed UTxO via the proven reduce+checkpoint machinery, disk-backed, gated=byte-identical, before drop(utxo), fail-closed)"
fi
exit $FAILED
