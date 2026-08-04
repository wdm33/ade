#!/usr/bin/env bash
set -uo pipefail

# PREPROD-ENTRY-AUTHORITY P6 (DC-STORE-10/11) -- no authoritative store reader may bypass the
# semantics gate, and no "trust me" remediation may be added.
#
# P4 (e1de7a2e) proved a durable store can be structurally valid, fully decodable, and semantically
# stale. The gate that catches that is only as good as its coverage: an authority-bearing store that
# opens WITHOUT checking its marker reintroduces the hole silently. Mechanical enforcement
# (IDD principle 10):
#   (A) every AuthorityArtifact variant has a store that checks it on open.
#   (B) each of the three authority stores calls check_store_semantics_version in its open path.
#   (C) each of the three stores STAMPS the marker on a fresh artifact (otherwise (B) rejects
#       everything and the node can never bootstrap).
#   (D) RemediationAction has EXACTLY ONE variant -- the type must remain unable to express "stamp it"
#       or "continue anyway". This is the constitutional bit: no override may be added quietly.
#   (E) no CLI flag or env var weakens the gate.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CRATES="$REPO_ROOT/crates"
SEM="$CRATES/ade_ledger/src/store_semantics.rs"
CHAINDB="$CRATES/ade_runtime/src/chaindb/persistent.rs"
ACC="$CRATES/ade_runtime/src/chaindb/epoch_accumulator_store.rs"
RED="$CRATES/ade_runtime/src/chaindb/reduced_utxo_checkpoint.rs"

FAILED=0
print_fail() { echo "FAIL: $1"; FAILED=1; }

for f in "$SEM" "$CHAINDB" "$ACC" "$RED"; do
    [[ -e "$f" ]] || print_fail "missing expected path $f"
done

# (A) the artifact enum and the checking stores must stay in step.
VARIANTS=$(awk '/^pub enum AuthorityArtifact/{f=1;next} f&&/^}/{exit} f' "$SEM" \
    | grep -cE '^\s{4}[A-Z][A-Za-z]+,' || true)
if [[ "$VARIANTS" -ne 3 ]]; then
    print_fail "(A) AuthorityArtifact has $VARIANTS variants but 3 stores are wired -- a new authority-bearing store must ALSO check its marker on open, and this gate must be extended to it"
fi

# (B) each store checks ON OPEN -- REACHABILITY, not mere presence.
#
# The first version of this check only grepped that the file mentioned check_store_semantics_version
# anywhere. Its own negative test caught the hole: deleting the call from `open` left the checking
# function defined-but-unwired, the file still matched, and the gate passed. Same failure shape the
# RF-1 gate documents ("deleting the post-commit call COMPILES CLEAN and every unit test still
# passes -- only the structural gate catches an unwired call"). So walk the two hops explicitly:
# `open` must call its initializer, and that initializer must call the checker.
fn_body() { awk -v pat="$2" '$0 ~ pat {f=1} f {print} f && /^    \}$/ {exit}' "$1"; }

for triple in \
    "chaindb:$CHAINDB:ChainDb:init_or_check_schema" \
    "accumulator:$ACC:EpochAccumulator:init_or_check_store_semantics" \
    "reduced:$RED:ReducedCheckpoint:init_or_check_store_semantics"; do
    IFS=':' read -r name path variant initfn <<<"$triple"

    OPEN_BODY=$(fn_body "$path" '^    pub fn open\(')
    if [[ -z "$OPEN_BODY" ]]; then
        print_fail "(B) $name has no 'pub fn open(' -- cannot prove the gate is reachable"
    elif ! grep -q "$initfn" <<<"$OPEN_BODY"; then
        print_fail "(B) $name::open does not call $initfn -- the semantics gate is UNWIRED (it may still exist in the file, unreachable)"
    fi

    INIT_BODY=$(fn_body "$path" "fn $initfn\\(")
    if [[ -z "$INIT_BODY" ]]; then
        print_fail "(B) $name has no $initfn body to inspect"
    elif ! grep -q 'check_store_semantics_version' <<<"$INIT_BODY"; then
        print_fail "(B) $name::$initfn does not call check_store_semantics_version"
    fi

    grep -q "AuthorityArtifact::$variant" "$path" \
        || print_fail "(B) $name does not identify itself as AuthorityArtifact::$variant"
done

# (C) each store stamps a fresh artifact, so bootstrap is still possible.
for pair in "chaindb:$CHAINDB" "accumulator:$ACC" "reduced:$RED"; do
    name="${pair%%:*}"; path="${pair#*:}"
    grep -q 'STORE_SEMANTICS_VERSION' "$path" \
        || print_fail "(C) $name never writes STORE_SEMANTICS_VERSION -- a fresh store would be unmarked and rejected forever"
done

# (D) the remediation surface stays closed at exactly one variant.
ACTIONS=$(awk '/^pub enum RemediationAction/{f=1;next} f&&/^}/{exit} f' "$SEM" \
    | grep -cE '^\s{4}[A-Z][A-Za-z]+,' || true)
if [[ "$ACTIONS" -ne 1 ]]; then
    print_fail "(D) RemediationAction has $ACTIONS variants, expected exactly 1 (RebootstrapRequired). Adding a stamp/override variant recreates the P4 failure mode with an official-looking button."
fi
# Every QUALIFIED use of the remediation enum, anywhere in the tree, must name the single sanctioned
# variant. This is checked on variant USAGE rather than by grepping for words like "stamp": the module
# docs and the operator message both legitimately say "there is no stamp path, by design", and a gate
# that cannot survive its own invariant being written down is the wrong gate.
BAD_USES=$(grep -rn 'RemediationAction::' "$CRATES" --include=*.rs \
    | grep -v 'RemediationAction::RebootstrapRequired' || true)
if [[ -n "$BAD_USES" ]]; then
    print_fail "(D) a remediation other than RebootstrapRequired is referenced: $BAD_USES"
fi

# (E) no flag/env may weaken it.
if grep -rniE 'skip[_-]?semantics|ignore[_-]?semantics|force[_-]?store[_-]?semantics|ADE_SKIP_STORE_SEMANTICS' \
    "$CRATES" --include=*.rs; then
    print_fail "(E) a bypass flag / env var for the semantics gate exists"
fi

if [[ "$FAILED" -ne 0 ]]; then
    echo "ci_check_store_semantics_gate: FAILED"
    exit 1
fi
echo "ci_check_store_semantics_gate: OK (all 3 authority stores gated; remediation surface closed)"
