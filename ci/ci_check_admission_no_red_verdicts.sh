#!/usr/bin/env bash
set -uo pipefail

# PHASE4-N-M-C S3 — RED admission stack never synthesizes
# AgreementVerdict (DC-PUMP-01 / ¬P-C3).
#
# The verdict is GREEN evidence (see
# `[[feedback-evidence-reducers-are-green-not-authority]]`); the
# only call site for `verdict::derive` lives in the admission
# RUNNER (which is also RED, but composes GREEN evidence —
# distinct from the wire pump, which moves bytes only).
#
# Mechanical guards:
#   1. `crates/ade_runtime/src/admission/wire_pump.rs` MUST NOT
#      mention `AgreementVerdict` or `derive_verdict` outside of
#      comments.
#   2. `crates/ade_runtime/src/admission/` MUST NOT contain a
#      file that imports `ade_node::admission::verdict` (RED
#      runtime cannot reach into the ade_node verdict adapter).

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PUMP_DIR="$REPO_ROOT/crates/ade_runtime/src/admission"

FAILED=0
print_fail() { echo "FAIL: $1"; FAILED=1; }

if [[ ! -d "$PUMP_DIR" ]]; then
    print_fail "missing $PUMP_DIR"
    exit "$FAILED"
fi

strip_for_grep() {
    awk '
        { line=$0; sub(/\/\/.*$/, "", line); print line }
    ' "$1"
}

# Guard 1.
for f in $(find "$PUMP_DIR" -type f -name '*.rs'); do
    body=$(strip_for_grep "$f")
    if echo "$body" | grep -E 'AgreementVerdict|derive_verdict\(' >/dev/null 2>&1; then
        print_fail "RED admission module references verdict-derivation in code (DC-PUMP-01): $f"
        echo "$body" | grep -nE 'AgreementVerdict|derive_verdict\(' || true
    fi
done

# Guard 2.
for f in $(find "$PUMP_DIR" -type f -name '*.rs'); do
    if grep -E 'ade_node::admission::verdict|ade_node::admission_log' "$f" >/dev/null 2>&1; then
        print_fail "RED runtime admission module reaches into ade_node namespace (boundary violation): $f"
        grep -nE 'ade_node::' "$f"
    fi
done

if (( FAILED == 0 )); then
    echo "OK: RED admission stack carries no verdict synthesis path (DC-PUMP-01 / ¬P-C3)"
fi
exit $FAILED
