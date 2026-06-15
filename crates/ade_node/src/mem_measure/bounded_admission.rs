//! GREEN: deterministic bounded inbound-admission gate (CN-MEM-01).
//!
//! Untrusted inbound work is admitted through a deterministic bounded policy
//! BEFORE the scarce authoritative resource — the BLUE `mempool_ingress`
//! validation — is consumed. The bound is a fixed, closed per-batch budget
//! (count AND cumulative bytes); over-budget events are deterministically shed,
//! head-of-line, WITHOUT reaching the authoritative path. The gate NEVER
//! changes the validity verdict of a forwarded event: a forwarded tx gets
//! exactly the `AdmitOutcome` a direct `mempool_ingress` call would yield (the
//! DC-MEM-01 / DC-MEM-03 floor). Pure: no clock, no RNG, no `HashMap`, no
//! float, no I/O.

use ade_ledger::mempool::{mempool_ingress, AdmitOutcome, IngressEvent, MempoolState};
use ade_ledger::state::LedgerState;

/// Fixed, closed per-batch event-count budget. Non-configurable (no feature
/// flag, no env). Mirrors the `MAX_SERVE_RANGE_BLOCKS` / `MAX_WIRE_PUMP_LOOKAHEAD`
/// closed-constant pattern: a bound is a correctness constant, not a tuning knob.
pub const MAX_INBOUND_ADMISSION_COUNT: usize = 1024;

/// Fixed, closed per-batch cumulative tx-bytes budget (4 MiB), symmetric in
/// spirit with the inbound-reassembly / serve-range byte bounds elsewhere.
pub const MAX_INBOUND_ADMISSION_BYTES: usize = 4 * 1024 * 1024;

/// Closed reason an inbound event was shed by the bound — it never reached the
/// authoritative `mempool_ingress`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShedReason {
    /// The per-batch event-count budget was already exhausted.
    CountBudgetExhausted,
    /// Admitting this event would exceed the per-batch cumulative-byte budget.
    ByteBudgetExhausted,
}

/// Closed outcome of the bounded gate for one inbound event.
///
/// `Forwarded` means the event passed the bound and reached `mempool_ingress`;
/// the inner `AdmitOutcome` is the unchanged BLUE verdict. `Shed` means the
/// event was dropped by the bound before any authoritative resource was used.
/// The two are distinct variants by construction: a shed event is NEVER an
/// acceptance.
#[derive(Debug, Clone, PartialEq)]
pub enum BoundedOutcome {
    /// Passed the bound; carries the unchanged BLUE `mempool_ingress` verdict.
    Forwarded(AdmitOutcome),
    /// Dropped by the bound before the authoritative path; carries the reason.
    Shed(ShedReason),
}

/// Fold an ordered, canonical inbound trace through the bounded gate against a
/// single `base` ledger state.
///
/// Determinism: identical `(base, events)` always yield identical
/// `(MempoolState, Vec<BoundedOutcome>)`. The gate forwards events in input
/// order while BOTH budgets hold; the first event that would breach either
/// budget — and every event after it — is shed (head-of-line). The number of
/// `Forwarded` events (hence `mempool_ingress` calls) is therefore always
/// `<= MAX_INBOUND_ADMISSION_COUNT`, and their cumulative `tx_bytes` length is
/// always `<= MAX_INBOUND_ADMISSION_BYTES`.
///
/// The caller is responsible for canonical event ordering (e.g. the
/// `canonicalize_peer_streams` authority); the bound is deterministic given a
/// fixed order, exactly as the unbounded `replay_ingress_trace` is.
pub fn replay_bounded_ingress_trace(
    base: LedgerState,
    events: &[IngressEvent],
) -> (MempoolState, Vec<BoundedOutcome>) {
    let mut mempool = MempoolState::new(base);
    let mut outcomes = Vec::with_capacity(events.len());
    let mut forwarded_count: usize = 0;
    let mut forwarded_bytes: usize = 0;
    // Once a budget is breached the batch is closed head-of-line: this records
    // the closing reason so every subsequent event sheds with it.
    let mut closed: Option<ShedReason> = None;

    for event in events {
        let outcome = match closed {
            Some(reason) => BoundedOutcome::Shed(reason),
            None => {
                let event_bytes = event.tx_bytes().len();
                if forwarded_count >= MAX_INBOUND_ADMISSION_COUNT {
                    closed = Some(ShedReason::CountBudgetExhausted);
                    BoundedOutcome::Shed(ShedReason::CountBudgetExhausted)
                } else if forwarded_bytes.saturating_add(event_bytes) > MAX_INBOUND_ADMISSION_BYTES
                {
                    closed = Some(ShedReason::ByteBudgetExhausted);
                    BoundedOutcome::Shed(ShedReason::ByteBudgetExhausted)
                } else {
                    let (next, admit) = mempool_ingress(&mempool, event);
                    mempool = next;
                    forwarded_count += 1;
                    forwarded_bytes += event_bytes;
                    BoundedOutcome::Forwarded(admit)
                }
            }
        };
        outcomes.push(outcome);
    }

    (mempool, outcomes)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use ade_ledger::mempool::{mempool_ingress, AdmitOutcome, IngressEvent, IngressSource};
    use ade_ledger::state::LedgerState;
    use ade_testkit::mempool::{
        b_track_corpus_as_ingress, replay_ingress_trace, BTrackCase, ExpectedOutcome,
    };
    use ade_types::CardanoEra;

    fn junk(byte: u8) -> IngressEvent {
        IngressEvent::new(IngressSource::N2N, vec![byte])
    }

    #[test]
    fn bounded_admission_respects_count_budget() {
        let base = LedgerState::new(CardanoEra::Conway);
        let n_extra = 5usize;
        let total = MAX_INBOUND_ADMISSION_COUNT + n_extra;
        let events: Vec<IngressEvent> = (0..total).map(|_| junk(0x80)).collect();

        let (_, outs) = replay_bounded_ingress_trace(base, &events);

        let forwarded = outs
            .iter()
            .filter(|o| matches!(o, BoundedOutcome::Forwarded(_)))
            .count();
        let shed = outs
            .iter()
            .filter(|o| matches!(o, BoundedOutcome::Shed(ShedReason::CountBudgetExhausted)))
            .count();
        assert_eq!(
            forwarded, MAX_INBOUND_ADMISSION_COUNT,
            "exactly the budget forwards"
        );
        assert_eq!(
            shed, n_extra,
            "the overflow sheds with CountBudgetExhausted"
        );

        // Head-of-line: nothing is forwarded after the first shed.
        let first_shed = outs
            .iter()
            .position(|o| matches!(o, BoundedOutcome::Shed(_)))
            .unwrap();
        assert!(outs[first_shed..]
            .iter()
            .all(|o| matches!(o, BoundedOutcome::Shed(_))));
    }

    #[test]
    fn bounded_admission_respects_byte_budget() {
        let base = LedgerState::new(CardanoEra::Conway);
        // 1.5 MiB each: two fit (3 MiB), the third breaches the 4 MiB budget.
        let big = vec![0u8; 3 * 1024 * 1024 / 2];
        let events: Vec<IngressEvent> = (0..4)
            .map(|_| IngressEvent::new(IngressSource::N2N, big.clone()))
            .collect();

        let (_, outs) = replay_bounded_ingress_trace(base, &events);

        let forwarded = outs
            .iter()
            .filter(|o| matches!(o, BoundedOutcome::Forwarded(_)))
            .count();
        assert_eq!(
            forwarded, 2,
            "two 1.5 MiB events fit under the 4 MiB budget"
        );
        assert!(matches!(
            outs[2],
            BoundedOutcome::Shed(ShedReason::ByteBudgetExhausted)
        ));
        assert!(matches!(
            outs[3],
            BoundedOutcome::Shed(ShedReason::ByteBudgetExhausted)
        ));
        assert!(2 * big.len() <= MAX_INBOUND_ADMISSION_BYTES);
    }

    #[test]
    fn bounded_admission_is_deterministic() {
        let base = LedgerState::new(CardanoEra::Conway);
        let events: Vec<IngressEvent> = (0..50u8)
            .map(|i| IngressEvent::new(IngressSource::N2N, vec![0x80, i]))
            .collect();

        let (m1, o1) = replay_bounded_ingress_trace(base.clone(), &events);
        let (m2, o2) = replay_bounded_ingress_trace(base, &events);
        assert_eq!(m1, m2, "MempoolState is byte-identical across runs");
        assert_eq!(o1, o2, "outcome trace is byte-identical across runs");
    }

    #[test]
    fn bounded_gate_under_budget_equals_unbounded() {
        // For each B-track case, a single-event trace is trivially under budget;
        // the bounded fold must equal the unbounded `replay_ingress_trace`.
        let cases = b_track_corpus_as_ingress(IngressSource::N2N);
        assert!(cases.len() >= 5, "valid + 4 adversarial mutations");

        for BTrackCase { event, base, .. } in cases {
            let evs = [event];
            let (m_b, o_b) = replay_bounded_ingress_trace(base.clone(), &evs);
            let (m_u, o_u) = replay_ingress_trace(base, &evs);

            assert_eq!(m_b, m_u, "below the cap the mempool evolves identically");
            assert_eq!(o_b.len(), o_u.len());
            for (bo, uo) in o_b.iter().zip(o_u.iter()) {
                match bo {
                    BoundedOutcome::Forwarded(a) => assert_eq!(a, uo, "verdict preserved"),
                    BoundedOutcome::Shed(_) => panic!("an under-budget event must not be shed"),
                }
            }
        }
    }

    #[test]
    fn bounded_gate_preserves_admit_verdict() {
        // A single forwarded event's inner verdict equals a direct call.
        let cases = b_track_corpus_as_ingress(IngressSource::N2N);
        for BTrackCase { event, base, .. } in cases {
            let mempool = MempoolState::new(base.clone());
            let (_, direct) = mempool_ingress(&mempool, &event);
            let (_, outs) = replay_bounded_ingress_trace(base, std::slice::from_ref(&event));
            match &outs[0] {
                BoundedOutcome::Forwarded(a) => assert_eq!(*a, direct),
                BoundedOutcome::Shed(_) => panic!("single under-budget event must forward"),
            }
        }
    }

    #[test]
    fn bounded_gate_no_false_accept_under_pressure() {
        let cases = b_track_corpus_as_ingress(IngressSource::N2N);
        let valid = cases
            .into_iter()
            .find(|c| matches!(c.expected, ExpectedOutcome::Admit))
            .expect("corpus has a valid case");

        // Within budget: the valid tx forwards and is admitted.
        let (_, ok) =
            replay_bounded_ingress_trace(valid.base.clone(), std::slice::from_ref(&valid.event));
        assert!(matches!(
            ok[0],
            BoundedOutcome::Forwarded(AdmitOutcome::Admitted { .. })
        ));

        // Over budget: the SAME valid tx is shed — never silently accepted.
        let mut trace: Vec<IngressEvent> = (0..MAX_INBOUND_ADMISSION_COUNT)
            .map(|_| junk(0x80))
            .collect();
        trace.push(valid.event.clone());
        let (_, over) = replay_bounded_ingress_trace(valid.base, &trace);
        let last = over.last().unwrap();
        assert!(
            matches!(last, BoundedOutcome::Shed(ShedReason::CountBudgetExhausted)),
            "an over-budget valid tx is shed (liveness drop), never a false accept"
        );
        assert!(!matches!(last, BoundedOutcome::Forwarded(_)));
    }
}
