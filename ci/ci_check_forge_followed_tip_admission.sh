#!/usr/bin/env bash
set -euo pipefail

# PHASE4-N-AE.A (DC-NODE-15 + DC-CONS-24): the --mode node forge is admissible
# ONLY when the durable servable tip equals the followed peer tip; otherwise it
# fails closed to a typed structured refusal. The recovered snapshot anchor is
# NEVER a forge base, and the followed-peer-tip signal is a forge-admissibility
# input only (it may PREVENT a forge, never select / replace / prefer a chain).
#
# Asserts:
#  (a) the ForgeTick `selected_tip` has NO `recovered.tip` fallback — the forge
#      base is `ChainDb::tip()` (the durable servable tip), never recovered.tip;
#  (b) the DC-NODE-15 initial-catch-up gate is invoked via the call chain
#      run_relay_loop_with_sched -> dc_node_15_refusal -> forge_followed_tip_admission
#      (BOTH links verified), and the classifier requires durable_servable_tip ==
#      followed_peer_tip on BOTH hash AND block_no. This is the PHASE4-N-AH /
#      DC-NODE-20 phase-split: INITIAL catch-up requires the followed echo (here),
#      while the POST-self-admit forge base is the local ChainDb::tip — part (a) —
#      with NO followed re-check;
#  (c) NotCaughtUp is a TYPED structured refusal carrying
#      { local_servable_tip, followed_peer_tip, reason } — not a log-string-only
#      path;
#  (d) the followed-peer-tip signal does NOT reach select_best_chain /
#      chain_selector / fork_choice (no chain-selection authority).
#
# Fails closed if a future change reintroduces the recovered.tip forge-base
# fallback, weakens the classifier to a one-field compare, buries the refusal in
# a log string, or routes the peer-tip signal into a chain selector.
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
fail() { echo "FAIL (forge-followed-tip admission): $1"; FAILED=1; }

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

# Isolate the run_relay_loop_with_sched body (signature → next top-level brace).
LOOP_FN="$(awk '
    /pub async fn run_relay_loop_with_sched/ { capture=1 }
    capture { print }
    capture && /^}/ { exit }
' <<<"$LIFE_PROD")"
if [[ -z "$LOOP_FN" ]]; then
    fail "could not isolate run_relay_loop_with_sched body (signature moved/renamed?)"
fi

# --- (a) no recovered.tip forge-base fallback at the ForgeTick selected_tip ---
# The OLD bug: `let selected_tip = match ChainDb::tip(...) { Some(t) => .., None
# => act.recovered.tip.clone() }`. The selected_tip must come from ChainDb::tip
# with no recovered.tip fallback. (act.recovered.tip.is_none() as the
# from-genesis cold-start DISCRIMINATOR is allowed — that is not a forge base.)
SELECTED_LINE="$(grep -E 'let selected_tip' <<<"$LOOP_FN" | head -1)"
if [[ -z "$SELECTED_LINE" ]]; then
    fail "run_relay_loop_with_sched no longer binds a selected_tip (unexpected)"
elif ! grep -qE 'ChainDb::tip\(' <<<"$SELECTED_LINE"; then
    fail "the ForgeTick selected_tip is not derived from ChainDb::tip( — the durable servable tip must be the forge base"
elif grep -qE 'recovered' <<<"$SELECTED_LINE"; then
    fail "the ForgeTick selected_tip line references 'recovered' — the recovered.tip forge-base fallback must be removed (recovered.tip is NEVER a forge base)"
fi
# The specific old fallback expression must not appear anywhere in the loop body.
if grep -qE 'recovered\.tip\.clone\(\)' <<<"$LOOP_FN"; then
    fail "run_relay_loop_with_sched still clones recovered.tip (the removed forge-base fallback)"
fi

# DC-NODE-15 initial-catch-up gate — the call chain, BOTH links verified.
# PHASE4-N-AH / DC-NODE-20 moved the followed-tip admissibility check out of the
# loop body into the named `dc_node_15_refusal` helper. This encodes the PHASE-SPLIT:
# the INITIAL catch-up still requires durable == followed (the helper -> classifier),
# while the POST-self-admit forge base is the local ChainDb::tip (part (a)) with NO
# followed re-check. Verify BOTH links of the chain — not "the function exists
# somewhere in the repo":
#   run_relay_loop_with_sched -> dc_node_15_refusal -> forge_followed_tip_admission
#
# (b1) the dc_node_15_refusal helper invokes the DC-NODE-15 classifier.
REFUSAL_FN="$(awk '
    /fn dc_node_15_refusal/ { capture=1 }
    capture { print }
    capture && /^}/ { exit }
' <<<"$LIFE_PROD")"
if [[ -z "$REFUSAL_FN" ]]; then
    fail "dc_node_15_refusal helper not found in $LIFECYCLE (the DC-NODE-15 initial-catch-up refusal path moved/renamed?)"
elif ! grep -qE 'forge_followed_tip_admission\(' <<<"$REFUSAL_FN"; then
    fail "dc_node_15_refusal does not call forge_followed_tip_admission (the DC-NODE-15 caught-up classifier)"
fi
# (b2) the loop invokes the dc_node_15_refusal helper, BEFORE the single fenced forge.
ADM_LINE="$(grep -nE 'dc_node_15_refusal\(' <<<"$LOOP_FN" | head -1 | cut -d: -f1)"
FORGE_LINE="$(grep -nE 'forge_one_from_recovered\(' <<<"$LOOP_FN" | head -1 | cut -d: -f1)"
if [[ -z "$ADM_LINE" ]]; then
    fail "run_relay_loop_with_sched does not call the dc_node_15_refusal helper (the initial catch-up / refusal path)"
elif [[ -z "$FORGE_LINE" ]]; then
    fail "run_relay_loop_with_sched no longer calls forge_one_from_recovered (unexpected)"
elif (( ADM_LINE >= FORGE_LINE )); then
    fail "dc_node_15_refusal (line $ADM_LINE) must precede forge_one_from_recovered (line $FORGE_LINE) — initial-catch-up admissibility is decided BEFORE the forge"
fi

# --- (b) the classifier compares hash AND block_no -------------------------
ADM_FN="$(awk '
    /pub fn forge_followed_tip_admission/ { capture=1 }
    capture { print }
    capture && /^}/ { exit }
' <<<"$SYNC_PROD")"
if [[ -z "$ADM_FN" ]]; then
    fail "could not isolate forge_followed_tip_admission in $SYNC"
else
    if ! grep -qE '\.hash *== *' <<<"$ADM_FN"; then
        fail "forge_followed_tip_admission does not compare .hash (caught-up requires hash equality)"
    fi
    if ! grep -qE '\.block_no *== *' <<<"$ADM_FN"; then
        fail "forge_followed_tip_admission does not compare .block_no (caught-up requires block_no equality — never hash-only)"
    fi
fi

# --- (c) NotCaughtUp is a typed refusal carrying the tips + reason ----------
# The ForgeRefused::NotCaughtUp variant must carry the three structured fields.
REFUSED_VARIANT="$(awk '
    /enum ForgeRefused/ { capture=1 }
    capture { print }
    capture && /^}/ { exit }
' <<<"$SYNC_PROD")"
if [[ -z "$REFUSED_VARIANT" ]]; then
    fail "enum ForgeRefused not found in $SYNC (the typed refusal surface)"
else
    for fld in 'local_servable_tip' 'followed_peer_tip' 'reason'; do
        if ! grep -qE "$fld" <<<"$REFUSED_VARIANT"; then
            fail "ForgeRefused::NotCaughtUp does not carry the structured field '$fld' (no log-string-only path)"
        fi
    done
fi
# The typed refusal must be CONSTRUCTED (carrying the tips) in the dc_node_15_refusal
# helper, and RECORDED into the typed last_forge_refused surface BY THE LOOP — never a
# log-only line. (PHASE4-N-AH / DC-NODE-20 moved the construction into the helper, which
# returns Option<ForgeRefused>; the loop records the helper's returned refusal.)
if ! grep -qE 'ForgeRefused::NotCaughtUp' <<<"$REFUSAL_FN"; then
    fail "dc_node_15_refusal does not construct a typed ForgeRefused::NotCaughtUp on the not-caught-up path"
fi
if ! grep -qE 'last_forge_refused *= *Some' <<<"$LOOP_FN"; then
    fail "the not-caught-up refusal is not recorded into the typed last_forge_refused surface by the loop (it must not be a log-only path)"
fi

# --- (d) the peer-tip signal never reaches a chain selector -----------------
# fork-choice / chain selection is NOT on this path (slice §6). Assert neither
# the gate/source module nor the loop wiring references a chain selector.
for tok in 'select_best_chain' 'chain_selector' 'fork_choice'; do
    if grep -qE "$tok" <<<"$SYNC_PROD"; then
        fail "$SYNC references a chain selector ($tok) — the followed-peer-tip signal is admissibility-only, never a chain selector"
    fi
    if grep -qE "$tok" <<<"$LOOP_FN"; then
        fail "run_relay_loop_with_sched references a chain selector ($tok) — the followed-peer-tip signal may only PREVENT a forge"
    fi
done

if (( FAILED == 0 )); then
    echo "OK (forge-followed-tip admission): selected_tip is the durable ChainDb::tip (no recovered.tip fallback — DC-NODE-20 post-self-admit local-tip); the DC-NODE-15 initial-catch-up gate is invoked via run_relay_loop_with_sched -> dc_node_15_refusal -> forge_followed_tip_admission BEFORE the forge, gated on durable==followed (hash AND block_no); NotCaughtUp is a typed refusal carrying { local_servable_tip, followed_peer_tip, reason }; no chain selector on the peer-tip path (DC-NODE-15 / DC-NODE-20 phase-split / DC-CONS-24)"
fi
exit $FAILED
