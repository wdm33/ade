#!/usr/bin/env bash
set -uo pipefail

# PHASE4-N-F-G-R (DC-NODE-11): the node-level serve gate (serve_gate_admits) admits a self-accepted forge
#   handoff to the ServedChainView ONLY when its block_no strictly exceeds the highest already-served -- so the
#   first genesis-successor block 0 is served STABLY and the hermetic forge's block-0 re-forges are NOT
#   re-served. No durable own-tip advance; no bypass of self_accept; the forge + served_chain_admit unchanged.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
LIFE="$REPO_ROOT/crates/ade_node/src/node_lifecycle.rs"
FORGE_T="$REPO_ROOT/crates/ade_node/tests/forge_succeeds.rs"
REG="$REPO_ROOT/docs/ade-invariant-registry.toml"

FAILED=0
print_fail() { echo "FAIL: $1"; FAILED=1; }

for f in "$LIFE" "$FORGE_T" "$REG"; do
    [[ -f "$f" ]] || print_fail "missing expected file $f"
done

# (1) the pure monotone-block_no serve gate exists.
grep -Eq 'pub fn serve_gate_admits' "$LIFE" \
    || print_fail "serve_gate_admits (the monotone-block_no serve gate) missing from node_lifecycle.rs"

# (2) the serve sibling USES the gate (does not blindly push_atomic every handoff) + tracks the highest served.
grep -Eq 'serve_gate_admits\(highest_served_block_no' "$LIFE" \
    || print_fail "the serve sibling does not gate via serve_gate_admits(highest_served_block_no, ...)"
grep -Eq 'highest_served_block_no = Some' "$LIFE" \
    || print_fail "the serve sibling does not advance highest_served_block_no after a served push"

# (3) pin tests exist (the gate decision + the served-view-holds-one-block-0 integration).
grep -Eq 'fn serve_gate_admits_first_block_zero_then_skips_reforged_block_zero' "$LIFE" \
    || print_fail "the serve-gate decision pin is missing from node_lifecycle.rs"
grep -Eq 'fn serve_gate_keeps_first_block_zero_skips_reforge' "$FORGE_T" \
    || print_fail "the served-view (two block-0 -> one served) pin is missing from forge_succeeds.rs"

# (4) DC-NODE-11 present and enforced.
awk '/id = "DC-NODE-11"/{f=1} f&&/status = "enforced"/{print "ok"; exit}' "$REG" | grep -q ok \
    || print_fail "DC-NODE-11 not present-and-enforced in the registry"

if [[ "$FAILED" -ne 0 ]]; then
    echo "ci_check_served_chain_stability: FAILED"
    exit 1
fi
echo "ci_check_served_chain_stability: OK (DC-NODE-11 -- stable served block 0 via the monotone serve gate)"
