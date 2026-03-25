#!/usr/bin/env bash
set -euo pipefail

# CE-74: Verify ledger determinism — same inputs produce identical state.
#
# Applies the same block sequence twice from identical initial state,
# asserts all state fields are identical. Covers all 7 eras with both
# single-block and multi-block sequences.
#
# Authoritative test for DC-LEDGER-01.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

echo "=== CE-74: Ledger Determinism ==="
echo "Running determinism tests across all 7 eras..."

cargo test --test ledger_determinism -- --nocapture 2>&1

echo "PASS: CE-74 ledger determinism — all 7 eras"
