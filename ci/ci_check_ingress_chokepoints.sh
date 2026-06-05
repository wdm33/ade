#!/usr/bin/env bash
set -euo pipefail

# Verify that all CBOR decode flows go through named chokepoints.
# Pattern scan: no raw CBOR decoding outside named chokepoint functions.
#
# Named chokepoints (grows with each era slice):
#   - decode_block_envelope                              (ade_codec)
#   - decode_byron_ebb_block, decode_byron_regular_block (ade_codec)
#   - decode_shelley_block, decode_allegra_block,
#     decode_mary_block, decode_alonzo_block,
#     decode_babbage_block, decode_conway_block          (ade_codec)
#   - decode_address                                     (ade_codec)
#   - PlutusScript::from_cbor                            (ade_plutus)
#
# Block CBOR ingress lives in ade_codec. Plutus script CBOR is a
# distinct ingress surface and lives in ade_plutus — its chokepoint
# file is exempt from Check 3 (see allowlist below).
#
# Invariants: DC-INGRESS-01, T-INGRESS-01

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"

BLUE_CRATES=("ade_codec" "ade_types" "ade_crypto" "ade_core" "ade_ledger" "ade_plutus")
ADE_NETWORK_BLUE_PATHS=(
    "crates/ade_network/src/mux/frame.rs"
    "crates/ade_network/src/codec"
    "crates/ade_network/src/handshake"
    "crates/ade_network/src/chain_sync"
    "crates/ade_network/src/block_fetch"
    "crates/ade_network/src/tx_submission"
    "crates/ade_network/src/keep_alive"
    "crates/ade_network/src/peer_sharing"
    "crates/ade_network/src/n2c"
)

FAILED=0

# Check 1: PreservedCbor::new must only appear in ade_codec (pub(crate) enforced by compiler,
# but verify no leakage through re-export or unsafe workarounds)
for crate in "${BLUE_CRATES[@]}"; do
    if [ "$crate" = "ade_codec" ]; then
        continue
    fi

    SRC_DIR="$REPO_ROOT/crates/$crate/src"
    if [ ! -d "$SRC_DIR" ]; then
        continue
    fi

    matches=$(grep -rn 'PreservedCbor::new\|PreservedCbor\s*{' "$SRC_DIR" --include='*.rs' 2>/dev/null | \
        grep -v ':[0-9]*:\s*//' || true)

    if [ -n "$matches" ]; then
        echo "FAIL: PreservedCbor construction outside ade_codec in $crate:"
        echo "$matches"
        FAILED=1
    fi
done

for path in "${ADE_NETWORK_BLUE_PATHS[@]}"; do
    FULL_PATH="$REPO_ROOT/$path"
    if [ ! -e "$FULL_PATH" ]; then
        continue
    fi
    matches=$(grep -rn 'PreservedCbor::new\|PreservedCbor\s*{' "$FULL_PATH" --include='*.rs' 2>/dev/null | \
        grep -v ':[0-9]*:\s*//' || true)

    if [ -n "$matches" ]; then
        echo "FAIL: PreservedCbor construction outside ade_codec in ade_network path $path:"
        echo "$matches"
        FAILED=1
    fi
done

# Check 2: Verify that decode_block_envelope exists as a named chokepoint
ENVELOPE_FILE="$REPO_ROOT/crates/ade_codec/src/cbor/envelope.rs"
if [ ! -f "$ENVELOPE_FILE" ]; then
    echo "FAIL: Missing envelope chokepoint file: $ENVELOPE_FILE"
    FAILED=1
else
    if ! grep -q 'pub fn decode_block_envelope' "$ENVELOPE_FILE"; then
        echo "FAIL: decode_block_envelope function not found in envelope.rs"
        FAILED=1
    fi
fi

# Check 3: No direct minicbor::decode or raw CBOR parsing outside ade_codec
# Allowlist: ade_plutus/src/evaluator.rs hosts PlutusScript::from_cbor,
# the named chokepoint for Plutus script ingress (a distinct surface from
# block CBOR, with its own decoder via aiken/pallas).
for crate in "${BLUE_CRATES[@]}"; do
    if [ "$crate" = "ade_codec" ]; then
        continue
    fi

    SRC_DIR="$REPO_ROOT/crates/$crate/src"
    if [ ! -d "$SRC_DIR" ]; then
        continue
    fi

    matches=$(grep -rn 'minicbor::decode\|minicbor::Decode\|from_cbor\|cbor_decode' "$SRC_DIR" --include='*.rs' 2>/dev/null | \
        grep -v ':[0-9]*:\s*//' | \
        grep -v '/ade_plutus/src/evaluator\.rs:' | \
        grep -v 'minicbor::decode::Error' || true)

    if [ -n "$matches" ]; then
        echo "FAIL: Raw CBOR decoding outside ade_codec in $crate:"
        echo "$matches"
        FAILED=1
    fi
done

for path in "${ADE_NETWORK_BLUE_PATHS[@]}"; do
    FULL_PATH="$REPO_ROOT/$path"
    if [ ! -e "$FULL_PATH" ]; then
        continue
    fi
    matches=$(grep -rn 'minicbor::decode\|minicbor::Decode\|from_cbor\|cbor_decode' "$FULL_PATH" --include='*.rs' 2>/dev/null | \
        grep -v ':[0-9]*:\s*//' | \
        grep -v 'minicbor::decode::Error' || true)

    if [ -n "$matches" ]; then
        echo "FAIL: Raw CBOR decoding outside ade_codec in ade_network path $path:"
        echo "$matches"
        FAILED=1
    fi
done

if [ "$FAILED" -eq 0 ]; then
    echo "PASS: All decode flows through named chokepoints"
    exit 0
else
    exit 1
fi
