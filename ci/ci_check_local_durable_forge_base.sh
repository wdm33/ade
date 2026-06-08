#!/usr/bin/env bash
set -euo pipefail

# PHASE4-N-AH (DC-NODE-20): local selected durable chain forge-base authority. In a
# declared rung-1 single-producer venue, after Ade self-admits a valid forged block
# through pump_block onto its local durable ChainDB spine, the forge base is the LOCAL
# selected durable tip (ChainDb::tip) -- NOT followed_peer_tip and NOT an operator
# adoption certificate. The extend state is ENTERED directly on self-admit; the cert
# is removed from the forge / proceed_to_forge path (evidence-only, DC-NODE-21).
#
# Asserts (production bodies of node_sync.rs + node_lifecycle.rs):
#  (a) forge_mode_after_admit enters SingleProducerExtendOwnDurableSpine DIRECTLY from
#      CaughtUpToPeerTip -- no FirstOwnBlockServed cert-wait;
#  (b) single_producer_forge_decision takes NO certificate parameter and has NO
#      cert-promotion (Promote) / await-cert (AwaitAdoptionCertificate) arm; the
#      extend decision is ExtendOwnSpine on the durable spine head;
#  (c) the run_relay_loop proceed_to_forge gate does NOT read the cert (no
#      read_adoption_cert / continuation_cert_missing) and the forge base is the
#      durable ChainDb::tip (selected_tip);
#  (d) the forge-base / decision path never reaches a chain selector (DC-CONS-03
#      untouched -- a competing candidate fails closed, never resolved).
#
# Fails closed if a future change re-introduces the cert into the forge-base /
# continuation authority, restores the FirstOwnBlockServed cert-wait, or routes the
# forge base through a chain selector.
#
# Repo-root-relative. Mirrors the other ci_check_*.sh gates.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

SYNC="crates/ade_node/src/node_sync.rs"
LIFECYCLE="crates/ade_node/src/node_lifecycle.rs"

for f in "$SYNC" "$LIFECYCLE"; do
    if [[ ! -f "$f" ]]; then echo "FAIL: $f not found"; exit 1; fi
done

FAILED=0
fail() { echo "FAIL (local-durable-forge-base): $1"; FAILED=1; }

# Production body (drop #[cfg(test)]; strip line/doc comments so commentary naming a
# token does not trip the greps).
prod_body() { awk '/#\[cfg\(test\)\]/{exit} {print}' "$1" | sed -E 's://.*::'; }
SYNC_PROD="$(prod_body "$SYNC")"
LIFE_PROD="$(prod_body "$LIFECYCLE")"
if [[ -z "$SYNC_PROD" || -z "$LIFE_PROD" ]]; then
    echo "FAIL: could not isolate production bodies"; exit 1
fi

ADMIT_FN="$(awk '/pub fn forge_mode_after_admit/{c=1} c{print} c&&/^}/{exit}' <<<"$SYNC_PROD")"
DECISION_FN="$(awk '/pub fn single_producer_forge_decision/{c=1} c{print} c&&/^}/{exit}' <<<"$SYNC_PROD")"
LOOP_FN="$(awk '/pub async fn run_relay_loop_with_sched/{c=1} c{print} c&&/^}/{exit}' <<<"$LIFE_PROD")"

# --- (a) self-admit enters the extend state directly (no FirstOwnBlockServed) -
if [[ -z "$ADMIT_FN" ]]; then
    fail "pub fn forge_mode_after_admit not found in $SYNC"
else
    if ! grep -qE 'SingleProducerExtendOwnDurableSpine' <<<"$ADMIT_FN"; then
        fail "forge_mode_after_admit does not enter the extend state directly on self-admit"
    fi
    if grep -qE 'FirstOwnBlockServed' <<<"$ADMIT_FN"; then
        fail "forge_mode_after_admit still routes through FirstOwnBlockServed (the cert-wait) — DC-NODE-20 folds it out"
    fi
fi

# --- (b) the decision takes no cert + has no cert-promotion / await arm -------
if [[ -z "$DECISION_FN" ]]; then
    fail "pub fn single_producer_forge_decision not found in $SYNC"
else
    if grep -qE 'cert *:|VenueAdoptionCertificate' <<<"$DECISION_FN"; then
        fail "single_producer_forge_decision still takes a certificate parameter — the forge base must not consult the cert (DC-NODE-20)"
    fi
    if grep -qE 'AwaitAdoptionCertificate|Promote' <<<"$DECISION_FN"; then
        fail "single_producer_forge_decision still has a cert-promotion / await-cert arm (DC-NODE-20 enters extend on self-admit)"
    fi
    if ! grep -qE 'ExtendOwnSpine' <<<"$DECISION_FN"; then
        fail "single_producer_forge_decision has no ExtendOwnSpine path (the extend forges on the durable spine head)"
    fi
fi

# --- (c) the loop's proceed_to_forge reads no cert; forge base = ChainDb::tip -
if [[ -z "$LOOP_FN" ]]; then
    fail "could not isolate run_relay_loop_with_sched body"
else
    if grep -qE 'read_adoption_cert\(|continuation_cert_missing' <<<"$LOOP_FN"; then
        fail "the loop still reads the adoption cert / gates the continuation on it — the cert is evidence-only (DC-NODE-21); the forge base is ChainDb::tip"
    fi
    SELECTED_LINE="$(grep -E 'let selected_tip' <<<"$LOOP_FN" | head -1)"
    if [[ -z "$SELECTED_LINE" ]]; then
        fail "the loop no longer binds a selected_tip (the durable forge base)"
    elif ! grep -qE 'ChainDb::tip\(' <<<"$SELECTED_LINE"; then
        fail "the ForgeTick selected_tip is not derived from ChainDb::tip( — the forge base must be the local durable tip"
    fi
    if ! grep -qE 'single_producer_forge_decision\(' <<<"$LOOP_FN"; then
        fail "the loop does not call single_producer_forge_decision (the fenced forge-base decision)"
    fi
fi

# --- (d) no chain selector on the forge-base / decision path (DC-CONS-03) -----
for tok in 'select_best_chain' 'chain_selector' 'fork_choice'; do
    if grep -qE "$tok" <<<"$DECISION_FN$ADMIT_FN"; then
        fail "the forge-base decision references a chain selector ($tok) — a competing candidate fails closed, never resolved (DC-CONS-03 is the rung-2 successor)"
    fi
done

if (( FAILED == 0 )); then
    echo "OK (local-durable-forge-base): forge_mode_after_admit enters SingleProducerExtendOwnDurableSpine directly on self-admit (no FirstOwnBlockServed cert-wait); single_producer_forge_decision takes no certificate + has no Promote/await arm + forges on the durable spine head (ExtendOwnSpine); the loop's proceed_to_forge reads no adoption cert and the forge base is the local durable ChainDb::tip; no chain selector on the forge-base path (DC-NODE-20 entry+continuation authority; DC-NODE-21 cert demoted; DC-CONS-03 untouched)"
fi
exit $FAILED
