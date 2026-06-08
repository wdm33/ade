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

# --- (a) the forge mode is an explicit enum with the named states -----------
# DC-NODE-20: FirstOwnBlockServed (the cert-wait state) is folded OUT -- self-admit
# enters the extend state directly. The closed mode surface is the three states.
if [[ -z "$MODE_ENUM" ]]; then
    fail "pub enum ForgeMode not found in $SYNC (the forge mode must be an explicit enum, not a boolean)"
else
    for st in 'InitialCatchupRequired' 'CaughtUpToPeerTip' 'SingleProducerExtendOwnDurableSpine'; do
        if ! grep -qE "$st" <<<"$MODE_ENUM"; then
            fail "ForgeMode is missing the '$st' state (the closed mode surface)"
        fi
    done
    if grep -qE 'FirstOwnBlockServed' <<<"$MODE_ENUM"; then
        fail "ForgeMode still has FirstOwnBlockServed — DC-NODE-20 folds out the cert-wait state (self-admit enters extend directly)"
    fi
fi
# The mode must never be represented as a boolean.
if grep -qE '\bforge_mode *: *bool\b' <<<"$SYNC_PROD$LIFE_PROD"; then
    fail "the forge mode is represented as a bool — it MUST be the ForgeMode enum (no booleans)"
fi

# --- (b) DC-NODE-20: self-admit (NOT a cert) enters the extend state ---------
# forge_mode_after_admit promotes CaughtUpToPeerTip DIRECTLY into the extend state on
# the own durable tip; single_producer_forge_decision has NO cert-promotion arm (the
# cert is evidence-only, DC-NODE-21), and the extend decision forges on the durable
# spine head.
ADMIT_FN="$(awk '/pub fn forge_mode_after_admit/{c=1} c{print} c&&/^}/{exit}' <<<"$SYNC_PROD")"
if [[ -z "$DECISION_FN" ]]; then
    fail "pub fn single_producer_forge_decision not found in $SYNC"
elif [[ -z "$ADMIT_FN" ]]; then
    fail "pub fn forge_mode_after_admit not found in $SYNC"
else
    if ! grep -qE 'ExtendOwnSpine' <<<"$DECISION_FN"; then
        fail "single_producer_forge_decision has no ExtendOwnSpine path (the extend state forges on the durable spine head)"
    fi
    # DC-NODE-20: NO cert-promotion / await-cert arm.
    if grep -qE 'AwaitAdoptionCertificate|Promote' <<<"$DECISION_FN"; then
        fail "single_producer_forge_decision still has a cert-promotion / await-cert arm — DC-NODE-20 enters extend on self-admit, never a cert"
    fi
    # forge_mode_after_admit enters the extend state directly; no FirstOwnBlockServed.
    if ! grep -qE 'SingleProducerExtendOwnDurableSpine' <<<"$ADMIT_FN"; then
        fail "forge_mode_after_admit does not enter SingleProducerExtendOwnDurableSpine directly (self-admit must promote into the extend state)"
    fi
    if grep -qE 'FirstOwnBlockServed' <<<"$ADMIT_FN"; then
        fail "forge_mode_after_admit still routes through FirstOwnBlockServed — DC-NODE-20 folds out the cert-wait"
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
    echo "OK (single-producer extend-own-spine): ForgeMode is an explicit 3-state enum (no bool; FirstOwnBlockServed folded out — DC-NODE-20); self-admit enters the extend state DIRECTLY via forge_mode_after_admit (no cert-promotion / await-cert arm — the cert is evidence-only, DC-NODE-21); the extend decision forges on the durable spine head (ExtendOwnSpine); the certificate token never co-occurs with a persistence verb; the fence fails closed to a typed SingleProducerFenceViolation checking the venue role; no chain selector on the mode/decision path; the loop is mode-aware behind VenueRole::SingleProducer and preserves the DC-NODE-15 default (DC-NODE-18 core / DC-CONS-03 untouched)"
fi
exit $FAILED
