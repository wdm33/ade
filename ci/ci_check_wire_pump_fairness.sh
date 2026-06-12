#!/usr/bin/env bash
# ci_check_wire_pump_fairness.sh -- PHASE4-N-AO S8 (DC-PUMP-04).
#
# Multi-peer wire-pump fairness. Each connected peer gets its OWN bounded lane,
# drained by a fair round-robin merge over a DETERMINISTIC order derived from the
# configured --peer list -- so a continuously-producing peer can no longer starve
# the others off the participant receive path. RED scheduling discipline ONLY: the
# merge order may affect delivery opportunity but never decides fork-choice
# (select_best_chain stays arrival-order independent, CN-CONS-01).
set -euo pipefail

FM="crates/ade_node/src/fair_merge.rs"
NL="crates/ade_node/src/node_lifecycle.rs"
fail() { echo "FAIL (ci_check_wire_pump_fairness): $1" >&2; exit 1; }
[ -f "$FM" ] || fail "module $FM missing"
[ -f "$NL" ] || fail "module $NL missing"

# CODE = fair_merge.rs minus standalone comment lines (doc comments legitimately
# name HashMap/wall-clock/fork-choice when describing the discipline; check CODE).
FMCODE="$(grep -vE '^[[:space:]]*//' "$FM")"

# (A) The source wires PER-PEER lanes + a fair merge -- NOT one shared fan-in.
REGION="$(awk '/^fn spawn_live_wire_pump_source/{f=1} f&&/^(fn|async fn|pub fn|pub async fn) /&&!/spawn_live_wire_pump_source/{exit} f{print}' "$NL")"
[ -n "$REGION" ] || fail "could not locate spawn_live_wire_pump_source"
RCODE="$(echo "$REGION" | grep -vE '^[[:space:]]*//')"
echo "$RCODE" | grep -Eq 'mpsc::channel::<AdmissionPeerEvent>\(PER_PEER_LANE_CAP\)' \
  || fail "each peer must get its OWN bounded lane (mpsc::channel(PER_PEER_LANE_CAP))"
echo "$RCODE" | grep -Eq 'fair_merge\(lanes' \
  || fail "the per-peer lanes must be drained by fair_merge(lanes, ..)"
# The pre-S8 shared fan-in (all pumps cloning ONE events_tx) must be gone.
if echo "$RCODE" | grep -Eq 'events_tx\.clone\(\)'; then
  fail "pre-S8 shared-channel fan-in (events_tx.clone()) must NOT remain -- it lets a hot peer monopolise the feed"
fi

# (B) Fair round-robin: rotating cursor over the lane Vec + retire a closed lane IN
# PLACE (no reorder of the remaining peers).
echo "$FMCODE" | grep -Eq '\*start = \(i \+ 1\) % n' \
  || fail "the merge must rotate the start cursor (round-robin fairness)"
echo "$FMCODE" | grep -Eq 'lanes\[i\] = None' \
  || fail "a closed lane must be retired IN PLACE (lanes[i] = None), not removed/reordered"

# (C) Deterministic peer order -- an explicit Vec, never HashMap/HashSet iteration.
if echo "$FMCODE" | grep -Eq 'HashMap|HashSet'; then
  fail "peer/lane order must NOT come from HashMap/HashSet iteration (nondeterministic)"
fi

# (D) No wall-clock / rand may influence the merge order (RED determinism).
if echo "$FMCODE" | grep -Eq 'Instant::now|SystemTime|wall|rand|thread_rng|Date::now'; then
  fail "the merge order must not depend on wall-clock or rand"
fi

# (E) No fork-choice-relevant event dropped because another peer is hot: forward via
# a backpressuring out.send (await), never a lossy try_send-and-drop in the merge.
echo "$FMCODE" | grep -Eq 'out\.send\(' \
  || fail "the merge must forward via a backpressuring out.send (per-peer backpressure, not drop)"
if echo "$FMCODE" | grep -Eq 'out\.try_send'; then
  fail "the merge must NOT lossy-drop events (no out.try_send); a hot peer self-backpressures its own lane"
fi

# (F) RED-only: the fairness layer must touch NO selector / fork-choice / BLUE authority.
for forbidden in 'select_best_chain' 'build_candidate_fragment' 'walk_to_durable_lca' 'commit_rollback' 'pump_block'; do
  if echo "$FMCODE" | grep -Eq "$forbidden"; then
    fail "the RED fairness layer must NOT call BLUE/selector authority ('$forbidden')"
  fi
done

echo "OK: wire-pump multi-peer fairness -- per-peer bounded lanes + deterministic round-robin merge, closed-lane retire-in-place, no shared fan-in, RED-only (DC-PUMP-04)"
