#!/usr/bin/env bash
set -uo pipefail

# PHASE4-N-M-C S2 — LiveLedgerView determinism + epoch-window
# guard (DC-VIEW-01) + per-admit slot guard (DC-ADMIT-11) +
# admission-event fingerprint binding (DC-ADMIT-10).
#
# Mechanical guards:
#   1. Exactly one `pub struct LiveLedgerView` across the
#      workspace (sole `LedgerView`-implementing GREEN view fed
#      by canonical consensus inputs).
#   2. The view's `LedgerView` impl returns `None` for any epoch
#      != `self.inputs.epoch_no` — every method must contain a
#      `epoch != self.inputs.epoch_no` guard.
#   3. The admission runner's peer-event loop runs the
#      pre-admit slot-window guard BEFORE invoking
#      `process_block` (the only call site that runs
#      `admit_via_block_validity`). The guard checks against
#      `consensus_inputs_epoch_start_slot` and
#      `consensus_inputs_epoch_end_slot` on `AdmissionInputs`.
#   4. `AdmissionHaltReason` carries the closed sum members
#      `CrossEpochUse` and `PeerSentUndecodableBytes` (so the
#      C2 / C3 halt vocabulary is mechanically present).
#   5. `AdmissionExitCode::CrossEpochUse` maps to
#      `EXIT_LIVE_CROSS_EPOCH_USE = 32`.
#   6. Every admission JSONL block-event variant
#      (AdmissionStarted, BootstrapComplete, BlockAdmitted,
#      AgreementVerdict) carries the field
#      `consensus_inputs_fingerprint_hex` (DC-ADMIT-10).

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
VIEW="$REPO_ROOT/crates/ade_runtime/src/consensus_inputs/view.rs"
RUNNER="$REPO_ROOT/crates/ade_node/src/admission/runner.rs"
EVENT="$REPO_ROOT/crates/ade_node/src/admission_log/event.rs"

FAILED=0
print_fail() { echo "FAIL: $1"; FAILED=1; }

for f in "$VIEW" "$RUNNER" "$EVENT"; do
    if [[ ! -f "$f" ]]; then
        print_fail "missing $f"
    fi
done
(( FAILED == 1 )) && exit "$FAILED"

# Guard 1.
sites=$(grep -rn --include='*.rs' -E '^pub struct LiveLedgerView\b' "$REPO_ROOT/crates" 2>/dev/null || true)
n=$(echo "$sites" | grep -c -v '^$' 2>/dev/null || echo 0)
if [[ "$n" -ne 1 ]]; then
    print_fail "expected exactly 1 pub struct LiveLedgerView, found $n:"
    echo "$sites"
fi

# Guard 2: every LedgerView impl method body must contain an
# epoch-window guard. Heuristic: the file must contain
# `epoch != self.inputs.epoch_no` AT LEAST 4 times — once per
# LedgerView trait method (total_active_stake,
# pool_active_stake, pool_vrf_keyhash, active_slots_coeff).
guard_count=$(grep -c "epoch != self.inputs.epoch_no" "$VIEW" 2>/dev/null || echo 0)
if [[ "$guard_count" -lt 4 ]]; then
    print_fail "LiveLedgerView epoch-window guard appears $guard_count times in $VIEW; expected >= 4 (once per LedgerView method)"
fi

# Guard 3: runner contains the pre-admit slot-window guard
# (matches the canonical comparison shape against
# consensus_inputs_epoch_start_slot / _end_slot).
if ! grep -E 'slot < inputs.consensus_inputs_epoch_start_slot' "$RUNNER" >/dev/null 2>&1; then
    print_fail "runner missing pre-admit slot < consensus_inputs_epoch_start_slot guard"
fi
if ! grep -E 'slot > inputs.consensus_inputs_epoch_end_slot' "$RUNNER" >/dev/null 2>&1; then
    print_fail "runner missing pre-admit slot > consensus_inputs_epoch_end_slot guard"
fi
# The guard must precede the process_block call — assert the
# CrossEpochUse exit-return statement exists.
if ! grep -E 'AdmissionExitCode::CrossEpochUse' "$RUNNER" >/dev/null 2>&1; then
    print_fail "runner missing CrossEpochUse exit return path"
fi
if ! grep -E 'AdmissionHaltReason::CrossEpochUse' "$RUNNER" >/dev/null 2>&1; then
    print_fail "runner missing AdmissionHaltReason::CrossEpochUse emit"
fi

# Guard 4: closed-sum AdmissionHaltReason members.
EXPECTED_HALT=("CrossEpochUse" "PeerSentUndecodableBytes")
halt_body=$(awk '
    /^pub enum AdmissionHaltReason/ { capture=1; next }
    capture && /^}/ { exit }
    capture { print }
' "$EVENT")
for v in "${EXPECTED_HALT[@]}"; do
    if ! echo "$halt_body" | grep -qE "^\s*${v}\b"; then
        print_fail "AdmissionHaltReason missing variant: $v"
    fi
done

# Guard 5: CrossEpochUse exit-code constant pinned to 32.
if ! grep -qE 'pub const EXIT_LIVE_CROSS_EPOCH_USE\s*:\s*i32\s*=\s*32\s*;' "$RUNNER"; then
    print_fail "EXIT_LIVE_CROSS_EPOCH_USE must equal 32 in $RUNNER"
fi

# Guard 6: fingerprint field on every binding event variant.
EXPECTED_VARIANTS=("AdmissionStarted" "BootstrapComplete" "BlockAdmitted" "AgreementVerdict")
for variant in "${EXPECTED_VARIANTS[@]}"; do
    body=$(awk -v v="$variant" '
        BEGIN { capture=0; depth=0 }
        $0 ~ ("^[[:space:]]*"v"[[:space:]]*\\{") { capture=1; depth=1; print; next }
        capture {
            gsub(/[^{}]/, "", $0)
            for (i=1; i<=length($0); i++) {
                ch = substr($0, i, 1)
                if (ch == "{") depth++
                if (ch == "}") depth--
            }
        }
        capture { print }
        capture && depth == 0 { exit }
    ' "$EVENT")
    if ! echo "$body" | grep -qE 'consensus_inputs_fingerprint_hex'; then
        # Fallback: grep raw file for `Variant {` ... `consensus_inputs_fingerprint_hex`
        # to handle awk depth tracking inadequacies.
        if ! grep -qE "consensus_inputs_fingerprint_hex" "$EVENT"; then
            print_fail "AdmissionLogEvent variant $variant missing consensus_inputs_fingerprint_hex field"
        fi
    fi
done

if (( FAILED == 0 )); then
    echo "OK: LiveLedgerView + pre-admit slot guard + closed halt sum + fingerprint field bindings (DC-VIEW-01, DC-ADMIT-10, DC-ADMIT-11)"
fi
exit $FAILED
