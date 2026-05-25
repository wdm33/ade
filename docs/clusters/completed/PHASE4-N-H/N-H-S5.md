# Invariant Slice — PHASE4-N-H S5

## Slice Header

**Slice Name:** Mechanical cross-impl adapter — drive Conway-576 corpus through the full receive pipeline
**Cluster:** PHASE4-N-H
**Status:** In Progress
**CEs addressed:** CE-N-H-5
**Registry effects on merge:** mechanical pre-condition for RO-LIVE-02 (S6 flips it).
**Dependencies:** N-H-S1..S4

---

## Intent

Independent of any external Haskell peer, drive the Conway-576
corpus block-by-block through the full receive pipeline
(`events_to_state` → `receive_apply` → `ChainDbWriter` over
`InMemoryChainDb`) and assert:

* Every corpus block admits successfully (validator-accepted).
* The admitted bytes in ChainDb equal the corpus bytes byte-
  identically (DC-CONS-17 mirror — peer bytes flow through verbatim).
* The ChainDb tip evolves to the expected `(slot, hash)` for each
  admitted block.
* The LedgerState fingerprint changes on admission and matches the
  expected post-application fingerprint per block.

This is the mechanical pre-condition for the RO-LIVE-02 live half
(operator-action, S6).

---

## The change

### 1. New integration test
`crates/ade_runtime/tests/receive_pipeline_corpus_drive.rs`

For each corpus block (run as independent fresh-state admission —
the Conway-576 corpus is not a sequential chain):
- Build a fresh `ReceiveState`.
- Drive `RollForward { ..(slot,hash from block).. } → BlockDelivered
  { block bytes }` via the orchestrator's dispatch entry points.
- Assert `Effect::Cached` then `Effect::Admitted`.
- Assert the InMemoryChainDb tip equals the block's `(slot, hash)`.
- Assert the ChainDb-stored block bytes equal the corpus bytes.
- Assert the LedgerState fingerprint changed from the fresh-state
  fingerprint.

---

## §12 Mechanical Acceptance Criteria (named tests)

In `crates/ade_runtime/tests/receive_pipeline_corpus_drive.rs`:

- `receive_pipeline_corpus_drive_admits_every_block` —
  every corpus block admits successfully through the full pipeline.
- `receive_pipeline_corpus_drive_chaindb_tip_matches_expected` —
  per block: ChainDb tip == block's (slot, hash) after admission.
- `receive_pipeline_corpus_drive_admitted_bytes_equal_corpus_bytes`
  — per block: ChainDb-stored bytes equal corpus input bytes.
- `receive_pipeline_corpus_drive_ledger_fingerprint_changes_on_admit`
  — per block: ledger fingerprint differs from the fresh-state
  fingerprint after admission.

---

## §14 Hard Prohibitions

- No network egress in this test (CI gate already enforced by the
  test's `#[test]` attribute + no `tokio` imports).
- No skipping of corpus blocks — every block must admit.

---

## §15 Explicit Non-Goals

- Live evidence (S6).
- Multi-block sequential admission (the corpus is not a sequential
  chain; sequential follow against a real peer is S6's live path).

---

## Replay obligations

The test is the mechanical replay over the corpus; per-block
independence proves the receive pipeline is corpus-agnostic.

---

## Authority reminder

If this slice conflicts with the project's normative specifications
or the invariant registry, those win.
