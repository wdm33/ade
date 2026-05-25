#!/usr/bin/env bash
set -uo pipefail

# PHASE4-N-H S3 — GREEN receive adapter + chain-write purity.
#
# Mechanical guards for CE-N-H-3 (DC-PROTO-09):
#
#   1. `events_to_state.rs` and `in_memory_chain_write.rs` may not
#      import wall-clock, randomness, async runtime, or HashMap.
#   2. `events_to_state.rs` must NOT decode header_bytes or
#      block_bytes (opaque pass-through; reducer decodes).
#   3. Positive presence: lift_chain_sync_signal +
#      lift_block_fetch_event + ChainDbWriter exist.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
EVENTS="$REPO_ROOT/crates/ade_runtime/src/receive/events_to_state.rs"
CHAIN_WRITE="$REPO_ROOT/crates/ade_runtime/src/receive/in_memory_chain_write.rs"

FAILED=0

print_fail() {
    echo "FAIL: $1"
    FAILED=1
}

check_purity() {
    local file="$1"
    local label="$2"
    if [[ ! -f "$file" ]]; then
        print_fail "target file missing: $file"
        return
    fi
    local body
    body=$(awk '
        /^#\[cfg\(test\)\]/ { in_test=1 }
        in_test { next }
        { line=$0; sub(/\/\/.*$/, "", line); print line }
    ' "$file")
    if echo "$body" | grep -qE '\bHashMap\b|\bHashSet\b'; then
        print_fail "$label: HashMap/HashSet forbidden"
    fi
    if echo "$body" | grep -qE 'std::time::SystemTime|\btokio\b|\brand::|\brand_core'; then
        print_fail "$label: wall-clock / tokio / rand forbidden"
    fi
}

check_purity "$EVENTS" "events_to_state.rs"
check_purity "$CHAIN_WRITE" "in_memory_chain_write.rs"

# events_to_state.rs must NOT decode header/block bytes itself.
if [[ -f "$EVENTS" ]]; then
    events_body=$(awk '
        /^#\[cfg\(test\)\]/ { in_test=1 }
        in_test { next }
        { line=$0; sub(/\/\/.*$/, "", line); print line }
    ' "$EVENTS")
    if echo "$events_body" | grep -qE 'decode_block_envelope|\bdecode_block\b|accepted_block_header_bytes'; then
        print_fail "events_to_state.rs must not decode header/block bytes (pass-through only)"
    fi
fi

# Positive presence.
if ! grep -qE 'pub fn lift_chain_sync_signal' "$EVENTS"; then
    print_fail "lift_chain_sync_signal missing"
fi
if ! grep -qE 'pub fn lift_block_fetch_event' "$EVENTS"; then
    print_fail "lift_block_fetch_event missing"
fi
if ! grep -qE 'impl<.*> ChainDbWrite for ChainDbWriter' "$CHAIN_WRITE"; then
    print_fail "ChainDbWriter must impl ChainDbWrite"
fi

if (( FAILED == 0 )); then
    echo "OK: receive GREEN adapter + chain-write invariants hold"
fi
exit $FAILED
