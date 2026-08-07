#!/usr/bin/env bash
set -uo pipefail

# LIVE-2c ACTIVATION -- ONE forge slot authority, and no silent forge tick.
#
# The defect this gates against was measured live: the `--mode node` forge path derived its slot from
# a single `(anchor_millis, start_slot, slot_length_ms)` triple, which cannot express a venue whose
# slot length changed. On preprod that ran 86_400 x (20s - 1s) = 1_641_600 slots (~19 days) fast, and
# NOTHING downstream refused it -- the operator op-cert covered the wrong slot too (period 1018,
# evolution 48, inside a 0..62 window). The only thing that prevented a 19-day-ahead forge was an
# unrelated tip gate.
#
# So this gate asserts STRUCTURE, not merely that tests exist. Re-adding a second conversion,
# re-deriving timing inside the operator-key ingress, letting the CLI choose the venue calendar, or
# restoring B11's silent `Option` skip each fail here even if everything still compiles.
#
# NB (here-string discipline): never `echo "$BIGVAR" | grep -q` under pipefail -- grep exits on first
# match, echo takes SIGPIPE, and the gate flakes on large files. Grep files directly, or `<<<`.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"
FAILED=0
fail() { echo "FAIL: $1"; FAILED=1; }
ok()   { echo "  ok: $1"; }

ERA=crates/ade_core/src/consensus/era_schedule.rs
CLOCK=crates/ade_runtime/src/clock.rs
LIFE=crates/ade_node/src/node_lifecycle.rs
OPF=crates/ade_node/src/operator_forge.rs
TIM=crates/ade_node/src/forge_timing.rs
COORD=crates/ade_runtime/src/producer/coordinator.rs

for f in "$ERA" "$CLOCK" "$LIFE" "$OPF" "$TIM" "$COORD"; do
    [ -f "$f" ] || { fail "missing $f"; echo "RESULT: FAIL"; exit 1; }
done

# Strip `#[cfg(test)]` modules + line comments: the rules below are about PRODUCTION code, and test
# fixtures legitimately mention the shapes being banned.
strip_for_grep() {
    awk '
        /^#\[cfg\(test\)\]/ { in_test=1 }
        in_test { next }
        { line=$0; sub(/\/\/.*$/, "", line); print line }
    ' "$1"
}

echo "== CE-L2c-2: the naive conversion is UNREACHABLE from forging =="

# Removal, not deprecation. A merely-unpreferred second authority is the defect class. Compared
# against the comment-stripped body: the tombstone comment in clock.rs names both symbols on purpose,
# and a gate that cannot tell an explanation from a definition is a gate nobody can keep.
CLOCK_BODY="$(strip_for_grep "$CLOCK")"
if grep -qE 'fn +checked_millis_to_slot' <<< "$CLOCK_BODY"; then
    fail "checked_millis_to_slot is back in $CLOCK -- the node forge path's naive conversion must stay DELETED"
else
    ok "checked_millis_to_slot does not exist"
fi
if grep -qE '\bSlotAlignmentError\b' <<< "$CLOCK_BODY"; then
    fail "SlotAlignmentError is back in $CLOCK"
else
    ok "SlotAlignmentError does not exist"
fi

# `millis_to_slot` survives ONLY for the orchestrator leadership session, whose entry point is
# test-reachable only. It must never appear in the node forge path's production scope.
LIFE_BODY="$(strip_for_grep "$LIFE")"
if grep -qE '\bmillis_to_slot\b' <<< "$LIFE_BODY"; then
    fail "$LIFE references millis_to_slot -- the node forge path must use the bound timing authority only"
else
    ok "the node lifecycle production scope has no millis_to_slot"
fi
OPF_BODY="$(strip_for_grep "$OPF")"
if grep -qE '\bmillis_to_slot\b|anchor_millis|start_slot: *SlotNo\(0\)' <<< "$OPF_BODY"; then
    fail "$OPF re-emits a slot-conversion anchor -- operator KEY ingress must not own timing geometry"
else
    ok "operator-key ingress emits no conversion anchor"
fi

# The single conversion call site. Counted by OCCURRENCE, not by line: `grep -c` counts matching
# lines, so two calls written on one line would read as one and a second slot authority could be
# smuggled in on the same line as the first.
CONV_CALLS="$(grep -oE 'timing\.slot_at\(' <<< "$LIFE_BODY" | wc -l)"
if [ "$CONV_CALLS" -ne 1 ]; then
    fail "expected exactly ONE timing.slot_at call site in $LIFE production scope, found $CONV_CALLS"
else
    ok "exactly one wall-clock->slot conversion call site"
fi

echo "== CE-L2c-A1: the STORE selects the venue calendar, never the operator =="

TIM_BODY="$(strip_for_grep "$TIM")"
grep -qE 'fn +venue_timing_history_for_genesis' <<< "$TIM_BODY" \
    || fail "the calendar must be resolvable BY the durable genesis hash"
grep -qE 'venue_timing_history_for_genesis\(&sidecar\.genesis_hash\)' <<< "$TIM_BODY" \
    || fail "establish_forge_timing_authority must select the calendar by the DURABLE genesis hash"
grep -qE 'NetworkDisagreesWithStore' <<< "$TIM_BODY" \
    || fail "a --network disagreeing with the store must be a terminal error, not a preference"
ok "the venue calendar is store-selected with a fail-closed CLI cross-check"

# The forge-ON path must ESTABLISH the authority; it may not construct an activation without one.
grep -qE 'establish_forge_timing_authority' <<< "$LIFE_BODY" \
    || fail "the --mode node forge arm must establish the bootstrap-bound timing authority"
ok "--mode node establishes the bound timing authority"

echo "== CE-L2c-12 carried: slot derivation stays TIMING-only =="

# `slot_at` must not read era identity or epoch geometry. (The scope guard test proves the behaviour;
# this catches the edit that would make it possible.)
SLOT_AT_BODY="$(awk '/^pub fn slot_at\(/,/^}/' "$ERA")"
if grep -qE '\.era\b|start_epoch|epoch_length_slots|safe_zone_slots|randomness_stabilisation' <<< "$SLOT_AT_BODY"; then
    fail "slot_at reads a NON-TIMING field -- historical era semantics are leaking into slot derivation"
else
    ok "slot_at reads timing fields only"
fi
# The constitutional refusal stays in BOTH inverse directions.
grep -qE 'ScheduleDoesNotCoverSystemStart' <<< "$SLOT_AT_BODY" \
    || fail "slot_at no longer refuses a schedule that misses system start (constitutional guard)"
START_TIME_BODY="$(awk '/^pub fn slot_start_time_ms\(/,/^}/' "$ERA")"
grep -qE 'ScheduleDoesNotCoverSystemStart' <<< "$START_TIME_BODY" \
    || fail "slot_start_time_ms no longer refuses a truncated schedule (the anchor-derivation guard)"
ok "the truncated-schedule refusal holds on both conversion directions"

# The anchor must be derivable ONLY from a bootstrap fact -- no operator-facing timestamp entry.
grep -qE 'fn +derive_for_bootstrap_anchor' "$ERA" \
    || fail "derive_for_bootstrap_anchor is gone -- the anchor domain must come from a bootstrap slot"
ok "the anchor domain is derived from a bootstrap slot"

echo "== CE-L2c-6: B11 -- no admitted tick may disappear into an Option =="

grep -qE 'fn +kes_period_for_slot_checked' "$COORD" \
    || fail "the typed kes_period_for_slot_checked is gone"
# The Option accessor must DELEGATE, so the two can never disagree about which slots are signable.
OPT_BODY="$(awk '/pub fn kes_period_for_slot\(/,/^    }/' "$COORD")"
grep -qE 'kes_period_for_slot_checked\(slot\)\.ok\(\)' <<< "$OPT_BODY" \
    || fail "kes_period_for_slot must delegate to the checked form (one definition, never two)"
ok "one KES-window definition; the Option form delegates"

# The ForgeTick arm must consume the Result. `if let Some(kes_period) = ...kes_period_for_slot(` is
# exactly B11.
if grep -qE 'if let Some\(kes_period\) *= *act\.coordinator_state\.kes_period_for_slot\(' <<< "$LIFE_BODY"; then
    fail "B11 is back: the ForgeTick arm skips on an Option instead of recording a typed refusal"
else
    ok "the ForgeTick arm consumes the typed KES Result"
fi
grep -qE 'ForgeRefused::KesWindow' <<< "$LIFE_BODY" \
    || fail "the ForgeTick arm must record a typed ForgeRefused::KesWindow"
# The three KES reasons must stay separable in the emitted record.
for r in KesBeforeOpcertStart KesAfterOpcertEnd KesPeriodOverflow; do
    grep -qE "R::$r" <<< "$LIFE_BODY" \
        || fail "the emitted skip reasons collapsed: $r is no longer produced"
done
ok "the three KES-window reasons stay distinguishable"

# A refusal may not outlive its own tick.
grep -qE 'act\.last_forge_refused = None;' <<< "$LIFE_BODY" \
    || fail "the per-tick refusal reset is gone -- a stale reason can be re-emitted as this tick's"
ok "refusals are reset per tick"

echo "== the proofs actually RAN (cargo exits 0 on an empty filter) =="

run_and_count() { # run_and_count <pkg> <filter> <min>
    local out passed
    out="$(cargo test -p "$1" --lib "$2" 2>&1)"
    if ! grep -q "test result: ok" <<< "$out"; then
        fail "$1 :: $2 did not pass"
        return
    fi
    passed="$(grep -oE 'test result: ok\. [0-9]+ passed' <<< "$out" | grep -oE '[0-9]+' | head -1)"
    passed="${passed:-0}"
    if [ "$passed" -lt "$3" ]; then
        fail "$1 :: $2 ran only $passed test(s), expected >= $3 -- an empty filter exits 0 and proves nothing"
    else
        ok "$1 :: $2 -- $passed test(s) passed"
    fi
}

run_and_count ade_core live2c_derived_anchor_tests 8
run_and_count ade_node forge_timing:: 7
run_and_count ade_node ce_l2c 3
run_and_count ade_runtime kes_period_for_slot 2

if (( FAILED == 0 )); then
    echo "RESULT: OK -- one forge slot authority; no admitted tick disappears"
else
    echo "RESULT: FAIL"
fi
exit $FAILED
