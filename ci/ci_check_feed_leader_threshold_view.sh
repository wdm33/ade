#!/usr/bin/env bash
set -uo pipefail

# PHASE4-N-F-G-P (DC-CINPUT-04): the feed/receive header-validation view is the RECOVERED consensus surface
#   (the SAME projection the forge uses, PoolDistrView::from_seed_epoch_consensus_inputs) -- never an empty
#   placeholder; fail-closed (FeedMissingRecoveredConsensusInputs) when --peer is set but the recovered record
#   is absent. Forge + feed share ONE consensus surface. NOT eta0, NOT VRF (proven by capture: eta0 correct,
#   verify_ok=true; the failure was Step 7 leader threshold against an empty pool distribution).

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
LIFE="$REPO_ROOT/crates/ade_node/src/node_lifecycle.rs"
SYNC="$REPO_ROOT/crates/ade_node/src/node_sync.rs"
VIEW="$REPO_ROOT/crates/ade_ledger/src/consensus_view.rs"
FORGE_T="$REPO_ROOT/crates/ade_node/tests/forge_succeeds.rs"
REG="$REPO_ROOT/docs/ade-invariant-registry.toml"

FAILED=0
print_fail() { echo "FAIL: $1"; FAILED=1; }

for f in "$LIFE" "$SYNC" "$VIEW" "$FORGE_T" "$REG"; do
    [[ -f "$f" ]] || print_fail "missing expected file $f"
done

# (1) the feed run-loop (node_lifecycle) projects its header-validation view from the recovered record.
grep -Eq 'from_seed_epoch_consensus_inputs' "$LIFE" \
    || print_fail "node_lifecycle.rs does not project the feed ledger_view via from_seed_epoch_consensus_inputs"

# (2) fail-closed when a live feed is wired but the recovered record is absent.
grep -Eq 'FeedMissingRecoveredConsensusInputs' "$LIFE" \
    || print_fail "node_lifecycle.rs has no FeedMissingRecoveredConsensusInputs fail-closed for the feed view"
grep -Eq 'None if live_feed_wired' "$LIFE" \
    || print_fail "node_lifecycle.rs feed view does not fail closed on a missing record when a live feed is wired"

# (3) forge AND feed share ONE projection authority (the forge uses it too) ...
grep -Eq 'from_seed_epoch_consensus_inputs' "$SYNC" \
    || print_fail "node_sync.rs (forge) no longer uses from_seed_epoch_consensus_inputs -- forge/feed surfaces diverged"
# ... and that authority is defined exactly once (the single recovered surface).
DEF_COUNT="$(grep -rE 'fn from_seed_epoch_consensus_inputs' "$REPO_ROOT/crates" --include=*.rs | wc -l | tr -d ' ')"
[[ "$DEF_COUNT" == "1" ]] \
    || print_fail "from_seed_epoch_consensus_inputs must be defined exactly once (the single projection authority); found $DEF_COUNT"

# (4) the pin test exists (recovered view validates Step 5+7; empty view fails closed).
grep -Eq 'fn feed_header_validates_against_recovered_surface_not_empty_view' "$FORGE_T" \
    || print_fail "the G-P header-validation regression pin is missing from forge_succeeds.rs"

# (5) DC-CINPUT-04 present and enforced.
awk '/id = "DC-CINPUT-04"/{f=1} f&&/status = "enforced"/{print "ok"; exit}' "$REG" | grep -q ok \
    || print_fail "DC-CINPUT-04 not present-and-enforced in the registry"

if [[ "$FAILED" -ne 0 ]]; then
    echo "ci_check_feed_leader_threshold_view: FAILED"
    exit 1
fi
echo "ci_check_feed_leader_threshold_view: OK (DC-CINPUT-04 -- feed header-validation view = recovered consensus surface, fail-closed, one surface with the forge)"
