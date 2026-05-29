#!/usr/bin/env bash
set -uo pipefail

# PHASE4-N-Y S2 — forward-sync chokepoint-only + GREEN reducer purity.
#
# Mechanical guards for CE-Y-5 + DC-SYNC-01:
#
#   1. The GREEN forward-sync reducer module
#      (crates/ade_runtime/src/forward_sync/reducer.rs) holds no I/O
#      state: no tokio, no redb, no SystemTime / wall-clock, no rand,
#      no HashMap/HashSet, no floating point. (GREEN-by-content.)
#   2. The reducer admits blocks ONLY through the BLUE chokepoint
#      `admit_via_block_validity` (transitively, via `receive_apply`).
#      No store / WAL / tip-advance path may bypass it: the only call
#      that advances ledger state is `receive_apply`, and the reducer
#      does not call `put_block` / `WalStore::append` / construct an
#      AdvanceTip outside the durable `AdmitPlan` path.
#   3. The redb writes + WAL appends live in the RED pump
#      (forward_sync/pump.rs), never in the reducer.
#   4. Positive presence: SyncEffect enum + forward_sync_step exist;
#      AdvanceTip is only emitted from the durable plan constructor.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
REDUCER="$REPO_ROOT/crates/ade_runtime/src/forward_sync/reducer.rs"
PUMP="$REPO_ROOT/crates/ade_runtime/src/forward_sync/pump.rs"

FAILED=0

print_fail() {
    echo "FAIL: $1"
    FAILED=1
}

# Strip line comments + the #[cfg(test)] block so doc-comment prose and
# test code don't trip the greps.
strip_body() {
    awk '
        /^#\[cfg\(test\)\]/ { in_test=1 }
        in_test { next }
        { line=$0; sub(/\/\/.*$/, "", line); print line }
    ' "$1"
}

if [[ ! -f "$REDUCER" ]]; then
    print_fail "GREEN reducer missing: $REDUCER"
    exit "$FAILED"
fi
if [[ ! -f "$PUMP" ]]; then
    print_fail "RED pump missing: $PUMP"
    exit "$FAILED"
fi

reducer_body=$(strip_body "$REDUCER")

# 1. GREEN purity grep on the reducer.
if echo "$reducer_body" | grep -qE '\btokio\b'; then
    print_fail "reducer.rs: tokio forbidden in GREEN reducer"
fi
if echo "$reducer_body" | grep -qE '\bredb\b'; then
    print_fail "reducer.rs: redb forbidden in GREEN reducer"
fi
if echo "$reducer_body" | grep -qE 'std::time::SystemTime|\bInstant\b|\bUNIX_EPOCH\b'; then
    print_fail "reducer.rs: wall-clock / SystemTime forbidden in GREEN reducer"
fi
if echo "$reducer_body" | grep -qE '\brand::|\brand_core\b'; then
    print_fail "reducer.rs: rand forbidden in GREEN reducer"
fi
if echo "$reducer_body" | grep -qE '\bHashMap\b|\bHashSet\b'; then
    print_fail "reducer.rs: HashMap/HashSet forbidden in GREEN reducer"
fi
if echo "$reducer_body" | grep -qE '\b(f32|f64)\b'; then
    print_fail "reducer.rs: floating point forbidden in GREEN reducer"
fi

# 2. No durability / tip primitive in the reducer: the reducer must not
#    itself call put_block or WalStore::append — those are the pump's.
if echo "$reducer_body" | grep -qE '\.put_block\b|\.append\(|FileWalStore'; then
    print_fail "reducer.rs: must not perform store/WAL I/O (RED pump only)"
fi

# 3. Admission chokepoint: the reducer's only state-advancing call is
#    receive_apply (which routes through admit_via_block_validity). It
#    must reference receive_apply and must NOT short-circuit with a
#    direct ledger-apply that skips the chokepoint.
if ! echo "$reducer_body" | grep -qE '\breceive_apply\b'; then
    print_fail "reducer.rs: must admit via the BLUE receive_apply chokepoint"
fi
if echo "$reducer_body" | grep -qE '\bapply_block_with_verdicts\b|\bvalidate_and_apply_header\b'; then
    print_fail "reducer.rs: must not call ledger/header apply directly (bypasses admit_via_block_validity)"
fi

# 4. Positive presence.
if ! echo "$reducer_body" | grep -qE 'pub enum SyncEffect'; then
    print_fail "reducer.rs: SyncEffect enum missing"
fi
if ! echo "$reducer_body" | grep -qE 'pub fn forward_sync_step'; then
    print_fail "reducer.rs: forward_sync_step missing"
fi

# AdvanceTip must be *constructed* in exactly one place — the durable
# plan constructor `AdmitPlan::durable`. A construction is the
# qualified struct literal `SyncEffect::AdvanceTip {` with field
# assignments; pattern matches use `{ .. }` and the enum definition is
# unqualified (`AdvanceTip {`), so neither is counted here.
advance_sites=$(echo "$reducer_body" \
    | grep -E 'SyncEffect::AdvanceTip \{' \
    | grep -vcE '\{ \.\. \}')
if (( advance_sites > 1 )); then
    print_fail "reducer.rs: AdvanceTip constructed in >1 place (found $advance_sites) — single durable-plan emit only"
fi

# The RED pump applies effects in order and guards tip-before-durable.
pump_body=$(strip_body "$PUMP")
if ! echo "$pump_body" | grep -qE 'TipBeforeDurable'; then
    print_fail "pump.rs: missing the tip-before-durable fail-closed guard"
fi
if ! echo "$pump_body" | grep -qE '\.put_block\b'; then
    print_fail "pump.rs: must perform the preserved-byte store via put_block"
fi
if ! echo "$pump_body" | grep -qE '\.append\('; then
    print_fail "pump.rs: must perform the WAL append"
fi

if (( FAILED == 0 )); then
    echo "OK: forward-sync GREEN reducer is pure + chokepoint-only; tip-advance is durable-gated (DC-SYNC-01)"
fi
exit $FAILED
