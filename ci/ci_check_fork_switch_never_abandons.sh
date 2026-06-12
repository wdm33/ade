#!/usr/bin/env bash
# ci_check_fork_switch_never_abandons.sh -- PHASE4-N-AO S4 (DC-NODE-37).
#
# A PendingForkSwitch authorizes PROOF of the selected replacement branch, not a
# rollback. The proof (prove_fork_switch: fetch + read-only materialize +
# prevalidate_branch) is MUTATION-FREE and STRICTLY precedes the irreversible
# commit_rollback in apply_fork_switch. A proof failure leaves the current chain
# byte-unchanged and HOLDS the forge fence (never cleared by an unproven branch).
set -euo pipefail

M="crates/ade_node/src/node_lifecycle.rs"
fail() { echo "FAIL (ci_check_fork_switch_never_abandons): $1" >&2; exit 1; }
[ -f "$M" ] || fail "module $M missing"

# --- PROVE region: prove_fork_switch must be MUTATION-FREE ---
# (comment lines stripped; the doc comments legitimately name the apply tokens.)
PROVE="$(awk '/fn prove_fork_switch/{f=1} /pub fn apply_fork_switch/{f=0} f' "$M" | grep -vE '^[[:space:]]*//')"
[ -n "$PROVE" ] || fail "could not locate prove_fork_switch"
echo "$PROVE" | grep -Eq 'materialize_rolled_back_state\(' \
  || fail "prove must read-only materialize the fork anchor (CN-STORE-07)"
echo "$PROVE" | grep -Eq 'prevalidate_branch\(' \
  || fail "prove must call prevalidate_branch (the complete-branch proof)"
for forbidden in 'commit_rollback' 'apply_chain_event' 'pump_block' 'wal\.append' 'WalEntry::'; do
  if echo "$PROVE" | grep -Eq "$forbidden"; then
    fail "prove_fork_switch must be MUTATION-FREE -- '$forbidden' must not appear"
  fi
done

# --- APPLY region: apply_fork_switch must gate the commit on the proof ---
APPLY="$(awk '/pub fn apply_fork_switch/{f=1} /^#\[cfg\(test\)\]/{f=0} f' "$M" | grep -vE '^[[:space:]]*//')"
[ -n "$APPLY" ] || fail "could not locate apply_fork_switch"
echo "$APPLY" | grep -Eq 'prove_fork_switch\(' \
  || fail "apply must prove the branch (prove_fork_switch) first"
echo "$APPLY" | grep -Eq 'RollbackReason::ForkChoiceWin' \
  || fail "adoption must use RollbackReason::ForkChoiceWin"
echo "$APPLY" | grep -Eq 'switch\.fork_anchor\.hash' \
  || fail "the rollback target must bind the PendingForkSwitch durable-stored anchor (DC-NODE-29)"

# ORDERING: every ProofFailed early-return precedes the first commit. The proof
# gates the irreversible step -- no commit-then-repair.
first_apply="$(echo "$APPLY" | grep -nE 'apply_chain_event\(' | head -1 | cut -d: -f1)"
last_prooffail="$(echo "$APPLY" | grep -nE 'ForkSwitchOutcome::ProofFailed' | tail -1 | cut -d: -f1)"
[ -n "$first_apply" ] || fail "apply must adopt via apply_chain_event"
[ -n "$last_prooffail" ] || fail "apply must have a structured ProofFailed path"
if [ "$last_prooffail" -ge "$first_apply" ]; then
  fail "a ProofFailed return appears at/after the first commit -- the proof must gate the commit"
fi

# FENCE: the forge fence clears exactly once (the success path), AFTER the commit.
# A proof-failure path must NOT clear it (no silent 'failed winner, resume forging').
fence_clear="$(echo "$APPLY" | grep -nE '\*pending_reselection = false' | head -1 | cut -d: -f1)"
[ -n "$fence_clear" ] || fail "the success path must clear the forge fence after reconcile"
if [ "$fence_clear" -le "$first_apply" ]; then
  fail "the forge fence is cleared before/at the commit -- it must clear only after reconcile"
fi
n_fence="$(echo "$APPLY" | grep -cE '\*pending_reselection = false')"
[ "$n_fence" -eq 1 ] || fail "the forge fence must clear exactly once (the success path); found $n_fence"

echo "OK: fork-switch proof gates commit; prove is mutation-free; fence held on failure (DC-NODE-37)"
