#!/usr/bin/env bash
set -uo pipefail

# PHASE4-N-F-A A2 + PHASE4-N-F-G-I — seed-epoch consensus-input provenance
# containment (CN-CINPUT-02 + CN-CINPUT-03).
#
# The persisted `SeedEpochConsensusInputs` sidecar may be populated ONLY on
# the verified-bootstrap composition path, through the anchor-keyed
# `SnapshotStore` surface. The bounty-primary forge-time path
# (`produce_mode`, `import_live_consensus_inputs*`,
# `pool_distr_view_from_consensus_inputs`, `--consensus-inputs-path`) must not
# build or `put` the sidecar.
#
# PHASE4-N-F-G-I extracted the populate primitives into a SINGLE shared
# authority `ade_runtime::seed_epoch_lineage::persist_seed_epoch_consensus_inputs`
# (the put -> append commit-point ordering lives there). The verified-bootstrap
# composition sites (`genesis_bootstrap`, `mithril_bootstrap`, and the operator
# admission/pre-seed `ade_node::admission::bootstrap`) reach the populator ONLY
# by calling that shared authority — a STRONGER containment than the prior
# inline-per-composer shape: one populator, an explicit closed caller set.
#
# Mechanical, data-flow-resistant guards (containment, not a bypassable RHS grep):
#   (a)  POSITIVE: the single shared populator authority `seed_epoch_lineage.rs`
#        holds the put + merge + encode + append primitives.
#   (a2) POSITIVE: each verified-bootstrap composition site calls
#        `seed_epoch_lineage::persist_seed_epoch_consensus_inputs(`.
#   (b)  NEGATIVE forge-time fence: `produce_mode.rs` names NONE of the sidecar
#        build/put/encode/append/persist tokens.
#   (c)  GLOBAL containment:
#        (c1) the populate primitives (`.put_..(`, `merge_..(`, `append_..(`)
#             live ONLY in the shared authority + their defining modules.
#        (c2) the shared persist authority is *called*
#             (`seed_epoch_lineage::persist_seed_epoch_consensus_inputs(`) ONLY
#             by the verified-bootstrap composition sites.
#   (d)  CN-CINPUT-03 consume-side fence on the lifecycle forge path.
#
# `#[cfg(test)]` modules and line comments are stripped before grepping.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CRATES="$REPO_ROOT/crates"

SHARED_AUTH="$CRATES/ade_runtime/src/seed_epoch_lineage.rs"
GENESIS_COMP="$CRATES/ade_runtime/src/genesis_bootstrap.rs"
MITHRIL_COMP="$CRATES/ade_runtime/src/mithril_bootstrap.rs"
ADMISSION_COMP="$CRATES/ade_node/src/admission/bootstrap.rs"
MERGE_MOD="$CRATES/ade_runtime/src/seed_consensus_merge.rs"
PROVENANCE_MOD="$CRATES/ade_runtime/src/seed_consensus_provenance.rs"
PRODUCE_MODE="$CRATES/ade_node/src/produce_mode.rs"

FAILED=0
print_fail() { echo "FAIL: $1"; FAILED=1; }

# Strip the #[cfg(test)] module (everything from the test attribute to EOF)
# and line comments, so guards only see production code.
strip_for_grep() {
    awk '
        /^#\[cfg\(test\)\]/ { in_test=1 }
        in_test { next }
        { line=$0; sub(/\/\/.*$/, "", line); print line }
    ' "$1"
}

for f in "$SHARED_AUTH" "$GENESIS_COMP" "$MITHRIL_COMP" "$ADMISSION_COMP" "$MERGE_MOD" "$PROVENANCE_MOD" "$PRODUCE_MODE"; do
    if [[ ! -f "$f" ]]; then
        print_fail "missing expected source $f"
    fi
done
if (( FAILED != 0 )); then
    echo "FAIL: ci_check_consensus_input_provenance"
    exit 1
fi

# --- Guard (a): POSITIVE — the single shared populator authority holds the
# put + merge + encode + append primitives (the populator lives in one place).
AUTH_BODY="$(strip_for_grep "$SHARED_AUTH")"
if ! echo "$AUTH_BODY" | grep -qE '\.put_seed_epoch_consensus_inputs\('; then
    print_fail "shared populator authority seed_epoch_lineage.rs does not call .put_seed_epoch_consensus_inputs( — the sidecar populator must live at the single shared authority"
fi
if ! echo "$AUTH_BODY" | grep -qE '\bmerge_seed_epoch_consensus_inputs\('; then
    print_fail "shared populator authority seed_epoch_lineage.rs does not build via merge_seed_epoch_consensus_inputs("
fi
if ! echo "$AUTH_BODY" | grep -qE '\bencode_seed_epoch_consensus_inputs\('; then
    print_fail "shared populator authority seed_epoch_lineage.rs does not encode via the A1 sole encoder encode_seed_epoch_consensus_inputs("
fi
if ! echo "$AUTH_BODY" | grep -qE '\bappend_seed_epoch_provenance\('; then
    print_fail "shared populator authority seed_epoch_lineage.rs does not append the WAL provenance entry via append_seed_epoch_provenance( (A3a)"
fi

# --- Guard (a2): POSITIVE — each verified-bootstrap composition site reaches
# the populator ONLY via the shared persist authority.
for comp in "$GENESIS_COMP" "$MITHRIL_COMP" "$ADMISSION_COMP"; do
    body="$(strip_for_grep "$comp")"
    if ! echo "$body" | grep -qE 'seed_epoch_lineage::persist_seed_epoch_consensus_inputs\('; then
        print_fail "composition site $(basename "$comp") does not call seed_epoch_lineage::persist_seed_epoch_consensus_inputs( — the verified bootstrap must persist the lineage via the shared authority"
    fi
done

# --- Guard (b): NEGATIVE forge-time fence — produce_mode names none of the
# sidecar build/put/encode/append/persist tokens. produce_mode owns the
# forge-time consensus-input path (import_live_consensus_inputs +
# pool_distr_view_from_consensus_inputs + --consensus-inputs-path), so fencing
# this file fences that whole path.
FORGE_BODY="$(strip_for_grep "$PRODUCE_MODE")"
FORGE_FENCED_RE='(put_seed_epoch_consensus_inputs|merge_seed_epoch_consensus_inputs|encode_seed_epoch_consensus_inputs|append_seed_epoch_provenance|persist_seed_epoch_consensus_inputs|SeedEpochConsensusInputsImported|SeedEpochConsensusInputs)'
if echo "$FORGE_BODY" | grep -qE "$FORGE_FENCED_RE"; then
    print_fail "forge-time produce_mode.rs references a seed-epoch sidecar token (CN-CINPUT-02): the forge-time consensus-inputs path must not build, put, or persist the sidecar — $(echo "$FORGE_BODY" | grep -nE "$FORGE_FENCED_RE" | head -n1)"
fi

# --- Guard (c): GLOBAL containment (data-flow-resistant). Scan every
# production (test-stripped) Rust file.
#   (c1) the populate primitives (.put_../merge_../append_..) live ONLY in the
#        shared authority + their defining modules (seed_consensus_merge defines
#        the builder; seed_consensus_provenance defines the A3a appender).
#   (c2) the shared persist authority is *called*
#        (`seed_epoch_lineage::persist_seed_epoch_consensus_inputs(`) ONLY by the
#        verified-bootstrap composition sites — closing the "hidden caller
#        anywhere in the tree" class. The `fn persist_..(` definition lives in
#        seed_epoch_lineage.rs and has no `seed_epoch_lineage::` qualifier, so it
#        is not a call.
while IFS= read -r -d '' rsfile; do
    body="$(strip_for_grep "$rsfile")"
    # (c1) populate primitives
    case "$rsfile" in
        *"/seed_epoch_lineage.rs"|*"/seed_consensus_merge.rs"|*"/seed_consensus_provenance.rs")
            ;;
        *)
            if echo "$body" | grep -qE '\.put_seed_epoch_consensus_inputs\('; then
                print_fail "production file $rsfile calls .put_seed_epoch_consensus_inputs( outside the shared populator authority (CN-CINPUT-02)"
            fi
            if echo "$body" | grep -qE '\bmerge_seed_epoch_consensus_inputs\('; then
                print_fail "production file $rsfile references merge_seed_epoch_consensus_inputs( outside the shared populator authority (CN-CINPUT-02)"
            fi
            if echo "$body" | grep -qE '\bappend_seed_epoch_provenance\('; then
                print_fail "production file $rsfile references append_seed_epoch_provenance( outside the shared populator authority (CN-CINPUT-02 / A3a)"
            fi
            ;;
    esac
    # (c2) the shared persist authority call
    case "$rsfile" in
        *"/seed_epoch_lineage.rs"|*"/genesis_bootstrap.rs"|*"/mithril_bootstrap.rs"|*"/admission/bootstrap.rs")
            ;;
        *)
            if echo "$body" | grep -qE 'seed_epoch_lineage::persist_seed_epoch_consensus_inputs\('; then
                print_fail "production file $rsfile calls seed_epoch_lineage::persist_seed_epoch_consensus_inputs( outside the verified-bootstrap composition sites (CN-CINPUT-02)"
            fi
            ;;
    esac
done < <(find "$CRATES" -name '*.rs' -type f -print0)

# --- Guard (d): CN-CINPUT-03 — CONSUME-side fence on the lifecycle forge
# path (PHASE4-N-F-C L5). The node-lifecycle forge handoff (`node_sync.rs`,
# `forge_one_from_recovered`) must derive its leadership PoolDistrView ONLY
# from the recovered surface — projected via `from_seed_epoch_consensus_inputs`
# — and must NOT reach for the forge-time operator bundle path, nor fabricate
# the recovered record. This is the consume-side mirror of the populate-side
# containment above: DC-CINPUT-02b (consume the recovered surface) +
# CN-CINPUT-03 (no bundle / no shape-swap on the forge path).
NODE_SYNC="$CRATES/ade_node/src/node_sync.rs"
if [[ ! -f "$NODE_SYNC" ]]; then
    print_fail "missing expected source $NODE_SYNC (CN-CINPUT-03)"
else
    SYNC_BODY="$(strip_for_grep "$NODE_SYNC")"
    # POSITIVE: the forge path projects from the recovered surface.
    if ! echo "$SYNC_BODY" | grep -qE '\bfrom_seed_epoch_consensus_inputs\('; then
        print_fail "node_sync.rs forge path must derive the leadership view via from_seed_epoch_consensus_inputs( — the recovered surface is the sole consensus-input source on the lifecycle forge path (DC-CINPUT-02b)"
    fi
    # NEGATIVE: no bundle / cold tokens on the lifecycle forge path.
    for tok in 'import_live_consensus_inputs' 'pool_distr_view_from_consensus_inputs' 'consensus_inputs_path' 'InMemoryChainDb'; do
        if echo "$SYNC_BODY" | grep -qE "$tok"; then
            print_fail "node_sync.rs forge path references a forbidden bundle/cold token: $tok — the lifecycle forge base must come from the recovered surface, never a forge-time bundle (CN-CINPUT-03)"
        fi
    done
    # NO SHAPE-SWAP: the forge path must RECEIVE the recovered
    # SeedEpochConsensusInputs via the recovered BootstrapState and project it —
    # never CONSTRUCT a `SeedEpochConsensusInputs { ... }` literal itself.
    if echo "$SYNC_BODY" | grep -qE '\bSeedEpochConsensusInputs[[:space:]]*\{'; then
        print_fail "node_sync.rs forge path constructs a SeedEpochConsensusInputs { ... } literal — the forge path must receive the recovered record via BootstrapState and project it, never fabricate it (no shape-swap, CN-CINPUT-03)"
    fi
fi

# Sanity floor: the positive-guard target files must exist as named (guards
# against a path-typo that would make a positive scan vacuously pass).
if ! echo "$SHARED_AUTH" | grep -qE '/seed_epoch_lineage\.rs$'; then
    print_fail "internal: SHARED_AUTH no longer matches seed_epoch_lineage.rs"
fi
if ! echo "$GENESIS_COMP" | grep -qE '/genesis_bootstrap\.rs$'; then
    print_fail "internal: GENESIS_COMP no longer matches genesis_bootstrap.rs"
fi
if ! echo "$MITHRIL_COMP" | grep -qE '/mithril_bootstrap\.rs$'; then
    print_fail "internal: MITHRIL_COMP no longer matches mithril_bootstrap.rs"
fi
if ! echo "$ADMISSION_COMP" | grep -qE '/admission/bootstrap\.rs$'; then
    print_fail "internal: ADMISSION_COMP no longer matches admission/bootstrap.rs"
fi

if (( FAILED == 0 )); then
    echo "OK: SeedEpochConsensusInputs sidecar populated only via the single shared seed_epoch_lineage authority, called only by the verified-bootstrap composition sites; forge-time path fenced (CN-CINPUT-02 + CN-CINPUT-03)"
fi
exit $FAILED
