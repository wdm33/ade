#!/usr/bin/env bash
set -uo pipefail

# ACCUMULATOR-REFOLD-BOUND -- bounded post-rollback accumulator refold enforcement.
#
# Gates DC-EPOCH-26..31. The regression these prevent is a measured live one: rewinding the epoch
# accumulator to the BOOTSTRAP anchor on every reorg made the refold grow without bound with node
# uptime (26.6 min at 85,690 slots out, per-slot cost still rising). Because the accumulator is the
# frozen-leadership authority, a refold that outgrows the inter-reorg interval means leadership can
# never be promoted at a boundary -- i.e. the node cannot forge.
#
# This gate asserts STRUCTURE, not just that tests exist: removing the bootstrap fallback, comparing
# settledness in slots instead of blocks, dropping a lineage or uncertified guarantee, or promoting
# without k separation each fail here even if everything still compiles.
#
# NB (here-string discipline): never `echo "$BIGVAR" | grep -q` under pipefail -- grep exits on first
# match, echo takes SIGPIPE, and the gate flakes on large files. Grep files directly, or `<<<`.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"
FAILED=0
fail() { echo "FAIL: $1"; FAILED=1; }
ok()   { echo "  ok: $1"; }

STORE=crates/ade_runtime/src/chaindb/epoch_accumulator_store.rs
LIFE=crates/ade_node/src/node_lifecycle.rs
ADV=crates/ade_runtime/src/chaindb/epoch_accumulator_advance.rs

for f in "$STORE" "$LIFE" "$ADV"; do
    [ -f "$f" ] || { fail "missing $f"; echo "RESULT: FAIL"; exit 1; }
done

echo "== DC-EPOCH-26: settled target (k separation, in BLOCK units) =="

ADMIT="$(awk '/fn settled_rewind_admissible/,/^}/' "$LIFE")"
[ -n "$ADMIT" ] || fail "settled_rewind_admissible missing -- the rewind would be ungated"

# Settledness MUST be compared in block units. A slot comparison would need an active-slot-coefficient
# assumption and would be wrong wherever f differs.
grep -qE 'block_no\.0\.saturating_add\(security_param\)' <<< "$ADMIT" \
    || fail "settledness must be k BLOCKS of separation (block_no + security_param vs tip block_no)"
grep -qE 'slot\.0\.saturating_add\(security_param\)|slot\.0 \+ security_param' <<< "$ADMIT" \
    && fail "settledness must NOT be compared in SLOTS (that assumes an active-slot coefficient)"
ok "settledness compared in block units"

# Promotion in the store must require the same k separation.
ROLL="$(awk '/pub fn roll_settled_rewind_point/,/^    }/' "$STORE")"
grep -qE 'block_no\.0\.saturating_add\(security_param\)' <<< "$ROLL" \
    || fail "promotion must require k blocks of separation from the tip"
ok "promotion requires k separation"

echo "== DC-EPOCH-27: lineage-bound, and the bootstrap fallback survives =="

grep -qE 'header_hash' <<< "$ADMIT" \
    || fail "admission must verify the point's header hash still resolves canonically"
grep -qE 'target\.slot' <<< "$ADMIT" \
    || fail "admission must refuse a point ahead of the rollback target"
ok "lineage + target-order conditions present"

# The bootstrap rewind is the safety net every refusal falls back to. It must still be reachable
# from the rollback path -- removing it would turn a refusal into an unhandled case.
CLEAR="$(awk '/fn accumulator_admit_and_clear_for_rollback/,/^}/' "$LIFE")"
grep -qE 'reset_to_bootstrap\(\)' <<< "$CLEAR" \
    || fail "the bootstrap rewind fallback must remain in the rollback path"
grep -qE 'settled_rewind_admissible' <<< "$CLEAR" \
    || fail "the rollback path must consult the settled-rewind admission gate"
ok "bootstrap fallback retained under the admission gate"

echo "== DC-EPOCH-28/29: leadership coherence + uncertified after rewind =="

SETTLED="$(awk '/pub fn reset_to_settled/,/^    }/' "$STORE")"
[ -n "$SETTLED" ] || fail "reset_to_settled missing"
grep -qE 'remove\(LAST_ADVANCED_POINT_KEY\)' <<< "$SETTLED" \
    || fail "a rewind MUST clear LAST_ADVANCED_POINT (a rewound store is never lineage authority)"
grep -qE 'CURRENT_LEADERSHIP_BY_EPOCH' <<< "$SETTLED" \
    || fail "a rewind MUST restore leadership to the rewind point (no object may outrun the fold)"
grep -qE 'remove\(PENDING_BOUNDARY_MARK_KEY\)' <<< "$SETTLED" \
    || fail "a rewind MUST drop the pending boundary-mark binding (its lineage no longer holds)"
ok "rewind clears the anchor, restores leadership, drops the stale mark"

# Both rewind paths must discard the STAGED buffer -- it was taken on the abandoned chain.
for fn in reset_to_settled reset_to_bootstrap; do
    BODY="$(awk "/pub fn ${fn}/,/^    }/" "$STORE")"
    grep -qE 'remove\(PENDING_POINT_KEY\)' <<< "$BODY" \
        || fail "${fn} must discard the staged rewind point (abandoned chain)"
done
ok "both rewind paths discard the staged buffer"

echo "== DC-EPOCH-30/31: bounded refold + replay equivalence proofs =="

check_test() { grep -qE "fn ${1}\b" "$2" || fail "missing proof test: ${1} (expected in $2)"; }
check_test settled_point_is_only_promoted_once_k_blocks_settled          "$STORE"
check_test settled_point_never_falls_further_than_2k_behind_the_tip      "$STORE"
check_test reset_to_settled_restores_pair_and_leaves_store_uncertified   "$STORE"
check_test bootstrap_reset_discards_the_settled_rewind_point             "$STORE"
check_test settled_leadership_encoding_roundtrips_and_fails_closed_when_torn "$STORE"
check_test refold_from_settled_point_equals_fold_from_bootstrap          "$ADV"
check_test settled_rewind_admission_requires_settled_depth_and_intact_lineage "$LIFE"
[ "$FAILED" -eq 0 ] && ok "all 7 named proof tests present"

if [ "$FAILED" -ne 0 ]; then
    echo "RESULT: FAIL (bounded-refold enforcement regressed)"
    exit 1
fi
echo "RESULT: PASS (DC-EPOCH-26..31 structurally enforced + proofs present)"
