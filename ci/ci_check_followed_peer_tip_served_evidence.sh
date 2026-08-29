#!/usr/bin/env bash
set -euo pipefail

# SLICE B12 (DC-NODE-47) -- the followed-peer-tip signal reports the STRONGEST evidence
# that the followed peer possesses a block: the tip it ADVERTISED, or a tip it SERVED
# and Ade DURABLY ADMITTED, whichever is higher by block_no.
#
# DC-NODE-15's predicate is NOT this gate's subject and is NOT modified by the slice --
# ci_check_forge_followed_tip_admission.sh owns it and is REUSED rather than restated
# (a second copy of an invariant is a second thing to drift).
#
# Asserts:
#  (a) FollowedPeerTipSignal carries a `served` half beside `latest`, and both are
#      separately readable -- the two evidences must not be collapsed into one field;
#  (b) tip() combines them with a STRICTLY-greater block_no comparison. `>=` would hand
#      a same-height fork to the served side; the tie must fall to the advertisement so
#      the gate refuses with TipMismatch intact;
#  (c) the served fact is written ONLY at the successful durable admit -- the call sits
#      after `admitted_this_pass += 1`, which exists only inside `if let Some(t) = tip`.
#      Receipt is not possession evidence Ade may act on;
#  (d) it is built from the pump's own validated tip (t.slot / t.hash / t.block_no), NEVER
#      from ChainDbServedSource::tip(), which re-reads AND re-decodes -- per admitted block
#      that is exactly the fixed per-block cost B6 removed;
#  (e) any rollback clears it;
#  (f) the CONVERGENCE-EVIDENCE sites consume advertised(), not tip(). They feed
#      derive() -> AgreementVerdict; the combined signal folds in the block just admitted
#      and would make every verdict read `Agreed` by construction;
#  (g) neither half reaches a chain selector / next_block / pump_block;
#  (h) the slice's tests exist and are non-vacuous (>= 7 #[test] fns).
#
# Repo-root-relative. Mirrors the other ci_check_*.sh gates.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

SYNC="crates/ade_node/src/node_sync.rs"
LIFECYCLE="crates/ade_node/src/node_lifecycle.rs"
PUMP="crates/ade_runtime/src/forward_sync/pump.rs"
TESTS="crates/ade_node/tests/b12_served_or_advertised_peer_tip.rs"

for f in "$SYNC" "$LIFECYCLE" "$PUMP" "$TESTS"; do
    if [[ ! -f "$f" ]]; then
        echo "FAIL (b12 served evidence): $f not found"
        exit 1
    fi
done

FAILED=0
fail() { echo "FAIL (b12 served evidence): $1"; FAILED=1; }

# Production body only: drop the #[cfg(test)] module and strip line comments, so
# commentary naming a token cannot satisfy or trip a grep.
prod_body() {
    # Truncate at the TRAILING #[cfg(test)] MODULE only. Anchoring on the first
    # bare '#[cfg(test)]' silently truncated this gate from ECA-5 (26565bec)
    # onward, when an inline '#[cfg(test)] async fn run_node_sync_no_eview' shim
    # landed above every symbol the gate inspects -- so it "passed" by seeing
    # nothing and then failed closed on empty bodies. A gate that cannot see the
    # code enforces nothing.
    awk '
        /^#\[cfg\(test\)\]$/ { pend = $0; holding = 1; next }
        holding && /^#\[allow\(/ { pend = pend "\n" $0; next }
        holding && /^mod / { exit }
        holding { print pend; holding = 0 }
        { print }
    ' "$1" | sed -E 's://.*::'
}

SYNC_PROD="$(prod_body "$SYNC")"
LIFE_PROD="$(prod_body "$LIFECYCLE")"
PUMP_PROD="$(prod_body "$PUMP")"
if [[ -z "$SYNC_PROD" || -z "$LIFE_PROD" || -z "$PUMP_PROD" ]]; then
    echo "FAIL (b12 served evidence): could not isolate production bodies"
    exit 1
fi

isolate() { # isolate <signature-regex> <body>
    awk -v pat="$1" '
        $0 ~ pat { capture=1 }
        capture { print }
        capture && /^}/ { exit }
    ' <<<"$2"
}

# --- (a) both halves exist and are separately readable ----------------------
SIGNAL_STRUCT="$(awk '
    /pub struct FollowedPeerTipSignal/ { capture=1 }
    capture { print }
    capture && /^}/ { exit }
' <<<"$SYNC_PROD")"
if [[ -z "$SIGNAL_STRUCT" ]]; then
    fail "FollowedPeerTipSignal struct not found in $SYNC (moved/renamed?)"
else
    grep -qE '^\s*latest: *Option<TipPoint>' <<<"$SIGNAL_STRUCT" \
        || fail "FollowedPeerTipSignal lost its advertised half ('latest')"
    grep -qE '^\s*served: *Option<TipPoint>' <<<"$SIGNAL_STRUCT" \
        || fail "FollowedPeerTipSignal has no 'served' half -- DC-NODE-47 requires service to be recorded alongside advertisement, not instead of it"
fi
for acc in 'pub fn advertised' 'pub fn served' 'pub fn tip'; do
    grep -qE "$acc\(&self\)" <<<"$SYNC_PROD" \
        || fail "FollowedPeerTipSignal is missing the '${acc##* }()' accessor -- the two evidences must stay separately readable (the evidence path reads advertised(), the gate reads tip())"
done

# --- (b) the combination rule is STRICTLY greater ---------------------------
TIP_FN="$(isolate 'pub fn tip\(&self\)' "$SYNC_PROD")"
if [[ -z "$TIP_FN" ]]; then
    fail "could not isolate FollowedPeerTipSignal::tip"
else
    if ! grep -qE 'served\.block_no *> *advertised\.block_no' <<<"$TIP_FN"; then
        fail "tip() does not compare served.block_no > advertised.block_no -- the combination rule must be explicit and by block_no"
    fi
    if grep -qE 'block_no *>= *' <<<"$TIP_FN"; then
        fail "tip() uses >= -- a tie at the same block_no with differing hashes is a FORK the AO owns; it must fall to the ADVERTISEMENT so the gate refuses with TipMismatch, never to the served side"
    fi
    for sloppy in 'unwrap_or' 'unwrap_or_default'; do
        grep -qE "$sloppy" <<<"$TIP_FN" \
            && fail "tip() uses $sloppy -- absent evidence is None (=> NoFollowedPeerTip), never a fabricated tip"
    done
fi

# --- (c) written ONLY at the successful durable admit -----------------------
SYNC_FN="$(isolate 'pub async fn run_node_sync' "$SYNC_PROD")"
if [[ -z "$SYNC_FN" ]]; then
    fail "could not isolate run_node_sync"
else
    RECORD_COUNT="$(grep -cE 'record_served_tip\(' <<<"$SYNC_FN" || true)"
    if [[ "$RECORD_COUNT" != "1" ]]; then
        fail "run_node_sync calls record_served_tip $RECORD_COUNT times -- exactly ONE call site is allowed, at the successful admit"
    fi
    ADMIT_LINE="$(grep -nE 'admitted_this_pass \+= 1' <<<"$SYNC_FN" | head -1 | cut -d: -f1)"
    RECORD_LINE="$(grep -nE 'record_served_tip\(' <<<"$SYNC_FN" | head -1 | cut -d: -f1)"
    if [[ -z "$ADMIT_LINE" ]]; then
        fail "run_node_sync no longer marks the successful admit (admitted_this_pass) -- the served-fact anchor is gone"
    elif [[ -z "$RECORD_LINE" ]]; then
        fail "run_node_sync never records the served fact"
    elif (( RECORD_LINE <= ADMIT_LINE )); then
        fail "record_served_tip (line $RECORD_LINE) is not inside the successful-admit block (admitted_this_pass at $ADMIT_LINE) -- service must be recorded AFTER the durable admit, never on receipt"
    fi
fi

# --- (d) built from the pump's validated tip, not a re-read + re-decode -----
if [[ -n "$SYNC_FN" ]]; then
    RECORD_ARG="$(awk '/record_served_tip\(/{c=1} c{print} c&&/\}\);/{exit}' <<<"$SYNC_FN")"
    for field in 't.slot' 't.hash' 't.block_no'; do
        grep -qF "$field" <<<"$RECORD_ARG" \
            || fail "the served fact does not read $field from the pump tip -- it must be built from the decode the admit ALREADY did"
    done
    if grep -qE 'ChainDbServedSource' <<<"$SYNC_FN"; then
        fail "run_node_sync references ChainDbServedSource -- its tip() re-reads AND re-decodes a block; calling it per admitted block reintroduces exactly the fixed per-block cost B6 removed"
    fi
fi
grep -qE '^\s*pub block_no: u64,' <<<"$PUMP_PROD" \
    || fail "PumpTip carries no block_no -- without it the served fact cannot be built at the admit boundary without a second decode"

# --- (e) any rollback clears it ---------------------------------------------
if [[ -n "$SYNC_FN" ]]; then
    grep -qE 'clear_served_tip\(\)' <<<"$SYNC_FN" \
        || fail "run_node_sync never clears the served fact -- a rollback must invalidate it, or the signal keeps naming a block that may no longer be on the selected chain"
    ROLLBACK_BLOCK="$(awk '/resolve_and_apply_peer_rollback\(/{c=1} c{print} c&&/continue;/{exit}' <<<"$SYNC_FN")"
    grep -qE 'clear_served_tip\(\)' <<<"$ROLLBACK_BLOCK" \
        || fail "the served fact is not cleared on the rollback path (resolve_and_apply_peer_rollback -> continue)"
fi

# --- (f) the evidence sites read advertised(), NOT the combined signal ------
# emit_admit_and_verdict feeds derive() -> AgreementVerdict. Fed the combined signal it
# would read 'Agreed' on every admit -- an evidence stream that always agrees, silently.
EVIDENCE_SITES="$(grep -nE 'let peer_tip = source\.followed_peer_tip_signal\(\)' <<<"$LIFE_PROD" || true)"
if [[ -z "$EVIDENCE_SITES" ]]; then
    fail "no convergence-evidence peer_tip binding found in $LIFECYCLE (moved/renamed? the advertised()-vs-tip() split must stay checkable)"
else
    SITE_COUNT="$(wc -l <<<"$EVIDENCE_SITES")"
    if (( SITE_COUNT < 2 )); then
        fail "expected both convergence-evidence peer_tip sites, found $SITE_COUNT"
    fi
    if grep -qE 'let peer_tip = source\.followed_peer_tip_signal\(\)\.tip\(\)' <<<"$LIFE_PROD"; then
        fail "a convergence-evidence site binds peer_tip from tip() -- it must read advertised(). An AgreementVerdict asks what the peer SAID; the combined signal makes every admit read Agreed by construction"
    fi
    while IFS= read -r line; do
        grep -qE 'advertised\(\)' <<<"$line" \
            || fail "a convergence-evidence peer_tip binding does not read advertised(): ${line}"
    done <<<"$EVIDENCE_SITES"
fi

# --- (g) neither half is a sync / chain-selection authority -----------------
for fn in 'fn observe_served' 'fn clear_served' 'pub fn tip\(&self\)'; do
    BODY="$(isolate "$fn" "$SYNC_PROD")"
    for tok in 'select_best_chain' 'chain_selector' 'fork_choice' 'next_block' 'pump_block'; do
        grep -qE "$tok" <<<"$BODY" \
            && fail "${fn#fn } touches $tok -- the served fact is a forge-admissibility input only; it may PREVENT a forge, never drive sync or chain selection"
    done
done

# --- (h) the slice's tests exist and cannot pass vacuously ------------------
TEST_COUNT="$(grep -cE '^#\[test\]' "$TESTS" || true)"
if (( TEST_COUNT < 7 )); then
    fail "$TESTS declares $TEST_COUNT #[test] fns, expected >= 7 (CE-B12-1/2/3/4/5/8/9) -- a shrinking test file must not silently weaken this gate"
fi
for ce in \
    'the_census_frontier_tuple_resolves_caught_up_once_service_is_evidence' \
    'a_catch_up_gap_keeps_the_advertisement_dominant_and_the_gate_refusing' \
    'a_self_forged_tip_is_not_served_evidence_and_the_gate_refuses' \
    'a_rollback_clears_the_served_fact_and_the_signal_falls_back_to_the_advertisement' \
    'a_tie_at_the_same_height_resolves_to_the_advertisement_and_refuses' \
    'all_three_venue_routes_refuse_on_the_pre_fix_census_tuple'
do
    grep -qF "fn $ce" "$TESTS" \
        || fail "the named CE test is missing: $ce"
done
# The catch-up control is the slice's non-vacuity. It must assert a REFUSAL.
CATCHUP_FN="$(awk '/fn a_catch_up_gap_keeps_the_advertisement_dominant_and_the_gate_refusing/{c=1} c{print} c&&/^}/{exit}' "$TESTS")"
grep -qE 'NotCaughtUp' <<<"$CATCHUP_FN" \
    || fail "the catch-up control does not assert NotCaughtUp -- it is the whole non-vacuity of the slice: a node behind the peer must stay inadmissible"

if (( FAILED == 0 )); then
    echo "OK (b12 served evidence): FollowedPeerTipSignal carries advertisement AND service separately; tip() prefers service only on a STRICTLY higher block_no (ties fall to the advertisement, TipMismatch preserved); the served fact is written once, after the durable admit, from the pump's own validated tip (no re-read, no second decode) and cleared on every rollback; the convergence-evidence sites read advertised() so an AgreementVerdict cannot read Agreed by construction; no chain-selection or sync reach; $TEST_COUNT CE tests present (DC-NODE-47, DC-NODE-15 strengthened)"
fi
exit $FAILED
