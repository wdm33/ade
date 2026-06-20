#!/usr/bin/env bash
set -uo pipefail

# EPOCH-CONSENSUS-VIEW Slice 1 (DC-EVIEW-01, GATE-MEM / D4): the bounded-
# materialization memory gate ships a COMMITTED, reproducible corpus and a FIXED
# RssAnon-delta ceiling -- NOT "calibrate later". This gate pins those constants so
# the ceiling can never silently drift away or the assertion be removed. The metric
# is owned RssAnon (the anonymous resident heap, which EXCLUDES the mmap-backed redb
# store) so a correctly disk-backed materialization shows only a small bounded delta.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"; cd "$REPO_ROOT"
FAILED=0; fail() { echo "FAIL: $1"; FAILED=1; }
MEM=crates/ade_runtime/tests/transient_view_memory.rs

test -f "$MEM" || fail "the memory gate test ($MEM) is missing"

# (1) the committed corpus N is a FIXED constant.
grep -qE 'const CORPUS_N: u64 = [0-9_]+;' "$MEM" \
    || fail "CORPUS_N is not a fixed committed constant"

# (2) the hard regression ceiling is a FIXED constant (committed, not deferred).
grep -qE 'const RSS_ANON_DELTA_CEILING_KIB: u64 = [0-9_]+;' "$MEM" \
    || fail "RSS_ANON_DELTA_CEILING_KIB is not a fixed committed constant"

# (3) the test ASSERTS the delta is below the ceiling (the ceiling is not dead).
grep -qF 'delta < RSS_ANON_DELTA_CEILING_KIB' "$MEM" \
    || fail "the gate does not assert the RssAnon delta is below the ceiling"

# (4) the metric is owned RssAnon (excludes the mmap'd redb), read from /proc.
grep -qF 'RssAnon:' "$MEM" \
    || fail "the gate does not measure RssAnon (the anonymous-heap metric that excludes the mmap'd store)"
grep -qF '/proc/self/status' "$MEM" \
    || fail "the RssAnon reader does not read /proc/self/status"

# (5) the disk side is asserted: all N entries live on disk (len()==CORPUS_N).
grep -qF 'on_disk_len, CORPUS_N' "$MEM" \
    || fail "the gate does not assert all N entries are on disk (len()==CORPUS_N)"

# (6) the headline bounded-materialization test exists.
grep -qE 'fn transient_materialization_rss_anon_delta_bounded' "$MEM" \
    || fail "the bounded-materialization test is missing"

# (7) GATE-NOT-LIVE belt-and-braces: the memory gate must not enable track_utxo=true.
if grep -qE '\.track_utxo *= *true|track_utxo: *true' "$MEM"; then
    fail "the memory gate enables track_utxo=true -- the transient gate is track_utxo-agnostic"
fi

if (( FAILED == 0 )); then
    echo "OK: transient-view memory ceiling (DC-EVIEW-01 GATE-MEM; committed corpus + fixed RssAnon-delta ceiling, asserted, disk-backed)"
fi
exit $FAILED
