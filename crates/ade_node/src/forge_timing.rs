// Core Contract:
// - GREEN: transports reviewed venue constants inward; owns no conversion
// - The conversion and every binding check live in ade_core (BLUE)
// - Deterministic: no wall-clock, no peer, no rand
// - Structured, closed errors; fail closed

//! LIVE-2c ACTIVATION part 1 — the committed venue TIMING registry, and the one place a
//! bootstrap-bound forge slot authority is established.
//!
//! **The store selects the calendar, not the operator.** The seed-epoch sidecar records the
//! network's genesis hash at import (bound there against the committed [`NetworkProfile`]); this
//! module resolves the timing calendar BY that hash. `--network` cannot choose it. That discharges
//! proof 5 of the LIVE-2c six-proof bar ("no operator configuration can supply or override it") by
//! construction rather than by review, and it is why a `--network` that disagrees with the store is
//! a terminal error here rather than a silently preferred value.
//!
//! There is deliberately no second network registry: identity (magic, genesis hash, k, f, epoch
//! length) stays in [`crate::bootstrap_export::resolve_network_profile`], and this table adds ONLY
//! calendar geometry, keyed by the same network ids. A venue present here but absent there cannot
//! resolve, because the genesis hash it must match comes from the profile.
//!
//! What each check actually pins — stated because the strengths differ and a reader should not have
//! to infer it:
//!
//! | fact | pinned by |
//! |---|---|
//! | which venue's calendar is used | the DURABLE sidecar genesis hash (unforgeable by CLI) |
//! | segment boundaries (start slots, epoch lengths) | the DURABLE sidecar epoch geometry |
//! | the timing ORIGIN (system start) | the operator's real `shelley-genesis.json`, fail-closed |
//! | the ACTIVE segment's slot length | the operator's real `shelley-genesis.json`, fail-closed |
//! | HISTORICAL segment slot lengths | the committed reviewed table below + its venue provenance |
//!
//! The last row is the one the durable binding cannot reach (a calendar with correct boundaries but
//! a wrong historical slot duration reproduces the store's epoch geometry exactly —
//! `the_durable_epoch_binding_pins_boundaries_not_slot_durations` records that limit as a test).
//! It has the same standing as `security_param` / `active_slots_coeff` / `epoch_length` in the
//! profile registry: a reviewed, closed, per-network constant, never operator-supplied.

use ade_core::consensus::era_schedule::{
    BootstrapBoundTimingAuthority, BootstrapTimingBinding, TimingAuthorityError, VenueTimingHistory,
    VenueTimingSegment,
};
use ade_ledger::seed_consensus_inputs::SeedEpochConsensusInputs;
use ade_types::{EpochNo, Hash32, SlotNo};

use crate::bootstrap_export::resolve_network_profile;

/// The closed set of venues whose calendar Ade commits to. Adding one = a reviewed entry in
/// [`venue_timing_history`] + an entry in the profile registry + a test.
const VENUE_IDS: [&str; 2] = ["preprod", "preview"];

/// Closed, secret-free reason a forge timing authority could not be established.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ForgeTimingError {
    /// The store's genesis hash matches no committed venue. Fail closed — never a default calendar.
    UnknownVenueGenesis { store_genesis: Hash32 },
    /// `--network` names a venue whose genesis hash is not the store's. The store is the authority;
    /// a disagreeing CLI is terminal, never silently preferred (mirrors the DC-EPOCH-16 RSW
    /// cross-check posture).
    NetworkDisagreesWithStore {
        cli_network: String,
        cli_genesis: Hash32,
        store_genesis: Hash32,
    },
    /// The operator's `shelley-genesis.json` disagrees with the committed timing origin.
    GenesisSystemStartMismatch { committed_ms: u64, genesis_ms: u64 },
    /// The operator's `shelley-genesis.json` disagrees with the committed ACTIVE slot length.
    ActiveSlotLengthMismatch { committed_ms: u32, genesis_ms: u32 },
    /// The calendar could not be bound to the durable bootstrap facts.
    Authority(TimingAuthorityError),
}

impl std::fmt::Display for ForgeTimingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownVenueGenesis { store_genesis } => write!(
                f,
                "no committed venue timing calendar for the store's genesis hash {store_genesis:?} \
                 -- the registry is closed (preprod|preview)"
            ),
            Self::NetworkDisagreesWithStore {
                cli_network,
                cli_genesis,
                store_genesis,
            } => write!(
                f,
                "--network {cli_network} has genesis {cli_genesis:?} but the STORE was bootstrapped \
                 from {store_genesis:?}; the store is the timing authority"
            ),
            Self::GenesisSystemStartMismatch {
                committed_ms,
                genesis_ms,
            } => write!(
                f,
                "shelley-genesis systemStart {genesis_ms} ms disagrees with the committed venue \
                 timing origin {committed_ms} ms"
            ),
            Self::ActiveSlotLengthMismatch {
                committed_ms,
                genesis_ms,
            } => write!(
                f,
                "shelley-genesis slotLength {genesis_ms} ms disagrees with the committed active \
                 segment slot length {committed_ms} ms"
            ),
            Self::Authority(e) => write!(
                f,
                "the venue timing calendar could not be bound to the durable bootstrap facts: {e:?}"
            ),
        }
    }
}

impl std::error::Error for ForgeTimingError {}

/// The committed venue calendar. TIMING + CALENDAR geometry only — no era identity, no ledger rule.
///
/// Every value is a fact from that venue's own genesis configuration, recorded here with its
/// provenance so a reviewer can re-derive it rather than trust it:
///
/// **preprod** — `byron-genesis.json`: `startTime = 1654041600` (s), `blockVersionData.slotDuration
/// = 20000` (ms), `protocolConsts.k = 2160` ⇒ Byron epochs of `10k = 21_600` slots. Shelley hard
/// fork at epoch 4 ⇒ the Byron segment is `4 × 21_600 = 86_400` slots. `shelley-genesis.json`:
/// `systemStart = 2022-06-01T00:00:00Z` — the IDENTICAL instant as the Byron start, which is what
/// makes one origin serve the whole calendar; `slotLength = 1` s; `epochLength = 432_000`.
///
/// **preview** — `config.json`: `TestShelleyHardForkAtEpoch = 0`, so the Byron segment has ZERO
/// slots and preview is a single 1 s segment from slot 0. `shelley-genesis.json`: `systemStart =
/// 2022-10-25T00:00:00Z`, `slotLength = 1` s, `epochLength = 86_400`. It takes the identical code
/// path — the number of segments is data, never a venue branch.
fn venue_timing_history(network_id: &str) -> Option<VenueTimingHistory> {
    match network_id {
        "preprod" => Some(VenueTimingHistory {
            system_start_unix_ms: 1_654_041_600_000,
            segments: vec![
                VenueTimingSegment {
                    start_slot: SlotNo(0),
                    start_epoch: EpochNo(0),
                    slot_length_ms: 20_000,
                    epoch_length_slots: 21_600,
                },
                VenueTimingSegment {
                    start_slot: SlotNo(86_400),
                    start_epoch: EpochNo(4),
                    slot_length_ms: 1_000,
                    epoch_length_slots: 432_000,
                },
            ],
        }),
        "preview" => Some(VenueTimingHistory {
            system_start_unix_ms: 1_666_656_000_000,
            segments: vec![VenueTimingSegment {
                start_slot: SlotNo(0),
                start_epoch: EpochNo(0),
                slot_length_ms: 1_000,
                epoch_length_slots: 86_400,
            }],
        }),
        _ => None,
    }
}

/// Resolve the venue calendar from the DURABLE genesis hash. The operator has no say.
pub fn venue_timing_history_for_genesis(
    store_genesis: &Hash32,
) -> Option<(&'static str, VenueTimingHistory)> {
    for id in VENUE_IDS {
        let profile = match resolve_network_profile(id) {
            Ok(p) => p,
            Err(_) => continue,
        };
        if &profile.genesis_hash == store_genesis {
            return venue_timing_history(id).map(|h| (profile.id, h));
        }
    }
    None
}

/// The operator `shelley-genesis.json` facts this module cross-checks against the committed
/// calendar. Both are already parsed by the forge key-ingress path — nothing new is read from disk,
/// and neither is ever an AUTHORITY: a disagreement is terminal, not a source of truth.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenesisTimingCrossCheck {
    pub system_start_unix_ms: u64,
    pub active_slot_length_ms: u32,
}

/// LIVE-2c — establish the ONE bootstrap-bound wall-clock→slot authority the `--mode node` producer
/// path uses.
///
/// Deterministic and side-effect-free apart from one emit-only provenance line. Same sidecar + same
/// committed table ⇒ byte-identical authority, so a warm restart RECONSTRUCTS rather than re-mints
/// (the property `derive_for_bootstrap_anchor` exists to give).
///
/// `cli_network` and `genesis` are CROSS-CHECKS only. Passing `None` for the genesis facts (a
/// forge-off / keyless start) skips only that check; it never relaxes the durable binding.
pub fn establish_forge_timing_authority(
    sidecar: &SeedEpochConsensusInputs,
    cli_network: &str,
    genesis: Option<GenesisTimingCrossCheck>,
) -> Result<BootstrapBoundTimingAuthority, ForgeTimingError> {
    let (venue_id, history) = venue_timing_history_for_genesis(&sidecar.genesis_hash).ok_or(
        ForgeTimingError::UnknownVenueGenesis {
            store_genesis: sidecar.genesis_hash.clone(),
        },
    )?;
    // A restart CLI naming a DIFFERENT venue is terminal. An unknown/absent `--network` is not: the
    // store already selected the calendar, so there is simply no cross-check to run.
    if let Ok(cli_profile) = resolve_network_profile(cli_network) {
        if cli_profile.genesis_hash != sidecar.genesis_hash {
            return Err(ForgeTimingError::NetworkDisagreesWithStore {
                cli_network: cli_network.to_string(),
                cli_genesis: cli_profile.genesis_hash,
                store_genesis: sidecar.genesis_hash.clone(),
            });
        }
    }
    if let Some(g) = genesis {
        if g.system_start_unix_ms != history.system_start_unix_ms {
            return Err(ForgeTimingError::GenesisSystemStartMismatch {
                committed_ms: history.system_start_unix_ms,
                genesis_ms: g.system_start_unix_ms,
            });
        }
        // The ACTIVE segment is the last one -- the geometry in force now.
        let active = history
            .segments
            .last()
            .map(|s| s.slot_length_ms)
            .unwrap_or(0);
        if g.active_slot_length_ms != active {
            return Err(ForgeTimingError::ActiveSlotLengthMismatch {
                committed_ms: active,
                genesis_ms: g.active_slot_length_ms,
            });
        }
    }
    let binding = BootstrapTimingBinding {
        anchor_slot: sidecar.seed_point_slot,
        epoch: sidecar.epoch_no,
        epoch_start_slot: sidecar.epoch_start_slot,
        epoch_length_slots: sidecar.epoch_length_slots,
    };
    let authority = BootstrapBoundTimingAuthority::establish(&history, binding)
        .map_err(ForgeTimingError::Authority)?;
    // Emit-only PROVENANCE, not just the value. The prior defect was a slot anchor that looked
    // plausible in every log it appeared in; a number alone cannot distinguish "the store chose this
    // calendar" from "the CLI did" from "a default did". Recorded once, before any forge depends on
    // it. Never read back.
    crate::node_log!(
        "live2c-timing-authority: source=durable-genesis-hash venue={} store_genesis={:?} \
         cli_network={} bootstrap_epoch={:?} durable_epoch_start_slot={} anchor_slot={} \
         domain_start_ms={} cadence_ms={} commitment={:?} genesis_cross_check={}",
        venue_id,
        sidecar.genesis_hash,
        cli_network,
        sidecar.epoch_no,
        sidecar.epoch_start_slot.0,
        authority.binding().anchor_slot.0,
        authority.anchor().domain_start_ms(),
        authority.slot_cadence_ms(),
        authority.source_schedule_commitment(),
        match genesis {
            Some(_) => "agreed",
            None => "not supplied (forge-off / no genesis file)",
        }
    );
    Ok(authority)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use ade_core::consensus::vrf_cert::ActiveSlotsCoeff;
    use ade_ledger::seed_consensus_inputs::SeedEpochConsensusInputs;
    use ade_core::consensus::praos_state::Nonce;
    use std::collections::BTreeMap;

    /// The DURABLE facts recorded by `~/.cardano-live1/ade-preprod-s7` at import, verbatim from its
    /// own bootstrap receipt + `nonce1-seed-quad` line.
    const S7_GENESIS_HEX: &str = "162d29c4e1cf6b8a84f2d692e67a3ac6bc7851bc3e6e4afe64d15778bed8bd86";
    const S7_EPOCH: u64 = 304;
    const S7_EPOCH_START_SLOT: u64 = 129_686_400;
    const S7_SEED_POINT_SLOT: u64 = 129_813_427;
    /// The preserved LIVE fixture (docs/evidence/.../live2b-slot-authority-discriminators.txt).
    const CAPTURED_MS: u64 = 1_786_021_761_000;
    const EXPECTED_SLOT: u64 = 130_338_561;

    fn hash32(hex: &str) -> Hash32 {
        let mut a = [0u8; 32];
        for (i, b) in a.iter_mut().enumerate() {
            *b = u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).unwrap();
        }
        Hash32(a)
    }

    fn s7_sidecar() -> SeedEpochConsensusInputs {
        SeedEpochConsensusInputs {
            anchor_fp: Hash32([0u8; 32]),
            epoch_no: EpochNo(S7_EPOCH),
            epoch_start_slot: SlotNo(S7_EPOCH_START_SLOT),
            epoch_length_slots: 432_000,
            epoch_nonce: Nonce(Hash32([1u8; 32])),
            genesis_hash: hash32(S7_GENESIS_HEX),
            protocol_params_hash: Hash32([2u8; 32]),
            seed_point_slot: SlotNo(S7_SEED_POINT_SLOT),
            seed_point_hash: Hash32([3u8; 32]),
            active_slots_coeff: ActiveSlotsCoeff { numer: 1, denom: 20 },
            security_param: 2160,
            total_active_stake: 0,
            pool_distribution: BTreeMap::new(),
        }
    }

    fn preprod_genesis_facts() -> GenesisTimingCrossCheck {
        GenesisTimingCrossCheck {
            system_start_unix_ms: 1_654_041_600_000,
            active_slot_length_ms: 1_000,
        }
    }

    /// CE-L2c-A1 + CE-L2c-5: the LIVE store's own durable facts establish the authority, and it
    /// converts the preserved live instant to the measured slot.
    #[test]
    fn ce_l2c_a1_the_live_store_facts_establish_and_convert_the_measured_instant() {
        let auth = establish_forge_timing_authority(
            &s7_sidecar(),
            "preprod",
            Some(preprod_genesis_facts()),
        )
        .expect("the live venue's own durable facts must establish");
        assert_eq!(auth.slot_at(CAPTURED_MS).unwrap(), SlotNo(EXPECTED_SLOT));
        assert_eq!(auth.slot_cadence_ms(), 1_000);
    }

    /// CE-L2c-A1 (the half that matters): the calendar is chosen by the STORE. A `--network` naming
    /// a different venue is TERMINAL — it can neither select nor override.
    #[test]
    fn the_operator_cannot_choose_the_calendar() {
        let err = establish_forge_timing_authority(
            &s7_sidecar(),
            "preview", // a real, known venue -- and the wrong one for this store
            Some(preprod_genesis_facts()),
        )
        .expect_err("a --network disagreeing with the store must be terminal");
        assert!(
            matches!(err, ForgeTimingError::NetworkDisagreesWithStore { .. }),
            "got {err:?}"
        );
        // ...and an unknown/absent --network is NOT an error: the store already chose, so there is
        // simply no cross-check to run. This is what makes the store the sole selector.
        let auth =
            establish_forge_timing_authority(&s7_sidecar(), "", Some(preprod_genesis_facts()))
                .expect("no CLI venue to cross-check against is not a failure");
        assert_eq!(auth.slot_at(CAPTURED_MS).unwrap(), SlotNo(EXPECTED_SLOT));
    }

    /// A store from a venue Ade has not committed a calendar for fails closed — never a default.
    #[test]
    fn an_uncommitted_venue_fails_closed() {
        let mut s = s7_sidecar();
        s.genesis_hash = Hash32([0xAB; 32]);
        assert!(matches!(
            establish_forge_timing_authority(&s, "preprod", None),
            Err(ForgeTimingError::UnknownVenueGenesis { .. })
        ));
    }

    /// The operator's real genesis file cross-checks the timing ORIGIN and the ACTIVE slot length —
    /// the two facts the durable epoch binding cannot see. Both fail closed.
    #[test]
    fn the_operator_genesis_file_cross_checks_origin_and_active_slot_length() {
        let mut g = preprod_genesis_facts();
        g.system_start_unix_ms += 1;
        assert!(matches!(
            establish_forge_timing_authority(&s7_sidecar(), "preprod", Some(g)),
            Err(ForgeTimingError::GenesisSystemStartMismatch { .. })
        ));
        let mut g2 = preprod_genesis_facts();
        g2.active_slot_length_ms = 20_000;
        assert!(matches!(
            establish_forge_timing_authority(&s7_sidecar(), "preprod", Some(g2)),
            Err(ForgeTimingError::ActiveSlotLengthMismatch { .. })
        ));
    }

    /// The durable epoch geometry is checked, not assumed: a store whose recorded epoch start does
    /// not match the calendar refuses rather than forging on a calendar it disagrees with.
    #[test]
    fn a_store_epoch_the_calendar_cannot_reproduce_fails_closed() {
        let mut s = s7_sidecar();
        s.epoch_start_slot = SlotNo(S7_EPOCH_START_SLOT - 86_400); // the byron-blind value
        assert!(matches!(
            establish_forge_timing_authority(&s, "preprod", Some(preprod_genesis_facts())),
            Err(ForgeTimingError::Authority(
                ade_core::consensus::era_schedule::TimingAuthorityError::EpochStartSlotMismatch {
                    ..
                }
            ))
        ));
    }

    /// Warm start RECONSTRUCTS: same durable inputs + same committed table ⇒ byte-identical
    /// authority, so nothing about WHEN the node restarted can enter the slot derivation.
    #[test]
    fn reconstruction_is_byte_identical_across_restarts() {
        let a = establish_forge_timing_authority(&s7_sidecar(), "preprod", None).unwrap();
        let b = establish_forge_timing_authority(&s7_sidecar(), "preprod", None).unwrap();
        assert_eq!(a, b);
        assert_eq!(a.source_schedule_commitment(), b.source_schedule_commitment());
    }

    /// Structural, not a duplicate registry: every venue with a committed calendar must also have a
    /// committed identity profile (that is where its genesis hash comes from), and each must resolve
    /// by its OWN hash and no other.
    #[test]
    fn the_timing_table_and_the_identity_registry_cannot_drift() {
        for id in VENUE_IDS {
            let p = resolve_network_profile(id).expect("a committed calendar needs a profile");
            let (resolved_id, h) = venue_timing_history_for_genesis(&p.genesis_hash)
                .expect("each committed venue must resolve by its own genesis hash");
            assert_eq!(resolved_id, id);
            // The calendar's ACTIVE epoch length must be the profile's epoch length -- one venue,
            // one geometry, stated in two registries that must agree.
            assert_eq!(
                u64::from(h.segments.last().unwrap().epoch_length_slots),
                p.epoch_length,
                "{id}: calendar and profile disagree on the epoch length"
            );
        }
        // Distinct venues must not collide on a hash.
        let preprod = resolve_network_profile("preprod").unwrap().genesis_hash;
        let preview = resolve_network_profile("preview").unwrap().genesis_hash;
        assert_ne!(preprod, preview);
        assert_eq!(venue_timing_history_for_genesis(&preprod).unwrap().0, "preprod");
        assert_eq!(venue_timing_history_for_genesis(&preview).unwrap().0, "preview");
    }
}
