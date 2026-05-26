#!/usr/bin/env bash
set -uo pipefail

# PHASE4-N-M-C S1a — single LiveConsensusInputs importer authority
# (CN-CONS-IN-01) + closed importer error sum (DC-CONS-IN-01).
#
# Mechanical guards:
#   1. Exactly one `pub fn import_live_consensus_inputs_raw` in
#      the workspace (the `_from_bytes` sibling lives in the same
#      file and is the in-memory variant of the same authority).
#   2. Exactly one `pub enum LiveConsensusInputsImportError`.
#   3. Exactly one `pub struct LiveConsensusInputsRaw`.
#   4. The importer file must NOT carry `#[non_exhaustive]` on
#      `LiveConsensusInputsImportError` (¬P-C4 closed-sum
#      discipline).
#   5. The error sum carries every documented variant
#      (Io, Json, BadField, MissingField, BadHashHex,
#      BadEpochWindow, BadPoolDistribution, EraNotSupported).

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TARGET="$REPO_ROOT/crates/ade_runtime/src/consensus_inputs/importer.rs"

FAILED=0
print_fail() { echo "FAIL: $1"; FAILED=1; }

strip_for_grep() {
    awk '
        /^#\[cfg\(test\)\]/ { in_test=1 }
        in_test { next }
        { line=$0; sub(/\/\/.*$/, "", line); print line }
    ' "$1"
}

if [[ ! -f "$TARGET" ]]; then
    print_fail "expected target file missing: $TARGET"
    exit "$FAILED"
fi

# Guard 1: sole pub fn import_live_consensus_inputs_raw.
sites=$(grep -rn --include='*.rs' -E '^pub fn import_live_consensus_inputs_raw\b' "$REPO_ROOT/crates" 2>/dev/null || true)
n=$(echo "$sites" | grep -c -v '^$' 2>/dev/null || echo 0)
if [[ "$n" -ne 1 ]]; then
    print_fail "expected exactly 1 pub fn import_live_consensus_inputs_raw, found $n:"
    echo "$sites"
fi

# Guard 1b: sibling _from_bytes also in the same file (single authority).
fb=$(grep -rn --include='*.rs' -E '^pub fn import_live_consensus_inputs_raw_from_bytes\b' "$REPO_ROOT/crates" 2>/dev/null || true)
nfb=$(echo "$fb" | grep -c -v '^$' 2>/dev/null || echo 0)
if [[ "$nfb" -ne 1 ]]; then
    print_fail "expected exactly 1 pub fn import_live_consensus_inputs_raw_from_bytes, found $nfb:"
    echo "$fb"
fi
if [[ "$nfb" -eq 1 ]] && ! echo "$fb" | grep -qF "$TARGET"; then
    print_fail "import_live_consensus_inputs_raw_from_bytes must live in $TARGET"
fi

# Guard 2: sole pub enum LiveConsensusInputsImportError.
enum_sites=$(grep -rn --include='*.rs' -E '^pub enum LiveConsensusInputsImportError\b' "$REPO_ROOT/crates" 2>/dev/null || true)
ne=$(echo "$enum_sites" | grep -c -v '^$' 2>/dev/null || echo 0)
if [[ "$ne" -ne 1 ]]; then
    print_fail "expected exactly 1 pub enum LiveConsensusInputsImportError, found $ne:"
    echo "$enum_sites"
fi

# Guard 3: sole pub struct LiveConsensusInputsRaw.
struct_sites=$(grep -rn --include='*.rs' -E '^pub struct LiveConsensusInputsRaw\b' "$REPO_ROOT/crates" 2>/dev/null || true)
ns=$(echo "$struct_sites" | grep -c -v '^$' 2>/dev/null || echo 0)
if [[ "$ns" -ne 1 ]]; then
    print_fail "expected exactly 1 pub struct LiveConsensusInputsRaw, found $ns:"
    echo "$struct_sites"
fi

# Guard 4: no #[non_exhaustive] above the closed error sum.
ne_lines=$(grep -nE '#\[non_exhaustive\]' "$TARGET" 2>/dev/null || true)
if [[ -n "$ne_lines" ]]; then
    while IFS=':' read -r lineno _rest; do
        next=$((lineno + 1))
        next_line=$(awk "NR==$next" "$TARGET")
        if echo "$next_line" | grep -qE 'pub enum LiveConsensusInputsImportError'; then
            print_fail "LiveConsensusInputsImportError carries #[non_exhaustive]: $TARGET:$lineno"
        fi
    done <<< "$ne_lines"
fi

# Guard 5: every closed-sum variant present.
EXPECTED_VARIANTS=(
    "Io"
    "Json"
    "BadField"
    "MissingField"
    "BadHashHex"
    "BadEpochWindow"
    "BadPoolDistribution"
    "EraNotSupported"
)
body=$(awk '
    /^pub enum LiveConsensusInputsImportError/ { capture=1; next }
    capture && /^}/ { exit }
    capture { print }
' "$TARGET")
for v in "${EXPECTED_VARIANTS[@]}"; do
    if ! echo "$body" | grep -qE "^\s*${v}\b"; then
        print_fail "LiveConsensusInputsImportError missing variant: $v"
    fi
done

if (( FAILED == 0 )); then
    echo "OK: single LiveConsensusInputs importer authority + closed error sum (CN-CONS-IN-01, DC-CONS-IN-01)"
fi
exit $FAILED
