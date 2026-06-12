#!/usr/bin/env bash
# ci_check_live_selector_dispatch.sh -- PHASE4-N-AO S3 (DC-NODE-36).
#
# The live NeedsForkChoice dispatch (run_participant_sync's competing arm, via the
# RED `dispatch_competing_fork_choice` + the `decide_fork_switch` helper) routes a
# competing candidate to the SOLE BLUE select_best_chain, binds the fork anchor to
# Ade's durable stored point (never peer data), obtains the fork-point chain_dep by
# a read-only materialize, and DECIDES ONLY -- a win sets a provisional
# PendingForkSwitch + the DC-NODE-28 forge fence and applies NOTHING (S4 applies).
set -euo pipefail

M="crates/ade_node/src/node_lifecycle.rs"
fail() { echo "FAIL (ci_check_live_selector_dispatch): $1" >&2; exit 1; }
[ -f "$M" ] || fail "module $M missing"

# Extract the S3 dispatch path: from `fn decide_fork_switch` up to (not incl.)
# `pub async fn run_participant_sync`. The anti-apply checks are scoped HERE so
# they do not match apply_chain_event (the legitimate RollBack-arm apply authority)
# elsewhere in the file.
REGION="$(awk '/fn decide_fork_switch/{f=1} /pub async fn run_participant_sync/{f=0} f' "$M")"
[ -n "$REGION" ] || fail "could not locate the S3 dispatch region"
# CODE = the region with standalone comment lines stripped. The doc comments
# legitimately NAME the prohibited apply tokens (commit_rollback / pump_block /
# WalEntry::RollBack) to describe the S3/S4 boundary; the checks are on CODE.
CODE="$(echo "$REGION" | grep -vE '^[[:space:]]*//')"

# (A) SOLE selector: select_best_chain is called; no second selector / stream
# processor / density heuristic in the dispatch path.
echo "$CODE" | grep -Eq 'select_best_chain\(' \
  || fail "the dispatch must route to select_best_chain"
if echo "$CODE" | grep -Eq 'process_stream_input|fork_choice_density|second_selector'; then
  fail "select_best_chain must be the ONLY selector in the dispatch path"
fi

# (B) Proof center (PHASE4-N-AO S7, DC-NODE-38): the fork anchor is the durable
# LAST COMMON ANCESTOR discovered by walk_to_durable_lca (which resolves the LCA
# from the DURABLE store + binds its STORED slot internally), NOT the competing
# block's immediate parent; the anchor binds lca.anchor_slot; anchor_chain_dep via
# a read-only materialize at the LCA. No peer-supplied anchor slot/hash.
echo "$CODE" | grep -Eq 'walk_to_durable_lca\(' \
  || fail "the fork anchor must be discovered via the durable LCA walk (walk_to_durable_lca)"
echo "$CODE" | grep -Eq 'slot:[[:space:]]*lca\.anchor_slot' \
  || fail "the anchor must bind the durable LCA's STORED slot (lca.anchor_slot), never peer data"
echo "$CODE" | grep -Eq 'materialize_rolled_back_state\(' \
  || fail "anchor_chain_dep must come from a read-only materialize_rolled_back_state"

# (C) DECIDE-ONLY: no apply in the dispatch path -- no rollback-commit, no
# pump_block of a winner, no durable WAL append (that is S4).
for forbidden in 'commit_rollback' 'pump_block' 'WalEntry::RollBack' 'wal\.append' '\.append\(WalEntry'; do
  if echo "$CODE" | grep -Eq "$forbidden"; then
    fail "S3 is decide-only -- '$forbidden' must NOT appear in the dispatch path"
  fi
done

# (D) DC-NODE-28 forge fence + the provisional decision are set on a win.
echo "$CODE" | grep -Eq '\*pending_reselection = true' \
  || fail "a fork-choice win must set the DC-NODE-28 forge fence (*pending_reselection = true)"
echo "$CODE" | grep -Eq '\*pending_fork_switch = Some' \
  || fail "a fork-choice win must set the provisional PendingForkSwitch"

# (E) Candidates come from S2 build_candidate_fragment; nothing minted.
echo "$CODE" | grep -Eq 'build_candidate_fragment\(' \
  || fail "candidates must come from S2 build_candidate_fragment"
if echo "$CODE" | grep -Eq 'ValidatedHeaderSummary[[:space:]]*\{'; then
  fail "the dispatch must NOT mint a ValidatedHeaderSummary (S2 validates; S3 selects)"
fi

# A competing branch that cannot reach a durable LCA (genesis predecessor, a gap,
# over-k, a lying parent link, a cache self-binding violation) fails closed as a
# NO-OP -- keep the current validated chain, never adopt an un-anchorable branch.
# (Pre-S7 this was Err(UnexpectedRollback) on a non-durable immediate parent -- the
# live-geometry gap CE-AO-6 surfaced; S7 walks the preserved links instead.) The
# LcaError arm of the walk match is a keep-current `Err(_) => return Ok(())`.
echo "$CODE" | grep -Eq 'Err\(_\) => return Ok\(\(\)\)' \
  || fail "an un-anchorable competing branch (LcaError) must fail closed as a no-op (return Ok(()))"

echo "OK: live selector dispatch is sole-selector + durable-LCA-anchor-bound + read-only + decide-only (DC-NODE-36/38)"
