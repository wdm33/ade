// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! GREEN closed `--mode node` feed/forge scheduling event vocabulary
//! (PHASE4-N-F-G-J S1, `CN-NODE-04`).
//!
//! A CLOSED, fail-closed-on-unknown, **emit-only** diagnostic surface that
//! makes the relay loop's feed/forge scheduling decision observable WITHOUT
//! changing it. This is a SIBLING of the wire-only [`super::event::LiveLogEvent`]
//! — a separate closed enum with its own JSONL encoder; the wire-only
//! vocabulary is untouched.
//!
//! Two closed sums:
//!   - [`FeedReason`] — the closed feed-state taxonomy: WHY a feed yielded no
//!     block, with a fail-closed eligibility split. S1's producible taxonomy
//!     from the CURRENT `NodeBlockSource` signals is exactly three variants —
//!     a reason-less / ambiguous disconnect classifies INELIGIBLE
//!     [`FeedReason::UnknownDisconnected`] (option (b): no cross-crate wire-pump
//!     reason enrichment in S1). The future error reasons
//!     (`PeerLost`/`DecodeError`/`ProtocolError`/`SourceInvalid`) and a
//!     reason-enriched live `CleanEmpty` are a wire-pump prerequisite, NOT S1.
//!   - [`NodeSchedEvent`] — the closed event vocabulary the relay loop emits.
//!
//! Emit-only (`CN-NODE-04`): the GREEN planner (`run_loop_planner`) NEVER reads
//! a `NodeSchedEvent`. `ci/ci_check_node_sched_events_emit_only.sh` enforces the
//! one-directional planner -> log property mechanically. The events are
//! operational/diagnostic tier ONLY — never a consensus/acceptance/BA-02 signal,
//! never replay-equivalence-weighted.

/// Closed feed-state taxonomy (`CN-NODE-04`): WHY a feed yielded no block, with
/// a fail-closed eligibility split. No catch-all / `Other`; adding a variant is
/// a compile error at every exhaustive `match` until wired + allow-listed.
///
/// **S1 producible set (fail-closed-on-ambiguity, OQ1):**
///   - [`Self::NoBlockAvailable`] — a WirePump open but momentarily empty (NOT
///     disconnected, lookahead empty). **Eligible.**
///   - [`Self::CleanEmpty`] — an InMemory feed drained: a deterministic,
///     provably-clean exhaustion (the hermetic source). **Eligible.**
///   - [`Self::UnknownDisconnected`] — a reason-less / ambiguous WirePump
///     disconnect. The `disconnected: bool` collapse cannot prove a clean drain,
///     so this fails closed. **Ineligible.** No ambiguous disconnect may become
///     forge-eligible.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeedReason {
    /// A WirePump feed is open but has no block ready right now (eligible).
    NoBlockAvailable,
    /// An in-memory feed drained — a provably-clean deterministic exhaustion
    /// (eligible).
    CleanEmpty,
    /// A reason-less / ambiguous WirePump disconnect — cannot be proven clean,
    /// so it fails closed (ineligible). The C1-observed disconnect is this until
    /// a future wire-pump enrichment captures a closed clean/error reason.
    UnknownDisconnected,
}

impl FeedReason {
    /// Whether this feed reason is forge-eligible. Eligible iff the feed end is
    /// provably clean / no-block: [`Self::NoBlockAvailable`] | [`Self::CleanEmpty`].
    /// [`Self::UnknownDisconnected`] is INELIGIBLE — the load-bearing
    /// fail-closed-on-ambiguity rule. (S1 only CLASSIFIES + emits; S2 is the sole
    /// consumer of this eligibility for the forge allowance.)
    pub fn is_forge_eligible(self) -> bool {
        match self {
            Self::NoBlockAvailable | Self::CleanEmpty => true,
            Self::UnknownDisconnected => false,
        }
    }

    /// Stable discriminator string emitted as the JSON `reason` field. The set
    /// is closed — adding a variant means adding a discriminator + updating the
    /// allow-list.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NoBlockAvailable => "no_block_available",
            Self::CleanEmpty => "clean_empty",
            Self::UnknownDisconnected => "unknown_disconnected",
        }
    }
}

/// Closed forge-result outcome (`CN-NODE-04`): the closed projection of the
/// reused `CoordinatorEvent` forge result, plus the off-tip skip. No stringly
/// fields; no catch-all. Operational tier only — never an acceptance signal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForgeOutcome {
    /// The forge engine self-accepted a block (the reused
    /// `CoordinatorEvent::ForgeSucceeded`). NOT a peer-acceptance / BA-02 claim.
    Succeeded,
    /// The recovered-surface leader check decided the operator is not the leader
    /// for this slot (the reused `CoordinatorEvent::ForgeNotLeader`).
    NotLeader,
    /// The forge attempt failed (the reused `CoordinatorEvent::ForgeFailed`).
    Failed,
    /// A forge tick was scheduled but no selected tip was available, so no forge
    /// attempt ran (the relay loop's `selected_tip == None` skip).
    NoTipAvailable,
}

impl ForgeOutcome {
    /// Stable discriminator string emitted as the JSON `outcome` field.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Succeeded => "succeeded",
            Self::NotLeader => "not_leader",
            Self::Failed => "failed",
            Self::NoTipAvailable => "no_tip_available",
        }
    }
}

/// Closed `--mode node` feed/forge scheduling event vocabulary (`CN-NODE-04`).
///
/// The whole vocabulary — there is deliberately no catch-all / `Other` variant
/// and no stringly-typed field; `reason`/`outcome` are the closed enums above.
/// Not `#[non_exhaustive]`: a new event is a compile error at the encoder's
/// exhaustive `match` until wired + allow-listed (fail-closed-on-unknown).
///
/// EMIT-ONLY: the GREEN planner never constructs or reads a `NodeSchedEvent`;
/// the relay loop emits them around the planner call + the `LoopStep` arms.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeSchedEvent {
    /// The feed yielded no block at this boundary; `reason` is the closed
    /// taxonomy classification (eligible vs ineligible).
    FeedUnavailable { reason: FeedReason },
    /// A forge tick was considered (the planner returned `ForgeTick`).
    ForgeTickConsidered,
    /// A forge tick was skipped at a terminal feed-end (the C1 skip: the feed
    /// is `Ending`, a forge slot was due, and the planner halted cleanly).
    /// `reason` is the closed feed-state taxonomy classification.
    ForgeTickSkipped { reason: FeedReason },
    /// A forge attempt is about to run (a due forge slot on a live feed with a
    /// selected tip).
    ForgeAttempted,
    /// A forge attempt completed; `outcome` is the closed forge-result.
    ForgeResult { outcome: ForgeOutcome },
}

impl NodeSchedEvent {
    /// Stable discriminator string emitted as the JSON `event` field. The set is
    /// closed — adding a variant means adding a discriminator + updating
    /// `ci/ci_check_node_sched_events_emit_only.sh`'s allow-list.
    pub fn discriminator(&self) -> &'static str {
        match self {
            Self::FeedUnavailable { .. } => "feed_unavailable",
            Self::ForgeTickConsidered => "forge_tick_considered",
            Self::ForgeTickSkipped { .. } => "forge_tick_skipped",
            Self::ForgeAttempted => "forge_attempted",
            Self::ForgeResult { .. } => "forge_result",
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn feed_reason_eligibility_split_is_fail_closed() {
        // Eligible iff provably clean / no-block.
        assert!(FeedReason::NoBlockAvailable.is_forge_eligible());
        assert!(FeedReason::CleanEmpty.is_forge_eligible());
        // The reason-less / ambiguous disconnect is INELIGIBLE — the load-bearing
        // fail-closed-on-ambiguity rule.
        assert!(!FeedReason::UnknownDisconnected.is_forge_eligible());
    }

    /// Compile-time exhaustiveness: a new `FeedReason` variant fails to compile
    /// here until updated (closedness, not a silent default).
    #[test]
    fn feed_reason_match_is_exhaustive() {
        let r = FeedReason::UnknownDisconnected;
        let _: &str = match r {
            FeedReason::NoBlockAvailable => "no_block_available",
            FeedReason::CleanEmpty => "clean_empty",
            FeedReason::UnknownDisconnected => "unknown_disconnected",
        };
    }

    /// CE-G-J-1 (closedness / fail-closed-on-unknown): the emitted
    /// `NodeSchedEvent` discriminator set is EXACTLY the allow-list the emit-only
    /// gate enforces — no extra, none missing. A new/unknown variant fails this
    /// (and the exhaustive `discriminator()` match is a compile error) until it
    /// is added here AND to `ci/ci_check_node_sched_events_emit_only.sh`. There
    /// is no catch-all / stringly term that could silently emit an
    /// out-of-vocabulary event.
    #[test]
    fn node_sched_event_allowlist_rejects_unknown_variants() {
        use std::collections::BTreeSet;
        // One of every variant — the whole closed vocabulary.
        let all = [
            NodeSchedEvent::FeedUnavailable {
                reason: FeedReason::UnknownDisconnected,
            },
            NodeSchedEvent::ForgeTickConsidered,
            NodeSchedEvent::ForgeTickSkipped {
                reason: FeedReason::NoBlockAvailable,
            },
            NodeSchedEvent::ForgeAttempted,
            NodeSchedEvent::ForgeResult {
                outcome: ForgeOutcome::Succeeded,
            },
        ];
        let produced: BTreeSet<&str> = all.iter().map(|e| e.discriminator()).collect();
        let allow_list: BTreeSet<&str> = [
            "feed_unavailable",
            "forge_tick_considered",
            "forge_tick_skipped",
            "forge_attempted",
            "forge_result",
        ]
        .into_iter()
        .collect();
        assert_eq!(
            produced, allow_list,
            "the NodeSchedEvent vocabulary and the emit-only allow-list must be EXACTLY equal — \
             an added/unknown variant fails closed here until allow-listed"
        );
        // Every event has a UNIQUE discriminator (no aliasing into the allow-list).
        assert_eq!(
            produced.len(),
            all.len(),
            "every NodeSchedEvent variant has a distinct closed discriminator"
        );
        // The closed reason set: exactly the three S1-producible reasons, with the
        // fail-closed eligibility split intact (UnknownDisconnected ineligible).
        let reasons: BTreeSet<&str> = [
            FeedReason::NoBlockAvailable,
            FeedReason::CleanEmpty,
            FeedReason::UnknownDisconnected,
        ]
        .iter()
        .map(|r| r.as_str())
        .collect();
        assert_eq!(
            reasons,
            ["clean_empty", "no_block_available", "unknown_disconnected"]
                .into_iter()
                .collect::<BTreeSet<&str>>(),
            "the closed FeedReason vocabulary is exactly the three S1-producible reasons"
        );
        assert!(
            !FeedReason::UnknownDisconnected.is_forge_eligible(),
            "an unknown/ambiguous disconnect must remain ineligible (fail-closed)"
        );
    }
}
