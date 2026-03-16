#!/usr/bin/env bash
set -euo pipefail

# Verify byte-identical CBOR round-trip on all golden corpus blocks.
# Status: PARTIAL — passes vacuously until era-specific decoders exist.
# After T-13 (Conway), this becomes a full-corpus enforcement gate.
#
# Invariants: T-ENC-03, DC-CBOR-01, DC-CBOR-02

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CORPUS_DIR="$REPO_ROOT/corpus/golden"

# Count corpus blocks to verify we're scanning the right set
BLOCK_COUNT=$(find "$CORPUS_DIR" -name '*.cbor' | wc -l)
echo "Found $BLOCK_COUNT golden CBOR blocks"

if [ "$BLOCK_COUNT" -lt 42 ]; then
    echo "FAIL: Expected at least 42 corpus blocks, found $BLOCK_COUNT"
    exit 1
fi

# Run the integration test that exercises round-trip on implemented eras.
# Until era decoders exist, this tests envelope dispatch only.
echo "Running envelope dispatch test on all $BLOCK_COUNT corpus blocks..."
cargo test -p ade_codec --test envelope -- --quiet 2>&1

echo "PASS: CBOR round-trip check (partial — envelope dispatch verified on $BLOCK_COUNT blocks)"
exit 0
