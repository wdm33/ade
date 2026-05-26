# Invariant Slice — PHASE4-N-L S9

**Slice Name:** Replay-equivalence harness — session reducer over recorded byte-chunk transcript.
**Cluster:** PHASE4-N-L
**Status:** In Progress
**CEs addressed:** CE-N-L-5 (DC-SESS-03).
**Dependencies:** S2, S3.

## Intent

Prove the session reducer (S2) under any recorded byte-chunk corpus produces byte-identical effects across two runs. Pin the property mechanically and ship the corpus shape so future wire-layer changes are caught.

## Scope

- `crates/ade_network/tests/session_replay_equivalence.rs` — integration test that builds a byte-chunk transcript in-test (mirrors the N-K replay harness pattern), runs the session reducer twice, asserts identical effects.

## Design

```rust
#[test]
fn session_replay_equivalence_holds() {
    let chunks = build_canonical_byte_chunk_transcript();
    let run = || {
        let mut state = SessionState::new_handshaking_for_test();
        let mut effects = Vec::new();
        for chunk in &chunks {
            effects.extend(session::core::step(&mut state, chunk.clone()).expect("step"));
        }
        effects
    };
    let a = run();
    let b = run();
    assert_eq!(a, b);
}
```

## §12 Mechanical Acceptance Criteria

- [ ] `session_replay_equivalence_holds` passes.
- [ ] `session_replay_corpus_builds_deterministically` — the in-test corpus builder is deterministic.

## §14 Hard Prohibitions

- No tokio in the harness file (the session reducer is sync).
- No regenerating the corpus inside the test.

## §15 Non-Goals

- No live peer corpus (operator-action follow-on).
- No cross-impl equivalence with cardano-node wire bytes (Ade's session reducer is project-internal; wire bytes go through the existing BLUE codecs which already have per-mini-protocol oracle tests under N-A).
