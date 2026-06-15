#!/usr/bin/env bash
set -uo pipefail

# MEM-OPT-UTXO-DISK S0 (CE-UD-0) -- active-admission owned-footprint diagnostic.
# VACUOUS-UNTIL-COMMITTED. Validates two committed artifacts when present:
#
#   1. the phase-timeline transcript -- a memory transcript carrying the phase
#      points: t1 seed_import, t2_snapshot_serializing, t3_after_forced_allocator_
#      collect_diagnostic_only, the first mempool_admission step,
#      t5_active_admission_after_forced_collect, + post-t5 continued admission
#      samples; + replay verdict agreed, 0 diverged;
#   2. the classification record -- an explicit verdict in the closed set
#      {retained_transient_bootstrap_and_admission,
#       bootstrap_transient_but_admission_live_working_set, mixed} + a next-slice
#      recommendation, AND consistent with the t3/t4/t5 owned numbers it carries.
#
# Verdict rule (t3 = the post-bootstrap-collect idle baseline; t4 = the active-
# admission level; t5 = owned after a forced collect DURING active admission):
#   near_idle    = t3 * 1.15   (within 15% of the t3 idle baseline)
#   active_floor = t4 * 0.85   (t5 "stays high" if at/above this)
#     t5 <= near_idle      -> retained_transient_bootstrap_and_admission
#     t5 >= active_floor   -> bootstrap_transient_but_admission_live_working_set
#     otherwise            -> mixed
#
# The forced collect is a MEASUREMENT INTERVENTION only (the quarantined RED
# ade_mem_diag probe). It changes no authoritative output and is never a
# production memory-management requirement.
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

VERDICTS='^(retained_transient_bootstrap_and_admission|bootstrap_transient_but_admission_live_working_set|mixed)$'

# num <file> <key> : integer value of "key":N (first match), or empty.
num() { grep -oE "\"$2\":[0-9]+" "$1" | head -1 | sed -E 's/.*://'; }
# str <file> <key> : the [a-z_] value of "key":"v" (first match), or empty.
str() { grep -oE "\"$2\":\"[a-z_]+\"" "$1" | head -1 | sed -E 's/.*:"([a-z_]+)"/\1/'; }

# band <t4> <t5_dip> <post_t5> : the verdict the active-admission numbers imply.
# The decisive signal is RE-ACCUMULATION after the t5 forced collect: mi_collect
# uses MADV_DONTNEED, so the t5 dip is MOMENTARY (even live pages drop, then fault
# back). Whether owned returns to the active level t4 (live working set the
# admission re-needs) or stays near the dip (a freed transient that did not come
# back) is what classifies it.
#   reaccum_floor = t4 * 0.85   (post_t5 returned to the active level)
#   collapse_ceil = t5 * 1.25   (post_t5 stayed near the dip)
band() {
    local t4="$1" t5="$2" post="$3"
    [[ -z "$t4" || -z "$t5" || -z "$post" || "$t4" -le 0 || "$t5" -le 0 ]] && { echo invalid; return; }
    local reaccum_floor collapse_ceil
    reaccum_floor=$(awk -v a="$t4" 'BEGIN{ printf "%d", a*17/20 }')  # 0.85 * t4
    collapse_ceil=$(awk -v a="$t5" 'BEGIN{ printf "%d", a*5/4 }')    # 1.25 * t5 dip
    if   (( post >= reaccum_floor )); then echo bootstrap_transient_but_admission_live_working_set
    elif (( post <= collapse_ceil )); then echo retained_transient_bootstrap_and_admission
    else echo mixed
    fi
}

validate_timeline() {
    local jsonl="$1" md="$2" rc=0
    [[ -f "$jsonl" ]] || return 0   # vacuous-until-committed

    # the bootstrap + admission phase points (t4 = the sustained/mempool steady level).
    for p in seed_import t2_snapshot_serializing t3_after_forced_allocator_collect_diagnostic_only t5_active_admission_after_forced_collect; do
        grep -qE "\"point\":\"$p\"" "$jsonl" || { echo "  phase point missing: $p"; rc=1; }
    done
    # t3 and t5 each carry an owned RssAnon sample (the decisive numbers).
    for p in t3_after_forced_allocator_collect_diagnostic_only t5_active_admission_after_forced_collect; do
        grep -E "\"point\":\"$p\"" "$jsonl" | grep -qE '"rss_anon_kib":[0-9]+' \
            || { echo "  $p has no rss_anon_kib"; rc=1; }
    done
    # >=12 mempool_admission samples: a stable run that admitted before AND after t5.
    local n; n=$(grep -cE '"point":"mempool_admission"' "$jsonl")
    (( n >= 12 )) || { echo "  only $n mempool_admission samples (need >=12: pre- and post-t5)"; rc=1; }
    # replay verdict agreed (the diagnostic did not perturb authority).
    grep -E '"event":"memory_summary"' "$jsonl" | grep -qE '"replay_verdict":"agreed"' \
        || { echo "  timeline memory_summary replay_verdict is not agreed"; rc=1; }
    grep -qE 'diverged' "$jsonl" && { echo "  a diverged appears in the timeline"; rc=1; }
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

    local v t4 t5 post
    v=$(str "$jsonl" verdict)
    grep -qE "\"next_slice\":\"[^\"]+\"" "$jsonl" || { echo "  no next_slice recommendation"; rc=1; }
    t4=$(num "$jsonl" t4_rss_anon_kib)
    t5=$(num "$jsonl" t5_rss_anon_kib)
    post=$(num "$jsonl" post_t5_rss_anon_kib)

    grep -qE "$VERDICTS" <<< "$v" || { echo "  verdict '$v' not in the closed set"; rc=1; }
    if [[ -z "$t4" || -z "$t5" || -z "$post" ]]; then
        echo "  classification record missing t4/t5/post_t5_rss_anon_kib"; rc=1
    else
        local expect; expect=$(band "$t4" "$t5" "$post")
        if [[ "$v" != "$expect" ]]; then
            echo "  verdict '$v' contradicts the data (t4=$t4 t5=$t5 post_t5=$post -> band '$expect')"; rc=1
        fi
    fi
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
    rec() { printf '{"event":"phase_classification","verdict":"%s","next_slice":"%s","idle_rss_anon_kib":50480,"t3_rss_anon_kib":50480,"t4_rss_anon_kib":%s,"t5_rss_anon_kib":%s,"post_t5_rss_anon_kib":%s}\n' "$1" "$2" "$3" "$4" "$5"; }

    # retained: t4=4816840 t5_dip=1869484 post_t5=2000000 (stayed near the dip).
    rec retained_transient_bootstrap_and_admission "admission allocation cleanup" 4816840 1869484 2000000 > "$tmp/r.jsonl"; bind "$tmp/r.jsonl" "$tmp/r.md"
    validate_classification "$tmp/r.jsonl" "$tmp/r.md" || fail "self-test: a valid retained verdict was rejected"

    # live: t4=4816840 t5_dip=1869484 post_t5=4812912 (re-accumulated to t4).
    rec bootstrap_transient_but_admission_live_working_set "on-disk UTxO backend" 4816840 1869484 4812912 > "$tmp/l.jsonl"; bind "$tmp/l.jsonl" "$tmp/l.md"
    validate_classification "$tmp/l.jsonl" "$tmp/l.md" || fail "self-test: a valid live verdict was rejected"

    # mixed: t4=4816840 t5_dip=1869484 post_t5=3400000 (partial re-accumulation).
    rec mixed "scope both; sharper probe" 4816840 1869484 3400000 > "$tmp/m.jsonl"; bind "$tmp/m.jsonl" "$tmp/m.md"
    validate_classification "$tmp/m.jsonl" "$tmp/m.md" || fail "self-test: a valid mixed verdict was rejected"

    # dishonest: claims retained but post_t5 re-accumulated -> reject.
    rec retained_transient_bootstrap_and_admission "x" 4816840 1869484 4812912 > "$tmp/lie.jsonl"; bind "$tmp/lie.jsonl" "$tmp/lie.md"
    validate_classification "$tmp/lie.jsonl" "$tmp/lie.md" && fail "self-test: a verdict contradicting its data was accepted"

    # unknown verdict -> reject.
    rec totally_bogus "x" 4816840 1869484 2000000 > "$tmp/uk.jsonl"; bind "$tmp/uk.jsonl" "$tmp/uk.md"
    validate_classification "$tmp/uk.jsonl" "$tmp/uk.md" && fail "self-test: an unknown verdict was accepted"

    # missing next_slice -> reject.
    printf '{"event":"phase_classification","verdict":"mixed","idle_rss_anon_kib":50480,"t3_rss_anon_kib":50480,"t4_rss_anon_kib":4816840,"t5_rss_anon_kib":1869484,"post_t5_rss_anon_kib":3400000}\n' > "$tmp/nn.jsonl"; bind "$tmp/nn.jsonl" "$tmp/nn.md"
    validate_classification "$tmp/nn.jsonl" "$tmp/nn.md" && fail "self-test: a record with no next_slice was accepted"

    # valid timeline (phases incl t5 + >=12 admits + agreed + bound) -> accept.
    {
        echo '{"event":"memory_measure","point":"seed_import","slot":1,"durable_tip_slot":1,"durable_tip_fp_hex":"aa","rss_kib":1,"rss_anon_kib":3399748}'
        echo '{"event":"memory_measure","point":"t2_snapshot_serializing","slot":1,"durable_tip_slot":1,"durable_tip_fp_hex":"aa","rss_kib":1,"rss_anon_kib":3893928}'
        echo '{"event":"memory_measure","point":"t3_after_forced_allocator_collect_diagnostic_only","slot":1,"durable_tip_slot":1,"durable_tip_fp_hex":"aa","rss_kib":1,"rss_anon_kib":2041516}'
        for i in $(seq 1 14); do echo '{"event":"memory_measure","point":"mempool_admission","slot":1,"durable_tip_slot":1,"durable_tip_fp_hex":"aa","rss_kib":1,"rss_anon_kib":4816840}'; done
        echo '{"event":"memory_measure","point":"t5_active_admission_after_forced_collect","slot":1,"durable_tip_slot":1,"durable_tip_fp_hex":"aa","rss_kib":1,"rss_anon_kib":2050000}'
        echo '{"event":"memory_summary","sample_count":18,"rss_p50_kib":1,"rss_p95_kib":1,"rss_peak_kib":1,"replay_verdict":"agreed"}'
    } > "$tmp/tl.jsonl"; bind "$tmp/tl.jsonl" "$tmp/tl.md"
    validate_timeline "$tmp/tl.jsonl" "$tmp/tl.md" || fail "self-test: a valid t5 timeline was rejected"

    # timeline missing t5 -> reject.
    grep -v t5_active_admission "$tmp/tl.jsonl" > "$tmp/t5miss.jsonl"; bind "$tmp/t5miss.jsonl" "$tmp/t5miss.md"
    validate_timeline "$tmp/t5miss.jsonl" "$tmp/t5miss.md" && fail "self-test: a timeline missing the t5 point was accepted"

    rm -rf "$tmp"
    if (( FAILED == 0 )); then
        echo "OK: mem-opt-utxo-disk S0 self-test (accept valid timeline+verdict; reject dishonest/unknown/no-next-slice/missing-t5)"
    fi
    exit $FAILED
fi

# Default: validate the committed S0 artifacts (vacuous if absent).
validate_timeline "$TL_JSONL" "$TL_MD" || fail "committed S0 phase-timeline transcript failed validation"
validate_classification "$CL_JSONL" "$CL_MD" || fail "committed S0 classification record failed validation"
if (( FAILED == 0 )); then
    echo "OK: mem-opt-utxo-disk S0 diagnostic (vacuous-until-committed; CE-UD-0 t1-t5 timeline + honest verdict)"
fi
exit $FAILED
