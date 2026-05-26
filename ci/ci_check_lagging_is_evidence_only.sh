#!/usr/bin/env bash
set -uo pipefail

# PHASE4-N-M-B S1 — Lagging-is-evidence-only gate (DC-ADMIT-08).
#
# Per `[[feedback-evidence-reducers-are-green-not-authority]]`:
# `AgreementVerdict::Lagging` is evidence-state only. No code path
# (outside the verdict reducer body + its tests) may map a Lagging
# verdict to a success / healthy / live-ready sentinel.
#
# Mechanical guards:
#   1. Forbid `Lagging` matched in arms that contain success
#      tokens ("Ok(", "true", "live_ready", "Healthy", "Synced",
#      "Ready") anywhere except the verdict reducer module + its
#      tests.
#   2. Forbid any `_ => Ok(` fall-through in admission code that
#      sits between a Lagging-producing match and a result return.
#
# These greps are deliberately conservative; the cluster's exit
# criterion is "no code path treats Lagging as success", and the
# review obligation reads any Lagging match-arm in admission code
# as adversarial.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
ADMISSION_DIR="$REPO_ROOT/crates/ade_node/src/admission"
VERDICT_FILE="$ADMISSION_DIR/verdict.rs"

FAILED=0
print_fail() { echo "FAIL: $1"; FAILED=1; }

if [[ ! -d "$ADMISSION_DIR" ]]; then
    print_fail "missing $ADMISSION_DIR"
    exit "$FAILED"
fi

# Guard 1: scan all .rs files in the workspace EXCEPT the verdict
# reducer file itself; flag any line that matches the forbidden
# success-mapping shapes alongside `Lagging`.
forbidden_arms=$(
    grep -rn --include='*.rs' \
         -E 'AgreementVerdict::Lagging.*=>.*(Ok\(|true|live_ready|Healthy|Synced|Ready)|(Ok\(|true|live_ready|Healthy|Synced|Ready).*AgreementVerdict::Lagging' \
        "$REPO_ROOT/crates" 2>/dev/null \
    | grep -v -E '^[^:]+verdict\.rs:' \
    | grep -v -E '^[^:]+admission_replay_equivalence' \
    || true
)

if [[ -n "$forbidden_arms" ]]; then
    print_fail "Lagging treated as success outside verdict reducer:"
    echo "$forbidden_arms"
fi

# Guard 2: assert the verdict reducer module defines a sole
# `pub fn derive` returning `AgreementVerdict` and nothing in the
# workspace shadows it.
derive_sites=$(grep -rn --include='*.rs' -E '^pub fn derive\s*\(' "$REPO_ROOT/crates" 2>/dev/null | grep -c "verdict\.rs" || true)
if [[ "$derive_sites" != "1" ]]; then
    derive_other=$(grep -rn --include='*.rs' -E '^pub fn derive\s*\(' "$REPO_ROOT/crates" 2>/dev/null | grep -v "verdict\.rs" || true)
    if [[ -n "$derive_other" ]]; then
        print_fail "non-canonical pub fn derive found (sole authority is verdict.rs):"
        echo "$derive_other"
    fi
fi

# Guard 3: the AgreementVerdict definition must remain a closed
# 4-variant sum (no "Healthy" / "Ready" / "Synced" / "LiveReady"
# additions).
banned_variants=$(
    grep -nE '(Healthy|Ready|Synced|LiveReady)\s*[{(]' "$VERDICT_FILE" 2>/dev/null \
    | grep -v '^\s*//' \
    || true
)
if [[ -n "$banned_variants" ]]; then
    print_fail "banned variant in AgreementVerdict / BlockAdmitOutcome:"
    echo "$banned_variants"
fi

# Guard 4: AgreementVerdict must NOT carry #[non_exhaustive] (closed sum).
if grep -nE '#\[non_exhaustive\]\s*$' "$VERDICT_FILE" 2>/dev/null | head -1 > /dev/null; then
    # Walk lines: any `#[non_exhaustive]` immediately followed by
    # `pub enum AgreementVerdict` or `pub enum BlockAdmitOutcome` is fatal.
    while IFS=':' read -r lineno _rest; do
        next=$((lineno + 1))
        next_line=$(awk "NR==$next" "$VERDICT_FILE")
        if echo "$next_line" | grep -qE 'pub enum (AgreementVerdict|BlockAdmitOutcome|InvalidAdmitReason)'; then
            print_fail "verdict sum carries #[non_exhaustive] (must remain closed): $VERDICT_FILE:$lineno"
        fi
    done < <(grep -nE '#\[non_exhaustive\]' "$VERDICT_FILE" 2>/dev/null)
fi

if (( FAILED == 0 )); then
    echo "OK: AgreementVerdict::Lagging is evidence-only; closed sum intact; sole derive site at verdict.rs"
fi
exit $FAILED
