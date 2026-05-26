#!/usr/bin/env bash
set -uo pipefail

# PHASE4-N-M-B S6 — admit-replay-equivalence integration test
# (DC-ADMIT-07).
#
# The headline true-tier strengthening of CN-STORE-03 is proved by
# `crates/ade_node/tests/admission_replay_equivalence.rs`. This
# gate asserts:
#   1. The integration test file exists.
#   2. The headline test function name is present (the registry's
#      `tests` array points at this name).
#   3. The test file imports `run_admission` + `AdmissionInputs`
#      (sanity: it really drives the runner, not a mock).
#   4. The test file does NOT bypass `WalStore::append` with any
#      direct mutation (would invalidate the property).
#
# Honest scope per memory
# `[[feedback-evidence-reducers-are-green-not-authority]]`: the
# JSONL byte-identity half is covered by
# `admission_log/writer.rs::admission_log_writer_two_runs_are_byte_identical`;
# the runner-level byte-identity half is covered by this gate's
# integration test. C will extend it with real corpus blocks to
# prove byte-identity over an admit chain.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TEST_FILE="$REPO_ROOT/crates/ade_node/tests/admission_replay_equivalence.rs"

FAILED=0
print_fail() { echo "FAIL: $1"; FAILED=1; }

# Guard 1: integration test file present.
if [[ ! -f "$TEST_FILE" ]]; then
    print_fail "missing $TEST_FILE"
    exit "$FAILED"
fi

# Guard 2: registered headline test name present.
required_tests=(
    "admission_replay_equivalence_byte_identical_wal_after_two_runs"
    "admission_signal_shutdown_returns_clean_exit"
    "admission_disconnect_to_zero_peers_exits_clean"
    "admission_exit_codes_match_registered_values"
    "admission_tip_update_does_not_emit_wal_entry"
)
for t in "${required_tests[@]}"; do
    if ! grep -qE "^(async )?fn $t\b" "$TEST_FILE"; then
        print_fail "test $t missing from $TEST_FILE"
    fi
done

# Guard 3: test imports the real runner.
if ! grep -qE 'use ade_node::admission::\{' "$TEST_FILE"; then
    print_fail "test does not import ade_node::admission:: bundle (would mean it isn't driving the real runner)"
fi
for sym in run_admission AdmissionInputs AdmissionPeerEvent; do
    if ! grep -qE "$sym" "$TEST_FILE"; then
        print_fail "test does not reference $sym (sanity check)"
    fi
done

# Guard 4: test does NOT bypass WalStore::append (would falsify
# DC-ADMIT-07). The valid WAL touchpoints are FileWalStore::open
# + .read_all() + (transitively) the runner calling append.
forbidden_wal_calls=$(grep -nE '\bappend\s*\(' "$TEST_FILE" 2>/dev/null \
    | grep -v -E 'wal_store\.append' \
    | grep -v -E '//' \
    || true)
if [[ -n "$forbidden_wal_calls" ]]; then
    # Anywhere `append(` appears outside the WalStore trait or a
    # comment is a flag.
    print_fail "test contains suspicious direct .append() calls; replay-equivalence requires all WAL writes flow through the runner:"
    echo "$forbidden_wal_calls"
fi

if (( FAILED == 0 )); then
    echo "OK: admit-replay-equivalence integration test present + drives real run_admission + no WAL bypass"
fi
exit $FAILED
