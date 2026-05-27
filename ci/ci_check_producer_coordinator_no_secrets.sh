#!/usr/bin/env bash
set -euo pipefail

# PHASE4-N-Q S2 — coordinator state has no secret-key material.
#
# Mechanically enforces N9 (cluster doc §5; CN-PROD-02 statement):
# The GREEN coordinator never owns or stores private signing material.
# Adding a KesSecret / VrfSigningKey / ColdSigningKey field to
# CoordinatorState (or any other type in coordinator.rs) is a
# compile-time + grep-time error.
#
# Guards:
#   1. coordinator.rs (production lines only) contains no
#      KesSecret / VrfSigningKey / ColdSigningKey / Sum6Kes::SigningKey
#      identifier.
#   2. coordinator.rs imports no producer::signing module.
#   3. coordinator.rs imports no cardano_crypto::kes module.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
COORDINATOR_RS="$REPO_ROOT/crates/ade_runtime/src/producer/coordinator.rs"

if [ ! -f "$COORDINATOR_RS" ]; then
    echo "[ci_check_producer_coordinator_no_secrets] FAIL: missing $COORDINATOR_RS"
    exit 1
fi

# Production-only lines: stop emitting at the first top-level
# `#[cfg(test)]` attribute, and exclude pure comment lines (//, ///, //!).
# Doc comments explaining the prohibition are not violations; the
# guard fires only on code references.
emit_production_lines() {
    awk '
        /^#\[cfg\(test\)\]/ { exit }
        # Strip leading whitespace, then skip lines that start with //
        # (handles //, ///, //!).
        {
            trimmed = $0
            sub(/^[[:space:]]+/, "", trimmed)
            if (trimmed ~ /^\/\//) next
            print NR ":" $0
        }
    ' "$1"
}

FAILED=0
print_fail() {
    echo "[ci_check_producer_coordinator_no_secrets] FAIL: $1"
    FAILED=1
}

PROD_LINES="$(emit_production_lines "$COORDINATOR_RS")"

# Guard 1: forbidden identifiers in production.
FORBIDDEN_IDENTIFIERS=(
    'KesSecret'
    'VrfSigningKey'
    'ColdSigningKey'
    'Sum6Kes::SigningKey'
    'Sum0SigningKey'
    'SumSigningKey'
    'ZeroizingSeed'
)
for id in "${FORBIDDEN_IDENTIFIERS[@]}"; do
    matches=$(echo "$PROD_LINES" | grep -F "$id" || true)
    if [ -n "$matches" ]; then
        print_fail "Guard 1 — coordinator.rs production scope references forbidden secret-key identifier: $id"
        echo "$matches"
    fi
done

# Guard 2: no producer::signing import in production scope.
SIGNING_IMPORTS=$(echo "$PROD_LINES" | grep -E 'use\s+(crate::)?(super::)?signing|use\s+ade_runtime::producer::signing' || true)
if [ -n "$SIGNING_IMPORTS" ]; then
    print_fail "Guard 2 — coordinator.rs imports producer::signing in production scope"
    echo "$SIGNING_IMPORTS"
fi

# Guard 3: no cardano_crypto::kes import in production scope.
CC_KES_IMPORTS=$(echo "$PROD_LINES" | grep -E 'use\s+cardano_crypto::kes|use\s+ade_crypto::kes_sum' || true)
if [ -n "$CC_KES_IMPORTS" ]; then
    print_fail "Guard 3 — coordinator.rs imports a KES algorithm module in production scope (coordinator must not see signing primitives)"
    echo "$CC_KES_IMPORTS"
fi

if [ "$FAILED" -eq 0 ]; then
    echo "[ci_check_producer_coordinator_no_secrets] PASS (3/3 guards green)"
    exit 0
else
    exit 1
fi
