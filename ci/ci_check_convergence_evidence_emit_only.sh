#!/usr/bin/env bash
set -uo pipefail

# PHASE4-N-AJ AJ-S2 (DC-NODE-30) — convergence evidence is EMIT-ONLY.
#
# Evidence observes authority; evidence never becomes authority. The verdict /
# emit result MUST NEVER feed the participant routing (classify_receive /
# apply_chain_event / pump_block / fork-choice / forge), and a sink write error
# MUST NEVER halt authority (it is non-fatal, surfaced via the incomplete flag).

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
EV="$REPO_ROOT/crates/ade_node/src/convergence_evidence.rs"
NL="$REPO_ROOT/crates/ade_node/src/node_lifecycle.rs"

FAILED=0
fail() { echo "FAIL: $1"; FAILED=1; }

[[ -f "$EV" ]] || fail "missing $EV"
[[ -f "$NL" ]] || fail "missing $NL"

# Guard 1: the convergence-evidence module is pure GREEN emission -- it touches
# NO authority surface (it compares already-authoritative outputs, never decides).
# Match CALL-form (`symbol(`) so explanatory doc comments may name these fns.
if [[ -f "$EV" ]]; then
    for sym in classify_receive resolve_disposition apply_chain_event pump_block \
               select_best_chain fork_choice commit_rollback; do
        if grep -qE "\b${sym}\(" "$EV"; then
            fail "convergence_evidence.rs calls authority fn '$sym' (evidence must not become authority)"
        fi
    done
    # The evidence sink is distinct from the authoritative WAL.
    if grep -qE '\bWalStore\b|\bwal::' "$EV"; then
        fail "convergence_evidence.rs uses the WAL (the evidence sink is distinct from the authoritative WAL)"
    fi
fi

# Guard 2: in run_participant_sync, the evidence emit calls are EMIT-ONLY --
# statement-position, never `?`-propagated into the authoritative Result, never
# the scrutinee of an if/match (control flow).
if [[ -f "$NL" ]]; then
    if ! grep -qE 'emit_participant_admit\([^;]*\);' "$NL"; then
        fail "emit_participant_admit is not called in statement position in node_lifecycle.rs"
    fi
    if grep -qE 'emit_participant_admit\([^;]*\)\?' "$NL"; then
        fail "emit_participant_admit result is ?-propagated -- a write error must NOT halt authority"
    fi
    if grep -qE 'emit_block_received\([^;]*\)\?' "$NL"; then
        fail "emit_block_received result is ?-propagated -- a write error must NOT halt authority"
    fi
    if grep -qE '\b(if|match|while)\s+[A-Za-z_.]*emit_(participant_admit|block_received|block_admitted|agreement_verdict)\b' "$NL"; then
        fail "an evidence emit result drives control flow in node_lifecycle.rs (must be emit-only)"
    fi
fi

if (( FAILED == 0 )); then
    echo "OK: convergence evidence is emit-only (no authority symbols in the module; emit results never ?-propagate or drive control flow)"
fi
exit $FAILED
