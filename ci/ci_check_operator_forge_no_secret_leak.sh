#!/usr/bin/env bash
set -euo pipefail

# CN-NODE-03 (RED-custody-loading half, PHASE4-N-F-F S2):
# the node-path operator-material ingress site loads keys into ProducerShell
# (RED custody) and leaks NO private key bytes — no byte accessor, no
# serialization, no logging.
#
# Positive (whole file — banner/signature): `//! RED` banner;
# load_operator_producer_shell defined; ProducerShell::init called.
# Negative (production body only — doc/line comments + the #[cfg(test)] module
# stripped first): no key-byte leak vector.
#
# Repo-root-relative. Mirrors the other ci_check_*.sh gates.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

FILE="crates/ade_node/src/operator_forge.rs"

if [[ ! -f "$FILE" ]]; then
    echo "FAIL: $FILE not found"
    exit 1
fi

# --- positives (whole file; the banner is itself a comment) -----------------
if ! grep -qE '^//! RED' "$FILE"; then
    echo "FAIL: $FILE missing '//! RED' banner"
    exit 1
fi
if ! grep -qE '^pub fn load_operator_producer_shell' "$FILE"; then
    echo "FAIL: load_operator_producer_shell not defined in $FILE"
    exit 1
fi
if ! grep -qE 'ProducerShell::init\(' "$FILE"; then
    echo "FAIL: $FILE does not reuse ProducerShell::init"
    exit 1
fi

# Production body: drop everything from the #[cfg(test)] module onward, then
# strip line/doc comments so commentary that names a forbidden token (while
# explaining its exclusion) does not trip the negative greps.
PROD="$(awk '/#\[cfg\(test\)\]/{exit} {print}' "$FILE" | sed -E 's://.*::')"

if [[ -z "$PROD" ]]; then
    echo "FAIL: could not isolate production body of $FILE"
    exit 1
fi

# No private-key leak vectors: no logging of any value (the caller prints; this
# module returns Results), no byte accessor, no serialization of key material,
# no closed-vocabulary type, no GREEN coordinator state.
FORBIDDEN=(
    'println!'
    'eprintln!'
    'dbg!'
    'to_bytes'
    'as_bytes'
    'Serialize'
    'Deserialize'
    'unsafe'
    'CoordinatorState'
    '#\[non_exhaustive\]'
)

for pat in "${FORBIDDEN[@]}"; do
    if grep -qE "$pat" <<<"$PROD"; then
        echo "FAIL: operator_forge production body matches forbidden token: $pat"
        exit 1
    fi
done

# No pub fn may return raw key bytes (a private-byte escape hatch).
if grep -qE '^pub fn .*->.*(\[u8|Vec<u8>|&\[u8\])' <<<"$PROD"; then
    echo "FAIL: operator_forge exposes a raw-byte-returning pub fn (key-byte escape)"
    exit 1
fi

echo "OK: operator_forge is a RED-custody ingress site with no private-key leak vector (CN-NODE-03 custody half)"
exit 0
