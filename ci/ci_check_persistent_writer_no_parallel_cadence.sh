#!/usr/bin/env bash
set -uo pipefail

# PHASE4-N-K S3 — persistent writer no parallel cadence (DC-NODE-02).
#
# The persistent writer and the orchestrator core MUST consult
# `should_snapshot_after_block` (the sole cadence policy). No file
# outside `crates/ade_runtime/src/rollback/cadence.rs` may:
#   - re-implement an `every_n_blocks` modulo on block_no
#   - define a second `SnapshotCadence`-like struct
#   - hardcode a parallel "snapshot every N" constant in a writer

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
ROLLBACK_DIR="$REPO_ROOT/crates/ade_runtime/src/rollback"
ORCHESTRATOR_DIR="$REPO_ROOT/crates/ade_runtime/src/orchestrator"

FAILED=0

print_fail() {
    echo "FAIL: $1"
    FAILED=1
}

strip_for_grep() {
    awk '
        /^#\[cfg\(test\)\]/ { in_test=1 }
        in_test { next }
        { line=$0; sub(/\/\/.*$/, "", line); print line }
    ' "$1"
}

# Files allowed to define the cadence policy.
ALLOWED_CADENCE_DEFINER="$ROLLBACK_DIR/cadence.rs"

# Forbid a second `pub fn should_snapshot_after_block` anywhere.
for f in $(find "$REPO_ROOT/crates" -type f -name '*.rs'); do
    if [[ "$f" == "$ALLOWED_CADENCE_DEFINER" ]]; then
        continue
    fi
    body=$(strip_for_grep "$f")
    if echo "$body" | grep -qE 'pub fn should_snapshot_after_block'; then
        print_fail "second cadence definer: $f (must reuse rollback::cadence::should_snapshot_after_block)"
    fi
done

# Writer files (persistent_writer.rs, snapshot_writer.rs, orchestrator
# core) MUST consult `should_snapshot_after_block` if they make any
# snapshot-cadence decision. Positive grep: at least one such call in
# `persistent_writer.rs`.
WRITER="$ROLLBACK_DIR/persistent_writer.rs"
if [[ -f "$WRITER" ]]; then
    body=$(strip_for_grep "$WRITER")
    if ! echo "$body" | grep -qE 'should_snapshot_after_block'; then
        print_fail "persistent_writer.rs must consult should_snapshot_after_block (single cadence policy)"
    fi
fi

# Orchestrator core must consult cadence via should_snapshot_after_block
# (no inline modulo on block_no).
CORE="$ORCHESTRATOR_DIR/core.rs"
if [[ -f "$CORE" ]]; then
    body=$(strip_for_grep "$CORE")
    if ! echo "$body" | grep -qE 'should_snapshot_after_block'; then
        print_fail "orchestrator core must consult should_snapshot_after_block on admission"
    fi
    if echo "$body" | grep -qE 'every_n_blocks[[:space:]]*\*|block_no\.0[[:space:]]*%'; then
        print_fail "orchestrator core contains inline cadence arithmetic (must route through cadence.rs)"
    fi
fi

if (( FAILED == 0 )); then
    echo "OK: persistent writer + orchestrator core route cadence through cadence.rs"
fi
exit $FAILED
