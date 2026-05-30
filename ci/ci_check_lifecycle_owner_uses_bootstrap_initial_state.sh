#!/usr/bin/env bash
set -uo pipefail

# PHASE4-N-F-C — node lifecycle owner single-authority gate (CN-NODE-01).
#
# The cluster names exactly ONE production recovered-state lifecycle
# owner (the module carrying the `PHASE4-N-F-C-LIFECYCLE-OWNER` marker).
# CN-NODE-01 requires that the owner obtain initial state SOLELY via the
# single `bootstrap_initial_state` authority — never a parallel
# storage-init / cold-start path.
#
# Wiring state (L3): the owner has TWO wired arms.
#   - FirstRun (L2) routes initial state through the single authority
#     INDIRECTLY, via `bootstrap_from_mithril_snapshot(` (which calls
#     `bootstrap_initial_state` internally).
#   - WarmStart (L3) calls `bootstrap_initial_state(` DIRECTLY with
#     `SeedEpochConsensusSource::RequiredFromRecoveredProvenance` — the
#     production warm-start recovery path. This is still the SINGLE
#     authority (same `pub fn`), not a second one.
# So this gate now requires BOTH positive calls and keeps the cold/genesis
# fences; it also fences the L3-specific overclaim (`recover_node_state`)
# and contains the `RequiredFromRecoveredProvenance` constructor to the
# owner.
#
# Guards (comments + `#[cfg(test)]` stripped before the negative greps so
# the owner's own doc-comment prose cannot false-trip):
#   (a) exactly one module carries the PHASE4-N-F-C-LIFECYCLE-OWNER marker;
#   (b) the owner has NO parallel/cold storage-init: no InMemoryChainDb, no
#       materialize_rolled_back_state(  (it delegates materialization to the
#       authority; `bootstrap_initial_state(` is now PERMITTED — it is the
#       single authority the L3 warm-start arm calls directly);
#   (c-neg) the owner has NO genesis/bundle/cold/tip fallback: no
#       bootstrap_from_conway_genesis, no `genesis_initial: Some` cold seed,
#       no seed_graft, no tip_bundle (WarmStart writes `genesis_initial: None`);
#   (c-pos-first) the FirstRun arm calls `bootstrap_from_mithril_snapshot(`;
#   (c-pos-warm) the WarmStart arm calls `bootstrap_initial_state(` AND
#       constructs `RequiredFromRecoveredProvenance`;
#   (c-overclaim) the owner does NOT call `recover_node_state(` (that helper
#       passes NotRequired and would NOT recover the sidecar — see the L3
#       slice doc §9.3);
#   (d) the single bootstrap authority still holds: exactly one
#       `pub fn bootstrap_initial_state` in ade_runtime/src/bootstrap.rs;
#   (e) the diagnostic produce path stays diagnostic: produce_mode.rs does
#       not pass RequiredFromRecoveredProvenance;
#   (f) containment: `RequiredFromRecoveredProvenance` is CONSTRUCTED only in
#       the owner and the bootstrap authority definition — nowhere else.

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

# --- guard (b): no parallel / cold storage-init in the owner ----------------
# The owner must NOT open a cold InMemoryChainDb or materialize state itself.
# `bootstrap_initial_state(` is NO LONGER forbidden: the L3 warm-start arm
# calls the single authority directly. Building the *seed* `LedgerState` from
# the extracted UTxO (FirstRun) is NOT a parallel init either — so
# `LedgerState::new` stays permitted. The real invariant is enforced by "no
# InMemoryChainDb" + "no materialize" + the positive composer/authority
# requirements + the genesis/cold fences below.
for tok in 'InMemoryChainDb' 'materialize_rolled_back_state\('; do
    if echo "$OWNER_BODY" | grep -qE "$tok"; then
        print_fail "owner ($(basename "$OWNER")) contains a parallel/cold initial-state path token: $tok — the owner must delegate materialization to the single bootstrap_initial_state authority (CN-NODE-01)."
    fi
done

# --- guard (c-neg): no genesis/cold/graft fallback in the owner -------------
# Mithril-only first run + recovered-only warm start: NO genesis composer, NO
# cold genesis seed, NO tip-bundle seed-graft. The WarmStart arm passes
# `genesis_initial: None`, so the forbidden form is a cold seed
# `genesis_initial: Some`. The documented cardano-cli extraction
# (`import_live_consensus_inputs` / `import_cardano_cli_json_utxo`) IS
# permitted on the FirstRun arm — Mithril-bound bootstrap extraction (L2
# §9.5), not a forge-time input (forge-time fence is CN-CINPUT-03 / L5).
for tok in 'bootstrap_from_conway_genesis' 'genesis_initial: Some' 'seed_graft' 'tip_bundle'; do
    if echo "$OWNER_BODY" | grep -qE "$tok"; then
        print_fail "owner ($(basename "$OWNER")) references a forbidden genesis/graft token: $tok — the node lifecycle is Mithril-first-run + recovered-warm-start, fail-closed (no cold/genesis/bundle fallback)"
    fi
done

# --- guard (c-pos-first): FirstRun arm calls the Mithril composer -----------
if ! echo "$OWNER_BODY" | grep -qE 'bootstrap_from_mithril_snapshot\('; then
    print_fail "owner ($(basename "$OWNER")) must call bootstrap_from_mithril_snapshot( on the FirstRun arm — Mithril-only first-run bootstrap routes initial state through the single authority (L2 / CN-NODE-01)"
fi

# --- guard (c-pos-warm): WarmStart arm calls the single authority -----------
# The WarmStart recovery path calls `bootstrap_initial_state(` directly with
# the recovered provenance source. Both tokens must be present.
if ! echo "$OWNER_BODY" | grep -qE 'bootstrap_initial_state\('; then
    print_fail "owner ($(basename "$OWNER")) must call bootstrap_initial_state( on the WarmStart arm — production warm-start recovery routes through the single authority (L3 / CN-NODE-01)"
fi
if ! echo "$OWNER_BODY" | grep -qE 'RequiredFromRecoveredProvenance'; then
    print_fail "owner ($(basename "$OWNER")) WarmStart arm must use SeedEpochConsensusSource::RequiredFromRecoveredProvenance — the recovered seed-epoch surface is restored + verified by the authority (L3 / DC-CINPUT-01)"
fi

# --- guard (c-overclaim): owner does NOT call recover_node_state ------------
# `recover_node_state` hardcodes NotRequired and would NOT recover the
# sidecar; it is the test/capability helper. The owner builds the warm-start
# path itself (L3 §9.3). Calling it would be an overclaim.
if echo "$OWNER_BODY" | grep -qE 'recover_node_state\('; then
    print_fail "owner ($(basename "$OWNER")) must NOT call recover_node_state( — it passes NotRequired and would not recover the seed-epoch sidecar; the owner builds the production warm-start path via bootstrap_initial_state(RequiredFromRecoveredProvenance) (L3 §9.3)"
fi

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

# --- guard (f): RequiredFromRecoveredProvenance construction containment ----
# The recovered-provenance source may be CONSTRUCTED only in the owner module
# and the bootstrap authority that defines the variant. Any other production
# construction site is a second consume/authorize path (forbidden). We scan
# production (comment- + test-stripped) code across ade_node + ade_runtime,
# count files that construct the variant, and require the set to be exactly
# {owner, bootstrap.rs}. `SeedEpochConsensusSource::RequiredFromRecoveredProvenance`
# with a trailing `(` is the construction form; the enum *definition* and the
# match arm in bootstrap.rs live in that same allowed file.
allowed_owner="$OWNER"
allowed_boot="$BOOT"
mapfile -t ctor_files < <(
    for f in $(grep -rl 'RequiredFromRecoveredProvenance' "$NODE" "$RT" --include='*.rs' 2>/dev/null || true); do
        if strip_for_grep "$f" | grep -qE 'RequiredFromRecoveredProvenance\s*\('; then
            echo "$f"
        fi
    done
)
for f in "${ctor_files[@]:-}"; do
    [[ -z "$f" ]] && continue
    if [[ "$f" != "$allowed_owner" && "$f" != "$allowed_boot" ]]; then
        print_fail "RequiredFromRecoveredProvenance constructed outside the owner/authority: $f — only the lifecycle owner (recovery) and bootstrap.rs may construct it (CN-NODE-01 / L3)"
    fi
done

if (( FAILED == 0 )); then
    echo "OK (lifecycle owner): single marked owner ($(basename "$OWNER")) routes FirstRun via bootstrap_from_mithril_snapshot and WarmStart via bootstrap_initial_state(RequiredFromRecoveredProvenance); no parallel/cold init, no genesis/bundle/cold fallback, no recover_node_state overclaim; bootstrap_initial_state remains the sole authority; RequiredFromRecoveredProvenance construction contained to owner+authority"
fi
exit $FAILED
