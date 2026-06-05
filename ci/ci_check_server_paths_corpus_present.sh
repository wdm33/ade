#!/usr/bin/env bash
set -uo pipefail

# PHASE4-N-G S7 — Server-paths mechanical adapter + live-evidence
# binary presence gate.
#
# Mechanical guards for CE-N-G-7 + CE-N-G-8 (binary build):
#   1. The mechanical cross-impl integration test exists at the
#      expected path with the expected test names.
#   2. The live_block_fetch_session binary source file exists.
#   3. Its Cargo.toml [[bin]] entry is wired.
#   4. The CE-N-G-8 procedure doc exists.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
ADAPTER_TEST="$REPO_ROOT/crates/ade_runtime/tests/cross_impl_server_pipeline.rs"
TRANSCRIPT_TEST="$REPO_ROOT/crates/ade_runtime/tests/server_paths_transcript_replay.rs"
BINARY_SRC="$REPO_ROOT/crates/ade_core_interop/src/bin/live_block_fetch_session.rs"
BINARY_CARGO="$REPO_ROOT/crates/ade_core_interop/Cargo.toml"
PROCEDURE="$REPO_ROOT/docs/clusters/completed/PHASE4-N-G/CE-N-G-8_PROCEDURE.md"

FAILED=0

print_fail() {
    echo "FAIL: $1"
    FAILED=1
}

[[ -f "$ADAPTER_TEST" ]] || print_fail "missing $ADAPTER_TEST"
[[ -f "$TRANSCRIPT_TEST" ]] || print_fail "missing $TRANSCRIPT_TEST"
[[ -f "$BINARY_SRC" ]] || print_fail "missing $BINARY_SRC"
[[ -f "$BINARY_CARGO" ]] || print_fail "missing $BINARY_CARGO"
[[ -f "$PROCEDURE" ]] || print_fail "missing $PROCEDURE"

if [[ -f "$ADAPTER_TEST" ]]; then
    for tn in \
        cross_impl_server_pipeline_request_range_returns_decodable_bytes \
        cross_impl_server_pipeline_request_range_byte_identical_to_self_accept_input
    do
        if ! grep -qE "fn $tn\b" "$ADAPTER_TEST"; then
            print_fail "test $tn missing from $ADAPTER_TEST"
        fi
    done
fi

if [[ -f "$TRANSCRIPT_TEST" ]]; then
    for tn in \
        session_transcript_replay_byte_identical \
        session_transcript_served_block_bytes_equal_admitted_accepted_block_bytes \
        session_transcript_announced_header_matches_served_body_recipe
    do
        if ! grep -qE "fn $tn\b" "$TRANSCRIPT_TEST"; then
            print_fail "test $tn missing from $TRANSCRIPT_TEST"
        fi
    done
fi

if [[ -f "$BINARY_CARGO" ]]; then
    if ! grep -qE 'name = "live_block_fetch_session"' "$BINARY_CARGO"; then
        print_fail "live_block_fetch_session [[bin]] entry missing from Cargo.toml"
    fi
fi

if (( FAILED == 0 )); then
    echo "OK: server-paths corpus + mechanical adapter + live-evidence binary present"
fi
exit $FAILED
