#!/usr/bin/env bash
set -euo pipefail

# Run the ChainDb storage-contract test suite.
#
# Enforces the chaindb invariants that the storage layer must hold
# for every impl (InMemoryChainDb, PersistentChainDb):
#   - Append-only block storage (DC-STORE-02, CN-STORE-05)
#   - Atomic snapshot write (DC-STORE-03, CN-STORE-04)
#   - Schema version stability across reopens
#   - Corruption detection on tampered magic / version
#
# Tier: 5 (storage layer divergence allowed per CE-79). Tests verify
# the contract that ade_runtime::chaindb publishes; impls are free
# to use any backing store as long as the contract holds.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

if ! cargo test -p ade_runtime --lib chaindb:: --quiet 2>&1 | tail -20; then
    echo "FAIL: ChainDb contract suite failed"
    exit 1
fi

echo "PASS: ChainDb contract suite (DC-STORE-02/03, CN-STORE-04/05)"
