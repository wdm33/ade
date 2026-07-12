#!/usr/bin/env bash
set -uo pipefail

# LIVE-LEDGER-EPOCH-TRANSITION S4-pre-1c (DC-EPOCH-25): the leadership PoolDistr (nesPd) is answered by the
# self-contained, persisted FrozenLeadershipPoolDistr -- NEVER derived at leadership-use time from the go
# snapshot + active cert-state params. That derivation was a DISPROVEN hypothesis (LDAT): active params drop a
# retired-but-leadership-relevant pool's VRF (exactly 1 retired 1M-ADA pool on the v5 seed). The builder that
# encodes the disproven hypothesis is quarantined test-only as `from_accumulator_go_active_params_for_test_only`
# (the `_for_test_only` suffix blocks accidental production wiring). This gate proves it is referenced ONLY from
# its own definition + test / oracle / negative-regression code, never a production authority path.
#
# Repo-root-relative. Mirrors the other ci_check_*.sh gates (esp. ci_check_transient_view_no_fallback.sh).
#
# NOTE (S4 DONE): the SEED leadership authority (PoolDistrView::from_seed_epoch_consensus_inputs) has been
# retired from every production authority path -- S4-L1 flipped the initial/warm live view (node_lifecycle
# 658/840/3397 -> leadership_authority_for_epoch), and S4-L2 flipped the forward promotion path (epoch_wire
# prepare_authority_for_candidate_slot -> promotion_leadership_authority_for_epoch, seed+2 ceiling deleted).
# The seed projection now survives ONLY as a RECOVERY-identity check (proving a recovered view == the seed
# projection) and in tests -- never as a production leadership source. The forward promotion path is kept
# mechanically frozen-only by the sibling gate ci_check_frozen_promotion_no_seed_window.sh (S4-L2).

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"; cd "$REPO_ROOT"
FAILED=0; fail() { echo "FAIL: $1"; FAILED=1; }

# The quarantined go+active-params leadership builder (the DISPROVEN hypothesis).
SYM='from_accumulator_go_active_params_for_test_only'

# (1) It must NOT appear on any enumerated live authority surface (follow / forge / recovery / bootstrap /
#     snapshot / the durable stores). These files hold no legitimate reference at all.
AUTHORITY_PATHS=(
    crates/ade_node/src/node_lifecycle.rs
    crates/ade_node/src/node_sync.rs
    crates/ade_node/src/native_firstrun.rs
    crates/ade_node/src/epoch_wire.rs
    crates/ade_runtime/src/admission/
    crates/ade_runtime/src/forward_sync/
    crates/ade_runtime/src/rollback/
    crates/ade_runtime/src/receive/
    crates/ade_runtime/src/chaindb/
)
for p in "${AUTHORITY_PATHS[@]}"; do
    if [ -e "$p" ]; then
        hits=$(grep -rnE "$SYM" "$p" 2>/dev/null || true)
        if [ -n "$hits" ]; then
            fail "the disproven go+active leadership builder is referenced on an authority path ($p):"
            echo "$hits" | sed 's/^/    /'
        fi
    fi
done

# (2) Tree-wide: the ONLY allowed references are its definition + in-file test callers
#     (crates/ade_ledger/src/consensus_view.rs), test files, and bins. A code (non-comment) reference anywhere
#     else is a production leak. Comment lines (e.g. the doc reference in frozen_leadership.rs) are stripped.
LEAK=$(grep -rnE "$SYM" crates/ --include='*.rs' 2>/dev/null \
    | grep -vE 'crates/ade_ledger/src/consensus_view\.rs:' \
    | grep -vE '/tests/|/src/bin/' \
    | grep -vE ':[0-9]+:[[:space:]]*//' \
    || true)
if [ -n "$LEAK" ]; then
    fail "a non-test / non-definition code site references the quarantined leadership builder (it must stay test-only):"
    echo "$LEAK" | sed 's/^/    /'
fi

# (3) The quarantine name itself must persist -- a rename dropping the `_for_test_only` suffix would defeat this
#     grep-based guard. Assert the definition is present, spelled exactly.
if ! grep -qE "pub fn ${SYM}" crates/ade_ledger/src/consensus_view.rs; then
    fail "the quarantined builder 'pub fn ${SYM}' is missing from consensus_view.rs (renamed? the guard's contract is broken)"
fi

# (4) The rule is declared in the invariant registry.
REG="docs/ade-invariant-registry.toml"
grep -q 'DC-EPOCH-25' "$REG" \
    || fail "DC-EPOCH-25 is not declared in the invariant registry ($REG)"

# (5) S4-0 (DC-EPOCH-25 BRIDGE): leadership is read ONLY by EXACT epoch index. The epoch-indexed reader must
#     exist, and the store must define NO bare "current leadership" reader -- a method returning the leadership
#     object WITHOUT an epoch argument would reintroduce a "latest / current / nearest" production read. The
#     compiler already stops callers of a removed method, but a FUTURE re-addition of `fn leadership_authority(`
#     would silently compile; this grep keeps the absence mechanical.
STORE='crates/ade_runtime/src/chaindb/epoch_accumulator_store.rs'
grep -qE 'pub fn leadership_authority_for_epoch' "$STORE" \
    || fail "the exact epoch-indexed reader 'leadership_authority_for_epoch' is missing from the store (S4-0 contract broken)"
BARE=$(grep -nE 'fn leadership_authority\(' "$STORE" 2>/dev/null || true)
if [ -n "$BARE" ]; then
    fail "the store defines a bare 'leadership_authority(' current-leadership read (S4-0 forbids it -- read leadership ONLY by exact epoch):"
    echo "$BARE" | sed 's/^/    /'
fi

if (( FAILED == 0 )); then
    echo "OK: frozen-leadership authority (DC-EPOCH-25) -- the go+active builder stays test-only; leadership is answered ONLY by the exact epoch-indexed FrozenLeadershipPoolDistr read (no current / latest read)."
fi
exit $FAILED
