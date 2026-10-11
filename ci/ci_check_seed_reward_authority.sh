#!/usr/bin/env bash
set -euo pipefail

# SLICE LV-2 (DC-EPOCH-41) -- the reward paid at the seed->seed+1 boundary has exactly ONE owner,
# selected by a typed fact about the seed snapshot's nesRu, never by an empty map.
#
# ORACLE (cardano-ledger): the reward update for boundary E->E+1 is computed during epoch E from the
# go snapshot, nesBprev, ssFee and the reserves, and applied at the boundary. nesRu only records how
# far the computation has got: SNothing (not started), Pulsing, Complete. The same reward is paid in
# all three. Ade read SNothing as an EMPTY COMPLETED update, which switched its native calculation
# off for that boundary -- nobody paid (preprod 304->305: pool_pot=0 where cardano paid
# 416,110,584,109).
#
# Asserts:
#  (a) the snapshot decoder returns a typed sum; SNothing yields NotYetComputed and can never yield
#      an empty tuple / empty Complete; ssFee is DECODED, not skipped;
#  (b) the durable record is a closed sum (Imported | Native), schema 4, the variant inside the
#      commitment, and the pre-LV-2 Imported-only codec pair is not public;
#  (c) the accumulator holds Option<SeedRewardAuthority> and its seed-boundary transition matches
#      BOTH variants; the Native arm feeds the imported nesBprev and fee_ss, and reduces the fee pot;
#  (d) a Native seed boundary with blocks and a zero pot is a terminal;
#  (e) NO path applies an imported reward delta except from an Imported record: the window-replay
#      derive has a typed terminal for Native, and apply_bootstrap_reward_deltas has exactly the two
#      known call sites;
#  (f) first-run seals Native for NotYetComputed and never constructs an Imported record from it;
#  (g) the store-semantics version moved with the rule;
#  (h) the CE tests exist, and they RAN and passed.
#
# Repo-root-relative. Mirrors the other ci_check_*.sh gates.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

DECODE="crates/ade_ledger/src/ledgerdb_state.rs"
RECORD="crates/ade_ledger/src/bootstrap_reward_update.rs"
ACC="crates/ade_ledger/src/epoch_accumulator.rs"
CANDIDATE="crates/ade_node/src/epoch_candidate.rs"
FIRSTRUN="crates/ade_node/src/native_firstrun.rs"
DRIVER="crates/ade_runtime/src/chaindb/reduced_window_driver.rs"
SEMANTICS="crates/ade_ledger/src/store_semantics.rs"

for f in "$DECODE" "$RECORD" "$ACC" "$CANDIDATE" "$FIRSTRUN" "$DRIVER" "$SEMANTICS"; do
    if [[ ! -f "$f" ]]; then
        echo "FAIL (seed reward authority): $f not found"
        exit 1
    fi
done

FAILED=0
fail() { echo "FAIL (seed reward authority): $1"; FAILED=1; }

# Production body only: DROP every `#[cfg(test)] mod ... { ... }` block wherever it sits (one of
# these files has test modules in the middle, with production code after them -- truncating at the
# first would blind the gate to that code), and strip line comments so commentary naming a token
# cannot satisfy or trip a grep.
prod_body() {
    awk '
        /^#\[cfg\(test\)\]$/ { pend = $0; holding = 1; next }
        holding && /^#\[allow\(/ { pend = pend "\n" $0; next }
        holding && /^mod .*\{$/ { skipping = 1; holding = 0; next }
        holding { print pend; holding = 0 }
        skipping { if ($0 ~ /^}$/) skipping = 0; next }
        { print }
    ' "$1" | sed -E 's://.*::'
}

isolate() {
    awk -v pat="$1" '
        $0 ~ pat { capture=1 }
        capture { print }
        capture && /^}/ { exit }
    ' <<<"$2"
}

DECODE_PROD="$(prod_body "$DECODE")"
RECORD_PROD="$(prod_body "$RECORD")"
ACC_PROD="$(prod_body "$ACC")"
CANDIDATE_PROD="$(prod_body "$CANDIDATE")"
FIRSTRUN_PROD="$(prod_body "$FIRSTRUN")"
for v in DECODE_PROD RECORD_PROD ACC_PROD CANDIDATE_PROD FIRSTRUN_PROD; do
    if [[ -z "${!v}" ]]; then
        echo "FAIL (seed reward authority): could not isolate a production body ($v)"
        exit 1
    fi
done

# --- (a) the decoder ------------------------------------------------------------
grep -qE '^pub enum SnapshotRewardUpdate \{' <<<"$DECODE_PROD" \
    || fail "SnapshotRewardUpdate is not a pub enum in $DECODE"
RU_FN="$(isolate '^fn read_reward_update_deltas\(' "$DECODE_PROD")"
if [[ -z "$RU_FN" ]]; then
    fail "could not isolate read_reward_update_deltas"
else
    grep -qE '\) -> R<SnapshotRewardUpdate>' <<<"$RU_FN" \
        || fail "read_reward_update_deltas does not return the typed SnapshotRewardUpdate -- a tuple of deltas is how 'absent' became 'empty'"
    SM_ARM="$(awk '/if sm == 0 \{/{c=1} c{print} c&&/^    \}/{exit}' <<<"$RU_FN")"
    grep -qE 'SnapshotRewardUpdate::NotYetComputed \{ fee_ss \}' <<<"$SM_ARM" \
        || fail "the SNothing arm does not return NotYetComputed carrying fee_ss"
    grep -qE 'Complete|BTreeMap::new\(\)|Coin\(0\)' <<<"$SM_ARM" \
        && fail "the SNothing arm builds a Complete / empty / zero value -- an absent reward update must never be a zero-valued present one"
fi
grep -qE 'let fee_ss = Coin\(nn_read_u64\(d, o, "ssFee"\)\?\);' <<<"$DECODE_PROD" \
    || fail "ssFee is not decoded where the snapshots are read"
grep -qE 'skip_item\(d, o\)\?;\s*$' <<<"$(grep -nE 'ssFee' "$DECODE" || true)" \
    && fail "ssFee is still skipped"
grep -qE 'read_reward_update_deltas\(d, o, fee_ss\)' <<<"$DECODE_PROD" \
    || fail "the decoded ssFee is not handed to the nesRu decoder"

# --- (b) the durable record -------------------------------------------------------
grep -qE '^pub enum SeedRewardAuthority \{' <<<"$RECORD_PROD" \
    || fail "SeedRewardAuthority is not a pub enum"
ENUM_BODY="$(isolate '^pub enum SeedRewardAuthority \{' "$RECORD_PROD")"
VARIANTS="$(grep -cE '^\s+(Imported|Native)\(' <<<"$ENUM_BODY" || true)"
(( VARIANTS == 2 )) || fail "SeedRewardAuthority has $VARIANTS of the two expected variants (Imported, Native)"
grep -qE 'non_exhaustive|Default' <<<"$(grep -B3 -E '^pub enum SeedRewardAuthority' <<<"$RECORD_PROD")" \
    && fail "SeedRewardAuthority must be closed: no #[non_exhaustive], no Default"
SCHEMA="$(grep -oE 'BOOTSTRAP_RUPD_SCHEMA_VERSION: u32 = [0-9]+' "$RECORD" | grep -oE '[0-9]+$')"
(( SCHEMA >= 4 )) || fail "BOOTSTRAP_RUPD_SCHEMA_VERSION is $SCHEMA, expected >= 4 -- a v3 record cannot say whether its empty deltas were computed or absent"
COMMIT_FN="$(isolate '^fn commit_variant\(' "$RECORD_PROD")"
grep -qE 'write_uint_canonical\(&mut domained, variant\)' <<<"$COMMIT_FN" \
    || fail "the variant tag is not inside the commitment preimage"
for f in bootstrap_rupd_commitment native_seed_reward_commitment; do
    grep -qE 'commit_variant\(VARIANT_(IMPORTED|NATIVE), &body\)' <<<"$(isolate "^pub fn $f\\(" "$RECORD_PROD")" \
        || fail "$f does not commit through commit_variant"
done
grep -qE '^pub fn (encode|decode)_bootstrap_reward_update\(' <<<"$RECORD_PROD" \
    && fail "the Imported-only codec pair is public -- a caller decoding with it would read a Native record as an error-or-absent, the confusion this slice removes"
grep -qE '^pub fn encode_seed_reward_authority\(' <<<"$RECORD_PROD" \
    || fail "encode_seed_reward_authority is missing"
grep -qE '^pub fn decode_seed_reward_authority\(' <<<"$RECORD_PROD" \
    || fail "decode_seed_reward_authority is missing"
DEC_FN="$(isolate '^pub fn decode_seed_reward_authority\(' "$RECORD_PROD")"
VER_LINE="$(grep -nE 'UnknownVersion' <<<"$DEC_FN" | head -1 | cut -d: -f1)"
VAR_LINE="$(grep -nE 'let variant = ' <<<"$DEC_FN" | head -1 | cut -d: -f1)"
if [[ -z "$VER_LINE" || -z "$VAR_LINE" ]] || (( VER_LINE >= VAR_LINE )); then
    fail "the decoder does not gate the version before reading the variant -- a v3 record must be refused as UnknownVersion"
fi

# --- (c) the accumulator ------------------------------------------------------------
grep -qE 'pub pending_reward_update: Option<SeedRewardAuthority>,' <<<"$ACC_PROD" \
    || fail "EpochAccumulator.pending_reward_update is not Option<SeedRewardAuthority>"
TRANS_FN="$(isolate '^fn cross_epoch_boundary_transition\(' "$ACC_PROD")"
if [[ -z "$TRANS_FN" ]]; then
    fail "could not isolate cross_epoch_boundary_transition"
else
    grep -qE 'Some\(SeedRewardAuthority::Imported\(rupd\)\) => \{' <<<"$TRANS_FN" \
        || fail "the seed-boundary transition has no Imported arm"
    grep -qE 'Some\(SeedRewardAuthority::Native\(native\)\) => \{' <<<"$TRANS_FN" \
        || fail "the seed-boundary transition has no Native arm"
    NATIVE_ARM="$(awk '/Some\(SeedRewardAuthority::Native\(native\)\) => \{/{c=1} c{print} c&&/^        \}$/{exit}' <<<"$TRANS_FN")"
    grep -qE 'native_seed_reward_commitment\(' <<<"$NATIVE_ARM" \
        || fail "the Native arm does not verify the record's commitment"
    grep -qE 'checked_sub\(native\.fee_ss\.0\)' <<<"$NATIVE_ARM" \
        || fail "the Native arm does not reduce the fee pot by fee_ss -- the imported pot still contains the previous epoch's fees"
    grep -qE 'NativeFromSnapshot \{ fee_ss: native\.fee_ss \}' <<<"$NATIVE_ARM" \
        || fail "the Native arm does not hand the snapshot's fee_ss to the reward inputs"
    grep -qE 'apply_bootstrap_reward_deltas|BTreeMap::new\(\)' <<<"$NATIVE_ARM" \
        && fail "the Native arm applies a reward delta / builds an empty map -- there is no imported result to apply"
    FEED="$(awk '/SeedBoundaryRewardInputs::NativeFromSnapshot \{ fee_ss \} => \{/{c=1} c{print} c&&/^        \}$/{exit}' <<<"$TRANS_FN")"
    grep -qE 'block_production = acc\.prev_block_production\.clone\(\)' <<<"$FEED" \
        || fail "the Native seed boundary is not fed the imported nesBprev"
    grep -qE 'epoch_fees = fee_ss;' <<<"$FEED" \
        || fail "the Native seed boundary is not fed fee_ss as its fee input"

    # --- (d) the zero-pot terminal ---------------------------------------------------
    grep -qE 'accounting\.pool_reward_pot == 0' <<<"$TRANS_FN" \
        || fail "the zero-pot check is gone"
    grep -qE 'LedgerTransitionError::SeedBoundaryZeroRewardPot' <<<"$TRANS_FN" \
        || fail "a Native seed boundary with blocks and a zero pot is no longer a terminal"
fi

# --- (e) no empty delta for a Native seed, on any path -------------------------------
DERIVE_FN="$(isolate '^pub fn derive_candidate\(' "$CANDIDATE_PROD")"
if [[ -z "$DERIVE_FN" ]]; then
    fail "could not isolate derive_candidate"
else
    grep -qE '\(Some\(SeedRewardAuthority::Native\(_\)\), true\) => \{' <<<"$DERIVE_FN" \
        || fail "derive_candidate has no arm for a Native seed on the seed window"
    NAT="$(awk '/\(Some\(SeedRewardAuthority::Native\(_\)\), true\) => \{/{c=1} c{print} c&&/^        \}$/{exit}' <<<"$DERIVE_FN")"
    grep -qE 'return Err\(CandidateDeriveError::SeedRewardOwnedByNativeCalculation' <<<"$NAT" \
        || fail "derive_candidate does not fail closed for a Native seed -- it must never proceed with an empty or absent delta"
    grep -qE '\(Some\(SeedRewardAuthority::Imported\(rupd\)\), true\) => Some\(&rupd\.reward_delta\)' <<<"$DERIVE_FN" \
        || fail "derive_candidate does not take the delta from the Imported record"
fi
# apply_bootstrap_reward_deltas: the accumulator's Imported arm and the window driver, nothing else.
APPLY_SITES="$(grep -rnE 'apply_bootstrap_reward_deltas\(' crates --include='*.rs' \
    | grep -vE '^\S+:[0-9]+:\s*//' | grep -vE 'fn apply_bootstrap_reward_deltas' \
    | grep -vE '/tests/' | awk -F: '{print $1}' | sort | uniq -c | awk '{print $2":"$1}' | tr '\n' ' ')"
PROD_APPLY=0
for f in "$ACC" "$DRIVER"; do
    n="$(prod_body "$f" | grep -cE 'apply_bootstrap_reward_deltas\(' || true)"
    (( n == 1 )) || fail "$f production body calls apply_bootstrap_reward_deltas $n times, expected exactly 1"
    PROD_APPLY=$((PROD_APPLY + n))
done
OTHER_APPLY="$(grep -rlE 'apply_bootstrap_reward_deltas\(' crates --include='*.rs' \
    | grep -vE "^($ACC|$DRIVER|crates/ade_ledger/src/delegation.rs)$" || true)"
[[ -z "$OTHER_APPLY" ]] || fail "apply_bootstrap_reward_deltas has a new caller: $(tr '\n' ' ' <<<"$OTHER_APPLY") -- every caller must take its delta from an Imported record ($APPLY_SITES)"

# --- (f) first-run seals the right variant --------------------------------------------
NYC="$(awk '/SnapshotRewardUpdate::NotYetComputed \{ fee_ss \} => \{/{c=1} c{print} c&&/^            \}$/{exit}' <<<"$FIRSTRUN_PROD")"
if [[ -z "$NYC" ]]; then
    fail "native first-run has no NotYetComputed arm"
else
    grep -qE 'SeedRewardAuthority::Native\(' <<<"$NYC" \
        || fail "native first-run does not seal a Native record for NotYetComputed"
    grep -qE 'BootstrapRewardUpdate \{|Imported' <<<"$NYC" \
        && fail "native first-run builds an Imported record for NotYetComputed -- the exact defect"
fi
grep -qE 'encode_seed_reward_authority\(&rupd\)' <<<"$FIRSTRUN_PROD" \
    || fail "native first-run does not persist through encode_seed_reward_authority"

# --- (g) the store-semantics version moved ---------------------------------------------
STORE="$(grep -oE 'STORE_SEMANTICS_VERSION: u32 = [0-9]+' "$SEMANTICS" | grep -oE '[0-9]+$')"
(( STORE >= 9 )) || fail "STORE_SEMANTICS_VERSION is $STORE, expected >= 9 -- a store seeded from an SNothing snapshot by an earlier binary is wrong in every epoch and cannot be repaired in place"

# --- (h) the CE tests exist, ran, and passed ---------------------------------------------
check_tests() { # crate, file, names...
    local crate="$1" file="$2"; shift 2
    local filter=""
    for t in "$@"; do
        grep -qF "fn $t" "$file" || fail "the named CE test is missing from $file: $t"
    done
    local out ran=0
    for t in "$@"; do
        out="$(cargo test -p "$crate" --lib "$t" 2>&1 || true)"
        if grep -qE "^test .*${t} \.\.\. ok$" <<<"$out"; then
            ran=$((ran + 1))
        else
            fail "CE test did not run and pass: $crate $t"
        fi
    done
    echo "$ran"
}
RAN_A="$(check_tests ade_ledger "$DECODE" \
    snothing_decodes_to_not_yet_computed_carrying_ss_fee \
    an_empty_complete_update_is_not_not_yet_computed \
    a_pulsing_update_still_fails_closed | tail -1)"
RAN_B="$(check_tests ade_ledger "$RECORD" \
    both_variants_round_trip_as_themselves \
    a_v3_record_is_refused_not_reinterpreted \
    the_variant_is_inside_the_commitment | tail -1)"
RAN_C="$(check_tests ade_ledger "$ACC" \
    native_seed_boundary_runs_the_native_reward_on_nesbprev_and_fee_ss \
    native_seed_boundary_reward_depends_on_fee_ss \
    native_seed_boundary_rotates_exactly_the_seed_epochs_own_fees \
    native_seed_boundary_with_blocks_and_a_zero_pot_is_a_terminal \
    accumulator_codec_round_trips_a_native_seed_authority \
    pending_reward_update_applied_once_at_seed_boundary \
    seed_boundary_reduces_fee_pot_by_delta_fees | tail -1)"
RAN_D="$(check_tests ade_node "$CANDIDATE" \
    seed2_window_with_a_native_seed_authority_is_a_typed_terminal \
    seed2_window_without_rupd_fails_closed | tail -1)"
# `fail` inside the $(...) subshells above cannot set FAILED here; re-derive it from the counts.
[[ "$RAN_A" == "3" ]] || { echo "FAIL (seed reward authority): decoder CE tests: $RAN_A"; FAILED=1; }
[[ "$RAN_B" == "3" ]] || { echo "FAIL (seed reward authority): record CE tests: $RAN_B"; FAILED=1; }
[[ "$RAN_C" == "7" ]] || { echo "FAIL (seed reward authority): accumulator CE tests: $RAN_C"; FAILED=1; }
[[ "$RAN_D" == "2" ]] || { echo "FAIL (seed reward authority): candidate CE tests: $RAN_D"; FAILED=1; }

if (( FAILED == 0 )); then
    echo "OK (seed reward authority): the nesRu decoder returns a typed sum, SNothing is NotYetComputed carrying the decoded ssFee and never an empty Complete; the durable record is the closed sum SeedRewardAuthority at schema $SCHEMA with the variant inside the commitment and no public Imported-only codec; the accumulator's seed boundary matches both variants, feeds a Native seed the imported nesBprev and fee_ss, reduces the fee pot, and treats a zero pot with blocks as a terminal; the window-replay derive fails closed for a Native seed and apply_bootstrap_reward_deltas has only its two known callers; first-run seals Native for NotYetComputed; store semantics $STORE; 15 CE tests ran and passed (DC-EPOCH-41)"
fi
exit $FAILED
