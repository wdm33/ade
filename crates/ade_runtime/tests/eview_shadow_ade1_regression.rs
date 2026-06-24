//! Hermetic regression fixture — EPOCH-CONSENSUS-VIEW shadow stake agreement (DC-EPOCH-11).
//!
//! Pins `ReducedUtxoCheckpoint::derive_stake_by_pool` to the REAL ADE1 result proven live against
//! `cardano-cli` on Preview epoch 1334 (off-repo evidence
//! `~/.cardano-c2-preview/eview-oracle-evidence/checkpoint-shadow-RESULT.txt`): ADE1's two
//! base-credential delegators sum to exactly the `cardano-cli stake-snapshot` value, and ADE1
//! carried no rewards. This is a GREEN regression guard (NOT a pre-promotion gate) — a
//! reduce/aggregate/derive regression makes the derived ADE1 stake diverge from the captured
//! oracle and fails here. The embedded values are public on-chain Preview data.

use std::collections::BTreeMap;

use ade_ledger::delegation::DelegationState;
use ade_ledger::reduced_utxo::ReducedStakeRef;
use ade_runtime::chaindb::ReducedUtxoCheckpoint;
use ade_types::shelley::cert::StakeCredential;
use ade_types::tx::{Coin, PoolId, TxIn};
use ade_types::{Hash28, Hash32};

fn hexb(s: &str) -> Vec<u8> {
    (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap()).collect()
}
fn h28(s: &str) -> Hash28 {
    let mut a = [0u8; 28];
    a.copy_from_slice(&hexb(s));
    Hash28(a)
}
fn keyhash(s: &str) -> StakeCredential {
    StakeCredential::KeyHash(h28(s))
}
fn txin(i: u16) -> TxIn {
    let mut h = [0u8; 32];
    h[..2].copy_from_slice(&i.to_be_bytes()); // distinct key per entry (the checkpoint sums by cred)
    TxIn { tx_hash: Hash32(h), index: 0 }
}

// ADE1 (preview pool 431549bf…) and its TWO real base-credential delegators, captured from the
// live preview chain at epoch 1334 (cardano-cli query utxo + stake-snapshot, 2026-06-20).
const ADE1_POOL: &str = "431549bf1414e0d4a95b9fdeccbe60f66109ff8b81f502b628b2b8f3";
const DELEGATOR_A: &str = "185fff1f32d0681c569cb9829004acf909ee681564154845f0c4b00d";
const STAKE_A: u64 = 1_000_014_603_080;
const DELEGATOR_B: &str = "49b7177b07234931a23ff18aac877123edaef1b66b8a5f469beaf359";
const STAKE_B: u64 = 1_497_795_823;
const ADE1_ORACLE: u64 = 1_001_512_398_903; // cardano-cli stake-snapshot MARK, preview ep1334

#[test]
fn derive_stake_by_pool_matches_cardano_cli_ade1_preview_ep1334() {
    let dir = tempfile::tempdir().unwrap();
    let cp = ReducedUtxoCheckpoint::open(&dir.path().join("ade1-shadow.redb")).unwrap();

    let cred_a = keyhash(DELEGATOR_A);
    let cred_b = keyhash(DELEGATOR_B);
    // a base credential present in the UTxO but NOT delegated -> must contribute nothing.
    let cred_undelegated = keyhash("00000000000000000000000000000000000000000000000000000001");

    // (1) the reduced UTxO routed through the live -mat checkpoint primitive.
    let mut utxo: BTreeMap<TxIn, (Coin, ReducedStakeRef)> = BTreeMap::new();
    utxo.insert(txin(0), (Coin(STAKE_A), ReducedStakeRef::Base(cred_a.clone())));
    utxo.insert(txin(1), (Coin(STAKE_B), ReducedStakeRef::Base(cred_b.clone())));
    utxo.insert(txin(2), (Coin(777_777_777), ReducedStakeRef::Base(cred_undelegated)));
    utxo.insert(txin(3), (Coin(999_999), ReducedStakeRef::NonContributing));
    cp.build_from(&utxo).unwrap();

    // (2) the delegation (both ADE1 delegators -> ADE1; no rewards, matching the capture).
    let ade1 = PoolId(h28(ADE1_POOL));
    let mut deleg = DelegationState::default();
    deleg.delegations.insert(cred_a, ade1.clone());
    deleg.delegations.insert(cred_b, ade1.clone());

    // (3) derive — sum_base_credential_stake + aggregate_pool_stake, the same composition the
    //     live shadow proof exercised on the full real 3.07M-entry preview UTxO.
    let sbp = cp.derive_stake_by_pool(&deleg).unwrap();

    assert_eq!(STAKE_A + STAKE_B, ADE1_ORACLE, "fixture self-consistency");
    assert_eq!(
        sbp.pool_stakes.get(&ade1).map(|c| c.0),
        Some(ADE1_ORACLE),
        "derived ADE1 stake must equal the captured cardano-cli stake-snapshot value"
    );
    // only ADE1 contributes: the undelegated base cred + the non-contributing entry add nothing.
    assert_eq!(sbp.pool_stakes.len(), 1, "only ADE1 has delegated stake");
    assert_eq!(sbp.total_active_stake.0, ADE1_ORACLE, "total == ADE1 (nothing else delegated)");
}
