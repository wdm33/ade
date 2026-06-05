#!/usr/bin/env bash
set -uo pipefail

# PHASE4-N-F-D S2 / PHASE4-N-U S1 — the relay run loop owns NO authority
# (CN-NODE-02) and advances the durable tip ONLY through the fenced drivers
# `run_node_sync` (received blocks, DC-SYNC-02) and `admit_forged_block_durably`
# (own-forged blocks, DC-NODE-12) — each routes through `forward_sync::pump_block`,
# which makes StoreBlockBytes + AppendWal durable BEFORE AdvanceTip. The loop
# body NEVER mutates the tip directly.
#
# The `--mode node` relay loop (`node_lifecycle::run_relay_loop_with_sched`)
# composes the GREEN planner over: a `SyncOnce` step → `run_node_sync`, and a
# `ForgeTick` step → exactly one fenced `forge_one_from_recovered` whose
# self-accepted handoff is durably admitted via `admit_forged_block_durably`
# (DC-NODE-12) and then served (G-R). This gate proves the LOOP BODY advances
# the tip ONLY through those two fenced drivers and never reaches for a direct
# apply / manual-tip / evidence / verdict / follower / second-bootstrap path.
# (The forged driver's internal routing — pump_block, byte-identity, no
# fork-choice — is fenced separately by ci_check_forged_durable_admit_via_pump.sh.)
#
# Scope: the `run_relay_loop_with_sched` function body in
# `crates/ade_node/src/node_lifecycle.rs`, production code only. Line comments
# + the `#[cfg(test)]` module are stripped first, then the function body is
# isolated (from its `pub async fn run_relay_loop_with_sched` signature to the next
# top-level `^}`), so the greps see ONLY the loop — NOT the dispatcher
# `run_node_lifecycle_inner` (which legitimately calls the bootstrap
# authority + first_run/warm_start arms) and NOT the test module.
#
# Guards (scoped to the run_relay_loop_with_sched body):
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

# Isolate the run_relay_loop_with_sched body: from the `pub async fn run_relay_loop_with_sched`
# signature line up to (and including) the next top-level `^}`.
isolate_loop_fn() {
    strip_for_grep "$1" | awk '
        /pub async fn run_relay_loop_with_sched/ { capture=1 }
        capture { print }
        capture && /^}/ { exit }
    '
}

LOOP_FN="$(isolate_loop_fn "$OWNER")"

if [[ -z "$LOOP_FN" ]]; then
    print_fail "could not isolate run_relay_loop_with_sched body (signature moved/renamed?)"
    echo "FAIL: ci_check_node_run_loop_containment"
    exit 1
fi

# --- guard (pos): received sync drives the tip via run_node_sync ------------
if ! echo "$LOOP_FN" | grep -qE 'run_node_sync\('; then
    print_fail "run_relay_loop_with_sched must call run_node_sync( — the received-block durable tip-advance driver (DC-SYNC-02)"
fi

# --- guard (pos): own-forged blocks become durable via the fenced driver ----
# PHASE4-N-U S1 (DC-NODE-12): a self-accepted forged block is admitted to the
# durable tip ONLY through admit_forged_block_durably, which routes through the
# SAME pump_block chokepoint (durable-before-tip). Its routing + byte-identity +
# no-fork-choice is fenced by ci_check_forged_durable_admit_via_pump.sh.
if ! echo "$LOOP_FN" | grep -qE 'admit_forged_block_durably\('; then
    print_fail "run_relay_loop_with_sched must admit a forged block via admit_forged_block_durably( — the own-forged durable tip-advance driver (DC-NODE-12)"
fi

# --- guard (neg1): no direct durable-apply / manual tip advance -------------
# The loop advances the tip ONLY through the two fenced drivers (run_node_sync,
# admit_forged_block_durably); each wraps pump_block in node_sync.rs. The loop
# body itself must never call pump_block directly or mutate the tip.
for tok in 'pump_block\(' '\.put_block\(' 'AdvanceTip' 'rollback_to_slot\('; do
    if echo "$LOOP_FN" | grep -qE "$tok"; then
        print_fail "run_relay_loop_with_sched reaches a tip-mutation token directly: $tok — the tip advances ONLY through the fenced drivers run_node_sync / admit_forged_block_durably (CN-NODE-02 / DC-SYNC-02 / DC-NODE-12)"
    fi
done

# --- guard (neg2): forge is the ONE fenced self-accept call -----------------
# PHASE4-N-F-E S2: the loop may make EXACTLY ONE fenced forge call —
# `forge_one_from_recovered` (self-accept-only; recovered-surface leadership via
# guard (d) of ci_check_consensus_input_provenance.sh; advances no tip). Every
# other forge/evidence token stays forbidden, and the fenced call is the SOLE
# forge entry — no direct `run_real_forge`, no evidence correlation.
for tok in 'run_real_forge' 'correlate\(' 'Ba02Manifest'; do
    if echo "$LOOP_FN" | grep -qE "$tok"; then
        print_fail "run_relay_loop_with_sched references a forbidden forge/evidence token: $tok — forge ONLY via the fenced forge_one_from_recovered; no direct run_real_forge / evidence (CN-NODE-02)"
    fi
done

# The fenced forge call must appear EXACTLY ONCE — the single permitted
# self-accept forge attempt (CE-E-4). More than one is a second forge path;
# zero means the forge branch is not wired.
FORGE_CALLS=$(echo "$LOOP_FN" | grep -cE 'forge_one_from_recovered\(')
if (( FORGE_CALLS != 1 )); then
    print_fail "run_relay_loop_with_sched has $FORGE_CALLS forge_one_from_recovered( call(s) — exactly one fenced forge attempt is required (CE-E-4)"
fi

# --- guard (neg2b): no direct serve / broadcast / gossip in the loop body ---
# PHASE4-N-U S1: the forged block IS durably admitted (via the fenced
# admit_forged_block_durably driver -> pump_block, DC-NODE-12). PHASE4-N-U S3:
# the durable block is what the serve task PROJECTS (DC-NODE-13) — the G-R
# push sibling + tx.send serve handoff are retired. The loop body itself never
# serves/broadcasts/gossips directly — no served_chain_admit / push_atomic /
# broadcast / block_fetch / OutboundCommand here (serve is run_node_serve_task).
for tok in 'served_chain_admit' 'push_atomic' 'OutboundCommand' 'broadcast' 'block_fetch'; do
    if echo "$LOOP_FN" | grep -qE "$tok"; then
        print_fail "run_relay_loop_with_sched references a serve/broadcast token: $tok — a forged block is self-accept-only; the loop serves/admits/gossips nothing (CE-E-4)"
    fi
done

# --- guard (neg3): no verdict-as-sync / follower-as-sync --------------------
for tok in 'derive_verdict' 'run_admission\(' 'ade_core_interop' 'follow\('; do
    if echo "$LOOP_FN" | grep -qE "$tok"; then
        print_fail "run_relay_loop_with_sched references a verdict/follower token: $tok — tip-agreement is not validating sync (DC-SYNC-02)"
    fi
done

# --- guard (neg4): no second bootstrap / recovery on the loop path ----------
for tok in 'bootstrap_initial_state\(' 'bootstrap_from_' 'warm_start_recovery\('; do
    if echo "$LOOP_FN" | grep -qE "$tok"; then
        print_fail "run_relay_loop_with_sched references a bootstrap/recovery token: $tok — initial state is obtained ONCE by the dispatcher before the loop (CN-NODE-01/02)"
    fi
done

if (( FAILED == 0 )); then
    echo "OK (node run-loop containment): run_relay_loop_with_sched advances the tip only via the fenced drivers run_node_sync (received) + admit_forged_block_durably (own-forged, DC-NODE-12), forges via exactly one fenced forge_one_from_recovered, no direct pump_block/put_block/AdvanceTip/rollback, no run_real_forge/evidence, no verdict/follower, no second bootstrap (CN-NODE-02 / DC-SYNC-02 / DC-NODE-12)"
fi
exit $FAILED
