#!/usr/bin/env bash
# DURABLE-ADMISSION-BYTES memory guardrail (BA-08 owned-RSS must not regress).
#
# The LIVE admission runner persists each admitted block's preserved bytes to
# the disk-backed ChainDb (ChainDb::put_block) and drops them at the end of the
# admission step. It MUST NOT build or retain a heap-resident block-bytes
# collection during live follow -- that is the memory-regression path the
# MEM-OPT-UTXO-DISK cluster eliminated for the static UTxO.
#
# Scope: the LIVE runner ONLY. The WarmStart replay map
# (node_lifecycle.rs::warm_start_recovery, a BTreeMap<Hash32, Vec<u8>>) is a
# bootstrap-only recovery surface, built once from the WAL across a restart and
# dropped after replay -- it is intentionally NOT covered by this gate.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
RUNNER="$ROOT/crates/ade_node/src/admission/runner.rs"
fail=0

# Forbidden: a Vec-of-byte-vecs that would accumulate followed block bytes.
if grep -nE 'Vec *< *Vec *< *u8 *> *>' "$RUNNER"; then
  echo "FAIL: Vec<Vec<u8>> in the live admission runner (block-bytes buffering)."
  fail=1
fi
# Forbidden: a hash->bytes map that would materialize a live replay map.
if grep -nE '(BTreeMap|HashMap) *<[^>]*, *Vec *< *u8 *> *>' "$RUNNER"; then
  echo "FAIL: a <_, Vec<u8>> map in the live admission runner (live replay map)."
  fail=1
fi

if [ "$fail" -eq 0 ]; then
  echo "OK: admission runner holds no heap-resident block-bytes map; bytes are"
  echo "    written to the disk-backed ChainDb per step and released (memory guardrail)."
fi
exit "$fail"
