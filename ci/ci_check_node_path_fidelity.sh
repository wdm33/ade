#!/usr/bin/env bash
set -uo pipefail

# PHASE4-N-F-G-D S1 — node path fidelity (CN-REHEARSAL-FIDELITY-01 clause 1).
#
# The C1 private-testnet dry-run MUST exercise the SAME --mode node accepted-block
# path as the preview/preprod bounty pass. G-D therefore introduces NO new CLI flag
# and NO from-genesis consensus-inputs constructor: the C1 dry-run differs from the
# preprod pass only in operator INPUTS (a private genesis whose stake allocation
# makes Ade win slots fast) and the evidence LABEL (S2) — never in code. "Fast
# slots" is operator setup, not a private-only code path. A condition that would
# fail on preprod is a shared-path bug to fix in the shared path, never special-cased.
#
# Guards:
#   (a) the cli.rs argv flag-literal set equals the pinned closed allow-list — G-D
#       adds no flag (a private-only / venue flag would change the set and trip).
#   (b) no from-genesis consensus-inputs constructor exists, AND node_lifecycle.rs
#       sources the forge base's consensus inputs only via the shared
#       import_live_consensus_inputs (the same authority the preprod pass uses).

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CLI="$REPO_ROOT/crates/ade_node/src/cli.rs"
NODE_LIFECYCLE="$REPO_ROOT/crates/ade_node/src/node_lifecycle.rs"
CRATES="$REPO_ROOT/crates"

FAIL=0
print_fail() { echo "FAIL (node path fidelity): $1"; FAIL=1; }

[[ -f "$CLI" ]] || print_fail "missing $CLI"
[[ -f "$NODE_LIFECYCLE" ]] || print_fail "missing $NODE_LIFECYCLE"

# --- guard (a): cli.rs flag set == pinned closed allow-list ------------------
# Pinned baseline (sorted, unique). G-D adds NO flag. A future LEGITIMATE flag
# (from a non-G-D cluster) updates this baseline with explicit review; within G-D
# the set is frozen — a private-only / venue flag (e.g. --private-net,
# --from-genesis, --devnet, --rehearsal) would change the set and trip this guard.
# PHASE4-N-AN triage (2026-06-11): baseline extended (with review) for two
# legitimately-added, path-PRESERVING flags from later clusters —
# --participant-venue (N-AI, the sigma=0 participant role; the --mode node admit
# path is UNCHANGED, mirroring the pinned --single-producer-venue) and
# --convergence-evidence-path (N-AJ, an emit-only evidence sink, like --evidence-log).
# The path-diverging private flags above stay excluded.
PINNED_FLAGS="$(cat <<'EOF'
--chain-db
--cold-skey
--consensus-inputs-path
--convergence-evidence-path
--evidence-log
--genesis-file
--genesis-hash
--genesis-path
--json-seed
--kes-skey
--listen
--log
--max-slots
--mithril-manifest-path
--mode
--network
--network-magic
--opcert
--out-file
--participant-venue
--peer
--period-idx
--seed-block-hash
--seed-file
--seed-point-slot
--single-producer-venue
--snapshot-dir
--snapshot-store
--tip-read-timeout-secs
--vrf-skey
--wal-dir
EOF
)"

if [[ -f "$CLI" ]]; then
    CURRENT_FLAGS="$(grep -oE '"--[a-zA-Z-]+" =>' "$CLI" | sed -E 's/ =>//' | tr -d '"' | sort -u)"
    DIFF="$(diff <(echo "$PINNED_FLAGS") <(echo "$CURRENT_FLAGS") || true)"
    if [[ -n "$DIFF" ]]; then
        print_fail "cli.rs flag set diverged from the pinned closed allow-list (G-D must add no flag; a private-only / venue flag is forbidden). diff (< pinned / > current):
$DIFF"
    fi
fi

# --- guard (b): no from-genesis consensus-inputs constructor -----------------
# A fn whose name carries BOTH "genesis" and "consensus" is a from-genesis
# consensus-inputs constructor — the private-only path G-D must NOT build (N0).
# The shared import_live_consensus_inputs (no "genesis" in the name) is the only
# consensus-inputs authority. Line comments are stripped first so prose naming the
# forbidden construct (in tests / doc-comments) cannot self-trip.
BAD_CTOR="$(grep -rE 'fn ' "$CRATES" --include=*.rs \
    | sed -E 's://.*$::' \
    | grep -E 'fn +[a-zA-Z0-9_]*(genesis[a-zA-Z0-9_]*consensus|consensus[a-zA-Z0-9_]*genesis)[a-zA-Z0-9_]*' \
    || true)"
if [[ -n "$BAD_CTOR" ]]; then
    print_fail "a from-genesis consensus-inputs constructor exists (private-only path forbidden; the --mode node path must source consensus inputs via the shared import_live_consensus_inputs):
$BAD_CTOR"
fi

# Positive: the node lifecycle populates consensus inputs via the shared importer.
if [[ -f "$NODE_LIFECYCLE" ]] && ! grep -q 'import_live_consensus_inputs' "$NODE_LIFECYCLE"; then
    print_fail "node_lifecycle.rs does not use the shared import_live_consensus_inputs (the --mode node path must source consensus inputs via the same authority as the preprod pass)"
fi

if (( FAIL == 0 )); then
    echo "OK (node path fidelity): cli.rs flag set matches the pinned closed allow-list (29 flags incl. the DC-NODE-18 --single-producer-venue; the DC-NODE-21-retired --adoption-cert-path is gone); no from-genesis consensus-inputs constructor; node path sources consensus inputs via the shared import_live_consensus_inputs (CN-REHEARSAL-FIDELITY-01 clause 1)."
fi
exit $FAIL
