#!/usr/bin/env bash
set -euo pipefail

# CN-NODE-02 (planner half) + T-REC-03 precondition (PHASE4-N-F-D S1):
# the GREEN loop planner is a pure, closed lifecycle decision function that
# owns NO authority.
#
# Positive (whole file — banners/signatures): the module exists with a
# `//! GREEN` banner; LoopStep is defined; plan_loop_step is defined.
# Negative (production body only — doc/line comments + the #[cfg(test)]
# module stripped first, so commentary that names a forbidden token or
# #[non_exhaustive] while explaining its exclusion does NOT trip the gate):
# closed vocabulary (no #[non_exhaustive]), no authority/I/O token, no
# wildcard match arm.
#
# Repo-root-relative. Mirrors the other ci_check_*.sh gates.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

FILE="crates/ade_node/src/run_loop_planner.rs"

if [[ ! -f "$FILE" ]]; then
    echo "FAIL: $FILE not found"
    exit 1
fi

# --- positives (whole file; the banner is itself a comment) -----------------
if ! grep -qE '^//! GREEN' "$FILE"; then
    echo "FAIL: $FILE missing '//! GREEN' banner"
    exit 1
fi
if ! grep -qE '^pub enum LoopStep' "$FILE"; then
    echo "FAIL: LoopStep enum not defined in $FILE"
    exit 1
fi
if ! grep -qE '^pub fn plan_loop_step' "$FILE"; then
    echo "FAIL: plan_loop_step not defined in $FILE"
    exit 1
fi

# Production body: drop everything from the #[cfg(test)] module onward, then
# strip line/doc comments so commentary that names a forbidden token (while
# explaining its exclusion) does not trip the negative greps.
PROD="$(awk '/#\[cfg\(test\)\]/{exit} {print}' "$FILE" | sed -E 's://.*::')"

if [[ -z "$PROD" ]]; then
    echo "FAIL: could not isolate production body of $FILE"
    exit 1
fi

# Closed vocabulary: no #[non_exhaustive] on any planner type (production body).
if grep -qE '#\[non_exhaustive\]' <<<"$PROD"; then
    echo "FAIL: planner production body uses #[non_exhaustive] (closed vocabulary)"
    exit 1
fi

# No authority vocabulary and no I/O/clock/nondeterminism in the planner.
# Note (PHASE4-N-F-E S1): `SlotNo` is NOT in this whole-module ban — the pure
# `forge_slot_status` monotonic guard legitimately consumes a `SlotNo`. It is
# instead forbidden ONLY inside `plan_loop_step` (scoped check below), so step
# selection stays content-blind over the closed `ForgeSlotStatus`.
FORBIDDEN=(
    'pump_block'
    'run_node_sync'
    'run_real_forge'
    'forge_one_from_recovered'
    'correlate'
    'Ba02Manifest'
    'ChainDb'
    'LedgerState'
    'BlockHash'
    'ChainTip'
    'PumpTip'
    'put_block'
    'AdvanceTip'
    'rollback_to_slot'
    'std::fs'
    'tokio'
    'SystemTime'
    'Instant'
    'HashMap'
    '\bawait\b'
)

for pat in "${FORBIDDEN[@]}"; do
    if grep -qE "$pat" <<<"$PROD"; then
        echo "FAIL: planner production body matches forbidden token: $pat"
        exit 1
    fi
done

# No wildcard match arm — the decision table must be exhaustive by name.
if grep -qE '_[[:space:]]*=>' <<<"$PROD"; then
    echo "FAIL: planner uses a wildcard '_ =>' match arm (must be exhaustive)"
    exit 1
fi

# Scoped check (PHASE4-N-F-E S1): `plan_loop_step` itself must stay
# content-blind — it may NOT reference `SlotNo` (only the `forge_slot_status`
# guard may). Isolate the plan_loop_step body (from its signature up to the next
# top-level `pub fn`) out of the comment-stripped production body and assert it
# names no SlotNo.
PLAN_FN="$(awk '/^pub fn plan_loop_step/{f=1} f&&/^pub fn /&&!/plan_loop_step/{f=0} f{print}' <<<"$PROD")"
if [[ -z "$PLAN_FN" ]]; then
    echo "FAIL: could not isolate plan_loop_step body of $FILE"
    exit 1
fi
if grep -qE 'SlotNo' <<<"$PLAN_FN"; then
    echo "FAIL: plan_loop_step references SlotNo (must stay content-blind; SlotNo only in the forge_slot_status guard)"
    exit 1
fi

echo "OK: run_loop_planner is a closed, pure, authority-free GREEN planner (CN-NODE-02 planner half + N-F-E forge step)"
exit 0
