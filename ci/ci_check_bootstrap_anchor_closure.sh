#!/usr/bin/env bash
set -uo pipefail

# PHASE4-N-M-A S2 — single BootstrapAnchor mint authority
# (CN-ANCHOR-01) + single encoder/decoder pair (DC-ANCHOR-01).
#
# Mechanical guards:
#   1. Exactly one `pub fn mint` in the workspace whose return
#      type is `BootstrapAnchor`.
#   2. Exactly one `pub fn encode_bootstrap_anchor` and one
#      `pub fn decode_bootstrap_anchor` in the workspace.
#   3. `BootstrapAnchor` and `SeedPoint` structs have no
#      `Default` impl and no `#[non_exhaustive]` attribute
#      (¬P-A3).

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
ANCHOR_RS="$REPO_ROOT/crates/ade_ledger/src/bootstrap_anchor/anchor.rs"
MINT_RS="$REPO_ROOT/crates/ade_runtime/src/bootstrap_anchor.rs"

FAILED=0
print_fail() { echo "FAIL: $1"; FAILED=1; }

strip_for_grep() {
    awk '
        /^#\[cfg\(test\)\]/ { in_test=1 }
        in_test { next }
        { line=$0; sub(/\/\/.*$/, "", line); print line }
    ' "$1"
}

# Rule 1: single pub fn mint returning BootstrapAnchor.
EXTRA_MINT=()
for f in $(find "$REPO_ROOT/crates" -type f -name '*.rs'); do
    if [[ "$f" == "$MINT_RS" ]]; then continue; fi
    body=$(strip_for_grep "$f")
    if echo "$body" | grep -qE 'pub fn mint\b.*-> *BootstrapAnchor'; then
        EXTRA_MINT+=("$f")
    fi
done
if (( ${#EXTRA_MINT[@]} > 0 )); then
    print_fail "second pub fn mint -> BootstrapAnchor in:"
    for f in "${EXTRA_MINT[@]}"; do echo "  $f"; done
fi
if [[ -f "$MINT_RS" ]]; then
    body=$(strip_for_grep "$MINT_RS")
    if ! echo "$body" | grep -qE 'pub fn mint\b'; then
        print_fail "ade_runtime::bootstrap_anchor::mint missing from $MINT_RS"
    fi
else
    print_fail "missing $MINT_RS"
fi

# Rule 2: single encode_bootstrap_anchor / decode_bootstrap_anchor.
EXTRA_ENC=()
EXTRA_DEC=()
for f in $(find "$REPO_ROOT/crates" -type f -name '*.rs'); do
    if [[ "$f" == "$ANCHOR_RS" ]]; then continue; fi
    body=$(strip_for_grep "$f")
    if echo "$body" | grep -qE 'pub fn encode_bootstrap_anchor\b'; then
        EXTRA_ENC+=("$f")
    fi
    if echo "$body" | grep -qE 'pub fn decode_bootstrap_anchor\b'; then
        EXTRA_DEC+=("$f")
    fi
done
if (( ${#EXTRA_ENC[@]} > 0 )); then
    print_fail "second pub fn encode_bootstrap_anchor in:"
    for f in "${EXTRA_ENC[@]}"; do echo "  $f"; done
fi
if (( ${#EXTRA_DEC[@]} > 0 )); then
    print_fail "second pub fn decode_bootstrap_anchor in:"
    for f in "${EXTRA_DEC[@]}"; do echo "  $f"; done
fi

# Rule 3: no Default / non_exhaustive on BootstrapAnchor / SeedPoint.
if [[ -f "$ANCHOR_RS" ]]; then
    body=$(strip_for_grep "$ANCHOR_RS")
    if echo "$body" | grep -qE 'impl +Default +for +BootstrapAnchor'; then
        print_fail "BootstrapAnchor must not have a Default impl (¬P-A3)"
    fi
    if echo "$body" | grep -qE 'impl +Default +for +SeedPoint'; then
        print_fail "SeedPoint must not have a Default impl (¬P-A3)"
    fi
    if echo "$body" | grep -qE '#\[non_exhaustive\]'; then
        print_fail "anchor.rs must not contain #[non_exhaustive] (¬P-A3)"
    fi
else
    print_fail "missing $ANCHOR_RS"
fi

if (( FAILED == 0 )); then
    echo "OK: BootstrapAnchor closure invariants hold (CN-ANCHOR-01 + DC-ANCHOR-01)"
fi
exit $FAILED
