#!/usr/bin/env bash
set -uo pipefail

# EPOCH-CONSENSUS-VIEW Slice 1 (DC-EVIEW-01, GATE-NO-FALLBACK): the transient
# replay store is GREEN / non-authoritative. It may enable bounded materialization
# but may NEVER become a fallback source for follow, forge, recovery, or snapshot
# activation. This gate proves NO live authority-path source file references the
# transient store -- it is unreachable from the authoritative code, by construction.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"; cd "$REPO_ROOT"
FAILED=0; fail() { echo "FAIL: $1"; FAILED=1; }

# The transient-store symbols that must NOT appear on any authority path.
SYMS='TransientEpochViewStore|transient_root|transient_epoch_view|purge_transient_root|TransientViewError'

# The live follow / forge / recovery / snapshot authority surfaces.
AUTHORITY_PATHS=(
    crates/ade_node/src/node_lifecycle.rs
    crates/ade_node/src/node_sync.rs
    crates/ade_runtime/src/admission/
    crates/ade_runtime/src/forward_sync/
    crates/ade_runtime/src/rollback/
    crates/ade_runtime/src/receive/
)

for p in "${AUTHORITY_PATHS[@]}"; do
    if [ -e "$p" ]; then
        hits=$(grep -rnE "$SYMS" "$p" 2>/dev/null || true)
        if [ -n "$hits" ]; then
            fail "the transient store is referenced on the authority path ($p):"
            echo "$hits"
        fi
    fi
done

# No non-test, non-bin caller of the transient store anywhere (the store is GREEN
# execution support, reachable ONLY from tests and the kill-target bin). The module
# itself, the chaindb mod re-export, the tests, and the bin are the only legal sites.
PROD_CALLERS=$(grep -rnE 'TransientEpochViewStore::open|\.materialize_batch\(' crates/ --include='*.rs' \
    | grep -vE 'tests/|src/bin/|chaindb/transient_epoch_view.rs' || true)
if [ -n "$PROD_CALLERS" ]; then
    fail "a non-test/non-bin call site constructs/uses the transient store (it must be GREEN/test-only):"
    echo "$PROD_CALLERS"
fi

if (( FAILED == 0 )); then
    echo "OK: transient-view no-fallback (DC-EVIEW-01 GATE-NO-FALLBACK; the transient store is unreachable from follow/forge/recovery/snapshot authority)"
fi
exit $FAILED
