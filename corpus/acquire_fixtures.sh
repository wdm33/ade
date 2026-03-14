#!/usr/bin/env bash
set -euo pipefail

# Acquire golden mainnet CBOR block fixtures for the Ade test corpus.
#
# Current corpus was extracted from a real cardano-node 10.6.2 ImmutableDB
# via Mithril snapshot (epoch 618, immutable file 8419). Blocks were read
# from chunk files using the secondary index (56-byte entries) and stored
# as raw CBOR without any decoding or re-encoding.
#
# To add new fixtures:
#   1. Ensure a running cardano-node with synced ImmutableDB, OR
#      download a Mithril snapshot
#   2. Extract blocks from the desired ImmutableDB chunk files
#   3. Place .cbor files in the appropriate era directory under golden/
#   4. Add entries to the era's manifest.toml with full provenance
#   5. Run verify_checksums.sh to confirm integrity
#
# Era identification: Each block's CBOR starts with [era_tag, block_body].
# Era tags: 0=ByronEBB, 1=ByronRegular, 2=Shelley, 3=Allegra, 4=Mary,
#           5=Alonzo, 6=Babbage, 7=Conway
#
# Does NOT decode, parse, normalize, or interpret CBOR content.

CORPUS_DIR="$(cd "$(dirname "$0")" && pwd)"
GOLDEN_DIR="$CORPUS_DIR/golden"

echo "Corpus status:"
echo ""

ERAS=(byron shelley allegra mary alonzo babbage conway)
TOTAL=0

for era in "${ERAS[@]}"; do
    blocks_dir="$GOLDEN_DIR/$era/blocks"
    count=$(find "$blocks_dir" -name '*.cbor' 2>/dev/null | wc -l)
    TOTAL=$((TOTAL + count))
    if [ "$count" -gt 0 ]; then
        size=$(du -sh "$blocks_dir" 2>/dev/null | awk '{print $1}')
        echo "  $era: $count blocks ($size)"
    else
        echo "  $era: no blocks (needs acquisition)"
    fi
done

echo ""
echo "Total: $TOTAL blocks"

if [ "$TOTAL" -eq 0 ]; then
    echo ""
    echo "No fixtures found. See README.md for acquisition instructions."
    exit 1
fi
