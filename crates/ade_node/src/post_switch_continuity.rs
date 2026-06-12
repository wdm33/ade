// Core Contract:
// - Deterministic: same inputs => byte-identical verdict (replay-equivalent)
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types (closed verdict sum, no free-form strings)
// - Reads ONLY Ade's own admitted-block lineage; the peer tip is NEVER an input

//! GREEN replayable post-switch branch-continuity verdict (PHASE4-N-AO S10,
//! DC-EVIDENCE-05).
//!
//! A pure, total, deterministic reducer over the closed convergence-evidence
//! transcript. After a `ForkChoiceWin` adoption at tip X it classifies Ade's
//! OWN validated admitted-block lineage into a closed [`PostSwitchContinuity`]
//! verdict. `ContinuesSelectedBranch` requires: unbroken `prev_hash` lineage
//! from X across every post-X `block_admitted`, no `diverged` after X, and every
//! `fork_choice_selected{win}` paired to a terminal.
//!
//! This module is **GREEN, not authority**. It does not decide chain selection,
//! validity, admission, or storage truth — it OBSERVES already-authoritative
//! outputs and derives evidence. The node NEVER consumes the verdict to select
//! chains; CI and `/cluster-close` use it as the CE-AO-6 release gate. The
//! peer's tip is never an input to the continuity verdict — only Ade's own
//! admitted blocks (`slot`, `block_hash`, `prev_hash`), the fork-switch events,
//! and `diverged`. Replay-equivalent: the same events yield a byte-identical
//! verdict (the replay test `post_switch_continuity_replays_byte_identical`).
//!
//! The release-window decision ([`evaluate_release_window`]) layers the
//! DC-EVIDENCE-04 acceptance terminal on top: the hard fork-switch proof (both
//! peers delivered) + `ContinuesSelectedBranch` + a bounded-window terminal of
//! exact agreement at X-or-descendant OR a validated prefix of the peer (peer
//! observed ahead). "Peer is ahead" is a RED observed comparison — it never
//! becomes consensus authority.

/// One parsed transcript event — only the fields the continuity verdict needs.
/// Construct directly in tests; the `post_switch_continuity` bin builds these
/// from the closed JSONL via [`EventView::from_json`].
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EventView {
    /// The `event` discriminator (`block_admitted`, `fork_switch_applied`, …).
    pub event: String,
    /// `slot` (block_admitted / agreement_verdict / block_received) or
    /// `new_tip_slot` (fork_switch_applied).
    pub slot: Option<u64>,
    /// `block_hash_hex` (block_admitted / block_received) or `new_tip_hash_hex`
    /// (fork_switch_applied).
    pub hash: Option<String>,
    /// `prev_hash_hex` (block_admitted) — the admitted block's VALIDATED parent.
    pub prev_hash: Option<String>,
    /// `peer` (block_received / fork-choice events).
    pub peer: Option<String>,
    /// `fork_switch_id` (all fork-choice events).
    pub fork_switch_id: Option<String>,
    /// `result` (fork_choice_selected: `win` / `loss`).
    pub result: Option<String>,
    /// `rollback_reason` (fork_switch_applied: `fork_choice_win`).
    pub rollback_reason: Option<String>,
    /// `kind` (agreement_verdict: `agreed` / `lagging` / `diverged` / …).
    pub kind: Option<String>,
    /// `our_hash_hex` (agreement_verdict).
    pub our_hash: Option<String>,
    /// `peer_hash_hex` (agreement_verdict).
    pub peer_hash: Option<String>,
    /// `peer_slot` (agreement_verdict `lagging`).
    pub peer_slot: Option<u64>,
}

/// Closed continuity verdict (DC-EVIDENCE-05). No free-form strings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PostSwitchContinuity {
    /// Every post-X admitted block chains from X by its own validated
    /// `prev_hash`; no diverged after X; every win has a terminal.
    ContinuesSelectedBranch {
        admitted_after_switch: u32,
        tip_slot: u64,
        tip_hash: String,
    },
    /// A `diverged` verdict occurred after the switch tip X.
    Diverged { slot: u64 },
    /// A post-X admitted block's `prev_hash` does not equal the prior admitted
    /// block's hash — a rollback below X or a branch jump.
    BrokenLineage {
        at_slot: u64,
        expected_prev: String,
        found_prev: String,
    },
    /// A `fork_choice_selected{win}` has no terminal (applied|failed|superseded).
    DanglingForkChoiceWin { fork_switch_id: String },
    /// The transcript lacks the prerequisites to judge continuity.
    InsufficientEvidence { reason: ContinuityGap },
}

/// Closed reasons the continuity verdict cannot be formed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContinuityGap {
    /// No `fork_switch_applied` event — no winning branch was durably adopted.
    NoForkSwitchApplied,
    /// `fork_switch_applied` whose `rollback_reason` is not `fork_choice_win`.
    SwitchNotForkChoiceWin,
    /// The adopted `fork_switch_id` lacks its prior `fork_choice_selected{win}`
    /// / `branch_fetch_completed` / `branch_prevalidated`.
    IncompleteSwitchProof,
    /// The switch tip X itself was never `block_admitted`.
    NoPostSwitchAdmit,
}

const FORK_CHOICE_WIN: &str = "fork_choice_win";

/// Pure, total, deterministic continuity reducer (DC-EVIDENCE-05). Reads ONLY
/// Ade's own admitted lineage + the fork-choice events; never the peer tip.
pub fn derive_post_switch_continuity(events: &[EventView]) -> PostSwitchContinuity {
    // The switch tip X = the FIRST fork_switch_applied. (A single clean switch
    // then continuity is the proven shape; a second in-window switch would break
    // the linear prev_hash walk and conservatively fail as BrokenLineage.)
    let x_idx = match events.iter().position(|e| e.event == "fork_switch_applied") {
        Some(i) => i,
        None => {
            return PostSwitchContinuity::InsufficientEvidence {
                reason: ContinuityGap::NoForkSwitchApplied,
            }
        }
    };
    let x = &events[x_idx];
    if x.rollback_reason.as_deref() != Some(FORK_CHOICE_WIN) {
        return PostSwitchContinuity::InsufficientEvidence {
            reason: ContinuityGap::SwitchNotForkChoiceWin,
        };
    }
    let x_fsid = x.fork_switch_id.clone().unwrap_or_default();
    let x_hash = match &x.hash {
        Some(h) => h.clone(),
        None => {
            return PostSwitchContinuity::InsufficientEvidence {
                reason: ContinuityGap::IncompleteSwitchProof,
            }
        }
    };

    // Hard proof for X's fork_switch_id: the prior decide+fetch+prevalidate chain.
    let has_for_fsid = |ev: &str| {
        events
            .iter()
            .any(|e| e.event == ev && e.fork_switch_id.as_deref() == Some(x_fsid.as_str()))
    };
    let selected_win = events.iter().any(|e| {
        e.event == "fork_choice_selected"
            && e.fork_switch_id.as_deref() == Some(x_fsid.as_str())
            && e.result.as_deref() == Some("win")
    });
    if !selected_win
        || !has_for_fsid("branch_fetch_completed")
        || !has_for_fsid("branch_prevalidated")
    {
        return PostSwitchContinuity::InsufficientEvidence {
            reason: ContinuityGap::IncompleteSwitchProof,
        };
    }

    // Post-X events. A diverged here is the most severe failure.
    let post = &events[x_idx + 1..];
    if let Some(d) = post
        .iter()
        .find(|e| e.event == "agreement_verdict" && e.kind.as_deref() == Some("diverged"))
    {
        return PostSwitchContinuity::Diverged {
            slot: d.slot.unwrap_or(0),
        };
    }

    // Lineage walk: the root is the first block_admitted of X (hash == X.hash);
    // every subsequent admitted block must chain by its OWN prev_hash.
    let admits: Vec<&EventView> = post.iter().filter(|e| e.event == "block_admitted").collect();
    let root_pos = admits.iter().position(|a| a.hash.as_deref() == Some(x_hash.as_str()));
    let root_pos = match root_pos {
        Some(p) => p,
        None => {
            return PostSwitchContinuity::InsufficientEvidence {
                reason: ContinuityGap::NoPostSwitchAdmit,
            }
        }
    };
    let mut prev_tip = x_hash.clone();
    let mut admitted_after: u32 = 0;
    let mut tip_slot = x.slot.unwrap_or(0);
    let mut tip_hash = x_hash.clone();
    for a in &admits[root_pos + 1..] {
        let found_prev = a.prev_hash.clone().unwrap_or_default();
        if found_prev != prev_tip {
            return PostSwitchContinuity::BrokenLineage {
                at_slot: a.slot.unwrap_or(0),
                expected_prev: prev_tip,
                found_prev,
            };
        }
        prev_tip = a.hash.clone().unwrap_or_default();
        tip_slot = a.slot.unwrap_or(tip_slot);
        tip_hash = a.hash.clone().unwrap_or(tip_hash);
        admitted_after += 1;
    }

    // Dangling win: every fork_choice_selected{win} fsid must have a terminal.
    if let Some(fsid) = first_dangling_win(events) {
        return PostSwitchContinuity::DanglingForkChoiceWin {
            fork_switch_id: fsid,
        };
    }

    PostSwitchContinuity::ContinuesSelectedBranch {
        admitted_after_switch: admitted_after,
        tip_slot,
        tip_hash,
    }
}

/// The first (transcript-order) `fork_choice_selected{win}` whose `fork_switch_id`
/// has no terminal (`fork_switch_applied|failed|superseded`), or `None`.
fn first_dangling_win(events: &[EventView]) -> Option<String> {
    for e in events {
        if e.event == "fork_choice_selected" && e.result.as_deref() == Some("win") {
            let fsid = e.fork_switch_id.clone().unwrap_or_default();
            let has_terminal = events.iter().any(|t| {
                matches!(
                    t.event.as_str(),
                    "fork_switch_applied" | "fork_switch_failed" | "fork_switch_superseded"
                ) && t.fork_switch_id.as_deref() == Some(fsid.as_str())
            });
            if !has_terminal {
                return Some(fsid);
            }
        }
    }
    None
}

/// The bounded-window convergence terminal (DC-EVIDENCE-04, refined by S10).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConvergenceTerminal {
    /// `agreement_verdict{agreed, our==peer}` exactly at the switch tip X.
    AgreedAtSwitchTip { slot: u64 },
    /// `agreement_verdict{agreed, our==peer}` at a descendant Y of X in-window.
    AgreedAtDescendant { slot: u64 },
    /// Continuity holds and the peer is observed ahead (`lagging`, peer_slot >
    /// our_slot) in-window — Ade is a validated catching-up prefix. The peer tip
    /// is an OBSERVED comparison point only, never authority.
    ValidatedPrefixOfPeer { our_slot: u64, peer_slot: u64 },
}

/// Closed release-gate verdict (the CE-AO-6 acceptance decision).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReleaseVerdict {
    Pass {
        continuity: PostSwitchContinuity,
        terminal: ConvergenceTerminal,
    },
    /// The hard fork-switch proof is incomplete (fewer than two peers delivered).
    FailHardProof { distinct_peers: u32 },
    /// Continuity did not hold (the verdict is the failing reason).
    FailContinuity { continuity: PostSwitchContinuity },
    /// Continuity held but no in-window terminal (no agreed at X-or-descendant
    /// and no observed peer-ahead prefix).
    FailNoTerminal,
}

/// Evaluate the bounded post-switch convergence window (the CE-AO-6 gate). The
/// bounds are fixed by the caller (the gate passes 200 / 20). The continuity
/// core is the replayable DC-EVIDENCE-05 verdict; this adds the DC-EVIDENCE-04
/// hard-proof (both peers) + bounded-window terminal.
pub fn evaluate_release_window(
    events: &[EventView],
    max_slots: u64,
    max_blocks: u32,
) -> ReleaseVerdict {
    // Hard proof part 1: both peers delivered block_received. (Distinct, ordered;
    // no HashSet — a small Vec membership check keeps it deterministic.)
    let mut peers: Vec<&str> = Vec::new();
    for e in events {
        if e.event == "block_received" {
            if let Some(p) = e.peer.as_deref() {
                if !peers.contains(&p) {
                    peers.push(p);
                }
            }
        }
    }
    if peers.len() < 2 {
        return ReleaseVerdict::FailHardProof {
            distinct_peers: peers.len() as u32,
        };
    }

    // The replayable continuity core. ContinuesSelectedBranch already implies X
    // exists, is ForkChoiceWin, the hard proof chain is complete, the lineage is
    // unbroken, no diverged, no dangling win.
    let continuity = derive_post_switch_continuity(events);
    if !matches!(continuity, PostSwitchContinuity::ContinuesSelectedBranch { .. }) {
        return ReleaseVerdict::FailContinuity { continuity };
    }

    // The bounded window after X: slot <= X.slot + max_slots, and at most
    // max_blocks admitted blocks.
    // ContinuesSelectedBranch implies a fork_switch_applied exists; handle the
    // impossible-None gracefully (no panic on adversarial transcript input).
    let x_idx = match events.iter().position(|e| e.event == "fork_switch_applied") {
        Some(i) => i,
        None => return ReleaseVerdict::FailContinuity { continuity },
    };
    let x_slot = events[x_idx].slot.unwrap_or(0);
    let mut window: Vec<&EventView> = Vec::new();
    let mut admitted = 0u32;
    for e in &events[x_idx..] {
        let s = e.slot.unwrap_or(x_slot);
        if s > x_slot.saturating_add(max_slots) {
            break;
        }
        if e.event == "block_admitted" {
            admitted += 1;
            if admitted > max_blocks {
                break;
            }
        }
        window.push(e);
    }

    // Terminal A: exact agreement at X or a descendant Y, our_hash == peer_hash.
    for e in &window {
        if e.event == "agreement_verdict"
            && e.kind.as_deref() == Some("agreed")
            && e.our_hash.is_some()
            && e.our_hash == e.peer_hash
        {
            let slot = e.slot.unwrap_or(x_slot);
            let terminal = if slot == x_slot {
                ConvergenceTerminal::AgreedAtSwitchTip { slot }
            } else {
                ConvergenceTerminal::AgreedAtDescendant { slot }
            };
            return ReleaseVerdict::Pass {
                continuity,
                terminal,
            };
        }
    }

    // Terminal B: validated prefix of peer — peer observed ahead (lagging,
    // peer_slot > our_slot) in-window, continuity holds. RED observed comparison.
    // Requires Ade to have FOLLOWED FORWARD: at least one admitted descendant of
    // X chained on (admitted_after_switch >= 1), so "stayed on the branch" speaks
    // of real blocks, not just the switch tip stalled.
    let followed_forward = matches!(
        continuity,
        PostSwitchContinuity::ContinuesSelectedBranch {
            admitted_after_switch,
            ..
        } if admitted_after_switch >= 1
    );
    if followed_forward {
        for e in &window {
            if e.event == "agreement_verdict" && e.kind.as_deref() == Some("lagging") {
                if let (Some(our_slot), Some(peer_slot)) = (e.slot, e.peer_slot) {
                    if peer_slot > our_slot {
                        return ReleaseVerdict::Pass {
                            continuity,
                            terminal: ConvergenceTerminal::ValidatedPrefixOfPeer {
                                our_slot,
                                peer_slot,
                            },
                        };
                    }
                }
            }
        }
    }

    ReleaseVerdict::FailNoTerminal
}

impl EventView {
    /// Parse one transcript line (closed JSONL) into the reducer view. Unknown
    /// fields are ignored; missing fields stay `None`. RED glue (used by the
    /// `post_switch_continuity` bin) — the reducer itself is JSON-free.
    pub fn from_json(v: &serde_json::Value) -> Option<EventView> {
        let event = v.get("event")?.as_str()?.to_string();
        let s = |k: &str| v.get(k).and_then(|x| x.as_str()).map(|x| x.to_string());
        let u = |k: &str| v.get(k).and_then(|x| x.as_u64());
        // `hash` and `slot` fold the per-event aliases.
        let hash = s("block_hash_hex").or_else(|| s("new_tip_hash_hex"));
        let slot = u("slot").or_else(|| u("new_tip_slot"));
        Some(EventView {
            event,
            slot,
            hash,
            prev_hash: s("prev_hash_hex"),
            peer: s("peer"),
            fork_switch_id: s("fork_switch_id"),
            result: s("result"),
            rollback_reason: s("rollback_reason"),
            kind: s("kind"),
            our_hash: s("our_hash_hex"),
            peer_hash: s("peer_hash_hex"),
            peer_slot: u("peer_slot"),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(event: &str) -> EventView {
        EventView {
            event: event.to_string(),
            ..Default::default()
        }
    }
    fn applied(fsid: &str, slot: u64, hash: &str) -> EventView {
        EventView {
            fork_switch_id: Some(fsid.into()),
            slot: Some(slot),
            hash: Some(hash.into()),
            rollback_reason: Some(FORK_CHOICE_WIN.into()),
            ..ev("fork_switch_applied")
        }
    }
    fn selected_win(fsid: &str) -> EventView {
        EventView {
            fork_switch_id: Some(fsid.into()),
            result: Some("win".into()),
            ..ev("fork_choice_selected")
        }
    }
    fn for_fsid(event: &str, fsid: &str) -> EventView {
        EventView {
            fork_switch_id: Some(fsid.into()),
            ..ev(event)
        }
    }
    fn admit(slot: u64, hash: &str, prev: &str) -> EventView {
        EventView {
            slot: Some(slot),
            hash: Some(hash.into()),
            prev_hash: Some(prev.into()),
            ..ev("block_admitted")
        }
    }
    fn received(peer: &str) -> EventView {
        EventView {
            peer: Some(peer.into()),
            ..ev("block_received")
        }
    }
    // A complete switch-then-continuity transcript: X @100 hash "x" (prev "anc"),
    // descendants "d1" @101, "d2" @102.
    fn happy() -> Vec<EventView> {
        vec![
            received("p1"),
            received("p2"),
            selected_win("f1"),
            for_fsid("branch_fetch_completed", "f1"),
            for_fsid("branch_prevalidated", "f1"),
            applied("f1", 100, "x"),
            admit(100, "x", "anc"),
            admit(101, "d1", "x"),
            admit(102, "d2", "d1"),
        ]
    }

    #[test]
    fn continuity_ok_yields_continues_selected_branch() {
        let v = derive_post_switch_continuity(&happy());
        assert_eq!(
            v,
            PostSwitchContinuity::ContinuesSelectedBranch {
                admitted_after_switch: 2,
                tip_slot: 102,
                tip_hash: "d2".into(),
            }
        );
    }

    #[test]
    fn broken_parent_link_yields_broken_lineage() {
        let mut t = happy();
        // d2 claims a parent that is not d1 — a branch jump / rollback below X.
        t[8] = admit(102, "d2", "WRONG");
        match derive_post_switch_continuity(&t) {
            PostSwitchContinuity::BrokenLineage {
                at_slot,
                found_prev,
                ..
            } => {
                assert_eq!(at_slot, 102);
                assert_eq!(found_prev, "WRONG");
            }
            other => panic!("expected BrokenLineage, got {other:?}"),
        }
    }

    #[test]
    fn post_switch_diverged_yields_diverged() {
        let mut t = happy();
        t.push(EventView {
            kind: Some("diverged".into()),
            slot: Some(103),
            ..ev("agreement_verdict")
        });
        assert_eq!(
            derive_post_switch_continuity(&t),
            PostSwitchContinuity::Diverged { slot: 103 }
        );
    }

    #[test]
    fn win_without_terminal_yields_dangling() {
        let mut t = happy();
        // A second win with NO terminal — must surface as dangling.
        t.insert(6, selected_win("f2"));
        match derive_post_switch_continuity(&t) {
            PostSwitchContinuity::DanglingForkChoiceWin { fork_switch_id } => {
                assert_eq!(fork_switch_id, "f2");
            }
            other => panic!("expected DanglingForkChoiceWin, got {other:?}"),
        }
    }

    #[test]
    fn no_switch_yields_insufficient_evidence() {
        let t = vec![received("p1"), received("p2"), admit(100, "x", "anc")];
        assert_eq!(
            derive_post_switch_continuity(&t),
            PostSwitchContinuity::InsufficientEvidence {
                reason: ContinuityGap::NoForkSwitchApplied,
            }
        );
    }

    #[test]
    fn continuity_verdict_ignores_peer_tip() {
        // Permuting every peer-tip field must NOT change the verdict — the
        // reducer reads only Ade's own admitted lineage.
        let base = derive_post_switch_continuity(&happy());
        let mut t = happy();
        for e in &mut t {
            if e.event == "agreement_verdict" || e.event == "block_received" {
                e.peer_hash = Some("ZZZ".into());
                e.peer_slot = Some(999_999);
            }
        }
        // Also append a lagging verdict (peer way ahead) — irrelevant to continuity.
        t.push(EventView {
            kind: Some("lagging".into()),
            slot: Some(102),
            peer_slot: Some(999),
            ..ev("agreement_verdict")
        });
        assert_eq!(derive_post_switch_continuity(&t), base);
    }

    #[test]
    fn post_switch_continuity_replays_byte_identical() {
        // The replay invariant (DC-EVIDENCE-05): same events => identical verdict,
        // repeatedly, with no dependence on iteration order / ambient state.
        let t = happy();
        let a = derive_post_switch_continuity(&t);
        let b = derive_post_switch_continuity(&t);
        let c = derive_post_switch_continuity(&t.clone());
        assert_eq!(a, b);
        assert_eq!(b, c);
        assert_eq!(format!("{a:?}"), format!("{c:?}"));
    }

    #[test]
    fn release_window_passes_on_validated_prefix() {
        // Continuity holds, no agreed, but the peer is observed ahead (lagging).
        let mut t = happy();
        t.push(EventView {
            kind: Some("lagging".into()),
            slot: Some(102),
            peer_slot: Some(140),
            ..ev("agreement_verdict")
        });
        match evaluate_release_window(&t, 200, 20) {
            ReleaseVerdict::Pass { terminal, .. } => assert_eq!(
                terminal,
                ConvergenceTerminal::ValidatedPrefixOfPeer {
                    our_slot: 102,
                    peer_slot: 140,
                }
            ),
            other => panic!("expected Pass(ValidatedPrefixOfPeer), got {other:?}"),
        }
    }

    #[test]
    fn release_window_passes_on_agreed_descendant() {
        let mut t = happy();
        t.push(EventView {
            kind: Some("agreed".into()),
            slot: Some(102),
            our_hash: Some("d2".into()),
            peer_hash: Some("d2".into()),
            ..ev("agreement_verdict")
        });
        match evaluate_release_window(&t, 200, 20) {
            ReleaseVerdict::Pass { terminal, .. } => {
                assert_eq!(terminal, ConvergenceTerminal::AgreedAtDescendant { slot: 102 })
            }
            other => panic!("expected Pass(AgreedAtDescendant), got {other:?}"),
        }
    }

    #[test]
    fn release_window_fails_one_peer() {
        let mut t = happy();
        t.retain(|e| !(e.event == "block_received" && e.peer.as_deref() == Some("p2")));
        t.push(EventView {
            kind: Some("agreed".into()),
            slot: Some(100),
            our_hash: Some("x".into()),
            peer_hash: Some("x".into()),
            ..ev("agreement_verdict")
        });
        assert!(matches!(
            evaluate_release_window(&t, 200, 20),
            ReleaseVerdict::FailHardProof { distinct_peers: 1 }
        ));
    }

    #[test]
    fn release_window_fails_no_terminal() {
        // Continuity holds, both peers, but no agreed and no peer-ahead lagging.
        assert!(matches!(
            evaluate_release_window(&happy(), 200, 20),
            ReleaseVerdict::FailNoTerminal
        ));
    }

    #[test]
    fn release_window_prefix_requires_a_followed_descendant() {
        // X admitted but NO descendants + a peer-ahead lagging: the prefix
        // terminal must NOT fire (Ade switched but did not follow forward).
        let mut t = vec![
            received("p1"),
            received("p2"),
            selected_win("f1"),
            for_fsid("branch_fetch_completed", "f1"),
            for_fsid("branch_prevalidated", "f1"),
            applied("f1", 100, "x"),
            admit(100, "x", "anc"),
        ];
        t.push(EventView {
            kind: Some("lagging".into()),
            slot: Some(100),
            peer_slot: Some(140),
            ..ev("agreement_verdict")
        });
        assert!(matches!(
            evaluate_release_window(&t, 200, 20),
            ReleaseVerdict::FailNoTerminal
        ));
    }

    #[test]
    fn release_window_terminal_outside_window_fails() {
        // An agreed verdict far beyond max_slots must NOT count.
        let mut t = happy();
        t.push(EventView {
            kind: Some("agreed".into()),
            slot: Some(100 + 500),
            our_hash: Some("z".into()),
            peer_hash: Some("z".into()),
            ..ev("agreement_verdict")
        });
        assert!(matches!(
            evaluate_release_window(&t, 200, 20),
            ReleaseVerdict::FailNoTerminal
        ));
    }
}
