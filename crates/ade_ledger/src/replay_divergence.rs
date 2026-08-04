// TCB: BLUE — deterministic, authoritative.
// - No wall-clock, no rand, no floating point, no I/O
// - Pure data + total functions over it
//! PREPROD-ENTRY-AUTHORITY P6-S4 — a SELF-DESCRIBING recovery divergence.
//!
//! When warm-start replay disagrees with the WAL-tail commitment (T-REC-05), the historical fault said
//! only:
//!
//! ```text
//! FingerprintMismatch { expected: c395bad1.., recovered: 2cda6765.. }
//! ```
//!
//! Two hashes and nothing else. Diagnosing P4 from that took hours and four wrong hypotheses; what
//! finally cracked it was per-COMPONENT fingerprints (the `snapshots` component moving across a single
//! block revealed an epoch boundary the live run never applied) plus the ledger-vs-schedule epoch pair
//! (1375 vs 1378). All of that was reconstructed after the fact with bespoke probes.
//!
//! This type carries that evidence IN THE FAULT, so the next divergence is diagnosed from the error
//! itself rather than from a forensic dig. It is pure data: the shell fills it in, the BLUE core owns
//! its meaning and its rendering.

use ade_types::primitives::{EpochNo, Hash32, SlotNo};

use crate::fingerprint::LedgerFingerprint;
use crate::store_semantics::AuthorityArtifact;

/// The seven fingerprint components, in declared order, paired with their names so a divergence can be
/// reported by NAME rather than by position.
fn components(fp: &LedgerFingerprint) -> [(&'static str, &Hash32); 7] {
    [
        ("era", &fp.era),
        ("utxo", &fp.utxo),
        ("cert", &fp.cert),
        ("epoch", &fp.epoch),
        ("snapshots", &fp.snapshots),
        ("pparams", &fp.pparams),
        ("governance", &fp.governance),
    ]
}

fn hex8(h: &Hash32) -> String {
    h.0.iter().take(4).map(|b| format!("{b:02x}")).collect()
}

/// Everything known about a warm-start replay divergence at the moment it is detected.
///
/// Every field is either free (already computed to detect the fault) or one cheap read. Nothing here
/// requires re-running the replay.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplayDivergenceReport {
    /// The WAL-tail slot the replay was reconstructing to.
    pub slot: SlotNo,
    /// Number of `AdmitBlock` entries in the WAL (the admitted chain length).
    pub admit_count: u64,
    /// The recovered ledger's own epoch.
    pub ledger_epoch: EpochNo,
    /// What the venue era schedule says the epoch at `slot` is. `None` when the slot is not locatable
    /// (a pre-schedule slot) — unverifiable, not violated, per DC-EPOCH-36.
    pub schedule_epoch: Option<EpochNo>,
    /// The WAL-tail commitment the replay had to reproduce.
    pub expected_combined: Hash32,
    /// The per-component fingerprint of what the replay actually produced.
    pub actual: LedgerFingerprint,
    /// The per-component fingerprint of the ANCHOR the replay started from, when it could be read.
    /// Components that differ between `anchor` and `actual` are the ones the replay MOVED — which is
    /// precisely the signal that identified P4's root cause.
    pub anchor: Option<LedgerFingerprint>,
    /// Slot of the snapshot the replay anchored on.
    pub anchor_slot: Option<SlotNo>,
    /// Admitted blocks in `(anchor_slot, slot]` — the replay span.
    pub span_blocks: Option<u64>,
    /// The authority-semantics generation the store declared (it passed DC-STORE-10 to get here, so a
    /// divergence is NOT explained by stale semantics — recording it rules that out on the record).
    pub store_semantics_version: u32,
    /// Which durable artifact the replay authority came from.
    pub artifact: AuthorityArtifact,
}

impl ReplayDivergenceReport {
    /// Component names that MOVED between the replay anchor and the replay result.
    ///
    /// Empty when the anchor could not be read. A component listed here changed during the replay; one
    /// absent did not. In P4 this list was `cert, epoch, snapshots, governance` with `utxo` and
    /// `pparams` absent — `snapshots` moving across a single mid-epoch block is what named the cause,
    /// because stake snapshots rotate only at an epoch boundary.
    pub fn moved_components(&self) -> Vec<&'static str> {
        let Some(anchor) = &self.anchor else {
            return Vec::new();
        };
        components(&self.actual)
            .iter()
            .zip(components(anchor).iter())
            .filter(|((_, a), (_, b))| a != b)
            .map(|((name, _), _)| *name)
            .collect()
    }

    /// True when the ledger and the venue schedule disagree about the epoch at `slot`. Cross-checks
    /// DC-EPOCH-36 at the recovery boundary: if this is ever true here, the divergence is an
    /// epoch-geometry fault and needs no further search.
    pub fn epoch_disagreement(&self) -> bool {
        matches!(self.schedule_epoch, Some(s) if s.0 != self.ledger_epoch.0)
    }
}

impl core::fmt::Display for ReplayDivergenceReport {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "warm-start replay diverged from the admitted chain at slot {} \
             (artifact={}, store_semantics=v{}, admits={}): expected {} but produced {}. \
             ledger_epoch={} schedule_epoch={}{}. anchor={} span_blocks={}. components[{}]",
            self.slot.0,
            self.artifact.as_str(),
            self.store_semantics_version,
            self.admit_count,
            hex8(&self.expected_combined),
            hex8(&self.actual.combined),
            self.ledger_epoch.0,
            self.schedule_epoch
                .map(|e| e.0.to_string())
                .unwrap_or_else(|| "unlocatable".to_string()),
            if self.epoch_disagreement() {
                " -- EPOCH DISAGREEMENT (this IS the fault; the replay applied a different epoch geometry)"
            } else {
                ""
            },
            self.anchor_slot
                .map(|s| s.0.to_string())
                .unwrap_or_else(|| "none".to_string()),
            self.span_blocks
                .map(|n| n.to_string())
                .unwrap_or_else(|| "?".to_string()),
            {
                let moved = self.moved_components();
                if self.anchor.is_none() {
                    "anchor unreadable".to_string()
                } else if moved.is_empty() {
                    "none moved".to_string()
                } else {
                    format!("moved: {}", moved.join(", "))
                }
            },
        )
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn fp(seed: u8) -> LedgerFingerprint {
        let h = |n: u8| Hash32([n; 32]);
        LedgerFingerprint {
            era: h(seed),
            utxo: h(seed + 1),
            cert: h(seed + 2),
            epoch: h(seed + 3),
            snapshots: h(seed + 4),
            pparams: h(seed + 5),
            governance: h(seed + 6),
            combined: h(seed + 7),
        }
    }

    fn report(
        anchor: Option<LedgerFingerprint>,
        ledger: u64,
        schedule: Option<u64>,
    ) -> ReplayDivergenceReport {
        ReplayDivergenceReport {
            slot: SlotNo(119_076_425),
            admit_count: 6953,
            ledger_epoch: EpochNo(ledger),
            schedule_epoch: schedule.map(EpochNo),
            expected_combined: Hash32([0xc3; 32]),
            actual: fp(10),
            anchor,
            anchor_slot: Some(SlotNo(119_075_343)),
            span_blocks: Some(53),
            store_semantics_version: 1,
            artifact: AuthorityArtifact::ChainDb,
        }
    }

    /// The P4 shape: only SOME components move. Naming them is what identified the cause.
    #[test]
    fn moved_components_names_only_what_changed() {
        let mut anchor = fp(10);
        anchor.snapshots = Hash32([0xAA; 32]);
        anchor.cert = Hash32([0xBB; 32]);
        let r = report(Some(anchor), 1378, Some(1378));
        assert_eq!(r.moved_components(), vec!["cert", "snapshots"]);
        assert!(r.to_string().contains("moved: cert, snapshots"));
    }

    #[test]
    fn identical_components_report_none_moved() {
        let r = report(Some(fp(10)), 1378, Some(1378));
        assert!(r.moved_components().is_empty());
        assert!(r.to_string().contains("none moved"));
    }

    /// An unreadable anchor must be distinguishable from "nothing moved" — otherwise the absence of a
    /// signal would read as evidence.
    #[test]
    fn unreadable_anchor_is_not_reported_as_no_movement() {
        let r = report(None, 1378, Some(1378));
        assert!(r.moved_components().is_empty());
        assert!(r.to_string().contains("anchor unreadable"));
    }

    /// An epoch disagreement at the recovery boundary IS the fault and says so, so the reader stops
    /// searching. This is the P3/P4 geometry signature.
    #[test]
    fn epoch_disagreement_is_called_out_explicitly() {
        let r = report(Some(fp(10)), 1375, Some(1378));
        assert!(r.epoch_disagreement());
        assert!(r.to_string().contains("EPOCH DISAGREEMENT"));
    }

    /// An unlocatable slot is not a disagreement (DC-EPOCH-36: unverifiable, not violated).
    #[test]
    fn unlocatable_schedule_epoch_is_not_a_disagreement() {
        let r = report(Some(fp(10)), 1375, None);
        assert!(!r.epoch_disagreement());
        assert!(r.to_string().contains("unlocatable"));
        assert!(!r.to_string().contains("EPOCH DISAGREEMENT"));
    }

    /// The report must name the artifact and the semantics generation, so a reader can rule stale
    /// semantics OUT without going to the store.
    #[test]
    fn report_names_artifact_and_semantics_version() {
        let s = report(Some(fp(10)), 1378, Some(1378)).to_string();
        assert!(s.contains("artifact=chain.db"));
        assert!(s.contains("store_semantics=v1"));
    }
}
