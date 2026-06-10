#!/usr/bin/env bash
set -uo pipefail

# PHASE4-N-AJ AJ-S1 — convergence-evidence vocabulary closure + isolation
# (DC-ADMIT-04 strengthening / DC-NODE-30 sink half).
#
# The convergence-evidence transcript (--convergence-evidence-path, CE-AI-6) is
# a NARROW evidence file, not a lifecycle log. The ConvergenceEvidenceSink
# wrapper restricts it (the compiler half) to the closed 3-variant subset of
# the REUSED AdmissionLogEvent — no new evidence enum:
#   block_received / block_admitted / agreement_verdict.
# This grep is the file-tree half: it asserts the sink module constructs ONLY
# those variants, exposes no raw-writer accessor (which would bypass the
# subset), carries no sched/forge/wire-only literals, and reuses only
# vocabulary the schema gate already allows.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SINK_FILE="$REPO_ROOT/crates/ade_node/src/convergence_evidence.rs"
SCHEMA_GATE="$REPO_ROOT/ci/ci_check_convergence_evidence_schema.sh"

FAILED=0
fail() { echo "FAIL: $1"; FAILED=1; }

[[ -f "$SINK_FILE" ]] || fail "missing $SINK_FILE"
[[ -f "$SCHEMA_GATE" ]] || fail "missing $SCHEMA_GATE"

# The closed convergence subset (the only AdmissionLogEvent variants the sink
# may construct) and their JSONL discriminators.
ALLOWED_VARIANTS=("BlockReceived" "BlockAdmitted" "AgreementVerdict")
ALLOWED_LITERALS=("block_received" "block_admitted" "agreement_verdict")

# Variants that MUST NOT be constructed in the convergence sink (the rest of
# the AdmissionLogEvent enum — admission lifecycle events).
FORBIDDEN_VARIANTS=("AdmissionStarted" "SnapshotImported" "BootstrapComplete" "AdmissionHalted" "AdmissionShutdown")

if [[ -f "$SINK_FILE" ]]; then
    # Guard 1: NONE of the forbidden AdmissionLogEvent variants is constructed.
    for v in "${FORBIDDEN_VARIANTS[@]}"; do
        if grep -qE "AdmissionLogEvent::$v" "$SINK_FILE"; then
            fail "convergence sink constructs forbidden variant AdmissionLogEvent::$v"
        fi
    done

    # Guard 2: every AdmissionLogEvent::<Variant> constructed is in the allowed
    # subset (catches a future variant slipping in).
    while IFS= read -r v; do
        ok=0
        for a in "${ALLOWED_VARIANTS[@]}"; do [[ "$v" == "$a" ]] && ok=1; done
        (( ok == 1 )) || fail "convergence sink constructs non-subset variant AdmissionLogEvent::$v"
    done < <(grep -oE 'AdmissionLogEvent::[A-Za-z]+' "$SINK_FILE" | sed -E 's/AdmissionLogEvent:://' | sort -u)

    # Guard 3: no raw-writer accessor (which would bypass the closed subset) —
    # no into_inner / as_writer / writer() method, no pub fn returning an
    # AdmissionLogWriter, and the `inner` field stays private.
    if grep -qE '\bfn +(into_inner|as_writer|writer)\b' "$SINK_FILE"; then
        fail "convergence sink exposes a raw-writer accessor (bypasses the closed subset)"
    fi
    if grep -qE 'pub +fn +[A-Za-z_]+\([^)]*\)[^{]*->[^{]*AdmissionLogWriter' "$SINK_FILE"; then
        fail "convergence sink has a pub fn returning AdmissionLogWriter (bypasses the closed subset)"
    fi
    if grep -qE 'pub +inner\b' "$SINK_FILE"; then
        fail "convergence sink exposes its inner writer field as pub"
    fi

    # Guard 4: no sched/forge/wire-only literals (it is an evidence file, not a
    # lifecycle/scheduling log).
    for lit in "forge_tick" "forge_base_selected" "forge_result" "sched_" "peer_dial_started" "handshake_ok" "wire_smoke_complete"; do
        if grep -qE "\"$lit" "$SINK_FILE"; then
            fail "convergence sink contains non-evidence literal \"$lit\""
        fi
    done
fi

# Guard 5: the allowed literals are a SUBSET of the schema gate's ALLOWED set
# (no new vocabulary; a convergence file passes ci_check_convergence_evidence_schema.sh).
if [[ -f "$SCHEMA_GATE" ]]; then
    for lit in "${ALLOWED_LITERALS[@]}"; do
        if ! grep -qE "$lit" "$SCHEMA_GATE"; then
            fail "convergence literal \"$lit\" is not in the schema gate's ALLOWED set ($SCHEMA_GATE)"
        fi
    done
fi

if (( FAILED == 0 )); then
    echo "OK: convergence-evidence vocabulary closed to {block_received, block_admitted, agreement_verdict}; no raw-writer accessor; subset of the schema gate ALLOWED"
fi
exit $FAILED
