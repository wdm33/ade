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

if (( FAILED == 0 )); then
    echo "OK: pre-resolve dependency enumeration (S2b; single era-aware closed extractor; spend+collateral+reference; deterministic sorted set; the Conway validator resolves through it)"
fi
exit $FAILED
