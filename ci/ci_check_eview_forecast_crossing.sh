#!/usr/bin/env bash
set -uo pipefail

# EPOCH-CONTINUITY-ACTIVATION ECA-5 (DC-EPOCH-15): the forecast horizon extends past an epoch boundary
# N->N+1 IFF the ActiveEpochAuthority has durably promoted the N+1 view (DC-EPOCH-14). The extension is
# DERIVED (never persisted), coupled to the promotion (the SAME transition), reconstructed
# byte-identically on warm-start, and free of any flag/clock/peer input. This gate asserts the
# mechanism + the coupling + the proofs.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"; cd "$REPO_ROOT"
FAILED=0; fail() { echo "FAIL: $1"; FAILED=1; }
NL=crates/ade_node/src/node_lifecycle.rs

# (1) the derived-state extension helper exists + takes the TARGET EPOCH (not a flag, not an ambient input).
grep -qF 'fn extend_schedule_to_epoch(era_schedule: &mut EraSchedule, target: EpochNo)' "$NL" \
    || fail "extend_schedule_to_epoch(&mut EraSchedule, EpochNo) (the derived extension) is missing"

# (2) the relay loop OWNS the forecast schedule (clone) + atomically replaces it -- no shared mutable
#     reference can leave validation on the old horizon after promotion.
grep -qF 'let mut era_schedule = era_schedule.clone();' "$NL" \
    || fail "the relay loop does not own the forecast schedule (no clone-to-own)"
grep -qF '*era_schedule = extended;' "$NL" \
    || fail "the extension does not atomically replace the owned schedule"

# (3) COUPLING: the extension is invoked ONLY with the promoted authority's epoch (the in-place
#     promotion site + the warm-start recovery), never pre-extended on an ambient input.
grep -qF 'extend_schedule_to_epoch(era_schedule, authority.epoch());' "$NL" \
    || fail "the in-place promotion does not extend the schedule to the authority's epoch"
grep -qF 'extend_schedule_to_epoch(&mut era_schedule, authority.epoch());' "$NL" \
    || fail "the warm-start recovery does not reconstruct the schedule to the authority's epoch"
# Count only PRODUCTION occurrences (before the test module): 1 def + 2 coupled call sites = 3.
PROD=$(awk '/#\[cfg\(test\)\]/{exit} /extend_schedule_to_epoch\(/{c++} END{print c+0}' "$NL")
if [[ "$PROD" != "3" ]]; then
    fail "extend_schedule_to_epoch must appear exactly 3x in production (1 def + 2 coupled call sites); found $PROD"
fi

# (4) the boundary promotion threads the schedule by &mut (in-place extension within the SAME transition).
grep -qF 'era_schedule: &mut EraSchedule,' "$NL" \
    || fail "maybe_activate_epoch_boundary does not take the schedule by &mut for in-place extension"

# (5) DERIVED from the seed geometry -- no hardcoded epoch length / venue switch / wall-clock / peer / env.
HELPER="$(awk '/fn extend_schedule_to_epoch/{f=1} f{print} f&&/^}/{exit}' "$NL")"
echo "$HELPER" | grep -qF 'seed.epoch_length_slots' \
    || fail "the extension does not derive the N+1 geometry from the seed epoch_length_slots"
if echo "$HELPER" | grep -qE '432000|86400|SystemTime|Instant|env::|getenv|peer'; then
    fail "the extension must not use a hardcoded epoch length / wall-clock / peer / env input"
fi

# (6) the proofs exist (coupling, byte-identical warm-start, adjacent same-era summaries).
for t in \
    forecast_extends_only_on_promotion \
    warmstart_reconstruction_is_byte_identical_to_live_append \
    eraschedule_supports_adjacent_same_era_summaries ; do
    grep -qF "fn $t" "$NL" || fail "the DC-EPOCH-15 proof '$t' is missing"
done

if (( FAILED == 0 )); then
    echo "OK: forecast horizon <=> N+1 authority promotion (DC-EPOCH-15; owned + atomic-replace schedule, derived-not-persisted, coupled to promotion, no ambient input; proofs present)"
fi
exit $FAILED
