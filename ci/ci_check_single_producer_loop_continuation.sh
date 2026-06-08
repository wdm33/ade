#!/usr/bin/env bash
set -euo pipefail

# PHASE4-N-AG (DC-NODE-19): single-producer forge-loop continuation after a
# structural follow-link EOF. In a certified single-producer venue already in the
# DC-NODE-18 extend state, a LoopState::Ending caused by a structural feed EOF
# does NOT terminate the forge loop — it continues forging the own certified
# durable spine, fenced to the certified-run conditions, until shutdown / a fatal
# error / an existing BLUE forge-validity bound / a competing chain. A
# loop-lifecycle refinement, NOT a fork-choice change.
#
# Asserts (run_relay_loop_with_sched production body):
#  (a) the loop threads the venue policy from (venue_role, forge_mode) into the
#      planner — `venue_policy(act.venue_role` — and keeps HaltOnFeedEnd as the
#      forge-off default (never a hard-wired HaltOnFeedEnd when forge is active);
#  (b) the Idle arm is venue-aware on loop_state: the Ending (dead-feed) branch
#      wakes on a clock-cadence timer (tokio::time::sleep) + shutdown — NEVER the
#      dead feed's source.wait_ready (which would park forever); the Continuing
#      branch keeps the feed-driven wait;
#  (c) the continuation reuses the DC-NODE-18 GREEN fence
#      (single_producer_forge_decision) — it never reimplements fork-choice
#      (no select_best_chain / chain_selector / fork_choice in the loop);
#  (d) the per-continuation certificate re-validation is present —
#      continuation_cert_missing + read_adoption_cert + the
#      AdoptionCertificateMissingOrMalformed fail-closed reason;
#  (e) the venue_policy projection (node_sync) yields ContinueInSingleProducerExtend
#      ONLY for an explicitly single-producer venue in the extend state, else
#      HaltOnFeedEnd.
#
# Fails closed if a future change hard-wires the loop's feed-end behavior, parks
# the dead-feed Idle on wait_ready, reimplements fork-choice in the loop, drops
# the per-continuation cert fence, or broadens the venue_policy projection.
#
# Repo-root-relative. Mirrors the other ci_check_*.sh gates.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

SYNC="crates/ade_node/src/node_sync.rs"
LIFECYCLE="crates/ade_node/src/node_lifecycle.rs"

for f in "$SYNC" "$LIFECYCLE"; do
    if [[ ! -f "$f" ]]; then
        echo "FAIL: $f not found"
        exit 1
    fi
done

FAILED=0
fail() { echo "FAIL (single-producer loop-continuation): $1"; FAILED=1; }

# Production body of each file (drop the #[cfg(test)] module; strip line/doc
# comments so commentary naming a token does not trip the greps).
prod_body() {
    awk '/#\[cfg\(test\)\]/{exit} {print}' "$1" | sed -E 's://.*::'
}

SYNC_PROD="$(prod_body "$SYNC")"
LIFE_PROD="$(prod_body "$LIFECYCLE")"
if [[ -z "$SYNC_PROD" || -z "$LIFE_PROD" ]]; then
    echo "FAIL: could not isolate production bodies"
    exit 1
fi

# Isolate the loop fn, its Idle arm, and the venue_policy projection.
LOOP_FN="$(awk '
    /pub async fn run_relay_loop_with_sched/ { capture=1 }
    capture { print }
    capture && /^}/ { exit }
' <<<"$LIFE_PROD")"
IDLE_ARM="$(awk '
    /LoopStep::Idle =>/ { capture=1 }
    capture { print }
    capture && /LoopStep::HaltCleanly =>/ { exit }
' <<<"$LOOP_FN")"
VENUE_FN="$(awk '
    /pub fn venue_policy/ { capture=1 }
    capture { print }
    capture && /^}/ { exit }
' <<<"$SYNC_PROD")"

if [[ -z "$LOOP_FN" ]]; then
    fail "could not isolate run_relay_loop_with_sched body"
fi

# --- (a) the loop threads the venue policy; HaltOnFeedEnd stays the forge-off default
if ! grep -qE 'venue_policy\(act\.venue_role' <<<"$LOOP_FN"; then
    fail "the loop does not thread venue_policy(act.venue_role, ..) into the planner (the continuation policy must be projected from the activation, not hard-wired)"
fi
if ! grep -qE 'VenuePolicy::HaltOnFeedEnd' <<<"$LOOP_FN"; then
    fail "the loop does not keep VenuePolicy::HaltOnFeedEnd as the forge-off / relay-only default"
fi
if ! grep -qE 'plan_loop_step\(' <<<"$LOOP_FN"; then
    fail "the loop does not call plan_loop_step"
fi

# --- (b) the Idle arm is venue-aware: dead-feed Ending wakes on a timer, NOT wait_ready
if [[ -z "$IDLE_ARM" ]]; then
    fail "could not isolate the LoopStep::Idle arm"
else
    if ! grep -qE 'LoopState::Ending' <<<"$IDLE_ARM"; then
        fail "the Idle arm does not branch on LoopState::Ending (the dead-feed continuation path)"
    fi
    if ! grep -qE 'tokio::time::sleep' <<<"$IDLE_ARM"; then
        fail "the Idle arm's Ending (dead-feed) branch does not wake on a clock-cadence timer (tokio::time::sleep) — it must NOT park forever on a dead feed"
    fi
    if ! grep -qE 'shutdown\.changed\(\)' <<<"$IDLE_ARM"; then
        fail "the Idle arm does not honor shutdown.changed() (operator shutdown must win)"
    fi
    if ! grep -qE 'source\.wait_ready\(\)' <<<"$IDLE_ARM"; then
        fail "the Idle arm's Continuing branch no longer keeps the feed-driven source.wait_ready() wait"
    fi
fi

# --- (c) the continuation reuses the DC-NODE-18 fence; no reimplemented fork-choice
if ! grep -qE 'single_producer_forge_decision\(' <<<"$LOOP_FN"; then
    fail "the loop does not reuse single_producer_forge_decision (the certified-run fence must be reused, not reimplemented)"
fi
for tok in 'select_best_chain' 'chain_selector' 'fork_choice'; do
    if grep -qE "$tok" <<<"$LOOP_FN"; then
        fail "the loop references a chain selector ($tok) — the continuation is a loop-lifecycle refinement, NOT fork-choice (DC-CONS-03 untouched)"
    fi
done

# --- (d) DC-NODE-20: the continuation no longer requires a certificate --------
# DC-NODE-20 supersedes DC-NODE-19's cert-fence clause: the extend-state continuation
# forges on the LOCAL durable spine (ChainDb::tip), NOT gated on an operator cert.
# The loop must NOT re-introduce the cert into the forge / continuation path.
if grep -qE 'continuation_cert_missing|AdoptionCertificateMissingOrMalformed' <<<"$LOOP_FN"; then
    fail "the loop still gates the continuation on a certificate (continuation_cert_missing / AdoptionCertificateMissingOrMalformed) — DC-NODE-20 forges on the local durable spine, not a cert"
fi
if grep -qE 'read_adoption_cert\(' <<<"$LOOP_FN"; then
    fail "the loop still reads the adoption cert in the forge / continuation path — the cert is evidence-only (DC-NODE-21); the forge base is ChainDb::tip"
fi

# --- (e) the venue_policy projection is single-producer-extend-only -----------
if [[ -z "$VENUE_FN" ]]; then
    fail "pub fn venue_policy not found in $SYNC"
else
    if ! grep -qE 'ContinueInSingleProducerExtend' <<<"$VENUE_FN"; then
        fail "venue_policy never returns ContinueInSingleProducerExtend"
    fi
    if ! grep -qE 'VenueRole::SingleProducer' <<<"$VENUE_FN"; then
        fail "venue_policy does not gate on VenueRole::SingleProducer"
    fi
    if ! grep -qE 'SingleProducerExtendOwnDurableSpine' <<<"$VENUE_FN"; then
        fail "venue_policy does not gate on the SingleProducerExtendOwnDurableSpine extend state"
    fi
    if ! grep -qE 'HaltOnFeedEnd' <<<"$VENUE_FN"; then
        fail "venue_policy does not default to HaltOnFeedEnd (every non-extend venue must halt on feed-end)"
    fi
fi

if (( FAILED == 0 )); then
    echo "OK (single-producer loop-continuation): the loop threads venue_policy(act.venue_role, &act.forge_mode) into plan_loop_step (HaltOnFeedEnd forge-off default); the Idle arm wakes the dead-feed Ending branch on a clock-cadence timer + shutdown (never source.wait_ready); the continuation reuses single_producer_forge_decision (no fork-choice); DC-NODE-20: the continuation forges on the LOCAL durable spine with NO cert in the path (the DC-NODE-19 cert-fence clause is superseded); venue_policy yields ContinueInSingleProducerExtend only for the single-producer extend state (DC-NODE-19 / DC-NODE-05 / DC-CONS-03 boundaries preserved)"
fi
exit $FAILED
