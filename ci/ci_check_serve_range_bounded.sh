#!/usr/bin/env bash
set -uo pipefail

# PHASE4-N-AA S2 — bounded peer-driven serve range (DC-SERVEMEM-01).
#
# The --mode node serve projection must bound per-request work: it reads via the
# S1 hash-free bounded primitives (range_bytes_capped / last_block_bytes), never
# the unbounded iter_from_slot (full-range Vec + per-block SLOT_BY_HASH scan) or
# the O(N) chaindb.tip(); it caps each request at a FIXED, non-configurable
# MAX_SERVE_RANGE_BLOCKS literal (no CLI/env/config escape, no unbounded mode);
# and it derives each block's hash from the bytes via the single BLUE decode
# authority (no second hash authority, no SLOT_BY_HASH reference on the serve
# path). Oversized ranges fail closed (CapExceeded -> empty -> reducer NoBlocks),
# pinned by the ServeRangeOutcome unit tests.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SERVE_RS="$REPO_ROOT/crates/ade_runtime/src/network/served_chain_projection.rs"

FAILED=0
print_fail() { echo "FAIL: $1"; FAILED=1; }

if [[ ! -f "$SERVE_RS" ]]; then
    echo "FAIL: missing $SERVE_RS"
    exit 1
fi

# Production view: strip line comments (// /// //!) and stop at the first
# #[cfg(test)] module, so the test module's synthetic helpers do not satisfy or
# trip the structural checks below.
PROD="$(awk '
    /^[[:space:]]*#\[cfg\(test\)\]/ { exit }
    { line = $0; sub(/\/\/.*$/, "", line); print line }
' "$SERVE_RS")"

# Guard 1 — the serve path reads via the S1 bounded, hash-free primitives.
if ! echo "$PROD" | grep -qE 'range_bytes_capped[[:space:]]*\('; then
    print_fail "serve projection does not use range_bytes_capped (the bounded range primitive)"
fi
if ! echo "$PROD" | grep -qE 'last_block_bytes[[:space:]]*\('; then
    print_fail "serve projection does not use last_block_bytes (the bounded tip primitive)"
fi

# Guard 2 — the serve path NEVER uses the unbounded iter_from_slot.
if echo "$PROD" | grep -qE '\.iter_from_slot[[:space:]]*\('; then
    print_fail "serve projection calls iter_from_slot (unbounded full-range Vec + O(N) hash scan) — must use the bounded primitives:"
    echo "$PROD" | grep -nE '\.iter_from_slot[[:space:]]*\('
fi

# Guard 3 — the serve path NEVER uses the O(N) chaindb.tip() (use last_block_bytes).
if echo "$PROD" | grep -qE 'chaindb\.tip[[:space:]]*\('; then
    print_fail "serve projection calls chaindb.tip() (O(N) iteration + hash scan) — must use last_block_bytes:"
    echo "$PROD" | grep -nE 'chaindb\.tip[[:space:]]*\('
fi

# Guard 4 — a FIXED MAX_SERVE_RANGE_BLOCKS literal exists (compile-time const).
if ! echo "$PROD" | grep -qE 'const MAX_SERVE_RANGE_BLOCKS:[[:space:]]*usize[[:space:]]*=[[:space:]]*[0-9_]+[[:space:]]*;'; then
    print_fail "no fixed 'const MAX_SERVE_RANGE_BLOCKS: usize = <literal>;' — the cap must be a compile-time constant"
fi
# ...and it is actually used to bound a read.
if ! echo "$PROD" | grep -qE 'range_bytes_capped\([^)]*MAX_SERVE_RANGE_BLOCKS'; then
    print_fail "MAX_SERVE_RANGE_BLOCKS is not passed to range_bytes_capped — the cap is declared but not applied"
fi

# Guard 5 — the cap is NOT runtime-configurable: no CLI/env/config read anywhere
# in the serve projection (no escape hatch, no unbounded mode).
if echo "$PROD" | grep -qE 'std::env|env::var|env!\(|option_env!\('; then
    print_fail "serve projection reads the environment — the cap must be a fixed compile-time bound, never runtime-configurable:"
    echo "$PROD" | grep -nE 'std::env|env::var|env!\(|option_env!\('
fi

# Guard 6 — the hash is derived via the single BLUE decode authority; NO second
# hash authority (no manual blake2b) and NO SLOT_BY_HASH reference on the serve.
if ! echo "$PROD" | grep -qE 'decode_block[[:space:]]*\('; then
    print_fail "serve projection does not derive the hash via decode_block (the single BLUE decode authority)"
fi
if echo "$PROD" | grep -qE 'blake2b|Blake2b|SLOT_BY_HASH'; then
    print_fail "serve projection references a second hash authority (blake2b) or SLOT_BY_HASH — the hash must come from decode_block only:"
    echo "$PROD" | grep -nE 'blake2b|Blake2b|SLOT_BY_HASH'
fi

if (( FAILED == 0 )); then
    echo "OK: serve range bounded — S1 bounded primitives only (no iter_from_slot / O(N) tip), fixed non-configurable MAX_SERVE_RANGE_BLOCKS cap, hash via decode_block (DC-SERVEMEM-01)"
fi
exit $FAILED
