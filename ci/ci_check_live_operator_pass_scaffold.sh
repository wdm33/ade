#!/usr/bin/env bash
set -uo pipefail

# PHASE4-N-M-C S5 — operator-pass live evidence scaffolding
# (DC-EVIDENCE-01).
#
# Mechanical guards:
#   1. The live-pass integration test exists at
#      `crates/ade_node/tests/admission_live_operator_pass.rs`
#      AND is env-gated by ADE_LIVE_OPERATOR_TEST=1.
#   2. The operator-side bundle generator exists at
#      `ci/build_consensus_inputs_bundle.sh` AND is executable.
#   3. The committed consensus-inputs bundle exists at
#      `docs/evidence/phase4-n-m-c-consensus-inputs.json` AND
#      is valid JSON.
#   4. The wire-integration anchor transcript exists at
#      `docs/evidence/phase4-n-m-c-wire-only-transcript.jsonl`
#      AND contains the closed wire-only vocabulary
#      (`peer_dial_started`, `handshake_ok`, `peer_tip_read`,
#      `wire_smoke_complete`).
#   5. The operator runbook exists at
#      `docs/evidence/phase4-n-m-c-operator-pass-README.md`.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TEST="$REPO_ROOT/crates/ade_node/tests/admission_live_operator_pass.rs"
GEN="$REPO_ROOT/ci/build_consensus_inputs_bundle.sh"
BUNDLE="$REPO_ROOT/docs/evidence/phase4-n-m-c-consensus-inputs.json"
WIRE_TRANSCRIPT="$REPO_ROOT/docs/evidence/phase4-n-m-c-wire-only-transcript.jsonl"
RUNBOOK="$REPO_ROOT/docs/evidence/phase4-n-m-c-operator-pass-README.md"

FAILED=0
print_fail() { echo "FAIL: $1"; FAILED=1; }

# Guard 1.
if [[ ! -f "$TEST" ]]; then
    print_fail "missing live-pass test: $TEST"
elif ! grep -qE 'ADE_LIVE_OPERATOR_TEST' "$TEST"; then
    print_fail "live-pass test missing ADE_LIVE_OPERATOR_TEST env-gate"
fi

# Guard 2.
if [[ ! -x "$GEN" ]]; then
    print_fail "bundle generator missing or not executable: $GEN"
fi

# Guard 3.
if [[ ! -f "$BUNDLE" ]]; then
    print_fail "consensus-inputs bundle missing: $BUNDLE"
else
    if ! python3 -c "import json; json.load(open('$BUNDLE'))" 2>/dev/null; then
        print_fail "consensus-inputs bundle is not valid JSON: $BUNDLE"
    fi
fi

# Guard 4.
if [[ ! -f "$WIRE_TRANSCRIPT" ]]; then
    print_fail "wire-integration anchor transcript missing: $WIRE_TRANSCRIPT"
else
    for lit in "peer_dial_started" "handshake_ok" "peer_tip_read" "wire_smoke_complete"; do
        if ! grep -qE "\"$lit\"" "$WIRE_TRANSCRIPT"; then
            print_fail "wire-only transcript missing literal: $lit"
        fi
    done
fi

# Guard 5.
if [[ ! -f "$RUNBOOK" ]]; then
    print_fail "operator-pass runbook missing: $RUNBOOK"
fi

if (( FAILED == 0 )); then
    echo "OK: operator-pass scaffolding + live wire-integration anchor + runbook + bundle (DC-EVIDENCE-01 scaffolding tier)"
fi
exit $FAILED
