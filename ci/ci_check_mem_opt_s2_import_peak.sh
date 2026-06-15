#!/usr/bin/env bash
set -uo pipefail

# CE-OPS-2 (OP-MEM-02 / MEM-OPT-OPS S2): the STREAMING seed import (a) removes the
# import peak and (b) produces the byte-identical imported ledger state. Two
# mechanical facts over the committed transcripts:
#
#   1. IMPORT PEAK: the `seed_import` measurement point's rss_hwm_kib -- the VmHWM
#      captured in bootstrap RIGHT AFTER import() returns, BEFORE the chain.db
#      snapshot write -- is STRICTLY BELOW the whole-buffer import footprint
#      (the MEM-MEASURE-A2 baseline rss_peak_kib; glibc retained the whole-buffer
#      import, so A2's resident peak == that footprint). This is the IMPORT-SPECIFIC
#      metric. NOTE: the run-end memory_summary.rss_hwm_kib is a LATER, larger
#      transient (the UTxO->chain.db snapshot serialization) and is NOT the import;
#      that is a separate finding/optimization target, deliberately not gated here.
#
#   2. EQUIVALENCE: the streaming run's bootstrap initial_ledger_fp_hex EQUALS the
#      S1 (whole-buffer) run's -- the streamed import is byte-identical, a memory
#      win and never a consensus change (DC-MEM-06 / DC-WAL-03).
#
# VACUOUS-UNTIL-COMMITTED: passes when a transcript is absent; strict when all
# present. Run `--self-test`.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
EV_DIR="$REPO_ROOT/docs/evidence"
A2="$EV_DIR/mem-measure-a2-preprod-memory.jsonl"         # whole-buffer reference (rss_peak)
S1="$EV_DIR/mem-opt-ops-s1-alloc-preprod-memory.jsonl"   # initial_ledger_fp reference
S2="$EV_DIR/mem-opt-ops-s2-import-preprod-memory.jsonl"  # the streaming run

FAILED=0
fail() { echo "FAIL: $1"; FAILED=1; }

# summary_field <jsonl> <key> : integer value of "key":<int> in memory_summary.
summary_field() {
    grep -E '"event":"memory_summary"' "$1" 2>/dev/null \
        | grep -oE "\"$2\":[0-9]+" | head -1 | sed -E 's/.*:([0-9]+)/\1/'
}
# seed_import_hwm <jsonl> : rss_hwm_kib of the `seed_import` memory_measure point
# (the import peak captured right after import(), before the snapshot write).
seed_import_hwm() {
    grep -E '"event":"memory_measure"[^}]*"point":"seed_import"' "$1" 2>/dev/null \
        | grep -oE '"rss_hwm_kib":[0-9]+' | head -1 | sed -E 's/.*:([0-9]+)/\1/'
}
# bootstrap_fp <jsonl> : initial_ledger_fp_hex from the bootstrap_complete event.
bootstrap_fp() {
    grep -E '"event":"bootstrap_complete"' "$1" 2>/dev/null \
        | grep -oE '"initial_ledger_fp_hex":"[0-9a-f]+"' | head -1 | sed -E 's/.*:"([0-9a-f]+)"/\1/'
}

# assert_s2 <a2> <s1> <s2> : 0 if import-peak-below + import-equivalent (or vacuous).
assert_s2() {
    local a2="$1" s1="$2" s2="$3"
    [[ -f "$a2" && -f "$s1" && -f "$s2" ]] || { echo "  (vacuous: a transcript is absent)"; return 0; }
    local rc=0 wb_peak s2_import_hwm s1_fp s2_fp
    wb_peak=$(summary_field "$a2" rss_peak_kib)
    s2_import_hwm=$(seed_import_hwm "$s2")
    s1_fp=$(bootstrap_fp "$s1")
    s2_fp=$(bootstrap_fp "$s2")
    [[ -n "$wb_peak" ]] || { echo "  missing A2 rss_peak_kib"; return 1; }
    [[ -n "$s2_import_hwm" ]] || { echo "  missing S2 seed_import rss_hwm_kib (the import-peak tap)"; return 1; }
    [[ -n "$s1_fp" && -n "$s2_fp" ]] || { echo "  missing bootstrap initial_ledger_fp_hex"; return 1; }
    if (( s2_import_hwm < wb_peak )); then
        echo "  import peak: S2 seed_import VmHWM $s2_import_hwm < whole-buffer $wb_peak kiB  (-$(( wb_peak - s2_import_hwm )) kiB) [ok]"
    else
        echo "  import peak NOT below the whole-buffer import: S2 $s2_import_hwm >= A2 $wb_peak kiB"; rc=1
    fi
    if [[ "$s2_fp" == "$s1_fp" ]]; then
        echo "  import equivalence: S2 initial_ledger_fp == S1 ($s2_fp) [ok]"
    else
        echo "  import DIVERGED (consensus change, not a memory win): S2 $s2_fp != S1 $s1_fp"; rc=1
    fi
    return $rc
}

self_test() {
    local tmp st=0
    tmp=$(mktemp -d); trap 'rm -rf "$tmp"' RETURN
    printf '%s\n' '{"event":"memory_summary","sample_count":3,"rss_p50_kib":6874024,"rss_p95_kib":6874028,"rss_peak_kib":6874028,"replay_verdict":"agreed"}' > "$tmp/a2.jsonl"
    printf '%s\n' '{"event":"bootstrap_complete","initial_ledger_fp_hex":"fb7cb12a","chain_tip_slot":1}' > "$tmp/s1.jsonl"

    # GOOD: seed_import VmHWM below A2 peak + fp matches S1.
    printf '%s\n' \
        '{"event":"bootstrap_complete","initial_ledger_fp_hex":"fb7cb12a","chain_tip_slot":1}' \
        '{"event":"memory_measure","point":"seed_import","slot":1,"durable_tip_slot":1,"durable_tip_fp_hex":"aa","rss_kib":2000000,"rss_hwm_kib":3000000}' \
        '{"event":"memory_summary","sample_count":3,"rss_p50_kib":4800000,"rss_p95_kib":4800000,"rss_peak_kib":4800000,"rss_hwm_kib":7900000,"replay_verdict":"agreed"}' > "$tmp/s2_good.jsonl"
    if assert_s2 "$tmp/a2.jsonl" "$tmp/s1.jsonl" "$tmp/s2_good.jsonl" >/dev/null; then
        echo "self-test: GOOD (import peak below + fp match) accepted [ok]"; else echo "self-test: GOOD should pass but FAILED"; st=1; fi

    # BAD import peak: seed_import VmHWM >= A2 peak (even though run-end summary looks fine).
    printf '%s\n' \
        '{"event":"bootstrap_complete","initial_ledger_fp_hex":"fb7cb12a","chain_tip_slot":1}' \
        '{"event":"memory_measure","point":"seed_import","slot":1,"durable_tip_slot":1,"durable_tip_fp_hex":"aa","rss_kib":2000000,"rss_hwm_kib":7000000}' \
        '{"event":"memory_summary","sample_count":3,"rss_p50_kib":4800000,"rss_p95_kib":4800000,"rss_peak_kib":4800000,"rss_hwm_kib":7000000,"replay_verdict":"agreed"}' > "$tmp/s2_hi.jsonl"
    if assert_s2 "$tmp/a2.jsonl" "$tmp/s1.jsonl" "$tmp/s2_hi.jsonl" >/dev/null; then
        echo "self-test: BAD (import peak not below) should FAIL but passed"; st=1; else echo "self-test: BAD (import peak not below) rejected [ok]"; fi

    # BAD equivalence: S2 fp != S1 fp.
    printf '%s\n' \
        '{"event":"bootstrap_complete","initial_ledger_fp_hex":"deadbeef","chain_tip_slot":1}' \
        '{"event":"memory_measure","point":"seed_import","slot":1,"durable_tip_slot":1,"durable_tip_fp_hex":"aa","rss_kib":2000000,"rss_hwm_kib":3000000}' \
        '{"event":"memory_summary","sample_count":3,"rss_p50_kib":4800000,"rss_p95_kib":4800000,"rss_peak_kib":4800000,"rss_hwm_kib":7900000,"replay_verdict":"agreed"}' > "$tmp/s2_div.jsonl"
    if assert_s2 "$tmp/a2.jsonl" "$tmp/s1.jsonl" "$tmp/s2_div.jsonl" >/dev/null; then
        echo "self-test: BAD (import diverged) should FAIL but passed"; st=1; else echo "self-test: BAD (import diverged) rejected [ok]"; fi

    # VACUOUS: S2 absent.
    if assert_s2 "$tmp/a2.jsonl" "$tmp/s1.jsonl" "$tmp/absent.jsonl" >/dev/null; then
        echo "self-test: VACUOUS (absent S2) accepted [ok]"; else echo "self-test: VACUOUS should pass but FAILED"; st=1; fi

    if (( st == 0 )); then
        echo "OK: --self-test — accepts import-peak-below + fp-match, rejects import-peak-not-below / import-diverged, vacuous when absent."
        return 0
    fi
    return 1
}

if [[ "${1:-}" == "--self-test" ]]; then
    self_test
    exit $?
fi

echo "MEM-OPT-OPS S2 import peak (CE-OPS-2): streaming seed_import VmHWM below whole-buffer import + byte-identical ledger fp"
assert_s2 "$A2" "$S1" "$S2" || fail "S2 streaming import did not meet CE-OPS-2 (import-peak-below + import-equivalence)"
if (( FAILED == 0 )); then
    echo "OK: mem-opt-ops S2 import-peak reduction (vacuous-until-committed)"
fi
exit $FAILED
