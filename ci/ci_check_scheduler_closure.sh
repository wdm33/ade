#!/usr/bin/env bash
set -uo pipefail

# PHASE4-N-C S6 — RED scheduler + GREEN tick-assembler + RED broadcast
# closure gate. Closes CE-N-C-6 mechanical half (OP-OPS-05 enforcement
# evidence).
#
# Mechanical guards:
#
#   1. `scheduler_step` and `assemble_tick` are pure of I/O. Grep for
#      `std::fs` / `std::env` / `tokio::time` / `getrandom` / `rand::` /
#      `println!` / `dbg!` / `async fn` / `.await` in
#      crates/ade_runtime/src/producer/{scheduler,tick_assembler,broadcast}.rs
#      — any match outside `#[cfg(test)]` fails the gate. `std::time` is
#      permitted ONLY in tests/producer_pipeline_slot_deadline.rs (the
#      whitelisted SLA-measurement integration test).
#   2. `broadcast::enqueue` consumes `AcceptedBlock` by value. Any
#      `fn enqueue.*&AcceptedBlock` / `fn enqueue.*Vec<u8>` /
#      `fn enqueue.*&\[u8\]` reference-typed or raw-byte signature is
#      a failure (defeats S5's type-level gate).
#   3. `SchedulerInput`, `SchedulerEffect`, `SchedulerHaltReason`,
#      `BroadcastError`, `TickAssemblyError` are closed sums (no
#      `#[non_exhaustive]`).
#   4. No `pub fn` in broadcast.rs returns raw bytes other than the
#      `dequeue() -> Option<AcceptedBlock>` token-preserving accessor.
#   5. No `cardano_crypto::vrf::VrfDraft03::prove` / `KesAlgorithm::sign_kes` /
#      `KesAlgorithm::update_kes` in scheduler.rs / tick_assembler.rs /
#      broadcast.rs. Signing is S1's signing.rs exclusive.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"

SCHEDULER_RS="$REPO_ROOT/crates/ade_runtime/src/producer/scheduler.rs"
TICK_RS="$REPO_ROOT/crates/ade_runtime/src/producer/tick_assembler.rs"
BROADCAST_RS="$REPO_ROOT/crates/ade_runtime/src/producer/broadcast.rs"
TIMING_TEST="$REPO_ROOT/crates/ade_runtime/tests/producer_pipeline_slot_deadline.rs"

TARGET_FILES=("$SCHEDULER_RS" "$TICK_RS" "$BROADCAST_RS")

FAILED=0

print_fail() {
    echo "FAIL: $1"
    FAILED=1
}

emit_production_lines() {
    local f="$1"
    awk '
        /^#\[cfg\(test\)\]/ { exit }
        {
            line=$0
            sub(/\/\/.*$/, "", line)
            print NR ":" line
        }
    ' "$f"
}

for f in "${TARGET_FILES[@]}"; do
    [ -f "$f" ] || print_fail "expected file missing: $f"
done
[ -f "$TIMING_TEST" ] || print_fail "expected timing test missing: $TIMING_TEST"
[ "$FAILED" -eq 0 ] || exit 1

# ---------------------------------------------------------------------------
# Guard 1 — purity. std::time only allowed in the whitelisted timing test.
# ---------------------------------------------------------------------------
GUARD1_PATTERNS=(
    'std::time'
    'tokio::time'
    'rand::'
    'getrandom'
    'std::fs'
    'std::env'
    'std::net'
    'println!'
    'eprintln!'
    'dbg!'
    'async fn'
    '\.await\b'
)

for f in "${TARGET_FILES[@]}"; do
    lines=$(emit_production_lines "$f")
    for pattern in "${GUARD1_PATTERNS[@]}"; do
        matches=$(echo "$lines" | grep -E "$pattern" || true)
        if [ -n "$matches" ]; then
            print_fail "Guard 1 (impure pattern '$pattern' in $f):"
            echo "$matches"
        fi
    done
done

# std::time is permitted ONLY in the timing test (whitelisted by path).
# Any other producer/ source file containing std::time is a failure.
# Each file is run through emit_production_lines first so that std::time
# in a `//` comment or inside a #[cfg(test)] module does not trip the
# gate (the whitelisted timing test lives in tests/, outside PRODUCER_DIR).
PRODUCER_DIR="$REPO_ROOT/crates/ade_runtime/src/producer"
if [ -d "$PRODUCER_DIR" ]; then
    while IFS= read -r -d '' rs; do
        hits=$(emit_production_lines "$rs" | grep -E 'std::time' || true)
        if [ -n "$hits" ]; then
            print_fail "Guard 1 (std::time inside producer/ source — only the timing integration test may import std::time):"
            echo "$hits" | while IFS= read -r h; do
                [ -z "$h" ] && continue
                echo "  $rs:$h"
            done
        fi
    done < <(find "$PRODUCER_DIR" -name '*.rs' -print0)
fi

# ---------------------------------------------------------------------------
# Guard 2 — broadcast::enqueue consumes AcceptedBlock by value.
# ---------------------------------------------------------------------------
ENQUEUE_LINES=$(grep -nE 'fn enqueue\b' "$BROADCAST_RS" 2>/dev/null || true)
if [ -z "$ENQUEUE_LINES" ]; then
    print_fail "Guard 2 (broadcast.rs has no enqueue function — surface incomplete)"
fi
# Reject reference-typed or raw-byte argument shapes.
if echo "$ENQUEUE_LINES" | grep -E -q 'fn enqueue.*&AcceptedBlock|fn enqueue.*Vec<u8>|fn enqueue.*&\[u8\]'; then
    print_fail "Guard 2 (broadcast::enqueue takes AcceptedBlock by reference or raw bytes — must be by value/move):"
    echo "$ENQUEUE_LINES"
fi
# Positive shape check: signature names AcceptedBlock by value.
if ! echo "$ENQUEUE_LINES" | grep -E -q 'fn enqueue\s*\(\s*&mut self\s*,\s*[A-Za-z_][A-Za-z0-9_]*\s*:\s*AcceptedBlock\b'; then
    print_fail "Guard 2 (broadcast::enqueue signature does not match 'fn enqueue(&mut self, _: AcceptedBlock) -> _'):"
    echo "$ENQUEUE_LINES"
fi

# ---------------------------------------------------------------------------
# Guard 3 — closed sums; no #[non_exhaustive].
# ---------------------------------------------------------------------------
GUARD3_TYPES=(
    "SchedulerInput:$SCHEDULER_RS"
    "SchedulerEffect:$SCHEDULER_RS"
    "SchedulerHaltReason:$SCHEDULER_RS"
    "BroadcastError:$BROADCAST_RS"
    "TickAssemblyError:$TICK_RS"
)
for entry in "${GUARD3_TYPES[@]}"; do
    ty="${entry%%:*}"
    file="${entry#*:}"
    if grep -B1 -E "pub (enum|struct) $ty\\b" "$file" | grep -q '#\[non_exhaustive\]'; then
        print_fail "Guard 3 ($ty is #[non_exhaustive] — must be a closed sum)"
    fi
done

# Reject String-bearing variants on every closed sum.
for entry in "${GUARD3_TYPES[@]}"; do
    ty="${entry%%:*}"
    file="${entry#*:}"
    body=$(awk -v ty="$ty" '
        $0 ~ "pub enum " ty " *\\{" { open=1; depth=0 }
        open {
            depth += gsub(/\{/, "{")
            depth -= gsub(/\}/, "}")
            print
            if (depth == 0 && /\}/) { exit }
        }
    ' "$file")
    if echo "$body" | grep -E -q ': *String\b|: *alloc::string::String\b'; then
        print_fail "Guard 3 ($ty has a String-bearing variant):"
        echo "$body" | grep -E ': *String\b|: *alloc::string::String\b'
    fi
done

# ---------------------------------------------------------------------------
# Guard 4 — broadcast.rs raw-byte pub fns only via the AcceptedBlock-typed
# dequeue path. `as_bytes` / `into_bytes` accessors live on AcceptedBlock
# itself (in ade_ledger::producer::self_accept), not in this file.
# ---------------------------------------------------------------------------
RAW_BYTE_PUB_FNS=$(emit_production_lines "$BROADCAST_RS" | grep -E 'pub fn .*-> *(Vec<u8>|&\[u8\])' || true)
while IFS= read -r line; do
    [ -z "$line" ] && continue
    print_fail "Guard 4 (pub fn returning raw bytes in broadcast.rs — must hand back AcceptedBlock token):"
    echo "  $line"
done <<< "$RAW_BYTE_PUB_FNS"

# ---------------------------------------------------------------------------
# Guard 5 — no signing API call in scheduler.rs / tick_assembler.rs /
# broadcast.rs. Signing primitives live in S1's signing.rs only.
# ---------------------------------------------------------------------------
GUARD5_PATTERNS=(
    'VrfDraft03::prove'
    'Sum6Kes::sign_kes'
    'Sum6Kes::update_kes'
    'KesAlgorithm::sign_kes'
    'KesAlgorithm::update_kes'
)
for f in "${TARGET_FILES[@]}"; do
    lines=$(emit_production_lines "$f")
    for pattern in "${GUARD5_PATTERNS[@]}"; do
        matches=$(echo "$lines" | grep -E "$pattern" || true)
        if [ -n "$matches" ]; then
            print_fail "Guard 5 (signing API call in $f — pattern $pattern):"
            echo "$matches"
        fi
    done
done

if [ "$FAILED" -eq 0 ]; then
    echo "PASS: scheduler closure gates green (5/5)"
    exit 0
else
    exit 1
fi
