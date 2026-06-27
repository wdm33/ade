// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

use ade_types::{CardanoEra, EpochNo, Hash32, SlotNo};

use crate::consensus::errors::{HFCError, OutsideForecastRange, SlotTimeError};

/// 32-byte anchor binding an EraSchedule to a particular genesis
/// configuration. Computed by `ade_runtime::consensus::genesis_parser`
/// as Blake2b-256 over a domain-separated concatenation of the four
/// genesis blob canonical encodings.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BootstrapAnchorHash(pub Hash32);

/// One era's parameters within the HFC schedule.
///
/// `slot_length_ms` and `epoch_length_slots` are captured per-era so
/// slot to time remains pure integer arithmetic. `safe_zone_slots` is
/// the stable forecast latitude past `start_slot` derived by the RED
/// parser from `(k, activeSlotsCoeff)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EraSummary {
    pub era: CardanoEra,
    pub start_slot: SlotNo,
    pub start_epoch: EpochNo,
    pub slot_length_ms: u32,
    pub epoch_length_slots: u32,
    pub safe_zone_slots: u32,
    /// `RSW = ceil(4·k / f)` in slots — the Praos candidate-nonce freeze latitude
    /// (`freeze_boundary = firstSlotNextEpoch − RSW`), derived by the RED parser
    /// from the venue genesis `(k, f)`. `None` = not supplied: a warm-start
    /// schedule rebuilt from the durable sidecar (which carries no `k`) — the
    /// candidate freeze is INERT and the boundary tick fails closed until B4
    /// persists it. `Some` on the FirstRun path (genesis `k` available). DC-EPOCH-16.
    pub randomness_stabilisation_window_slots: Option<u32>,
}

/// `RSW = ceil(4·k / f)` in slots, where `f = asc_numer / asc_denom` — the Praos
/// candidate-nonce freeze latitude (`freeze_boundary = firstSlotNextEpoch − RSW`),
/// mirroring `safe_zone_slots = ceil(3·k / f)`. The ONE source of truth: both the
/// RED genesis parser (FirstRun) and the live `--network` schedule builder derive
/// RSW here, so the genesis-parsed freeze and the live-follow freeze can never
/// desync (DC-EPOCH-16). Total — a zero numerator (degenerate `f`) or a product /
/// window that overflows `u64`/`u32` yields `None`, and the caller fails closed.
pub fn praos_rsw_slots(security_param: u64, asc_numer: u64, asc_denom: u64) -> Option<u32> {
    if asc_numer == 0 {
        return None;
    }
    let num = security_param.checked_mul(4)?.checked_mul(asc_denom)?;
    u32::try_from(num.div_ceil(asc_numer)).ok()
}

/// Pure result of `EraSchedule::locate(slot)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EraLocation {
    pub era_index: u8,
    pub era: CardanoEra,
    pub epoch: EpochNo,
    pub relative_slot_in_epoch: u32,
}

/// Typed BLUE-consumed HFC schedule.
///
/// Constructed once at startup by the RED genesis parser; never mutated.
/// Era ordering is strictly increasing by `start_slot`. All translation
/// methods are pure integer arithmetic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EraSchedule {
    anchor: BootstrapAnchorHash,
    system_start_unix_ms: u64,
    eras: Vec<EraSummary>,
}

impl EraSchedule {
    /// Construct a schedule, validating monotonicity and non-zero
    /// era parameters. Returns a structured `HFCError` on any
    /// violation; the node refuses to start on construction failure.
    pub fn new(
        anchor: BootstrapAnchorHash,
        system_start_unix_ms: u64,
        eras: Vec<EraSummary>,
    ) -> Result<Self, HFCError> {
        if eras.is_empty() {
            return Err(HFCError::EmptyEraList);
        }
        for (idx, era) in eras.iter().enumerate() {
            if era.slot_length_ms == 0 {
                return Err(HFCError::ZeroSlotLength {
                    era_index: idx as u8,
                });
            }
            if era.epoch_length_slots == 0 {
                return Err(HFCError::ZeroEpochLength {
                    era_index: idx as u8,
                });
            }
        }
        for window in eras.windows(2) {
            let prev = &window[0];
            let next = &window[1];
            if next.start_slot.0 <= prev.start_slot.0 {
                return Err(HFCError::NonMonotonicEras {
                    prev_start: prev.start_slot,
                    next_start: next.start_slot,
                });
            }
        }
        Ok(Self {
            anchor,
            system_start_unix_ms,
            eras,
        })
    }

    pub fn anchor(&self) -> &BootstrapAnchorHash {
        &self.anchor
    }

    pub fn system_start_unix_ms(&self) -> u64 {
        self.system_start_unix_ms
    }

    pub fn eras(&self) -> &[EraSummary] {
        &self.eras
    }

    /// Extend the schedule forward so it spans up to (and including) `target` epoch, by appending
    /// summaries cloned from the seed era (epoch 0) — the SAME forecast-horizon extension the live
    /// follow applies at each boundary, lifted here so the live path and the warm-start replay path
    /// share ONE definition (never two that can drift). A NO-OP when the schedule already reaches
    /// `target` (callers already covered stay byte-identical). Idempotent.
    pub fn extend_to_epoch(&mut self, target: EpochNo) {
        let (anchor, system_start, new_eras) = {
            let eras = self.eras();
            let seed = &eras[0];
            let last_epoch = eras[eras.len() - 1].start_epoch;
            if target.0 <= last_epoch.0 {
                return;
            }
            let l = u64::from(seed.epoch_length_slots);
            let mut new_eras: Vec<EraSummary> = eras.to_vec();
            for e in (last_epoch.0 + 1)..=target.0 {
                let offset = e - seed.start_epoch.0;
                new_eras.push(EraSummary {
                    randomness_stabilisation_window_slots: seed.randomness_stabilisation_window_slots,
                    era: seed.era,
                    start_slot: SlotNo(seed.start_slot.0 + offset * l),
                    start_epoch: EpochNo(e),
                    slot_length_ms: seed.slot_length_ms,
                    epoch_length_slots: seed.epoch_length_slots,
                    safe_zone_slots: seed.epoch_length_slots,
                });
            }
            (self.anchor().clone(), self.system_start_unix_ms(), new_eras)
        };
        if let Ok(extended) = EraSchedule::new(anchor, system_start, new_eras) {
            *self = extended;
        }
    }

    /// Extend so the schedule's forecast horizon covers `slot`. The epoch is computed from the seed
    /// era's geometry (the schedule may not yet reach `slot`, so `locate` can't be used), then the
    /// schedule is extended to that epoch via [`Self::extend_to_epoch`]. NO-OP when already covered.
    /// Used by the warm-start replay-forward so it can re-validate durable blocks past the seed
    /// epoch's frozen horizon, exactly as the live follow extends per boundary.
    pub fn extend_to_slot(&mut self, slot: SlotNo) {
        let (start_slot, start_epoch, epoch_len) = {
            let seed = &self.eras()[0];
            (
                seed.start_slot.0,
                seed.start_epoch.0,
                u64::from(seed.epoch_length_slots),
            )
        };
        if epoch_len == 0 || slot.0 < start_slot {
            return;
        }
        self.extend_to_epoch(EpochNo(start_epoch + (slot.0 - start_slot) / epoch_len));
    }

    /// Pure translation: which era / epoch / relative slot is `slot`?
    pub fn locate(&self, slot: SlotNo) -> Result<EraLocation, HFCError> {
        if self.eras.is_empty() {
            return Err(HFCError::EmptyEraList);
        }
        let first_start = self.eras[0].start_slot;
        if slot.0 < first_start.0 {
            return Err(HFCError::SlotBeforeSystemStart {
                slot,
                first_era_start: first_start,
            });
        }
        let mut chosen_idx: usize = self.eras.len() - 1;
        for (idx, pair) in self.eras.windows(2).enumerate() {
            let curr = &pair[0];
            let next = &pair[1];
            if slot.0 >= curr.start_slot.0 && slot.0 < next.start_slot.0 {
                chosen_idx = idx;
                break;
            }
        }
        let curr = &self.eras[chosen_idx];
        let slots_into_era = slot.0 - curr.start_slot.0;
        let epoch_len = u64::from(curr.epoch_length_slots);
        let era_epoch_offset = slots_into_era / epoch_len;
        let relative_slot_in_epoch = slots_into_era % epoch_len;
        let epoch_value = curr
            .start_epoch
            .0
            .checked_add(era_epoch_offset)
            .ok_or(HFCError::SlotAfterLastEra {
                slot,
                last_era_end: SlotNo(u64::MAX),
            })?;
        Ok(EraLocation {
            era_index: chosen_idx as u8,
            era: curr.era,
            epoch: EpochNo(epoch_value),
            relative_slot_in_epoch: relative_slot_in_epoch as u32,
        })
    }

    /// Slot to UTC instant in milliseconds since the unix epoch.
    /// Pure of wall-clock. Returns structured `Overflow` on integer
    /// overflow.
    pub fn slot_to_time_ms(&self, slot: SlotNo) -> Result<u64, SlotTimeError> {
        let location = self.locate(slot).map_err(SlotTimeError::HFC)?;
        let era_index = location.era_index as usize;
        let mut acc_ms: u64 = self.system_start_unix_ms;
        for idx in 0..era_index {
            let prior = &self.eras[idx];
            let next_start = self.eras[idx + 1].start_slot.0;
            let span = next_start
                .checked_sub(prior.start_slot.0)
                .ok_or(SlotTimeError::Overflow)?;
            let prior_ms = span
                .checked_mul(u64::from(prior.slot_length_ms))
                .ok_or(SlotTimeError::Overflow)?;
            acc_ms = acc_ms
                .checked_add(prior_ms)
                .ok_or(SlotTimeError::Overflow)?;
        }
        let curr = &self.eras[era_index];
        let slots_into_era = slot
            .0
            .checked_sub(curr.start_slot.0)
            .ok_or(SlotTimeError::Overflow)?;
        let era_ms = slots_into_era
            .checked_mul(u64::from(curr.slot_length_ms))
            .ok_or(SlotTimeError::Overflow)?;
        acc_ms
            .checked_add(era_ms)
            .ok_or(SlotTimeError::Overflow)
    }

    /// Forecast horizon = `last_era.start_slot + last_era.safe_zone_slots`.
    /// Slots strictly past this point yield `OutsideForecastRange`.
    pub fn check_forecast_horizon(
        &self,
        slot: SlotNo,
    ) -> Result<(), OutsideForecastRange> {
        let last = match self.eras.last() {
            Some(e) => e,
            None => {
                return Err(OutsideForecastRange {
                    requested: slot,
                    horizon: SlotNo(0),
                });
            }
        };
        let horizon = last
            .start_slot
            .0
            .saturating_add(u64::from(last.safe_zone_slots));
        if slot.0 > horizon {
            return Err(OutsideForecastRange {
                requested: slot,
                horizon: SlotNo(horizon),
            });
        }
        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::*;

    fn mainnet_like_eras() -> Vec<EraSummary> {
        vec![
            EraSummary {
                randomness_stabilisation_window_slots: None,
                era: CardanoEra::ByronRegular,
                start_slot: SlotNo(0),
                start_epoch: EpochNo(0),
                slot_length_ms: 20_000,
                epoch_length_slots: 21_600,
                safe_zone_slots: 129_600,
            },
            EraSummary {
                randomness_stabilisation_window_slots: None,
                era: CardanoEra::Shelley,
                start_slot: SlotNo(4_492_800),
                start_epoch: EpochNo(208),
                slot_length_ms: 1_000,
                epoch_length_slots: 432_000,
                safe_zone_slots: 129_600,
            },
            EraSummary {
                randomness_stabilisation_window_slots: None,
                era: CardanoEra::Allegra,
                start_slot: SlotNo(16_588_800),
                start_epoch: EpochNo(236),
                slot_length_ms: 1_000,
                epoch_length_slots: 432_000,
                safe_zone_slots: 129_600,
            },
            EraSummary {
                randomness_stabilisation_window_slots: None,
                era: CardanoEra::Mary,
                start_slot: SlotNo(23_068_800),
                start_epoch: EpochNo(251),
                slot_length_ms: 1_000,
                epoch_length_slots: 432_000,
                safe_zone_slots: 129_600,
            },
            EraSummary {
                randomness_stabilisation_window_slots: None,
                era: CardanoEra::Alonzo,
                start_slot: SlotNo(39_916_800),
                start_epoch: EpochNo(290),
                slot_length_ms: 1_000,
                epoch_length_slots: 432_000,
                safe_zone_slots: 129_600,
            },
            EraSummary {
                randomness_stabilisation_window_slots: None,
                era: CardanoEra::Babbage,
                start_slot: SlotNo(72_316_796),
                start_epoch: EpochNo(365),
                slot_length_ms: 1_000,
                epoch_length_slots: 432_000,
                safe_zone_slots: 129_600,
            },
            EraSummary {
                randomness_stabilisation_window_slots: None,
                era: CardanoEra::Conway,
                start_slot: SlotNo(133_660_800),
                start_epoch: EpochNo(507),
                slot_length_ms: 1_000,
                epoch_length_slots: 432_000,
                safe_zone_slots: 129_600,
            },
        ]
    }

    fn mainnet_like_schedule() -> EraSchedule {
        let anchor = BootstrapAnchorHash(Hash32([0u8; 32]));
        match EraSchedule::new(anchor, 1_506_203_091_000, mainnet_like_eras()) {
            Ok(s) => s,
            Err(_) => unreachable!("fixture is well-formed"),
        }
    }

    #[test]
    fn locate_first_slot_of_each_era() {
        let schedule = mainnet_like_schedule();
        for (idx, era) in mainnet_like_eras().iter().enumerate() {
            let loc = schedule
                .locate(era.start_slot)
                .unwrap_or_else(|_| unreachable!("first slot must locate"));
            assert_eq!(loc.era_index as usize, idx);
            assert_eq!(loc.era, era.era);
            assert_eq!(loc.epoch, era.start_epoch);
            assert_eq!(loc.relative_slot_in_epoch, 0);
        }
    }

    #[test]
    fn locate_last_slot_of_each_era() {
        let schedule = mainnet_like_schedule();
        let eras = mainnet_like_eras();
        for idx in 0..(eras.len() - 1) {
            let curr = &eras[idx];
            let next = &eras[idx + 1];
            let last_slot = SlotNo(next.start_slot.0 - 1);
            let loc = schedule
                .locate(last_slot)
                .unwrap_or_else(|_| unreachable!("last slot of era must locate"));
            assert_eq!(loc.era_index as usize, idx);
            assert_eq!(loc.era, curr.era);
            let slots = last_slot.0 - curr.start_slot.0;
            let epoch_len = u64::from(curr.epoch_length_slots);
            assert_eq!(loc.epoch.0, curr.start_epoch.0 + slots / epoch_len);
            assert_eq!(loc.relative_slot_in_epoch as u64, slots % epoch_len);
        }
    }

    #[test]
    fn locate_before_system_start_errors() {
        let anchor = BootstrapAnchorHash(Hash32([0u8; 32]));
        let eras = vec![EraSummary {
            randomness_stabilisation_window_slots: None,
            era: CardanoEra::Shelley,
            start_slot: SlotNo(100),
            start_epoch: EpochNo(0),
            slot_length_ms: 1_000,
            epoch_length_slots: 432_000,
            safe_zone_slots: 129_600,
        }];
        let schedule = match EraSchedule::new(anchor, 0, eras) {
            Ok(s) => s,
            Err(_) => unreachable!("well-formed"),
        };
        let err = schedule.locate(SlotNo(42));
        assert_eq!(
            err,
            Err(HFCError::SlotBeforeSystemStart {
                slot: SlotNo(42),
                first_era_start: SlotNo(100),
            })
        );
    }

    #[test]
    fn slot_to_time_monotone_increasing() {
        let schedule = mainnet_like_schedule();
        let probes: [SlotNo; 8] = [
            SlotNo(0),
            SlotNo(4_492_800),
            SlotNo(4_492_801),
            SlotNo(16_588_800),
            SlotNo(23_068_800),
            SlotNo(39_916_800),
            SlotNo(72_316_796),
            SlotNo(133_660_800),
        ];
        let mut prev_time: Option<u64> = None;
        for slot in probes {
            let t = schedule
                .slot_to_time_ms(slot)
                .unwrap_or_else(|_| unreachable!("probe must convert"));
            if let Some(p) = prev_time {
                assert!(t > p, "slot {} time {} <= prev {}", slot.0, t, p);
            }
            prev_time = Some(t);
        }
    }

    #[test]
    fn slot_to_time_overflow_returns_structured_error() {
        let anchor = BootstrapAnchorHash(Hash32([0u8; 32]));
        let eras = vec![EraSummary {
            randomness_stabilisation_window_slots: None,
            era: CardanoEra::Shelley,
            start_slot: SlotNo(0),
            start_epoch: EpochNo(0),
            slot_length_ms: 1_000,
            epoch_length_slots: 432_000,
            safe_zone_slots: 129_600,
        }];
        let schedule = match EraSchedule::new(anchor, u64::MAX, eras) {
            Ok(s) => s,
            Err(_) => unreachable!("well-formed"),
        };
        let result = schedule.slot_to_time_ms(SlotNo(1));
        assert_eq!(result, Err(SlotTimeError::Overflow));
    }

    #[test]
    fn forecast_horizon_boundary() {
        let schedule = mainnet_like_schedule();
        let last = mainnet_like_eras()
            .last()
            .cloned()
            .unwrap_or_else(|| unreachable!("non-empty"));
        let horizon = last.start_slot.0 + u64::from(last.safe_zone_slots);
        assert_eq!(schedule.check_forecast_horizon(SlotNo(horizon)), Ok(()));
        let beyond = SlotNo(horizon + 1);
        assert_eq!(
            schedule.check_forecast_horizon(beyond),
            Err(OutsideForecastRange {
                requested: beyond,
                horizon: SlotNo(horizon),
            })
        );
    }

    #[test]
    fn eraschedule_constructor_rejects_non_monotonic() {
        let anchor = BootstrapAnchorHash(Hash32([0u8; 32]));
        let bad = vec![
            EraSummary {
                randomness_stabilisation_window_slots: None,
                era: CardanoEra::ByronRegular,
                start_slot: SlotNo(100),
                start_epoch: EpochNo(0),
                slot_length_ms: 20_000,
                epoch_length_slots: 21_600,
                safe_zone_slots: 129_600,
            },
            EraSummary {
                randomness_stabilisation_window_slots: None,
                era: CardanoEra::Shelley,
                start_slot: SlotNo(100),
                start_epoch: EpochNo(1),
                slot_length_ms: 1_000,
                epoch_length_slots: 432_000,
                safe_zone_slots: 129_600,
            },
        ];
        let result = EraSchedule::new(anchor, 0, bad);
        assert_eq!(
            result,
            Err(HFCError::NonMonotonicEras {
                prev_start: SlotNo(100),
                next_start: SlotNo(100),
            })
        );
    }

    #[test]
    fn eraschedule_constructor_rejects_empty() {
        let anchor = BootstrapAnchorHash(Hash32([0u8; 32]));
        let result = EraSchedule::new(anchor, 0, vec![]);
        assert_eq!(result, Err(HFCError::EmptyEraList));
    }

    #[test]
    fn eraschedule_constructor_rejects_zero_slot_length() {
        let anchor = BootstrapAnchorHash(Hash32([0u8; 32]));
        let bad = vec![EraSummary {
            randomness_stabilisation_window_slots: None,
            era: CardanoEra::ByronRegular,
            start_slot: SlotNo(0),
            start_epoch: EpochNo(0),
            slot_length_ms: 0,
            epoch_length_slots: 21_600,
            safe_zone_slots: 129_600,
        }];
        let result = EraSchedule::new(anchor, 0, bad);
        assert_eq!(result, Err(HFCError::ZeroSlotLength { era_index: 0 }));
    }

    #[test]
    fn eraschedule_constructor_rejects_zero_epoch_length() {
        let anchor = BootstrapAnchorHash(Hash32([0u8; 32]));
        let bad = vec![EraSummary {
            randomness_stabilisation_window_slots: None,
            era: CardanoEra::ByronRegular,
            start_slot: SlotNo(0),
            start_epoch: EpochNo(0),
            slot_length_ms: 20_000,
            epoch_length_slots: 0,
            safe_zone_slots: 129_600,
        }];
        let result = EraSchedule::new(anchor, 0, bad);
        assert_eq!(result, Err(HFCError::ZeroEpochLength { era_index: 0 }));
    }

    #[test]
    fn determinism_across_runs() {
        let schedule = mainnet_like_schedule();
        let probes: [SlotNo; 7] = [
            SlotNo(0),
            SlotNo(4_492_800),
            SlotNo(16_588_800),
            SlotNo(23_068_800),
            SlotNo(39_916_800),
            SlotNo(72_316_796),
            SlotNo(133_660_800),
        ];
        let mut first: Vec<u64> = Vec::new();
        for _ in 0..2 {
            let answers: Vec<u64> = probes
                .iter()
                .map(|s| {
                    schedule
                        .slot_to_time_ms(*s)
                        .unwrap_or_else(|_| unreachable!("probes convert"))
                })
                .collect();
            if first.is_empty() {
                first = answers;
            } else {
                assert_eq!(first, answers);
            }
        }
    }
}
