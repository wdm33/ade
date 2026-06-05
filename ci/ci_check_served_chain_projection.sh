#!/usr/bin/env bash
set -uo pipefail

# ci_check_served_chain_projection.sh — PHASE4-N-U S3 gate (DC-NODE-13).
#
# Serve-as-durable-chain projection. The `--mode node` served view (ChainSync
# header advertisement + BlockFetch body) is a deterministic READ-ONLY PROJECTION
# of the durable ChainDb (whose sole production writers are pump_block / DC-NODE-12
# + the validated warm-start replay bootstrap_initial_state), NOT the retired
# PHASE4-N-F-G-R in-memory ServedChainSnapshot accumulator. This makes the
# CN-CONS-07 serve clause mechanical: serving cannot leak a byte that did not clear
# block_validity, because the durable ChainDb only ever holds block_validity-cleared
# bytes (forged via self_accept OR received via admit_via_block_validity).
#
# Checks:
#   (1) the projection adapter ChainDbServedSource exists, implements BOTH BLUE
#       serve seams (ServedHeaderLookup + ServedRangeLookup) over the durable
#       ChainDb (iter_from_slot / get_block_by_hash / tip);
#   (2) it reuses the single DC-CONS-18 header authority (block_header_bytes) and
#       serves stored.bytes VERBATIM — NO parallel splitter (no envelope re-walk),
#       NO re-encode (no tag-24 compose in the projection);
#   (3) the single serve-dispatch authority reads a ServedChainSource enum
#       {Snapshot | DurableChainDb} — still ONE dispatch (DC-NODE-07);
#   (4) the --mode node serve path reads ServedChainSource::DurableChainDb over an
#       Arc<dyn ChainDb> (run_node_serve_task);
#   (5) the G-R accumulator workaround is RETIRED from node_lifecycle: no
#       `pub fn serve_gate_admits`, no `.push_atomic(` call, no `ServedChainHandle::new(`;
#   (6) DC-NODE-13 is present-and-enforced in the registry;
#   (7) the superseded G-R gate ci_check_served_chain_stability.sh is removed.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

PROJ="crates/ade_runtime/src/network/served_chain_projection.rs"
DISP="crates/ade_runtime/src/network/serve_dispatch.rs"
LIFE="crates/ade_node/src/node_lifecycle.rs"
REG="docs/ade-invariant-registry.toml"
OLD_GATE="ci/ci_check_served_chain_stability.sh"

FAILED=0
fail() { echo "FAIL: $1"; FAILED=1; }

for f in "$PROJ" "$DISP" "$LIFE" "$REG"; do
    [[ -f "$f" ]] || fail "missing expected file $f"
done

# (1) projection adapter + both serve seams + durable ChainDb reads.
grep -Eq 'struct ChainDbServedSource' "$PROJ" \
    || fail "ChainDbServedSource (the durable-chain projection adapter) missing from $PROJ"
grep -Eq 'impl.*ServedHeaderLookup for ChainDbServedSource' "$PROJ" \
    || fail "ChainDbServedSource does not implement ServedHeaderLookup (ChainSync serve seam)"
grep -Eq 'impl.*ServedRangeLookup for ChainDbServedSource' "$PROJ" \
    || fail "ChainDbServedSource does not implement ServedRangeLookup (BlockFetch serve seam)"
grep -Eq 'iter_from_slot' "$PROJ" \
    || fail "the projection does not iterate the durable ChainDb (iter_from_slot)"
grep -Eq 'get_block_by_hash' "$PROJ" \
    || fail "the projection does not resolve durable blocks by hash (get_block_by_hash)"

# (2) reuses the single header authority; serves stored.bytes verbatim; no parallel
#     splitter; no re-encode in the projection.
grep -Eq 'block_header_bytes' "$PROJ" \
    || fail "the projection does not reuse the single header authority (block_header_bytes)"
grep -Eq 'stored\.bytes' "$PROJ" \
    || fail "the projection does not serve stored.bytes (the durable wire bytes)"
if grep -Eq 'decode_block_envelope|header_cbor_slice' "$PROJ"; then
    fail "the projection re-walks the block envelope (parallel splitter forbidden — reuse block_header_bytes)"
fi
if grep -Eq 'compose_blockfetch_block|compose_rollforward_header' "$PROJ"; then
    fail "the projection re-encodes/tag-24-wraps bytes (the BLUE reducer wraps; the projection serves stored.bytes verbatim)"
fi

# (3) single serve-dispatch authority reads a ServedChainSource enum (both arms).
grep -Eq 'enum ServedChainSource' "$DISP" \
    || fail "ServedChainSource enum missing from $DISP (the read-source selector)"
grep -Eq 'DurableChainDb' "$DISP" \
    || fail "ServedChainSource has no DurableChainDb arm (the --mode node durable projection)"
grep -Eq 'Snapshot' "$DISP" \
    || fail "ServedChainSource has no Snapshot arm (the --mode produce accumulator source)"

# (4) the --mode node serve path reads the durable projection.
grep -Eq 'ServedChainSource::DurableChainDb' "$LIFE" \
    || fail "the node serve path does not read ServedChainSource::DurableChainDb"
grep -Eq 'serve_chaindb: Arc<dyn ChainDb>' "$LIFE" \
    || fail "run_node_serve_task does not take the durable ChainDb (Arc<dyn ChainDb>)"

# (5) G-R accumulator workaround retired from node_lifecycle (code, not comments).
if grep -Eq 'pub fn serve_gate_admits' "$LIFE"; then
    fail "serve_gate_admits (the retired G-R monotone serve gate) is still defined in $LIFE"
fi
if grep -Eq '\.push_atomic\(' "$LIFE"; then
    fail "$LIFE still calls .push_atomic( (the retired accumulator feed) — serve must read the durable projection"
fi
if grep -Eq 'ServedChainHandle::new\(' "$LIFE"; then
    fail "$LIFE still constructs a ServedChainHandle (the retired in-memory accumulator)"
fi

# (6) DC-NODE-13 present-and-enforced.
awk '/id = "DC-NODE-13"/{f=1} f&&/status = "enforced"/{print "ok"; exit}' "$REG" | grep -q ok \
    || fail "DC-NODE-13 not present-and-enforced in the registry"

# (7) the superseded G-R gate is removed.
[[ ! -f "$OLD_GATE" ]] \
    || fail "the superseded G-R gate $OLD_GATE still exists (it is replaced by serve-as-projection)"

if [[ "$FAILED" -ne 0 ]]; then
    echo "ci_check_served_chain_projection: FAILED"
    exit 1
fi
echo "ci_check_served_chain_projection: OK (DC-NODE-13 — serve-as-projection of the durable ChainDb; G-R accumulator retired)"
