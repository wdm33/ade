#!/usr/bin/env bash
set -uo pipefail

# PHASE4-N-F-A A2 — seed-epoch consensus-input provenance containment
# (candidate CN-CINPUT-02).
#
# The persisted `SeedEpochConsensusInputs` sidecar may be populated ONLY
# on the verified-bootstrap composition path, through the anchor-keyed
# `SnapshotStore` surface. The bounty-primary forge-time path
# (`produce_mode`, `import_live_consensus_inputs*`,
# `pool_distr_view_from_consensus_inputs`, `--consensus-inputs-path`)
# must not build or `put` the sidecar.
#
# Mechanical, data-flow-resistant guards (N-Z
# `ci_check_mithril_seed_point_independence.sh` style — containment, not
# a bypassable RHS grep):
#   (a) POSITIVE: each verified-bootstrap composition site
#       (`genesis_bootstrap.rs`, `mithril_bootstrap.rs`) calls
#       `.put_seed_epoch_consensus_inputs(` — the populator lives where it
#       must.
#   (b) NEGATIVE forge-time fence: the forge-time composer
#       `produce_mode.rs` (which owns `import_live_consensus_inputs`,
#       `pool_distr_view_from_consensus_inputs`, and the
#       `--consensus-inputs-path` flag) names NONE of the sidecar
#       build/put/encode tokens.
#   (c) GLOBAL containment: across all production (test-stripped) Rust,
#       any *call* to `.put_seed_epoch_consensus_inputs(` or
#       `merge_seed_epoch_consensus_inputs(` outside the allowed
#       composition set is a FAIL — closing the "hidden second populator
#       anywhere in the tree" class.
#
# `#[cfg(test)]` modules and line comments are stripped before grepping.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CRATES="$REPO_ROOT/crates"

GENESIS_COMP="$CRATES/ade_runtime/src/genesis_bootstrap.rs"
MITHRIL_COMP="$CRATES/ade_runtime/src/mithril_bootstrap.rs"
MERGE_MOD="$CRATES/ade_runtime/src/seed_consensus_merge.rs"
PROVENANCE_MOD="$CRATES/ade_runtime/src/seed_consensus_provenance.rs"
PRODUCE_MODE="$CRATES/ade_node/src/produce_mode.rs"

FAILED=0
print_fail() { echo "FAIL: $1"; FAILED=1; }

# Strip the #[cfg(test)] module (everything from the test attribute to
# EOF) and line comments, so guards only see production code.
strip_for_grep() {
    awk '
        /^#\[cfg\(test\)\]/ { in_test=1 }
        in_test { next }
        { line=$0; sub(/\/\/.*$/, "", line); print line }
    ' "$1"
}

for f in "$GENESIS_COMP" "$MITHRIL_COMP" "$MERGE_MOD" "$PROVENANCE_MOD" "$PRODUCE_MODE"; do
    if [[ ! -f "$f" ]]; then
        print_fail "missing expected source $f"
    fi
done
if (( FAILED != 0 )); then
    echo "FAIL: ci_check_consensus_input_provenance"
    exit 1
fi

# --- Guard (a): POSITIVE — the populator lives at each composition site.
for comp in "$GENESIS_COMP" "$MITHRIL_COMP"; do
    body="$(strip_for_grep "$comp")"
    if ! echo "$body" | grep -qE '\.put_seed_epoch_consensus_inputs\('; then
        print_fail "composition site $(basename "$comp") does not call .put_seed_epoch_consensus_inputs( — the sidecar populator must live at the verified-bootstrap composition site"
    fi
    # And it must build the record via the A1 sole encoder + the GREEN merge.
    if ! echo "$body" | grep -qE '\bmerge_seed_epoch_consensus_inputs\('; then
        print_fail "composition site $(basename "$comp") does not build via merge_seed_epoch_consensus_inputs("
    fi
    if ! echo "$body" | grep -qE '\bencode_seed_epoch_consensus_inputs\('; then
        print_fail "composition site $(basename "$comp") does not encode via the A1 sole encoder encode_seed_epoch_consensus_inputs("
    fi
    # A3a: and it must append the WAL provenance entry via the shared
    # helper (the put → append commit-point ordering lives here).
    if ! echo "$body" | grep -qE '\bappend_seed_epoch_provenance\('; then
        print_fail "composition site $(basename "$comp") does not append the WAL provenance entry via append_seed_epoch_provenance( (A3a)"
    fi
done

# --- Guard (b): NEGATIVE forge-time fence — produce_mode names none of
# the sidecar build/put/encode tokens. produce_mode owns the forge-time
# consensus-input path (import_live_consensus_inputs +
# pool_distr_view_from_consensus_inputs + --consensus-inputs-path), so
# fencing this file fences that whole path.
FORGE_BODY="$(strip_for_grep "$PRODUCE_MODE")"
# A3a extends the fence with the WAL provenance tokens: the
# forge-time path must not build/put the sidecar NOR append its
# WAL provenance entry.
FORGE_FENCED_RE='(put_seed_epoch_consensus_inputs|merge_seed_epoch_consensus_inputs|encode_seed_epoch_consensus_inputs|append_seed_epoch_provenance|SeedEpochConsensusInputsImported|SeedEpochConsensusInputs)'
if echo "$FORGE_BODY" | grep -qE "$FORGE_FENCED_RE"; then
    print_fail "forge-time produce_mode.rs references a seed-epoch sidecar token (CN-CINPUT-02): the forge-time consensus-inputs path must not build or put the sidecar — $(echo "$FORGE_BODY" | grep -nE "$FORGE_FENCED_RE" | head -n1)"
fi

# --- Guard (c): GLOBAL containment (data-flow-resistant). Scan every
# production (test-stripped) Rust file. Any *call* to the populator,
# the builder, or the A3a WAL-provenance appender outside the allowed
# composition set means a second populate path could exist somewhere
# in the tree.
#   Allowed sites:
#     - genesis_bootstrap.rs / mithril_bootstrap.rs (the two composers)
#     - seed_consensus_merge.rs (defines + may self-reference the builder)
#     - seed_consensus_provenance.rs (defines the A3a appender; A3a)
# A `fn put_seed_epoch_consensus_inputs(` *definition* (the trait decl +
# the two SnapshotStore impls) is NOT a populate call — match only method
# *calls* (`.put_seed_epoch_consensus_inputs(`) for the put token.
# `append_seed_epoch_provenance` is a free fn (no leading-dot call form
# to distinguish from its `fn` definition), so its defining module
# `seed_consensus_provenance.rs` is allow-listed, exactly as the builder's
# module is — the call is then contained to the two composers.
ALLOWED_RE='/(genesis_bootstrap|mithril_bootstrap|seed_consensus_merge|seed_consensus_provenance)\.rs$'

while IFS= read -r -d '' rsfile; do
    case "$rsfile" in
        *"/genesis_bootstrap.rs"|*"/mithril_bootstrap.rs"|*"/seed_consensus_merge.rs"|*"/seed_consensus_provenance.rs")
            continue
            ;;
    esac
    body="$(strip_for_grep "$rsfile")"
    # Method-call to the populator (leading dot) outside the allowed set.
    if echo "$body" | grep -qE '\.put_seed_epoch_consensus_inputs\('; then
        print_fail "production file $rsfile calls .put_seed_epoch_consensus_inputs( outside the verified-bootstrap composition sites (CN-CINPUT-02)"
    fi
    # Any reference to the GREEN builder outside the allowed set.
    if echo "$body" | grep -qE '\bmerge_seed_epoch_consensus_inputs\('; then
        print_fail "production file $rsfile references merge_seed_epoch_consensus_inputs( outside the verified-bootstrap composition sites (CN-CINPUT-02)"
    fi
    # A3a: any reference to the WAL-provenance appender outside the
    # allowed set — the provenance append must live only at the two
    # composers (its definition lives in the allow-listed module).
    if echo "$body" | grep -qE '\bappend_seed_epoch_provenance\('; then
        print_fail "production file $rsfile references append_seed_epoch_provenance( outside the verified-bootstrap composition sites (CN-CINPUT-02 / A3a)"
    fi
done < <(find "$CRATES" -name '*.rs' -type f -print0)

# --- Guard (d): CN-CINPUT-03 — CONSUME-side fence on the lifecycle forge
# path (PHASE4-N-F-C L5). The node-lifecycle forge handoff
# (`node_sync.rs`, `forge_one_from_recovered`) must derive its leadership
# PoolDistrView ONLY from the recovered surface — projected via
# `from_seed_epoch_consensus_inputs` — and must NOT reach for the
# forge-time operator bundle path, nor fabricate the recovered record. This
# is the consume-side mirror of the populate-side containment above:
# DC-CINPUT-02b (consume the recovered surface) + CN-CINPUT-03 (no bundle /
# no shape-swap on the forge path).
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
    # SeedEpochConsensusInputs via the recovered BootstrapState and project
    # it — never CONSTRUCT a `SeedEpochConsensusInputs { ... }` literal
    # itself (that would be fabricating the recovered type from arbitrary
    # fields and feeding it to the projection). The populate-side guards
    # (a–c) already restrict who may BUILD/put the sidecar; this is the
    # consume-side mirror, scoped to the lifecycle forge module (CN-CINPUT-03).
    if echo "$SYNC_BODY" | grep -qE '\bSeedEpochConsensusInputs[[:space:]]*\{'; then
        print_fail "node_sync.rs forge path constructs a SeedEpochConsensusInputs { ... } literal — the forge path must receive the recovered record via BootstrapState and project it, never fabricate it (no shape-swap, CN-CINPUT-03)"
    fi
fi

# Sanity floor for guard (c): the allow-list regex must actually match
# the two composers (guards against an accidental path-typo that would
# make the global scan vacuously pass).
if ! echo "$GENESIS_COMP" | grep -qE "$ALLOWED_RE"; then
    print_fail "internal: ALLOWED_RE no longer matches genesis_bootstrap.rs"
fi
if ! echo "$MITHRIL_COMP" | grep -qE "$ALLOWED_RE"; then
    print_fail "internal: ALLOWED_RE no longer matches mithril_bootstrap.rs"
fi
if ! echo "$PROVENANCE_MOD" | grep -qE "$ALLOWED_RE"; then
    print_fail "internal: ALLOWED_RE no longer matches seed_consensus_provenance.rs"
fi

if (( FAILED == 0 )); then
    echo "OK: SeedEpochConsensusInputs sidecar populated only at the verified-bootstrap composition sites; forge-time path fenced (CN-CINPUT-02)"
fi
exit $FAILED
