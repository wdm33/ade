#!/usr/bin/env bash
set -uo pipefail

# PHASE4-N-F-D S2 — the relay run loop owns NO authority (CN-NODE-02) and
# advances the tip ONLY via run_node_sync -> pump_block (DC-SYNC-02).
#
# The `--mode node` relay loop (`node_lifecycle::run_relay_loop`) composes the
# GREEN planner over exactly one block-consumption path: a `SyncOnce` step
# calls `run_node_sync`, whose `pump_block` makes StoreBlockBytes + AppendWal
# durable BEFORE AdvanceTip. This gate proves the LOOP BODY drives sync only
# through `run_node_sync` and never reaches for a second apply / forge /
# evidence / verdict / follower / manual-tip / second-bootstrap path.
#
# Scope: the `run_relay_loop` function body in
# `crates/ade_node/src/node_lifecycle.rs`, production code only. Line comments
# + the `#[cfg(test)]` module are stripped first, then the function body is
# isolated (from its `pub async fn run_relay_loop` signature to the next
# top-level `^}`), so the greps see ONLY the loop — NOT the dispatcher
# `run_node_lifecycle_inner` (which legitimately calls the bootstrap
# authority + first_run/warm_start arms) and NOT the test module.
#
# Guards (scoped to the run_relay_loop body):
#   (pos)  the loop calls `run_node_sync(`  — the sole block-consumption path;
#   (neg1) no direct durable-apply / manual tip advance: no `pump_block(`
#          (must go THROUGH run_node_sync, not directly), `.put_block(`,
#          `AdvanceTip`, `rollback_to_slot(`;
#   (neg2) no forge / evidence: no `run_real_forge` / `forge_one_from_recovered`
#          / `correlate(` / `Ba02Manifest`;
#   (neg3) no verdict-as-sync / follower-as-sync: no `derive_verdict` /
#          `run_admission(` / `ade_core_interop` / `follow(`;
#   (neg4) no second bootstrap / recovery on the loop path: no
#          `bootstrap_initial_state(` / `bootstrap_from_` / `warm_start_recovery(`.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OWNER="$REPO_ROOT/crates/ade_node/src/node_lifecycle.rs"

FAILED=0
print_fail() { echo "FAIL (node run-loop containment): $1"; FAILED=1; }

if [[ ! -f "$OWNER" ]]; then
    echo "FAIL (node run-loop containment): lifecycle owner not found at $OWNER"
    echo "FAIL: ci_check_node_run_loop_containment"
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

# Isolate the run_relay_loop body: from the `pub async fn run_relay_loop`
# signature line up to (and including) the next top-level `^}`.
isolate_loop_fn() {
    strip_for_grep "$1" | awk '
        /pub async fn run_relay_loop/ { capture=1 }
        capture { print }
        capture && /^}/ { exit }
    '
}

LOOP_FN="$(isolate_loop_fn "$OWNER")"

if [[ -z "$LOOP_FN" ]]; then
    print_fail "could not isolate run_relay_loop body (signature moved/renamed?)"
    echo "FAIL: ci_check_node_run_loop_containment"
    exit 1
fi

# --- guard (pos): the loop drives sync via run_node_sync --------------------
if ! echo "$LOOP_FN" | grep -qE 'run_node_sync\('; then
    print_fail "run_relay_loop must call run_node_sync( — the sole block-consumption / tip-advance path (DC-SYNC-02)"
fi

# --- guard (neg1): no direct durable-apply / manual tip advance -------------
for tok in 'pump_block\(' '\.put_block\(' 'AdvanceTip' 'rollback_to_slot\('; do
    if echo "$LOOP_FN" | grep -qE "$tok"; then
        print_fail "run_relay_loop reaches a tip-mutation token directly: $tok — the tip advances ONLY through run_node_sync (CN-NODE-02 / DC-SYNC-02)"
    fi
done

# --- guard (neg2): no forge / evidence on the loop path ---------------------
for tok in 'run_real_forge' 'forge_one_from_recovered' 'correlate\(' 'Ba02Manifest'; do
    if echo "$LOOP_FN" | grep -qE "$tok"; then
        print_fail "run_relay_loop references a forge/evidence token: $tok — relay-only; the loop owns no forge/evidence authority (CN-NODE-02)"
    fi
done

# --- guard (neg3): no verdict-as-sync / follower-as-sync --------------------
for tok in 'derive_verdict' 'run_admission\(' 'ade_core_interop' 'follow\('; do
    if echo "$LOOP_FN" | grep -qE "$tok"; then
        print_fail "run_relay_loop references a verdict/follower token: $tok — tip-agreement is not validating sync (DC-SYNC-02)"
    fi
done

# --- guard (neg4): no second bootstrap / recovery on the loop path ----------
for tok in 'bootstrap_initial_state\(' 'bootstrap_from_' 'warm_start_recovery\('; do
    if echo "$LOOP_FN" | grep -qE "$tok"; then
        print_fail "run_relay_loop references a bootstrap/recovery token: $tok — initial state is obtained ONCE by the dispatcher before the loop (CN-NODE-01/02)"
    fi
done

if (( FAILED == 0 )); then
    echo "OK (node run-loop containment): run_relay_loop drives sync only via run_node_sync — no direct pump_block / manual tip advance, no forge/evidence, no verdict/follower, no second bootstrap (CN-NODE-02 / DC-SYNC-02)"
fi
exit $FAILED
