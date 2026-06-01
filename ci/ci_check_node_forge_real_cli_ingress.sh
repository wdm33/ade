#!/usr/bin/env bash
set -euo pipefail

# CE-G-A-2 (PHASE4-N-F-G-A S2): the --mode node operator-forge ingress site loads
# operator config through the REAL cardano-cli closed-contract parsers
# (parse_opcert_envelope + parse_shelley_genesis) and retires the parse_simple_*
# stubs ON THE NODE PATH. Fails closed if a future change reintroduces a
# simple-JSON parser on the node forge path.
#
# Repo-root-relative. Mirrors the other ci_check_*.sh gates.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

FILE="crates/ade_node/src/operator_forge.rs"

if [[ ! -f "$FILE" ]]; then
    echo "FAIL: $FILE not found"
    exit 1
fi

# Production body: drop the #[cfg(test)] module, then strip line/doc comments so
# commentary naming a retired symbol does not trip the negative greps.
PROD="$(awk '/#\[cfg\(test\)\]/{exit} {print}' "$FILE" | sed -E 's://.*::')"
if [[ -z "$PROD" ]]; then
    echo "FAIL: could not isolate production body of $FILE"
    exit 1
fi

# --- positives: the real parsers are used on the node path -------------------
for sym in parse_opcert_envelope parse_shelley_genesis; do
    if ! grep -qE "$sym" <<<"$PROD"; then
        echo "FAIL: $FILE production body does not use the real parser $sym"
        exit 1
    fi
done

# --- negatives: no simple-JSON parser on the node forge path -----------------
for sym in parse_simple_opcert_json parse_simple_genesis_json; do
    if grep -qE "$sym" <<<"$PROD"; then
        echo "FAIL: $FILE still references $sym on the node path (must retire it)"
        exit 1
    fi
done

echo "OK: node-path operator-forge ingress uses the real cardano-cli parsers; no parse_simple_* (CE-G-A-2)"
exit 0
