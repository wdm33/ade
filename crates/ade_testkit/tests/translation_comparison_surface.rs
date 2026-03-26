//! Integration test: Translation comparison surface.
//!
//! Verifies that HFC translation functions preserve the oracle's
//! sub-state quantities: epoch, UTxO count, treasury, reserves.
//! A translation that changes any of these is semantically wrong.
//!
//! This is T-26A.1: narrowing the comparison surface to locate
//! where the encoding gap actually is.

use ade_ledger::hfc::translate_era;
use ade_ledger::state::{EpochState, LedgerState};
use ade_ledger::pparams::ProtocolParameters;
use ade_ledger::utxo::UTxOState;
use ade_types::{CardanoEra, EpochNo, SlotNo};
use ade_types::tx::Coin;

fn make_state(era: CardanoEra, epoch: u64, treasury: u64, reserves: u64) -> LedgerState {
    LedgerState {
        utxo_state: UTxOState::new(),
        epoch_state: EpochState {
            epoch: EpochNo(epoch),
            slot: SlotNo(0),
            snapshots: ade_ledger::epoch::SnapshotState::new(),
            reserves: Coin(reserves),
            treasury: Coin(treasury),
            block_production: std::collections::BTreeMap::new(),
            epoch_fees: ade_types::tx::Coin(0),
        },
        protocol_params: ProtocolParameters::default(),
        era,
        track_utxo: false,
        cert_state: ade_ledger::delegation::CertState::new(),
        max_lovelace_supply: 45_000_000_000_000_000,
    }
}

/// Verify a translation preserves epoch, treasury, reserves, and UTxO count.
fn verify_translation_preserves(
    label: &str,
    from_era: CardanoEra,
    to_era: CardanoEra,
    epoch: u64,
    treasury: u64,
    reserves: u64,
) {
    let pre = make_state(from_era, epoch, treasury, reserves);
    let post = translate_era(&pre, to_era).unwrap();

    assert_eq!(post.era, to_era, "{label}: era");
    assert_eq!(post.epoch_state.epoch.0, epoch, "{label}: epoch preserved");
    assert_eq!(post.epoch_state.treasury.0, treasury, "{label}: treasury preserved");
    assert_eq!(post.utxo_state.len(), 0, "{label}: UTxO count preserved (empty)");

    // Reserves may change for Byron→Shelley (computed from UTxO).
    // For all other transitions, reserves must be preserved.
    if from_era != CardanoEra::ByronRegular && from_era != CardanoEra::ByronEbb {
        assert_eq!(
            post.epoch_state.reserves.0, reserves,
            "{label}: reserves preserved"
        );
    }
}

#[test]
fn shelley_to_allegra_preserves_oracle_quantities() {
    // From oracle: Shelley→Allegra HFC at epoch 236
    verify_translation_preserves(
        "Shelley→Allegra",
        CardanoEra::Shelley,
        CardanoEra::Allegra,
        236,
        217_021_606_000_000,
        13_112_607_632_000_000,
    );
}

#[test]
fn allegra_to_mary_preserves_oracle_quantities() {
    verify_translation_preserves(
        "Allegra→Mary",
        CardanoEra::Allegra,
        CardanoEra::Mary,
        251,
        330_358_412_000_000,
        12_759_915_293_000_000,
    );
}

#[test]
fn mary_to_alonzo_preserves_oracle_quantities() {
    verify_translation_preserves(
        "Mary→Alonzo",
        CardanoEra::Mary,
        CardanoEra::Alonzo,
        290,
        609_002_818_000_000,
        11_860_319_467_000_000,
    );
}

#[test]
fn alonzo_to_babbage_preserves_oracle_quantities() {
    verify_translation_preserves(
        "Alonzo→Babbage",
        CardanoEra::Alonzo,
        CardanoEra::Babbage,
        365,
        1_037_476_762_000_000,
        10_049_419_651_000_000,
    );
}

#[test]
fn babbage_to_conway_preserves_oracle_quantities() {
    verify_translation_preserves(
        "Babbage→Conway",
        CardanoEra::Babbage,
        CardanoEra::Conway,
        507,
        1_528_154_948_000_000,
        7_816_251_181_000_000,
    );
}

/// Summary: all non-Byron transitions preserve oracle sub-state.
#[test]
fn all_non_byron_translations_preserve_sub_state() {
    let cases = [
        ("Shelley→Allegra", CardanoEra::Shelley, CardanoEra::Allegra, 236u64, 217_021_606_000_000u64, 13_112_607_632_000_000u64),
        ("Allegra→Mary", CardanoEra::Allegra, CardanoEra::Mary, 251, 330_358_412_000_000, 12_759_915_293_000_000),
        ("Mary→Alonzo", CardanoEra::Mary, CardanoEra::Alonzo, 290, 609_002_818_000_000, 11_860_319_467_000_000),
        ("Alonzo→Babbage", CardanoEra::Alonzo, CardanoEra::Babbage, 365, 1_037_476_762_000_000, 10_049_419_651_000_000),
        ("Babbage→Conway", CardanoEra::Babbage, CardanoEra::Conway, 507, 1_528_154_948_000_000, 7_816_251_181_000_000),
    ];

    eprintln!("\n=== Translation Sub-State Preservation ===");
    eprintln!("{:<20} {:>5} {:>18} {:>18} Status", "Transition", "Epoch", "Treasury (ADA)", "Reserves (ADA)");
    eprintln!("{}", "-".repeat(75));

    for (label, from, to, epoch, treasury, reserves) in &cases {
        let pre = make_state(*from, *epoch, *treasury, *reserves);
        let post = translate_era(&pre, *to).unwrap();

        let ok = post.epoch_state.epoch.0 == *epoch
            && post.epoch_state.treasury.0 == *treasury
            && post.epoch_state.reserves.0 == *reserves
            && post.era == *to;

        let t_ada = *treasury / 1_000_000;
        let r_ada = *reserves / 1_000_000;

        eprintln!(
            "{:<20} {:>5} {:>18} {:>18} {}",
            label, epoch, t_ada, r_ada,
            if ok { "✓ preserved" } else { "✗ MISMATCH" }
        );

        assert!(ok, "{label}: sub-state not preserved across translation");
    }
    eprintln!("==========================================\n");
}
