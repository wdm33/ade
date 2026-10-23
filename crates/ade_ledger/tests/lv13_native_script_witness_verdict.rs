// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! LV-13 (DC-LEDGER-SCRIPT-03): native-script witness verdicts decide transaction validity.
//!
//! Constructed transactions pin each rule of cardano-ledger's `babbageUtxowTransition` script
//! closure and `evalTimelock`; three committed preprod blocks pin it against real chain data
//! (cardano-node accepted all three).

#![allow(clippy::unwrap_used, clippy::panic)]

use std::collections::{BTreeMap, BTreeSet};

use ade_codec::cbor::{self, canonical_width, ContainerEncoding};
use ade_ledger::error::{LedgerError, NativeScriptFailure, ScriptError};
use ade_ledger::rules::apply_block_with_verdicts;
use ade_ledger::state::{mainnet_shelley_schedule, LedgerState};
use ade_ledger::tx_validity::script_witnesses::{
    check_script_witnesses, decode_script_ref, native_script_verdict, script_signers, scripts_needed,
    ExactScriptVerdict, NativeScriptVerdict, ReducedScriptVerdict, ResolvedScriptContext,
    ResolvedScriptOutput, ScriptClosureFailures, ScriptPurpose,
};
use ade_ledger::tx_validity::verdict::TxValidityError;
use ade_ledger::tx_validity::{tx_validity, TxValidityVerdict};
use ade_ledger::witness::{decode_witness_info_single, prefixed_script_hash, WitnessInfo};
use ade_types::babbage::tx::BabbageTxOut;
use ade_types::conway::governance::{Anchor, GovAction, ProposalProcedure};
use ade_types::conway::tx::ConwayTxBody;
use ade_types::tx::{Coin, TxIn};
use ade_types::{CardanoEra, Hash28, Hash32, SlotNo};

// ---------------------------------------------------------------------------
// Native-script CBOR (Conway CDDL native_script)
// ---------------------------------------------------------------------------

fn arr(buf: &mut Vec<u8>, n: usize) {
    cbor::write_array_header(buf, ContainerEncoding::Definite(n as u64, canonical_width(n as u64)));
}

fn int(buf: &mut Vec<u8>, v: i64) {
    if v >= 0 {
        cbor::write_uint_canonical(buf, v as u64);
    } else {
        let m = (-1 - i128::from(v)) as u64;
        cbor::write_argument(buf, 1, m, canonical_width(m));
    }
}

fn sig(k: &Hash28) -> Vec<u8> {
    let mut b = Vec::new();
    arr(&mut b, 2);
    int(&mut b, 0);
    cbor::write_bytes_canonical(&mut b, &k.0);
    b
}

fn composite(tag: i64, threshold: Option<i64>, subs: &[Vec<u8>]) -> Vec<u8> {
    let mut b = Vec::new();
    arr(&mut b, if threshold.is_some() { 3 } else { 2 });
    int(&mut b, tag);
    if let Some(n) = threshold {
        int(&mut b, n);
    }
    arr(&mut b, subs.len());
    for s in subs {
        b.extend_from_slice(s);
    }
    b
}

fn all(subs: &[Vec<u8>]) -> Vec<u8> {
    composite(1, None, subs)
}
fn any(subs: &[Vec<u8>]) -> Vec<u8> {
    composite(2, None, subs)
}
fn n_of_k(n: i64, subs: &[Vec<u8>]) -> Vec<u8> {
    composite(3, Some(n), subs)
}
fn timelock(tag: i64, slot: u64) -> Vec<u8> {
    let mut b = Vec::new();
    arr(&mut b, 2);
    int(&mut b, tag);
    cbor::write_uint_canonical(&mut b, slot);
    b
}
fn invalid_before(slot: u64) -> Vec<u8> {
    timelock(4, slot)
}
fn invalid_hereafter(slot: u64) -> Vec<u8> {
    timelock(5, slot)
}

fn native_hash(script: &[u8]) -> Hash28 {
    prefixed_script_hash(0x00, script)
}

fn key(b: u8) -> (Vec<u8>, Hash28) {
    let vkey = vec![b; 32];
    let h = ade_crypto::blake2b_224(&vkey);
    (vkey, h)
}

// ---------------------------------------------------------------------------
// Witness sets, bodies, contexts
// ---------------------------------------------------------------------------

#[derive(Default)]
struct Wits {
    vkeys: Vec<Vec<u8>>,
    native: Vec<Vec<u8>>,
    plutus_v1: Vec<Vec<u8>>,
}

fn witness_set(w: &Wits) -> Vec<u8> {
    let mut b = Vec::new();
    let n = [!w.vkeys.is_empty(), !w.native.is_empty(), !w.plutus_v1.is_empty()]
        .iter()
        .filter(|x| **x)
        .count();
    cbor::write_map_header(&mut b, ContainerEncoding::Definite(n as u64, canonical_width(n as u64)));
    if !w.vkeys.is_empty() {
        cbor::write_uint_canonical(&mut b, 0);
        arr(&mut b, w.vkeys.len());
        for v in &w.vkeys {
            arr(&mut b, 2);
            cbor::write_bytes_canonical(&mut b, v);
            cbor::write_bytes_canonical(&mut b, &[0u8; 64]);
        }
    }
    if !w.native.is_empty() {
        cbor::write_uint_canonical(&mut b, 1);
        arr(&mut b, w.native.len());
        for s in &w.native {
            b.extend_from_slice(s);
        }
    }
    if !w.plutus_v1.is_empty() {
        cbor::write_uint_canonical(&mut b, 3);
        arr(&mut b, w.plutus_v1.len());
        for s in &w.plutus_v1 {
            cbor::write_bytes_canonical(&mut b, s);
        }
    }
    b
}

fn info(w: &Wits) -> WitnessInfo {
    decode_witness_info_single(&witness_set(w)).unwrap()
}

fn tx_in(b: u8, index: u16) -> TxIn {
    TxIn { tx_hash: Hash32([b; 32]), index }
}

fn key_address(h: &Hash28) -> Vec<u8> {
    let mut a = vec![0x60];
    a.extend_from_slice(&h.0);
    a
}

fn script_address(h: &Hash28) -> Vec<u8> {
    let mut a = vec![0x70];
    a.extend_from_slice(&h.0);
    a
}

fn script_reward_account(h: &Hash28) -> Vec<u8> {
    let mut a = vec![0xf0];
    a.extend_from_slice(&h.0);
    a
}

const INPUT: u8 = 0xA0;

fn body() -> ConwayTxBody {
    ConwayTxBody {
        inputs: [tx_in(INPUT, 0)].into_iter().collect(),
        outputs: vec![BabbageTxOut {
            address: key_address(&Hash28([0x99; 28])),
            coin: Coin(1_000_000),
            multi_asset: None,
            datum_option: None,
            script_ref: None,
        }],
        fee: Coin(200_000),
        ttl: None,
        certs: None,
        withdrawals: None,
        metadata_hash: None,
        validity_interval_start: None,
        mint: None,
        script_data_hash: None,
        collateral_inputs: None,
        required_signers: None,
        network_id: None,
        collateral_return: None,
        total_collateral: None,
        reference_inputs: None,
        voting_procedures: None,
        proposal_procedures: None,
        treasury_value: None,
        donation: None,
    }
}

/// `mint = { policy => { "" => 1 } }` for each policy.
fn mint(policies: &[Hash28]) -> Vec<u8> {
    let mut b = Vec::new();
    cbor::write_map_header(&mut b, ContainerEncoding::Definite(policies.len() as u64, canonical_width(policies.len() as u64)));
    for p in policies {
        cbor::write_bytes_canonical(&mut b, &p.0);
        cbor::write_map_header(&mut b, ContainerEncoding::Definite(1, canonical_width(1)));
        cbor::write_bytes_canonical(&mut b, &[]);
        cbor::write_uint_canonical(&mut b, 1);
    }
    b
}

fn minting(policy: &Hash28) -> ConwayTxBody {
    let mut b = body();
    b.mint = Some(mint(std::slice::from_ref(policy)));
    b
}

/// The input spends a key-address output; extra outputs as given.
fn context(b: &ConwayTxBody, extra: &[(TxIn, ResolvedScriptOutput)]) -> ResolvedScriptContext {
    let mut m: BTreeMap<TxIn, ResolvedScriptOutput> = extra.iter().cloned().collect();
    m.entry(tx_in(INPUT, 0))
        .or_insert(ResolvedScriptOutput { address: key_address(&Hash28([0x11; 28])), script_ref: None });
    ResolvedScriptContext::resolve(b, |t| m.get(t).cloned()).expect("every input resolves")
}

fn reduced(b: &ConwayTxBody, w: &WitnessInfo) -> ReducedScriptVerdict {
    match native_script_verdict(b, w, None).unwrap() {
        NativeScriptVerdict::Reduced(v) => v,
        other => panic!("no context must give a reduced verdict, got {other:?}"),
    }
}

fn exact(b: &ConwayTxBody, w: &WitnessInfo, ctx: &ResolvedScriptContext) -> ExactScriptVerdict {
    match native_script_verdict(b, w, Some(ctx)).unwrap() {
        NativeScriptVerdict::Exact(v) => v,
        other => panic!("a context must give an exact verdict, got {other:?}"),
    }
}

fn not_validating(h: &Hash28, reason: NativeScriptFailure) -> ExactScriptVerdict {
    ExactScriptVerdict::Invalid(ScriptClosureFailures {
        not_validating: [(h.clone(), reason)].into_iter().collect(),
        ..Default::default()
    })
}

// ---------------------------------------------------------------------------
// CE-LV13-1: accept and reject
// ---------------------------------------------------------------------------

#[test]
fn lv13_a_needed_passing_native_script_is_accepted() {
    let (vk, k) = key(1);
    let s = sig(&k);
    let b = minting(&native_hash(&s));
    let w = info(&Wits { vkeys: vec![vk], native: vec![s], ..Default::default() });
    assert_eq!(exact(&b, &w, &context(&b, &[])), ExactScriptVerdict::Valid);
    assert_eq!(reduced(&b, &w), ReducedScriptVerdict::Undetermined, "a reduced pass is never 'valid'");
}

#[test]
fn lv13_the_same_tx_with_a_failing_native_script_is_rejected_at_both_levels() {
    let (_, k) = key(1);
    let s = sig(&k);
    let h = native_hash(&s);
    let b = minting(&h);
    let w = info(&Wits { native: vec![s], ..Default::default() });
    let reason = NativeScriptFailure::MissingRequiredSignature { key_hash: k };
    assert_eq!(
        reduced(&b, &w),
        ReducedScriptVerdict::DefiniteReject(ScriptError { script_hash: h.clone(), reason: reason.clone() })
    );
    assert_eq!(exact(&b, &w, &context(&b, &[])), not_validating(&h, reason.clone()));
    assert_eq!(
        check_script_witnesses(&b, &w, None),
        Err(LedgerError::NativeScriptFailed(ScriptError { script_hash: h, reason }))
    );
}

#[test]
fn lv13_failing_all_any_and_n_of_k_each_reject() {
    let (vk1, k1) = key(1);
    let (_, k2) = key(2);
    let cases: Vec<(&str, Vec<u8>, NativeScriptFailure)> = vec![
        ("all", all(&[sig(&k1), sig(&k2)]), NativeScriptFailure::MissingRequiredSignature { key_hash: k2.clone() }),
        ("any", any(&[sig(&k2)]), NativeScriptFailure::ThresholdNotMet { required: 1, provided: 0 }),
        ("empty any", any(&[]), NativeScriptFailure::ThresholdNotMet { required: 1, provided: 0 }),
        ("2-of-2", n_of_k(2, &[sig(&k1), sig(&k2)]), NativeScriptFailure::ThresholdNotMet { required: 2, provided: 1 }),
        ("1-of-0", n_of_k(1, &[]), NativeScriptFailure::ThresholdNotMet { required: 1, provided: 0 }),
    ];
    for (name, s, reason) in cases {
        let h = native_hash(&s);
        let b = minting(&h);
        let w = info(&Wits { vkeys: vec![vk1.clone()], native: vec![s], ..Default::default() });
        assert_eq!(
            reduced(&b, &w),
            ReducedScriptVerdict::DefiniteReject(ScriptError { script_hash: h.clone(), reason: reason.clone() }),
            "{name}"
        );
        assert_eq!(exact(&b, &w, &context(&b, &[])), not_validating(&h, reason), "{name}");
    }
    // And the passing counterparts.
    for s in [all(&[sig(&k1)]), any(&[sig(&k2), sig(&k1)]), n_of_k(1, &[sig(&k2), sig(&k1)]), all(&[])] {
        let b = minting(&native_hash(&s));
        let w = info(&Wits { vkeys: vec![vk1.clone()], native: vec![s], ..Default::default() });
        assert_eq!(exact(&b, &w, &context(&b, &[])), ExactScriptVerdict::Valid);
    }
}

#[test]
fn lv13_a_threshold_of_zero_or_below_passes_exactly_as_cardano() {
    let (_, k) = key(1);
    for n in [0, -1, -2, i64::MIN] {
        for subs in [vec![], vec![sig(&k)]] {
            let s = n_of_k(n, &subs);
            let b = minting(&native_hash(&s));
            let w = info(&Wits { native: vec![s], ..Default::default() });
            assert_eq!(exact(&b, &w, &context(&b, &[])), ExactScriptVerdict::Valid, "n = {n}");
            assert_eq!(reduced(&b, &w), ReducedScriptVerdict::Undetermined, "n = {n}");
        }
    }
}

// ---------------------------------------------------------------------------
// CE-LV13-2: timelocks against the transaction's validity interval
// ---------------------------------------------------------------------------

fn timelock_verdict(s: Vec<u8>, start: Option<u64>, ttl: Option<u64>) -> ExactScriptVerdict {
    let mut b = minting(&native_hash(&s));
    b.validity_interval_start = start.map(SlotNo);
    b.ttl = ttl.map(SlotNo);
    let w = info(&Wits { native: vec![s], ..Default::default() });
    exact(&b, &w, &context(&b, &[]))
}

#[test]
fn lv13_invalid_before_holds_from_the_lock_and_never_without_a_start() {
    let lock = 100;
    let h = native_hash(&invalid_before(lock));
    let fails = |start: Option<u64>| {
        not_validating(
            &h,
            NativeScriptFailure::TimelockStartNotMet { lock_start: SlotNo(lock), tx_start: start.map(SlotNo) },
        )
    };
    assert_eq!(timelock_verdict(invalid_before(lock), Some(lock - 1), None), fails(Some(lock - 1)));
    assert_eq!(timelock_verdict(invalid_before(lock), Some(lock), None), ExactScriptVerdict::Valid);
    assert_eq!(timelock_verdict(invalid_before(lock), Some(lock + 1), None), ExactScriptVerdict::Valid);
    assert_eq!(timelock_verdict(invalid_before(lock), None, Some(lock + 50)), fails(None));
}

#[test]
fn lv13_invalid_hereafter_holds_up_to_the_lock_and_never_without_an_expiry() {
    let lock = 200;
    let h = native_hash(&invalid_hereafter(lock));
    let fails = |ttl: Option<u64>| {
        not_validating(
            &h,
            NativeScriptFailure::TimelockExpiryNotMet { lock_expiry: SlotNo(lock), tx_expiry: ttl.map(SlotNo) },
        )
    };
    assert_eq!(timelock_verdict(invalid_hereafter(lock), None, Some(lock - 1)), ExactScriptVerdict::Valid);
    assert_eq!(timelock_verdict(invalid_hereafter(lock), None, Some(lock)), ExactScriptVerdict::Valid);
    assert_eq!(timelock_verdict(invalid_hereafter(lock), None, Some(lock + 1)), fails(Some(lock + 1)));
    assert_eq!(timelock_verdict(invalid_hereafter(lock), Some(0), None), fails(None));
}

#[test]
fn lv13_the_block_slot_does_not_decide_a_timelock() {
    // A block at slot 150 satisfies `slot >= 100` and `slot < 200`, so the old block-slot rule
    // passed both. Cardano: no start means invalid_before never holds, and an expiry of 250 is past
    // the hereafter lock. Neither verdict depends on any block slot: the authority has no slot input.
    assert!(matches!(
        timelock_verdict(invalid_before(100), None, Some(160)),
        ExactScriptVerdict::Invalid(_)
    ));
    assert!(matches!(
        timelock_verdict(invalid_hereafter(200), Some(140), Some(250)),
        ExactScriptVerdict::Invalid(_)
    ));
    // And the window the block slot falls in, as a tx interval, passes.
    assert_eq!(
        timelock_verdict(all(&[invalid_before(100), invalid_hereafter(200)]), Some(140), Some(160)),
        ExactScriptVerdict::Valid
    );
}

// ---------------------------------------------------------------------------
// CE-LV13-3: missing, reference and extraneous scripts
// ---------------------------------------------------------------------------

/// `script_ref = #6.24(bytes .cbor [0, native_script])`.
fn native_script_ref(script: &[u8]) -> Vec<u8> {
    let mut inner = Vec::new();
    arr(&mut inner, 2);
    int(&mut inner, 0);
    inner.extend_from_slice(script);
    let mut b = Vec::new();
    cbor::write_tag(&mut b, 24, canonical_width(24));
    cbor::write_bytes_canonical(&mut b, &inner);
    b
}

#[test]
fn lv13_a_missing_required_native_script_is_missing() {
    let h = native_hash(&sig(&key(1).1));
    let b = minting(&h);
    let w = info(&Wits::default());
    assert_eq!(
        exact(&b, &w, &context(&b, &[])),
        ExactScriptVerdict::Invalid(ScriptClosureFailures { missing: [h].into_iter().collect(), ..Default::default() })
    );
    assert_eq!(reduced(&b, &w), ReducedScriptVerdict::Undetermined, "missing needs the needed set");
}

/// Spends a script-address output whose script is supplied by a reference input.
fn spending_by_reference(script: &[u8]) -> (ConwayTxBody, ResolvedScriptContext, Hash28) {
    let h = native_hash(script);
    let mut b = body();
    b.inputs = [tx_in(0xB0, 0)].into_iter().collect();
    b.reference_inputs = Some([tx_in(0xB1, 0)].into_iter().collect());
    let ctx = context(
        &b,
        &[
            (tx_in(0xB0, 0), ResolvedScriptOutput { address: script_address(&h), script_ref: None }),
            (
                tx_in(0xB1, 0),
                ResolvedScriptOutput { address: key_address(&Hash28([0x22; 28])), script_ref: Some(native_script_ref(script)) },
            ),
        ],
    );
    (b, ctx, h)
}

#[test]
fn lv13_a_reference_script_satisfies_the_need_and_a_witness_copy_is_extraneous() {
    let (vk, k) = key(1);
    let s = sig(&k);
    let (b, ctx, h) = spending_by_reference(&s);
    let w = info(&Wits { vkeys: vec![vk.clone()], ..Default::default() });
    assert_eq!(exact(&b, &w, &ctx), ExactScriptVerdict::Valid);
    let w = info(&Wits { vkeys: vec![vk], native: vec![s], ..Default::default() });
    assert_eq!(
        exact(&b, &w, &ctx),
        ExactScriptVerdict::Invalid(ScriptClosureFailures { extraneous: [h].into_iter().collect(), ..Default::default() })
    );
}

#[test]
fn lv13_a_failing_needed_reference_native_script_does_not_validate() {
    let (_, k) = key(1);
    let s = sig(&k);
    let (b, ctx, h) = spending_by_reference(&s);
    let w = info(&Wits::default());
    assert_eq!(
        exact(&b, &w, &ctx),
        not_validating(&h, NativeScriptFailure::MissingRequiredSignature { key_hash: k })
    );
    // Without the UTxO the reference script is invisible: nothing can be proven.
    assert_eq!(reduced(&b, &w), ReducedScriptVerdict::Undetermined);
}

// ---------------------------------------------------------------------------
// CE-LV13-4: an extra passing script, at both evidence levels
// ---------------------------------------------------------------------------

#[test]
fn lv13_an_extra_passing_script_is_undetermined_without_context_and_extraneous_with_it() {
    let s = n_of_k(0, &[]);
    let h = native_hash(&s);
    let b = body();
    let w = info(&Wits { native: vec![s], ..Default::default() });
    assert_eq!(reduced(&b, &w), ReducedScriptVerdict::Undetermined);
    assert!(check_script_witnesses(&b, &w, None).is_ok(), "reduced: neither accepted as valid nor rejected");
    assert_eq!(
        exact(&b, &w, &context(&b, &[])),
        ExactScriptVerdict::Invalid(ScriptClosureFailures { extraneous: [h].into_iter().collect(), ..Default::default() })
    );
}

#[test]
fn lv13_an_extra_failing_script_is_a_definite_reject_and_exactly_extraneous() {
    // Not needed, so Cardano does not evaluate it; it is extraneous instead. Either way: rejected.
    let s = sig(&key(1).1);
    let h = native_hash(&s);
    let b = body();
    let w = info(&Wits { native: vec![s], ..Default::default() });
    assert!(matches!(reduced(&b, &w), ReducedScriptVerdict::DefiniteReject(_)));
    assert_eq!(
        exact(&b, &w, &context(&b, &[])),
        ExactScriptVerdict::Invalid(ScriptClosureFailures { extraneous: [h].into_iter().collect(), ..Default::default() })
    );
}

// ---------------------------------------------------------------------------
// CE-LV13-5: every purpose of getConwayScriptsNeeded
// ---------------------------------------------------------------------------

fn h(b: u8) -> Hash28 {
    Hash28([b; 28])
}

fn script_credential(buf: &mut Vec<u8>, hash: &Hash28) {
    arr(buf, 2);
    int(buf, 1);
    cbor::write_bytes_canonical(buf, &hash.0);
}

#[test]
fn lv13_the_needed_set_covers_every_conway_purpose() {
    let mut b = body();
    // Spending: a script-address input.
    b.inputs = [tx_in(0xC0, 0)].into_iter().collect();
    // Rewarding: a script reward account.
    let mut w = Vec::new();
    cbor::write_map_header(&mut w, ContainerEncoding::Definite(1, canonical_width(1)));
    cbor::write_bytes_canonical(&mut w, &script_reward_account(&h(2)));
    cbor::write_uint_canonical(&mut w, 0);
    b.withdrawals = Some(w);
    // Certifying: tag 0 (registration without deposit) needs no witness; tag 7 does.
    let mut c = Vec::new();
    arr(&mut c, 2);
    arr(&mut c, 2);
    int(&mut c, 0);
    script_credential(&mut c, &h(30));
    arr(&mut c, 3);
    int(&mut c, 7);
    script_credential(&mut c, &h(3));
    cbor::write_uint_canonical(&mut c, 2_000_000);
    b.certs = Some(c);
    // Minting.
    b.mint = Some(mint(&[h(4)]));
    // Voting: a script DRep voter needs its script; a stake-pool voter never does.
    let mut v = Vec::new();
    cbor::write_map_header(&mut v, ContainerEncoding::Definite(2, canonical_width(2)));
    arr(&mut v, 2);
    int(&mut v, 3);
    cbor::write_bytes_canonical(&mut v, &h(5).0);
    cbor::write_map_header(&mut v, ContainerEncoding::Definite(0, canonical_width(0)));
    arr(&mut v, 2);
    int(&mut v, 4);
    cbor::write_bytes_canonical(&mut v, &h(50).0);
    cbor::write_map_header(&mut v, ContainerEncoding::Definite(0, canonical_width(0)));
    b.voting_procedures = Some(v);
    // Proposing: guardrails on a parameter change; none on a treasury withdrawal without one.
    let proposal = |gov_action| ProposalProcedure {
        deposit: Coin(0),
        return_addr: script_reward_account(&h(60)),
        gov_action,
        anchor: Anchor { raw: vec![] },
    };
    b.proposal_procedures = Some(vec![
        proposal(GovAction::ParameterChange { prev_action: None, update: vec![0xa0], policy_hash: Some(h(6)) }),
        proposal(GovAction::TreasuryWithdrawals { withdrawals: vec![], policy_hash: None }),
        proposal(GovAction::InfoAction),
    ]);
    let ctx = context(
        &b,
        &[(tx_in(0xC0, 0), ResolvedScriptOutput { address: script_address(&h(1)), script_ref: None })],
    );
    let needed = scripts_needed(&b, &ctx).unwrap();
    let expected: BTreeSet<(ScriptPurpose, Hash28)> = [
        (ScriptPurpose::Spending, h(1)),
        (ScriptPurpose::Rewarding, h(2)),
        (ScriptPurpose::Certifying, h(3)),
        (ScriptPurpose::Minting, h(4)),
        (ScriptPurpose::Voting, h(5)),
        (ScriptPurpose::Proposing, h(6)),
    ]
    .into_iter()
    .collect();
    assert_eq!(needed.provenance, expected);
    // A Byron or key-address input needs nothing.
    let ctx = context(&b, &[(tx_in(0xC0, 0), ResolvedScriptOutput { address: key_address(&h(1)), script_ref: None })]);
    assert!(!scripts_needed(&b, &ctx).unwrap().hashes.contains(&h(1)));
}

// ---------------------------------------------------------------------------
// CE-LV13-6: hashes over preserved bytes; bootstrap signers
// ---------------------------------------------------------------------------

#[test]
fn lv13_a_native_script_hash_is_over_its_own_bytes_not_a_re_encoding() {
    let (vk, k) = key(1);
    let canonical = sig(&k);
    // The same script with a non-canonical array head (0x98 0x02 for array(2)).
    let mut odd = vec![0x98, 0x02];
    odd.extend_from_slice(&canonical[1..]);
    let (h_odd, h_canon) = (native_hash(&odd), native_hash(&canonical));
    assert_ne!(h_odd, h_canon);
    let w = info(&Wits { vkeys: vec![vk], native: vec![odd], ..Default::default() });
    assert_eq!(w.native_scripts[0].hash, h_odd);
    let b = minting(&h_odd);
    assert_eq!(exact(&b, &w, &context(&b, &[])), ExactScriptVerdict::Valid);
    let b = minting(&h_canon);
    assert_eq!(
        exact(&b, &w, &context(&b, &[])),
        ExactScriptVerdict::Invalid(ScriptClosureFailures {
            extraneous: [h_odd].into_iter().collect(),
            missing: [h_canon].into_iter().collect(),
            ..Default::default()
        })
    );
}

#[test]
fn lv13_a_plutus_witness_counts_toward_the_closure_but_is_not_evaluated_here() {
    let program = vec![0x4d, 0x01, 0x00, 0x00, 0x33, 0x22, 0x22, 0x20, 0x05, 0x12, 0x00, 0x12, 0x00, 0x11];
    let hp = prefixed_script_hash(0x01, &program);
    let b = minting(&hp);
    let w = info(&Wits { plutus_v1: vec![program], ..Default::default() });
    assert_eq!(w.plutus_script_hashes, [hp].into_iter().collect());
    assert_eq!(exact(&b, &w, &context(&b, &[])), ExactScriptVerdict::Valid);
}

#[test]
fn lv13_a_script_ref_decodes_to_the_hash_of_its_native_bytes() {
    let s = all(&[sig(&key(7).1), invalid_before(5)]);
    let (hash, _) = decode_script_ref(&native_script_ref(&s)).unwrap();
    assert_eq!(hash, native_hash(&s));
}

// ---------------------------------------------------------------------------
// Real preprod fixtures
// ---------------------------------------------------------------------------

const TIMELOCK_BLOCK: &[u8] = include_bytes!("fixtures/lv13_timelock_block.cbor");
const TIMELOCK_UTXO: &str = include_str!("fixtures/lv13_timelock_utxo.txt");
const REFNATIVE_BLOCK: &[u8] = include_bytes!("fixtures/lv13_refnative_block.cbor");
const REFNATIVE_UTXO: &str = include_str!("fixtures/lv13_refnative_utxo.txt");
const BOOTSTRAP_BLOCK: &[u8] = include_bytes!("fixtures/lv13_bootstrap_block.cbor");
const BOOTSTRAP_UTXO: &str = include_str!("fixtures/lv13_bootstrap_utxo.txt");

fn unhex(s: &str) -> Vec<u8> {
    (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap()).collect()
}

/// `slot S tx I` then one `txid#ix address script_ref|-` line per resolved input.
fn fixture_utxo(text: &str) -> (usize, BTreeMap<TxIn, ResolvedScriptOutput>) {
    let mut lines = text.lines();
    let head: Vec<&str> = lines.next().unwrap().split(' ').collect();
    let tx: usize = head[3].parse().unwrap();
    let mut m = BTreeMap::new();
    for l in lines {
        let f: Vec<&str> = l.split(' ').collect();
        let (id, ix) = f[0].split_once('#').unwrap();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&unhex(id));
        m.insert(
            TxIn { tx_hash: Hash32(hash), index: ix.parse().unwrap() },
            ResolvedScriptOutput {
                address: unhex(f[1]),
                script_ref: if f[2] == "-" { None } else { Some(unhex(f[2])) },
            },
        );
    }
    (tx, m)
}

fn elements(data: &[u8]) -> Vec<(usize, usize)> {
    let mut o = 0;
    let n = match cbor::read_array_header(data, &mut o).unwrap() {
        ContainerEncoding::Definite(n, _) => n,
        ContainerEncoding::Indefinite => panic!("fixtures use definite arrays"),
    };
    (0..n).map(|_| cbor::skip_item(data, &mut o).unwrap()).collect()
}

struct RealTx {
    body: ConwayTxBody,
    body_bytes: Vec<u8>,
    witness_bytes: Vec<u8>,
    info: WitnessInfo,
}

fn real_tx(envelope: &[u8], tx: usize) -> RealTx {
    let env = ade_codec::cbor::envelope::decode_block_envelope(envelope).unwrap();
    let block = ade_codec::conway::decode_conway_block(&envelope[env.block_start..env.block_end]).unwrap();
    let blk = block.decoded();
    let (bs, be) = elements(&blk.tx_bodies)[tx];
    let (ws, we) = elements(&blk.witness_sets)[tx];
    let body_bytes = blk.tx_bodies[bs..be].to_vec();
    let witness_bytes = blk.witness_sets[ws..we].to_vec();
    let body = ade_codec::conway::tx::decode_conway_tx_body(&body_bytes, &mut 0).unwrap();
    let info = decode_witness_info_single(&witness_bytes).unwrap();
    RealTx { body, body_bytes, witness_bytes, info }
}

fn real_context(t: &RealTx, utxo: &BTreeMap<TxIn, ResolvedScriptOutput>) -> ResolvedScriptContext {
    ResolvedScriptContext::resolve(&t.body, |i| utxo.get(i).cloned()).expect("the fixture resolves every input")
}

#[test]
fn lv13_a_real_timelock_mint_is_exactly_valid() {
    let (tx, utxo) = fixture_utxo(TIMELOCK_UTXO);
    let t = real_tx(TIMELOCK_BLOCK, tx);
    let ctx = real_context(&t, &utxo);
    let needed = scripts_needed(&t.body, &ctx).unwrap();
    let timelocked: Vec<_> = t
        .info
        .native_scripts
        .iter()
        .filter(|w| needed.hashes.contains(&w.hash))
        .filter(|w| {
            w.script.nodes().iter().any(|n| {
                matches!(
                    n,
                    ade_types::allegra::script::NativeScriptNode::InvalidBefore(_)
                        | ade_types::allegra::script::NativeScriptNode::InvalidHereafter(_)
                )
            })
        })
        .collect();
    assert!(!timelocked.is_empty(), "the fixture carries a needed timelock script");
    assert_eq!(exact(&t.body, &t.info, &ctx), ExactScriptVerdict::Valid);
    assert_eq!(reduced(&t.body, &t.info), ReducedScriptVerdict::Undetermined);
}

#[test]
fn lv13_a_real_reference_script_multisig_spend_is_exactly_valid() {
    let (tx, utxo) = fixture_utxo(REFNATIVE_UTXO);
    let t = real_tx(REFNATIVE_BLOCK, tx);
    let ctx = real_context(&t, &utxo);
    let needed = scripts_needed(&t.body, &ctx).unwrap();
    let refs = ade_ledger::tx_validity::script_witnesses::reference_scripts(&t.body, &ctx).unwrap();
    assert!(
        needed.hashes.iter().any(|h| matches!(
            refs.get(h),
            Some(ade_ledger::tx_validity::script_witnesses::ReferenceScript::Native(_))
        )),
        "a needed script arrives as a reference native script"
    );
    assert_eq!(exact(&t.body, &t.info, &ctx), ExactScriptVerdict::Valid);
    // Dropping the signers makes that reference multisig fail: exactly not-validating.
    let unsigned = WitnessInfo { available_key_hashes: BTreeSet::new(), ..t.info.clone() };
    match exact(&t.body, &unsigned, &ctx) {
        ExactScriptVerdict::Invalid(f) => {
            assert!(!f.not_validating.is_empty() && f.extraneous.is_empty() && f.missing.is_empty(), "{f:?}")
        }
        v => panic!("expected not-validating, got {v:?}"),
    }
}

#[test]
fn lv13_a_real_bootstrap_witness_key_hash_is_its_byron_address_root() {
    let (tx, utxo) = fixture_utxo(BOOTSTRAP_UTXO);
    let t = real_tx(BOOTSTRAP_BLOCK, tx);
    assert!(!t.info.bootstrap_key_hashes.is_empty());
    // Byron address = [#6.24(bytes .cbor [root, attributes, type]), crc]; root is its first item.
    let mut roots = BTreeSet::new();
    for out in utxo.values() {
        let a = &out.address;
        if a.first() != Some(&0x82) {
            continue;
        }
        let mut o = 1;
        cbor::read_tag(a, &mut o).unwrap();
        let (inner, _) = cbor::read_bytes(a, &mut o).unwrap();
        let mut i = 0;
        cbor::read_array_header(&inner, &mut i).unwrap();
        let (root, _) = cbor::read_bytes(&inner, &mut i).unwrap();
        let mut r = [0u8; 28];
        r.copy_from_slice(&root);
        roots.insert(Hash28(r));
    }
    assert!(!roots.is_empty(), "the fixture spends a Byron input");
    assert!(
        roots.is_subset(&t.info.bootstrap_key_hashes),
        "bootstrap key hashes {:?} must include the spent Byron roots {:?}",
        t.info.bootstrap_key_hashes,
        roots
    );
    // So a sig script on that root is satisfied by this tx's witnesses.
    let root = roots.into_iter().next().unwrap();
    assert!(script_signers(&t.info).contains(&root));
}

// ---------------------------------------------------------------------------
// CE-LV13-1 / CE-LV13-7 on the real fixture: the block path, the standalone path, replay
// ---------------------------------------------------------------------------

/// The timelock fixture with its needed timelock script's lock moved so the script fails,
/// keeping every byte length (so the block and witness set stay well formed).
fn failing_timelock_block() -> (Vec<u8>, usize) {
    let (tx, _) = fixture_utxo(TIMELOCK_UTXO);
    let t = real_tx(TIMELOCK_BLOCK, tx);
    let mut block = TIMELOCK_BLOCK.to_vec();
    let ws_at = block.windows(t.witness_bytes.len()).position(|w| w == t.witness_bytes).unwrap();
    // Find a timelock node `[4|5, uint]` inside the witness set and push the lock beyond reach.
    let ws = &t.witness_bytes;
    let mut done = false;
    for i in 0..ws.len().saturating_sub(3) {
        if ws[i] == 0x82 && (ws[i + 1] == 0x04 || ws[i + 1] == 0x05) && ws[i + 2] == 0x1a {
            let v: &[u8] = if ws[i + 1] == 0x04 { &[0xff; 4] } else { &[0x00; 4] };
            block[ws_at + i + 3..ws_at + i + 7].copy_from_slice(v);
            done = true;
            break;
        }
    }
    assert!(done, "the fixture's timelock uses a 4-byte slot");
    (block, tx)
}

fn reduced_state(inner: &[u8]) -> LedgerState {
    let decoded = ade_codec::conway::decode_conway_block(inner).unwrap();
    let slot = decoded.decoded().header.body.slot;
    let mut state = LedgerState::new(CardanoEra::Conway);
    state.track_utxo = false;
    state.epoch_state.epoch = mainnet_shelley_schedule().locate(SlotNo(slot)).unwrap().epoch;
    state.epoch_state.slot = SlotNo(slot.saturating_sub(1));
    state
}

fn inner(envelope: &[u8]) -> Vec<u8> {
    let env = ade_codec::cbor::envelope::decode_block_envelope(envelope).unwrap();
    envelope[env.block_start..env.block_end].to_vec()
}

#[test]
fn lv13_the_block_path_accepts_the_real_block_and_rejects_it_with_a_failing_timelock() {
    let good = inner(TIMELOCK_BLOCK);
    let ok = apply_block_with_verdicts(&reduced_state(&good), CardanoEra::Conway, &good, &mainnet_shelley_schedule())
        .expect("the real block applies");
    assert!(ok.verdict.native_script_passed > 0);

    let (bad_env, tx) = failing_timelock_block();
    let bad = inner(&bad_env);
    let t = real_tx(&bad_env, tx);
    let expected = match reduced(&t.body, &t.info) {
        ReducedScriptVerdict::DefiniteReject(e) => e,
        v => panic!("the mutated script must fail, got {v:?}"),
    };
    assert!(matches!(
        expected.reason,
        NativeScriptFailure::TimelockStartNotMet { .. } | NativeScriptFailure::TimelockExpiryNotMet { .. }
    ));
    let state = reduced_state(&bad);
    let first = apply_block_with_verdicts(&state, CardanoEra::Conway, &bad, &mainnet_shelley_schedule()).err();
    assert_eq!(first, Some(LedgerError::NativeScriptFailed(expected.clone())));
    // Replay: the same input gives the same structured error.
    let again = apply_block_with_verdicts(&state, CardanoEra::Conway, &bad, &mainnet_shelley_schedule()).err();
    assert_eq!(first, again);

    // The standalone path reaches the same seam with the same error.
    let mut tx_cbor = vec![0x84];
    tx_cbor.extend_from_slice(&t.body_bytes);
    tx_cbor.extend_from_slice(&t.witness_bytes);
    tx_cbor.extend_from_slice(&[0xf5, 0xf6]);
    match tx_validity(&state, &tx_cbor).verdict {
        TxValidityVerdict::Invalid { error, .. } => {
            assert_eq!(error, TxValidityError::Phase1(LedgerError::NativeScriptFailed(expected)))
        }
        TxValidityVerdict::Valid { .. } => panic!("the standalone path must reject the failing timelock"),
    }
}

#[test]
fn lv13_the_mutated_real_script_is_exactly_missing_and_extraneous() {
    // With the UTxO, the changed bytes change the hash: the needed script is missing and the
    // supplied one is extraneous. It is not evaluated, because it is not needed.
    let (utx, utxo) = fixture_utxo(TIMELOCK_UTXO);
    let (bad_env, tx) = failing_timelock_block();
    assert_eq!(tx, utx);
    let t = real_tx(&bad_env, tx);
    match exact(&t.body, &t.info, &real_context(&t, &utxo)) {
        ExactScriptVerdict::Invalid(f) => {
            assert!(f.not_validating.is_empty(), "{f:?}");
            assert_eq!(f.missing.len(), 1, "{f:?}");
            assert_eq!(f.extraneous.len(), 1, "{f:?}");
        }
        v => panic!("expected missing + extraneous, got {v:?}"),
    }
}

#[test]
fn lv13_verdicts_are_deterministic() {
    let (tx, utxo) = fixture_utxo(REFNATIVE_UTXO);
    let t = real_tx(REFNATIVE_BLOCK, tx);
    let ctx = real_context(&t, &utxo);
    let a = native_script_verdict(&t.body, &t.info, Some(&ctx));
    let b = native_script_verdict(&t.body, &t.info, Some(&ctx));
    assert_eq!(a, b);
    let unsigned = WitnessInfo { available_key_hashes: BTreeSet::new(), ..t.info.clone() };
    assert_eq!(check_script_witnesses(&t.body, &unsigned, Some(&ctx)), check_script_witnesses(&t.body, &unsigned, Some(&ctx)));
}
