#!/usr/bin/env bash
# ci_check_live_blockfetch_byte_only.sh -- PHASE4-N-AO S6 (CE-AO-6).
#
# The live BlockFetch bridge transports BYTES, not truth. PrefetchedBranchBodies
# (the relay-loop fill from a live RequestRange) carries bytes only: no selection,
# no validity verdict, no rollback, no fence. apply_fork_switch (S4) remains the
# sole adopter; prevalidate_branch the sole gate on commit_rollback.
set -euo pipefail

FS="crates/ade_node/src/fork_switch.rs"
NL="crates/ade_node/src/node_lifecycle.rs"
fail() { echo "FAIL (ci_check_live_blockfetch_byte_only): $1" >&2; exit 1; }
[ -f "$FS" ] || fail "module $FS missing"

# The PrefetchedBranchBodies type + its BranchBodySource impl -- the byte carrier.
# Bounded at the next item (BranchProofError); comment lines stripped (the doc
# above the struct legitimately names prove_fork_switch/prevalidate_branch).
REGION="$(awk '/struct PrefetchedBranchBodies/{f=1} /pub enum BranchProofError/{f=0} f' "$FS" | grep -vE '^[[:space:]]*//')"
[ -n "$REGION" ] || fail "could not locate PrefetchedBranchBodies"

# (A) byte-only: the carrier must not certify selection / validity / rollback / fence.
for forbidden in 'select_best_chain' 'prevalidate_branch' 'commit_rollback' 'pending_reselection' 'block_validity' 'ValidatedHeaderSummary' 'verdict' 'apply_chain_event'; do
  if echo "$REGION" | grep -qE "$forbidden"; then
    fail "PrefetchedBranchBodies must carry BYTES only -- '$forbidden' must not appear"
  fi
done

# (B) it IS a BranchBodySource returning bytes (or a fetch error), nothing richer.
echo "$REGION" | grep -qE 'impl BranchBodySource for PrefetchedBranchBodies' \
  || fail "PrefetchedBranchBodies must impl BranchBodySource (the byte seam)"
echo "$REGION" | grep -qE 'Result<Vec<u8>, FetchError>' \
  || fail "fetch_body must return Vec<u8> bytes (or FetchError), nothing richer"

# (C) apply_fork_switch is still the sole adopter; NullBranchBodySource remains the
# no-mux fallback (the relay loop holds the fence when no fetch is possible).
grep -qE 'NullBranchBodySource' "$NL" \
  || fail "the relay loop must keep NullBranchBodySource as the no-mux fallback"
grep -qE 'apply_fork_switch\(' "$NL" \
  || fail "apply_fork_switch must remain the sole adopter in the relay loop"

echo "OK: live BlockFetch bridge is byte-only; S4 remains the sole adopter (CE-AO-6 boundary)"
