#!/usr/bin/env bash
set -uo pipefail

# PHASE4-N-AI AI-S1 — rollback WAL durability foundation (CE-AI-1 / DC-NODE-27 mechanism).
#
# Mechanical guards (production code only; test modules stripped):
#   1. WalEntry::RollBack uses the reserved tag 1 (TAG_ROLLBACK = 1),
#      NOT a freshly-invented tag.
#   2. The RollBack variant is handled in ALL FOUR exhaustive walks:
#      encode_wal_entry + decode_wal_entry (event.rs), replay_from_anchor
#      (replay.rs), verify_chain (store_trait.rs).
#   3. compute_superseded (the rollback-aware supersede pre-pass) exists.
#   4. NOT a second rollback implementation: the WAL fp-walk (replay.rs /
#      store_trait.rs) is fp-ONLY — it does NOT call
#      materialize_rolled_back_state / apply_block / block_validity. The
#      re-anchor uses the in-chain post_fp map (point_fp), never a
#      recorded rollback fp.
#   5. The re-anchor binds ONLY to_point (`{ to_point, .. }`) — replay
#      never sets the durable tip from selected_tip (no header-only
#      adoption via WAL metadata).
#   6. The AI-S1 test re-invokes the EXISTING materialize authority
#      (materialize_rolled_back_state), proving hard line 4 at the proper
#      layer.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
EVENT_RS="$REPO_ROOT/crates/ade_ledger/src/wal/event.rs"
REPLAY_RS="$REPO_ROOT/crates/ade_ledger/src/wal/replay.rs"
TRAIT_RS="$REPO_ROOT/crates/ade_ledger/src/wal/store_trait.rs"
TEST_RS="$REPO_ROOT/crates/ade_ledger/tests/wal_rollback_ai_s1.rs"

FAILED=0
print_fail() { echo "FAIL: $1"; FAILED=1; }

# Strip `#[cfg(test)]` modules + line comments (same shape as the
# sibling WAL gate) so guards assert on production code only.
strip_for_grep() {
    awk '
        /^#\[cfg\(test\)\]/ { in_test=1 }
        in_test { next }
        { line=$0; sub(/\/\/.*$/, "", line); print line }
    ' "$1"
}

for f in "$EVENT_RS" "$REPLAY_RS" "$TRAIT_RS" "$TEST_RS"; do
    [[ -f "$f" ]] || print_fail "missing $f"
done

EVENT=$(strip_for_grep "$EVENT_RS")
REPLAY=$(strip_for_grep "$REPLAY_RS")
TRAIT=$(strip_for_grep "$TRAIT_RS")

# 1. Reserved tag 1.
if ! echo "$EVENT" | grep -qE 'TAG_ROLLBACK: u64 = 1\b'; then
    print_fail "TAG_ROLLBACK must be the reserved slot 1 (event.rs)"
fi
if echo "$EVENT" | grep -qE 'TAG_ROLLBACK: u64 = 4\b'; then
    print_fail "TAG_ROLLBACK must NOT be 4 (use the reserved tag 1)"
fi

# 2. RollBack handled in all four walks.
if ! echo "$EVENT" | grep -qE 'TAG_ROLLBACK =>'; then
    print_fail "decode_wal_entry has no TAG_ROLLBACK arm (event.rs)"
fi
# encode arm + decode construct both produce `WalEntry::RollBack {`.
if [[ $(echo "$EVENT" | grep -cE 'WalEntry::RollBack \{') -lt 2 ]]; then
    print_fail "event.rs must encode AND decode WalEntry::RollBack"
fi
if ! echo "$REPLAY" | grep -qE 'WalEntry::RollBack \{ to_point, \.\. \} =>'; then
    print_fail "replay_from_anchor has no RollBack re-anchor arm (replay.rs)"
fi
if ! echo "$TRAIT" | grep -qE 'WalEntry::RollBack \{ to_point, \.\. \} =>'; then
    print_fail "verify_chain has no RollBack re-anchor arm (store_trait.rs)"
fi

# 3. Supersede pre-pass exists.
if ! echo "$REPLAY" | grep -qE 'fn compute_superseded\b'; then
    print_fail "compute_superseded missing (replay.rs)"
fi

# 4. fp-ONLY: the WAL fp-walk does not re-implement rollback / materialize.
for needle in materialize_rolled_back_state apply_block block_validity; do
    if echo "$REPLAY" | grep -qE "\b${needle}\b"; then
        print_fail "replay.rs must be fp-only (no ${needle} in the WAL walk)"
    fi
    if echo "$TRAIT" | grep -qE "\b${needle}\b"; then
        print_fail "store_trait.rs must be fp-only (no ${needle} in verify_chain)"
    fi
done
# Re-anchor uses the in-chain post_fp map, not a recorded rollback fp.
if ! echo "$REPLAY" | grep -qE '\bpoint_fp\b'; then
    print_fail "replay.rs RollBack re-anchor must use the in-chain point_fp map"
fi

# 6. The mechanism is proven against the EXISTING materialize authority.
if ! grep -qE '\bmaterialize_rolled_back_state\b' "$TEST_RS"; then
    print_fail "AI-S1 test must re-invoke materialize_rolled_back_state (hard line 4)"
fi
if ! grep -qE 'recovers_selected_not_abandoned' "$TEST_RS"; then
    print_fail "AI-S1 test must prove replay recovers the selected (not abandoned) chain"
fi

# 7. PHASE4-N-AO S5 (DC-NODE-27 ext): RollbackReason::ForkChoiceWin is a closed,
#    wire-coded reason. The replay/verify re-anchor binds `to_point` ONLY (guard #2's
#    `{ to_point, .. }` arms ignore `reason`), so a ForkChoiceWin reselection replays
#    byte-identically to any other RollBack -- reason-agnostic by construction. The
#    S5 replay/crash proofs live in ade_node/tests/reselection_replay_s5.rs.
if ! echo "$EVENT" | grep -qE '\bForkChoiceWin\b'; then
    print_fail "RollbackReason::ForkChoiceWin must be a closed wire-coded reason (event.rs)"
fi
S5_TEST="$REPO_ROOT/crates/ade_node/tests/reselection_replay_s5.rs"
if [[ -f "$S5_TEST" ]]; then
    if ! grep -qE 'forkchoicewin_rollback_without_bodies_is_no_fake_winner' "$S5_TEST"; then
        print_fail "S5 must prove the no-fake-winner case (forkchoicewin_rollback_without_bodies_is_no_fake_winner)"
    fi
    if ! grep -qE 'forkchoicewin_reselection_replays_byte_identical' "$S5_TEST"; then
        print_fail "S5 must prove ForkChoiceWin replay-equivalence (forkchoicewin_reselection_replays_byte_identical)"
    fi
else
    print_fail "S5 reselection replay proof file missing: $S5_TEST"
fi

if (( FAILED == 0 )); then
    echo "OK: rollback WAL durability foundation (CE-AI-1 / DC-NODE-27 mechanism)"
fi
exit $FAILED
