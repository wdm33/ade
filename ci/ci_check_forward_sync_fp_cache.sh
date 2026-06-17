#!/usr/bin/env bash
set -uo pipefail

# LIVE-FOLLOW-THROUGHPUT — the GREEN forward-sync reducer must derive the
# per-block WAL `post_fp` via the cached UTxO-component path
# (`fingerprint_v2_with_utxo` + `UtxoFpCache`), NOT the full `fingerprint()`,
# which re-runs the O(n) Ristretto255 UTxO set-commitment over the (preview:
# ~1.9M-entry) UTxO on EVERY admit (~20s/block, 99.8% CPU — the catch-up
# bottleneck). Byte-identity to the full fingerprint is proven by the pump test
# `pump_block_post_fp_is_byte_identical_to_full_fingerprint`; this gate is the
# mechanical guard against a regression back to the full per-block scan.
#
# Under the live `track_utxo=false` follow the imported UTxO never mutates, so
# the OverlayUtxo generation is stable across the per-block clones and the cache
# returns the constant component in O(1). Any mutation bumps the generation and
# forces a full recompute, so the cached value is ALWAYS byte-identical.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
REDUCER="$REPO_ROOT/crates/ade_runtime/src/forward_sync/reducer.rs"

FAILED=0
print_fail() {
    echo "FAIL: $1"
    FAILED=1
}

# Strip line comments + the #[cfg(test)] block so doc-comment prose and test
# code (which legitimately call the full fingerprint() as the oracle) don't trip
# the greps.
strip_body() {
    awk '
        /^#\[cfg\(test\)\]/ { in_test=1 }
        in_test { next }
        { line=$0; sub(/\/\/.*$/, "", line); print line }
    ' "$1"
}

if [[ ! -f "$REDUCER" ]]; then
    print_fail "GREEN reducer missing: $REDUCER"
    exit "$FAILED"
fi

body=$(strip_body "$REDUCER")

# 1. The cached path MUST be present.
if ! grep -qE '\bUtxoFpCache\b' <<< "$body"; then
    print_fail "reducer.rs: per-loop UtxoFpCache missing (LIVE-FOLLOW-THROUGHPUT)"
fi
if ! grep -qE 'utxo_fp_cache\.utxo_fingerprint\(' <<< "$body"; then
    print_fail "reducer.rs: per-block UTxO component must come from utxo_fp_cache.utxo_fingerprint(...)"
fi
if ! grep -qE '\bfingerprint_v2_with_utxo\(' <<< "$body"; then
    print_fail "reducer.rs: post_fp must be derived via fingerprint_v2_with_utxo(...)"
fi

# 2. The full per-block recompute MUST NOT return. A bare `fingerprint(` call is
#    the full O(n) scan; `fingerprint_v2_with_utxo(` and `utxo_fingerprint(` do
#    NOT match `\bfingerprint\(` (the preceding char is `_` / `:` is a boundary
#    only before a bare `fingerprint`), so a qualified `::fingerprint(` IS caught.
if grep -qE '\bfingerprint\(' <<< "$body"; then
    print_fail "reducer.rs: bare fingerprint() forbidden in the per-block admit — use the cached fingerprint_v2_with_utxo path (LIVE-FOLLOW-THROUGHPUT)"
fi

if (( FAILED == 0 )); then
    echo "OK: forward-sync per-block post_fp uses the cached UTxO-component path (no O(n) per-block UTxO recompute)"
fi
exit "$FAILED"
