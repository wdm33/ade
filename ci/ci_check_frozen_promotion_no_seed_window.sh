#!/usr/bin/env bash
set -uo pipefail

# LIVE-LEDGER-EPOCH-TRANSITION S4-L2 (DC-EPOCH-25, seed-authority resurrection guard): candidate leadership
# BEYOND the bootstrap bridge (>= seed+2) is promoted SOLELY from a promotion-certified, epoch-indexed
# FrozenLeadershipPoolDistr -- never a seed-window replay, a bootstrap re-materialization, a live-checkpoint
# shortcut, or a fabricated (zero) checkpoint commitment. The retired window-replay ceiling (the candidate ==
# seed+2 cap) is GONE; EVERY boundary past the bridge crosses through the frozen object. This gate keeps the
# forward promotion path mechanically frozen-only, so a future edit cannot resurrect the seed authority as the
# forward promotion source.
#
# This is the guard ci_check_frozen_leadership_authority.sh deferred to S4 ("belongs to S4"). It targets the
# FORWARD promotion block ONLY -- NOT the warm-start recovery, which legitimately re-derives the seed+2 twin as
# a RECOVERY identity check (that path is S5's, guarded elsewhere). Repo-root-relative; mirrors the sibling
# ci_check_*.sh gates.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"; cd "$REPO_ROOT"
FAILED=0; fail() { echo "FAIL: $1"; FAILED=1; }

EW='crates/ade_node/src/epoch_wire.rs'
STORE='crates/ade_runtime/src/chaindb/epoch_accumulator_store.rs'

# (1) Isolate the FORWARD promotion block: the `candidate >= seed+2` branch of
#     prepare_authority_for_candidate_slot. Extract from the branch head to its 4-space-indented close (the
#     block body is 8-space-indented, so the first `^    }` is the branch close, never a nested one).
BLOCK=$(awk '
    /if candidate_epoch\.0 >= seed_epoch\.0 \+ 2 \{/ { grab=1 }
    grab { print }
    grab && /^    \}$/ { exit }
' "$EW")
if [ -z "$BLOCK" ]; then
    fail "could not locate the candidate>=seed+2 promotion block in $EW (renamed / restructured? the guard anchor is broken)"
fi

# CODE = the block with comments stripped (the block's OWN doc comment enumerates the forbidden symbols as
# negations -- "NO materialize_bootstrap_into ..." -- so the resurrection check must see executable code only).
# The block contains no `//` inside string literals, so a blanket after-`//` strip is safe here.
CODE=$(echo "$BLOCK" | sed 's|//.*||')

# (2) The block MUST source candidate leadership from the promotion-certified frozen reader, and the checkpoint
#     commitment MUST come from the frozen object itself (freeze-time provenance) -- never re-derived / fabricated.
echo "$CODE" | grep -q 'promotion_leadership_authority_for_epoch(candidate_epoch)' \
    || fail "the promotion block does not read promotion_leadership_authority_for_epoch(candidate_epoch) (the SOLE candidate leadership source)"
echo "$CODE" | grep -q 'checkpoint_commitment: frozen\.source_checkpoint_commitment' \
    || fail "the promotion block's checkpoint commitment is not sourced from frozen.source_checkpoint_commitment (freeze-time provenance)"

# (3) The block's CODE MUST NOT resurrect any seed-window / bootstrap-materialization / live-shortcut leadership
#     source, nor fabricate a zero Hash32 (commitment / nonce).
declare -A FORBIDDEN=(
    [from_seed_epoch_consensus_inputs]='seed projection'
    [materialize_bootstrap_into]='bootstrap re-materialization'
    [try_activate_at_boundary]='window-replay activation'
    [compute_first_window_bounds]='seed-window bounds'
    [from_accumulator_go]='go+active-params derivation'
)
for sym in "${!FORBIDDEN[@]}"; do
    if echo "$CODE" | grep -q "$sym"; then
        fail "the promotion block references '$sym' (${FORBIDDEN[$sym]}) -- a resurrected seed-window leadership source"
    fi
done
if echo "$CODE" | grep -qE 'Hash32\(\[0(u8)?; ?32\]\)'; then
    fail "the promotion block constructs a zero Hash32 (a fabricated commitment / nonce) -- forbidden on the frozen promotion path"
fi

# (4) The store enforces promotion-certification mechanically: the epoch-indexed promotion reader exists and its
#     rejection variant is present (current-present AND bootstrap-absent, else NotPromotionCertified).
grep -qE 'pub fn promotion_leadership_authority_for_epoch' "$STORE" \
    || fail "the store is missing the promotion-certified reader 'promotion_leadership_authority_for_epoch'"
grep -qE 'NotPromotionCertified' "$STORE" \
    || fail "the store is missing the 'NotPromotionCertified' rejection (the promotion-certification gate)"

# (5) The rule is declared in the invariant registry.
REG="docs/ade-invariant-registry.toml"
grep -q 'DC-EPOCH-25' "$REG" \
    || fail "DC-EPOCH-25 is not declared in the invariant registry ($REG)"

if (( FAILED == 0 )); then
    echo "OK: frozen promotion (S4-L2) -- candidate leadership beyond the bridge is promoted ONLY from the promotion-certified epoch-indexed FrozenLeadershipPoolDistr; no seed-window replay / bootstrap re-materialization / fabricated commitment; the seed+2 ceiling is gone."
fi
exit $FAILED
