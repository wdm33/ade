#!/usr/bin/env bash
# CE-F3 (PHASE4-N-AE.F / DC-NODE-16) — receive idempotency at the durable-admit
# chokepoint. The already-have skip MUST be:
#   - HASH-keyed (get_block_by_hash on the decoded block hash), never slot-only;
#   - placed BEFORE the BLUE chokepoint reducer (forward_sync_step), so the no-op
#     runs no reducer step and appends nothing to the WAL;
# so a DIFFERENT block (different hash) at/before the last-applied slot is NOT
# short-circuited and still fails closed in the unchanged BLUE authority.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

PUMP="crates/ade_runtime/src/forward_sync/pump.rs"
fail() { echo "FAIL ci_check_receive_idempotency: $1" >&2; exit 1; }
[ -f "$PUMP" ] || fail "missing $PUMP"

# Production code only (exclude the #[cfg(test)] module).
PROD="$(awk '/#\[cfg\(test\)\]/{exit} {print}' "$PUMP")"

# (1) The idempotency gate queries the durable store by HASH.
printf '%s\n' "$PROD" | grep -q 'get_block_by_hash(&decoded.block_hash)' \
  || fail "the gate must query the durable store by HASH: get_block_by_hash(&decoded.block_hash)"

# (2) The no-op early-return exists (Ok(None)) and follows the hash query.
printf '%s\n' "$PROD" \
  | awk '/get_block_by_hash\(&decoded.block_hash\)/{f=1} f&&/return Ok\(None\)/{print "ok"; exit}' \
  | grep -q ok \
  || fail "the no-op 'return Ok(None)' must be gated by the get_block_by_hash hit (hash-keyed, not slot-only)"

# (3) The gate PRECEDES the BLUE chokepoint reducer (forward_sync_step), so a hit
#     runs no reducer step / no WAL append.
gbh=$(printf '%s\n' "$PROD" | grep -n 'get_block_by_hash(&decoded.block_hash)' | head -1 | cut -d: -f1)
step=$(printf '%s\n' "$PROD" | grep -n 'forward_sync_step(' | head -1 | cut -d: -f1)
[ -n "$gbh" ] && [ -n "$step" ] && [ "$gbh" -lt "$step" ] \
  || fail "the idempotency gate must precede the BLUE reducer (forward_sync_step) so the no-op runs no reducer step"

# (4) The hit is slot-consistent (belt-and-braces) — a stored block at a different
#     slot than the decoded slot does NOT short-circuit.
printf '%s\n' "$PROD" | grep -q 'stored.slot == decoded.header_input.slot' \
  || fail "the gate must confirm stored.slot == decoded.header_input.slot before the no-op"

echo "OK ci_check_receive_idempotency: hash-keyed already-have no-op precedes the BLUE reducer; a different block still reaches the authority and fails closed."
