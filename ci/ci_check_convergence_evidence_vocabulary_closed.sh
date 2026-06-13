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

# The closed convergence vocabulary (the only AdmissionLogEvent variants the sink
# may construct) and their JSONL discriminators. The AJ-era subset was the 3
# AgreementVerdict events; PHASE4-N-AO DELIBERATELY broadened the convergence
# transcript to also carry the closed fork-choice SELECT vocabulary (S9 / DC-EVIDENCE-04),
# the missing-bridge fail-closed event (S11 / DC-NODE-39), and the range re-fetch
# recovery events (S14 / DC-NODE-41) -- these ARE convergence evidence, each is a
# closed discriminant in the writer's DISCRIMINATORS allow-list (cross-checked in
# Guard 5). Lifecycle events stay FORBIDDEN. This gate is the per-sink closure check
# for DC-NODE-30; the per-event closure of the S9/S11/S14 vocabulary is additionally
# enforced by ci_check_fork_choice_evidence_closed.sh / ci_check_missing_bridge_fail_closed.sh
# / ci_check_missing_bridge_refetch.sh.
ALLOWED_VARIANTS=("BlockReceived" "BlockAdmitted" "AgreementVerdict" \
  "NeedsForkChoice" "LcaDiscovered" "CandidateFragmentBuilt" "ForkChoiceSelected" \
  "BranchFetchStarted" "BranchFetchCompleted" "BranchPrevalidated" \
  "ForkSwitchApplied" "ForkSwitchFailed" "ForkSwitchSuperseded" \
  "MissingBridge" "RangeRefetchStarted" "RangeRefetchCompleted")
ALLOWED_LITERALS=("block_received" "block_admitted" "agreement_verdict" \
  "needs_fork_choice" "lca_discovered" "candidate_fragment_built" "fork_choice_selected" \
  "branch_fetch_started" "branch_fetch_completed" "branch_prevalidated" \
  "fork_switch_applied" "fork_switch_failed" "fork_switch_superseded" \
  "missing_bridge" "range_refetch_started" "range_refetch_completed")

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

# Guard 5: every allowed convergence literal is a CLOSED discriminator in the
# writer's DISCRIMINATORS allow-list (crates/ade_node/src/admission_log/writer.rs) --
# the authoritative closed-vocabulary source of truth. (Repointed at PHASE4-N-AO from
# the AI-era ci_check_convergence_evidence_schema.sh, whose ALLOWED regex is the
# narrower single-best-peer AI transcript vocabulary and predates the S9/S11/S14
# convergence events; the AO convergence transcript is validated by the
# ci_check_post_switch_convergence_window.sh reducer, not the AI schema gate.) This
# keeps the sink's vocabulary provably a subset of the closed admission_log
# discriminator set -- no free-form / open vocabulary may slip in.
WRITER_FILE="$REPO_ROOT/crates/ade_node/src/admission_log/writer.rs"
if [[ -f "$WRITER_FILE" ]]; then
    for lit in "${ALLOWED_LITERALS[@]}"; do
        if ! grep -qE "\"$lit\"" "$WRITER_FILE"; then
            fail "convergence literal \"$lit\" is not a closed discriminator in the writer DISCRIMINATORS allow-list ($WRITER_FILE)"
        fi
    done
else
    fail "missing $WRITER_FILE (the closed-vocabulary source of truth)"
fi

if (( FAILED == 0 )); then
    echo "OK: convergence-evidence vocabulary closed to the AJ AgreementVerdict + S9/S11/S14 fork-choice/missing-bridge/range-refetch closed set; no lifecycle events; no raw-writer accessor; every literal in the writer DISCRIMINATORS allow-list"
fi
exit $FAILED
