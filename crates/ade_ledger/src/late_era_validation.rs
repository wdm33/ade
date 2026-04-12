// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! State-backed late-era validation — Slice S-27 (Cluster P-A).
//!
//! Pure check functions implementing the Alonzo/Babbage/Conway ledger
//! rules that require resolved UTxO state. Each function is state-free
//! in its signature: inputs are either tx fields or resolved outputs,
//! never mutable state. Callers are responsible for resolving inputs
//! against `UTxOState` before invoking.
//!
//! All checks mirror the Haskell cardano-ledger rules exactly, per the
//! citations in `docs/active/S-27_obligation_discharge.md`. Error
//! constructors are 1:1 with the Haskell variants
//! (`BadInputsUTxO`, `InsufficientCollateral`, etc.) for future
//! wire-level agreement.

use std::collections::{BTreeMap, BTreeSet};

use ade_crypto::blake2b::blake2b_256;
use ade_types::tx::{Coin, TxIn};
use ade_types::{Hash28, Hash32};

use crate::error::{
    BadInputsError, IncorrectTotalCollateralError, InsufficientCollateralError, LedgerError,
    MissingRequiredDatumsError, MissingRequiredSignersError, NonDisjointRefInputsError,
    WrongNetworkError, WrongNetworkOutputError,
};

// ---------------------------------------------------------------------------
// Input resolution (O-27.3)
// ---------------------------------------------------------------------------

/// Resolve a set of inputs against the UTxO, returning all missing inputs.
///
/// Mirrors Shelley's `validateBadInputsUTxO` predicate:
/// `failureOnNonEmptySet (inputs ∖ dom utxo) BadInputsUTxO`.
///
/// Used for spend inputs (all eras), collateral inputs (Alonzo+), and
/// reference inputs (Babbage+). The Haskell ledger treats all three
/// with the same constructor; callers may merge sets and call once.
///
/// On success, returns `Ok(())`. On any missing input, returns
/// `LedgerError::BadInputs` carrying the full missing set (not just
/// the first one — mirrors Haskell's `NonEmptySet` payload).
pub fn check_inputs_present<V>(
    inputs: &BTreeSet<TxIn>,
    utxo: &BTreeMap<TxIn, V>,
) -> Result<(), LedgerError> {
    let mut missing: BTreeSet<TxIn> = BTreeSet::new();
    for tx_in in inputs {
        if !utxo.contains_key(tx_in) {
            missing.insert(tx_in.clone());
        }
    }
    if missing.is_empty() {
        Ok(())
    } else {
        Err(LedgerError::BadInputs(BadInputsError { missing }))
    }
}

// ---------------------------------------------------------------------------
// Collateral checks (O-27.1, O-27.2)
// ---------------------------------------------------------------------------

/// Enforce that the collateral inputs set is non-empty when required.
///
/// Required whenever a tx uses Plutus scripts (`script_data_hash`
/// present). Mirrors Haskell `NoCollateralInputs` from Alonzo Utxo.
pub fn check_collateral_non_empty(collateral_inputs: &BTreeSet<TxIn>) -> Result<(), LedgerError> {
    if collateral_inputs.is_empty() {
        Err(LedgerError::NoCollateralInputs)
    } else {
        Ok(())
    }
}

/// Enforce the collateral percent rule: `100 * balance >= percent * fee`.
///
/// From O-27.1 discharge:
/// - `balance = sum(collateral_inputs.coin) − collateral_return.coin` (Babbage+)
/// - `percent` = protocol parameter `collateralPercentage` (mainnet: 150)
/// - `fee` = tx body fee field
///
/// Implementation uses `i128` cross-multiplication — no division, no
/// rounding in the predicate. Matches the Haskell `Val.scale`-based
/// check exactly. Overflow-safe for any well-typed `u64` fee because
/// `u64 * u16` fits in `i128` with room to spare.
///
/// The `required` field of the error payload is reporting-only,
/// computed as `ceiling((percent * fee) / 100)`. The validity
/// decision itself never rounds.
pub fn check_collateral_percent(
    balance: i128,
    percent: u16,
    fee: Coin,
) -> Result<(), LedgerError> {
    let fee_lovelace = fee.0 as i128;
    let percent_i128 = percent as i128;
    // 100 * balance >= percent * fee
    let lhs = balance.saturating_mul(100);
    let rhs = percent_i128.saturating_mul(fee_lovelace);
    if lhs >= rhs {
        Ok(())
    } else {
        // Reporting-only ceiling of required collateral.
        let required = ceil_div_u128(
            (percent as u128).saturating_mul(fee.0 as u128),
            100u128,
        );
        Err(LedgerError::InsufficientCollateral(
            InsufficientCollateralError {
                balance,
                required: u128_to_u64_clamped(required),
                percent,
                fee: fee.0,
            },
        ))
    }
}

/// Enforce that `totalCollateral` (when declared) matches the computed balance.
///
/// From O-27.2 discharge: Babbage's `validateCollateralEqBalance`
/// requires `sum(collateral_inputs.coin) − collateral_return.coin ==
/// totalCollateral`. Pre-Babbage eras do not support `totalCollateral`;
/// callers pass `None` for those.
pub fn check_total_collateral(
    balance: i128,
    declared: Option<Coin>,
) -> Result<(), LedgerError> {
    match declared {
        None => Ok(()),
        Some(d) if balance == d.0 as i128 => Ok(()),
        Some(d) => Err(LedgerError::IncorrectTotalCollateral(
            IncorrectTotalCollateralError {
                balance,
                declared: d.0,
            },
        )),
    }
}

/// Enforce that collateral inputs contain no non-ADA assets unless a
/// collateral return output is provided that can absorb them.
///
/// From O-27.2 discharge: `validateCollateralContainsNonADA` raises
/// `CollateralContainsNonADA` when collateral inputs carry native
/// assets and no collateral return is provided (the non-ADA cannot be
/// paid as fee and would be lost).
///
/// Pre-Babbage eras cannot provide a collateral return, so this
/// function reduces to "collateral must be pure ADA" for Alonzo.
pub fn check_collateral_contains_non_ada(
    any_collateral_has_non_ada: bool,
    has_collateral_return: bool,
) -> Result<(), LedgerError> {
    if any_collateral_has_non_ada && !has_collateral_return {
        Err(LedgerError::CollateralContainsNonADA)
    } else {
        Ok(())
    }
}

/// Compute the collateral ADA balance.
///
/// `balance = sum(collateral_inputs.coin) − collateral_return.coin`.
///
/// Returns `i128` because adversarial input values could theoretically
/// sum beyond `u64::MAX`, and a negative balance (return greater than
/// inputs) is a valid error-reportable state rather than an overflow
/// panic. The Haskell `DeltaCoin` is signed `Integer`-backed for the
/// same reason.
pub fn compute_collateral_balance(
    collateral_inputs_coin_sum: u128,
    collateral_return_coin: u64,
) -> i128 {
    (collateral_inputs_coin_sum as i128) - (collateral_return_coin as i128)
}

// ---------------------------------------------------------------------------
// Reference input disjointness (O-28.1)
// ---------------------------------------------------------------------------

/// PV gate lower bound (exclusive): `PV > 8` means `PV >= 9` = Conway.
const REF_INPUT_DISJOINT_PV_LOWER_EXCLUSIVE: u16 = 8;
/// PV gate upper bound (exclusive): Haskell reserves room via `< 11`.
/// Future eras beyond 10 may re-evaluate. Matches ledger exactly.
const REF_INPUT_DISJOINT_PV_UPPER_EXCLUSIVE: u16 = 11;

/// Enforce that `inputs ∩ reference_inputs == ∅` when the protocol
/// version gate fires.
///
/// From O-28.1 discharge: the ledger enforces this ONLY when
/// `PV > 8 && PV < 11` — i.e. Conway (PV 9, 10). Babbage (PV 7, 8)
/// silently accepts the overlap. Pre-Babbage eras do not have
/// reference inputs at all (callers should pass empty sets there).
///
/// Mirrors Haskell `disjointRefInputs`.
pub fn check_reference_input_disjoint(
    inputs: &BTreeSet<TxIn>,
    reference_inputs: &BTreeSet<TxIn>,
    pv_major: u16,
) -> Result<(), LedgerError> {
    let gate_fires = pv_major > REF_INPUT_DISJOINT_PV_LOWER_EXCLUSIVE
        && pv_major < REF_INPUT_DISJOINT_PV_UPPER_EXCLUSIVE;
    if !gate_fires {
        return Ok(());
    }
    let intersection: BTreeSet<TxIn> = inputs.intersection(reference_inputs).cloned().collect();
    if intersection.is_empty() {
        Ok(())
    } else {
        Err(LedgerError::NonDisjointRefInputs(
            NonDisjointRefInputsError { intersection },
        ))
    }
}

// ---------------------------------------------------------------------------
// Datum hash binding (O-28.2)
// ---------------------------------------------------------------------------

/// Compute a datum hash from the witness datum's preserved wire bytes.
///
/// From O-28.2 discharge: Haskell's `hashData` hashes the `MemoBytes`
/// (raw wire bytes preserved during deserialization) — NOT a re-encoded
/// canonical form. Callers on hash-critical paths MUST pass the exact
/// bytes received on the wire (e.g., via `PreservedCbor<T>.wire_bytes()`).
pub fn compute_datum_hash(wire_bytes: &[u8]) -> Hash32 {
    blake2b_256(wire_bytes)
}

/// Enforce that every required datum hash has a matching witness datum.
///
/// `required` = union of `datum_hash` fields over resolved inputs.
/// `provided` = set of hashes computed via `compute_datum_hash` over
/// each witness-provided datum's raw bytes.
///
/// Inline datums (Babbage+) are not required to appear in
/// `required` — the caller excludes inline-datum inputs from the
/// required set.
///
/// Mirrors Haskell `missingRequiredDatums`.
pub fn check_datum_hashes_present(
    required: &BTreeSet<Hash32>,
    provided: &BTreeSet<Hash32>,
) -> Result<(), LedgerError> {
    let missing: BTreeSet<Hash32> = required.difference(provided).cloned().collect();
    if missing.is_empty() {
        Ok(())
    } else {
        Err(LedgerError::MissingRequiredDatums(
            MissingRequiredDatumsError { missing },
        ))
    }
}

// ---------------------------------------------------------------------------
// Required signers (O-28.3)
// ---------------------------------------------------------------------------

/// Enforce `required_signers ⊆ available_key_hashes`.
///
/// From O-28.3 discharge: Haskell folds `required_signers` into
/// `witsVKeyNeeded` and applies the standard Shelley subset check.
/// A Plutus script, native script, or redeemer that *references* a
/// signer hash is NOT an acceptable substitute — only an actual
/// vkey witness satisfies the requirement.
///
/// The check is unconditional: applies even when no Plutus scripts
/// are present.
///
/// Mirrors Haskell `validateNeededWitnesses` limited to the
/// required-signers contribution.
pub fn check_required_signers(
    required: &BTreeSet<Hash28>,
    available_key_hashes: &BTreeSet<Hash28>,
) -> Result<(), LedgerError> {
    let missing: BTreeSet<Hash28> = required.difference(available_key_hashes).cloned().collect();
    if missing.is_empty() {
        Ok(())
    } else {
        Err(LedgerError::MissingRequiredSigners(
            MissingRequiredSignersError { missing },
        ))
    }
}

// ---------------------------------------------------------------------------
// Network ID (O-28.4)
// ---------------------------------------------------------------------------

/// Enforce the tx-body `network_id` field matches the current network,
/// when present.
///
/// From O-28.4 discharge: the field is optional (CBOR key 15,
/// `StrictMaybe Network`). When absent, pass. When present, must
/// equal the current network ID.
///
/// Mirrors Haskell `validateWrongNetworkInTxBody`.
pub fn check_tx_network_id(declared: Option<u8>, current: u8) -> Result<(), LedgerError> {
    match declared {
        None => Ok(()),
        Some(d) if d == current => Ok(()),
        Some(d) => Err(LedgerError::WrongNetworkInTxBody(WrongNetworkError {
            declared: d,
            current,
        })),
    }
}

/// Enforce that an output address's network nibble matches the
/// current network.
///
/// Shelley-format addresses (Shelley+) encode the network ID in the
/// low nibble of the first byte. Mainnet = 1, testnets = 0.
///
/// Byron bootstrap addresses have a different encoding; callers are
/// responsible for classifying and handling those separately (Byron
/// addresses in late eras are validated via CBOR magic, not via this
/// function). This function is intended for Shelley/Alonzo+ outputs
/// where the first byte's low nibble is authoritative.
///
/// Babbage+: this function must also be called for `collateralReturn`
/// (Haskell's `allOutputs` widening). Callers are responsible for
/// that union.
///
/// Mirrors Haskell `validateWrongNetwork` (per-address predicate).
pub fn check_address_network(address: &[u8], current: u8) -> Result<(), LedgerError> {
    if address.is_empty() {
        // An empty address cannot be validated; treat as violation.
        return Err(LedgerError::WrongNetworkInOutput(WrongNetworkOutputError {
            address_first_byte: 0,
            current,
        }));
    }
    let first = address[0];
    let network_nibble = first & 0x0f;
    if network_nibble == current {
        Ok(())
    } else {
        Err(LedgerError::WrongNetworkInOutput(WrongNetworkOutputError {
            address_first_byte: first,
            current,
        }))
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn ceil_div_u128(n: u128, d: u128) -> u128 {
    // d is never zero in this module's call sites (always 100).
    if d == 0 {
        return 0;
    }
    (n + d - 1) / d
}

fn u128_to_u64_clamped(v: u128) -> u64 {
    if v > u64::MAX as u128 {
        u64::MAX
    } else {
        v as u64
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use ade_types::Hash32;

    fn tx_in(hash_byte: u8, index: u16) -> TxIn {
        TxIn {
            tx_hash: Hash32([hash_byte; 32]),
            index,
        }
    }

    // -----------------------------------------------------------------------
    // check_inputs_present (O-27.3)
    // -----------------------------------------------------------------------

    #[test]
    fn inputs_present_empty_set_passes() {
        let utxo: BTreeMap<TxIn, ()> = BTreeMap::new();
        let inputs: BTreeSet<TxIn> = BTreeSet::new();
        assert!(check_inputs_present(&inputs, &utxo).is_ok());
    }

    #[test]
    fn inputs_present_all_resolved_passes() {
        let mut utxo = BTreeMap::new();
        utxo.insert(tx_in(0x01, 0), ());
        utxo.insert(tx_in(0x02, 0), ());

        let mut inputs = BTreeSet::new();
        inputs.insert(tx_in(0x01, 0));
        inputs.insert(tx_in(0x02, 0));

        assert!(check_inputs_present(&inputs, &utxo).is_ok());
    }

    #[test]
    fn inputs_present_missing_one_reports_it() {
        let mut utxo = BTreeMap::new();
        utxo.insert(tx_in(0x01, 0), ());

        let mut inputs = BTreeSet::new();
        inputs.insert(tx_in(0x01, 0));
        inputs.insert(tx_in(0x99, 0));

        match check_inputs_present(&inputs, &utxo) {
            Err(LedgerError::BadInputs(e)) => {
                assert_eq!(e.missing.len(), 1);
                assert!(e.missing.contains(&tx_in(0x99, 0)));
            }
            other => panic!("expected BadInputs, got {other:?}"),
        }
    }

    #[test]
    fn inputs_present_missing_all_reports_all() {
        let utxo: BTreeMap<TxIn, ()> = BTreeMap::new();

        let mut inputs = BTreeSet::new();
        inputs.insert(tx_in(0x01, 0));
        inputs.insert(tx_in(0x02, 0));
        inputs.insert(tx_in(0x03, 0));

        match check_inputs_present(&inputs, &utxo) {
            Err(LedgerError::BadInputs(e)) => {
                assert_eq!(e.missing.len(), 3);
            }
            other => panic!("expected BadInputs, got {other:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // check_collateral_non_empty (O-27.2)
    // -----------------------------------------------------------------------

    #[test]
    fn collateral_non_empty_passes_when_present() {
        let mut col = BTreeSet::new();
        col.insert(tx_in(0xaa, 0));
        assert!(check_collateral_non_empty(&col).is_ok());
    }

    #[test]
    fn collateral_non_empty_fails_when_empty() {
        let col: BTreeSet<TxIn> = BTreeSet::new();
        assert!(matches!(
            check_collateral_non_empty(&col),
            Err(LedgerError::NoCollateralInputs)
        ));
    }

    // -----------------------------------------------------------------------
    // check_collateral_percent (O-27.1)
    // -----------------------------------------------------------------------

    #[test]
    fn percent_150_fee_100_balance_150_passes() {
        // 100 * 150 == 150 * 100 → equality, >= holds
        assert!(check_collateral_percent(150, 150, Coin(100)).is_ok());
    }

    #[test]
    fn percent_150_fee_100_balance_149_fails() {
        // 100 * 149 = 14900 < 15000 = 150 * 100
        match check_collateral_percent(149, 150, Coin(100)) {
            Err(LedgerError::InsufficientCollateral(e)) => {
                assert_eq!(e.balance, 149);
                assert_eq!(e.required, 150); // ceil(15000/100) = 150
                assert_eq!(e.percent, 150);
                assert_eq!(e.fee, 100);
            }
            other => panic!("expected InsufficientCollateral, got {other:?}"),
        }
    }

    #[test]
    fn percent_150_fee_101_required_ceiling_153() {
        // ceil(150 * 101 / 100) = ceil(151.5) = 152
        match check_collateral_percent(0, 150, Coin(101)) {
            Err(LedgerError::InsufficientCollateral(e)) => {
                assert_eq!(e.required, 152);
            }
            other => panic!("expected InsufficientCollateral, got {other:?}"),
        }
    }

    #[test]
    fn zero_fee_any_balance_passes() {
        // 100 * 0 >= 150 * 0 → trivially true
        assert!(check_collateral_percent(0, 150, Coin(0)).is_ok());
    }

    #[test]
    fn negative_balance_fails() {
        // balance = -1 (return exceeded inputs)
        match check_collateral_percent(-1, 150, Coin(100)) {
            Err(LedgerError::InsufficientCollateral(e)) => {
                assert_eq!(e.balance, -1);
            }
            other => panic!("expected InsufficientCollateral, got {other:?}"),
        }
    }

    #[test]
    fn large_fee_does_not_overflow() {
        // Near-u64::MAX fee should still evaluate without overflow.
        // u64::MAX = 18_446_744_073_709_551_615
        // 150 * u64::MAX fits in i128 easily (< 2^71)
        let fee = u64::MAX;
        let balance_enough = (fee as i128) * 150 / 100 + 1;
        // Sanity: passes
        assert!(check_collateral_percent(balance_enough, 150, Coin(fee)).is_ok());
        // balance 0 fails
        assert!(check_collateral_percent(0, 150, Coin(fee)).is_err());
    }

    #[test]
    fn percent_5_boundary_inclusive() {
        // 100 * 5 == 5 * 100 — inclusive >= should pass
        assert!(check_collateral_percent(5, 5, Coin(100)).is_ok());
        // one less fails
        assert!(check_collateral_percent(4, 5, Coin(100)).is_err());
    }

    // -----------------------------------------------------------------------
    // check_total_collateral (O-27.2)
    // -----------------------------------------------------------------------

    #[test]
    fn total_collateral_absent_always_passes() {
        assert!(check_total_collateral(100, None).is_ok());
        assert!(check_total_collateral(-1, None).is_ok());
    }

    #[test]
    fn total_collateral_matches_passes() {
        assert!(check_total_collateral(150, Some(Coin(150))).is_ok());
    }

    #[test]
    fn total_collateral_mismatch_fails() {
        match check_total_collateral(150, Some(Coin(149))) {
            Err(LedgerError::IncorrectTotalCollateral(e)) => {
                assert_eq!(e.balance, 150);
                assert_eq!(e.declared, 149);
            }
            other => panic!("expected IncorrectTotalCollateral, got {other:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // check_collateral_contains_non_ada (O-27.2)
    // -----------------------------------------------------------------------

    #[test]
    fn non_ada_without_return_fails() {
        assert!(matches!(
            check_collateral_contains_non_ada(true, false),
            Err(LedgerError::CollateralContainsNonADA)
        ));
    }

    #[test]
    fn non_ada_with_return_passes() {
        assert!(check_collateral_contains_non_ada(true, true).is_ok());
    }

    #[test]
    fn pure_ada_always_passes() {
        assert!(check_collateral_contains_non_ada(false, false).is_ok());
        assert!(check_collateral_contains_non_ada(false, true).is_ok());
    }

    // -----------------------------------------------------------------------
    // compute_collateral_balance (O-27.2)
    // -----------------------------------------------------------------------

    #[test]
    fn balance_no_return() {
        assert_eq!(compute_collateral_balance(1000, 0), 1000);
    }

    #[test]
    fn balance_with_return() {
        assert_eq!(compute_collateral_balance(1000, 200), 800);
    }

    #[test]
    fn balance_return_exceeds_inputs_is_negative() {
        assert_eq!(compute_collateral_balance(100, 200), -100);
    }

    #[test]
    fn balance_large_inputs_fits_i128() {
        // 1000 inputs of 10B ADA each = 10^16 lovelace — well within i128
        let sum: u128 = 1000u128 * 10_000_000_000_000_000u128;
        let bal = compute_collateral_balance(sum, 0);
        assert_eq!(bal, sum as i128);
    }

    // -----------------------------------------------------------------------
    // check_reference_input_disjoint (O-28.1)
    // -----------------------------------------------------------------------

    #[test]
    fn ref_disjoint_babbage_pv7_overlap_passes() {
        // Babbage PV 7 silently allows overlap.
        let mut ins = BTreeSet::new();
        ins.insert(tx_in(0x01, 0));
        let mut refs = BTreeSet::new();
        refs.insert(tx_in(0x01, 0));
        assert!(check_reference_input_disjoint(&ins, &refs, 7).is_ok());
    }

    #[test]
    fn ref_disjoint_babbage_pv8_overlap_passes() {
        let mut ins = BTreeSet::new();
        ins.insert(tx_in(0x01, 0));
        let mut refs = BTreeSet::new();
        refs.insert(tx_in(0x01, 0));
        assert!(check_reference_input_disjoint(&ins, &refs, 8).is_ok());
    }

    #[test]
    fn ref_disjoint_conway_pv9_overlap_fails() {
        let mut ins = BTreeSet::new();
        ins.insert(tx_in(0x01, 0));
        ins.insert(tx_in(0x02, 0));
        let mut refs = BTreeSet::new();
        refs.insert(tx_in(0x01, 0));
        refs.insert(tx_in(0x99, 0));

        match check_reference_input_disjoint(&ins, &refs, 9) {
            Err(LedgerError::NonDisjointRefInputs(e)) => {
                assert_eq!(e.intersection.len(), 1);
                assert!(e.intersection.contains(&tx_in(0x01, 0)));
            }
            other => panic!("expected NonDisjointRefInputs, got {other:?}"),
        }
    }

    #[test]
    fn ref_disjoint_conway_pv10_overlap_fails() {
        let mut ins = BTreeSet::new();
        ins.insert(tx_in(0x01, 0));
        let mut refs = BTreeSet::new();
        refs.insert(tx_in(0x01, 0));
        assert!(matches!(
            check_reference_input_disjoint(&ins, &refs, 10),
            Err(LedgerError::NonDisjointRefInputs(_))
        ));
    }

    #[test]
    fn ref_disjoint_future_pv11_returns_to_pass() {
        // PV 11+ is outside the gate; check does not apply until a
        // future era re-enables it. Matches Haskell's `< 11` bound.
        let mut ins = BTreeSet::new();
        ins.insert(tx_in(0x01, 0));
        let mut refs = BTreeSet::new();
        refs.insert(tx_in(0x01, 0));
        assert!(check_reference_input_disjoint(&ins, &refs, 11).is_ok());
    }

    #[test]
    fn ref_disjoint_conway_disjoint_passes() {
        let mut ins = BTreeSet::new();
        ins.insert(tx_in(0x01, 0));
        let mut refs = BTreeSet::new();
        refs.insert(tx_in(0x99, 0));
        assert!(check_reference_input_disjoint(&ins, &refs, 9).is_ok());
    }

    #[test]
    fn ref_disjoint_empty_sets_always_pass() {
        let empty: BTreeSet<TxIn> = BTreeSet::new();
        for pv in [5, 7, 9, 10, 11] {
            assert!(check_reference_input_disjoint(&empty, &empty, pv).is_ok());
        }
    }

    // -----------------------------------------------------------------------
    // compute_datum_hash + check_datum_hashes_present (O-28.2)
    // -----------------------------------------------------------------------

    #[test]
    fn datum_hash_bit_exact() {
        // Two different byte encodings of the same semantic value produce
        // different hashes — this is the bit-exact property.
        let a: [u8; 3] = [0x81, 0x01, 0x00]; // array(1) [uint 1, extra byte (malformed but distinct bytes)]
        let b: [u8; 2] = [0x81, 0x01]; // array(1) [uint 1]
        assert_ne!(compute_datum_hash(&a), compute_datum_hash(&b));
    }

    #[test]
    fn datum_hash_deterministic() {
        let bytes = [0x82, 0x01, 0x02];
        assert_eq!(compute_datum_hash(&bytes), compute_datum_hash(&bytes));
    }

    #[test]
    fn datum_hashes_present_all_match() {
        let bytes_a = [0x01u8, 0x02];
        let bytes_b = [0x03u8, 0x04];
        let ha = compute_datum_hash(&bytes_a);
        let hb = compute_datum_hash(&bytes_b);

        let mut required = BTreeSet::new();
        required.insert(ha.clone());
        required.insert(hb.clone());

        let mut provided = BTreeSet::new();
        provided.insert(ha);
        provided.insert(hb);

        assert!(check_datum_hashes_present(&required, &provided).is_ok());
    }

    #[test]
    fn datum_hashes_present_missing_one_reports_it() {
        let ha = compute_datum_hash(&[0x01, 0x02]);
        let hb = compute_datum_hash(&[0x03, 0x04]);

        let mut required = BTreeSet::new();
        required.insert(ha.clone());
        required.insert(hb.clone());

        let mut provided = BTreeSet::new();
        provided.insert(ha); // hb missing

        match check_datum_hashes_present(&required, &provided) {
            Err(LedgerError::MissingRequiredDatums(e)) => {
                assert_eq!(e.missing.len(), 1);
                assert!(e.missing.contains(&hb));
            }
            other => panic!("expected MissingRequiredDatums, got {other:?}"),
        }
    }

    #[test]
    fn datum_hashes_present_extra_provided_ok() {
        // Haskell also has NotAllowedSupplementalDatums, but our
        // check here is only the subset direction (required ⊆ provided).
        // Extras are permissible by this function.
        let ha = compute_datum_hash(&[0x01]);
        let hb = compute_datum_hash(&[0x02]);

        let mut required = BTreeSet::new();
        required.insert(ha.clone());

        let mut provided = BTreeSet::new();
        provided.insert(ha);
        provided.insert(hb);

        assert!(check_datum_hashes_present(&required, &provided).is_ok());
    }

    #[test]
    fn datum_hashes_empty_required_always_ok() {
        let empty: BTreeSet<Hash32> = BTreeSet::new();
        let mut provided = BTreeSet::new();
        provided.insert(compute_datum_hash(&[0x01]));
        assert!(check_datum_hashes_present(&empty, &provided).is_ok());
    }

    // -----------------------------------------------------------------------
    // check_required_signers (O-28.3)
    // -----------------------------------------------------------------------

    fn cred(b: u8) -> Hash28 {
        Hash28([b; 28])
    }

    #[test]
    fn required_signers_subset_passes() {
        let mut required = BTreeSet::new();
        required.insert(cred(0x01));
        required.insert(cred(0x02));

        let mut available = BTreeSet::new();
        available.insert(cred(0x01));
        available.insert(cred(0x02));
        available.insert(cred(0x03)); // extra is fine

        assert!(check_required_signers(&required, &available).is_ok());
    }

    #[test]
    fn required_signers_missing_reports_set() {
        let mut required = BTreeSet::new();
        required.insert(cred(0x01));
        required.insert(cred(0x02));
        required.insert(cred(0x03));

        let mut available = BTreeSet::new();
        available.insert(cred(0x01)); // 0x02 and 0x03 missing

        match check_required_signers(&required, &available) {
            Err(LedgerError::MissingRequiredSigners(e)) => {
                assert_eq!(e.missing.len(), 2);
                assert!(e.missing.contains(&cred(0x02)));
                assert!(e.missing.contains(&cred(0x03)));
            }
            other => panic!("expected MissingRequiredSigners, got {other:?}"),
        }
    }

    #[test]
    fn required_signers_empty_always_passes() {
        let empty: BTreeSet<Hash28> = BTreeSet::new();
        let mut available = BTreeSet::new();
        available.insert(cred(0x01));
        assert!(check_required_signers(&empty, &available).is_ok());
        assert!(check_required_signers(&empty, &empty).is_ok());
    }

    // -----------------------------------------------------------------------
    // check_tx_network_id (O-28.4 check 1)
    // -----------------------------------------------------------------------

    #[test]
    fn tx_network_id_none_passes() {
        assert!(check_tx_network_id(None, 1).is_ok());
        assert!(check_tx_network_id(None, 0).is_ok());
    }

    #[test]
    fn tx_network_id_matches_passes() {
        assert!(check_tx_network_id(Some(1), 1).is_ok());
        assert!(check_tx_network_id(Some(0), 0).is_ok());
    }

    #[test]
    fn tx_network_id_mismatch_fails() {
        match check_tx_network_id(Some(0), 1) {
            Err(LedgerError::WrongNetworkInTxBody(e)) => {
                assert_eq!(e.declared, 0);
                assert_eq!(e.current, 1);
            }
            other => panic!("expected WrongNetworkInTxBody, got {other:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // check_address_network (O-28.4 check 2)
    // -----------------------------------------------------------------------

    #[test]
    fn addr_mainnet_nibble_1_matches() {
        let addr = vec![0x61u8, 0xaa, 0xbb]; // high nibble 6 (enterprise), low nibble 1 (mainnet)
        assert!(check_address_network(&addr, 1).is_ok());
    }

    #[test]
    fn addr_testnet_nibble_0_matches() {
        let addr = vec![0x60u8, 0xaa, 0xbb];
        assert!(check_address_network(&addr, 0).is_ok());
    }

    #[test]
    fn addr_network_mismatch_fails() {
        let addr = vec![0x60u8, 0xaa]; // testnet
        match check_address_network(&addr, 1) {
            Err(LedgerError::WrongNetworkInOutput(e)) => {
                assert_eq!(e.address_first_byte & 0x0f, 0);
                assert_eq!(e.current, 1);
            }
            other => panic!("expected WrongNetworkInOutput, got {other:?}"),
        }
    }

    #[test]
    fn addr_empty_fails() {
        assert!(matches!(
            check_address_network(&[], 1),
            Err(LedgerError::WrongNetworkInOutput(_))
        ));
    }

    #[test]
    fn addr_high_nibble_ignored() {
        // Different high nibbles (address types) all share the same
        // network byte check; only the low nibble matters here.
        for high in 0x0..=0xF {
            let addr = vec![(high << 4) | 1u8, 0xaa];
            assert!(check_address_network(&addr, 1).is_ok(),
                "high nibble {:x} should not affect network check", high);
        }
    }

    // -----------------------------------------------------------------------
    // Determinism
    // -----------------------------------------------------------------------

    #[test]
    fn all_functions_deterministic() {
        let mut utxo = BTreeMap::new();
        utxo.insert(tx_in(0x01, 0), ());
        let mut ins = BTreeSet::new();
        ins.insert(tx_in(0x02, 0));

        let r1 = check_inputs_present(&ins, &utxo);
        let r2 = check_inputs_present(&ins, &utxo);
        assert_eq!(format!("{r1:?}"), format!("{r2:?}"));

        let c1 = check_collateral_percent(149, 150, Coin(100));
        let c2 = check_collateral_percent(149, 150, Coin(100));
        assert_eq!(format!("{c1:?}"), format!("{c2:?}"));

        let mut refs = BTreeSet::new();
        refs.insert(tx_in(0x02, 0));
        let d1 = check_reference_input_disjoint(&ins, &refs, 9);
        let d2 = check_reference_input_disjoint(&ins, &refs, 9);
        assert_eq!(format!("{d1:?}"), format!("{d2:?}"));
    }
}
