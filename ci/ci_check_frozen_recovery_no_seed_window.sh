#!/usr/bin/env bash
set -uo pipefail

# CE-4A.3-R1 (DC-EPOCH-25 recovery seam): the WARM-START recovery of a promoted epoch authority in the
# post-S4 frozen regime (durable record target >= seed+2) is reconstructed SOLELY from a promotion-certified,
# epoch-indexed FrozenLeadershipPoolDistr -- byte-identically to the LIVE promotion. NEVER a seed-window
# replay, a bootstrap re-materialization, a seed fallback, a latest/current/nearest read, an active-pool-param
# leadership derivation, or a seed+3 terminal. The twin of ci_check_frozen_promotion_no_seed_window.sh (which
# guards the FORWARD block only); this one guards the RECOVERY block around maybe_recover_promoted_authority so
# a future edit cannot resurrect the retired window-replay recovery path (the CE-4A.3 finding this slice fixes).

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"; cd "$REPO_ROOT"
FAILED=0; fail() { echo "FAIL: $1"; FAILED=1; }

EW='crates/ade_node/src/epoch_wire.rs'

# (1) Isolate the RECOVERY frozen-regime block: the `target_epoch.0 >= seed_epoch.0 + 2` branch of
#     maybe_recover_promoted_authority (the live block uses `candidate_epoch.0 >= ...`, so this anchor is
#     unique to the recovery). Extract from the branch head to its 4-space-indented close.
BLOCK=$(awk '
    /if target_epoch\.0 >= seed_epoch\.0 \+ 2 \{/ { grab=1 }
    grab { print }
    grab && /^    \}$/ { exit }
' "$EW")
if [ -z "$BLOCK" ]; then
    fail "could not locate the recovery target_epoch>=seed+2 block in $EW (renamed / restructured? the guard anchor is broken)"
fi
CODE=$(echo "$BLOCK" | sed 's|//.*||')

# (2) The block MUST source recovered leadership from the promotion-certified frozen reader, and the checkpoint
#     commitment + source point MUST come from the frozen object itself (freeze-time provenance).
echo "$CODE" | grep -q 'promotion_leadership_authority_for_epoch(target_epoch)' \
    || fail "the recovery block does not read promotion_leadership_authority_for_epoch(target_epoch) (the SOLE recovered leadership source)"
echo "$CODE" | grep -q 'checkpoint_commitment: frozen\.source_checkpoint_commitment' \
    || fail "the recovery block's checkpoint commitment is not sourced from frozen.source_checkpoint_commitment (freeze-time provenance)"
echo "$CODE" | grep -q 'source_point: Point' \
    || fail "the recovery block's source point is not built from the frozen object (frozen.source_slot / source_hash)"

# (3) The recovery block MUST NOT resurrect a seed-window / bootstrap-materialization / seed-fallback /
#     latest-nearest / active-pool-param leadership source, nor re-tick eta0.
declare -A FORBIDDEN=(
    [from_seed_epoch_consensus_inputs]='seed projection / fallback'
    [materialize_bootstrap_into]='bootstrap re-materialization'
    [try_recover_at_boundary]='window-replay recovery'
    [try_activate_at_boundary]='window-replay activation'
    [compute_first_window_bounds]='seed-window bounds'
    [from_accumulator_go]='go + active-pool-param derivation'
    [n_pool_vrfs]='active-pool-param leadership reconstruction'
    [apply_nonce_input]='eta0 re-tick (recovery uses the recovered eta0 AS-IS)'
    [NonceInput::EpochBoundary]='eta0 re-tick (recovery uses the recovered eta0 AS-IS)'
)
for sym in "${!FORBIDDEN[@]}"; do
    if echo "$CODE" | grep -qF "$sym"; then
        fail "the recovery block references '$sym' (${FORBIDDEN[$sym]}) -- a forbidden non-frozen recovery source"
    fi
done

# (4) A NON-promotion (latest/current/nearest) leadership read is forbidden: the ONLY leadership read is the
#     promotion-certified, exact-epoch one. Any bare leadership_authority_for_epoch / *_current / *_nearest /
#     *_latest read in the block is a resurrection.
if echo "$CODE" | grep -Eq '[^_]leadership_authority_for_epoch|leadership_current|leadership_nearest|leadership_latest|latest_leadership|nearest_leadership'; then
    fail "the recovery block performs a non-promotion (latest/current/nearest/bare) leadership read -- promotion-certified exact-epoch ONLY"
fi

# (5) NO WAL write during recovery (the durable record is already authoritative).
if echo "$CODE" | grep -Eq 'wal_write|append_wal|write.*WalEntry'; then
    fail "the recovery block writes a WAL record -- recovery reconstructs from the ALREADY-durable record, it must not create new authority"
fi

# (6) The frozen regime must be `>= seed+2` (covering seed+3+), NOT an `== seed+2` window-replay branch that
#     leaves a seed+3 terminal. A `target_epoch.0 == seed_epoch.0 + 2` guard in the recovery fn is the retired
#     window-replay dispatch -- forbidden.
RECOVERY_FN=$(awk '
    /fn maybe_recover_promoted_authority/ { grab=1 }
    grab { print }
    grab && /^\}$/ { exit }
' "$EW" | sed 's|//.*||')
if echo "$RECOVERY_FN" | grep -qF 'target_epoch.0 == seed_epoch.0 + 2'; then
    fail "maybe_recover_promoted_authority has a `target_epoch == seed+2` branch -- the retired window-replay dispatch (post-S4 recovery is frozen-only, >= seed+2)"
fi
echo "$RECOVERY_FN" | grep -qF 'target_epoch.0 >= seed_epoch.0 + 2' \
    || fail "maybe_recover_promoted_authority has no `target_epoch >= seed+2` frozen branch -- seed+3+ would terminal (the CE-4A.3 gap)"

if [ "$FAILED" -eq 0 ]; then
    echo "OK: warm-start recovery reconstructs the frozen-promoted authority (frozen-only, no seed-window / bootstrap / latest / active-param / re-tick; >= seed+2, no seed+3 terminal)"
fi
exit "$FAILED"
