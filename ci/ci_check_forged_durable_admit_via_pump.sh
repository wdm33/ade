#!/usr/bin/env bash
set -uo pipefail

# PHASE4-N-U S1 — own-forged blocks become durable ONLY through the fenced
# admit_forged_block_durably driver, which routes through forward_sync::pump_block
# (DC-NODE-12). The forge advances no durable tip directly; the driver feeds the
# EXACT self-accepted bytes (no re-encode, I-10) into the SAME pump_block
# chokepoint received blocks use, and adds NO admit-time fork-choice (DC-CONS-23:
# the durable admit is extend-only; a stale-tip forge fails closed inside
# pump_block via block_validity / prior_fp, never an own-block override).
#
# Scope: the admit_forged_block_durably function body in
# crates/ade_node/src/node_sync.rs, production code only (the #[cfg(test)] module
# + line comments are stripped, then the fn body is isolated from its
# `pub fn admit_forged_block_durably` signature to the next top-level `^}`).
#
# Guards (scoped to the driver body):
#   (pos)  routes through pump_block( — the single durable apply engine;
#   (pos)  feeds the self-accepted bytes via .accepted() + .as_bytes() (I-10:
#          no re-encode / reserialize between self_accept and durable admit);
#   (neg1) no admit-time fork-choice: no select_best_chain / fork_choice;
#   (neg2) no re-encode / re-serialize: no encode_block / ade_encode /
#          encode_block_envelope / wrap_tag24;
#   (neg3) no manual tip advance outside pump_block: no .put_block( /
#          AdvanceTip / rollback_to_slot(.
# Plus (call-site): the ForgeTick arm admits forged blocks via this driver
# (the run-loop containment / 2nd-tip-advancer pos is in
# ci_check_node_run_loop_containment.sh).

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SYNC="$REPO_ROOT/crates/ade_node/src/node_sync.rs"
LOOP="$REPO_ROOT/crates/ade_node/src/node_lifecycle.rs"

FAILED=0
print_fail() { echo "FAIL (forged durable admit via pump): $1"; FAILED=1; }

if [[ ! -f "$SYNC" ]]; then
    echo "FAIL (forged durable admit via pump): node_sync not found at $SYNC"
    echo "FAIL: ci_check_forged_durable_admit_via_pump"
    exit 1
fi

# Strip the `#[cfg(test)]` module (attribute to EOF) + line comments.
strip_for_grep() {
    awk '
        /^#\[cfg\(test\)\]/ { in_test=1 }
        in_test { next }
        { line=$0; sub(/\/\/.*$/, "", line); print line }
    ' "$1"
}

# Isolate the admit_forged_block_durably function body: from its `pub fn`
# signature line up to (and including) the next top-level closing brace.
isolate_admit_fn() {
    strip_for_grep "$1" | awk '
        /pub fn admit_forged_block_durably/ { capture=1 }
        capture { print }
        capture && /^}/ { exit }
    '
}

ADMIT_FN="$(isolate_admit_fn "$SYNC")"

if [[ -z "$ADMIT_FN" ]]; then
    print_fail "could not isolate admit_forged_block_durably body in node_sync.rs (signature moved/renamed?)"
    echo "FAIL: ci_check_forged_durable_admit_via_pump"
    exit 1
fi

# --- guard (pos): routes through pump_block ---------------------------------
if ! echo "$ADMIT_FN" | grep -qE 'pump_block\('; then
    print_fail "admit_forged_block_durably must call pump_block( — own-forged blocks become durable ONLY through the single durable apply engine (DC-NODE-12)"
fi

# --- guard (pos): feeds the self-accepted bytes (I-10, no re-encode) --------
if ! echo "$ADMIT_FN" | grep -qE 'accepted\(\)'; then
    print_fail "admit_forged_block_durably must feed handoff.accepted() — the BLUE self-accepted token (CN-FORGE-01 / I-10)"
fi
if ! echo "$ADMIT_FN" | grep -qE 'as_bytes\(\)'; then
    print_fail "admit_forged_block_durably must feed accepted().as_bytes() — the EXACT self-accepted bytes, no re-encode (I-10)"
fi

# --- guard (neg1): no admit-time fork-choice (DC-CONS-23) -------------------
for tok in 'select_best_chain' 'fork_choice'; do
    if echo "$ADMIT_FN" | grep -qE "$tok"; then
        print_fail "admit_forged_block_durably references an admit-time fork-choice token: $tok — the durable admit is EXTEND-ONLY; a stale-tip forge fails closed inside pump_block (DC-CONS-23)"
    fi
done

# --- guard (neg2): no re-encode / re-serialize -----------------------------
for tok in 'encode_block' 'ade_encode' 'encode_block_envelope' 'wrap_tag24'; do
    if echo "$ADMIT_FN" | grep -qE "$tok"; then
        print_fail "admit_forged_block_durably references a re-encode token: $tok — the durably-admitted bytes ARE the self-accepted bytes (I-10), never re-serialized"
    fi
done

# --- guard (neg3): no manual tip advance outside pump_block -----------------
for tok in '\.put_block\(' 'AdvanceTip' 'rollback_to_slot\('; do
    if echo "$ADMIT_FN" | grep -qE "$tok"; then
        print_fail "admit_forged_block_durably performs a manual tip-advance token outside pump_block: $tok — the tip advances ONLY via pump_block's apply_plan (DC-SYNC-01)"
    fi
done

# --- call-site: the ForgeTick arm admits forged blocks via this driver ------
if [[ -f "$LOOP" ]]; then
    if ! grep -qE 'admit_forged_block_durably\(' "$LOOP"; then
        print_fail "node_lifecycle.rs must admit forged blocks via admit_forged_block_durably( — the ForgeTick arm routes the self-accepted handoff through the fenced driver (DC-NODE-12)"
    fi
fi

if (( FAILED == 0 )); then
    echo "OK (forged durable admit via pump): admit_forged_block_durably routes the self-accepted forged bytes (no re-encode, I-10) through pump_block (DC-NODE-12), adds no admit-time fork-choice (DC-CONS-23, extend-only), no manual tip advance; the ForgeTick arm admits via this driver"
fi
exit $FAILED
