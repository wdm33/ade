# Invariant Slice AI-S4a — Wire rollback signal preservation

> Slice of PHASE4-N-AI. AI-S1/S2/S3 closed + pushed (`7f78dd98`). **RED wire-signal
> preservation; merges LATENT** — adds + tests the event; the live loop does not consume it
> until AI-S4b. This is *not* fork-choice wiring.

## 2. Slice Header
- **Slice Name:** Preserve the peer's chain-sync `RollBackward` point as a closed
  `AdmissionPeerEvent`.
- **Cluster:** PHASE4-N-AI. **Status:** Proposed.
- **Role:** **Cluster prerequisite for CE-AI-2 / CE-AI-3 live wiring** (NOT a CE prover): it
  preserves the chain-sync rollback point so AI-S4b can route it into detector / orchestrator /
  apply. AI-S4a proves no detector/resolver/apply behavior itself.
- **Dependencies:** none (independent of S1/S2/S3 code).

## 4. Intent
The peer's `RollBackward` **point** is an authority input for fork-choice + durable rollback.
Today the RED admission wire pump **discards** it (`RollBackward { point: _, tip }` →
`TipUpdate` only). This slice makes the point a first-class, closed wire event so it can never
be silently dropped — without any consumer yet acting on it.

## 5. Scope
- **Modules:** RED `ade_runtime::admission` (`wire_pump` + the `AdmissionPeerEvent` enum).
  Mechanical latent skip-arms at existing consumers (`ade_node::node_sync::NodeBlockSource` + any
  other `AdmissionPeerEvent` match) so they compile + ignore the new event.
- **Out of scope (all AI-S4b):** loop consumption, `StreamInput` / orchestrator, ChainDb, forge,
  Participant venue.

## 6. Execution Boundary (TCB color)
- **RED:** `ade_runtime::admission::wire_pump` (the `RollBackward` arm), the `AdmissionPeerEvent`
  enum. Latent skip-arms in `ade_node::node_sync` (RED) + any other consumer.
- **No BLUE, no GREEN.**

## 7. Invariants Preserved (registry IDs)
`DC-PUMP-01` (the pump's closed `AdmissionPeerEvent` contract — extended **additively**;
`Block` / `TipUpdate` / `Disconnected` behavior byte-unchanged),
`[[feedback-shell-must-not-overstate-semantic-truth]]` (closed event vocabulary; the wire
preserves the point, asserts nothing about admission / agreement), `DC-NODE-20` / `DC-CONS-03`
(untouched — no orchestrator, no forge, no selection here).

## 8. Invariants Strengthened / Introduced
- **Strengthens `DC-PUMP-01`** — the closed `AdmissionPeerEvent` vocabulary gains one additive
  variant (`RollBackward { point }`) and the pump now **preserves** the rollback point instead of
  collapsing it to a `TipUpdate`. This is a **cluster prerequisite** for the AI-S4b live wiring
  (CE-AI-2 / CE-AI-3); it does not itself prove detector / resolver / apply behavior.
  *(If the registry has no `DC-PUMP-01`, I'll surface the exact admission-pump event-contract
  rule to strengthen/introduce when writing the registry edit at close.)*

## 9. Design Summary
- **New closed variant:** `AdmissionPeerEvent::RollBackward { peer: String, point: Point, tip: Tip }`
  (`Point` / `Tip` = the existing `ade_network::codec::chain_sync` types `TipUpdate` already
  uses). The **authority field is `point`**; `tip` is carried for transcript / consistency
  (parity with `TipUpdate`).
- **Carry the raw wire `Point`, reject `Origin` before emit (confirmed design):** the RED wire
  layer preserves the peer's protocol signal faithfully (it does not reinterpret the point into a
  different internal type this early), but the surfaced event only carries rollback points this
  rung can handle:
  - `RollBackward { point: Block(slot, hash), tip }` → `AdmissionPeerEvent::RollBackward { peer, point, tip }` (point preserved verbatim).
  - `RollBackward { point: Origin, tip }` → typed fail-closed `AdmissionWirePumpError` (drop peer), **no event emitted** (rollback-to-genesis is unsupported for single-best-peer-within-k).
- **The `RollBackward` arm (`wire_pump.rs:415`)** stops discarding the point and stops
  representing a rollback as a `TipUpdate`; it emits the new event (or fails closed on Origin),
  then continues the existing protocol step (`queue_chain_sync_request_next`).
- **Latent skip-arms:** `NodeBlockSource::next_block` (and any other `AdmissionPeerEvent` match)
  gains a `RollBackward` arm that **ignores** it (parity with how `TipUpdate` is skipped) —
  mechanical, compile-driven, no consumption.

## 10. Changes Introduced
`AdmissionPeerEvent::RollBackward` (closed variant); the `wire_pump` `RollBackward` arm (emit +
Origin fail-closed); a typed `AdmissionWirePumpError::UnsupportedRollbackPoint` (or reuse an
existing fail-closed variant); latent skip-arms at consumers. No durable / state / forge change.

## 11. Replay / Crash / Epoch
Not applicable — RED wire layer, no durable / authoritative state, no determinism surface.

## 12. Mechanical Acceptance Criteria
- [ ] `wire_pump_rollbackward_block_preserves_point` — a chain-sync `RollBackward { Block{slot,hash}, tip }` emits `AdmissionPeerEvent::RollBackward { point: Block{slot,hash}, tip }` (point preserved).
- [ ] `wire_pump_rollbackward_origin_fails_closed` — `RollBackward { Origin, .. }` → typed `AdmissionWirePumpError` (drop peer), **no** event emitted.
- [ ] `wire_pump_block_tipupdate_disconnected_unchanged` — existing `Block` / `TipUpdate` / `Disconnected` emission unchanged.
- [ ] `wire_pump_rollbackward_event_is_latent_until_s4b` — `NodeBlockSource` / existing consumers skip `RollBackward` exactly like `TipUpdate`; no `apply_chain_event`, no `StreamInput::RollBack`, no ChainDb mutation, no forge state change (the latent guarantee, made explicit).
- [ ] New gate **`ci/ci_check_wire_rollback_signal_preserved.sh`**: the old silent downgrade `RollBackward { point: _, tip } => AdmissionPeerEvent::TipUpdate` (rollback represented as a tip update only) is **gone**; the `RollBackward` arm references `point` + emits `AdmissionPeerEvent::RollBackward`; the arm + the consumers contain no `orchestrator` / `chain_selector` / `apply_chain_event` / `StreamInput::RollBack` / `ChainDb` / forge reference (preservation only — latent).
- [ ] `cargo test -p ade_runtime` green; `cargo build -p ade_node` green (latent skip-arms).

## 13. Failure Modes
`AdmissionWirePumpError::UnsupportedRollbackPoint` (Origin) + the existing malformed-message
fail-closed — both drop the peer deterministically; never a silent skip.

## 14. Hard Prohibitions
No orchestrator / `StreamInput` / `apply_chain_event` call; no ChainDb read / mutation; no forge
behavior change; no Participant venue wiring; no loop consumption of the new event (all AI-S4b);
the event vocabulary stays a closed enum; `Block` / `TipUpdate` / `Disconnected` unchanged; the
wire layer asserts no semantic truth beyond "the peer announced a rollback to this point";
**a rollback is never represented as a `TipUpdate` only.**

## 15. Explicit Non-Goals
Loop consumption, fork-choice, orchestrator, venue declaration, forge gate (AI-S4b); multi-peer;
rollback-to-Origin support.

## 16. Completion Checklist
- [ ] `AdmissionPeerEvent::RollBackward { peer, point, tip }` added; the wire-pump arm preserves
  the raw wire point + fails closed on Origin; no rollback-as-TipUpdate-only path remains.
- [ ] Latent skip-arms compile at all consumers; loop behavior unchanged.
- [ ] 4 tests + `ci_check_wire_rollback_signal_preserved.sh` green; `cargo test -p ade_runtime` +
  `cargo build -p ade_node` green.
- [ ] No orchestrator / ChainDb / forge / Participant; existing events unchanged.
