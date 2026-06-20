#!/usr/bin/env bash
set -uo pipefail

# EPOCH-CONSENSUS-VIEW Slice 1 (DC-EVIEW-01, GATE-NOT-LIVE): this slice proves the
# transient-materialization SUBSTRATE only. It must NOT enable track_utxo=true on the
# live producer path, must NOT add a runtime --transient-view-dir flag (no config
# surface on a consensus-adjacent lifecycle), and must keep the redb UTxO anchor
# dormant on the live path. The transient store is reachable only from test/bench code.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"; cd "$REPO_ROOT"
FAILED=0; fail() { echo "FAIL: $1"; FAILED=1; }
MOD=crates/ade_runtime/src/chaindb/transient_epoch_view.rs

test -f "$MOD" || fail "the transient module ($MOD) is missing"

# (1) the slice does NOT enable track_utxo=true on the live producer path.
if grep -rqE '\.track_utxo *= *true|track_utxo: *true' \
    crates/ade_node/src/node_lifecycle.rs crates/ade_node/src/node_sync.rs \
    crates/ade_runtime/src/admission/ crates/ade_runtime/src/forward_sync/ 2>/dev/null; then
    fail "the live producer path enables track_utxo=true -- not this slice (that is LIVE-LEDGER-APPLY)"
fi
# the transient module itself is track_utxo-agnostic (it stores raw entries, no ledger flag).
if grep -qE '\.track_utxo *= *true|track_utxo: *true' "$MOD"; then
    fail "the transient module enables track_utxo=true"
fi

# (2) NO runtime --transient-view-dir flag (D1: a fixed owned subtree, no config surface).
if grep -rqE 'transient[-_]view[-_]dir' crates/ade_node/src/cli.rs 2>/dev/null; then
    fail "a runtime --transient-view-dir flag was added (D1 forbids it: fixed owned subtree, no semantic flag)"
fi

# (3) the transient root is the FIXED owned subtree constant, derived from the data
#     root -- never the WAL / snapshot / ChainDb dir.
grep -qE 'pub const TRANSIENT_SUBTREE: &str = "transient-epoch-view";' "$MOD" \
    || fail "the owned transient subtree constant is missing/renamed"

# (4) the transient store has NO live wiring: it is referenced only from the module,
#     the chaindb mod re-export, tests, and the kill-target bin. (Authority-path
#     absence is the no-fallback gate; here we assert the module is not pulled into
#     the node binary's live run via node_lifecycle / produce_mode.)
if grep -rnE 'TransientEpochViewStore|transient_root\(|purge_transient_root' \
    crates/ade_node/src/ 2>/dev/null | grep -vE ':[0-9]+:[[:space:]]*//'; then
    fail "ade_node (the node binary) references the transient store on a live path"
fi

if (( FAILED == 0 )); then
    echo "OK: transient-view not-live (DC-EVIEW-01 GATE-NOT-LIVE; no track_utxo=true live, no runtime flag, fixed owned subtree, test/bench-only)"
fi
exit $FAILED
