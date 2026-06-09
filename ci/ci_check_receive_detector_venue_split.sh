#!/usr/bin/env bash
set -uo pipefail

# PHASE4-N-AI AI-S2 — shared receive detector (DC-NODE-23) + venue-split
# resolver (DC-NODE-24). Production code only (test modules stripped).
#
# Guards:
#   1. The detector/resolver surface exists (classify_receive, resolve_disposition,
#      ReceiveClass, ReceiveDisposition, CandidateSummary, VenueRole::Participant).
#   2. classify_receive is venue-BLIND: its signature does NOT reference VenueRole.
#   3. Neither the detector nor the resolver references select_best_chain /
#      chain_selector / fork_choice / ChainDb (DC-CONS-03 stays the chain-selection
#      authority; the detector is GREEN and pure).
#   4. resolve_disposition is total over the closed VenueRole (all three variants).
#   5. The AI-S2 test proves the Participant fast path (only Competing -> fork-choice).
#
# DC-NODE-20 (SingleProducer fail-closed) is covered by
# ci_check_single_producer_extend_own_spine.sh (single_producer_forge_decision
# is untouched by this slice).

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
NS="$REPO_ROOT/crates/ade_node/src/node_sync.rs"
TEST="$REPO_ROOT/crates/ade_node/tests/receive_detector_ai_s2.rs"

FAILED=0
print_fail() { echo "FAIL: $1"; FAILED=1; }
strip_for_grep() {
    awk '
        /^#\[cfg\(test\)\]/ { in_test=1 }
        in_test { next }
        { line=$0; sub(/\/\/.*$/, "", line); print line }
    ' "$1"
}

[[ -f "$NS" ]] || print_fail "missing $NS"
[[ -f "$TEST" ]] || print_fail "missing $TEST"

PROD=$(strip_for_grep "$NS")

# 1. Surface exists.
for needle in \
    'pub struct CandidateSummary' \
    'pub enum ReceiveClass' \
    'pub enum ReceiveDisposition' \
    'pub fn classify_receive' \
    'pub fn resolve_disposition' ; do
    echo "$PROD" | grep -qF "$needle" || print_fail "missing: $needle"
done
echo "$PROD" | grep -qE '^[[:space:]]*Participant,' || print_fail "VenueRole::Participant variant missing"

# 2. Venue-blind detector: classify_receive signature has no VenueRole.
SIG=$(echo "$PROD" | awk '/pub fn classify_receive\(/{f=1} f{print} /-> ReceiveClass/{if(f) exit}')
if echo "$SIG" | grep -qE '\bVenueRole\b'; then
    print_fail "classify_receive signature must NOT reference VenueRole (venue-blind)"
fi

# 3. No chain-selection / ChainDb leakage in the AI-S2 detector/resolver region.
REGION=$(echo "$PROD" | awk '/pub struct CandidateSummary/{f=1} /pub enum ForgeMode/{f=0} f{print}')
for needle in select_best_chain chain_selector fork_choice ChainDb; do
    if echo "$REGION" | grep -qE "\b${needle}\b"; then
        print_fail "AI-S2 detector/resolver must not reference ${needle}"
    fi
done

# 4. resolve_disposition total over the closed VenueRole.
for v in Participant SingleProducer Unknown; do
    echo "$REGION" | grep -qE "VenueRole::${v}\b" || print_fail "resolve_disposition missing VenueRole::${v} arm"
done

# 5. Participant fast path proven (only Competing -> fork-choice).
grep -qE 'do_not_call_fork_choice' "$TEST" || print_fail "AI-S2 test must prove the Participant fast path"

if (( FAILED == 0 )); then
    echo "OK: shared detector (DC-NODE-23) + venue-split resolver (DC-NODE-24)"
fi
exit $FAILED
