# Invariant Slice — PHASE4-N-H S6

## Slice Header

**Slice Name:** `live_block_follow_session` evidence binary + CE-N-H-6 procedure
**Cluster:** PHASE4-N-H
**Status:** In Progress
**CEs addressed:** CE-N-H-6 (conditional)
**Registry flips on merge:** `RO-LIVE-02` → `partial` +
`open_obligation = "blocked_until_operator_peer_available"` (no
private peer at HEAD); the operator flips it to `enforced` after
capturing the log.
**Dependencies:** N-H-S1..S5

---

## Intent

Ship the operator-action binary that drives the receive pipeline
against a real cardano-node peer and captures a tip-following log.
Mirrors the N-C `live_block_production_session` and N-G
`live_block_fetch_session` patterns:

* Hermetic default mode: prints readiness and exits 0. No sockets,
  no operator key material (the receive side has no keys anyway).
* `--connect` mode: drives the receive pipeline against a real
  cardano-node peer; captures `CE-N-H-LIVE_<date>.log` (JSONL,
  one record per admitted block) with the ChainDb-tip vs. peer-tip
  agreement at each step.

The `--connect` mode is a stub at this slice — the receive
orchestrator (S4) is a pure driver; plugging it into a tokio socket
is one layer up. Same shape as N-G's `live_block_fetch_session`.

Procedure doc records the flip-to-`enforced` ritual.

---

## The change

### 1. New binary `crates/ade_core_interop/src/bin/live_block_follow_session.rs`

### 2. New procedure doc `docs/clusters/PHASE4-N-H/CE-N-H-6_PROCEDURE.md`

### 3. New build-and-start test `crates/ade_core_interop/tests/live_block_follow_session.rs`

### 4. CI gate `ci/ci_check_receive_paths_corpus_present.sh`

Positive presence: cross-impl test + transcript replay test + binary
source + Cargo.toml `[[bin]]` entry + procedure doc.

---

## §12 Mechanical Acceptance Criteria (named tests)

In `crates/ade_core_interop/tests/live_block_follow_session.rs`:
- `live_block_follow_session_hermetic_default_prints_readiness`.

CI: `ci/ci_check_receive_paths_corpus_present.sh` (new).

---

## §14 Hard Prohibitions

- Hermetic default MUST NOT open sockets or read operator material.
- Binary must not depend on `producer::signing` (receive side has no
  keys; the orchestrator dep-boundary CI covers the underlying
  module).
- No credentials / peer IPs / private topology in the procedure
  doc (public-repo discipline).

---

## §15 Explicit Non-Goals

- Wiring the binary to a specific peer — operator scope.
- Full mux-level fan-out — narrow scope: single-peer single-protocol
  follow.

---

## Replay obligations

None added — the mechanical replay corpus is in S3/S5.

---

## Authority reminder

If this slice conflicts with the project's normative specifications
or the invariant registry, those win.
