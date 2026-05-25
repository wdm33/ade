#!/usr/bin/env bash
set -uo pipefail

# PHASE4-N-I S4 — snapshot cadence purity (DC-STORE-07).
#
# Mechanical guards:
#   1. cadence.rs production code has no HashMap/wall-clock/tokio/rand.
#   2. SnapshotCadence has no runtime-mutable input field — its only
#      field is the BLUE-structural `every_n_blocks: u32`. Operator-
#      tunable cadence is explicitly out of scope per DC-STORE-07.
#   3. in_memory_cache.rs and chaindb_block_source.rs are pure.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
ROLLBACK_DIR="$REPO_ROOT/crates/ade_runtime/src/rollback"

FAILED=0

print_fail() {
    echo "FAIL: $1"
    FAILED=1
}

if [[ ! -d "$ROLLBACK_DIR" ]]; then
    print_fail "target directory missing: $ROLLBACK_DIR"
    exit "$FAILED"
fi

strip_for_grep() {
    awk '
        /^#\[cfg\(test\)\]/ { in_test=1 }
        in_test { next }
        { line=$0; sub(/\/\/.*$/, "", line); print line }
    ' "$1"
}

for f in "$ROLLBACK_DIR"/*.rs; do
    [[ -f "$f" ]] || continue
    body=$(strip_for_grep "$f")
    label=$(basename "$f")
    if echo "$body" | grep -qE '\bHashMap\b|\bHashSet\b'; then
        print_fail "$label: HashMap/HashSet forbidden"
    fi
    if echo "$body" | grep -qE 'std::time::SystemTime|\btokio\b|\brand::|\brand_core'; then
        print_fail "$label: wall-clock / tokio / rand forbidden"
    fi
done

# SnapshotCadence: only field is every_n_blocks (no operator-tunable
# runtime input).
cadence_struct_body=$(awk '
    /pub struct SnapshotCadence/ { in_struct=1; next }
    in_struct && /^}/ { exit }
    in_struct { print }
' "$ROLLBACK_DIR/cadence.rs")
field_count=$(echo "$cadence_struct_body" | grep -cE '^\s*pub [a-z_]+:')
if (( field_count != 1 )); then
    print_fail "SnapshotCadence must have exactly 1 field (every_n_blocks); found $field_count"
fi
if ! echo "$cadence_struct_body" | grep -qE 'every_n_blocks'; then
    print_fail "SnapshotCadence must contain every_n_blocks"
fi

# Positive presence.
if ! grep -qE 'pub fn should_snapshot_after_block' "$ROLLBACK_DIR/cadence.rs"; then
    print_fail "should_snapshot_after_block missing"
fi
if ! grep -qE 'impl SnapshotReader for InMemorySnapshotCache' "$ROLLBACK_DIR/in_memory_cache.rs"; then
    print_fail "InMemorySnapshotCache must impl SnapshotReader"
fi
if ! grep -qE 'impl<.*> BlockSource for ChainDbBlockSource' "$ROLLBACK_DIR/chaindb_block_source.rs"; then
    print_fail "ChainDbBlockSource must impl BlockSource"
fi

if (( FAILED == 0 )); then
    echo "OK: snapshot cadence + cache + block source invariants hold"
fi
exit $FAILED
