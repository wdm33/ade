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

/// LIVE-2b: why a wall-clock instant could not be converted to a logical slot. Closed and structured
/// — a conversion that cannot be justified must refuse, never return a plausible number.
///
/// Exists because the pre-LIVE-2b forge path did `slot = (now − systemStart) / shelley_slot_length`,
/// which silently ignores that Byron slots were 20s. On preprod that is wrong by exactly
/// `86_400 × (20 − 1) = 1_641_600` slots ≈ 19 days, and nothing rejected it: KES happened to refuse the
/// resulting future slot as out-of-range, which is the right answer to the wrong question.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlotDerivationError {
    /// The captured instant precedes the schedule's system start.
    BeforeSystemStart { captured_ms: u64, system_start_ms: u64 },
    /// The schedule has no eras.
    EmptySchedule,
    /// The schedule's FIRST era does not begin at slot 0, so its segments cannot be anchored to
    /// `system_start_unix_ms` by accumulation and no start TIME is available for it.
    ///
    /// This is a real condition in Ade, not a defensive branch: the native-Mithril bootstrap builds a
    /// SINGLE-era Conway schedule anchored at the snapshot epoch's absolute first slot. Such a schedule
    /// cannot answer "what slot is it now?" from the system start alone, and a forge path must be told
    /// that rather than handed a number derived from a mismatched anchor.
    ScheduleDoesNotCoverSystemStart { first_era_start_slot: u64 },
    /// A zero slot length would make the conversion undefined.
    ZeroSlotLength { era_index: u8 },
    /// Elapsed-time or slot arithmetic overflowed.
    Overflow,
}

/// LIVE-2b — THE canonical wall-clock → logical-slot conversion.
///
/// **BLUE — authoritative, not GREEN glue.** The arithmetic is small and deterministic, which makes it
/// tempting to call transport; it is not. The `SlotNo` this returns selects the KES period, drives VRF
/// leadership evaluation, is written into the block header, and is signed. A wrong result here produces
/// an invalid block, so the conversion OWNS authoritative meaning and lives in `ade_core` accordingly.
/// GREEN may carry the captured instant in and the verdict out; it must never own this conversion, or
/// GREEN would be affecting authoritative output.
///
/// The boundary is:
/// ```text
/// RED   capture UnixMillis
/// BLUE  slot_at(captured_ms) -> SlotNo | SlotDerivationError
/// BLUE  forecast / KES / leadership decisions
/// RED   signing and transmission
/// ```
///
/// Deterministic and total over the schedule. It reads no clock, no filesystem, no network and no peer
/// — the instant is an argument, which is what makes it replayable.
///
/// Each era segment is anchored by ACCUMULATION from the system start: segment `i+1` begins at
/// `t_i + (start_slot_{i+1} − start_slot_i) × slot_length_i`. That is what makes Byron's 20s slots and
/// Shelley's 1s slots compose without a venue-specific branch — the schedule carries the geometry, so
/// mainnet, preprod, preview and any synthetic multi-era schedule take the identical code path.
///
/// The peer tip is deliberately NOT an input. A legitimate chain has empty slots, so `derived ≈ tip` is
/// not a validity condition — peer lag is operational evidence, never the definition of "now".
pub fn slot_at(
    schedule: &EraSchedule,
    captured_ms: u64,
) -> Result<SlotNo, SlotDerivationError> {
    let eras = schedule.eras();
    let first = eras.first().ok_or(SlotDerivationError::EmptySchedule)?;
    if first.start_slot.0 != 0 {
        return Err(SlotDerivationError::ScheduleDoesNotCoverSystemStart {
            first_era_start_slot: first.start_slot.0,
        });
    }
    let system_start = schedule.system_start_unix_ms();
    if captured_ms < system_start {
        return Err(SlotDerivationError::BeforeSystemStart {
            captured_ms,
            system_start_ms: system_start,
        });
    }
    // Walk the segments, carrying each one's start TIME. The applicable segment is the last whose
    // start time does not exceed the captured instant.
    let mut seg_start_ms = system_start;
    for (i, era) in eras.iter().enumerate() {
        if era.slot_length_ms == 0 {
            return Err(SlotDerivationError::ZeroSlotLength { era_index: i as u8 });
        }
        let next_start_ms = match eras.get(i + 1) {
            None => None,
            Some(next) => {
                let span_slots = next
                    .start_slot
                    .0
                    .checked_sub(era.start_slot.0)
                    .ok_or(SlotDerivationError::Overflow)?;
                let span_ms = span_slots
                    .checked_mul(u64::from(era.slot_length_ms))
                    .ok_or(SlotDerivationError::Overflow)?;
                Some(
                    seg_start_ms
                        .checked_add(span_ms)
                        .ok_or(SlotDerivationError::Overflow)?,
                )
            }
        };
        let in_this_segment = match next_start_ms {
            None => true,
            Some(next_ms) => captured_ms < next_ms,
        };
        if in_this_segment {
            let elapsed_ms = captured_ms
                .checked_sub(seg_start_ms)
                .ok_or(SlotDerivationError::Overflow)?;
            let elapsed_slots = elapsed_ms / u64::from(era.slot_length_ms);
            let slot = era
                .start_slot
                .0
                .checked_add(elapsed_slots)
                .ok_or(SlotDerivationError::Overflow)?;
            return Ok(SlotNo(slot));
        }
        seg_start_ms = next_start_ms.ok_or(SlotDerivationError::Overflow)?;
    }
    Err(SlotDerivationError::EmptySchedule)
}

/// LIVE-2c: a 32-byte commitment to ONE full canonical timing schedule. Binds a
/// [`DerivedTimingAnchor`] to the exact history it was projected from, so "derived" is a checkable
/// fact rather than a claim in a comment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScheduleCommitment(pub Hash32);

/// LIVE-2c: the canonical TIMING commitment of a schedule — `system_start_unix_ms` plus, per segment,
/// `(start_slot, slot_length_ms)`. Deliberately covers the timing fields ONLY: two schedules that
/// differ solely in `era` / `start_epoch` / `epoch_length_slots` / `safe_zone_slots` / RSW commit
/// IDENTICALLY, which is the same Conway-only scope boundary CE-L2c-12 enforces for `slot_at`.
pub fn schedule_timing_commitment(schedule: &EraSchedule) -> ScheduleCommitment {
    let mut buf = Vec::new();
    buf.extend_from_slice(b"ade.live2c.timing-schedule.v1");
    buf.extend_from_slice(&schedule.system_start_unix_ms().to_be_bytes());
    buf.extend_from_slice(&(schedule.eras().len() as u64).to_be_bytes());
    for e in schedule.eras() {
        buf.extend_from_slice(&e.start_slot.0.to_be_bytes());
        buf.extend_from_slice(&u64::from(e.slot_length_ms).to_be_bytes());
    }
    ScheduleCommitment(ade_crypto::blake2b_256(&buf))
}

/// LIVE-2c: why a [`DerivedTimingAnchor`] could not be derived or used.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimingAnchorError {
    /// The requested domain start could not be converted against the full schedule.
    DomainStartNotDerivable(SlotDerivationError),
    /// A captured instant precedes the anchor's DECLARED domain. The anchor answers for its domain
    /// and refuses outside it — it is a projection, not a replacement history.
    BeforeDomainStart { captured_ms: u64, domain_start_ms: u64 },
    /// The anchor's segments are empty or non-monotonic.
    MalformedSegments,
    /// Arithmetic overflow.
    Overflow,
}

/// LIVE-2c — a COMPACT timing projection with lineage. **Not** a schedule, and deliberately not
/// convertible into one.
///
/// The Mithril bootstrap gives Ade a snapshot-local view; a schedule beginning mid-chain cannot answer
/// "what absolute slot is it now?", and `slot_at` refuses it (`ScheduleDoesNotCoverSystemStart`). That
/// refusal is CONSTITUTIONAL, not an inconvenience — relaxing it would create a second slot authority.
/// This type is the sanctioned alternative: a projection that carries its own lineage.
///
/// The type makes five facts explicit, so none of them rests on a convention:
///
/// | fact | how the type carries it |
/// |---|---|
/// | it is NOT full history | a distinct type; no `From<EraSchedule>`, no deref, no accessor returning one |
/// | it has a DECLARED domain | `domain_start_ms` / `domain_start_slot`, and use before it is refused |
/// | its first binding was DERIVED, not supplied | [`DerivedTimingAnchor::derive`] is the ONLY constructor and takes the full `EraSchedule` |
/// | it is committed to ONE canonical schedule | `source_schedule_commitment` |
/// | it cannot be used before its domain | `BeforeDomainStart` |
///
/// There is no operator-facing constructor by design: an operator-entered timestamp is exactly the
/// second, disagreeing clock authority the ruling forbids.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DerivedTimingAnchor {
    domain_start_ms: u64,
    domain_start_slot: SlotNo,
    /// `(start_slot, slot_length_ms, segment_start_ms)` for the domain-start segment and every later
    /// one. `segment_start_ms` is the segment's TRUE start time, derived from the full history — NOT
    /// the declared domain start.
    ///
    /// That distinction is load-bearing and cost a real bug: anchoring the arithmetic to
    /// `domain_start_ms` (an arbitrary instant that may fall MID-SLOT) discards the sub-slot offset and
    /// under-counts by one across a later transition. CE-L2c-13's dense sweep caught it. The declared
    /// domain governs ADMISSIBILITY; the segment start governs ARITHMETIC. Timing only — never era
    /// identity or epoch geometry.
    segments: Vec<(SlotNo, u32, u64)>,
    source_schedule_commitment: ScheduleCommitment,
}

impl DerivedTimingAnchor {
    /// LIVE-2c — THE canonical constructor: derive the anchor for a BOOTSTRAP ANCHOR SLOT.
    ///
    /// The declared domain must come from a canonical bootstrap FACT, never from `now`. Process start
    /// time, the current wall clock, a peer-tip observation and an operator-supplied timestamp are all
    /// excluded, because each makes the anchor a function of WHEN the node happened to start:
    ///
    /// ```text
    /// same bootstrap anchor + same timing history  =>  same DerivedTimingAnchor
    /// ```
    ///
    /// A restart days later must RECONSTRUCT the same anchor, not mint a new one from that restart's
    /// clock. That is what makes the anchor durable-bindable and replay-checkable, and it is why this
    /// takes a `SlotNo` — a bootstrap fact — rather than a timestamp.
    ///
    /// The domain start TIME is then the canonical slot-start time of that slot, derived from the full
    /// schedule (never supplied), so the arithmetic origin lands exactly on a slot boundary — the
    /// mid-slot origin bug CE-L2c-13 caught cannot recur through this path by construction.
    pub fn derive_for_bootstrap_anchor(
        schedule: &EraSchedule,
        bootstrap_anchor_slot: SlotNo,
    ) -> Result<Self, TimingAnchorError> {
        let domain_start_ms = slot_start_time_ms(schedule, bootstrap_anchor_slot)
            .map_err(TimingAnchorError::DomainStartNotDerivable)?;
        let anchor = Self::derive(schedule, domain_start_ms)?;
        // The slot-start time must round-trip to the very slot it came from. Cheap, and it pins the
        // inverse pair together so a future edit to either cannot silently drift.
        if anchor.domain_start_slot != bootstrap_anchor_slot {
            return Err(TimingAnchorError::MalformedSegments);
        }
        Ok(anchor)
    }

    /// Derive the anchor for `domain_start_ms` from the COMPLETE schedule. The sole constructor.
    pub fn derive(
        schedule: &EraSchedule,
        domain_start_ms: u64,
    ) -> Result<Self, TimingAnchorError> {
        let domain_start_slot =
            slot_at(schedule, domain_start_ms).map_err(TimingAnchorError::DomainStartNotDerivable)?;
        // Walk the FULL history once, computing each segment's true start time, then keep the
        // domain-start segment and everything after it. Earlier history is dropped only after its
        // timing contribution has been accounted for.
        let mut all: Vec<(SlotNo, u32, u64)> = Vec::with_capacity(schedule.eras().len());
        let mut seg_ms = schedule.system_start_unix_ms();
        for (i, e) in schedule.eras().iter().enumerate() {
            if e.slot_length_ms == 0 {
                return Err(TimingAnchorError::MalformedSegments);
            }
            all.push((e.start_slot, e.slot_length_ms, seg_ms));
            if let Some(next) = schedule.eras().get(i + 1) {
                let span = next
                    .start_slot
                    .0
                    .checked_sub(e.start_slot.0)
                    .ok_or(TimingAnchorError::MalformedSegments)?;
                seg_ms = seg_ms
                    .checked_add(
                        span.checked_mul(u64::from(e.slot_length_ms))
                            .ok_or(TimingAnchorError::Overflow)?,
                    )
                    .ok_or(TimingAnchorError::Overflow)?;
            }
        }
        let first_idx = all
            .iter()
            .rposition(|(start, _, _)| start.0 <= domain_start_slot.0)
            .ok_or(TimingAnchorError::MalformedSegments)?;
        let segments: Vec<(SlotNo, u32, u64)> = all[first_idx..].to_vec();
        if segments.is_empty() {
            return Err(TimingAnchorError::MalformedSegments);
        }
        Ok(Self {
            domain_start_ms,
            domain_start_slot,
            segments,
            source_schedule_commitment: schedule_timing_commitment(schedule),
        })
    }

    pub fn domain_start_ms(&self) -> u64 {
        self.domain_start_ms
    }
    pub fn domain_start_slot(&self) -> SlotNo {
        self.domain_start_slot
    }
    /// The slot length of the segment the DECLARED DOMAIN starts in — the venue's active slot
    /// cadence. Pacing only (`SystemClock`'s tick interval); never a conversion input.
    pub fn domain_slot_length_ms(&self) -> u32 {
        // `derive` keeps the domain-start segment first and refuses an empty segment list, so this
        // cannot be absent; the fallback keeps the accessor total without a panic path.
        self.segments.first().map(|(_, l, _)| *l).unwrap_or(1)
    }
    pub fn source_schedule_commitment(&self) -> &ScheduleCommitment {
        &self.source_schedule_commitment
    }
    /// Is this anchor a projection of THAT schedule? The lineage check a consumer runs before trusting it.
    pub fn is_derived_from(&self, schedule: &EraSchedule) -> bool {
        self.source_schedule_commitment == schedule_timing_commitment(schedule)
    }

    /// Convert a captured instant within the DECLARED domain. Must agree with the full schedule
    /// everywhere in that domain (CE-L2c-13).
    pub fn slot_at(&self, captured_ms: u64) -> Result<SlotNo, TimingAnchorError> {
        if captured_ms < self.domain_start_ms {
            return Err(TimingAnchorError::BeforeDomainStart {
                captured_ms,
                domain_start_ms: self.domain_start_ms,
            });
        }
        // Each segment carries its own TRUE start time, so this is the identical arithmetic the full
        // history performs — no accumulation from the (possibly mid-slot) domain start.
        for (i, (seg_slot, slot_len, seg_ms)) in self.segments.iter().enumerate() {
            let in_this = match self.segments.get(i + 1) {
                None => true,
                Some((_, _, next_ms)) => captured_ms < *next_ms,
            };
            if in_this {
                let elapsed = captured_ms
                    .checked_sub(*seg_ms)
                    .ok_or(TimingAnchorError::Overflow)?;
                return Ok(SlotNo(
                    seg_slot
                        .0
                        .checked_add(elapsed / u64::from(*slot_len))
                        .ok_or(TimingAnchorError::Overflow)?,
                ));
            }
        }
        Err(TimingAnchorError::MalformedSegments)
    }
}

/// LIVE-2c — ONE timing segment of a venue's absolute slot calendar.
///
/// Timing and CALENDAR geometry only. There is deliberately no `era: CardanoEra` field: era identity
/// is ledger semantics, Ade executes Conway only, and CE-L2c-12 proves `slot_at` cannot read it. Not
/// carrying it is the difference between "we intend not to branch on the era" and "there is nothing
/// to branch on".
///
/// `epoch_length_slots` is carried for exactly ONE purpose — reconstructing an epoch's absolute start
/// slot so the committed calendar can be checked against the durable bootstrap facts
/// ([`BootstrapTimingBinding`]). `slot_at` never reads it; CE-L2c-12 keeps that mechanical.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VenueTimingSegment {
    pub start_slot: SlotNo,
    pub start_epoch: EpochNo,
    pub slot_length_ms: u32,
    pub epoch_length_slots: u32,
}

/// LIVE-2c — a venue's COMPLETE wall-clock→absolute-slot calendar, from system start.
///
/// Complete is load-bearing: the first segment must begin at slot 0, because that is what lets every
/// later segment be anchored by accumulation from `system_start_unix_ms`. A snapshot-local schedule
/// cannot answer "what absolute slot is it now?" and `slot_at` refuses it
/// (`ScheduleDoesNotCoverSystemStart`) — that refusal is constitutional and this type satisfies it
/// rather than routing around it.
///
/// Preprod needs two segments (20 s Byron slots, then 1 s); preview needs one (its Shelley hard fork
/// is at epoch 0, so its Byron segment has ZERO slots). Both take the identical code path — the
/// geometry is data, never a venue branch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VenueTimingHistory {
    pub system_start_unix_ms: u64,
    pub segments: Vec<VenueTimingSegment>,
}

impl VenueTimingHistory {
    /// Project the calendar onto the timing [`EraSchedule`] `slot_at` consumes.
    ///
    /// Every segment is labelled `CardanoEra::Conway` — not because the Byron segment is Conway, but
    /// because `EraSummary` structurally requires the field and CE-L2c-12 proves no timing answer can
    /// depend on it. A uniform value makes an era-identity branch impossible to introduce THROUGH
    /// this constructor, which is stronger than supplying a truthful label nobody may read.
    pub fn to_schedule(&self) -> Result<EraSchedule, HFCError> {
        EraSchedule::new(
            BootstrapAnchorHash(Hash32([0u8; 32])),
            self.system_start_unix_ms,
            self.segments
                .iter()
                .map(|s| EraSummary {
                    era: CardanoEra::Conway,
                    start_slot: s.start_slot,
                    start_epoch: s.start_epoch,
                    slot_length_ms: s.slot_length_ms,
                    epoch_length_slots: s.epoch_length_slots,
                    safe_zone_slots: s.epoch_length_slots,
                    randomness_stabilisation_window_slots: None,
                })
                .collect(),
        )
    }

    /// The absolute start slot and epoch length of `epoch`, from the calendar alone.
    ///
    /// The check this exists for: preprod epoch 304 must land on 129_686_400
    /// (`86_400 + (304 − 4) × 432_000`). Drop the Byron segment and the same expression yields
    /// 129_600_000 — off by exactly the `86_400 × 19 s` the LIVE-2b defect is made of. That is what
    /// turns a committed constant table into a fact the store can refute.
    pub fn epoch_geometry(&self, epoch: EpochNo) -> Option<(SlotNo, u32)> {
        let seg = self
            .segments
            .iter()
            .rev()
            .find(|s| s.start_epoch.0 <= epoch.0)?;
        let offset = epoch.0.checked_sub(seg.start_epoch.0)?;
        let start = seg
            .start_slot
            .0
            .checked_add(offset.checked_mul(u64::from(seg.epoch_length_slots))?)?;
        Some((SlotNo(start), seg.epoch_length_slots))
    }
}

/// LIVE-2c — the DURABLE bootstrap facts a reconstructed calendar must reproduce before it may be
/// trusted as the forge's slot authority.
///
/// Every field is read from the store's own seed-epoch sidecar, written at import and never
/// re-supplied by a restart CLI. That is what makes the timing authority *bootstrap-bound* rather
/// than *configured*: a committed constant table proposes the calendar, and the store disposes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BootstrapTimingBinding {
    /// The certified bootstrap point's slot — the anchor's DECLARED domain start. A bootstrap FACT,
    /// never `now` (CE-L2c-14).
    pub anchor_slot: SlotNo,
    pub epoch: EpochNo,
    pub epoch_start_slot: SlotNo,
    pub epoch_length_slots: u32,
}

/// LIVE-2c — why a timing authority could not be established. Closed and structured: an authority
/// that cannot be bound to the store must refuse, never fall back to a plausible calendar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TimingAuthorityError {
    /// The calendar is not a well-formed schedule (empty, non-monotonic, zero geometry).
    MalformedHistory(HFCError),
    /// A segment transition is not epoch-aligned with the PREVIOUS segment's own epoch length, so the
    /// table disagrees with itself about where its eras meet.
    NonAlignedSegmentTransition {
        segment_index: u8,
        declared_start_slot: u64,
        implied_start_slot: u64,
    },
    /// The calendar does not reach the durable bootstrap epoch.
    EpochNotCovered { epoch: u64 },
    /// The reconstructed epoch start disagrees with the durable one — THE check that catches a
    /// dropped or mis-sized historical timing segment.
    EpochStartSlotMismatch {
        epoch: u64,
        durable: u64,
        reconstructed: u64,
    },
    EpochLengthMismatch {
        epoch: u64,
        durable: u32,
        reconstructed: u32,
    },
    /// The bootstrap anchor slot does not lie inside the durable bootstrap epoch, so the two facts
    /// are not from the same store.
    AnchorSlotOutsideBootstrapEpoch {
        anchor_slot: u64,
        epoch_start_slot: u64,
        epoch_end_slot: u64,
    },
    Anchor(TimingAnchorError),
    /// The derived anchor does not verify against the calendar it claims to project.
    LineageMismatch,
}

/// LIVE-2c — THE forge's wall-clock→slot authority: a [`DerivedTimingAnchor`] that has been bound to
/// the store's own bootstrap facts.
///
/// The type exists so "bootstrap-bound" is a state you can only reach through
/// [`Self::establish`], never a claim in a comment. There is no other constructor, no field is
/// public, and the only way in carries a [`BootstrapTimingBinding`] read from durable state.
///
/// ```text
/// same bootstrap lineage + same timing calendar  =>  byte-identical authority
/// ```
///
/// Warm start reconstructs rather than reloads: nothing here is persisted, so nothing can drift out
/// of agreement with the store it was checked against.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BootstrapBoundTimingAuthority {
    anchor: DerivedTimingAnchor,
    binding: BootstrapTimingBinding,
    slot_cadence_ms: u32,
}

impl BootstrapBoundTimingAuthority {
    /// Reconstruct the calendar, check it against the durable bootstrap facts, and derive the anchor.
    /// Pure, total, deterministic — no clock, no filesystem, no peer.
    pub fn establish(
        history: &VenueTimingHistory,
        binding: BootstrapTimingBinding,
    ) -> Result<Self, TimingAuthorityError> {
        // Self-consistency FIRST: a table whose segment transition disagrees with its own epoch
        // length would still reproduce some epoch starts by luck, so check the structure before
        // trusting any value it yields.
        for (i, pair) in history.segments.windows(2).enumerate() {
            let (prev, next) = (&pair[0], &pair[1]);
            let epochs = next
                .start_epoch
                .0
                .checked_sub(prev.start_epoch.0)
                .ok_or(TimingAuthorityError::NonAlignedSegmentTransition {
                    segment_index: (i + 1) as u8,
                    declared_start_slot: next.start_slot.0,
                    implied_start_slot: prev.start_slot.0,
                })?;
            let implied = prev
                .start_slot
                .0
                .checked_add(epochs.saturating_mul(u64::from(prev.epoch_length_slots)))
                .unwrap_or(u64::MAX);
            if implied != next.start_slot.0 {
                return Err(TimingAuthorityError::NonAlignedSegmentTransition {
                    segment_index: (i + 1) as u8,
                    declared_start_slot: next.start_slot.0,
                    implied_start_slot: implied,
                });
            }
        }
        let schedule = history
            .to_schedule()
            .map_err(TimingAuthorityError::MalformedHistory)?;

        let (reconstructed_start, reconstructed_len) = history
            .epoch_geometry(binding.epoch)
            .ok_or(TimingAuthorityError::EpochNotCovered {
                epoch: binding.epoch.0,
            })?;
        if reconstructed_start != binding.epoch_start_slot {
            return Err(TimingAuthorityError::EpochStartSlotMismatch {
                epoch: binding.epoch.0,
                durable: binding.epoch_start_slot.0,
                reconstructed: reconstructed_start.0,
            });
        }
        if reconstructed_len != binding.epoch_length_slots {
            return Err(TimingAuthorityError::EpochLengthMismatch {
                epoch: binding.epoch.0,
                durable: binding.epoch_length_slots,
                reconstructed: reconstructed_len,
            });
        }
        let epoch_end = binding
            .epoch_start_slot
            .0
            .saturating_add(u64::from(binding.epoch_length_slots));
        if binding.anchor_slot.0 < binding.epoch_start_slot.0 || binding.anchor_slot.0 >= epoch_end {
            return Err(TimingAuthorityError::AnchorSlotOutsideBootstrapEpoch {
                anchor_slot: binding.anchor_slot.0,
                epoch_start_slot: binding.epoch_start_slot.0,
                epoch_end_slot: epoch_end,
            });
        }

        let anchor = DerivedTimingAnchor::derive_for_bootstrap_anchor(&schedule, binding.anchor_slot)
            .map_err(TimingAuthorityError::Anchor)?;
        // Lineage: the anchor must be a projection of THIS calendar. Cheap here, and it is the
        // property a later refactor would silently break.
        if !anchor.is_derived_from(&schedule) {
            return Err(TimingAuthorityError::LineageMismatch);
        }
        let slot_cadence_ms = anchor.domain_slot_length_ms();
        Ok(Self {
            anchor,
            binding,
            slot_cadence_ms,
        })
    }

    /// THE conversion the forge path calls. Refuses outside the anchor's declared domain rather than
    /// returning a plausible number.
    pub fn slot_at(&self, captured_ms: u64) -> Result<SlotNo, TimingAnchorError> {
        self.anchor.slot_at(captured_ms)
    }

    /// The active segment's slot length. RED PACING ONLY — when to wake up, never what slot it is.
    /// Exposed from here so the tick cadence and the slot conversion cannot come from two numbers.
    pub fn slot_cadence_ms(&self) -> u32 {
        self.slot_cadence_ms
    }

    pub fn anchor(&self) -> &DerivedTimingAnchor {
        &self.anchor
    }

    pub fn binding(&self) -> &BootstrapTimingBinding {
        &self.binding
    }

    pub fn source_schedule_commitment(&self) -> &ScheduleCommitment {
        self.anchor.source_schedule_commitment()
    }
}

/// LIVE-2c — the inverse of [`slot_at`]: the canonical START TIME of a slot, from the full schedule.
///
/// Exists so a domain start can be a bootstrap FACT (a slot) rather than an observed instant. Walks the
/// same accumulated segment times `slot_at` walks, so the two are one geometry rather than two.
pub fn slot_start_time_ms(
    schedule: &EraSchedule,
    slot: SlotNo,
) -> Result<u64, SlotDerivationError> {
    let eras = schedule.eras();
    let first = eras.first().ok_or(SlotDerivationError::EmptySchedule)?;
    if first.start_slot.0 != 0 {
        return Err(SlotDerivationError::ScheduleDoesNotCoverSystemStart {
            first_era_start_slot: first.start_slot.0,
        });
    }
    let mut seg_start_ms = schedule.system_start_unix_ms();
    for (i, era) in eras.iter().enumerate() {
        if era.slot_length_ms == 0 {
            return Err(SlotDerivationError::ZeroSlotLength { era_index: i as u8 });
        }
        let ends_at = eras.get(i + 1).map(|n| n.start_slot.0);
        let in_this = match ends_at {
            None => true,
            Some(end) => slot.0 < end,
        };
        if in_this {
            let offset = slot
                .0
                .checked_sub(era.start_slot.0)
                .ok_or(SlotDerivationError::Overflow)?;
            return seg_start_ms
                .checked_add(
                    offset
                        .checked_mul(u64::from(era.slot_length_ms))
                        .ok_or(SlotDerivationError::Overflow)?,
                )
                .ok_or(SlotDerivationError::Overflow);
        }
        let span = ends_at
            .ok_or(SlotDerivationError::Overflow)?
            .checked_sub(era.start_slot.0)
            .ok_or(SlotDerivationError::Overflow)?;
        seg_start_ms = seg_start_ms
            .checked_add(
                span.checked_mul(u64::from(era.slot_length_ms))
                    .ok_or(SlotDerivationError::Overflow)?,
            )
            .ok_or(SlotDerivationError::Overflow)?;
    }
    Err(SlotDerivationError::EmptySchedule)
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

    /// The epoch length in slots of the era covering `slot` (same era selection as [`locate`]).
    /// The reward update derives its monetary-expansion expected-block denominator
    /// (`epochLength × activeSlotCoeff`) from this, so expansion uses the network's REAL epoch
    /// geometry — preview's 86_400-slot epoch, not the mainnet 432_000 constant the reward math
    /// previously hardcoded.
    pub fn epoch_length_slots(&self, slot: SlotNo) -> Result<u32, HFCError> {
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
            if slot.0 >= pair[0].start_slot.0 && slot.0 < pair[1].start_slot.0 {
                chosen_idx = idx;
                break;
            }
        }
        Ok(self.eras[chosen_idx].epoch_length_slots)
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

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)]
mod live2b_slot_authority_tests {
    use super::*;
    use ade_types::{CardanoEra, EpochNo};

    fn era(start_slot: u64, start_epoch: u64, slot_len: u32, epoch_len: u32) -> EraSummary {
        EraSummary {
            era: CardanoEra::Conway,
            start_slot: SlotNo(start_slot),
            start_epoch: EpochNo(start_epoch),
            slot_length_ms: slot_len,
            epoch_length_slots: epoch_len,
            safe_zone_slots: 1,
            randomness_stabilisation_window_slots: None,
        }
    }
    fn sched(system_start_ms: u64, eras: Vec<EraSummary>) -> EraSchedule {
        EraSchedule::new(BootstrapAnchorHash(Hash32([0u8; 32])), system_start_ms, eras).unwrap()
    }

    /// The PRESERVED FIXTURE from the live discriminators (docs/evidence/.../
    /// live2b-slot-authority-discriminators.txt): preprod, 2026-08-06T13:09:21Z.
    ///
    /// byron  slots 0..86_400 at 20s   (epochs 0-3, 21_600 slots each)
    /// shelley from slot 86_400 at 1s
    /// captured instant 1_786_021_761 s  =>  slot 130_338_561
    ///
    /// Independently derived at diagnosis time and corroborated by a FRESH peer whose tip was
    /// 130_338_559 — two slots back, which is ordinary chain emptiness, not disagreement.
    const PREPROD_SYSTEM_START_MS: u64 = 1_654_041_600_000;
    const CAPTURED_MS: u64 = 1_786_021_761_000;
    const EXPECTED_SLOT: u64 = 130_338_561;

    fn preprod() -> EraSchedule {
        sched(
            PREPROD_SYSTEM_START_MS,
            vec![era(0, 0, 20_000, 21_600), era(86_400, 4, 1_000, 432_000)],
        )
    }

    #[test]
    fn preprod_fixture_reproduces_the_measured_slot() {
        assert_eq!(slot_at(&preprod(), CAPTURED_MS).unwrap(), SlotNo(EXPECTED_SLOT));
    }

    /// NEGATIVE CONTROL: the pre-LIVE-2b calculation — system start with the SHELLEY slot length,
    /// Byron's 20s segment ignored — reproduces the measured error EXACTLY. This is the defect
    /// `operator_forge.rs` shipped (`anchor_millis = slot_zero_time_unix_ms`, `start_slot = 0`,
    /// `slot_length_ms = 1000`), pinned so it can never be reintroduced as "simpler".
    #[test]
    fn naive_single_slot_length_reproduces_the_1_641_600_error() {
        let naive = (CAPTURED_MS - PREPROD_SYSTEM_START_MS) / 1_000;
        assert_eq!(naive - EXPECTED_SLOT, 1_641_600, "the measured live gap");
        assert_eq!(86_400 * (20 - 1), 1_641_600, "= byron slots x (20s - 1s)");
    }

    /// Era-boundary triple: last slot before the transition, the exact transition instant, and the
    /// first slot after it. The Byron→Shelley seam is where an off-by-one hides.
    #[test]
    fn era_boundary_last_exact_and_first() {
        let s = preprod();
        let transition_ms = PREPROD_SYSTEM_START_MS + 86_400 * 20_000;
        assert_eq!(slot_at(&s, transition_ms - 1).unwrap(), SlotNo(86_399), "last byron slot");
        assert_eq!(slot_at(&s, transition_ms).unwrap(), SlotNo(86_400), "exact transition");
        assert_eq!(slot_at(&s, transition_ms + 1_000).unwrap(), SlotNo(86_401), "first shelley slot");
    }

    /// Venue-independent by construction: three different geometries, one code path, no branches.
    #[test]
    fn multi_venue_and_synthetic_schedules_use_one_path() {
        // mainnet-shaped: byron 0..4_492_800 at 20s, shelley from 4_492_800 at 1s.
        let mainnet = sched(1_506_203_091_000, vec![era(0, 0, 20_000, 21_600), era(4_492_800, 208, 1_000, 432_000)]);
        let t = 1_506_203_091_000 + 4_492_800 * 20_000 + 5_000;
        assert_eq!(slot_at(&mainnet, t).unwrap(), SlotNo(4_492_805));
        // preview: single 1s era from genesis (no byron segment) — must still be exact.
        let preview = sched(1_666_656_000_000, vec![era(0, 0, 1_000, 86_400)]);
        assert_eq!(slot_at(&preview, 1_666_656_000_000 + 12_345_000).unwrap(), SlotNo(12_345));
        // synthetic THREE-era schedule: 20s -> 5s -> 1s.
        let synth = sched(1_000_000, vec![era(0, 0, 20_000, 100), era(100, 1, 5_000, 100), era(200, 2, 1_000, 100)]);
        let after_two = 1_000_000 + 100 * 20_000 + 100 * 5_000;
        assert_eq!(slot_at(&synth, after_two).unwrap(), SlotNo(200));
        assert_eq!(slot_at(&synth, after_two + 7_000).unwrap(), SlotNo(207));
    }

    /// SCOPE GUARD (LIVE-2c): non-timing historical semantics MUST NOT leak into slot derivation.
    ///
    /// Ade is Conway-only for execution; historical eras enter solely as the minimum TIMING projection
    /// needed to derive the current absolute slot. This makes that boundary MECHANICAL rather than
    /// documented: holding the timing fields fixed (`system_start`, `start_slot`, `slot_length_ms`),
    /// arbitrary changes to every NON-timing field — `era`, `start_epoch`, `epoch_length_slots`,
    /// `safe_zone_slots`, `randomness_stabilisation_window_slots` — must not move `slot_at` for ANY
    /// captured instant.
    ///
    /// If someone later teaches `slot_at` to consult era identity or epoch geometry, this fails. That is
    /// the point: it is the difference between "we intend to stay timing-only" and "we cannot drift".
    #[test]
    fn non_timing_fields_cannot_influence_slot_derivation() {
        let timing = [(0u64, 20_000u32), (86_400, 1_000)];
        // A: the honest preprod-shaped schedule.
        let a = sched(
            PREPROD_SYSTEM_START_MS,
            vec![
                era(timing[0].0, 0, timing[0].1, 21_600),
                era(timing[1].0, 4, timing[1].1, 432_000),
            ],
        );
        // B: IDENTICAL timing, every non-timing field deliberately wrong/absurd.
        let b = sched(
            PREPROD_SYSTEM_START_MS,
            vec![
                EraSummary {
                    era: CardanoEra::Conway,          // wrong era identity
                    start_slot: SlotNo(timing[0].0),  // timing: fixed
                    start_epoch: EpochNo(9_999),      // absurd
                    slot_length_ms: timing[0].1,      // timing: fixed
                    epoch_length_slots: 7,            // absurd
                    safe_zone_slots: 123_456,         // absurd
                    randomness_stabilisation_window_slots: Some(999_999),
                },
                EraSummary {
                    era: CardanoEra::ByronEbb,        // wrong era identity
                    start_slot: SlotNo(timing[1].0),  // timing: fixed
                    start_epoch: EpochNo(0),          // absurd
                    slot_length_ms: timing[1].1,      // timing: fixed
                    epoch_length_slots: 1,            // absurd
                    safe_zone_slots: 0,
                    randomness_stabilisation_window_slots: None,
                },
            ],
        );
        let transition_ms = PREPROD_SYSTEM_START_MS + 86_400 * 20_000;
        for t in [
            PREPROD_SYSTEM_START_MS,
            PREPROD_SYSTEM_START_MS + 1,
            PREPROD_SYSTEM_START_MS + 19_999,
            PREPROD_SYSTEM_START_MS + 20_000,
            transition_ms - 1,
            transition_ms,
            transition_ms + 1,
            transition_ms + 1_000,
            CAPTURED_MS,
            CAPTURED_MS + 86_400_000,
        ] {
            assert_eq!(
                slot_at(&a, t),
                slot_at(&b, t),
                "non-timing fields changed slot_at at instant {t} -- historical era SEMANTICS have \
                 leaked into slot derivation, which is outside Ade's Conway-only execution scope"
            );
        }
        // And the shared answer is still the measured fixture, so this is not vacuous agreement.
        assert_eq!(slot_at(&a, CAPTURED_MS).unwrap(), SlotNo(EXPECTED_SLOT));
    }

    /// Structured refusals, never a plausible number.
    #[test]
    fn refusals_are_structured() {
        let s = preprod();
        assert!(matches!(
            slot_at(&s, PREPROD_SYSTEM_START_MS - 1),
            Err(SlotDerivationError::BeforeSystemStart { .. })
        ));
        // THE Ade condition: a single-era schedule anchored mid-chain (the native-Mithril bootstrap
        // builds exactly this) cannot be anchored from the system start, and says so.
        let mid = sched(PREPROD_SYSTEM_START_MS, vec![era(130_118_400, 305, 1_000, 432_000)]);
        assert!(matches!(
            slot_at(&mid, CAPTURED_MS),
            Err(SlotDerivationError::ScheduleDoesNotCoverSystemStart { first_era_start_slot: 130_118_400 })
        ));
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)]
mod live2c_derived_anchor_tests {
    use super::*;
    use ade_types::{CardanoEra, EpochNo};

    const PREPROD_SYSTEM_START_MS: u64 = 1_654_041_600_000;
    const CAPTURED_MS: u64 = 1_786_021_761_000;
    const EXPECTED_SLOT: u64 = 130_338_561;
    const TRANSITION_MS: u64 = PREPROD_SYSTEM_START_MS + 86_400 * 20_000;

    fn era(start_slot: u64, start_epoch: u64, slot_len: u32, epoch_len: u32) -> EraSummary {
        EraSummary {
            era: CardanoEra::Conway,
            start_slot: SlotNo(start_slot),
            start_epoch: EpochNo(start_epoch),
            slot_length_ms: slot_len,
            epoch_length_slots: epoch_len,
            safe_zone_slots: 1,
            randomness_stabilisation_window_slots: None,
        }
    }
    fn full() -> EraSchedule {
        EraSchedule::new(
            BootstrapAnchorHash(Hash32([0u8; 32])),
            PREPROD_SYSTEM_START_MS,
            vec![era(0, 0, 20_000, 21_600), era(86_400, 4, 1_000, 432_000)],
        )
        .unwrap()
    }

    /// CE-L2c-13 — the equivalence obligation, as a UNIVERSAL property over the declared domain
    /// rather than a few interior samples.
    ///
    /// Domains chosen to span the awkward cases: one BEFORE the 20s→1s transition (so the anchor must
    /// carry the transition itself), one exactly AT it, and one well after. For each, the anchor must
    /// agree with the full history at its first admissible instant, at every transition edge and both
    /// sides of it, at the preserved live fixture, and across a dense sweep.
    #[test]
    fn ce_l2c_13_compact_anchor_equals_full_history_over_its_domain() {
        let f = full();
        for domain_start_ms in [
            PREPROD_SYSTEM_START_MS,          // whole history
            PREPROD_SYSTEM_START_MS + 1,      // just inside byron
            TRANSITION_MS - 20_000,           // last byron slot
            TRANSITION_MS - 1,                // byron/shelley edge, low side
            TRANSITION_MS,                    // exactly the transition
            TRANSITION_MS + 1,                // edge, high side
            TRANSITION_MS + 1_000,            // first shelley slot
            1_700_000_000_000,                // arbitrary mid-domain
            CAPTURED_MS,                      // the live fixture instant
        ] {
            let a = DerivedTimingAnchor::derive(&f, domain_start_ms)
                .unwrap_or_else(|e| panic!("derive at {domain_start_ms} failed: {e:?}"));
            assert!(a.is_derived_from(&f), "lineage must verify against its source");

            // FIRST admissible instant, every transition edge, and the fixture.
            let mut probes = vec![
                domain_start_ms,
                domain_start_ms + 1,
                TRANSITION_MS - 1,
                TRANSITION_MS,
                TRANSITION_MS + 1,
                CAPTURED_MS,
                CAPTURED_MS + 86_400_000,
            ];
            // A dense sweep so agreement is not an artefact of the chosen edges.
            let mut t = domain_start_ms;
            for _ in 0..200 {
                probes.push(t);
                t += 997; // deliberately not a slot multiple
            }
            for t in probes {
                if t < domain_start_ms {
                    continue; // outside the declared domain: the anchor refuses, by design
                }
                assert_eq!(
                    a.slot_at(t).unwrap(),
                    slot_at(&f, t).unwrap(),
                    "domain_start={domain_start_ms} instant={t}: compact anchor disagrees with the \
                     full timing history INSIDE its declared domain"
                );
            }
            // Non-vacuous: the shared answer at the fixture is the measured slot.
            assert_eq!(a.slot_at(CAPTURED_MS).unwrap(), SlotNo(EXPECTED_SLOT));
        }
    }

    /// LIVE-2c DOMAIN-START RULING: the anchor is a function of a BOOTSTRAP FACT, not of `now`.
    ///
    /// The property that matters for restart: same bootstrap anchor + same timing history => the same
    /// anchor, whenever it is rebuilt. A restart days later must RECONSTRUCT it, not mint a new one
    /// from that restart's clock.
    #[test]
    fn anchor_is_reproducible_from_the_bootstrap_slot_not_the_clock() {
        let f = full();
        // The real preprod Mithril anchor slot.
        let boot = SlotNo(129_813_427);
        let a1 = DerivedTimingAnchor::derive_for_bootstrap_anchor(&f, boot).unwrap();
        // "Rebuilt days later" — nothing about the rebuild references a clock.
        let a2 = DerivedTimingAnchor::derive_for_bootstrap_anchor(&f, boot).unwrap();
        assert_eq!(a1, a2, "same bootstrap anchor + same history must give the SAME anchor");
        assert_eq!(a1.domain_start_slot(), boot, "the domain start IS the bootstrap slot");

        // The domain start lands exactly on a slot boundary, so the mid-slot origin bug CE-L2c-13
        // caught cannot recur through this constructor.
        assert_eq!(slot_at(&f, a1.domain_start_ms()).unwrap(), boot);
        assert_eq!(slot_start_time_ms(&f, boot).unwrap(), a1.domain_start_ms());

        // CONTRAST: a wall-clock-derived domain is NOT reproducible — two "startups" a second apart
        // yield different anchors. This is the shape the ruling excludes.
        let t = 1_786_021_761_000u64;
        let w1 = DerivedTimingAnchor::derive(&f, t).unwrap();
        let w2 = DerivedTimingAnchor::derive(&f, t + 1_000).unwrap();
        assert_ne!(w1, w2, "a clock-derived domain changes with startup time -- excluded by the ruling");

        // And it still agrees with the full history over its domain.
        for probe in [a1.domain_start_ms(), a1.domain_start_ms() + 1, CAPTURED_MS] {
            assert_eq!(a1.slot_at(probe).unwrap(), slot_at(&f, probe).unwrap());
        }
        assert_eq!(a1.slot_at(CAPTURED_MS).unwrap(), SlotNo(EXPECTED_SLOT));
    }

    /// `slot_start_time_ms` is the exact inverse of `slot_at` on slot boundaries, across the
    /// transition. If these two ever drift, a domain start stops landing on a boundary.
    #[test]
    fn slot_start_time_is_the_inverse_of_slot_at() {
        let f = full();
        for slot in [0u64, 1, 86_399, 86_400, 86_401, 130_118_400, EXPECTED_SLOT] {
            let t = slot_start_time_ms(&f, SlotNo(slot)).unwrap();
            assert_eq!(slot_at(&f, t).unwrap(), SlotNo(slot), "round-trip at slot {slot}");
            // ...and one ms before a slot start belongs to the PREVIOUS slot.
            if slot > 0 {
                assert_eq!(slot_at(&f, t - 1).unwrap(), SlotNo(slot - 1), "boundary at slot {slot}");
            }
        }
    }

    /// The anchor is a projection, not a replacement history: outside its domain it REFUSES.
    #[test]
    fn anchor_refuses_before_its_declared_domain() {
        let f = full();
        let a = DerivedTimingAnchor::derive(&f, CAPTURED_MS).unwrap();
        assert_eq!(a.domain_start_slot(), SlotNo(EXPECTED_SLOT));
        assert!(matches!(
            a.slot_at(CAPTURED_MS - 1),
            Err(TimingAnchorError::BeforeDomainStart { .. })
        ));
        assert!(a.slot_at(CAPTURED_MS).is_ok());
    }

    /// Lineage is checkable, and the commitment is TIMING-ONLY: a schedule differing only in
    /// non-timing fields commits identically (the CE-L2c-12 boundary, applied to the commitment).
    #[test]
    fn commitment_binds_timing_and_only_timing() {
        let f = full();
        // Same timing, absurd non-timing fields -> SAME commitment, so lineage still verifies.
        let same_timing = EraSchedule::new(
            BootstrapAnchorHash(Hash32([0xAB; 32])),
            PREPROD_SYSTEM_START_MS,
            vec![era(0, 9_999, 20_000, 7), era(86_400, 0, 1_000, 1)],
        )
        .unwrap();
        assert_eq!(schedule_timing_commitment(&f), schedule_timing_commitment(&same_timing));
        // Different TIMING -> different commitment, so a foreign anchor cannot pass as derived.
        let other_timing = EraSchedule::new(
            BootstrapAnchorHash(Hash32([0u8; 32])),
            PREPROD_SYSTEM_START_MS,
            vec![era(0, 0, 20_000, 21_600), era(86_400, 4, 2_000, 432_000)],
        )
        .unwrap();
        assert_ne!(schedule_timing_commitment(&f), schedule_timing_commitment(&other_timing));
        let a = DerivedTimingAnchor::derive(&f, CAPTURED_MS).unwrap();
        assert!(a.is_derived_from(&f));
        assert!(!a.is_derived_from(&other_timing), "a foreign schedule must not claim this anchor");
    }

    // ================================================================================
    // LIVE-2c ACTIVATION part 1 — the bootstrap-bound timing authority.
    //
    // Every constant below is a VENUE fact taken from that venue's own genesis, not folklore:
    //   preprod byron-genesis  startTime=1654041600  slotDuration=20000  protocolConsts.k=2160
    //                          => 21_600-slot byron epochs, shelley hard fork at epoch 4
    //   preprod shelley-genesis systemStart=2022-06-01T00:00:00Z (the IDENTICAL instant)
    //   preview config          TestShelleyHardForkAtEpoch=0 => ZERO byron slots
    // and the durable store pins them: ade-preprod-s7's sidecar records epoch 304 starting at
    // absolute slot 129_686_400 with the seed point at 129_813_427.
    // ================================================================================

    /// The committed preprod calendar. Two segments; the transition is epoch-aligned by construction.
    fn preprod_history() -> VenueTimingHistory {
        VenueTimingHistory {
            system_start_unix_ms: PREPROD_SYSTEM_START_MS,
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
        }
    }

    /// The DURABLE facts from `~/.cardano-live1/ade-preprod-s7`'s seed-epoch sidecar.
    const S7_SEED_EPOCH: u64 = 304;
    const S7_EPOCH_START_SLOT: u64 = 129_686_400;
    const S7_SEED_POINT_SLOT: u64 = 129_813_427;

    fn s7_binding() -> BootstrapTimingBinding {
        BootstrapTimingBinding {
            anchor_slot: SlotNo(S7_SEED_POINT_SLOT),
            epoch: EpochNo(S7_SEED_EPOCH),
            epoch_start_slot: SlotNo(S7_EPOCH_START_SLOT),
            epoch_length_slots: 432_000,
        }
    }

    /// CE-L2c-A2 + CE-L2c-5: the committed calendar reproduces the store's own epoch geometry, and
    /// the authority it establishes converts the PRESERVED live instant to the measured slot.
    ///
    /// Non-vacuous in both directions: the calendar is checked against a fact it did not supply
    /// (129_686_400 comes from the store), and the conversion is checked against a fact measured
    /// live (130_338_561, corroborated at diagnosis time by a peer two slots back).
    #[test]
    fn ce_l2c_a2_committed_calendar_reproduces_the_durable_epoch_and_the_measured_slot() {
        let h = preprod_history();
        assert_eq!(
            h.epoch_geometry(EpochNo(S7_SEED_EPOCH)),
            Some((SlotNo(S7_EPOCH_START_SLOT), 432_000)),
            "86_400 + (304 - 4) * 432_000 must be the store's recorded epoch-304 start"
        );
        let auth = BootstrapBoundTimingAuthority::establish(&h, s7_binding())
            .unwrap_or_else(|e| panic!("the live venue's own facts must establish: {e:?}"));
        assert_eq!(auth.slot_at(CAPTURED_MS).unwrap(), SlotNo(EXPECTED_SLOT));
        assert_eq!(auth.slot_cadence_ms(), 1_000, "preprod's ACTIVE segment is 1s");
        assert_eq!(auth.binding().anchor_slot, SlotNo(S7_SEED_POINT_SLOT));
    }

    /// THE mutation: drop the historical timing segment ("Ade is Conway-only, why carry Byron at
    /// all?"). The calendar no longer reproduces the store's epoch start, so it is REFUSED instead of
    /// yielding a plausible number.
    #[test]
    fn dropping_the_historical_timing_segment_is_refused_by_the_durable_binding() {
        let naive = VenueTimingHistory {
            system_start_unix_ms: PREPROD_SYSTEM_START_MS,
            segments: vec![VenueTimingSegment {
                start_slot: SlotNo(0),
                start_epoch: EpochNo(0),
                slot_length_ms: 1_000,
                epoch_length_slots: 432_000,
            }],
        };
        assert_eq!(
            BootstrapBoundTimingAuthority::establish(&naive, s7_binding()),
            Err(TimingAuthorityError::EpochStartSlotMismatch {
                epoch: S7_SEED_EPOCH,
                durable: S7_EPOCH_START_SLOT,
                reconstructed: 131_328_000,
            })
        );
        // ...and the slot it WOULD have handed the forge is exactly the shipped defect.
        let naive_slot = (CAPTURED_MS - PREPROD_SYSTEM_START_MS) / 1_000;
        assert_eq!(naive_slot - EXPECTED_SLOT, 1_641_600);
        assert_eq!(86_400 * (20 - 1), 1_641_600);
    }

    /// What the durable epoch cross-check does NOT buy — recorded as a test so the limit is a known
    /// fact rather than an assumption someone later leans on.
    ///
    /// The cross-check pins segment BOUNDARIES (start slots and epoch lengths). It cannot pin a
    /// segment's slot DURATION: a calendar with the correct boundaries but a wrong historical slot
    /// length reproduces the store's epoch geometry exactly, and still converts wall-clock wrongly —
    /// here by the full 1_641_600 slots. Slot durations are therefore held by the committed
    /// genesis-hash-selected registry plus the ACTIVE-segment cross-check against the operator's real
    /// `shelley-genesis.json`, not by this binding.
    #[test]
    fn the_durable_epoch_binding_pins_boundaries_not_slot_durations() {
        let mut wrong_duration = preprod_history();
        wrong_duration.segments[0].slot_length_ms = 1_000; // byron at 1s, boundaries untouched
        assert_eq!(
            wrong_duration.epoch_geometry(EpochNo(S7_SEED_EPOCH)),
            Some((SlotNo(S7_EPOCH_START_SLOT), 432_000)),
            "boundaries still reproduce the durable fact -- the binding cannot see the duration"
        );
        let auth = BootstrapBoundTimingAuthority::establish(&wrong_duration, s7_binding())
            .expect("the durable binding passes: this is the limit being recorded");
        assert_eq!(
            auth.slot_at(CAPTURED_MS).unwrap().0,
            EXPECTED_SLOT + 1_641_600,
            "and the answer is wrong by the whole byron segment"
        );
        // The commitment DOES separate them, which is what the registry review is anchored to.
        let good = BootstrapBoundTimingAuthority::establish(&preprod_history(), s7_binding()).unwrap();
        assert_ne!(
            auth.source_schedule_commitment(),
            good.source_schedule_commitment()
        );
    }

    /// A calendar that disagrees with ITSELF is refused before any value it yields is trusted — a
    /// mis-sized historical epoch length can otherwise still hit the right epoch start by luck.
    #[test]
    fn a_self_inconsistent_calendar_is_refused_at_the_transition() {
        let mut h = preprod_history();
        h.segments[0].epoch_length_slots = 21_599; // 4 * 21_599 = 86_396 != 86_400
        assert_eq!(
            BootstrapBoundTimingAuthority::establish(&h, s7_binding()),
            Err(TimingAuthorityError::NonAlignedSegmentTransition {
                segment_index: 1,
                declared_start_slot: 86_400,
                implied_start_slot: 86_396,
            })
        );
    }

    /// CE-L2c-A4: an altered TIMING value is rejected. Both directions are covered — a wrong slot
    /// length changes the commitment (so a foreign anchor cannot pass as derived), and a wrong system
    /// start moves the whole calendar off the store's epoch geometry.
    #[test]
    fn ce_l2c_a4_an_altered_timing_schedule_is_rejected() {
        let good = preprod_history();
        let auth = BootstrapBoundTimingAuthority::establish(&good, s7_binding()).unwrap();

        // (a) altered historical slot length: same epoch geometry, DIFFERENT timing.
        let mut altered = preprod_history();
        altered.segments[0].slot_length_ms = 10_000;
        let altered_auth = BootstrapBoundTimingAuthority::establish(&altered, s7_binding())
            .expect("epoch geometry is unchanged, so the binding still passes");
        assert_ne!(
            auth.source_schedule_commitment(),
            altered_auth.source_schedule_commitment(),
            "a TIMING change must move the commitment"
        );
        assert_ne!(
            altered_auth.slot_at(CAPTURED_MS).unwrap(),
            SlotNo(EXPECTED_SLOT),
            "and must move the answer -- otherwise the commitment guards nothing"
        );
        assert!(!auth.anchor().is_derived_from(&altered.to_schedule().unwrap()));

        // (b) altered system start: the calendar no longer lands on the store's epoch.
        let mut shifted = preprod_history();
        shifted.system_start_unix_ms += 1_000;
        let a = BootstrapBoundTimingAuthority::establish(&shifted, s7_binding()).unwrap();
        assert_ne!(a.slot_at(CAPTURED_MS).unwrap(), SlotNo(EXPECTED_SLOT));
        assert_ne!(a.source_schedule_commitment(), auth.source_schedule_commitment());
    }

    /// CE-L2c-A3 / CE-L2c-10: reconstruction is the warm-start contract. Same bootstrap lineage +
    /// same calendar => byte-identical authority, however many times and whenever it is rebuilt.
    #[test]
    fn ce_l2c_a3_reconstruction_is_byte_identical_and_replayable() {
        let a = BootstrapBoundTimingAuthority::establish(&preprod_history(), s7_binding()).unwrap();
        let b = BootstrapBoundTimingAuthority::establish(&preprod_history(), s7_binding()).unwrap();
        assert_eq!(a, b, "warm start must RECONSTRUCT the same authority, not mint a new one");
        for t in [
            CAPTURED_MS,
            CAPTURED_MS + 1,
            CAPTURED_MS + 999,
            CAPTURED_MS + 1_000,
            CAPTURED_MS + 86_400_000,
        ] {
            assert_eq!(a.slot_at(t).unwrap(), b.slot_at(t).unwrap());
            // ...and it agrees with the FULL history everywhere, which is CE-L2c-13's obligation
            // carried through the bound authority rather than restated for it.
            assert_eq!(
                a.slot_at(t).unwrap(),
                slot_at(&preprod_history().to_schedule().unwrap(), t).unwrap()
            );
        }
    }

    /// The binding must come from ONE store: an anchor slot outside the durable bootstrap epoch means
    /// two facts from two places, and is refused rather than reconciled.
    #[test]
    fn an_anchor_slot_outside_the_bootstrap_epoch_is_refused() {
        let mut b = s7_binding();
        b.anchor_slot = SlotNo(S7_EPOCH_START_SLOT - 1);
        assert!(matches!(
            BootstrapBoundTimingAuthority::establish(&preprod_history(), b),
            Err(TimingAuthorityError::AnchorSlotOutsideBootstrapEpoch { .. })
        ));
        let mut b2 = s7_binding();
        b2.anchor_slot = SlotNo(S7_EPOCH_START_SLOT + 432_000);
        assert!(matches!(
            BootstrapBoundTimingAuthority::establish(&preprod_history(), b2),
            Err(TimingAuthorityError::AnchorSlotOutsideBootstrapEpoch { .. })
        ));
    }

    /// Preview takes the identical path with no venue branch: its Shelley hard fork is at epoch 0, so
    /// its byron segment has ZERO slots and its calendar is a single 1s segment from slot 0.
    #[test]
    fn preview_single_segment_calendar_uses_the_same_path() {
        const PREVIEW_START_MS: u64 = 1_666_656_000_000;
        let h = VenueTimingHistory {
            system_start_unix_ms: PREVIEW_START_MS,
            segments: vec![VenueTimingSegment {
                start_slot: SlotNo(0),
                start_epoch: EpochNo(0),
                slot_length_ms: 1_000,
                epoch_length_slots: 86_400,
            }],
        };
        // preview epoch 1331 begins at 1331 * 86_400.
        assert_eq!(
            h.epoch_geometry(EpochNo(1331)),
            Some((SlotNo(114_998_400), 86_400))
        );
        let auth = BootstrapBoundTimingAuthority::establish(
            &h,
            BootstrapTimingBinding {
                anchor_slot: SlotNo(114_998_400 + 12_345),
                epoch: EpochNo(1331),
                epoch_start_slot: SlotNo(114_998_400),
                epoch_length_slots: 86_400,
            },
        )
        .unwrap();
        let t = PREVIEW_START_MS + (114_998_400 + 20_000) * 1_000;
        assert_eq!(auth.slot_at(t).unwrap(), SlotNo(114_998_400 + 20_000));
        assert_eq!(auth.slot_cadence_ms(), 1_000);
    }

    /// The constitutional guard survives the new type: a truncated (snapshot-local) calendar cannot
    /// establish an authority. `ScheduleDoesNotCoverSystemStart` is satisfied, never relaxed.
    #[test]
    fn a_truncated_calendar_cannot_establish_a_timing_authority() {
        let truncated = VenueTimingHistory {
            system_start_unix_ms: PREPROD_SYSTEM_START_MS,
            segments: vec![VenueTimingSegment {
                start_slot: SlotNo(S7_EPOCH_START_SLOT),
                start_epoch: EpochNo(S7_SEED_EPOCH),
                slot_length_ms: 1_000,
                epoch_length_slots: 432_000,
            }],
        };
        // The epoch binding PASSES (it is the snapshot's own epoch) -- so the refusal must come from
        // the slot-0 coverage requirement, not from the durable cross-check.
        assert_eq!(
            truncated.epoch_geometry(EpochNo(S7_SEED_EPOCH)),
            Some((SlotNo(S7_EPOCH_START_SLOT), 432_000))
        );
        assert_eq!(
            BootstrapBoundTimingAuthority::establish(&truncated, s7_binding()),
            Err(TimingAuthorityError::Anchor(
                TimingAnchorError::DomainStartNotDerivable(
                    SlotDerivationError::ScheduleDoesNotCoverSystemStart {
                        first_era_start_slot: S7_EPOCH_START_SLOT
                    }
                )
            ))
        );
    }

    /// The constitutional guard is NOT relaxed: a truncated schedule is still refused by `slot_at`,
    /// and the anchor is the sanctioned alternative rather than a way around it.
    #[test]
    fn truncated_schedule_still_refused_anchor_is_the_alternative() {
        let truncated = EraSchedule::new(
            BootstrapAnchorHash(Hash32([0u8; 32])),
            PREPROD_SYSTEM_START_MS,
            vec![era(130_118_400, 305, 1_000, 432_000)],
        )
        .unwrap();
        assert!(matches!(
            slot_at(&truncated, CAPTURED_MS),
            Err(SlotDerivationError::ScheduleDoesNotCoverSystemStart { .. })
        ));
        // ...and an anchor cannot be derived from it either: derivation goes through `slot_at`.
        assert!(matches!(
            DerivedTimingAnchor::derive(&truncated, CAPTURED_MS),
            Err(TimingAnchorError::DomainStartNotDerivable(
                SlotDerivationError::ScheduleDoesNotCoverSystemStart { .. }
            ))
        ));
    }
}
