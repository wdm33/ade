# Invariant Slice — PHASE4-N-L-LIVE S3

**Slice Name:** Hermetic loopback responder tests + operator-pass procedure doc.
**Cluster:** PHASE4-N-L-LIVE
**Status:** In Progress
**CEs addressed:** CE-N-L-LIVE-1 (mechanical full coverage), CE-N-L-LIVE-6 (operator-pass procedure doc).
**Dependencies:** S2.

## Intent

Ship the integration tests that exercise the wire-only mode
end-to-end against an in-process loopback responder + write the
operator-pass procedure document so the AWS live pass is
deterministic.

## Scope

- `crates/ade_node/tests/wire_only_loopback.rs` — integration
  tests per the slice's mechanical-acceptance list.
- `docs/clusters/PHASE4-N-L-LIVE/CE-N-L-LIVE-PROCEDURE.md` —
  operator-pass procedure (verbatim shell commands, expected
  JSONL shape, attachment instructions).
- `docs/clusters/PHASE4-N-L-LIVE/CE-N-L-LIVE-LIVE.log` —
  placeholder pending operator pass.

## §12 Mechanical Acceptance Criteria

- [ ] All S2 tests pass in CI (`cargo test -p ade_node
  --test wire_only_loopback`).
- [ ] The operator-pass procedure doc lists the verbatim
  command line + the expected event sequence.

## §14 Hard Prohibitions

- No emitting fake JSONL lines into the operator log file;
  the placeholder MUST be replaced at cluster close by the
  real captured log (or the cluster does NOT close).

## §15 Non-Goals

- No automation of the AWS pass — the operator runs it
  manually.
