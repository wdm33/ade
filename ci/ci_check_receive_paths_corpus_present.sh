#!/usr/bin/env bash
set -uo pipefail

# PHASE4-N-H S6 — receive-side mechanical adapter + live-evidence
# binary presence gate.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CORPUS_DRIVE="$REPO_ROOT/crates/ade_runtime/tests/receive_pipeline_corpus_drive.rs"
TRANSCRIPT="$REPO_ROOT/crates/ade_runtime/tests/receive_session_transcript_replay.rs"
TWO_PEER="$REPO_ROOT/crates/ade_runtime/tests/receive_two_peer_independence.rs"
BINARY_SRC="$REPO_ROOT/crates/ade_core_interop/src/bin/live_block_follow_session.rs"
BINARY_CARGO="$REPO_ROOT/crates/ade_core_interop/Cargo.toml"
PROCEDURE="$REPO_ROOT/docs/clusters/PHASE4-N-H/CE-N-H-6_PROCEDURE.md"

FAILED=0

print_fail() {
    echo "FAIL: $1"
    FAILED=1
}

[[ -f "$CORPUS_DRIVE" ]] || print_fail "missing $CORPUS_DRIVE"
[[ -f "$TRANSCRIPT" ]] || print_fail "missing $TRANSCRIPT"
[[ -f "$TWO_PEER" ]] || print_fail "missing $TWO_PEER"
[[ -f "$BINARY_SRC" ]] || print_fail "missing $BINARY_SRC"
[[ -f "$BINARY_CARGO" ]] || print_fail "missing $BINARY_CARGO"

# Procedure doc lives under PHASE4-N-H/ pre-close and migrates to
# completed/PHASE4-N-H/ on cluster close. Accept either path.
if [[ ! -f "$PROCEDURE" && ! -f "$REPO_ROOT/docs/clusters/completed/PHASE4-N-H/CE-N-H-6_PROCEDURE.md" ]]; then
    print_fail "missing CE-N-H-6_PROCEDURE.md in either docs/clusters/PHASE4-N-H/ or docs/clusters/completed/PHASE4-N-H/"
fi

if [[ -f "$CORPUS_DRIVE" ]]; then
    for tn in \
        receive_pipeline_corpus_drive_admits_every_block \
        receive_pipeline_corpus_drive_chaindb_tip_matches_expected \
        receive_pipeline_corpus_drive_admitted_bytes_equal_corpus_bytes \
        receive_pipeline_corpus_drive_ledger_fingerprint_changes_on_admit
    do
        if ! grep -qE "fn $tn\b" "$CORPUS_DRIVE"; then
            print_fail "test $tn missing from $CORPUS_DRIVE"
        fi
    done
fi

if [[ -f "$TRANSCRIPT" ]]; then
    if ! grep -qE 'fn receive_session_transcript_replay_byte_identical\b' "$TRANSCRIPT"; then
        print_fail "transcript replay test missing from $TRANSCRIPT"
    fi
fi

if [[ -f "$TWO_PEER" ]]; then
    for tn in \
        two_peers_admit_same_block_into_shared_chaindb \
        two_peers_per_session_transcripts_match_solo_runs
    do
        if ! grep -qE "fn $tn\b" "$TWO_PEER"; then
            print_fail "test $tn missing from $TWO_PEER"
        fi
    done
fi

if [[ -f "$BINARY_CARGO" ]]; then
    if ! grep -qE 'name = "live_block_follow_session"' "$BINARY_CARGO"; then
        print_fail "live_block_follow_session [[bin]] entry missing from Cargo.toml"
    fi
fi

if (( FAILED == 0 )); then
    echo "OK: receive-paths mechanical adapter + live-evidence binary present"
fi
exit $FAILED
