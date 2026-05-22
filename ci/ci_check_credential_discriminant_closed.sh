#!/usr/bin/env bash
set -uo pipefail

# DC-LEDGER-10: credential key/script discriminant fidelity. Credential identity
# is a closed sum {KeyHash, ScriptHash}, never a tag-erased Hash28. This gate
# defends that the collapse cannot silently return — CI-enforced, not only
# compiler-checked + tested:
#
#   1. StakeCredential is a closed 2-variant ENUM, not the old tuple struct
#      `StakeCredential(pub Hash28)` (crates/ade_types/src/shelley/cert.rs).
#   2. Both era decoders PRESERVE the key/script tag — they map it to
#      KeyHash/ScriptHash and have no tag-discard form `let (_cred_type|_tag`
#      (crates/ade_codec/src/{shelley,conway}/cert.rs).
#   3. No tuple-construction `StakeCredential(<hash>)` coercion remains on the
#      BLUE authority path — credentials are built only via ::KeyHash / ::ScriptHash
#      (or decode). Bare-hash coercion would re-introduce the collapse.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"

TYPE="$REPO_ROOT/crates/ade_types/src/shelley/cert.rs"
SHELLEY="$REPO_ROOT/crates/ade_codec/src/shelley/cert.rs"
CONWAY="$REPO_ROOT/crates/ade_codec/src/conway/cert.rs"

FAIL=0

for f in "$TYPE" "$SHELLEY" "$CONWAY"; do
    [ -f "$f" ] || { echo "FAIL: expected file missing: $f"; FAIL=1; }
done
[ "$FAIL" -eq 0 ] || exit 1

# 1. closed enum, not the tuple struct.
if ! grep -qE '^pub enum StakeCredential' "$TYPE"; then
    echo "FAIL: StakeCredential is not the closed enum in ade_types (the discriminated type)"
    FAIL=1
fi
if grep -qE 'struct StakeCredential\(' "$TYPE"; then
    echo "FAIL: the tuple-struct StakeCredential(Hash28) shape reappeared — the collapse is back"
    FAIL=1
fi

# 2. both decoders preserve the tag (map to variants, no discard form).
for f in "$SHELLEY" "$CONWAY"; do
    if ! grep -q 'StakeCredential::KeyHash' "$f" || ! grep -q 'StakeCredential::ScriptHash' "$f"; then
        echo "FAIL: $f decode_stake_credential does not map the tag to KeyHash/ScriptHash"
        FAIL=1
    fi
    if grep -qE 'let \(_(cred_type|tag)' "$f"; then
        echo "FAIL: $f drops the credential type tag (let (_cred_type|_tag …)) — must preserve it"
        FAIL=1
    fi
done

# 3. no tuple-construction coercion on the BLUE path.
COERCE=$(grep -rnE 'StakeCredential\(' \
    "$REPO_ROOT/crates/ade_codec/src" \
    "$REPO_ROOT/crates/ade_ledger/src" \
    "$REPO_ROOT/crates/ade_types/src" \
    --include=*.rs 2>/dev/null \
    | grep -v 'StakeCredential::' | grep -v 'enum StakeCredential')
if [ -n "$COERCE" ]; then
    echo "FAIL: tuple-construction StakeCredential(<hash>) coercion on the BLUE path:"
    echo "$COERCE"
    FAIL=1
fi

# 4. committee credential surface stays discriminated (COMMITTEE-CRED-FIDELITY,
#    strengthens DC-LEDGER-10): the committee member map and committee_votes must
#    key/carry the discriminated StakeCredential, never bare Hash28.
STATE="$REPO_ROOT/crates/ade_ledger/src/state.rs"
GOVTYPE="$REPO_ROOT/crates/ade_types/src/conway/governance.rs"
if [ -f "$STATE" ] && ! grep -qE 'pub committee:.*BTreeMap<.*StakeCredential' "$STATE"; then
    echo "FAIL: ConwayGovState.committee is not StakeCredential-keyed (committee member discriminant lost)"
    FAIL=1
fi
if [ -f "$GOVTYPE" ] && ! grep -qE 'pub committee_votes:.*StakeCredential' "$GOVTYPE"; then
    echo "FAIL: GovActionState.committee_votes does not carry StakeCredential (committee voter discriminant lost)"
    FAIL=1
fi

# 5. DRep-voter surface stays discriminated (DREP-VOTE-FIDELITY, strengthens
#    DC-LEDGER-10): drep_votes carries StakeCredential, and DRep stake resolution
#    has NO key/script OR-fallback (a key-hash voter must not tally a script-hash
#    DRep's stake of equal bytes).
GOVLOGIC="$REPO_ROOT/crates/ade_ledger/src/governance.rs"
if [ -f "$GOVTYPE" ] && ! grep -qE 'pub drep_votes:.*StakeCredential' "$GOVTYPE"; then
    echo "FAIL: GovActionState.drep_votes does not carry StakeCredential (DRep voter discriminant lost)"
    FAIL=1
fi
if [ -f "$GOVLOGIC" ] && grep -qE 'DRep::KeyHash\([^)]*\)\)?\s*\.or_else' "$GOVLOGIC"; then
    echo "FAIL: governance.rs has a DRep key/script OR-fallback (cross-resolution) — resolve to the exact variant"
    FAIL=1
fi

# 6. EnactmentEffects.committee_changes stays discriminated (ENACTMENT-COMMITTEE-
#    FIDELITY, strengthens DC-LEDGER-10): the (dormant) committee-enactment
#    effect carries StakeCredential, never bare Hash28 — so committee enactment,
#    when wired, cannot re-collapse the discriminated committee map on write-back.
if [ -f "$GOVLOGIC" ] && ! grep -qE 'pub committee_changes:.*StakeCredential' "$GOVLOGIC"; then
    echo "FAIL: EnactmentEffects.committee_changes is not StakeCredential-typed (committee-enactment re-collapse risk)"
    FAIL=1
fi

# 7. UpdateCommittee enactment surface is structured + discriminated
#    (ENACTMENT-COMMITTEE-WRITEBACK, strengthens DC-LEDGER-10 + DC-EPOCH-01): the
#    GovAction::UpdateCommittee committee members are discriminated StakeCredential
#    (a removed set + an added map), never opaque raw bytes or bare Hash28 — so
#    the epoch-boundary committee write-back cannot re-collapse the committee map.
if [ -f "$GOVTYPE" ]; then
    if ! grep -qE 'removed:[[:space:]]*BTreeSet<StakeCredential>' "$GOVTYPE"; then
        echo "FAIL: GovAction::UpdateCommittee.removed is not BTreeSet<StakeCredential> (committee-member discriminant lost)"
        FAIL=1
    fi
    if ! grep -qE 'added:[[:space:]]*BTreeMap<StakeCredential' "$GOVTYPE"; then
        echo "FAIL: GovAction::UpdateCommittee.added is not BTreeMap<StakeCredential,_> (committee-member discriminant lost)"
        FAIL=1
    fi
fi
#    and the apply site writes the committee back via the pure transition rather
#    than cloning it unchanged (the enactment must be reachable).
if [ -f "$GOVLOGIC" ] && ! grep -qE 'pub fn apply_committee_enactment' "$GOVLOGIC"; then
    echo "FAIL: governance.rs missing apply_committee_enactment (committee write-back not wired)"
    FAIL=1
fi
RULESLOGIC="$REPO_ROOT/crates/ade_ledger/src/rules.rs"
if [ -f "$RULESLOGIC" ] && ! grep -qE 'apply_committee_enactment' "$RULESLOGIC"; then
    echo "FAIL: rules.rs epoch boundary does not call apply_committee_enactment (committee write-back dropped)"
    FAIL=1
fi

if [ "$FAIL" -eq 0 ]; then
    echo "PASS: DC-LEDGER-10 credential discriminant is closed and faithful (committee + DRep-vote + enactment + committee-write-back surfaces)"
fi
exit "$FAIL"
