// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// PHASE4-N-E S3 (DC-MEM-04 multi-peer half): deterministic per-peer
// canonicalizer. Round-robin by sorted PeerId — peers are visited in
// byte-lex PeerId order; each round emits one tx from every peer that
// still has one. Pure function of the inputs; no I/O, no concurrency,
// no HashMap/HashSet, no clocks.

use crate::mempool::ingress::{IngressEvent, IngressSource};

/// Opaque peer identifier. Ordering is byte-lex on `PeerId.0`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PeerId(pub Vec<u8>);

/// One peer's submission queue: an ordered list of tx CBOR byte strings
/// in arrival order, paired with the source variant the peer carries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeerSubmissionQueue {
    pub peer: PeerId,
    pub source: IngressSource,
    pub txs: Vec<Vec<u8>>,
}

/// Deterministic round-robin canonicalization: peers are visited in
/// `PeerId` byte-lex order; each round emits one tx from every peer that
/// still has one, in `PeerId` order. Repeats until every queue is drained.
///
/// Pure: `canonicalize_peer_streams(qs) == canonicalize_peer_streams(qs)`
/// for any `qs`, and independent of `qs`'s iteration order — the function
/// sorts internally by `peer`. Tx bytes are passed verbatim.
pub fn canonicalize_peer_streams(queues: &[PeerSubmissionQueue]) -> Vec<IngressEvent> {
    // Sort queues by PeerId byte-lex; ties broken by source (N2N < N2C
    // by enum-declaration order is NOT a stable property — we order by
    // source byte-tag explicitly).
    let mut sorted: Vec<&PeerSubmissionQueue> = queues.iter().collect();
    sorted.sort_by(|a, b| match a.peer.cmp(&b.peer) {
        core::cmp::Ordering::Equal => source_byte(a.source).cmp(&source_byte(b.source)),
        ord => ord,
    });

    // Round-robin: round[i] = sorted[0].txs[i], sorted[1].txs[i], ...
    // Skip a peer whose queue is exhausted at this round.
    let max_len = sorted.iter().map(|q| q.txs.len()).max().unwrap_or(0);
    let mut out: Vec<IngressEvent> = Vec::new();
    for i in 0..max_len {
        for q in &sorted {
            if let Some(tx) = q.txs.get(i) {
                out.push(IngressEvent::new(q.source, tx.clone()));
            }
        }
    }
    out
}

/// Single-byte tag for IngressSource — stable across binary builds.
/// Used only for tie-break ordering inside `canonicalize_peer_streams`.
fn source_byte(s: IngressSource) -> u8 {
    match s {
        IngressSource::N2N => 0,
        IngressSource::N2C => 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pid(bytes: &[u8]) -> PeerId {
        PeerId(bytes.to_vec())
    }

    fn queue(peer: &[u8], source: IngressSource, txs: &[&[u8]]) -> PeerSubmissionQueue {
        PeerSubmissionQueue {
            peer: pid(peer),
            source,
            txs: txs.iter().map(|b| b.to_vec()).collect(),
        }
    }

    #[test]
    fn single_peer_canonicalizes_to_submission_order() {
        let q = queue(b"A", IngressSource::N2N, &[b"tx0", b"tx1", b"tx2"]);
        let events = canonicalize_peer_streams(&[q]);
        assert_eq!(events.len(), 3);
        assert_eq!(events[0].tx_bytes(), b"tx0");
        assert_eq!(events[1].tx_bytes(), b"tx1");
        assert_eq!(events[2].tx_bytes(), b"tx2");
        for e in &events {
            assert_eq!(e.source(), IngressSource::N2N);
        }
    }

    #[test]
    fn multi_peer_round_robin_by_sorted_peer_id() {
        let qa = queue(b"A", IngressSource::N2N, &[b"a0", b"a1", b"a2"]);
        let qb = queue(b"B", IngressSource::N2N, &[b"b0", b"b1", b"b2"]);
        let qc = queue(b"C", IngressSource::N2N, &[b"c0", b"c1", b"c2"]);

        let events = canonicalize_peer_streams(&[qa, qb, qc]);
        let bytes: Vec<&[u8]> = events.iter().map(|e| e.tx_bytes()).collect();

        // Round 0: a0, b0, c0; Round 1: a1, b1, c1; Round 2: a2, b2, c2.
        assert_eq!(
            bytes,
            [
                b"a0".as_ref(),
                b"b0".as_ref(),
                b"c0".as_ref(),
                b"a1".as_ref(),
                b"b1".as_ref(),
                b"c1".as_ref(),
                b"a2".as_ref(),
                b"b2".as_ref(),
                b"c2".as_ref(),
            ]
        );
    }

    #[test]
    fn unsorted_input_canonicalizes_identically_to_sorted_input() {
        let qa = queue(b"A", IngressSource::N2N, &[b"a0", b"a1"]);
        let qb = queue(b"B", IngressSource::N2N, &[b"b0", b"b1"]);
        let qc = queue(b"C", IngressSource::N2N, &[b"c0", b"c1"]);

        let sorted = canonicalize_peer_streams(&[qa.clone(), qb.clone(), qc.clone()]);
        let shuffled1 = canonicalize_peer_streams(&[qc.clone(), qa.clone(), qb.clone()]);
        let shuffled2 = canonicalize_peer_streams(&[qb.clone(), qc.clone(), qa.clone()]);

        assert_eq!(sorted, shuffled1);
        assert_eq!(sorted, shuffled2);
    }

    #[test]
    fn empty_queue_for_a_peer_skipped() {
        let qa = queue(b"A", IngressSource::N2N, &[b"a0"]);
        let qb = queue(b"B", IngressSource::N2N, &[]);
        let qc = queue(b"C", IngressSource::N2N, &[b"c0"]);

        let events = canonicalize_peer_streams(&[qa, qb, qc]);
        let bytes: Vec<&[u8]> = events.iter().map(|e| e.tx_bytes()).collect();
        assert_eq!(bytes, [b"a0".as_ref(), b"c0".as_ref()]);
    }

    #[test]
    fn peer_with_longest_queue_finishes_alone() {
        let qa = queue(b"A", IngressSource::N2N, &[b"a0", b"a1", b"a2"]);
        let qb = queue(b"B", IngressSource::N2N, &[b"b0"]);

        let events = canonicalize_peer_streams(&[qa, qb]);
        let bytes: Vec<&[u8]> = events.iter().map(|e| e.tx_bytes()).collect();

        // Round 0: a0, b0. Round 1: a1 (B drained). Round 2: a2.
        assert_eq!(
            bytes,
            [b"a0".as_ref(), b"b0".as_ref(), b"a1".as_ref(), b"a2".as_ref()]
        );
    }

    #[test]
    fn tx_bytes_preserved_verbatim() {
        // The canonicalizer must not normalize, truncate, or re-encode
        // tx bytes. Use a payload covering all byte values.
        let payload: Vec<u8> = (0u8..=255u8).collect();
        let q = queue(b"X", IngressSource::N2N, &[payload.as_slice()]);
        let events = canonicalize_peer_streams(&[q]);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].tx_bytes(), payload.as_slice());
    }

    #[test]
    fn source_propagated_from_queue() {
        let qa = queue(b"A", IngressSource::N2N, &[b"a0"]);
        let qb = queue(b"B", IngressSource::N2C, &[b"b0"]);
        let events = canonicalize_peer_streams(&[qa, qb]);
        assert_eq!(events[0].source(), IngressSource::N2N);
        assert_eq!(events[1].source(), IngressSource::N2C);
    }

    #[test]
    fn empty_input_produces_empty_output() {
        let events = canonicalize_peer_streams(&[]);
        assert!(events.is_empty());
    }

    #[test]
    fn same_peer_id_same_source_stable_ordering() {
        // Pathological: two queues sharing the same PeerId. The function
        // must still be deterministic; the tie-break is `source` byte tag.
        let q1 = PeerSubmissionQueue {
            peer: pid(b"X"),
            source: IngressSource::N2N,
            txs: vec![b"n2n".to_vec()],
        };
        let q2 = PeerSubmissionQueue {
            peer: pid(b"X"),
            source: IngressSource::N2C,
            txs: vec![b"n2c".to_vec()],
        };
        let e1 = canonicalize_peer_streams(&[q1.clone(), q2.clone()]);
        let e2 = canonicalize_peer_streams(&[q2, q1]);
        assert_eq!(e1, e2, "same-peer-id input must canonicalize deterministically");
        assert_eq!(e1[0].source(), IngressSource::N2N, "N2N (source tag 0) before N2C (tag 1)");
        assert_eq!(e1[1].source(), IngressSource::N2C);
    }
}
