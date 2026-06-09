#!/usr/bin/env bash
set -uo pipefail

# PHASE4-N-AI AI-S5 (CE-AI-6, CN-CONS-03) — convergence-pass evidence schema
# (operator-gated, derived-tier). VACUOUS-UNTIL-COMMITTED: passes when the
# transcript is absent; strict when present. Reuses the existing live-log /
# AgreementVerdict vocabulary (no new evidence enum). The convergence-through-
# reorg assertion: closed vocabulary + sha256-binding + 0 Diverged + >=1 slot
# regression (the peer rollback was followed) + no boring same-tip-only run.
#
# Run `--self-test` to validate the validator against temp fixtures.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
EV_DIR="$REPO_ROOT/docs/evidence"
JSONL_DEFAULT="$EV_DIR/phase4-n-ai-convergence-pass.jsonl"
MD_DEFAULT="$EV_DIR/phase4-n-ai-convergence-pass.md"

FAILED=0
fail() { echo "FAIL: $1"; FAILED=1; }

# Closed vocabulary of known live-log event tags (the existing --mode node /
# admission transcript vocabulary; no new evidence enum is introduced).
ALLOWED='^(admission_started|admission_shutdown|bootstrap_complete|node_started|node_shutdown|handshake_ok|peer_dial_started|block_received|peer_tip_read|peer_chain_tip|agreement_verdict|block_admitted|bar_verdict|bar_summary|catch_up|caught_up|relay_adopted|relay_adoption|run_meta|provenance|stop|immutable_settlement)$'

# validate_transcript <jsonl> <md> : 0 = a valid convergence-through-reorg
# transcript (or absent → vacuous); non-zero = reject.
validate_transcript() {
    local jsonl="$1" md="$2" rc=0
    [[ -f "$jsonl" ]] || return 0   # vacuous-until-committed

    # (a) closed vocabulary — every event tag is in the allow-list.
    local tag
    while IFS= read -r tag; do
        grep -qE "$ALLOWED" <<< "$tag" || { echo "  unknown event tag: '$tag'"; rc=1; }
    done < <(grep -oE '"event":"[a-z_]+"' "$jsonl" | sed -E 's/.*:"([a-z_]+)"/\1/')

    # (b) no Diverged verdict (Ade never disagreed with the peer).
    if grep -qE 'diverged' "$jsonl"; then
        echo "  a Diverged verdict is present -- not a convergence pass"; rc=1
    fi

    # (c) >=1 slot regression in the received-block/tip sequence -- the peer
    #     rollback was actually FOLLOWED (a monotonic, boring same-tip run is
    #     NOT sufficient for CE-AI-6, which proves convergence THROUGH a reorg).
    # A reorg is a STRICT slot decrease (the tip went backward); a same-slot line
    # (e.g. a verdict at the last block's slot) is not a regression.
    local regressed=0 maxslot=-1 sl
    while IFS= read -r sl; do
        if (( sl < maxslot )); then regressed=1; fi
        if (( sl > maxslot )); then maxslot=$sl; fi
    done < <(grep -oE '"slot":[0-9]+' "$jsonl" | grep -oE '[0-9]+$')
    (( regressed == 1 )) \
        || { echo "  no slot regression -> no reorg-follow exercised (a boring same-tip run is not CE-AI-6)"; rc=1; }

    # (d) sha256-binding: the .md manifest carries the .jsonl's sha256.
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
    bind() { echo "convergence-pass manifest; jsonl sha256: $(sha256sum "$1" | cut -d' ' -f1)" > "$2"; }

    # valid: a reorg (slot 12 after max 13) + final agreement + closed vocab + bound.
    printf '%s\n' \
        '{"event":"node_started"}' \
        '{"event":"block_received","slot":11}' \
        '{"event":"block_received","slot":12}' \
        '{"event":"block_received","slot":13}' \
        '{"event":"peer_tip_read","slot":12}' \
        '{"event":"block_received","slot":13}' \
        '{"event":"agreement_verdict","slot":13}' > "$tmp/v.jsonl"
    bind "$tmp/v.jsonl" "$tmp/v.md"
    validate_transcript "$tmp/v.jsonl" "$tmp/v.md" || fail "self-test: a valid reorg-follow transcript was rejected"

    # no reorg (monotonic) -> reject (convergence_gate_rejects_transcript_without_rollback_follow).
    printf '%s\n' \
        '{"event":"block_received","slot":11}' \
        '{"event":"block_received","slot":12}' \
        '{"event":"agreement_verdict","slot":12}' > "$tmp/nr.jsonl"
    bind "$tmp/nr.jsonl" "$tmp/nr.md"
    validate_transcript "$tmp/nr.jsonl" "$tmp/nr.md" && fail "self-test: a no-reorg (boring) transcript was accepted"

    # diverged -> reject.
    printf '%s\n' \
        '{"event":"block_received","slot":13}' \
        '{"event":"block_received","slot":12}' \
        '{"event":"agreement_verdict","verdict":"diverged","slot":12}' > "$tmp/dv.jsonl"
    bind "$tmp/dv.jsonl" "$tmp/dv.md"
    validate_transcript "$tmp/dv.jsonl" "$tmp/dv.md" && fail "self-test: a Diverged transcript was accepted"

    # unknown tag -> reject.
    printf '%s\n' \
        '{"event":"totally_unknown","slot":13}' \
        '{"event":"block_received","slot":12}' > "$tmp/uk.jsonl"
    bind "$tmp/uk.jsonl" "$tmp/uk.md"
    validate_transcript "$tmp/uk.jsonl" "$tmp/uk.md" && fail "self-test: an unknown-tag transcript was accepted"

    # sha256 mismatch -> reject.
    cp "$tmp/v.jsonl" "$tmp/sm.jsonl"
    echo "convergence-pass manifest; jsonl sha256: deadbeef" > "$tmp/sm.md"
    validate_transcript "$tmp/sm.jsonl" "$tmp/sm.md" && fail "self-test: a sha256-mismatch transcript was accepted"

    rm -rf "$tmp"
    if (( FAILED == 0 )); then
        echo "OK: convergence evidence schema self-test (accept valid; reject no-reorg/diverged/unknown-tag/sha256-mismatch)"
    fi
    exit $FAILED
fi

# Default: validate the committed transcript (vacuous if absent).
validate_transcript "$JSONL_DEFAULT" "$MD_DEFAULT" || fail "committed convergence transcript failed validation"
if (( FAILED == 0 )); then
    echo "OK: convergence evidence schema (vacuous-until-committed; CE-AI-6, operator-gated)"
fi
exit $FAILED
