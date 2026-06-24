#!/usr/bin/env bash
set -uo pipefail

# ci_check_mithril_authority_transition.sh -- MITHRIL-VERIFIED-ANCHOR-INTEGRATION S1b.
#
# The native Mithril AUTHORITY TRANSITION: assemble the COMPLETE authoritative
# seed (LedgerState + PraosChainDepState + a NATIVE LiveConsensusInputsCanonical)
# from ONLY the verified manifest + the S1a NativeSnapshotNonUtxoState + the
# Stage-2 UTxO + genesis constants, enforce POINT COHERENCE as a terminal gate,
# and persist the durable bootstrap artifacts ATOMICALLY before the authority is
# visible -- with NO cardano-cli / JSON consensus-input bundle / operator seed /
# convenience fallback on this path (DC-MITHRIL-03).
#
# Mechanical guards on the native assembly module:
#   (A) the module + the two entry points exist;
#   (B) NO CLI / JSON / operator-bundle seed token participates on the native
#       path (the production body, cfg(test) stripped, references none of
#       import_cardano_cli_json_utxo / import_live_consensus_inputs /
#       --json-seed / consensus_inputs_path / require_forge_current_pparams);
#   (C) the four POINT-COHERENCE terminal arms are present (era/point/epoch/
#       network), each a structured error, before any persist;
#   (D) PERSIST-BEFORE-VISIBILITY: the native entry routes through the single
#       closed composition bootstrap_from_mithril_snapshot (which calls the sole
#       bootstrap_initial_state authority + the seed-epoch sidecar +
#       put_recovered_anchor_point + the WAL commit) -- it does NOT re-implement
#       a parallel storage-init path;
#   (E) the S1b tests are present.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
MOD="$REPO_ROOT/crates/ade_runtime/src/mithril_native_assembly.rs"
LINEAGE="$REPO_ROOT/crates/ade_runtime/src/seed_epoch_lineage.rs"

FAILED=0
print_fail() { echo "FAIL: $1"; FAILED=1; }

# Strip the #[cfg(test)] module (everything from the test attribute to EOF) and
# line comments, so guards only see the production assembly.
strip_for_grep() {
    awk '
        /^#\[cfg\(test\)\]/ { in_test=1 }
        in_test { next }
        { line=$0; sub(/\/\/.*$/, "", line); print line }
    ' "$1"
}

# (A) module + entry points exist.
[ -f "$MOD" ] || { print_fail "missing native-assembly module $MOD"; echo "FAIL: ci_check_mithril_authority_transition"; exit 1; }
BODY="$(strip_for_grep "$MOD")"

echo "$BODY" | grep -Eq '\bpub fn assemble_native_mithril_seed\b' \
    || print_fail "missing assemble_native_mithril_seed entry"
echo "$BODY" | grep -Eq '\bpub fn bootstrap_from_native_mithril_snapshot\b' \
    || print_fail "missing bootstrap_from_native_mithril_snapshot entry"

# (B) NO CLI / JSON / operator-bundle seed on the native path. These are the
# RED diagnostic/oracle importers; they must NOT participate in native bootstrap.
for forbidden in \
    'import_cardano_cli_json_utxo' \
    'import_live_consensus_inputs' \
    'require_forge_current_pparams' \
    'json_seed_path' \
    'consensus_inputs_path' \
    '--json-seed'
do
    if echo "$BODY" | grep -qF -- "$forbidden"; then
        print_fail "native assembly references the CLI/JSON/operator-bundle seed token '$forbidden' (DC-MITHRIL-03: the snapshot is the sole source on the native path)"
    fi
done

# (C) the four point-coherence terminal arms are present.
for arm in \
    'NonConwayEra' \
    'PointMismatch' \
    'PointHashMismatch' \
    'EpochMismatch' \
    'NetworkMismatch'
do
    echo "$BODY" | grep -q "$arm" \
        || print_fail "missing point-coherence terminal arm '$arm'"
done

# The assembly must enforce coherence BEFORE assembling the LedgerState: the
# first point-coherence return must precede the LedgerState construction.
COHERENCE_LINE=$(echo "$BODY" | grep -nE 'MithrilNativeAssemblyError::(NonConwayEra|PointMismatch)' | head -n1 | cut -d: -f1)
ASSEMBLE_LINE=$(echo "$BODY" | grep -nE '\blet ledger = LedgerState \{' | head -n1 | cut -d: -f1)
if [[ -n "$COHERENCE_LINE" && -n "$ASSEMBLE_LINE" ]]; then
    if (( COHERENCE_LINE >= ASSEMBLE_LINE )); then
        print_fail "point coherence (line $COHERENCE_LINE) must precede LedgerState assembly (line $ASSEMBLE_LINE)"
    fi
fi

# (D) persist-before-visibility: route through the single closed composition;
# do NOT re-implement a second storage-init path.
echo "$BODY" | grep -Eq '\bbootstrap_from_mithril_snapshot\(' \
    || print_fail "native entry must route through the single closed bootstrap_from_mithril_snapshot"
if echo "$BODY" | grep -qE '\bpub fn bootstrap_initial_state\b'; then
    print_fail "native assembly declares a second bootstrap_initial_state authority (no parallel storage-init path)"
fi
# The recovered-anchor point is persisted by the shared lineage persist, which
# the composition calls; assert that authority still calls put_recovered_anchor_point.
[ -f "$LINEAGE" ] || print_fail "missing shared lineage persist $LINEAGE"
if [ -f "$LINEAGE" ]; then
    grep -Eq '\bput_recovered_anchor_point\(' "$LINEAGE" \
        || print_fail "the shared lineage persist no longer calls put_recovered_anchor_point (the imported anchor point must be persisted + recoverable)"
fi

# (E) the S1b tests are present.
for t in \
    'native_assembled_seed_is_deterministic' \
    'native_assembly_maps_each_field_from_its_source' \
    'interrupted_persist_leaves_no_discoverable_anchor_lineage' \
    'native_bootstrap_persists_and_anchor_point_is_recoverable' \
    'point_mismatch_is_terminal' \
    'wrong_era_is_terminal' \
    'wrong_network_is_terminal'
do
    grep -q "fn $t" "$MOD" || print_fail "missing S1b test '$t'"
done

if (( FAILED == 0 )); then
    echo "OK: native Mithril authority transition assembles from manifest/S1a/Stage-2/genesis only (no CLI/JSON/operator seed), enforces point coherence before persist, and persists atomically through the single closed composition (DC-MITHRIL-03)"
fi
exit $FAILED
