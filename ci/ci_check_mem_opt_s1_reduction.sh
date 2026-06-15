#!/usr/bin/env bash
set -uo pipefail

# CE-OPS-1 (OP-MEM-02 / MEM-OPT-OPS S1): the allocator-swapped re-measurement's
# resident memory is STRICTLY BELOW the MEM-MEASURE-A2 baseline on the identical
# preprod protocol (only the process global allocator changed: glibc System ->
# mimalloc). The memory lever's effect, mechanically asserted on the committed
# transcripts' memory_summary p50 + peak.
#
# VACUOUS-UNTIL-COMMITTED: passes when either transcript is absent; strict when
# both are present. Schema validity of each transcript is the separate
# ci_check_mem_measure_evidence.sh; this gate asserts only the reduction.
#
# Run `--self-test` to validate the gate against temp fixtures.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
EV_DIR="$REPO_ROOT/docs/evidence"
BASELINE="$EV_DIR/mem-measure-a2-preprod-memory.jsonl"
S1="$EV_DIR/mem-opt-ops-s1-alloc-preprod-memory.jsonl"

FAILED=0
fail() { echo "FAIL: $1"; FAILED=1; }

# summary_field <jsonl> <key> : the integer value of `"key":<int>` in the
# memory_summary line, or empty if absent.
summary_field() {
    grep -E '"event":"memory_summary"' "$1" 2>/dev/null \
        | grep -oE "\"$2\":[0-9]+" | head -1 | sed -E 's/.*:([0-9]+)/\1/'
}

# assert_reduction <baseline.jsonl> <s1.jsonl> : 0 if S1 p50 AND peak strictly
# below baseline (or vacuous: a file absent); non-zero otherwise.
assert_reduction() {
    local base="$1" s1="$2"
    [[ -f "$base" && -f "$s1" ]] || { echo "  (vacuous: a transcript is absent)"; return 0; }
    local b50 bpk s50 spk v
    b50=$(summary_field "$base" rss_p50_kib); bpk=$(summary_field "$base" rss_peak_kib)
    s50=$(summary_field "$s1" rss_p50_kib);   spk=$(summary_field "$s1" rss_peak_kib)
    for v in "$b50" "$bpk" "$s50" "$spk"; do
        [[ -n "$v" ]] || { echo "  missing rss summary field (baseline or S1)"; return 1; }
    done
    local rc=0
    if (( s50 < b50 )); then
        echo "  p50:  S1 $s50 < A2 $b50 kiB  (-$(( b50 - s50 )) kiB) [ok]"
    else
        echo "  p50 NOT strictly below baseline: S1 $s50 >= A2 $b50 kiB"; rc=1
    fi
    if (( spk < bpk )); then
        echo "  peak: S1 $spk < A2 $bpk kiB  (-$(( bpk - spk )) kiB) [ok]"
    else
        echo "  peak NOT strictly below baseline: S1 $spk >= A2 $bpk kiB"; rc=1
    fi
    return $rc
}

self_test() {
    local tmp st=0
    tmp=$(mktemp -d); trap 'rm -rf "$tmp"' RETURN
    local base='{"event":"memory_summary","sample_count":3,"rss_p50_kib":6874024,"rss_p95_kib":6874028,"rss_peak_kib":6874028,"replay_verdict":"agreed"}'
    printf '%s\n' "$base" > "$tmp/base.jsonl"

    # GOOD: S1 strictly below baseline.
    printf '%s\n' '{"event":"memory_summary","sample_count":3,"rss_p50_kib":4824884,"rss_p95_kib":4824968,"rss_peak_kib":4824976,"replay_verdict":"agreed"}' > "$tmp/s1.jsonl"
    if assert_reduction "$tmp/base.jsonl" "$tmp/s1.jsonl" >/dev/null; then
        echo "self-test: GOOD (S1 below) accepted [ok]"; else echo "self-test: GOOD should pass but FAILED"; st=1; fi

    # BAD: S1 equal to baseline (not strictly below).
    printf '%s\n' "$base" > "$tmp/s1eq.jsonl"
    if assert_reduction "$tmp/base.jsonl" "$tmp/s1eq.jsonl" >/dev/null; then
        echo "self-test: BAD (equal) should FAIL but passed"; st=1; else echo "self-test: BAD (equal, not strictly below) rejected [ok]"; fi

    # BAD: S1 peak above baseline.
    printf '%s\n' '{"event":"memory_summary","sample_count":3,"rss_p50_kib":4824884,"rss_p95_kib":9000000,"rss_peak_kib":9000000,"replay_verdict":"agreed"}' > "$tmp/s1hi.jsonl"
    if assert_reduction "$tmp/base.jsonl" "$tmp/s1hi.jsonl" >/dev/null; then
        echo "self-test: BAD (peak above) should FAIL but passed"; st=1; else echo "self-test: BAD (peak above) rejected [ok]"; fi

    # VACUOUS: absent S1.
    if assert_reduction "$tmp/base.jsonl" "$tmp/absent.jsonl" >/dev/null; then
        echo "self-test: VACUOUS (absent S1) accepted [ok]"; else echo "self-test: VACUOUS should pass but FAILED"; st=1; fi

    if (( st == 0 )); then
        echo "OK: --self-test — accepts S1-below-baseline, rejects equal / above, vacuous when absent."
        return 0
    fi
    return 1
}

if [[ "${1:-}" == "--self-test" ]]; then
    self_test
    exit $?
fi

echo "MEM-OPT-OPS S1 reduction (CE-OPS-1): S1 RSS strictly below the A2 baseline"
assert_reduction "$BASELINE" "$S1" || fail "S1 transcript is not strictly below the A2 baseline"
if (( FAILED == 0 )); then
    echo "OK: mem-opt-ops S1 RSS reduction (vacuous-until-committed)"
fi
exit $FAILED
