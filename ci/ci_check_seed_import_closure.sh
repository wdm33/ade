#!/usr/bin/env bash
set -uo pipefail

# PHASE4-N-M-A S1 — single seed-import authority (CN-SEED-01).
#
# Exactly one pub fn named `import_cardano_cli_json_utxo` in the
# workspace (the `_from_bytes` sibling is the in-memory variant
# called by the file variant and counts as the same authority —
# the gate distinguishes by exact name).

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"

FAILED=0
print_fail() { echo "FAIL: $1"; FAILED=1; }

strip_for_grep() {
    awk '
        /^#\[cfg\(test\)\]/ { in_test=1 }
        in_test { next }
        { line=$0; sub(/\/\/.*$/, "", line); print line }
    ' "$1"
}

EXTRA=()
TARGET="$REPO_ROOT/crates/ade_runtime/src/seed_import/importer.rs"
for f in $(find "$REPO_ROOT/crates" -type f -name '*.rs'); do
    if [[ "$f" == "$TARGET" ]]; then continue; fi
    body=$(strip_for_grep "$f")
    if echo "$body" | grep -qE '\bpub fn import_cardano_cli_json_utxo\b'; then
        EXTRA+=("$f")
    fi
done

if (( ${#EXTRA[@]} > 0 )); then
    print_fail "second pub fn import_cardano_cli_json_utxo in:"
    for f in "${EXTRA[@]}"; do echo "  $f"; done
fi

if [[ ! -f "$TARGET" ]]; then
    print_fail "expected target file missing: $TARGET"
elif ! grep -qE '\bpub fn import_cardano_cli_json_utxo\b' "$TARGET"; then
    print_fail "import_cardano_cli_json_utxo missing from $TARGET"
fi

if (( FAILED == 0 )); then
    echo "OK: single seed-import authority (CN-SEED-01) holds"
fi
exit $FAILED
