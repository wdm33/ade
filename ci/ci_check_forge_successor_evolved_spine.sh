#!/usr/bin/env bash
set -uo pipefail

# PHASE4-N-F-G-Q (DC-NODE-10): the forge-successor derives its header position (block_no) + self-accept chain
#   state from the EVOLVED admitted node-spine state (state.receive), NOT the stale WarmStart baseline.
#   No guessed block_no, no unwrap_or(1), no synthetic numbering -- the fail-closed is RecoveredTipMissingBlockNo.
#   The ChainBreak-on-restart durability is a SEPARATE N-U slice (not addressed here).

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SYNC="$REPO_ROOT/crates/ade_node/src/node_sync.rs"
LIFE="$REPO_ROOT/crates/ade_node/src/node_lifecycle.rs"
REG="$REPO_ROOT/docs/ade-invariant-registry.toml"

FAILED=0
print_fail() { echo "FAIL: $1"; FAILED=1; }

for f in "$SYNC" "$LIFE" "$REG"; do
    [[ -f "$f" ]] || print_fail "missing expected file $f"
done

# (1) forge_one_from_recovered takes the EVOLVED chain state explicitly (live_chain_dep + live_ledger).
grep -Eq 'live_chain_dep: &PraosChainDepState' "$SYNC" \
    || print_fail "forge_one_from_recovered does not take the evolved live_chain_dep"
grep -Eq 'live_ledger: &LedgerState' "$SYNC" \
    || print_fail "forge_one_from_recovered does not take the evolved live_ledger"

# (2) the forge-successor position reads the EVOLVED chain_dep's block_no ...
grep -Eq 'forge_header_position\(selected_tip, live_chain_dep\.last_block_no\)' "$SYNC" \
    || print_fail "forge_header_position no longer reads live_chain_dep.last_block_no (the evolved block_no)"
# ... and the STALE baseline read is GONE on the forge path.
grep -Eq 'forge_header_position\(selected_tip, recovered\.chain_dep\.last_block_no\)' "$SYNC" \
    && print_fail "forge_header_position still reads the STALE recovered.chain_dep.last_block_no baseline"

# (3) the relay loop threads the EVOLVED spine (state.receive) into the forge call.
grep -Eq '&state\.receive\.chain_dep' "$LIFE" \
    || print_fail "node_lifecycle does not thread the evolved state.receive.chain_dep into the forge"
grep -Eq '&state\.receive\.ledger' "$LIFE" \
    || print_fail "node_lifecycle does not thread the evolved state.receive.ledger into the forge"

# (4) the successor block_no fail-closes (RecoveredTipMissingBlockNo via ok_or) -- no guessed/synthetic number.
grep -Eq 'ok_or\(NodeForgeError::RecoveredTipMissingBlockNo\)' "$SYNC" \
    || print_fail "forge_header_position lost its RecoveredTipMissingBlockNo fail-closed (ok_or) -- a guess may have crept in"

# (5) the pin test exists (evolved block_no read -> not RecoveredTipMissingBlockNo; stale None -> RecoveredTipMissingBlockNo).
grep -Eq 'fn forge_successor_reads_evolved_spine_block_no_not_stale_baseline_g_q' "$SYNC" \
    || print_fail "the G-Q forge-successor regression pin is missing"

# (6) DC-NODE-10 present and enforced.
awk '/id = "DC-NODE-10"/{f=1} f&&/status = "enforced"/{print "ok"; exit}' "$REG" | grep -q ok \
    || print_fail "DC-NODE-10 not present-and-enforced in the registry"

if [[ "$FAILED" -ne 0 ]]; then
    echo "ci_check_forge_successor_evolved_spine: FAILED"
    exit 1
fi
echo "ci_check_forge_successor_evolved_spine: OK (DC-NODE-10 -- forge-successor from the evolved admitted spine, no guessed block_no)"
