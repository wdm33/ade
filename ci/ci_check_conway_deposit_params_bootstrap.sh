#!/usr/bin/env bash
set -uo pipefail

# DC-CINPUT-07: Conway deposit-parameter bootstrap authority.
#
# The Conway-only deposit params (drep_deposit / gov_action_deposit / drep_activity) are DECODED from the
# certified Mithril snapshot's Conway curPParams (positions 27/28/29) into native bootstrap authority, never
# defaulted; threaded into the EpochAccumulator seed; bound into the accumulator fingerprint; and the
# accumulator schema version rejects prior stores. Missing/malformed fails closed with a structured error.
# This is the fix for the native-bootstrap blocker where a governance-active epoch boundary fail-closed
# `CertApply(ValidationEnvironment(MissingDRepActivityParam))` because the seed carried
# `conway_deposit_params = None`.
#
# Mechanical enforcement (IDD principle 10): grep the source so the fix cannot silently regress to a
# defaulted / None deposit param on the native bootstrap path.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
ASSEMBLY="$REPO_ROOT/crates/ade_runtime/src/mithril_native_assembly.rs"
DECODER="$REPO_ROOT/crates/ade_ledger/src/ledgerdb_state.rs"
ACC="$REPO_ROOT/crates/ade_ledger/src/epoch_accumulator.rs"
REG="$REPO_ROOT/docs/ade-invariant-registry.toml"

FAILED=0
print_fail() { echo "FAIL: $1"; FAILED=1; }

for f in "$ASSEMBLY" "$DECODER" "$ACC" "$REG"; do
    [[ -f "$f" ]] || print_fail "missing expected file $f"
done
[[ $FAILED -eq 0 ]] || exit 1

# Strip comment-only lines so prose describing the prohibition does not trip a negative grep.
strip_comments() { grep -v '^[[:space:]]*//' || true; }

# Restrict a file to its NON-test lines (everything before the first `#[cfg(test)]`). Test fixtures may
# legitimately construct deposit params from literals; the authority rule constrains the production path.
prod_only() { awk '/#\[cfg\(test\)\]/{exit} {print}' "$1"; }

# (1) The native bootstrap assembly must NOT seed a None / defaulted deposit param. The production ledger
#     construction binds `conway_deposit_params: Some(s1a.conway_deposit_params.clone())` (the decoded value).
if prod_only "$ASSEMBLY" | strip_comments | grep -q 'conway_deposit_params:[[:space:]]*None'; then
    print_fail "(1) mithril_native_assembly bootstrap path seeds conway_deposit_params: None (must import from the certified snapshot)"
fi
if ! grep -q 'conway_deposit_params:[[:space:]]*Some(s1a\.conway_deposit_params\.clone())' "$ASSEMBLY"; then
    print_fail "(1) mithril_native_assembly does not thread the decoded Some(s1a.conway_deposit_params) into the assembled ledger"
fi

# (2) No fixture-literal drep_activity (e.g. a hardcoded Some(20)) on the production assembly path — the value
#     comes ONLY from the decoded snapshot, never a constant written next to the field.
if prod_only "$ASSEMBLY" | strip_comments | grep -Eq 'drep_activity:[[:space:]]*[0-9]'; then
    print_fail "(2) mithril_native_assembly production path assigns a literal drep_activity (must read the decoded snapshot value)"
fi

# (3) The BLUE canonical decoder READS the three deposit params from the verified curPParams positions
#     27/28/29 (never skips them) — the single-decoder authority, not a parallel parser.
for c in CONWAY_PP_GOV_ACTION_DEPOSIT_INDEX CONWAY_PP_DREP_DEPOSIT_INDEX CONWAY_PP_DREP_ACTIVITY_INDEX; do
    grep -q "const $c" "$DECODER" || print_fail "(3) decoder missing the curPParams index constant $c"
done
grep -q 'nn_read_u64(d, o, "pp.dRepActivity")' "$DECODER" \
    || print_fail "(3) decoder does not READ dRepActivity (idx 29) with nn_read_u64 (fail-closed on wrong type)"

# (4) The accumulator schema version is a versioned gate (>= 2), and the structured fail-closed error variant
#     exists so a Conway store with a missing deposit param is rejected (never loaded as a defaulted set). The
#     exact version advances with later bumps (v3 added the bootstrap-RUPD feeSS, DC-EPOCH-23); the gate here
#     only requires it stay past the pre-deposit-param v1 -- the MissingConwayDepositParams check below is the
#     deposit-param-specific fail-close.
grep -Eq 'EPOCH_ACCUMULATOR_SCHEMA_VERSION:[[:space:]]*u32[[:space:]]*=[[:space:]]*([2-9]|[1-9][0-9]+)' "$ACC" \
    || print_fail "(4) EPOCH_ACCUMULATOR_SCHEMA_VERSION is not a versioned gate >= 2 (the bump that rejects prior stores)"
grep -q 'MissingConwayDepositParams' "$ACC" \
    || print_fail "(4) the structured EpochAccumulatorCodecError::MissingConwayDepositParams variant is missing"
grep -q 'conway_deposit_params.is_none()' "$ACC" \
    || print_fail "(4) decode_epoch_accumulator does not fail closed on a Conway accumulator with None deposit params"

# (5) The invariant is in the registry.
grep -q 'id = "DC-CINPUT-07"' "$REG" || print_fail "(5) DC-CINPUT-07 is not in the invariant registry"

if [[ $FAILED -eq 0 ]]; then
    echo "PASS: Conway deposit params decoded from the certified snapshot into bootstrap authority, versioned + fail-closed (DC-CINPUT-07)"
    exit 0
else
    exit 1
fi
