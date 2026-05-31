#!/usr/bin/env bash
set -euo pipefail

# CN-NODE-03 (intent-classification half, PHASE4-N-F-F S1):
# the GREEN forge-intent classifier is a pure, closed, secret-free tri-state
# decision over CLI key-flag presence. A partial operator key set can never
# collapse into forge-on (or forge-off) — it fails closed.
#
# Positive (whole file — banners/signatures): the module exists with a
# `//! GREEN` banner; ForgeIntent is defined; classify_forge_intent is defined.
# Negative (production body only — doc/line comments + the #[cfg(test)] module
# stripped first, so commentary that names a forbidden token while explaining
# its exclusion does NOT trip the gate): closed vocabulary (no
# #[non_exhaustive]), no key-material / I/O / clock / nondeterminism token, and
# NO wildcard arm in the classification decision that could collapse an
# unenumerated presence combination into On or Off.
#
# Repo-root-relative. Mirrors the other ci_check_*.sh gates.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

FILE="crates/ade_node/src/forge_intent.rs"

if [[ ! -f "$FILE" ]]; then
    echo "FAIL: $FILE not found"
    exit 1
fi

# --- positives (whole file; the banner is itself a comment) -----------------
if ! grep -qE '^//! GREEN' "$FILE"; then
    echo "FAIL: $FILE missing '//! GREEN' banner"
    exit 1
fi
if ! grep -qE '^pub enum ForgeIntent' "$FILE"; then
    echo "FAIL: ForgeIntent enum not defined in $FILE"
    exit 1
fi
if ! grep -qE '^pub fn classify_forge_intent' "$FILE"; then
    echo "FAIL: classify_forge_intent not defined in $FILE"
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

# Closed vocabulary: no #[non_exhaustive] on any forge-intent type.
if grep -qE '#\[non_exhaustive\]' <<<"$PROD"; then
    echo "FAIL: forge_intent production body uses #[non_exhaustive] (closed vocabulary)"
    exit 1
fi

# No key material, no I/O, no clock, no nondeterminism — this slice decides
# intent from path PRESENCE, never from key bytes or file contents.
FORBIDDEN=(
    'KesSecret'
    'VrfSigningKey'
    'ColdSigningKey'
    'ProducerShell'
    'ForgeActivation'
    'std::fs'
    '\bFile\b'
    '\bread\('
    'read_to_string'
    'tokio'
    'SystemTime'
    'Instant'
    'HashMap'
    '\bawait\b'
)

for pat in "${FORBIDDEN[@]}"; do
    if grep -qE "$pat" <<<"$PROD"; then
        echo "FAIL: forge_intent production body matches forbidden token: $pat"
        exit 1
    fi
done

# Isolate the classify_forge_intent decision surface (production body, from its
# signature to the end of the production region — it is the last item before
# the test module, which PROD already dropped).
CLASSIFY="$(awk '/^pub fn classify_forge_intent/{f=1} f{print}' <<<"$PROD")"
if [[ -z "$CLASSIFY" ]]; then
    echo "FAIL: could not isolate classify_forge_intent body of $FILE"
    exit 1
fi

# No wildcard arm in the classification decision. A `_ =>` arm could collapse an
# unenumerated presence combination into On or Off; the decision must enumerate
# the two total cases by explicit pattern and bind the partial case by name.
# (A `_` used in closures elsewhere — `|(_, p)|` — is `_,`/`_)`, never `_ =>`.)
if grep -qE '_[[:space:]]*=>' <<<"$CLASSIFY"; then
    echo "FAIL: classify_forge_intent uses a wildcard '_ =>' arm (decision must enumerate cases by name)"
    exit 1
fi

# Positive: the two total outcomes are matched by explicit patterns (not a
# wildcard). All-present => On; all-absent => Off.
if ! grep -qE '\(Some\(.*Some\(.*Some\(.*Some\(.*Some\(.*\)[[:space:]]*=>' <<<"$CLASSIFY"; then
    echo "FAIL: classify_forge_intent missing the explicit all-present (5x Some) arm"
    exit 1
fi
if ! grep -qE '\(None,[[:space:]]*None,[[:space:]]*None,[[:space:]]*None,[[:space:]]*None\)[[:space:]]*=>' <<<"$CLASSIFY"; then
    echo "FAIL: classify_forge_intent missing the explicit all-absent (5x None) arm"
    exit 1
fi

echo "OK: forge_intent is a closed, pure, secret-free GREEN tri-state classifier (CN-NODE-03 intent half)"
exit 0
