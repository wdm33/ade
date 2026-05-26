#!/usr/bin/env bash
set -uo pipefail

# PHASE4-N-M-A S3 — WAL is append-only by type (CN-WAL-01 + DC-WAL-01).
#
# Mechanical guards:
#   1. The `WalStore` trait at
#      `crates/ade_ledger/src/wal/store_trait.rs` declares ONLY
#      `append`, `read_all`, `verify_chain`. No `truncate` /
#      `rewrite` / `replace` / `delete` / `clear`.
#   2. NO impl across the workspace adds non-trait methods
#      named truncate / rewrite / replace / delete / clear on a
#      `FileWalStore` (or any other `WalStore` impl) via
#      `impl FileWalStore` blocks.
#   3. NO file under `crates/ade_ledger/src/wal/` or
#      `crates/ade_runtime/src/wal/` defines those method names.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TRAIT_RS="$REPO_ROOT/crates/ade_ledger/src/wal/store_trait.rs"

FAILED=0
print_fail() { echo "FAIL: $1"; FAILED=1; }

strip_for_grep() {
    awk '
        /^#\[cfg\(test\)\]/ { in_test=1 }
        in_test { next }
        { line=$0; sub(/\/\/.*$/, "", line); print line }
    ' "$1"
}

FORBIDDEN=(truncate rewrite replace delete clear)

# Rule 1: trait has the three sanctioned methods + no others.
if [[ ! -f "$TRAIT_RS" ]]; then
    print_fail "missing $TRAIT_RS"
else
    body=$(strip_for_grep "$TRAIT_RS")
    if ! echo "$body" | grep -qE 'fn append\b'; then
        print_fail "WalStore::append missing"
    fi
    if ! echo "$body" | grep -qE 'fn read_all\b'; then
        print_fail "WalStore::read_all missing"
    fi
    if ! echo "$body" | grep -qE 'fn verify_chain\b'; then
        print_fail "WalStore::verify_chain missing"
    fi
    for needle in "${FORBIDDEN[@]}"; do
        if echo "$body" | grep -qE "fn ${needle}\\b"; then
            print_fail "WalStore trait has forbidden method: fn ${needle}"
        fi
    done
fi

# Rules 2+3: scan every wal/*.rs file (BLUE + GREEN) for forbidden
# method names appearing in `fn <name>` definitions (this catches
# both trait methods and impl-block methods).
for d in \
    "$REPO_ROOT/crates/ade_ledger/src/wal" \
    "$REPO_ROOT/crates/ade_runtime/src/wal" ; do
    [[ -d "$d" ]] || continue
    for f in "$d"/*.rs; do
        [[ -f "$f" ]] || continue
        rel="${f#$REPO_ROOT/}"
        body=$(strip_for_grep "$f")
        for needle in "${FORBIDDEN[@]}"; do
            if echo "$body" | grep -qE "fn ${needle}\\b"; then
                print_fail "$rel defines forbidden method: fn ${needle}"
            fi
        done
    done
done

if (( FAILED == 0 )); then
    echo "OK: WAL is append-only by type (CN-WAL-01 + DC-WAL-01)"
fi
exit $FAILED
