#!/usr/bin/env bash
set -euo pipefail

# PHASE4-N-AF (DC-NODE-18): single-producer extend-own-durable-spine. After the
# initial DC-NODE-15 catch-up and the serve of the first own successor, a node in
# an EXPLICITLY single-producer venue may extend its OWN durable spine without the
# relay re-announcing each own block — promotion gated on an explicit RED
# venue-adoption certificate, NEVER inferred from self-admit, behind a fail-closed
# fence. A gate-APPLICABILITY refinement, NOT a fork-choice weakening.
#
# Asserts:
#  (a) the forge mode is an explicit ENUM with the four named states — never a
#      boolean (no `forge_mode: bool` / mode-as-bool);
#  (b) promotion into SingleProducerExtendOwnDurableSpine requires an explicit
#      certificate (the FirstOwnBlockServed arm matches on `cert`/adopted_tip and
#      has an AwaitAdoptionCertificate no-cert path) — never inferred from
#      self-admit;
#  (c) the venue-adoption certificate is ADMISSIBILITY-ONLY — it is never
#      persisted / replay-visible (no cert token co-occurs with a persist verb);
#  (d) the fence FAILS CLOSED to a typed structured SingleProducerFenceViolation
#      { reason, durable_tip, followed_peer_tip, observed_peer_tip, venue_role },
#      checking the venue role first;
#  (e) the mode / decision NEVER references a chain selector (select_best_chain /
#      chain_selector / fork_choice) — DC-CONS-03 untouched;
#  (f) the loop is mode-aware AND preserves the pure DC-NODE-15 default path.
#
# Fails closed if a future change collapses the mode to a boolean, lets self-admit
# promote without a certificate, persists the certificate, buries the fence in a
# log string, or routes the single-producer machinery into a chain selector.
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
fail() { echo "FAIL (single-producer extend-own-spine): $1"; FAILED=1; }

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

# Isolate the relevant production items.
isolate() { awk -v pat="$1" '$0 ~ pat { capture=1 } capture { print } capture && /^}/ { exit }' <<<"$2"; }
MODE_ENUM="$(isolate 'pub enum ForgeMode' "$SYNC_PROD")"
DECISION_FN="$(isolate 'pub fn single_producer_forge_decision' "$SYNC_PROD")"
REFUSED_ENUM="$(isolate 'pub enum ForgeRefused' "$SYNC_PROD")"
LOOP_FN="$(awk '
    /pub async fn run_relay_loop_with_sched/ { capture=1 }
    capture { print }
    capture && /^}/ { exit }
' <<<"$LIFE_PROD")"

# --- (a) the forge mode is an explicit enum with the four named states ------
if [[ -z "$MODE_ENUM" ]]; then
    fail "pub enum ForgeMode not found in $SYNC (the forge mode must be an explicit enum, not a boolean)"
else
    for st in 'InitialCatchupRequired' 'CaughtUpToPeerTip' 'FirstOwnBlockServed' 'SingleProducerExtendOwnDurableSpine'; do
        if ! grep -qE "$st" <<<"$MODE_ENUM"; then
            fail "ForgeMode is missing the '$st' state (the four-state machine is the closed mode surface)"
        fi
    done
fi
# The mode must never be represented as a boolean.
if grep -qE '\bforge_mode *: *bool\b' <<<"$SYNC_PROD$LIFE_PROD"; then
    fail "the forge mode is represented as a bool — it MUST be the ForgeMode enum (no booleans)"
fi

# --- (b) promotion requires an explicit certificate -------------------------
if [[ -z "$DECISION_FN" ]]; then
    fail "pub fn single_producer_forge_decision not found in $SYNC"
else
    if ! grep -qE '\bcert\b' <<<"$DECISION_FN"; then
        fail "single_producer_forge_decision does not consult a certificate (promotion must require explicit evidence)"
    fi
    if ! grep -qE 'Promote' <<<"$DECISION_FN"; then
        fail "single_producer_forge_decision has no Promote path"
    fi
    if ! grep -qE 'AwaitAdoptionCertificate' <<<"$DECISION_FN"; then
        fail "single_producer_forge_decision has no AwaitAdoptionCertificate path — without a cert it must AWAIT, never promote (no self-admit inference)"
    fi
    # The Promote must be guarded by a certificate match (adopted_tip), not unconditional.
    if ! grep -qE 'adopted_tip' <<<"$DECISION_FN"; then
        fail "single_producer_forge_decision Promote is not guarded by the certificate's adopted_tip (promotion must match the served own tip)"
    fi
fi

# --- (c) the certificate is admissibility-only — never persisted ------------
# No cert token may co-occur with a persistence verb on any production line.
CERT_TOKENS='VenueAdoptionCertificate|adoption_cert|read_adoption_cert'
PERSIST_VERBS='wal\.|append_seed|append_|put_|\.capture\(|pump_block|admit_forged|WalEntry|\.persist'
if grep -E "$CERT_TOKENS" <<<"$SYNC_PROD$LIFE_PROD" | grep -qE "$PERSIST_VERBS"; then
    fail "a venue-adoption-certificate token co-occurs with a persistence verb — the certificate MUST be admissibility-only (never persisted / replay-visible)"
fi

# --- (d) the fence fails closed to a typed structured violation -------------
if [[ -z "$REFUSED_ENUM" ]]; then
    fail "enum ForgeRefused not found in $SYNC"
else
    if ! grep -qE 'SingleProducerFenceViolation' <<<"$REFUSED_ENUM"; then
        fail "ForgeRefused has no SingleProducerFenceViolation variant (the fence must fail closed to a typed refusal)"
    fi
    for fld in 'reason' 'durable_tip' 'followed_peer_tip' 'observed_peer_tip' 'venue_role'; do
        if ! grep -qE "$fld" <<<"$REFUSED_ENUM"; then
            fail "SingleProducerFenceViolation does not carry the structured field '$fld'"
        fi
    done
fi
if [[ -n "$DECISION_FN" ]]; then
    if ! grep -qE 'SingleProducerFenceViolation' <<<"$DECISION_FN"; then
        fail "single_producer_forge_decision never returns SingleProducerFenceViolation (the fence must be reachable)"
    fi
    # The venue role must be checked (fail closed when not single-producer).
    if ! grep -qE 'VenueNotDeclaredSingleProducer' <<<"$DECISION_FN"; then
        fail "single_producer_forge_decision does not fail closed on a non-single-producer venue (VenueNotDeclaredSingleProducer)"
    fi
fi

# --- (e) the mode / decision never references a chain selector --------------
for tok in 'select_best_chain' 'chain_selector' 'fork_choice'; do
    if grep -qE "$tok" <<<"$DECISION_FN$MODE_ENUM"; then
        fail "the single-producer decision/mode references a chain selector ($tok) — DC-CONS-03 must be untouched (admissibility-only)"
    fi
done

# --- (f) the loop is mode-aware AND preserves the DC-NODE-15 default ---------
if [[ -z "$LOOP_FN" ]]; then
    fail "could not isolate run_relay_loop_with_sched body"
else
    if ! grep -qE 'single_producer_forge_decision\(' <<<"$LOOP_FN"; then
        fail "run_relay_loop_with_sched does not call single_producer_forge_decision (the mode-aware gate is not wired in)"
    fi
    # The default (non-single-producer) venue must still take the DC-NODE-15 gate.
    if ! grep -qE 'dc_node_15_refusal\(|forge_followed_tip_admission\(' <<<"$LOOP_FN"; then
        fail "run_relay_loop_with_sched no longer preserves the pure DC-NODE-15 gate for the default venue"
    fi
    if ! grep -qE 'VenueRole::SingleProducer' <<<"$LOOP_FN"; then
        fail "run_relay_loop_with_sched does not gate the extend machinery behind VenueRole::SingleProducer (it must be venue-scoped, fail-closed by default)"
    fi
fi

if (( FAILED == 0 )); then
    echo "OK (single-producer extend-own-spine): ForgeMode is an explicit 4-state enum (no bool); promotion requires an explicit adopted_tip certificate (AwaitAdoptionCertificate without one — no self-admit inference); the certificate is admissibility-only (never persisted); the fence fails closed to a typed SingleProducerFenceViolation checking the venue role; no chain selector on the mode/decision path; the loop is mode-aware behind VenueRole::SingleProducer and preserves the DC-NODE-15 default (DC-NODE-18 / DC-CONS-03 untouched)"
fi
exit $FAILED
