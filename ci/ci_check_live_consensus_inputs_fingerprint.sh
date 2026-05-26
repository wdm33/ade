#!/usr/bin/env bash
set -uo pipefail

# PHASE4-N-M-C S1b — canonical fingerprint of consensus inputs
# (DC-CONS-IN-02).
#
# Mechanical guards:
#   1. Exactly one `pub struct LiveConsensusInputsCanonical`
#      across the workspace.
#   2. Exactly one `pub fn import_live_consensus_inputs` (the
#      file variant) — the SOLE Canonical-returning authority
#      (CN-CONS-IN-01 full form).
#   3. Exactly one `pub fn import_live_consensus_inputs_from_bytes`
#      (in-memory sibling) and one `pub fn canonical_from_raw`.
#   4. Exactly one `pub fn encode_canonical_cbor` private to the
#      canonical module — i.e. NOT exposed publicly: the
#      canonical encoding rule is a single internal function,
#      consumed only by `canonical_from_raw`. The fingerprint
#      MUST be computed via `blake2b_256` of its output (a grep
#      lock on the lift function body proves the binding).
#   5. The Canonical struct carries a `fingerprint: Hash32` field.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TARGET="$REPO_ROOT/crates/ade_runtime/src/consensus_inputs/canonical.rs"

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

# Guard 1.
s_sites=$(grep -rn --include='*.rs' -E '^pub struct LiveConsensusInputsCanonical\b' "$REPO_ROOT/crates" 2>/dev/null || true)
ns=$(echo "$s_sites" | grep -c -v '^$' 2>/dev/null || echo 0)
if [[ "$ns" -ne 1 ]]; then
    print_fail "expected exactly 1 pub struct LiveConsensusInputsCanonical, found $ns:"
    echo "$s_sites"
fi

# Guard 2.
fn_file_sites=$(grep -rn --include='*.rs' -E '^pub fn import_live_consensus_inputs\(' "$REPO_ROOT/crates" 2>/dev/null || true)
nf=$(echo "$fn_file_sites" | grep -c -v '^$' 2>/dev/null || echo 0)
if [[ "$nf" -ne 1 ]]; then
    print_fail "expected exactly 1 pub fn import_live_consensus_inputs(path:...), found $nf:"
    echo "$fn_file_sites"
fi

# Guard 3.
for name in import_live_consensus_inputs_from_bytes canonical_from_raw; do
    sites=$(grep -rn --include='*.rs' -E "^pub fn ${name}\b" "$REPO_ROOT/crates" 2>/dev/null || true)
    nn=$(echo "$sites" | grep -c -v '^$' 2>/dev/null || echo 0)
    if [[ "$nn" -ne 1 ]]; then
        print_fail "expected exactly 1 pub fn ${name}, found $nn:"
        echo "$sites"
    fi
done

# Guard 4: encode_canonical_cbor is a single PRIVATE function in
# the canonical module (no `pub fn`) AND canonical_from_raw
# computes the fingerprint as blake2b_256 of its output.
priv_def=$(grep -nE '^fn encode_canonical_cbor\b' "$TARGET" 2>/dev/null || true)
pub_def=$(grep -rn --include='*.rs' -E '^pub fn encode_canonical_cbor\b' "$REPO_ROOT/crates" 2>/dev/null || true)
if [[ -z "$priv_def" ]]; then
    print_fail "missing private encode_canonical_cbor in $TARGET"
fi
if [[ -n "$pub_def" ]]; then
    print_fail "encode_canonical_cbor must be private (no pub fn):"
    echo "$pub_def"
fi
# canonical_from_raw must compute fingerprint as blake2b_256(encode_canonical_cbor(...))
if ! grep -E 'blake2b_256\(&encoded\)' "$TARGET" >/dev/null 2>&1; then
    print_fail "canonical_from_raw must compute fingerprint via blake2b_256(&encode_canonical_cbor(...))"
fi
if ! grep -E 'let encoded = encode_canonical_cbor\(' "$TARGET" >/dev/null 2>&1; then
    print_fail "canonical_from_raw must invoke encode_canonical_cbor to build the hash preimage"
fi

# Guard 5: Canonical struct carries fingerprint: Hash32.
body=$(awk '
    /^pub struct LiveConsensusInputsCanonical/ { capture=1; next }
    capture && /^}/ { exit }
    capture { print }
' "$TARGET")
if ! echo "$body" | grep -qE 'pub fingerprint:\s*Hash32'; then
    print_fail "LiveConsensusInputsCanonical missing field: pub fingerprint: Hash32"
fi

if (( FAILED == 0 )); then
    echo "OK: LiveConsensusInputsCanonical + canonical-CBOR fingerprint authority (DC-CONS-IN-02)"
fi
exit $FAILED
