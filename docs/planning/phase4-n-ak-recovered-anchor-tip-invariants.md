# PHASE4-N-AK — Recovered anchor tip is the live-follow start authority

> Invariant sketch (IDD Part I). NARROW regression-remediation cluster (recovery/follow
> authority — NOT evidence emission). **N-AJ is paused until AK restores the live
> recover→follow path.** The DC-NODE-31 registry append is deferred to `/cluster-doc`
> (matching the AJ discipline).

## Context — the regression (confirmed by a live A/B)

Same venue / recovered store / frozen relay / flags, only the binary differs:

- **N-AH binary (`c66fa9a9`) `--mode node --single-producer-venue`** recovers @ a block-8 anchor and
  **FOLLOWS** the frozen relay — sched log `entered_forge_mode: initial_catchup_required ->
  caught_up_to_peer_tip`, `forge_base_block_no: 13` (the relay's frozen tip), then forges 29.
- **Current binary (HEAD, post-N-AJ)** halts at `UnsupportedRollbackPoint`, 0 forges, never follows.

So this is **not** venue drift — a real post-N-AH regression, **exposed (not caused)** by N-AI
AI-S4a's stricter `RollBackward(Origin) => UnsupportedRollbackPoint` refusal. It slipped through
because no live recover→follow was rerun since N-AH (N-AI's rollback-follow was hermetic; the
CE-AI-6 live transcript was deferred) — the AE-lesson trap.

### Root cause (grounded)
`crates/ade_runtime/src/bootstrap.rs::bootstrap_initial_state` (L167):
- L174 `tip = chaindb.tip()` is `None` for a **bare-anchor** recovery (a snapshot exists at the
  anchor slot, but there are **no servable post-anchor blocks** in ChainDb).
- The warm-start materialization target for this snapshot-only case (L198-218) uses the snapshot
  slot with a **null hash** (`Hash32([0u8;32])`) — comment: *"we do not synthesize a tip hash; let
  the caller re-discover the canonical tip."*
- The returned `BootstrapState.tip` (L259-262) = that `None` ChainDb tip.

Consequence: `node_lifecycle::warm_start_recovery` (L1721) returns `state.tip = None`;
`spawn_live_wire_pump_source` (L566 / L784-795) sets ChainSync `start_point = Point::Origin` when
`state.tip` is `None`; the relay's initial cursor `RollBackward(Origin)` then hits AI-S4a
(`wire_pump.rs:447` `Point::Origin => UnsupportedRollbackPoint`) → fail-closed halt. At N-AH there
was no such refusal, so Ade tolerated the Origin restart and rolled forward 1→13. **The fix is the
tip surface, not the (correct) AI-S4a refusal.**

## Pure transformation (the concept is understood)

```
resolve_live_follow_start(servable_chaindb_tip: Option<ChainTip>,
                          recovered_anchor: Option<ChainTip>) -> Option<ChainTip>
  = (1) servable_chaindb_tip            if Some
    (2) else recovered_anchor           if Some(non-Origin)
    (3) else None                       (truly Origin / cold-start)
```
Deterministic, no I/O — `canonical recovered store → canonical live-follow start tip`.

## 1. What must always be true

- **AK-INV-1 (core, new — DC-NODE-31):** After recovery from a non-Origin bootstrap anchor, **the
  live-follow start tip exposed by `BootstrapState` resolves to** the recovered anchor tip (slot +
  **real** hash) whenever ChainDb has no servable post-anchor block. *(Wording is deliberately about
  the live-follow start authority — it does NOT assert that every consumer of `BootstrapState.tip`
  treats the anchor as a servable ChainDb block.)*
- **AK-INV-2:** the wire-pump FindIntersect `start_point` == the exposed live-follow start tip
  (`Origin` **iff** that tip is `None`) — unchanged consumer (`spawn_live_wire_pump_source`).
- **AK-INV-3:** recovery is replay-equivalent (T-REC-05) — same on-disk store ⇒ byte-identical
  `BootstrapState`, now including the resolved live-follow start tip.
- **AK-INV-4 (preserved):** AI-S4a — `RollBackward(Origin)` on the single-best-peer pump stays
  fail-closed (`UnsupportedRollbackPoint`).
- **AK-INV-5 (preserved):** `ChainDb::tip()` returns `Some` only for a servable post-anchor block —
  the storage contract is unchanged.
- **AK-INV-6 (preserved):** `pump_block` sole roll-forward admit; `apply_chain_event` sole rollback
  authority; DC-NODE-28 forge gate; recovered ledger fp == WAL-tail post_fp (T-REC-05).

## 2. What must never be possible

- A non-Origin recovered anchor surfacing as a `None`/Origin live-follow start tip (the regression).
- Synthesizing a servable block in ChainDb to carry the tip (ChainDb must not invent a block).
- Weakening AI-S4a's `RollBackward(Origin)` fail-close.
- Using WAL `admit_count` as the anchor-**hash** proxy (`admit_count==0` carries no hash).
- A true Origin / cold-start recovery surfacing a non-`None` live-follow start tip.
- The resolution overriding a servable ChainDb tip with the anchor when post-anchor blocks exist.
- Any change to the N-AJ convergence-evidence emission.

## 3. Deterministic surface · 4. Replay-equivalent

`resolve_live_follow_start` and the exposed live-follow start tip are pure functions of the on-disk
recovered store (ChainDb tip · snapshot slots · recovered anchor `seed_point`). Same store ⇒
byte-identical live-follow start tip ⇒ byte-identical FindIntersect `start_point`. Extends T-REC-05
from the recovered *ledger* fingerprint to the recovered *tip* surface.

## 5. State transitions in scope

1. `bootstrap_initial_state: (chaindb_tip: Option<ChainTip>, snapshot_slots, recovered_anchor seed_point)
   → Result<BootstrapState{ live_follow_start_tip = resolve_live_follow_start(...) }, BootstrapError>`
2. `spawn_live_wire_pump_source: (live-follow start tip) → start_point` (`Block(tip)` | `Origin`) — **unchanged**
3. wire-pump `RollBackward(point) → event | UnsupportedRollbackPoint(Origin)` — **AI-S4a UNCHANGED**

## 6. TCB color hypothesis

- **`resolve_live_follow_start(...)` / the `BootstrapState` live-follow start tip resolution
  (`ade_runtime/bootstrap.rs`): BLUE-authoritative deterministic recovery decision.** It is
  authoritative — not merely GREEN — because *what point the node recovered to* governs the
  FindIntersect start point, live-follow behavior, and replay-equivalent recovery state. (It lives in
  a crate that also hosts RED orchestration; the *decision* is BLUE regardless of host.)
- recovered anchor `seed_point` (`BootstrapAnchor`, minted from `seed_slot`/`seed_block_hash`):
  canonical input.
- `spawn_live_wire_pump_source` / the wire pump (`node_lifecycle`, `wire_pump.rs`): **RED** shell —
  consumes the resolved tip; **unchanged**.

## 7. Open questions

- **OQ-AK-1 (anchor source) — answered:** the recovered anchor `seed_point` (slot+hash) is already
  recorded (`mithril_bootstrap.rs`: `BootstrapAnchor.seed_point` minted from `seed_slot`/`seed_block_hash`).
  The slice threads that existing anchor point into `bootstrap_initial_state` (extend `BootstrapInputs`
  or read from recovered provenance). No new data; no ChainDb change. *(Mechanism = slice detail.)*
- **OQ-AK-2 (materialization, out of scope):** **AK targets the live-follow start point. It must NOT
  alter ledger materialization unless a test proves materialization currently depends on the
  null-hash placeholder** (`bootstrap.rs:216`). The ledger skeleton is snapshot-addressed; FindIntersect
  consumes the resolved tip. Keep materialization untouched absent such a proof — no snapshot-materialization
  rabbit hole.
- **OQ-AK-3 (admission is diagnostic, not primary):** `--mode node` is in scope — it is the CE-AH-6
  proven live-follow path. `--mode admission` (which surfaced `our_hash=0000`/diverged in the same
  probe) is in scope **only if it consumes the same recovered live-follow start tip helper**; **do not
  redesign admission orchestration in AK.** The admission `0000` is a red herring unless it shares the
  exact helper and gets fixed naturally.

## 8. Proposed registry rule (DECLARE at `/cluster-doc`, do not append now)

- **DC-NODE-31** (family DC, derived) — *Recovered-anchor live-follow start authority.* After recovery
  from a non-Origin bootstrap anchor, the live-follow start tip exposed by `BootstrapState` resolves to
  the recovered anchor tip (slot + real hash) whenever ChainDb has no servable post-anchor block;
  resolution = servable ChainDb tip → recovered anchor (non-Origin) → Origin/None only if truly Origin.
  **Does not change `ChainDb::tip()` semantics and does not synthesize a servable block;** AI-S4a
  `RollBackward(Origin)` fail-close unchanged; replay-equivalent (extends T-REC-05 to the recovered tip
  surface). Targeted **enforced** at AK close (hermetic positive/negative/post-anchor + a
  CE-AH-6-mirroring **live** regression verification). `introduced_in = PHASE4-N-AK`.

## Acceptance bar (carried into the cluster doc)

- **Hermetic positive:** recover @ non-Origin anchor, `admit_count == 0` ⇒ live-follow start tip ==
  anchor (slot+hash) ⇒ wire-pump FindIntersect starts at the anchor ⇒ no Origin fallback.
- **Negative:** a TRUE Origin recovery (cold-start, empty snapshot set) ⇒ live-follow start tip ==
  `None`/Origin (unchanged).
- **Post-anchor:** ChainDb HAS servable blocks above the anchor ⇒ the servable ChainDb tip wins (rule 1).
- **Live regression (the test that would have caught this):** a CE-AH-6-mirroring recover @ block 8 ⇒
  `--mode node --single-producer-venue` follows the frozen relay ⇒ catches up to the relay tip ⇒ no
  `UnsupportedRollbackPoint`.
