#!/usr/bin/env bash
set -uo pipefail

# CE-OPS-3 (OP-MEM-02 / MEM-OPT-OPS S3): the OWNED-footprint measurement + an
# honest owned comparison. Two mechanical facts over the committed artifacts:
#
#   1. SCHEMA: the S3 re-measurement transcript carries BOTH gross and OWNED
#      metrics, clearly labeled — each memory_measure has rss_anon_kib +
#      private_dirty_kib; the memory_summary has owned_rss_anon_p50_kib /
#      _peak_kib + owned_private_dirty_p50_kib / _peak_kib alongside the gross
#      fields — with replay_verdict agreed.
#
#   2. HONEST VERDICT: the regenerated owned comparison
#      (mem-opt-ops-s3-owned-compare) carries a comparison_summary whose verdict
#      is CONSISTENT with the values: `ade_below` iff Ade's owned RssAnon is
#      strictly below the Haskell node's, else `ade_heavier`. Both reported; the
#      owned metric (RssAnon — excludes the chain.db mmap) is the gated one, gross
#      VmRSS informational.
#
# VACUOUS-UNTIL-COMMITTED. Run `--self-test`.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
EV_DIR="$REPO_ROOT/docs/evidence"
S3="$EV_DIR/mem-opt-ops-s3-owned-preprod-memory.jsonl"
S3_MD="$EV_DIR/mem-opt-ops-s3-owned-preprod-memory.md"
CMP="$EV_DIR/mem-opt-ops-s3-owned-compare-preprod.jsonl"

FAILED=0
fail() { echo "FAIL: $1"; FAILED=1; }

jnum() { # <key> (line on stdin) : first integer value of "key":<int>
    grep -oE "\"$1\":[0-9]+" | head -1 | sed -E 's/.*:([0-9]+)/\1/'
}

# validate_schema <s3.jsonl> <s3.md> : 0 = owned schema present + agreed + bound.
validate_schema() {
    local j="$1" md="$2" rc=0
    [[ -f "$j" ]] || { echo "  (vacuous: S3 transcript absent)"; return 0; }
    # memory_summary owned fields present.
    local sum; sum="$(grep -E '"event":"memory_summary"' "$j" 2>/dev/null | head -1)"
    [[ -n "$sum" ]] || { echo "  no memory_summary"; return 1; }
    for k in owned_rss_anon_p50_kib owned_rss_anon_peak_kib owned_private_dirty_p50_kib owned_private_dirty_peak_kib; do
        grep -qE "\"$k\":[0-9]+" <<< "$sum" || { echo "  memory_summary missing owned field $k"; rc=1; }
    done
    grep -qE '"replay_verdict":"agreed"' <<< "$sum" || { echo "  S3 replay_verdict not agreed"; rc=1; }
    grep -qE 'diverged' "$j" && { echo "  a diverged appears in the S3 transcript"; rc=1; }
    # at least one memory_measure carries the owned per-point fields.
    grep -E '"event":"memory_measure"' "$j" 2>/dev/null | grep -qE '"rss_anon_kib":[0-9]+' \
        || { echo "  no memory_measure with rss_anon_kib (owned per-point)"; rc=1; }
    grep -E '"event":"memory_measure"' "$j" 2>/dev/null | grep -qE '"private_dirty_kib":[0-9]+' \
        || { echo "  no memory_measure with private_dirty_kib"; rc=1; }
    # sha256 binding (the .md carries the .jsonl sha256).
    if [[ -f "$md" ]]; then
        local sha; sha=$(sha256sum "$j" | cut -d' ' -f1)
        grep -qF "$sha" "$md" || { echo "  .md does not bind the S3 .jsonl sha256 ($sha)"; rc=1; }
    else
        echo "  S3 transcript present but no .md manifest"; rc=1
    fi
    return $rc
}

# validate_comparison <cmp.jsonl> : 0 = verdict consistent with the owned values.
validate_comparison() {
    local c="$1" rc=0
    [[ -f "$c" ]] || { echo "  (vacuous: comparison artifact absent)"; return 0; }
    local line; line="$(grep -E '"event":"comparison_summary"' "$c" 2>/dev/null | head -1)"
    [[ -n "$line" ]] || { echo "  no comparison_summary"; return 1; }
    local ade hask verdict
    ade="$(jnum <<< "$line" ade_owned_rss_anon_kib)"
    hask="$(jnum <<< "$line" haskell_owned_rss_anon_kib)"
    verdict="$(grep -oE '"verdict":"[a-z_]+"' <<< "$line" | head -1 | sed -E 's/.*:"([a-z_]+)"/\1/')"
    [[ -n "$ade" && -n "$hask" && -n "$verdict" ]] || { echo "  comparison_summary missing ade/haskell/verdict"; return 1; }
    local expected="ade_heavier"
    (( ade < hask )) && expected="ade_below"
    if [[ "$verdict" == "$expected" ]]; then
        echo "  owned comparison: Ade RssAnon $ade vs Haskell $hask kiB -> verdict $verdict [ok]"
    else
        echo "  DISHONEST verdict: $verdict but Ade $ade vs Haskell $hask implies $expected"; rc=1
    fi
    return $rc
}

self_test() {
    local tmp st=0; tmp=$(mktemp -d); trap 'rm -rf "$tmp"' RETURN
    # GOOD schema.
    printf '%s\n' \
        '{"event":"memory_measure","point":"sustained","slot":1,"durable_tip_slot":1,"durable_tip_fp_hex":"aa","rss_kib":4800000,"rss_hwm_kib":8000000,"rss_anon_kib":2300000,"private_dirty_kib":2200000}' \
        '{"event":"memory_summary","sample_count":3,"rss_p50_kib":4800000,"rss_p95_kib":4800000,"rss_peak_kib":4800000,"rss_hwm_kib":8000000,"owned_rss_anon_p50_kib":2300000,"owned_rss_anon_peak_kib":2350000,"owned_private_dirty_p50_kib":2200000,"owned_private_dirty_peak_kib":2250000,"replay_verdict":"agreed"}' > "$tmp/s3.jsonl"
    echo "sha256: $(sha256sum "$tmp/s3.jsonl" | cut -d' ' -f1)" > "$tmp/s3.md"
    validate_schema "$tmp/s3.jsonl" "$tmp/s3.md" >/dev/null && echo "self-test: GOOD schema accepted [ok]" || { echo "self-test: GOOD schema should pass"; st=1; }
    # BAD schema (missing owned field).
    grep -v owned_rss_anon_p50_kib "$tmp/s3.jsonl" > "$tmp/bad.jsonl"; echo "sha256: $(sha256sum "$tmp/bad.jsonl"|cut -d' ' -f1)" > "$tmp/bad.md"
    validate_schema "$tmp/bad.jsonl" "$tmp/bad.md" >/dev/null && { echo "self-test: BAD schema should FAIL"; st=1; } || echo "self-test: BAD schema (missing owned field) rejected [ok]"
    # GOOD comparison (ade_below, ade<hask).
    printf '%s\n' '{"event":"comparison_summary","metric":"owned_rss_anon_kib","ade_owned_rss_anon_kib":2300000,"haskell_owned_rss_anon_kib":4600000,"verdict":"ade_below"}' > "$tmp/cmp.jsonl"
    validate_comparison "$tmp/cmp.jsonl" >/dev/null && echo "self-test: GOOD comparison (ade_below) accepted [ok]" || { echo "self-test: GOOD comparison should pass"; st=1; }
    # DISHONEST comparison (ade_below but ade>hask).
    printf '%s\n' '{"event":"comparison_summary","metric":"owned_rss_anon_kib","ade_owned_rss_anon_kib":5000000,"haskell_owned_rss_anon_kib":4600000,"verdict":"ade_below"}' > "$tmp/dis.jsonl"
    validate_comparison "$tmp/dis.jsonl" >/dev/null && { echo "self-test: DISHONEST verdict should FAIL"; st=1; } || echo "self-test: DISHONEST verdict rejected [ok]"
    if (( st == 0 )); then echo "OK: --self-test — owned schema + honest-verdict checks."; return 0; fi
    return 1
}

if [[ "${1:-}" == "--self-test" ]]; then
    self_test; exit $?
fi

echo "MEM-OPT-OPS S3 owned-footprint (CE-OPS-3): owned-evidence schema + honest owned comparison"
validate_schema "$S3" "$S3_MD" || fail "S3 owned-evidence schema invalid"
validate_comparison "$CMP" || fail "S3 owned comparison verdict not honest"
if (( FAILED == 0 )); then
    echo "OK: mem-opt-ops S3 owned-footprint (vacuous-until-committed)"
fi
exit $FAILED
