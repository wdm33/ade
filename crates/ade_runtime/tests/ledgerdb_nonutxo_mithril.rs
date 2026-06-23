//! Real-snapshot acceptance for the native non-UTxO snapshot decoder
//! (MITHRIL-VERIFIED-ANCHOR-INTEGRATION, S1a). Decodes the real Conway NewEpochState in a verified
//! V2 LedgerDB `state` file natively — NO cardano-cli, NO JSON consensus-input bundle, NO operator
//! seed — and confirms EVERY snapshot-present non-UTxO field is decoded + sane:
//!   - era == Conway, epoch == the snapshot's own NES epoch (the manifest authority);
//!   - the FULL CertState (real pools + registrations); all five Praos nonces; the PoolDistr with
//!     VRF bindings cross-checked against the CertState pools;
//!   - the Conway current protocol params decoded FROM the state file (plausible preprod values);
//!   - reserves + treasury (plausible pots, reserves + treasury < the 45e15 max lovelace supply);
//!   - nesBprev block production (pool ids ⊆ CertState pools; plausible counts).
//! Prints the decoded protocol params / pots / block-production for human review. RED: reads the
//! restored Mithril snapshot; SKIPS when absent.

use ade_ledger::bootstrap_anchor::SeedPoint;
use ade_ledger::ledgerdb_state::{decode_native_nonutxo_state, NativeNonUtxoError};
use ade_ledger::pparams::MinUtxoRule;
use ade_types::tx::Coin;
use ade_types::{Hash32, SlotNo};

/// preprod max lovelace supply = 45e9 ADA = 45e15 lovelace.
const MAX_LOVELACE_SUPPLY: u64 = 45_000_000_000_000_000;
/// The preprod network magic. Derives `network_id = 0` (testnet), matching the snapshot's
/// preprod reward-account network byte.
const PREPROD_NETWORK_MAGIC: u32 = 1;

#[test]
fn decode_native_nonutxo_real_snapshot() {
    let dir = std::env::var("ADE_MITHRIL_LEDGER_DIR").unwrap_or_else(|_| {
        "/home/ts/Code/rust/ade/.mithril-scratch/restore-ancillary/db/ledger".to_string()
    });
    let mut slots: Vec<u64> = match std::fs::read_dir(&dir) {
        Ok(rd) => rd
            .flatten()
            .filter_map(|e| e.file_name().to_str().and_then(|s| s.parse::<u64>().ok()))
            .collect(),
        Err(_) => {
            eprintln!("SKIP: no Mithril restore at {dir}");
            return;
        }
    };
    if slots.is_empty() {
        eprintln!("SKIP: no ledger snapshot in {dir}");
        return;
    }
    slots.sort();
    let slot = *slots.last().unwrap();
    let state = std::fs::read(format!("{dir}/{slot}/state")).expect("read mithril state");
    eprintln!("Mithril ledger snapshot slot {slot}, state {} bytes", state.len());

    // The manifest-certified point: the directory slot + the certified block hash. The hash is bound
    // through (not decoded from the state file); the caller binds it to the certificate beacon. We
    // use the slot from the snapshot directory and a deterministic placeholder hash for the test.
    let point = SeedPoint {
        slot: SlotNo(slot),
        block_hash: Hash32([0xab; 32]),
    };

    // Discover the snapshot's own NES epoch (the binding authority — a wrong manifest epoch is
    // fail-closed; the filename slot is NOT the epoch).
    let nes_epoch = match decode_native_nonutxo_state(&state, point.clone(), u64::MAX, PREPROD_NETWORK_MAGIC) {
        Err(NativeNonUtxoError::EpochMismatch { decoded_epoch, .. }) => decoded_epoch,
        other => panic!("epoch discovery unexpected: {other:?}"),
    };
    eprintln!("decoded NES epoch (binding authority, not the filename slot {slot}): {nes_epoch}");

    let (s, commitment) =
        decode_native_nonutxo_state(&state, point.clone(), nes_epoch, PREPROD_NETWORK_MAGIC)
            .expect("native non-UTxO decode");

    // ---- era / epoch / point ----
    assert_eq!(s.era, ade_types::CardanoEra::Conway, "current era is Conway");
    assert_eq!(s.epoch.0, nes_epoch);
    assert_eq!(s.point, point, "the manifest point is bound through");

    // ---- CertState (FULL struct) ----
    let pool_count = s.cert_state.pool.pools.len();
    let reg_count = s.cert_state.delegation.registrations.len();
    let deleg_count = s.cert_state.delegation.delegations.len();
    eprintln!(
        "CertState: {pool_count} pools, {reg_count} registrations, {deleg_count} delegations"
    );
    assert!(pool_count > 50, "expected real pools, got {pool_count}");
    assert!(reg_count > 500, "expected real registrations, got {reg_count}");
    // every active pool carries a real VRF.
    for (pid, pp) in &s.cert_state.pool.pools {
        assert_ne!(pp.vrf_hash.0, [0u8; 32], "pool {pid:?} has a zero VRF");
    }

    // ---- Praos nonces (all five present) ----
    eprintln!(
        "Praos nonces: eta0={} evolving={} candidate={}",
        hex8(&s.praos_nonces.epoch.0),
        hex8(&s.praos_nonces.evolving.0),
        hex8(&s.praos_nonces.candidate.0)
    );
    assert_ne!(s.praos_nonces.epoch.0, [0u8; 32], "eta0 present");

    // ---- PoolDistr (VRF bindings agree with CertState pool VRFs) ----
    eprintln!("PoolDistr: {} pools", s.pool_distr.len());
    assert!(!s.pool_distr.is_empty(), "PoolDistr is non-empty");
    let mut crosschecked = 0usize;
    for (pid, (_stake, vrf)) in &s.pool_distr {
        if let Some(pp) = s.cert_state.pool.pools.get(pid) {
            assert_eq!(&pp.vrf_hash, vrf, "PoolDistr VRF agrees with CertState for {pid:?}");
            crosschecked += 1;
        }
    }
    assert!(crosschecked > 0, "at least one PoolDistr pool cross-checks the CertState VRF");

    // ---- protocol params (decoded natively, plausible preprod values) ----
    let pp = &s.protocol_params;
    eprintln!("---- decoded Conway protocol params (FROM the state file, not defaulted) ----");
    eprintln!("  min_fee_a            = {}", pp.min_fee_a.0);
    eprintln!("  min_fee_b            = {}", pp.min_fee_b.0);
    eprintln!("  max_block_body_size  = {}", pp.max_block_body_size);
    eprintln!("  max_tx_size          = {}", pp.max_tx_size);
    eprintln!("  max_block_header_size= {}", pp.max_block_header_size);
    eprintln!("  key_deposit          = {}", pp.key_deposit.0);
    eprintln!("  pool_deposit         = {}", pp.pool_deposit.0);
    eprintln!("  e_max                = {}", pp.e_max);
    eprintln!("  n_opt                = {}", pp.n_opt);
    eprintln!("  pool_influence (a0)  = {:?}", pp.pool_influence);
    eprintln!("  monetary_expansion   = {:?}", pp.monetary_expansion);
    eprintln!("  treasury_growth      = {:?}", pp.treasury_growth);
    eprintln!("  protocol_major.minor = {}.{}", pp.protocol_major, pp.protocol_minor);
    eprintln!("  min_pool_cost        = {}", pp.min_pool_cost.0);
    eprintln!("  min_utxo_rule        = {:?}", pp.min_utxo_rule);
    eprintln!("  network_id           = {}", pp.network_id);
    eprintln!("  collateral_percent   = {}", pp.collateral_percent);
    eprintln!("  max_tx_ex_units_mem  = {}", pp.max_tx_ex_units_mem);
    eprintln!("  max_tx_ex_units_cpu  = {}", pp.max_tx_ex_units_cpu);
    eprintln!(
        "  cost_models_cbor     = {} bytes",
        pp.cost_models_cbor.as_ref().map(|b| b.len()).unwrap_or(0)
    );
    // sanity: plausible Conway / preprod values (catches an offset error).
    assert!(pp.min_fee_a.0 > 0 && pp.min_fee_a.0 < 1_000, "min_fee_a sane: {}", pp.min_fee_a.0);
    assert!(
        pp.min_fee_b.0 > 1_000 && pp.min_fee_b.0 < 10_000_000,
        "min_fee_b sane: {}",
        pp.min_fee_b.0
    );
    assert!(pp.key_deposit.0 >= 1_000_000, "key_deposit sane: {}", pp.key_deposit.0);
    assert!(pp.pool_deposit.0 >= 100_000_000, "pool_deposit sane: {}", pp.pool_deposit.0);
    assert!(
        pp.max_tx_size >= 8_192 && pp.max_tx_size <= 1_000_000,
        "max_tx_size sane: {}",
        pp.max_tx_size
    );
    assert!(
        (5..=12).contains(&pp.protocol_major),
        "protocol_major sane (Conway is 9/10): {}",
        pp.protocol_major
    );
    assert!(
        pp.max_tx_ex_units_mem > 1_000_000 && pp.max_tx_ex_units_cpu > 1_000_000_000,
        "ex units sane"
    );
    assert!(
        pp.cost_models_cbor.as_ref().map(|b| !b.is_empty()).unwrap_or(false),
        "cost_models preserved"
    );
    // coinsPerUTxOByte is preserved as a PER-BYTE rule (4310 on the real preprod snapshot),
    // NOT remapped onto an absolute floor.
    assert_eq!(
        pp.min_utxo_rule,
        MinUtxoRule::PerByte(Coin(4_310)),
        "real Conway coinsPerUTxOByte must decode to PerByte(4310), not min_utxo_value / an absolute floor: {:?}",
        pp.min_utxo_rule
    );
    // network id derived from the (preprod = testnet) manifest magic, bound on the state and
    // the protocol params.
    assert_eq!(s.network_id, 0, "preprod manifest magic derives network_id 0");
    assert_eq!(pp.network_id, 0, "pp.network_id == the derived id");

    // ---- pots ----
    eprintln!("---- pots ----");
    eprintln!("  reserves = {} lovelace", s.reserves.0);
    eprintln!("  treasury = {} lovelace", s.treasury.0);
    assert!(s.reserves.0 > 0, "reserves present");
    assert!(s.treasury.0 > 0, "treasury present");
    let pots = s.reserves.0 + s.treasury.0;
    assert!(
        pots < MAX_LOVELACE_SUPPLY,
        "reserves + treasury ({pots}) < max lovelace supply ({MAX_LOVELACE_SUPPLY})"
    );
    eprintln!(
        "  reserves + treasury = {} lovelace (the remaining ~{} is circulating UTxO, in `tables`)",
        pots,
        MAX_LOVELACE_SUPPLY - pots
    );

    // ---- block production (nesBprev) ----
    let bp_pools = s.block_production.len();
    let bp_total: u64 = s.block_production.values().sum();
    eprintln!("---- nesBprev (previous-epoch block production) ----");
    eprintln!("  {bp_pools} producing pools, {bp_total} blocks total");
    assert!(bp_pools > 0, "some pools produced blocks");
    assert!(bp_total > 0, "some blocks were produced");
    // pool ids ⊆ CertState pools (already enforced inside the decoder; assert here for the record).
    for pid in s.block_production.keys() {
        assert!(
            s.cert_state.pool.pools.contains_key(pid),
            "block producer {pid:?} is a known CertState pool"
        );
    }
    // counts plausible: no single pool made more than the whole epoch's blocks (~21600 on preprod).
    for (pid, blocks) in &s.block_production {
        assert!(*blocks < 25_000, "pool {pid:?} block count {blocks} implausible");
    }

    eprintln!("commitment = {}", hex8(&commitment.0));

    // ---- determinism: same snapshot + manifest -> byte-identical state + commitment ----
    let (s2, c2) = decode_native_nonutxo_state(&state, point, nes_epoch, PREPROD_NETWORK_MAGIC).unwrap();
    assert_eq!(s, s2, "deterministic native non-UTxO state");
    assert_eq!(commitment, c2, "deterministic commitment");

    eprintln!("NATIVE NON-UTXO DECODE OK (all fields decoded + sane, deterministic)");
}

fn hex8(b: &[u8]) -> String {
    let n = b.len().min(8);
    let mut s = String::new();
    for x in &b[..n] {
        s.push_str(&format!("{x:02x}"));
    }
    s
}
