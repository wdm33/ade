#!/usr/bin/env bash
set -uo pipefail

# LIVE-REFOLD-THRASH RF-1 -- a bounded settled rewind must survive the recovery pass after rollback.
#
# Gates DC-EPOCH-35. The defect: `reset_to_settled` applies a correct BOUNDED rewind and clears the
# anchor (DC-EPOCH-29). The next recovery pass then reconciles an ABSENT anchor to
# `ResetAndRefold { AnchorAbsent }` and calls `reset_to_bootstrap`, discarding the rewind AND
# deleting the settled triple -- so every later rollback is unbounded too. Measured live growing
# 153,565 -> 171,449 slots per refold until the refold outgrew the inter-rollback interval and the
# node stopped holding tip at all (a forge blocker: leadership can never be promoted at a boundary).
#
# ORDER IS THE SAFETY PROPERTY, and it is why this gate exists at all: removing the post-commit call
# entirely COMPILES CLEAN and every unit test still passes, because the tests exercise the function
# directly. Only a structural check catches an unwired or mis-ordered call.
#
# NB (here-string discipline): never `echo "$BIGVAR" | grep -q` under pipefail -- grep exits on first
# match, echo takes SIGPIPE, and the gate flakes on large files. Grep files directly, or `<<<`.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"
FAILED=0
fail() { echo "FAIL: $1"; FAILED=1; }
ok()   { echo "  ok: $1"; }

LIFE=crates/ade_node/src/node_lifecycle.rs
STORE=crates/ade_runtime/src/chaindb/epoch_accumulator_store.rs
for f in "$LIFE" "$STORE"; do
    [ -f "$f" ] || { fail "missing $f"; echo "RESULT: FAIL"; exit 1; }
done

echo "== DC-EPOCH-35: the anchor is never carried ACROSS the rollback commit =="

RB="$(awk '/^pub\(crate\) fn resolve_and_apply_peer_rollback/,/^}/' "$LIFE")"
[ -n "$RB" ] || fail "resolve_and_apply_peer_rollback missing"

line_of() { grep -nE "$1" <<< "$RB" | head -1 | cut -d: -f1; }
PRECLEAR="$(line_of 'accumulator_admit_and_clear_for_rollback\(')"
APPLY="$(line_of 'let applied = apply_chain_event\(')"
RECERT="$(line_of 'accumulator_recertify_settled_after_rollback\(')"

[ -n "$PRECLEAR" ] || fail "the S5 PRE-CLEAR must still run in the rollback path"
[ -n "$APPLY" ]    || fail "could not locate apply_chain_event in the rollback path"
[ -n "$RECERT" ]   || fail "the post-commit re-certification is NOT WIRED -- removing it compiles clean and every unit test still passes"

if [ -n "$PRECLEAR" ] && [ -n "$APPLY" ] && [ -n "$RECERT" ]; then
    [ "$PRECLEAR" -lt "$APPLY" ] \
        || fail "the anchor pre-clear MUST run BEFORE the ChainDb rollback commits (crash-window safety)"
    [ "$RECERT" -gt "$APPLY" ] \
        || fail "re-certification MUST run AFTER the rollback commits -- earlier would carry an anchor across the window"
    ok "pre-clear -> rollback commit -> re-certify, in that order"
fi

# DC-EPOCH-29 must not be weakened to make this easier: the rewind itself still de-certifies.
SETTLED="$(awk '/pub fn reset_to_settled/,/^    }/' "$STORE")"
grep -qE 'remove\(LAST_ADVANCED_POINT_KEY\)' <<< "$SETTLED" \
    || fail "reset_to_settled must STILL clear the anchor (DC-EPOCH-29) -- re-certification happens later, it does not replace the pre-clear"
ok "the rewind still de-certifies (DC-EPOCH-29 intact)"

echo "== DC-EPOCH-35: re-certification is proof-carrying against the POST-rollback chain =="

REC="$(awk '/^fn accumulator_recertify_settled_after_rollback/,/^}/' "$LIFE")"
[ -n "$REC" ] || fail "accumulator_recertify_settled_after_rollback missing"

grep -qE 'header_hash' <<< "$REC" \
    || fail "must re-prove the settled point is CANONICAL at its slot on the new chain (a hash pins its whole ancestry)"
grep -qE 'block_no\.0\.saturating_add\(policy\.security_param\.0\)' <<< "$REC" \
    || fail "must re-prove k-settledness against the NEW tip, in BLOCK units"
grep -qE 'slot\.0\.saturating_add\(policy\.security_param\.0\)' <<< "$REC" \
    && fail "k-settledness must NOT be compared in SLOTS (that assumes an active-slot coefficient)"
grep -qE 'recertify_settled_anchor\(\)' <<< "$REC" \
    || fail "must delegate the integrity + cursor check to the store"
ok "canonical-at-slot + k-settled (block units) + store-side integrity"

RCA="$(awk '/pub fn recertify_settled_anchor/,/^    }/' "$STORE")"
[ -n "$RCA" ] || fail "recertify_settled_anchor missing"
grep -qE 'settled_fingerprint\(' <<< "$RCA" \
    || fail "re-certification must RE-VERIFY the CE-RF-6 fingerprint (certifying an unverified triple is worse than restoring from one)"
grep -qE 'get\(LAST_SLOT_KEY\)' <<< "$RCA" \
    || fail "must confirm the accumulator cursor SITS at the settled point (else it certifies lineage the state lacks)"
grep -qE 'insert\(LAST_ADVANCED_POINT_KEY' <<< "$RCA" \
    || fail "re-certification must WRITE a new LastAdvancedPoint -- that is the whole behaviour"
ok "fingerprint re-verified, cursor confirmed, anchor written"

echo "== DC-EPOCH-35: proof tests present =="

check() { grep -qE "fn ${1}\b" "$2" || fail "missing proof test: ${1}"; }
check a_recertified_settled_point_makes_the_next_pass_forward_fold            "$LIFE"
check recertification_refuses_a_settled_point_the_new_chain_abandoned          "$LIFE"
check recertification_refuses_a_settled_point_not_k_settled_against_the_new_tip "$LIFE"
check recertify_settled_anchor_writes_the_anchor_at_the_settled_point          "$STORE"
check recertify_refuses_a_triple_whose_fingerprint_does_not_verify             "$STORE"
check recertify_refuses_when_the_cursor_is_not_at_the_settled_point            "$STORE"
check recertify_refuses_when_no_settled_triple_exists                          "$STORE"
[ "$FAILED" -eq 0 ] && ok "all 7 named proof tests present"

if [ "$FAILED" -ne 0 ]; then
    echo "RESULT: FAIL (settled rewind no longer survives recovery)"
    exit 1
fi
echo "RESULT: PASS (DC-EPOCH-35 structurally enforced + proofs present)"
