#!/usr/bin/env bash
#
# ci_check_no_haskell_fingerprint_equality.sh — PHASE4-N-Y S5 gate (CE-Y-13,
# DC-COMPAT-01).
#
# Cardano compatibility may be proven ONLY on observable surfaces (per-block
# verdict, selected tip hash, block hashes, `query utxo`, transcripts). It is
# FORBIDDEN to assert that Ade's internal ledger `fingerprint` equals a
# Haskell / cardano-node serialized-state hash.
#
# This gate is a negative grep over the test tree. It fails if any line is an
# equality assertion that pairs Ade's `fingerprint` with a Haskell/oracle
# serialized-state hash — i.e. a line that:
#   * is an equality assertion (`assert_eq!` or contains `==`), AND
#   * mentions `fingerprint`, AND
#   * mentions an oracle/haskell serialized-state-hash token
#     (`oracle`, `haskell`, or `serialized_state` / `serialized-state`).
#
# It is deliberately precise so it does NOT flag the legitimate Ade-vs-Ade
# internal cross-path fingerprint equality (S4's
# `genesis_path_fp_equals_snapshot_path_fp`, which compares two Ade-derived
# fingerprints and names neither an oracle nor a Haskell hash), nor doc/comment
# lines (which are not assertions).

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

# Search Rust sources across the workspace (inline #[cfg(test)] modules live in
# src/, integration tests in tests/). Match assertion-shaped equality lines.
HITS=$(grep -rnE '(assert_eq!|==)' --include='*.rs' crates/ \
  | grep -iE 'fingerprint' \
  | grep -iE 'oracle|haskell|serialized[_-]state' \
  | grep -vE '^[^:]+:[0-9]+:\s*(//|/\*|\*)' \
  || true)

if [[ -n "$HITS" ]]; then
  echo "[ci_check_no_haskell_fingerprint_equality] FAIL — forbidden Ade-fingerprint == Haskell/oracle serialized-state-hash assertion(s):"
  echo "$HITS"
  echo
  echo "Compatibility must be proven on observable surfaces only (DC-COMPAT-01)."
  exit 1
fi

echo "[ci_check_no_haskell_fingerprint_equality] PASS (no fingerprint == Haskell/oracle serialized-state-hash assertion)"
