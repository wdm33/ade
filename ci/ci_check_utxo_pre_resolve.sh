#!/usr/bin/env bash
set -uo pipefail

# MEM-OPT-UTXO-DISK S2b (pre-resolve enumeration): ONE authoritative, era-aware,
# CLOSED extractor of every TxIn a tx's validation can read (spend + collateral +
# reference inputs), returning a DETERMINISTIC sorted set (BTreeSet, never a
# HashSet). The Conway validator USES it, so the pre-resolved set == the validated
# set by construction -- there is no hidden lazy-fetch path that could diverge from
# what validation reads (the BLUE/RED boundary stays intact when wired).

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"; cd "$REPO_ROOT"
FAILED=0; fail() { echo "FAIL: $1"; FAILED=1; }
P=crates/ade_ledger/src/pre_resolve.rs
C=crates/ade_ledger/src/conway.rs
D=docs/clusters/MEM-OPT-UTXO-DISK/S2b-pre-resolve.md

# (1) the extractor exists + returns a deterministic sorted set (BTreeSet, not HashSet).
grep -qF 'pub fn collect_required_txins(body: &ConwayTxBody) -> BTreeSet<TxIn>' "$P" \
    || fail "collect_required_txins is not the (ConwayTxBody -> BTreeSet<TxIn>) form"
if grep -qE 'HashSet[<:]|HashMap[<:]|collections::Hash(Set|Map)' "$P"; then
    fail "pre_resolve uses a Hash* container -- the required set must be deterministic (BTreeSet)"
fi

# (2) all THREE UTxO classes are enumerated (spend U collateral U reference).
grep -qF 'body.inputs' "$P" || fail "spend inputs not enumerated"
grep -qF 'body.collateral_inputs' "$P" || fail "collateral inputs not enumerated"
grep -qF 'body.reference_inputs' "$P" \
    || fail "reference inputs not enumerated (the script-context dependency)"

# (3) the Conway validator USES the single authority (no inline all_inputs rebuild).
grep -qF 'crate::pre_resolve::collect_required_txins(body)' "$C" \
    || fail "the Conway validator does not resolve through the single pre-resolve authority"

# (4) the closed-era completeness proof + the determinism proof + the dependency table.
grep -qE 'fn conway_required_set_includes_spend_collateral_and_reference' "$P" \
    || fail "the closed-era completeness proof (all three classes) is missing"
grep -qE 'fn required_set_is_canonically_sorted' "$P" \
    || fail "the determinism/sorted proof is missing"
test -f "$D" || fail "the per-era dependency table (S2b-pre-resolve.md) is missing"
grep -qiE 'reference inputs.*script.context|script.context.*reference|script/context' "$D" \
    || fail "the dependency table does not document the Babbage/Conway reference-input / script-context dependency"

# (5) the resolved-view WORKING-SET (GREEN) with EXPLICIT transitions only (seed +
#     apply_tx_acceptance) -- not a general-purpose mutable UTxO map.
grep -qE 'pub struct WorkingSet' "$P" || fail "WorkingSet (the resolved view) is missing"
grep -qE 'fn seed_required_from_anchor' "$P" || fail "WorkingSet::seed_required_from_anchor missing"
grep -qE 'fn apply_tx_acceptance' "$P" || fail "WorkingSet::apply_tx_acceptance (the sequential transition) missing"
grep -qF 'impl UtxoStore for WorkingSet' "$P" || fail "WorkingSet does not impl UtxoStore (BLUE's resolved view)"
if grep -qE 'pub fn (insert|remove|get_mut|clear)\b' "$P"; then
    fail "WorkingSet exposes general-purpose mutation -- only seed + apply_tx_acceptance are allowed"
fi

# (6) the RED anchor-read path -- the ONLY place the anchor is read for validation.
grep -qE 'fn resolve_required' crates/ade_runtime/src/chaindb/utxo_anchor.rs \
    || fail "UtxoAnchor::resolve_required (the WorkingSet seed) is missing"

# (7) the wiring proofs: resolved-view == full-UTxO verdict; intra-block produced-then-
#     spent; missing input fails closed with a structured error.
grep -qE 'fn resolved_view_verdict_equals_full_utxo_verdict' "$C" \
    || fail "resolved-view == full-UTxO verdict-equivalence proof missing"
grep -qE 'fn resolved_view_missing_input_fails_closed' "$C" \
    || fail "missing-input fail-closed proof missing"
grep -qE 'fn working_set_intra_block_produced_then_spent' "$P" \
    || fail "intra-block produced-then-spent proof missing"

# (8) GUARDRAIL: the BLUE/GREEN ledger crate NEVER reaches the storage backend.
if grep -qE '^\s*redb\b' crates/ade_ledger/Cargo.toml; then
    fail "ade_ledger depends on redb -- BLUE/GREEN must never reach the storage backend"
fi
if grep -rqE '\bUtxoAnchor\b' crates/ade_ledger/src/; then
    fail "ade_ledger references UtxoAnchor -- BLUE/GREEN must never hold the storage backend"
fi

# (9) the bounded read cache is a SEPARATE later slice -- NOT introduced here.
if grep -qiE '\b(cache|lru)\b' "$P"; then
    fail "a cache appears in pre_resolve -- the bounded read cache is a separate later slice"
fi

if (( FAILED == 0 )); then
    echo "OK: pre-resolve enumeration + resolved-view wiring (S2b; single closed extractor; WorkingSet seed+apply transitions; RED resolve_required; verdict-equivalent to full UTxO; intra-block + fail-closed proven; no redb/anchor in BLUE; no cache)"
fi
exit $FAILED
