#!/usr/bin/env bash
set -euo pipefail

# CN-FOLLOW-01 / DC-FOLLOW-FORGE-01: Participant venue forges on the AO-selected durable
# head. A keyed producer in the Participant venue follows the AO-selected chain
# (run_participant_sync + fork-choice + store rewind, proven CN-CONS-03) and must PRODUCE
# on it: the forge base is the durable ChainDb::tip (the AO-selected servable head), fenced
# by DC-NODE-28 (pending fork-choice / reselection / missing-bridge), NOT the per-tick
# DC-NODE-15 exact-equality re-check the racing frontier makes unsatisfiable.
#
# Asserts (production bodies of node_sync.rs + node_lifecycle.rs):
#  (a) participant_forge_decision exists, returns ExtendOnSelectedHead on the durable head,
#      and fences on the DC-NODE-28 pending state (ForkChoicePending);
#  (b) the GREEN decision NEVER reaches a chain selector (select_best_chain / chain_selector
#      / fork_choice) -- it consumes the AO-selected durable result, never re-selects;
#  (c) the GREEN decision constructs NO FindIntersect point-list (no DC-NODE-42
#      resurrection) and carries NO kes/vrf signing-key material (signing stays RED);
#  (d) the ForgeTick routes VenueRole::Participant to participant_forge_decision, the forge
#      base is the durable ChainDb::tip (selected_tip), and there is no Origin fallback when
#      a durable head exists (the cold-start Genesis path is gated on selected_tip.is_none()).
#
# Fails closed if a future change routes the Participant forge base through a chain selector,
# re-introduces the point-list, leaks signing keys into the GREEN decision, drops the
# DC-NODE-28 fence, or adds an Origin fallback for a populated durable head.
#
# Repo-root-relative. Mirrors ci/ci_check_local_durable_forge_base.sh.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

SYNC="crates/ade_node/src/node_sync.rs"
LIFECYCLE="crates/ade_node/src/node_lifecycle.rs"

for f in "$SYNC" "$LIFECYCLE"; do
    if [[ ! -f "$f" ]]; then echo "FAIL: $f not found"; exit 1; fi
done

FAILED=0
fail() { echo "FAIL (participant-forge-on-selected-head): $1"; FAILED=1; }

# Production body (drop #[cfg(test)]; strip line/doc comments so commentary naming a
# token does not trip the greps).
prod_body() { awk '/#\[cfg\(test\)\]/{exit} {print}' "$1" | sed -E 's://.*::'; }
SYNC_PROD="$(prod_body "$SYNC")"
LIFE_PROD="$(prod_body "$LIFECYCLE")"
if [[ -z "$SYNC_PROD" || -z "$LIFE_PROD" ]]; then
    echo "FAIL: could not isolate production bodies"; exit 1
fi

DECISION_FN="$(awk '/pub fn participant_forge_decision/{c=1} c{print} c&&/^}/{exit}' <<<"$SYNC_PROD")"
LOOP_FN="$(awk '/pub async fn run_relay_loop_with_sched/{c=1} c{print} c&&/^}/{exit}' <<<"$LIFE_PROD")"

# --- (a) the decision exists + forges on the durable head + fences on DC-NODE-28 -
if [[ -z "$DECISION_FN" ]]; then
    fail "pub fn participant_forge_decision not found in $SYNC"
else
    if ! grep -qE 'ExtendOnSelectedHead' <<<"$DECISION_FN"; then
        fail "participant_forge_decision has no ExtendOnSelectedHead path (the extend forges on the AO-selected durable head)"
    fi
    # DC-NODE-28: the fence must read the pending fork-choice / reselection / missing-bridge state.
    if ! grep -qE 'pending_reselection|pending_fork_switch|pending_missing_bridge' <<<"$DECISION_FN"; then
        fail "participant_forge_decision does not fence on the DC-NODE-28 pending state"
    fi
    if ! grep -qE 'ForkChoicePending' <<<"$DECISION_FN"; then
        fail "participant_forge_decision has no ForkChoicePending refusal (DC-NODE-28 fence)"
    fi
    # The forge base must be a present durable tip byte-equal the latched extend head;
    # an absent durable tip must fail closed (NO Origin / cold-start fallback in the decision).
    if ! grep -qE 'NoDurableServableTip' <<<"$DECISION_FN"; then
        fail "participant_forge_decision does not fail closed on an absent durable servable tip (no silent fallback)"
    fi
fi

# --- (b) no chain selector on the GREEN decision (DC-CONS-03 untouched) ---------
for tok in 'select_best_chain' 'chain_selector' 'fork_choice'; do
    if grep -qE "$tok" <<<"$DECISION_FN"; then
        fail "participant_forge_decision references a chain selector ($tok) -- it must CONSUME the AO result, never re-select (no duplicate fork-choice)"
    fi
done

# --- (c) no FindIntersect point-list (DC-NODE-42) + no signing-key material ------
if grep -qE 'FindIntersect|point_list|points\s*:|Vec<\s*Point\s*>' <<<"$DECISION_FN"; then
    fail "participant_forge_decision constructs a FindIntersect point-list (DC-NODE-42 is quarantined; do not resurrect it in the decision)"
fi
for tok in 'kes_sign' 'kes_skey' 'KesSecret' 'kes_secret' 'vrf_skey' 'VrfSecret' 'vrf_secret' 'sign_header'; do
    if grep -qiE "$tok" <<<"$DECISION_FN"; then
        fail "participant_forge_decision references signing-key material ($tok) -- signing stays RED, never in the GREEN decision"
    fi
done

# --- (d) the loop routes Participant -> the decision; base = ChainDb::tip; no Origin fallback -
if [[ -z "$LOOP_FN" ]]; then
    fail "could not isolate run_relay_loop_with_sched body"
else
    if ! grep -qE 'participant_forge_decision\(' <<<"$LOOP_FN"; then
        fail "the loop does not call participant_forge_decision (the Participant fenced forge-base decision)"
    fi
    if ! grep -qE 'VenueRole::Participant' <<<"$LOOP_FN"; then
        fail "the ForgeTick does not branch on VenueRole::Participant"
    fi
    SELECTED_LINE="$(grep -E 'let selected_tip' <<<"$LOOP_FN" | head -1)"
    if [[ -z "$SELECTED_LINE" ]]; then
        fail "the loop no longer binds a selected_tip (the durable forge base)"
    elif ! grep -qE 'ChainDb::tip\(' <<<"$SELECTED_LINE"; then
        fail "the ForgeTick selected_tip is not derived from ChainDb::tip( -- the forge base must be the local durable tip"
    fi
    # No Origin fallback when a durable head exists: the only cold-start (Genesis) path is
    # gated on selected_tip.is_none() AND recovered.tip.is_none(); a populated durable head
    # never cold-starts.
    if ! grep -qE 'selected_tip\.is_none\(\)' <<<"$LOOP_FN"; then
        fail "the cold-start Genesis path is not gated on selected_tip.is_none() -- an Origin fallback with a durable head present is forbidden"
    fi
fi

if (( FAILED == 0 )); then
    echo "OK (participant-forge-on-selected-head): participant_forge_decision forges ExtendOnSelectedHead on the durable ChainDb::tip, fences on the DC-NODE-28 pending state (ForkChoicePending) + fails closed on an absent base (NoDurableServableTip); no chain selector / no FindIntersect point-list / no signing-key material in the GREEN decision; the ForgeTick routes VenueRole::Participant to it with the forge base = local durable ChainDb::tip and no Origin fallback when a durable head exists (CN-FOLLOW-01 / DC-FOLLOW-FORGE-01; DC-NODE-28 fence; DC-CONS-03 untouched)"
fi
exit $FAILED
