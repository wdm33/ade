#!/usr/bin/env bash
set -euo pipefail

# PHASE4-N-P S4 — Sum6KES compatibility gate.
#
# Mechanical guards for DC-CRYPTO-08 + DC-CRYPTO-09:
#
#   1. cardano-cli expanded-payload corpus exists and every fixture
#      is preceded by the throwaway-fixture comment.
#   2. No `.skey` envelope files committed under crates/ade_crypto/.
#   3. cardano_crypto is imported in crates/ade_crypto/src/**
#      ONLY inside #[cfg(test)] blocks. (Post-S3 the production code
#      path no longer uses upstream KES; this guard locks the door
#      against accidental reintroduction.)
#   4. The expand_seed prefix bytes match Haskell cardano-base
#      (0x01 / 0x02) and NOT cardano-crypto Rust 1.0.8 (0x00 / 0x01).
#      The cardano-cli ground-truth corpus would fail if these
#      drifted, but a literal-byte check here surfaces drift faster.
#
# Failure exits 1 with a `print_fail` message; passes exit 0.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
ADE_CRYPTO_SRC="$REPO_ROOT/crates/ade_crypto/src"
KES_SUM_DIR="$ADE_CRYPTO_SRC/kes_sum"
CORPUS_RS="$KES_SUM_DIR/cardano_cli_corpus.rs"
HASH_RS="$KES_SUM_DIR/hash.rs"

FAILED=0
print_fail() {
    echo "FAIL: $1"
    FAILED=1
}

# ---------------------------------------------------------------------------
# Guard 1 — corpus exists + every fixture has the throwaway comment.
# ---------------------------------------------------------------------------
if [ ! -f "$CORPUS_RS" ]; then
    print_fail "Guard 1: missing $CORPUS_RS"
else
    # Count: each SKEY{N} const must be preceded by the throwaway comment.
    skey_count=$(grep -cE '^pub\(super\) const SKEY[0-9]+: &\[u8; 608\]' "$CORPUS_RS" || true)
    throwaway_count=$(grep -cE 'TEST ONLY: throwaway deterministic fixture generated for Sum6KES' "$CORPUS_RS" || true)
    if [ "$skey_count" -lt 3 ]; then
        print_fail "Guard 1: expected at least 3 SKEY fixtures, found $skey_count"
    fi
    if [ "$throwaway_count" -lt "$skey_count" ]; then
        print_fail "Guard 1: $throwaway_count throwaway comments < $skey_count fixtures"
    fi
fi

# ---------------------------------------------------------------------------
# Guard 2 — no `.skey` envelope files anywhere under crates/ade_crypto/.
# ---------------------------------------------------------------------------
skey_files=$(find "$REPO_ROOT/crates/ade_crypto" -name '*.skey' 2>/dev/null || true)
if [ -n "$skey_files" ]; then
    print_fail "Guard 2: forbidden .skey files under crates/ade_crypto/:"
    echo "$skey_files"
fi

# ---------------------------------------------------------------------------
# Guard 3 — cardano_crypto::kes (and CompactSum) only inside #[cfg(test)]
# within ade_crypto/src.
#
# N9 (no upstream-shim in production) is scoped to KES only: VRF and
# DSIGN continue to use cardano-crypto Rust upstream (those are
# separate, future-cluster concerns). After PHASE4-N-P S3 there are
# no `cardano_crypto::kes::*` imports in production code under
# ade_crypto/src/**; this gate locks the door against accidental
# reintroduction.
#
# Heuristic: for each .rs file under ade_crypto/src that imports
# `cardano_crypto::kes`, the import must appear inside a test-only
# file (tests.rs / *_tests.rs / *_corpus.rs) OR after a top-level
# `#[cfg(test)]` marker.
# ---------------------------------------------------------------------------
emit_production_lines() {
    local f="$1"
    awk '/^#\[cfg\(test\)\]/ { exit } { print NR ":" $0 }' "$f"
}

while IFS= read -r -d '' rs; do
    # Skip test-only files by convention:
    # - tests.rs / *_tests.rs — Rust convention for #[cfg(test)] mod tests.
    # - *_corpus.rs — test-corpus modules (also #[cfg(test)] only).
    case "$rs" in
        */tests.rs|*_tests.rs|*_corpus.rs) continue ;;
    esac
    # Production lines:
    # N9 is KES-scoped; permit `cardano_crypto::vrf` and
    # `cardano_crypto::dsign` in production code (separate clusters).
    matches=$(emit_production_lines "$rs" | grep -E 'cardano_crypto::kes|use\s+cardano_crypto::kes' || true)
    if [ -n "$matches" ]; then
        bad=$(echo "$matches" | grep -vE '^[0-9]+://' || true)
        if [ -n "$bad" ]; then
            print_fail "Guard 3 (cardano_crypto::kes referenced outside #[cfg(test)] in $rs):"
            echo "$bad"
        fi
    fi
done < <(find "$ADE_CRYPTO_SRC" -name '*.rs' -print0)

# ---------------------------------------------------------------------------
# Guard 4 — expand_seed prefix bytes match Haskell cardano-base.
# ---------------------------------------------------------------------------
if [ ! -f "$HASH_RS" ]; then
    print_fail "Guard 4: missing $HASH_RS"
else
    # Look for the literal "left_input[0] = 0x01" and "right_input[0] = 0x02".
    if ! grep -q 'left_input\[0\] = 0x01' "$HASH_RS"; then
        print_fail "Guard 4: hash.rs does not set left_input[0] = 0x01 (Haskell convention)"
    fi
    if ! grep -q 'right_input\[0\] = 0x02' "$HASH_RS"; then
        print_fail "Guard 4: hash.rs does not set right_input[0] = 0x02 (Haskell convention)"
    fi
    # Defense-in-depth: explicitly forbid the cardano-crypto Rust
    # 1.0.8 prefixes (0x00 / 0x01) in left/right_input positions.
    if grep -qE 'left_input\[0\] = 0x00' "$HASH_RS"; then
        print_fail "Guard 4: hash.rs uses cardano-crypto Rust 1.0.8 prefix (0x00) — diverges from Haskell"
    fi
fi

if [ "$FAILED" -eq 0 ]; then
    echo "PASS: kes_sum compatibility gates green (4/4)"
    exit 0
else
    exit 1
fi
