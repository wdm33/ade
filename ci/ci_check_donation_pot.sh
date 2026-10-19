#!/usr/bin/env bash
set -euo pipefail

# SLICE LV-6 (DC-EPOCH-45) -- a treasury donation is held in one pot and moved to the treasury once, at
# the boundary.
#
# REFERENCE (cardano-ledger-conway-1.17.0.0, node 11.0.1): Conway/Rules/Utxos.hs adds
# `treasuryDonationTxBodyL` to `utxosDonation` only on the IsValid True branch; Conway/Rules/Epoch.hs
# applies POOLREAP, then the enacted treasury withdrawals (accountState1 -> accountState2), then
# `asTreasuryL <>~ (utxosDonation <> fold unclaimed)` (accountState3), then `utxosDonationL .~ zero`.
# Ade kept no donation pot: preprod's two epoch-305 donations (500,000 each) never reached the treasury.
#
# Asserts:
#  (a) EpochState carries `donation_pot`; ONE contribution function, gated on phase-2 validity;
#  (b) exactly one production caller of that function (the accumulator scan), passing the validity
#      gate, and exactly one production credit of the pot (the accumulator fold);
#  (c) no reconstruction: outside the allowed files no production code touches `donation_pot`;
#  (d) order: the boundary adds the pot after POOLREAP and the gov deposit returns, with a typed
#      overflow terminal, and zeroes it; ratification/enactment (governance.rs) never reads the pot;
#  (e) bootstrap imports the pot: the seed decoder reads UTxOState[5] and refuses its absence, the
#      commitment binds it, Mithril assembly carries it; no `donation_pot: Coin(0)` in a bootstrap path;
#  (f) the version gates moved;
#  (g) the CE tests exist and ran.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

STATE="crates/ade_ledger/src/state.rs"
ACC="crates/ade_ledger/src/epoch_accumulator.rs"
RULES="crates/ade_ledger/src/rules.rs"
GOV="crates/ade_ledger/src/governance.rs"
DECODE="crates/ade_ledger/src/ledgerdb_state.rs"
ASSEMBLY="crates/ade_runtime/src/mithril_native_assembly.rs"
FRAMING="crates/ade_ledger/src/snapshot/framing.rs"
SEMANTICS="crates/ade_ledger/src/store_semantics.rs"

for f in "$STATE" "$ACC" "$RULES" "$GOV" "$DECODE" "$ASSEMBLY" "$FRAMING" "$SEMANTICS"; do
    [[ -f "$f" ]] || { echo "FAIL (donation pot): $f not found"; exit 1; }
done

FAILED=0
fail() { echo "FAIL (donation pot): $1"; FAILED=1; }

# Production body: drop EVERY #[cfg(test)] module and strip line comments so commentary cannot satisfy
# or trip a grep.
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
    awk -v pat="$1" '$0 ~ pat { c=1 } c { print } c && /^}/ { exit }' <<<"$2"
}

STATE_PROD="$(prod_body "$STATE")"
ACC_PROD="$(prod_body "$ACC")"
RULES_PROD="$(prod_body "$RULES")"
GOV_PROD="$(prod_body "$GOV")"
DECODE_PROD="$(prod_body "$DECODE")"
ASSEMBLY_PROD="$(prod_body "$ASSEMBLY")"

# --- (a) the field and the one contribution function -----------------------------------------
ES_TY="$(isolate '^pub struct EpochState \{' "$STATE_PROD")"
grep -qE 'pub donation_pot: Coin' <<<"$ES_TY" || fail "(a) EpochState carries no donation_pot"
CONTRIB="$(isolate '^pub fn tx_donation_contribution\(' "$STATE_PROD")"
[[ -n "$CONTRIB" ]] || fail "(a) tx_donation_contribution is not defined in state.rs"
grep -qE '\(true, Some\(d\)\) => d' <<<"$CONTRIB" || fail "(a) the contribution is not gated on validity"
DEFS="$(grep -rhoE 'fn tx_donation_contribution\b' crates/*/src | wc -l || true)"
(( ${DEFS:-0} == 1 )) || fail "(a) tx_donation_contribution is defined $DEFS times, expected 1"

# --- (b) one caller, one credit --------------------------------------------------------------
CALLERS=0
for f in $(grep -rlE 'tx_donation_contribution\(' crates/*/src); do
    n="$(prod_body "$f" | grep -cE 'tx_donation_contribution\(' || true)"
    [[ "$f" == "$STATE" ]] && n=$(( n - 1 ))
    CALLERS=$(( CALLERS + n ))
    if (( n > 0 )) && [[ "$f" != "$ACC" ]]; then fail "(b) $f calls tx_donation_contribution"; fi
done
(( CALLERS == 1 )) || fail "(b) $CALLERS production callers of tx_donation_contribution, expected 1"
SCAN_FN="$(isolate '^fn apply_tx_scan\(' "$ACC_PROD")"
grep -qE '!invalid\.contains\(&index\)' <<<"$(grep -A3 'tx_donation_contribution(' <<<"$SCAN_FN")" \
    || fail "(b) the scan does not pass the phase-2 validity gate to the contribution"
CREDITS="$(grep -cE 'acc\.epoch_state\.donation_pot = Coin\(' <<<"$ACC_PROD" || true)"
(( CREDITS == 1 )) || fail "(b) $CREDITS credits of the pot in the accumulator, expected 1 (the fold)"

# --- (c) no reconstruction -------------------------------------------------------------------
ALLOWED=(
    "crates/ade_ledger/src/state.rs"
    "crates/ade_ledger/src/epoch_accumulator.rs"
    "crates/ade_ledger/src/rules.rs"
    "crates/ade_ledger/src/hfc.rs"
    "crates/ade_ledger/src/snapshot/epoch_state.rs"
    "crates/ade_ledger/src/fingerprint.rs"
    "crates/ade_ledger/src/ledgerdb_state.rs"
    "crates/ade_runtime/src/mithril_native_assembly.rs"
)
for f in $(grep -rlE 'donation_pot' crates/*/src); do
    if prod_body "$f" | grep -qE 'donation_pot'; then
        ok=0
        for a in "${ALLOWED[@]}"; do [[ "$f" == "$a" ]] && ok=1; done
        (( ok == 1 )) || fail "(c) $f touches donation_pot outside the three transitions"
    fi
done

# --- (d) order at the boundary ---------------------------------------------------------------
BFN="$(isolate '^pub fn apply_epoch_boundary_with_registrations\(' "$RULES_PROD")"
[[ -n "$BFN" ]] || fail "(d) could not isolate apply_epoch_boundary_with_registrations"
# `|| true`: under pipefail an absent step would otherwise abort the script with no FAIL line.
L_REAP="$(grep -nE 'saturating_add\(poolreap_to_treasury\)' <<<"$BFN" | head -1 | cut -d: -f1 || true)"
L_GOV="$(grep -nE 'saturating_add\(gov_deposit_to_treasury\)' <<<"$BFN" | head -1 | cut -d: -f1 || true)"
L_POT="$(grep -nE 'state\.epoch_state\.donation_pot' <<<"$BFN" | head -1 | cut -d: -f1 || true)"
# The boundary reads the pot in exactly one place, the step-6 credit, and adds it unscaled.
N_POT_READS="$(grep -cE 'state\.epoch_state\.donation_pot' <<<"$BFN" || true)"
[[ "$N_POT_READS" == "1" ]] \
    || fail "(d) the boundary reads the donation pot $N_POT_READS times; expected exactly one step-6 credit"
grep -qE '^\s*\.checked_add\(state\.epoch_state\.donation_pot\.0\)$' <<<"$BFN" \
    || fail "(d) the step-6 credit does not add the pot exactly once, unscaled"
if [[ -z "$L_REAP" || -z "$L_GOV" || -z "$L_POT" ]]; then
    fail "(d) could not find the POOLREAP / gov-deposit / donation treasury steps"
elif (( L_POT <= L_REAP || L_POT <= L_GOV )); then
    fail "(d) the donation pot is added before POOLREAP or the gov deposit returns"
fi
grep -qE 'GovernanceTerminal::TreasuryOverflow' <<<"$BFN" || fail "(d) no typed overflow terminal for the pot"
grep -qE 'donation_pot: ade_types::tx::Coin\(0\)' <<<"$BFN" || fail "(d) the new epoch's pot is not zeroed"
grep -qE 'donation_pot' <<<"$GOV_PROD" && fail "(d) governance.rs (ratification/enactment) reads donation_pot"

# --- (e) bootstrap imports the pot -----------------------------------------------------------
grep -qE 'let donation_pot = Coin\(nn_read_u64\(d, o, "UTxOState\.donation"\)\?\)' <<<"$DECODE_PROD" \
    || fail "(e) the seed decoder does not read UTxOState[5]"
grep -qE 'has no donation pot' <<<"$DECODE_PROD" || fail "(e) a UTxOState without the pot is not refused"
grep -qE 'commitment-v13' <<<"$DECODE_PROD" || fail "(e) the seed commitment tag is not v13"
grep -qE 'extend_from_slice\(&s\.donation_pot\.0\.to_be_bytes\(\)\)' <<<"$DECODE_PROD" \
    || fail "(e) the seed commitment does not bind the pot"
grep -qE 'donation_pot: s1a\.donation_pot' <<<"$ASSEMBLY_PROD" || fail "(e) Mithril assembly does not carry the pot"
for p in "$DECODE_PROD" "$ASSEMBLY_PROD"; do
    grep -qE 'donation_pot: Coin\(0\)' <<<"$p" && fail "(e) a bootstrap path initialises the pot to Coin(0)"
done
grep -qE 'parse_donation_pot\(&self\.raw_cbor\)' crates/ade_testkit/src/harness/snapshot_loader.rs \
    || fail "(e) the test-kit loader does not read the pot from the snapshot"

# --- (f) versions ----------------------------------------------------------------------------
STORE="$(grep -oE '^pub const STORE_SEMANTICS_VERSION: u32 = [0-9]+' "$SEMANTICS" | grep -oE '[0-9]+$' || true)"
(( ${STORE:-0} >= 11 )) || fail "(f) STORE_SEMANTICS_VERSION is $STORE, expected >= 11"
ACCV="$(grep -oE '^pub const EPOCH_ACCUMULATOR_SCHEMA_VERSION: u32 = [0-9]+' "$ACC" | grep -oE '[0-9]+$' || true)"
(( ${ACCV:-0} >= 6 )) || fail "(f) EPOCH_ACCUMULATOR_SCHEMA_VERSION is $ACCV, expected >= 6"
FRV="$(grep -oE '^pub const SCHEMA_VERSION: u32 = [0-9]+' "$FRAMING" | grep -oE '[0-9]+$' || true)"
(( ${FRV:-0} >= 4 )) || fail "(f) snapshot framing SCHEMA_VERSION is $FRV, expected >= 4"

# --- (g) the CE tests exist and ran ----------------------------------------------------------
LIB_TESTS=(
    'lv6_a_valid_txs_donation_is_credited_and_an_invalid_txs_is_not'
    'lv6_the_real_epoch_305_blocks_carry_500_000_each'
    'lv6_the_real_pair_folds_into_the_pot_and_the_boundary_moves_it'
    'lv6_the_boundary_moves_the_whole_pot_once_and_zeroes_it'
    'lv6_donations_and_unclaimed_deposits_both_reach_the_treasury'
    'lv6_the_pot_never_funds_a_treasury_withdrawal'
    'lv6_a_pot_that_would_overflow_the_treasury_is_a_terminal'
    'codec_rejects_pre_lv6_store_rebootstrap_required'
)
LOUT="$(cargo test -p ade_ledger --lib -- lv6_ codec_rejects_pre_lv6 2>&1 || true)"
for t in "${LIB_TESTS[@]}"; do
    grep -qE "^test .*::$t \.\.\. ok$" <<<"$LOUT" || fail "(g) unit test $t did not run and pass"
done
HOUT="$(cargo test -p ade_ledger --test ledgerdb_nonutxo_hermetic 2>&1 || true)"
for t in 'a_seed_with_donations_imports_its_donation_pot' 'the_seed_commitment_binds_the_donation_pot' \
         'a_utxo_state_without_a_donation_pot_is_refused'; do
    grep -qE "^test $t \.\.\. ok$" <<<"$HOUT" || fail "(g) seed-import test $t did not run and pass"
done
ROUT="$(cargo test -p ade_runtime --lib -- lv6_ mithril_native_assembly 2>&1 || true)"
grep -qE '^test .*lv6_a_donation_pot_survives_a_warm_restart_byte_identically \.\.\. ok$' <<<"$ROUT" \
    || fail "(g) the warm-restart identity test did not run and pass"
grep -qE '^test result: ok\.' <<<"$ROUT" || fail "(g) the ade_runtime LV-6 / assembly tests did not pass"

if (( FAILED == 0 )); then
    echo "OK (donation pot): one pot in EpochState; one validity-gated contribution, one caller, one credit; no reconstruction; transferred after POOLREAP and the gov deposit returns with a typed overflow terminal, then zeroed; governance never reads it; seed import reads UTxOState[5] (commitment v13), Mithril carries it; store semantics $STORE, accumulator schema $ACCV, framing $FRV; CE tests ran (DC-EPOCH-45)"
fi
exit $FAILED
