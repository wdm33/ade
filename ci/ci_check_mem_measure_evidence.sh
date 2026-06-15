#!/usr/bin/env bash
set -uo pipefail

# MEM-MEASURE-A2 (OP-MEM-01) — live C2-LOCAL memory-evidence transcript schema
# (operator-gated, operational-tier). VACUOUS-UNTIL-COMMITTED: passes when the
# transcript is absent; strict when present. The committed transcript is a
# `--mode node` convergence-evidence file extended with the closed memory
# vocabulary (memory_measure / memory_summary, DC-ADMIT-04 closure).
#
# The no-starvation + replay-equivalence assertion: closed convergence+memory
# vocabulary + closed measurement points + the run-level replay verdict `agreed`
# + >=1 block_admitted interleaved with the samples (block validation kept making
# progress under memory observation) + 0 diverged + sha256-binding.
#
# Run `--self-test` to validate the validator against temp fixtures.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
EV_DIR="$REPO_ROOT/docs/evidence"
JSONL_DEFAULT="$EV_DIR/mem-measure-a2-preprod-memory.jsonl"
MD_DEFAULT="$EV_DIR/mem-measure-a2-preprod-memory.md"
# MEM-OPT-OPS S1: same schema, the allocator-swapped re-measurement (CE-OPS-1).
S1_JSONL="$EV_DIR/mem-opt-ops-s1-alloc-preprod-memory.jsonl"
S1_MD="$EV_DIR/mem-opt-ops-s1-alloc-preprod-memory.md"
# MEM-OPT-OPS S2: same schema + the `seed_import` point, the streaming import (CE-OPS-2).
S2_JSONL="$EV_DIR/mem-opt-ops-s2-import-preprod-memory.jsonl"
S2_MD="$EV_DIR/mem-opt-ops-s2-import-preprod-memory.md"
# MEM-OPT-OPS S3: same schema + owned (RssAnon/Private_Dirty) fields (CE-OPS-3).
S3_JSONL="$EV_DIR/mem-opt-ops-s3-owned-preprod-memory.jsonl"
S3_MD="$EV_DIR/mem-opt-ops-s3-owned-preprod-memory.md"
# MEM-OPT-UTXO-DISK S0: same schema + the t2/t3 phase-diagnostic points (CE-UD-0).
UD_S0_JSONL="$EV_DIR/mem-opt-utxo-disk-s0-phase-timeline-preprod.jsonl"
UD_S0_MD="$EV_DIR/mem-opt-utxo-disk-s0-phase-timeline-preprod.md"

FAILED=0
fail() { echo "FAIL: $1"; FAILED=1; }

# Closed vocabulary = the convergence-evidence subset (the only variants the
# ConvergenceEvidenceSink constructs) + the MEM-MEASURE-A2 memory events. Kept in
# lockstep with ci_check_convergence_evidence_vocabulary_closed.sh's ALLOWED_LITERALS.
ALLOWED='^(admission_started|snapshot_imported|bootstrap_complete|admission_shutdown|admission_halted|block_received|block_admitted|agreement_verdict|needs_fork_choice|lca_discovered|candidate_fragment_built|fork_choice_selected|branch_fetch_started|branch_fetch_completed|branch_prevalidated|fork_switch_applied|fork_switch_failed|fork_switch_superseded|missing_bridge|range_refetch_started|range_refetch_completed|memory_measure|memory_summary)$'

# Closed measurement points (the only `point` values a memory_measure may carry).
# t2_snapshot_serializing / t3_after_forced_allocator_collect_diagnostic_only:
# MEM-OPT-UTXO-DISK S0 phase-diagnostic points (emitted only under the RED
# ADE_MEM_PHASE_DIAGNOSTIC env toggle). The t3 label names the measurement
# intervention (a forced allocator collect) explicitly.
POINTS='^(seed_import|idle_recovered_tip|chain_sync_follow|block_fetch_serve|mempool_admission|wal_checkpoint_recovery|sustained|t2_snapshot_serializing|t3_after_forced_allocator_collect_diagnostic_only|t5_active_admission_after_forced_collect)$'

# validate_transcript <jsonl> <md> : 0 = a valid memory transcript (or absent →
# vacuous); non-zero = reject.
validate_transcript() {
    local jsonl="$1" md="$2" rc=0
    [[ -f "$jsonl" ]] || return 0   # vacuous-until-committed

    # (a) closed vocabulary — every event tag is in the allow-list.
    local tag
    while IFS= read -r tag; do
        grep -qE "$ALLOWED" <<< "$tag" || { echo "  unknown event tag: '$tag'"; rc=1; }
    done < <(grep -oE '"event":"[a-z_]+"' "$jsonl" | sed -E 's/.*:"([a-z_]+)"/\1/')

    # (b) every memory_measure `point` is in the closed set.
    local pt
    while IFS= read -r pt; do
        grep -qE "$POINTS" <<< "$pt" || { echo "  unknown measurement point: '$pt'"; rc=1; }
    done < <(grep -oE '"point":"[a-z0-9_]+"' "$jsonl" | sed -E 's/.*:"([a-z0-9_]+)"/\1/')

    # (c) at least one RSS sample.
    grep -qE '"event":"memory_measure"' "$jsonl" \
        || { echo "  no memory_measure samples"; rc=1; }

    # (d) exactly the run summary, replay verdict `agreed` (never diverged).
    if ! grep -qE '"event":"memory_summary"' "$jsonl"; then
        echo "  no memory_summary (the run-level replay verdict is missing)"; rc=1
    fi
    if grep -E '"event":"memory_summary"' "$jsonl" | grep -qE '"replay_verdict":"diverged"'; then
        echo "  memory_summary replay_verdict is diverged -- not replay-equivalent"; rc=1
    fi
    grep -E '"event":"memory_summary"' "$jsonl" | grep -qE '"replay_verdict":"agreed"' \
        || { echo "  memory_summary replay_verdict is not agreed"; rc=1; }

    # (e) no starvation: block validation made progress (>=1 block_admitted) while
    #     the run was being memory-sampled.
    grep -qE '"event":"block_admitted"' "$jsonl" \
        || { echo "  no block_admitted -- block validation did not progress (starvation?)"; rc=1; }

    # (f) 0 diverged anywhere (Ade never disagreed with the peer).
    if grep -qE 'diverged' "$jsonl"; then
        echo "  a diverged appears in the transcript"; rc=1
    fi

    # (g) sha256-binding: the .md manifest carries the .jsonl's sha256.
    if [[ -f "$md" ]]; then
        local sum
        sum=$(sha256sum "$jsonl" | cut -d' ' -f1)
        grep -qF "$sum" "$md" || { echo "  .md manifest does not bind the .jsonl sha256 ($sum)"; rc=1; }
    else
        echo "  transcript present but no .md manifest (sha256 binding required)"; rc=1
    fi

    return $rc
}

if [[ "${1:-}" == "--self-test" ]]; then
    tmp=$(mktemp -d)
    bind() { echo "mem-measure-a2 manifest; jsonl sha256: $(sha256sum "$1" | cut -d' ' -f1)" > "$2"; }

    # valid: samples across points + block_admitted + summary agreed + bound.
    printf '%s\n' \
        '{"event":"memory_measure","point":"wal_checkpoint_recovery","slot":100,"durable_tip_slot":100,"durable_tip_fp_hex":"aa","rss_kib":1000}' \
        '{"event":"memory_measure","point":"idle_recovered_tip","slot":100,"durable_tip_slot":100,"durable_tip_fp_hex":"aa","rss_kib":1000}' \
        '{"event":"block_received","slot":101,"block_hash_hex":"bb"}' \
        '{"event":"block_admitted","slot":101,"block_hash_hex":"bb","prev_hash_hex":"aa","post_fp_hex":"cc","consensus_inputs_fingerprint_hex":"00"}' \
        '{"event":"memory_measure","point":"chain_sync_follow","slot":101,"durable_tip_slot":101,"durable_tip_fp_hex":"cc","rss_kib":1010}' \
        '{"event":"memory_measure","point":"sustained","slot":101,"durable_tip_slot":101,"durable_tip_fp_hex":"cc","rss_kib":1010}' \
        '{"event":"memory_summary","sample_count":4,"rss_p50_kib":1010,"rss_p95_kib":1010,"rss_peak_kib":1010,"replay_verdict":"agreed"}' > "$tmp/v.jsonl"
    bind "$tmp/v.jsonl" "$tmp/v.md"
    validate_transcript "$tmp/v.jsonl" "$tmp/v.md" || fail "self-test: a valid memory transcript was rejected"

    # diverged summary -> reject.
    sed 's/"replay_verdict":"agreed"/"replay_verdict":"diverged"/' "$tmp/v.jsonl" > "$tmp/dv.jsonl"
    bind "$tmp/dv.jsonl" "$tmp/dv.md"
    validate_transcript "$tmp/dv.jsonl" "$tmp/dv.md" && fail "self-test: a diverged-verdict transcript was accepted"

    # unknown measurement point -> reject.
    sed 's/"point":"sustained"/"point":"totally_unknown"/' "$tmp/v.jsonl" > "$tmp/up.jsonl"
    bind "$tmp/up.jsonl" "$tmp/up.md"
    validate_transcript "$tmp/up.jsonl" "$tmp/up.md" && fail "self-test: an unknown-point transcript was accepted"

    # unknown event tag -> reject.
    printf '%s\n' \
        '{"event":"totally_unknown","slot":1}' \
        '{"event":"memory_summary","sample_count":1,"rss_p50_kib":1,"rss_p95_kib":1,"rss_peak_kib":1,"replay_verdict":"agreed"}' \
        '{"event":"block_admitted","slot":1,"block_hash_hex":"bb","prev_hash_hex":"aa","post_fp_hex":"cc","consensus_inputs_fingerprint_hex":"00"}' \
        '{"event":"memory_measure","point":"sustained","slot":1,"durable_tip_slot":1,"durable_tip_fp_hex":"cc","rss_kib":1}' > "$tmp/uk.jsonl"
    bind "$tmp/uk.jsonl" "$tmp/uk.md"
    validate_transcript "$tmp/uk.jsonl" "$tmp/uk.md" && fail "self-test: an unknown-tag transcript was accepted"

    # no block_admitted (starvation) -> reject.
    grep -v block_admitted "$tmp/v.jsonl" > "$tmp/st.jsonl"
    bind "$tmp/st.jsonl" "$tmp/st.md"
    validate_transcript "$tmp/st.jsonl" "$tmp/st.md" && fail "self-test: a no-block_admitted (starvation) transcript was accepted"

    # no memory_summary -> reject.
    grep -v memory_summary "$tmp/v.jsonl" > "$tmp/ns.jsonl"
    bind "$tmp/ns.jsonl" "$tmp/ns.md"
    validate_transcript "$tmp/ns.jsonl" "$tmp/ns.md" && fail "self-test: a no-summary transcript was accepted"

    # sha256 mismatch -> reject.
    cp "$tmp/v.jsonl" "$tmp/sm.jsonl"
    echo "mem-measure-a2 manifest; jsonl sha256: deadbeef" > "$tmp/sm.md"
    validate_transcript "$tmp/sm.jsonl" "$tmp/sm.md" && fail "self-test: a sha256-mismatch transcript was accepted"

    rm -rf "$tmp"
    if (( FAILED == 0 )); then
        echo "OK: mem-measure-a2 evidence schema self-test (accept valid; reject diverged/unknown-point/unknown-tag/starvation/no-summary/sha256-mismatch)"
    fi
    exit $FAILED
fi

# Default: validate the committed transcript(s) (vacuous if absent).
validate_transcript "$JSONL_DEFAULT" "$MD_DEFAULT" || fail "committed mem-measure-a2 transcript failed validation"
validate_transcript "$S1_JSONL" "$S1_MD" || fail "committed mem-opt-ops-s1 transcript failed validation"
validate_transcript "$S2_JSONL" "$S2_MD" || fail "committed mem-opt-ops-s2 transcript failed validation"
validate_transcript "$S3_JSONL" "$S3_MD" || fail "committed mem-opt-ops-s3 transcript failed validation"
validate_transcript "$UD_S0_JSONL" "$UD_S0_MD" || fail "committed mem-opt-utxo-disk-s0 transcript failed validation"
if (( FAILED == 0 )); then
    echo "OK: mem-measure evidence schema (vacuous-until-committed; A2 + MEM-OPT-OPS S1/S2/S3 transcripts; OP-MEM-01/OP-MEM-02, operator-gated)"
fi
exit $FAILED
