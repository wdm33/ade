#!/usr/bin/env bash
set -uo pipefail

# PHASE4-N-AI AI-S4b-i — Participant venue declaration (inert). Production code
# only (test modules stripped).
#
# Guards:
#   1. declare_participant_venue exists and sets VenueRole::Participant.
#   2. It is wired ONLY from `if cli.participant_venue` (no default inference).
#   3. CLI: both venue flags fail closed (ConflictingVenue); participant_venue
#      defaults to false (no inference from absence).
#   4. INERT: node_lifecycle wires NO live fork-choice routing yet
#      (classify_receive / resolve_disposition / process_stream_input are AI-S4b-ii),
#      and the ONLY VenueRole::Participant is the setter assignment (no loop
#      routing branch on Participant).
#   5. The SingleProducer path is unchanged.
#
# NOTE: greps use here-strings (`<<<`), NOT `echo "$VAR" | grep -q`, because the
# latter under `set -o pipefail` reports a false failure when grep -q matches
# early and SIGPIPEs the echo of a large stripped file.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CLI="$REPO_ROOT/crates/ade_node/src/cli.rs"
NL="$REPO_ROOT/crates/ade_node/src/node_lifecycle.rs"

FAILED=0
fail() { echo "FAIL: $1"; FAILED=1; }
strip_for_grep() {
    awk '
        /^#\[cfg\(test\)\]/ { in_test=1 }
        in_test { next }
        { line=$0; sub(/\/\/.*$/, "", line); print line }
    ' "$1"
}

[[ -f "$CLI" ]] || fail "missing $CLI"
[[ -f "$NL" ]] || fail "missing $NL"
CLIP=$(strip_for_grep "$CLI")
NLP=$(strip_for_grep "$NL")

# 1. Setter exists + sets Participant.
grep -qE 'fn declare_participant_venue' <<< "$NLP" || fail "declare_participant_venue missing"
grep -qE 'venue_role = VenueRole::Participant' <<< "$NLP" \
    || fail "declare_participant_venue must set VenueRole::Participant"

# 2. Wired from `if cli.participant_venue` only.
grep -qE 'if cli\.participant_venue' <<< "$NLP" \
    || fail "participant_venue not wired (if cli.participant_venue)"
grep -qE 'activation\.declare_participant_venue\(\)' <<< "$NLP" \
    || fail "declare_participant_venue not called from the wiring"

# 3. CLI fail-closed + default false.
grep -qE 'single_producer_venue && participant_venue' <<< "$CLIP" \
    || fail "both-venues conflict check missing"
grep -qE 'CliError::ConflictingVenue' <<< "$CLIP" || fail "ConflictingVenue error missing"
grep -qE 'let mut participant_venue = false' <<< "$CLIP" \
    || fail "participant_venue must default to false (no inference from absence)"

# 4. INERT: no live routing wired yet; exactly one VenueRole::Participant (the setter).
for needle in classify_receive resolve_disposition process_stream_input; do
    if grep -qF "$needle" <<< "$NLP"; then
        fail "node_lifecycle must not wire ${needle} yet (AI-S4b-ii)"
    fi
done
PCOUNT=$(grep -cE 'VenueRole::Participant' <<< "$NLP")
if [[ "$PCOUNT" -ne 1 ]]; then
    fail "expected exactly 1 VenueRole::Participant (the setter); found $PCOUNT -- a routing branch leaked in"
fi

# 5. SingleProducer path unchanged.
grep -qE 'fn declare_single_producer_venue' <<< "$NLP" || fail "declare_single_producer_venue removed"
grep -qE 'venue_role = VenueRole::SingleProducer' <<< "$NLP" || fail "SingleProducer setter changed"

if (( FAILED == 0 )); then
    echo "OK: Participant venue declaration is explicit, fail-closed, and inert (AI-S4b-i)"
fi
exit $FAILED
