#!/usr/bin/env bash
set -uo pipefail

# PHASE4-N-F-C L1 — node lifecycle owner single-authority gate (CN-NODE-01).
#
# The cluster names exactly ONE production recovered-state lifecycle
# owner (the module carrying the `PHASE4-N-F-C-LIFECYCLE-OWNER` marker).
# CN-NODE-01 requires that the owner obtain initial state SOLELY via the
# single `bootstrap_initial_state` authority — never a parallel
# storage-init / cold-start path.
#
# L1 honesty: at L1 the owner does NOT yet *call* `bootstrap_initial_state`
# (the first-run arm needs the Mithril composition from L2; the warm-start
# arm needs the recovered provenance from L3 — both arms fail closed in
# L1). So this gate enforces the L1-true half of CN-NODE-01: the owner
# contains NO parallel initial-state path and NO genesis/bundle/cold
# fallback, so when state IS obtained it can only be via the single
# authority. (L2/L3 TIGHTEN this gate to additionally require the positive
# `bootstrap_initial_state(` call on each wired arm — see the marked TODO.)
#
# Guards (comments + `#[cfg(test)]` stripped before the negative greps so
# the owner's own doc-comment prose cannot false-trip):
#   (a) exactly one module carries the PHASE4-N-F-C-LIFECYCLE-OWNER marker;
#   (b) that owner has NO parallel storage-init: no InMemoryChainDb, no
#       LedgerState::new(, no materialize_rolled_back_state(;
#   (c) that owner has NO genesis/bundle/cold/tip fallback: no
#       import_live_consensus_inputs, no consensus_inputs_path, no
#       genesis_initial, no bootstrap_from_conway_genesis;
#   (d) the single bootstrap authority still holds: exactly one
#       `pub fn bootstrap_initial_state` in ade_runtime/src/bootstrap.rs;
#   (e) the diagnostic produce path stays diagnostic: produce_mode.rs does
#       not pass RequiredFromRecoveredProvenance.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
NODE="$REPO_ROOT/crates/ade_node/src"
RT="$REPO_ROOT/crates/ade_runtime/src"
MARKER="PHASE4-N-F-C-LIFECYCLE-OWNER"

FAILED=0
print_fail() { echo "FAIL (lifecycle owner): $1"; FAILED=1; }

# Strip the `#[cfg(test)]` module (attribute to EOF) + line comments, so
# guards only see production, non-comment code.
strip_for_grep() {
    awk '
        /^#\[cfg\(test\)\]/ { in_test=1 }
        in_test { next }
        { line=$0; sub(/\/\/.*$/, "", line); print line }
    ' "$1"
}

# --- guard (a): exactly one module carries the marker -----------------------
mapfile -t owners < <(grep -rl "$MARKER" "$NODE" "$RT" --include='*.rs' 2>/dev/null || true)
if [[ "${#owners[@]}" -ne 1 ]]; then
    print_fail "expected exactly 1 module carrying $MARKER, found ${#owners[@]}: ${owners[*]:-none}"
    echo "FAIL: ci_check_lifecycle_owner_uses_bootstrap_initial_state"
    exit 1
fi
OWNER="${owners[0]}"
OWNER_BODY="$(strip_for_grep "$OWNER")"

# --- guard (b): no parallel storage-init in the owner -----------------------
for tok in 'InMemoryChainDb' 'LedgerState::new\(' 'materialize_rolled_back_state\('; do
    if echo "$OWNER_BODY" | grep -qE "$tok"; then
        print_fail "owner ($(basename "$OWNER")) contains a parallel initial-state path token: $tok — initial state must come only via bootstrap_initial_state (CN-NODE-01)"
    fi
done

# --- guard (c): no genesis/bundle/cold/tip fallback in the owner ------------
for tok in 'import_live_consensus_inputs' 'consensus_inputs_path' 'genesis_initial' 'bootstrap_from_conway_genesis'; do
    if echo "$OWNER_BODY" | grep -qE "$tok"; then
        print_fail "owner ($(basename "$OWNER")) references a forbidden fallback token: $tok — the node lifecycle is Mithril-only, fail-closed"
    fi
done

# --- guard (d): single bootstrap authority still holds ----------------------
BOOT="$RT/bootstrap.rs"
if [[ ! -f "$BOOT" ]]; then
    print_fail "bootstrap.rs not found at $BOOT"
else
    boot_body="$(strip_for_grep "$BOOT")"
    c="$(echo "$boot_body" | grep -cE 'pub fn bootstrap_initial_state')"
    if [[ "$c" != "1" ]]; then
        print_fail "expected exactly 1 'pub fn bootstrap_initial_state', found $c"
    fi
fi

# --- guard (e): produce_mode stays diagnostic (no recovered-state consume) ---
PRODUCE="$NODE/produce_mode.rs"
if [[ -f "$PRODUCE" ]]; then
    if strip_for_grep "$PRODUCE" | grep -q 'RequiredFromRecoveredProvenance'; then
        print_fail "produce_mode must NOT pass RequiredFromRecoveredProvenance (stays diagnostic; recovered-state consume is the node-lifecycle owner path)"
    fi
fi

# NOTE (L2/L3 TODO): once the first-run (L2) and warm-start (L3) arms are
# wired, add a positive guard requiring the owner to CALL
# `bootstrap_initial_state(`, and require that any
# `RequiredFromRecoveredProvenance` construction in the workspace appears
# only in the owner / recovery module.

if (( FAILED == 0 )); then
    echo "OK (lifecycle owner): single marked owner ($(basename "$OWNER")) has no parallel storage-init and no genesis/bundle/cold fallback; bootstrap_initial_state remains the sole authority (L1: both arms fail closed pending L2/L3)"
fi
exit $FAILED
