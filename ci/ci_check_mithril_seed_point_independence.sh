#!/usr/bin/env bash
set -uo pipefail

# PHASE4-N-Z S1 — Mithril bootstrap seed-point independence
# (DC-MITHRIL-02) + verify-before-bootstrap call-order
# (CN-MITHRIL-01, strengthened).
#
# Mechanical guards on the production composition
# `bootstrap_from_mithril_snapshot`:
#   (a) POSITIVE call-order: `verify_mithril_binding(` appears BEFORE
#       `bootstrap_initial_state(` in the composition source — the
#       binding must be checked (and Ok) before any storage init.
#   (b) NEGATIVE source-origin: the composition's `MintInputs`
#       `seed_slot:` / `seed_block_hash:` are NOT assigned from
#       `import.report.*`, `.certified_point`, `provenance`, or
#       `SeedProvenance::Mithril` — the anchor's seed_point must come
#       from the operator seed-point parameter, never the manifest.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
COMPOSITION="$REPO_ROOT/crates/ade_runtime/src/mithril_bootstrap.rs"

FAILED=0
print_fail() { echo "FAIL: $1"; FAILED=1; }

# Strip the #[cfg(test)] module (everything from the test attribute to
# EOF) and line comments, so guards only see the production composition.
strip_for_grep() {
    awk '
        /^#\[cfg\(test\)\]/ { in_test=1 }
        in_test { next }
        { line=$0; sub(/\/\/.*$/, "", line); print line }
    ' "$1"
}

if [[ ! -f "$COMPOSITION" ]]; then
    print_fail "missing composition source $COMPOSITION"
    echo "FAIL: ci_check_mithril_seed_point_independence"
    exit 1
fi

BODY="$(strip_for_grep "$COMPOSITION")"

# Guard (a): verify_mithril_binding precedes bootstrap_initial_state.
VERIFY_LINE=$(echo "$BODY" | grep -nE '\bverify_mithril_binding\(' | head -n1 | cut -d: -f1)
BOOTSTRAP_LINE=$(echo "$BODY" | grep -nE '\bbootstrap_initial_state\(' | head -n1 | cut -d: -f1)

if [[ -z "$VERIFY_LINE" ]]; then
    print_fail "composition does not call verify_mithril_binding("
fi
if [[ -z "$BOOTSTRAP_LINE" ]]; then
    print_fail "composition does not call bootstrap_initial_state("
fi
if [[ -n "$VERIFY_LINE" && -n "$BOOTSTRAP_LINE" ]]; then
    if (( VERIFY_LINE >= BOOTSTRAP_LINE )); then
        print_fail "verify_mithril_binding (line $VERIFY_LINE) must precede bootstrap_initial_state (line $BOOTSTRAP_LINE)"
    fi
fi

# Guard (b): the MintInputs seed_slot/seed_block_hash assignments must
# NOT trace to the manifest import. Extract each assignment's RHS and
# fail if it mentions a manifest-origin token.
MANIFEST_ORIGIN_RE='(\breport\b|\.certified_point\b|\bprovenance\b|SeedProvenance::Mithril|\bimport\.)'

check_field_origin() {
    local field="$1"
    # The RHS of `field: <expr>,` on its own line.
    local rhs
    rhs=$(echo "$BODY" | grep -E "\b${field}:" | sed -E "s/.*\b${field}: *//")
    if [[ -z "$rhs" ]]; then
        print_fail "could not find MintInputs.${field} assignment in composition"
        return
    fi
    if echo "$rhs" | grep -qE "$MANIFEST_ORIGIN_RE"; then
        print_fail "MintInputs.${field} is sourced from the manifest import (DC-MITHRIL-02): $rhs"
    fi
}

check_field_origin "seed_slot"
check_field_origin "seed_block_hash"

if (( FAILED == 0 )); then
    echo "OK: Mithril bootstrap mints seed_point from operator inputs only (DC-MITHRIL-02); verify_mithril_binding precedes bootstrap_initial_state (CN-MITHRIL-01)"
fi
exit $FAILED
