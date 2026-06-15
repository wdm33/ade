#!/usr/bin/env bash
set -uo pipefail

# MEM-OPT-UTXO-DISK S0 (CE-UD-0) -- active-admission owned-footprint diagnostic.
# VACUOUS-UNTIL-COMMITTED. Validates two committed artifacts when present:
#
#   1. the phase-timeline transcript -- a memory transcript that carries the FOUR
#      phase points (t1 seed_import, t2_snapshot_serializing, t3_after_forced_
#      allocator_collect_diagnostic_only, t4 sustained) + replay verdict agreed;
#   2. the classification record -- an explicit classification in the closed set
#      {serialization_transient, live_working_set, mixed} + a next-slice
#      recommendation, AND consistent with the t2->t3 owned reclaim the numbers
#      show (an honest classification cannot contradict its own data).
#
# Classification rule (the decisive control is the t2->t3 RssAnon reclaim, since
# mimalloc's lazy MADV_FREE keeps freed pages resident until a forced collect):
#   reclaimed_pct = (t2_rss_anon - t3_rss_anon) * 100 / t2_rss_anon   (clamp >=0)
#     serialization_transient  <=>  reclaimed_pct >= 40   (most of it returned)
#     live_working_set         <=>  reclaimed_pct <= 15   (little returned)
#     mixed                    <=>  15 < reclaimed_pct < 40
#
# Run `--self-test` to validate the validator against temp fixtures.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
EV_DIR="$REPO_ROOT/docs/evidence"
TL_JSONL="$EV_DIR/mem-opt-utxo-disk-s0-phase-timeline-preprod.jsonl"
TL_MD="$EV_DIR/mem-opt-utxo-disk-s0-phase-timeline-preprod.md"
CL_JSONL="$EV_DIR/mem-opt-utxo-disk-s0-classification.jsonl"
CL_MD="$EV_DIR/mem-opt-utxo-disk-s0-classification.md"

FAILED=0
fail() { echo "FAIL: $1"; FAILED=1; }

CLASSES='^(serialization_transient|live_working_set|mixed)$'

# num <file> <key> : echo the integer value of "key":N (first match), or empty.
num() { grep -oE "\"$2\":[0-9]+" "$1" | head -1 | sed -E 's/.*://'; }
# str <file> <key> : echo the [a-z_] value of "key":"v" (first match), or empty.
str() { grep -oE "\"$2\":\"[a-z_]+\"" "$1" | head -1 | sed -E 's/.*:"([a-z_]+)"/\1/'; }

# band <t2> <t3> : echo the classification band the t2->t3 RssAnon reclaim implies.
band() {
    local t2="$1" t3="$2" pct
    pct=$(awk -v a="$t2" -v b="$t3" 'BEGIN{ if (a<=0){print -1; exit} d=a-b; if(d<0)d=0; printf "%d", (d*100)/a }')
    if   (( pct < 0  )); then echo invalid
    elif (( pct >= 40 )); then echo serialization_transient
    elif (( pct <= 15 )); then echo live_working_set
    else echo mixed
    fi
}

validate_timeline() {
    local jsonl="$1" md="$2" rc=0
    [[ -f "$jsonl" ]] || return 0   # vacuous-until-committed

    # all FOUR phase points present.
    for p in seed_import t2_snapshot_serializing t3_after_forced_allocator_collect_diagnostic_only sustained; do
        grep -qE "\"point\":\"$p\"" "$jsonl" || { echo "  phase point missing: $p"; rc=1; }
    done
    # the t3 point carries an owned RssAnon sample (the diagnostic's decisive number).
    grep -E '"point":"t3_after_forced_allocator_collect_diagnostic_only"' "$jsonl" \
        | grep -qE '"rss_anon_kib":[0-9]+' || { echo "  t3 point has no rss_anon_kib"; rc=1; }
    # replay verdict agreed (the diagnostic did not perturb authority).
    grep -E '"event":"memory_summary"' "$jsonl" | grep -qE '"replay_verdict":"agreed"' \
        || { echo "  timeline memory_summary replay_verdict is not agreed"; rc=1; }
    grep -qE 'diverged' "$jsonl" && { echo "  a diverged appears in the timeline"; rc=1; }
    # sha256 binding.
    if [[ -f "$md" ]]; then
        local sum; sum=$(sha256sum "$jsonl" | cut -d' ' -f1)
        grep -qF "$sum" "$md" || { echo "  timeline .md does not bind the .jsonl sha256 ($sum)"; rc=1; }
    else
        echo "  timeline present but no .md manifest"; rc=1
    fi
    return $rc
}

validate_classification() {
    local jsonl="$1" md="$2" rc=0
    [[ -f "$jsonl" ]] || return 0   # vacuous-until-committed

    local cls next t2 t3
    cls=$(str "$jsonl" classification)
    grep -qE "\"next_slice\":\"[^\"]+\"" "$jsonl" || { echo "  no next_slice recommendation"; rc=1; }
    t2=$(num "$jsonl" t2_rss_anon_kib)
    t3=$(num "$jsonl" t3_rss_anon_kib)

    grep -qE "$CLASSES" <<< "$cls" || { echo "  classification '$cls' not in the closed set"; rc=1; }
    if [[ -z "$t2" || -z "$t3" ]]; then
        echo "  classification record missing t2_rss_anon_kib / t3_rss_anon_kib"; rc=1
    else
        local expect; expect=$(band "$t2" "$t3")
        if [[ "$cls" != "$expect" ]]; then
            echo "  classification '$cls' contradicts the data (t2=$t2 t3=$t3 -> band '$expect')"; rc=1
        fi
    fi
    # sha256 binding.
    if [[ -f "$md" ]]; then
        local sum; sum=$(sha256sum "$jsonl" | cut -d' ' -f1)
        grep -qF "$sum" "$md" || { echo "  classification .md does not bind the .jsonl sha256 ($sum)"; rc=1; }
    else
        echo "  classification present but no .md manifest"; rc=1
    fi
    return $rc
}

if [[ "${1:-}" == "--self-test" ]]; then
    tmp=$(mktemp -d)
    bind() { echo "manifest; jsonl sha256: $(sha256sum "$1" | cut -d' ' -f1)" > "$2"; }

    # valid serialization_transient: t2=4600000 t3=2000000 -> 56% reclaimed.
    printf '%s\n' \
        '{"event":"phase_classification","classification":"serialization_transient","next_slice":"seed_to_snapshot streaming fix","t1_rss_anon_kib":1300000,"t2_rss_anon_kib":4600000,"t3_rss_anon_kib":2000000,"t4_rss_anon_kib":2100000}' > "$tmp/st.jsonl"
    bind "$tmp/st.jsonl" "$tmp/st.md"
    validate_classification "$tmp/st.jsonl" "$tmp/st.md" || fail "self-test: a valid serialization_transient record was rejected"

    # valid live_working_set: t2=4600000 t3=4500000 -> 2% reclaimed.
    printf '%s\n' \
        '{"event":"phase_classification","classification":"live_working_set","next_slice":"on-disk UTxO backend","t1_rss_anon_kib":1300000,"t2_rss_anon_kib":4600000,"t3_rss_anon_kib":4500000,"t4_rss_anon_kib":4590000}' > "$tmp/lw.jsonl"
    bind "$tmp/lw.jsonl" "$tmp/lw.md"
    validate_classification "$tmp/lw.jsonl" "$tmp/lw.md" || fail "self-test: a valid live_working_set record was rejected"

    # dishonest: claims live_working_set but 56% reclaimed -> reject.
    sed 's/"classification":"serialization_transient"/"classification":"live_working_set"/' "$tmp/st.jsonl" > "$tmp/lie.jsonl"
    bind "$tmp/lie.jsonl" "$tmp/lie.md"
    validate_classification "$tmp/lie.jsonl" "$tmp/lie.md" && fail "self-test: a classification contradicting its data was accepted"

    # unknown classification -> reject.
    sed 's/"classification":"serialization_transient"/"classification":"totally_bogus"/' "$tmp/st.jsonl" > "$tmp/uk.jsonl"
    bind "$tmp/uk.jsonl" "$tmp/uk.md"
    validate_classification "$tmp/uk.jsonl" "$tmp/uk.md" && fail "self-test: an unknown classification was accepted"

    # missing next_slice -> reject.
    sed 's/,"next_slice":"seed_to_snapshot streaming fix"//' "$tmp/st.jsonl" > "$tmp/nn.jsonl"
    bind "$tmp/nn.jsonl" "$tmp/nn.md"
    validate_classification "$tmp/nn.jsonl" "$tmp/nn.md" && fail "self-test: a record with no next_slice was accepted"

    # valid timeline (4 points + agreed + bound) -> accept.
    printf '%s\n' \
        '{"event":"memory_measure","point":"seed_import","slot":1,"durable_tip_slot":1,"durable_tip_fp_hex":"aa","rss_kib":1,"rss_anon_kib":1300000}' \
        '{"event":"memory_measure","point":"t2_snapshot_serializing","slot":1,"durable_tip_slot":1,"durable_tip_fp_hex":"aa","rss_kib":1,"rss_anon_kib":4600000}' \
        '{"event":"memory_measure","point":"t3_after_forced_allocator_collect_diagnostic_only","slot":1,"durable_tip_slot":1,"durable_tip_fp_hex":"aa","rss_kib":1,"rss_anon_kib":2000000}' \
        '{"event":"memory_measure","point":"sustained","slot":1,"durable_tip_slot":1,"durable_tip_fp_hex":"aa","rss_kib":1,"rss_anon_kib":2100000}' \
        '{"event":"memory_summary","sample_count":4,"rss_p50_kib":1,"rss_p95_kib":1,"rss_peak_kib":1,"replay_verdict":"agreed"}' > "$tmp/tl.jsonl"
    bind "$tmp/tl.jsonl" "$tmp/tl.md"
    validate_timeline "$tmp/tl.jsonl" "$tmp/tl.md" || fail "self-test: a valid 4-phase timeline was rejected"

    # timeline missing the t3 point -> reject.
    grep -v t3_after_forced "$tmp/tl.jsonl" > "$tmp/t3miss.jsonl"
    bind "$tmp/t3miss.jsonl" "$tmp/t3miss.md"
    validate_timeline "$tmp/t3miss.jsonl" "$tmp/t3miss.md" && fail "self-test: a timeline missing the t3 phase point was accepted"

    rm -rf "$tmp"
    if (( FAILED == 0 )); then
        echo "OK: mem-opt-utxo-disk S0 self-test (accept valid timeline+classification; reject dishonest/unknown/no-next-slice/missing-phase)"
    fi
    exit $FAILED
fi

# Default: validate the committed S0 artifacts (vacuous if absent).
validate_timeline "$TL_JSONL" "$TL_MD" || fail "committed S0 phase-timeline transcript failed validation"
validate_classification "$CL_JSONL" "$CL_MD" || fail "committed S0 classification record failed validation"
if (( FAILED == 0 )); then
    echo "OK: mem-opt-utxo-disk S0 diagnostic (vacuous-until-committed; CE-UD-0 phase timeline + honest classification)"
fi
exit $FAILED
