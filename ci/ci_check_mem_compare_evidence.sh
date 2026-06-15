#!/usr/bin/env bash
set -uo pipefail

# MEM-COMPARE-D (CE-MM-6, BA-08) — Haskell-vs-Ade RSS comparison artifact
# (operator-gated, release-tier). VACUOUS-UNTIL-COMMITTED: passes when the
# artifact is absent; strict when present. Offline evidence (no Ade runtime
# change): Haskell `cardano-node-preprod` VmRSS samples + the committed Ade A2
# RSS reference + a mechanical comparison verdict.
#
# The gate does NOT require Ade to win -- it requires the comparison to be
# present, well-formed, sha256-bound, and HONEST (the verdict field is the
# mechanical ade_rss vs haskell_peak result, never spun).
#
# Run `--self-test` to validate the validator against temp fixtures.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
EV_DIR="$REPO_ROOT/docs/evidence"
JSONL_DEFAULT="$EV_DIR/mem-compare-d-preprod.jsonl"
MD_DEFAULT="$EV_DIR/mem-compare-d-preprod.md"

FAILED=0
fail() { echo "FAIL: $1"; FAILED=1; }

ALLOWED='^(haskell_rss_sample|comparison_summary)$'

validate_artifact() {
    local jsonl="$1" md="$2" rc=0
    [[ -f "$jsonl" ]] || return 0   # vacuous-until-committed

    # (a) closed vocabulary.
    local tag
    while IFS= read -r tag; do
        grep -qE "$ALLOWED" <<< "$tag" || { echo "  unknown event tag: '$tag'"; rc=1; }
    done < <(grep -oE '"event":"[a-z_]+"' "$jsonl" | sed -E 's/.*:"([a-z_]+)"/\1/')

    # (b) >=6 Haskell samples.
    local nsamp
    nsamp=$(grep -cE '"event":"haskell_rss_sample"' "$jsonl")
    (( nsamp >= 6 )) || { echo "  too few haskell_rss_sample ($nsamp < 6)"; rc=1; }

    # (c) exactly one comparison_summary carrying the required fields.
    local nsum
    nsum=$(grep -cE '"event":"comparison_summary"' "$jsonl")
    (( nsum == 1 )) || { echo "  comparison_summary count != 1 ($nsum)"; rc=1; }
    local sumline
    sumline=$(grep -E '"event":"comparison_summary"' "$jsonl" | head -1)
    for key in ade_rss_kib haskell_peak_kib gap_kib gap_pct verdict; do
        grep -qE "\"$key\"" <<< "$sumline" || { echo "  comparison_summary missing field: $key"; rc=1; }
    done

    # (d) HONESTY: the verdict must mechanically match ade_rss vs haskell_peak --
    #     no spin. ade_rss > haskell_peak => ade_heavier; else ade_competitive.
    local ade hpk vd
    ade=$(grep -oE '"ade_rss_kib":[0-9]+' <<< "$sumline" | grep -oE '[0-9]+$')
    hpk=$(grep -oE '"haskell_peak_kib":[0-9]+' <<< "$sumline" | grep -oE '[0-9]+$')
    vd=$(grep -oE '"verdict":"[a-z_]+"' <<< "$sumline" | sed -E 's/.*:"([a-z_]+)"/\1/')
    if [[ -n "$ade" && -n "$hpk" ]]; then
        local want="ade_competitive"
        (( ade > hpk )) && want="ade_heavier"
        [[ "$vd" == "$want" ]] || { echo "  verdict '$vd' contradicts the numbers (ade=$ade haskell_peak=$hpk -> $want)"; rc=1; }
    else
        echo "  could not parse ade_rss_kib / haskell_peak_kib"; rc=1
    fi

    # (e) sha256-binding.
    if [[ -f "$md" ]]; then
        local sum
        sum=$(sha256sum "$jsonl" | cut -d' ' -f1)
        grep -qF "$sum" "$md" || { echo "  .md manifest does not bind the .jsonl sha256 ($sum)"; rc=1; }
    else
        echo "  artifact present but no .md manifest (sha256 binding required)"; rc=1
    fi

    return $rc
}

if [[ "${1:-}" == "--self-test" ]]; then
    tmp=$(mktemp -d)
    bind() { echo "mem-compare-d manifest; jsonl sha256: $(sha256sum "$1" | cut -d' ' -f1)" > "$2"; }

    # valid: 6 samples + honest ade_heavier summary + bound.
    : > "$tmp/v.jsonl"
    for i in 0 1 2 3 4 5; do echo '{"event":"haskell_rss_sample","node":"cardano-node-preprod","sample_index":'$i',"rss_kib":5770580}' >> "$tmp/v.jsonl"; done
    echo '{"event":"comparison_summary","ade_rss_kib":6874028,"haskell_peak_kib":5770580,"gap_kib":1103448,"gap_pct":19.1,"verdict":"ade_heavier"}' >> "$tmp/v.jsonl"
    bind "$tmp/v.jsonl" "$tmp/v.md"
    validate_artifact "$tmp/v.jsonl" "$tmp/v.md" || fail "self-test: a valid honest artifact was rejected"

    # spun verdict (ade heavier but claims competitive) -> reject.
    sed 's/"verdict":"ade_heavier"/"verdict":"ade_competitive"/' "$tmp/v.jsonl" > "$tmp/spin.jsonl"
    bind "$tmp/spin.jsonl" "$tmp/spin.md"
    validate_artifact "$tmp/spin.jsonl" "$tmp/spin.md" && fail "self-test: a spun verdict was accepted"

    # too few samples -> reject.
    head -3 "$tmp/v.jsonl" > "$tmp/few.jsonl"; tail -1 "$tmp/v.jsonl" >> "$tmp/few.jsonl"
    bind "$tmp/few.jsonl" "$tmp/few.md"
    validate_artifact "$tmp/few.jsonl" "$tmp/few.md" && fail "self-test: a too-few-samples artifact was accepted"

    # no summary -> reject.
    grep haskell_rss_sample "$tmp/v.jsonl" > "$tmp/nosum.jsonl"
    bind "$tmp/nosum.jsonl" "$tmp/nosum.md"
    validate_artifact "$tmp/nosum.jsonl" "$tmp/nosum.md" && fail "self-test: a no-summary artifact was accepted"

    # sha mismatch -> reject.
    cp "$tmp/v.jsonl" "$tmp/sm.jsonl"; echo "mem-compare-d manifest; jsonl sha256: deadbeef" > "$tmp/sm.md"
    validate_artifact "$tmp/sm.jsonl" "$tmp/sm.md" && fail "self-test: a sha256-mismatch artifact was accepted"

    rm -rf "$tmp"
    if (( FAILED == 0 )); then
        echo "OK: mem-compare-d evidence self-test (accept valid+honest; reject spun-verdict/too-few/no-summary/sha-mismatch)"
    fi
    exit $FAILED
fi

validate_artifact "$JSONL_DEFAULT" "$MD_DEFAULT" || fail "committed mem-compare-d artifact failed validation"
if (( FAILED == 0 )); then
    echo "OK: mem-compare-d evidence (vacuous-until-committed; CE-MM-6 / BA-08)"
fi
exit $FAILED
