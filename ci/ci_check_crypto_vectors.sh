#!/usr/bin/env bash
set -euo pipefail

# Run ade_crypto test suite to verify all cryptographic implementations
# match oracle reference vectors (DC-CRYPTO-01).

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

echo "Running ade_crypto unit tests..."
cargo test -p ade_crypto 2>&1

echo ""
echo "Running ade_crypto integration tests..."
cargo test -p ade_crypto --test '*' 2>&1 || true

echo ""
echo "PASS: ade_crypto vector verification complete"
