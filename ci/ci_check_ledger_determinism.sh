#!/usr/bin/env bash
set -euo pipefail

# Verify ledger determinism: same inputs produce same outputs.
# Runs ade_ledger tests that exercise determinism properties.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

echo "Running ledger determinism tests..."
cargo test -p ade_ledger -- deterministic 2>&1

echo "Running ledger rules tests..."
cargo test -p ade_ledger -- rules:: 2>&1

echo "PASS: Ledger determinism checks complete"
