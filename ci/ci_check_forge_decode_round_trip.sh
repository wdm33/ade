#!/usr/bin/env bash
set -euo pipefail

# PHASE4-N-V — producer/validator codec symmetry (CN-FORGE-03).
#
# forge_block MUST wrap its output in the canonical era envelope so it
# round-trips through the same decode_block authority that validates
# received blocks. The bug this gate locks: forge_block emitting a bare
# Conway block (array(5)) with no [era, block] envelope, which
# decode_block_envelope rejects at offset 0 before any validation.
#
# Guards:
#   1. forge_block wraps its output via ade_codec ... encode_block_envelope.
#   2. The forge<->decode round-trip regression test exists.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
FORGE_RS="$REPO_ROOT/crates/ade_ledger/src/producer/forge.rs"

if [ ! -f "$FORGE_RS" ]; then
    echo "[ci_check_forge_decode_round_trip] FAIL: missing $FORGE_RS"
    exit 1
fi

FAILED=0
print_fail() {
    echo "[ci_check_forge_decode_round_trip] FAIL: $1"
    FAILED=1
}

# Guard 1 — forge output is era-enveloped via the single encoder.
if ! grep -qE 'encode_block_envelope' "$FORGE_RS"; then
    print_fail "Guard 1 — forge.rs does not call encode_block_envelope (forge output must be era-enveloped so it round-trips through decode_block)"
fi

# Guard 2 — the forge<->decode round-trip regression test exists.
if ! grep -qE 'fn forge_block_output_decodes_via_decode_block' "$FORGE_RS"; then
    print_fail "Guard 2 — the forge<->decode round-trip test (forge_block_output_decodes_via_decode_block) is missing"
fi

if [ "$FAILED" -eq 0 ]; then
    echo "[ci_check_forge_decode_round_trip] PASS (2/2 guards green)"
    exit 0
else
    exit 1
fi
