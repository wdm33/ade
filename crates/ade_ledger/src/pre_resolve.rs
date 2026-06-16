// Core Contract:
// - Deterministic enumeration; no I/O, no HashMap/HashSet, no wall-clock, no floats
// - Canonical (BTreeSet) ordering for the resolved-view plan

//! MEM-OPT-UTXO-DISK S2b: the era-aware pre-resolve dependency extractor (GREEN).
//!
//! The SINGLE authority for *what UTxOs validating this tx reads*. The RED shell
//! resolves exactly this set from the on-disk anchor into an in-memory working-set
//! BEFORE BLUE validation runs, so BLUE never reaches disk. Returns a DETERMINISTIC
//! sorted set (`BTreeSet`) — never a `HashSet` — so the resolved-view order is
//! canonical and replay-stable.
//!
//! The per-era dependency table (the proof artifact) lives in
//! `docs/clusters/MEM-OPT-UTXO-DISK/S2b-pre-resolve.md`. The live admission path
//! decodes to `ConwayTxBody` — the structural superset (spend ∪ collateral ∪
//! reference); earlier eras are documented subsets. The danger is NOT the obvious
//! spend inputs but script/context construction, which for Babbage/Conway pulls
//! **reference inputs**; the completeness test fails if any class is dropped.

use std::collections::BTreeSet;

use ade_types::conway::tx::ConwayTxBody;
use ade_types::tx::TxIn;

/// Every `TxIn` the validation of a Conway tx can read: spend inputs ∪ collateral
/// inputs ∪ reference inputs. This is the EXACT set the Conway validator feeds to
/// `check_inputs_present` and the Plutus script-context resolver — the pre-resolve
/// set and the validated set are the same set by construction.
pub fn collect_required_txins(body: &ConwayTxBody) -> BTreeSet<TxIn> {
    let mut required: BTreeSet<TxIn> = body.inputs.iter().cloned().collect();
    if let Some(collateral) = &body.collateral_inputs {
        required.extend(collateral.iter().cloned());
    }
    if let Some(reference) = &body.reference_inputs {
        required.extend(reference.iter().cloned());
    }
    required
}

/// The union of every tx's required `TxIn`s across a block (deterministic sorted
/// set). NOTE: an output produced by an earlier tx in the SAME block is NOT in the
/// anchor — block application seeds it into the resolved working-set as txs apply
/// (the wiring step); this enumeration is purely structural.
pub fn collect_required_txins_block<'a>(
    bodies: impl IntoIterator<Item = &'a ConwayTxBody>,
) -> BTreeSet<TxIn> {
    let mut required = BTreeSet::new();
    for body in bodies {
        required.extend(collect_required_txins(body));
    }
    required
}

#[cfg(test)]
mod tests {
    use super::*;
    use ade_types::babbage::tx::BabbageTxOut;
    use ade_types::tx::Coin;
    use ade_types::Hash32;

    fn txin(h: u8, i: u16) -> TxIn {
        TxIn {
            tx_hash: Hash32([h; 32]),
            index: i,
        }
    }

    fn body_with(
        inputs: &[TxIn],
        collateral: Option<&[TxIn]>,
        reference: Option<&[TxIn]>,
    ) -> ConwayTxBody {
        ConwayTxBody {
            inputs: inputs.iter().cloned().collect(),
            outputs: vec![BabbageTxOut {
                address: vec![0x00; 29],
                coin: Coin(1),
                multi_asset: None,
                datum_option: None,
                script_ref: None,
            }],
            fee: Coin(0),
            ttl: None,
            certs: None,
            withdrawals: None,
            metadata_hash: None,
            validity_interval_start: None,
            mint: None,
            script_data_hash: None,
            collateral_inputs: collateral.map(|c| c.iter().cloned().collect()),
            required_signers: None,
            network_id: None,
            collateral_return: None,
            total_collateral: None,
            reference_inputs: reference.map(|r| r.iter().cloned().collect()),
            voting_procedures: None,
            proposal_procedures: None,
            treasury_value: None,
            donation: None,
        }
    }

    /// The closed-era completeness proof: a Conway body's required set includes ALL
    /// THREE UTxO classes — spends, collateral, AND reference. Dropping reference
    /// (the script-context danger the dependency table warns about) fails this test.
    #[test]
    fn conway_required_set_includes_spend_collateral_and_reference() {
        let body = body_with(
            &[txin(0x01, 0), txin(0x02, 0)],
            Some(&[txin(0x10, 0)]),
            Some(&[txin(0x20, 0), txin(0x21, 0)]),
        );
        let required = collect_required_txins(&body);
        assert!(
            required.contains(&txin(0x01, 0)) && required.contains(&txin(0x02, 0)),
            "spend inputs must be required"
        );
        assert!(required.contains(&txin(0x10, 0)), "collateral must be required");
        assert!(
            required.contains(&txin(0x20, 0)) && required.contains(&txin(0x21, 0)),
            "reference inputs must be required (the script-context dependency)"
        );
        assert_eq!(required.len(), 5, "exactly the union, no extras");
    }

    #[test]
    fn required_set_is_canonically_sorted() {
        let body = body_with(&[txin(0x03, 0), txin(0x01, 0)], Some(&[txin(0x02, 0)]), None);
        let required: Vec<_> = collect_required_txins(&body).into_iter().collect();
        let mut sorted = required.clone();
        sorted.sort();
        assert_eq!(required, sorted, "the required set iterates in canonical TxIn order");
        assert_eq!(
            required,
            vec![txin(0x01, 0), txin(0x02, 0), txin(0x03, 0)],
            "deterministic union regardless of field order"
        );
    }

    #[test]
    fn no_collateral_or_reference_is_just_spends() {
        let body = body_with(&[txin(0x01, 0)], None, None);
        let required = collect_required_txins(&body);
        assert_eq!(required.len(), 1);
        assert!(required.contains(&txin(0x01, 0)));
    }

    #[test]
    fn block_union_covers_every_tx_and_class() {
        let t1 = body_with(&[txin(0x01, 0)], None, Some(&[txin(0x90, 0)]));
        let t2 = body_with(&[txin(0x02, 0)], Some(&[txin(0x10, 0)]), None);
        let union = collect_required_txins_block([&t1, &t2]);
        assert_eq!(union.len(), 4);
        for k in [txin(0x01, 0), txin(0x90, 0), txin(0x02, 0), txin(0x10, 0)] {
            assert!(union.contains(&k), "block union must cover {k:?}");
        }
    }
}
