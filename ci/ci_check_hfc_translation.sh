#!/usr/bin/env bash
set -euo pipefail

# CE-73-semantic: Verify ledger-side HFC translation produces semantically
# correct state across all 6 era transitions.
#
# Authoritative test for DC-EPOCH-02 (status: enforced). The associated
# CE-73-bytes claim (byte-identical Haskell ExtLedgerState CBOR) is
# explicit non-goal per CE-79 Tier 4; see docs/active/CE-73_reclassification.md.
#
# Three test surfaces:
#   - translation_summary_proof: 22/22 encoding-independent fields match
#     oracle at the Allegra→Mary boundary; predicates uniform across the
#     other non-Byron transitions.
#   - translation_comparison_surface: per-field comparison harness output.
#   - transition_proof_surface: full pre-HFC-snapshot → translate → replay
#     boundary block diagnostic.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

echo "=== CE-73-semantic: HFC Ledger-Side Translation ==="

cargo test --test translation_summary_proof -- --nocapture 2>&1
cargo test --test translation_comparison_surface -- --nocapture 2>&1
cargo test --test transition_proof_surface -- --nocapture 2>&1

echo "PASS: CE-73-semantic — all 3 translation proof surfaces"
