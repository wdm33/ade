#!/usr/bin/env bash
set -uo pipefail

# PHASE4-N-M-C S3 — admission wire-pump sole-authority closure
# (CN-PUMP-01) + emit-vocabulary closure (DC-PUMP-01) +
# Undecodable-tightening discipline (DC-ADMIT-12).
#
# Mechanical guards:
#   1. Exactly one `pub async fn run_admission_wire_pump` across
#      the workspace.
#   2. Exactly one `pub enum AdmissionPeerEvent` declared in the
#      runtime pump module (this struct is what the pump
#      produces; `ade_node::admission` has its OWN
#      `AdmissionPeerEvent` consumed by the runner — they're two
#      distinct types with parallel shape).
#   3. The wire-pump module MUST NOT reference
#      `AgreementVerdict` (DC-PUMP-01 — verdicts are GREEN
#      reducer output, never RED pump output).
#   4. The wire-pump module MUST emit `TipUpdate` for every
#      chain-sync reply carrying a `Tip` — a grep heuristic for
#      DC-PUMP-02. Concretely: each of `IntersectFound`,
#      `IntersectNotFound`, `RollForward`, `RollBackward` arms
#      must call `tip_update(...)` or build a
#      `AdmissionPeerEvent::TipUpdate`.
#   5. The runner's `ProcessedBlock::Undecodable` arm MUST NOT
#      return `AdmissionExitCode::Ok` — C3 strengthens
#      N-M-B's silent clean-exit path (DC-ADMIT-12).

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PUMP="$REPO_ROOT/crates/ade_runtime/src/admission/wire_pump.rs"
RUNNER="$REPO_ROOT/crates/ade_node/src/admission/runner.rs"

FAILED=0
print_fail() { echo "FAIL: $1"; FAILED=1; }

for f in "$PUMP" "$RUNNER"; do
    if [[ ! -f "$f" ]]; then
        print_fail "missing $f"
    fi
done
(( FAILED == 1 )) && exit "$FAILED"

# Guard 1.
sites=$(grep -rn --include='*.rs' -E '^pub async fn run_admission_wire_pump\b' "$REPO_ROOT/crates" 2>/dev/null || true)
n=$(echo "$sites" | grep -c -v '^$' 2>/dev/null || echo 0)
if [[ "$n" -ne 1 ]]; then
    print_fail "expected exactly 1 pub async fn run_admission_wire_pump, found $n:"
    echo "$sites"
fi

# Guard 2: runtime AdmissionPeerEvent enum sole instance.
enum_sites=$(grep -rn --include='*.rs' -E '^pub enum AdmissionPeerEvent\b' "$REPO_ROOT/crates/ade_runtime" 2>/dev/null || true)
ne=$(echo "$enum_sites" | grep -c -v '^$' 2>/dev/null || echo 0)
if [[ "$ne" -ne 1 ]]; then
    print_fail "expected exactly 1 pub enum AdmissionPeerEvent in ade_runtime, found $ne:"
    echo "$enum_sites"
fi

# Guard 3: no AgreementVerdict in the pump file (excluding
# comments).
pump_no_comments=$(awk '
    { line=$0; sub(/\/\/.*$/, "", line); print line }
' "$PUMP")
if echo "$pump_no_comments" | grep -E 'AgreementVerdict' >/dev/null 2>&1; then
    print_fail "wire_pump.rs must not reference AgreementVerdict in code (DC-PUMP-01):"
    echo "$pump_no_comments" | grep -nE 'AgreementVerdict'
fi

# Guard 4: every chain-sync reply emits a CLOSED authority event.
# IntersectFound / IntersectNotFound / RollForward emit `tip_update(` /
# `AdmissionPeerEvent::TipUpdate`; RollBackward emits the DISTINCT
# `AdmissionPeerEvent::RollBackward` (PHASE4-N-AI AI-S4a — "a rollback is NEVER a
# TipUpdate only"; the closed fork-choice / durable-rollback signal). DC-PUMP-02
# refined for AI-S4a in the PHASE4-N-AN gate triage: the invariant is "a closed
# event per reply", and the RollBackward reply's closed event is its own variant.
# Heuristic: the per-arm closed event must appear within the arm.
for arm in IntersectFound IntersectNotFound RollForward RollBackward; do
    if [[ "$arm" == "RollBackward" ]]; then
        ev='AdmissionPeerEvent::RollBackward'
    else
        ev='tip_update[(]|AdmissionPeerEvent::TipUpdate'
    fi
    if ! awk -v needle="$arm" -v ev="$ev" '
        $0 ~ ("ChainSyncMessage::"needle) { found_arm=1; ctx=0 }
        found_arm && ctx < 20 { ctx++; if ($0 ~ ev) print "ok"; }
        found_arm && /^[[:space:]]*\}/ { found_arm=0 }
    ' "$PUMP" | grep -q '^ok$'; then
        print_fail "wire_pump.rs: chain-sync $arm arm does not emit its closed authority event [$ev] (DC-PUMP-02)"
    fi
done

# Guard 5: runner Undecodable arm must NOT return
# AdmissionExitCode::Ok AND must reference at least one of the
# C3-tightened exit paths. Heuristic: look at the 60 lines
# following the `ProcessedBlock::Undecodable =>` arm header.
und_block=$(awk '
    /ProcessedBlock::Undecodable[[:space:]]*=>/ { found=1; n=0 }
    found {
        print
        n++
        if (n >= 60) exit
    }
' "$RUNNER")
if [[ -z "$und_block" ]]; then
    print_fail "runner.rs: ProcessedBlock::Undecodable arm not found"
else
    if echo "$und_block" | grep -qE 'return[[:space:]]+AdmissionExitCode::Ok\b'; then
        print_fail "runner.rs: Undecodable arm returns AdmissionExitCode::Ok (DC-ADMIT-12 / ¬P-C9 violated)"
    fi
    if ! echo "$und_block" | grep -qE 'AdmissionExitCode::(Diverged|PeerSentUndecodableBytes)'; then
        print_fail "runner.rs: Undecodable arm does not route to Diverged or PeerSentUndecodableBytes (DC-ADMIT-12)"
    fi
fi

if (( FAILED == 0 )); then
    echo "OK: sole admission wire-pump entry + closed verdict-free emit set + Undecodable tightening (CN-PUMP-01, DC-PUMP-01, DC-PUMP-02, DC-ADMIT-12)"
fi
exit $FAILED
