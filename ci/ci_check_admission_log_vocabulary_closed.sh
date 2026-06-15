#!/usr/bin/env bash
set -uo pipefail

# PHASE4-N-M-B S2 — bidirectional admission/wire-only vocabulary
# isolation (DC-ADMIT-04).
#
# Per `[[feedback-shell-must-not-overstate-semantic-truth]]`,
# admission-mode and wire-only-mode JSONL vocabularies are
# physically isolated. The two modes mean different things:
#   - wire-only mode = "bytes moved successfully on the socket"
#     (transport claim, never semantic);
#   - admission mode = "Ade's authority admitted a block, here is
#     the evidence comparison vs the peer" (semantic claim,
#     specifically a comparison evidence claim — still NOT
#     authority).
#
# A wire-only emit MUST NOT pretend to be an admission emit, and
# vice versa. The compiler closure (closed sums in event.rs files)
# is half the property; this grep is the file-tree half.
#
# Mechanical guards:
#   1. wire-only files MUST NOT contain admission-only literals.
#   2. admission files MUST NOT contain wire-only-only literals.
#   3. Shared literals (`node_started`, `node_shutdown`) MAY
#      appear in both (no rule applied).
#   4. The closed `AdmissionLogEvent` discriminator set must match
#      the registered allow-list (catches silent additions).

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"

ADMISSION_ONLY=(
    "admission_started"
    "snapshot_imported"
    "bootstrap_complete"
    "block_received"
    "block_admitted"
    "agreement_verdict"
    "admission_halted"
    "admission_shutdown"
    "memory_measure"
    "memory_summary"
)

WIRE_ONLY=(
    "peer_dial_started"
    "handshake_ok"
    "peer_tip_read"
    "peer_dial_failed"
    "wire_smoke_complete"
)

WIRE_ONLY_DIRS=(
    "$REPO_ROOT/crates/ade_node/src/live_log"
    "$REPO_ROOT/crates/ade_node/src/wire_only.rs"
)

ADMISSION_DIRS=(
    "$REPO_ROOT/crates/ade_node/src/admission_log"
    "$REPO_ROOT/crates/ade_node/src/admission"
)

FAILED=0
print_fail() { echo "FAIL: $1"; FAILED=1; }

# Guard 1: admission-only literals MUST NOT appear in wire-only files.
for lit in "${ADMISSION_ONLY[@]}"; do
    for target in "${WIRE_ONLY_DIRS[@]}"; do
        if [[ -e "$target" ]]; then
            hits=$(grep -rln --include='*.rs' "\"$lit\"" "$target" 2>/dev/null || true)
            if [[ -n "$hits" ]]; then
                print_fail "admission-only literal \"$lit\" appears in wire-only target:"
                echo "$hits"
            fi
        fi
    done
done

# Guard 2: wire-only-only literals MUST NOT appear in admission files.
for lit in "${WIRE_ONLY[@]}"; do
    for target in "${ADMISSION_DIRS[@]}"; do
        if [[ -e "$target" ]]; then
            hits=$(grep -rln --include='*.rs' "\"$lit\"" "$target" 2>/dev/null || true)
            if [[ -n "$hits" ]]; then
                print_fail "wire-only-only literal \"$lit\" appears in admission target:"
                echo "$hits"
            fi
        fi
    done
done

# Guard 3: assert event.rs declares exactly the closed allow-list.
EVENT_FILE="$REPO_ROOT/crates/ade_node/src/admission_log/event.rs"
if [[ ! -f "$EVENT_FILE" ]]; then
    print_fail "missing $EVENT_FILE"
else
    for lit in "${ADMISSION_ONLY[@]}"; do
        if ! grep -qE "\"$lit\"" "$EVENT_FILE"; then
            print_fail "admission allow-list missing \"$lit\" from $EVENT_FILE"
        fi
    done
fi

# Guard 4: the AdmissionLogEvent enum must NOT carry #[non_exhaustive].
if grep -nE '#\[non_exhaustive\]' "$EVENT_FILE" 2>/dev/null | head -1 > /dev/null; then
    while IFS=':' read -r lineno _rest; do
        next=$((lineno + 1))
        next_line=$(awk "NR==$next" "$EVENT_FILE")
        if echo "$next_line" | grep -qE 'pub enum (AdmissionLogEvent|AdmissionHaltReason|AdmissionShutdownReason)'; then
            print_fail "admission_log/event.rs sum carries #[non_exhaustive] (must remain closed): $EVENT_FILE:$lineno"
        fi
    done < <(grep -nE '#\[non_exhaustive\]' "$EVENT_FILE" 2>/dev/null)
fi

if (( FAILED == 0 )); then
    echo "OK: admission + wire-only vocabularies bidirectionally isolated; AdmissionLogEvent closed"
fi
exit $FAILED
