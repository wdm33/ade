#!/usr/bin/env bash
set -uo pipefail

# LIVE-WIRE-LIVENESS -- wire/pump cooperative-liveness enforcement.
#
# Gates DC-PUMP-05..10 (and supports DC-PUMP-03's strengthening). These are the
# properties whose REGRESSION reproduced a real live run-ender on preview
# 2026-08-01: the peer shut Ade down with
# `ExceededTimeLimit (KeepAlive) ClientHasAgency` because the pump was parked on
# a bounded `events_out.send` OUTSIDE its `select!`, and the run then ended on
# `exit=Eof` because the per-peer task was one-shot.
#
# This gate is a STATIC assertion over the live tree plus a named-test census.
# It deliberately asserts STRUCTURE, not just that tests exist: a structural
# regression (making a handler async again, dropping the cooperative emit,
# widening the deferral bound to unbounded, forgetting a new error variant in
# the reconnect policy) fails here even if every test still compiles.
#
# NB (here-string discipline): never `echo "$BIGVAR" | grep -q` under pipefail --
# grep exits on first match, echo takes SIGPIPE, and the gate flakes on large
# files. Grep files directly, or use `grep <<< "$VAR"`.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"
FAILED=0
fail() { echo "FAIL: $1"; FAILED=1; }
ok()   { echo "  ok: $1"; }

PUMP=crates/ade_runtime/src/admission/wire_pump.rs
LIFE=crates/ade_node/src/node_lifecycle.rs

for f in "$PUMP" "$LIFE"; do
    [ -f "$f" ] || { fail "missing $f"; echo "RESULT: FAIL"; exit 1; }
done

echo "== DC-PUMP-05: cooperative keep-alive liveness =="

# The cooperative emit must exist: the WAIT for downstream capacity is itself a
# select! that keeps the keep-alive lane live in BOTH directions.
grep -qE 'async fn emit_cooperative' "$PUMP" \
    || fail "emit_cooperative missing -- the cooperative wait is the fix"
ok "emit_cooperative present"

# It must reserve capacity (cancel-safe) rather than await a bounded send.
grep -qE 'events_out\.reserve\(\)' "$PUMP" \
    || fail "emit_cooperative must await events_out.reserve(), not a blocking send"
ok "capacity acquired via reserve()"

# It must service BOTH directions while parked: fire the cadence AND consume the
# echoed response. Handling only the timer wedges in ClientAwaiting after one
# ping (~117s tolerance) instead of unbounded.
KA_BODY="$(awk '/async fn emit_cooperative/,/^}/' "$PUMP")"
grep -qE 'keep_alive_cadence_send' <<< "$KA_BODY" \
    || fail "emit_cooperative must fire the keep-alive cadence while parked"
grep -qE 'handle_keep_alive' <<< "$KA_BODY" \
    || fail "emit_cooperative must CONSUME the echoed response while parked \
(timer-only wedges in ClientAwaiting)"
ok "cadence + response both serviced while parked"

# Emission must be hoisted OUT of the frame handlers: if they are async they can
# await a bounded send again, which is exactly the regression.
grep -qE '^async fn handle_chain_sync' "$PUMP" \
    && fail "handle_chain_sync must NOT be async (it could await a bounded send)"
grep -qE '^async fn handle_block_fetch' "$PUMP" \
    && fail "handle_block_fetch must NOT be async (it could await a bounded send)"
grep -qE '^fn handle_chain_sync' "$PUMP" \
    || fail "handle_chain_sync missing / not synchronous"
grep -qE '^fn handle_block_fetch' "$PUMP" \
    || fail "handle_block_fetch missing / not synchronous"
ok "frame handlers are synchronous (cannot park on a bounded send)"

# The cadence must stay STRICTLY under the peer's observed ~97s limit.
CADENCE="$(grep -oE 'KEEP_ALIVE_CADENCE: Duration = Duration::from_secs\([0-9]+\)' "$PUMP" \
           | grep -oE '[0-9]+' | tail -1)"
if [ -z "${CADENCE:-}" ]; then
    fail "KEEP_ALIVE_CADENCE not found"
elif [ "$CADENCE" -ge 97 ]; then
    fail "KEEP_ALIVE_CADENCE=${CADENCE}s is not strictly under the ~97s peer timeout"
else
    ok "keep-alive cadence ${CADENCE}s < 97s peer timeout"
fi

echo "== DC-PUMP-06: ordered pump progression under backpressure =="

# One ordered FIFO of pending frames, dispatched one per iteration, so deferred
# frames can never interleave ahead of a chunk's remaining frames.
grep -qE 'pending_frames' "$PUMP" \
    || fail "pending_frames FIFO missing -- ordering guarantee lost"
grep -qE 'pending_frames\.push_back' "$PUMP" \
    || fail "frames must be ENQUEUED (push_back), not dispatched inline"
grep -qE 'pending_frames\.pop_front' "$PUMP" \
    || fail "frames must be dispatched from the FRONT of the FIFO"
ok "single ordered pending-frame FIFO"

echo "== DC-PUMP-07: bounded deferral, fail closed =="

DEFER="$(grep -oE 'MAX_DEFERRED_PEER_FRAMES: usize = [0-9_]+' "$PUMP" \
         | grep -oE '[0-9_]+$' | tr -d '_')"
if [ -z "${DEFER:-}" ]; then
    fail "MAX_DEFERRED_PEER_FRAMES missing -- deferral would be unbounded"
elif [ "$DEFER" -le 0 ]; then
    fail "MAX_DEFERRED_PEER_FRAMES must be a positive fixed bound"
else
    ok "deferral bounded at ${DEFER} frames"
fi
grep -qE 'DeferredFrameOverflow' "$PUMP" \
    || fail "overflow must fail closed with a typed DeferredFrameOverflow"
ok "overflow is a typed fail-closed halt"

echo "== DC-PUMP-08: reconnect policy exhaustiveness =="

grep -qE 'fn should_reconnect_after' "$LIFE" \
    || fail "should_reconnect_after missing -- reconnect policy must be one named authority"
POLICY="$(awk '/fn should_reconnect_after/,/^}/' "$LIFE")"

# TEETH: every AdmissionWirePumpError variant must be named in the policy. Add a
# variant without classifying it and this gate fails.
VARIANTS="$(awk '/pub enum AdmissionWirePumpError/,/^}/' "$PUMP" \
            | grep -oE '^    [A-Z][A-Za-z]+' | tr -d ' ')"
[ -n "$VARIANTS" ] || fail "could not extract AdmissionWirePumpError variants"
MISSING=""
while read -r v; do
    [ -z "$v" ] && continue
    grep -qE "\b${v}\b" <<< "$POLICY" || MISSING="$MISSING $v"
done <<< "$VARIANTS"
if [ -n "$MISSING" ]; then
    fail "should_reconnect_after does not classify:$MISSING (policy must be TOTAL)"
else
    ok "policy classifies every AdmissionWirePumpError variant"
fi

# Transport-only: the consumer-gone case must NOT reconnect.
grep -qE 'EventsChannelDropped' <<< "$POLICY" \
    || fail "policy must classify EventsChannelDropped (consumer gone => no reconnect)"
ok "EventsChannelDropped classified"

echo "== DC-PUMP-09: no bootstrap spin =="

# A first dial that fails must end the feed, never retry forever at boot.
SUP="$(awk '/fn spawn_live_wire_pump_source/,/^}/' "$LIFE")"
grep -qE 'established' <<< "$SUP" \
    || fail "supervisor must gate reconnect on an ESTABLISHED session (no boot spin)"
ok "reconnect gated on an established session"

echo "== DC-PUMP-10: deterministic bounded backoff =="

grep -qE 'RECONNECT_BACKOFF_SECS: &\[u64\]' "$LIFE" \
    || fail "backoff must be a fixed const schedule"
grep -qE 'fn reconnect_backoff_secs' "$LIFE" \
    || fail "reconnect_backoff_secs accessor missing"
# Determinism: no randomness anywhere in the reconnect path.
if grep -nE 'rand::|thread_rng|random\(\)' <<< "$SUP"; then
    fail "reconnect path must be deterministic -- no randomness"
else
    ok "backoff is a deterministic fixed schedule"
fi

echo "== named-test census (DC-PUMP-05..10 proofs) =="

check_test() { # $1 = test fn name, $2 = file
    grep -qE "fn ${1}\b" "$2" || fail "missing proof test: ${1} (expected in $2)"
}
check_test ce_wl_1_keep_alive_survives_unbounded_downstream_stall "$PUMP"
check_test ce_wl_2_backpressure_preserves_event_order              "$PUMP"
check_test ce_wl_3_deferred_frames_are_bounded_and_fail_closed     "$PUMP"
check_test reconnect_policy_is_transport_only                      "$LIFE"
check_test first_dial_failure_still_ends_the_feed_no_boot_spin     "$LIFE"
check_test reconnect_backoff_is_deterministic_monotone_and_capped  "$LIFE"
check_test spawn_live_wire_pump_source_with_no_usable_peer_yields_ended_feed "$LIFE"
[ "$FAILED" -eq 0 ] && ok "all 7 named proof tests present"

if [ "$FAILED" -ne 0 ]; then
    echo "RESULT: FAIL (wire-liveness enforcement regressed)"
    exit 1
fi
echo "RESULT: PASS (DC-PUMP-05..10 structurally enforced + proofs present)"
