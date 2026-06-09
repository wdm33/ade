# PHASE4-N-AI — OQ-1 Rollback Durability — Decision Record

> Read-only investigation, 2026-06-09. Decides the rollback durability mechanism that
> gates the PHASE4-N-AI cluster plan. No code changed.

## Decision

**A — explicit, version-gated `WalEntry::RollBack` marker that re-invokes the existing
rollback authority on replay. B (WAL-tail reconciliation) is NOT clean — rejected.**

This resolves the OQ-1 ambiguity in `DC-NODE-27`. The cluster's first slice is therefore a
small **BLUE** rollback-durability slice (the additive WAL variant + replay arm) — the one
BLUE change rung-2 needs — landing **before** the live-wiring slices.

## Acceptance-bar answers (with evidence)

**Q1 — Where is live rollback currently applied?**
On the live `--mode node` spine, rollback exists in exactly one place: **warm-start recovery
WAL-tail reconciliation** — `ChainDb::rollback_to_slot(wal_tail_slot)` at
`crates/ade_node/src/node_lifecycle.rs:1696` (production warm-start) and its mirror test
helper `crates/ade_runtime/src/recovery/restart.rs:176`. It drops ChainDb blocks **above**
the linear WAL tail (a torn `StoreBlockBytes`-before-`AppendWal` crash orphan).
**Steady-state competing-chain rollback does NOT exist:** `forward_sync::pump_block`
(`pump.rs:76`) is extend-only — a non-linear block falls through the DC-NODE-16 idempotency
no-op to the BLUE chokepoint and **fails closed** (`SlotBeforeLastApplied` /
`BlockNoOutOfOrder`). The DC-CONS-20 lockstep rollback authority
(`receive::reducer::roll_backward` → `rollback::commit_rollback`) exists but is **not wired
to the live spine** (the rollback context is `Option`; only the N-H/N-I receive bridge +
tests supply it).

**Q2 — What durable bytes record that rollback?**
**None.** (a) The warm-start reconciliation writes no new record — it truncates ChainDb to
match the (linear) WAL. (b) `commit_rollback` (`rollback/commit.rs:28`) mutates ChainDb +
ledger + chain_dep **in-memory** and appends **nothing** to the WAL. (c) `WalEntry`
(`wal/event.rs:47`) is a **closed two-variant sum** — `AdmitBlock` +
`SeedEpochConsensusInputsImported` (tag 0 / tag 3; tags 1–2 reserved) — **no `RollBack`**.
The WAL has no rollback representation at all.

**Q3 — On restart, what exact recovery code replays/reconstructs it?**
`node_lifecycle` warm-start (1630–1719): `replay_from_anchor` (BLUE, `wal/replay.rs:93`)
replays `AdmitBlock` entries as a **strictly linear fingerprint chain** — each `prior_fp`
MUST equal the previous `post_fp`, else `ChainBreak` (fail-closed). **No fork handling, no
rollback during replay.** Then `rollback_to_slot(wal_tail_slot)` drops orphans above the
linear tail; then `bootstrap_initial_state` forward-replays from the nearest snapshot ≤ tip.

**Q4 — Does recovery reproduce the same tip / ledger-fp / chain_dep for a live rollback?**
For a **linear** chain: yes (T-REC-05 enforces `recovered_fp == wal_tail_post_fp`). For a
chain that underwent a **live rollback: NO, dangerously** — because `commit_rollback` writes
no WAL record and the WAL is append-only + linear:
- If both the abandoned and adopted branches' `AdmitBlock`s are in the WAL →
  `replay_from_anchor` **`ChainBreak`s** (adopted `prior_fp` ≠ abandoned `post_fp`).
  Unrecoverable.
- If the WAL tail is the abandoned branch → recovery reconciles to the **abandoned** tip →
  **resurrects the abandoned branch.** This is exactly the "converge live, resurrect after
  restart" constitution violation OQ-1 exists to prevent.

**Q5 — Canonical, append-only, CI-testable?**
The WAL is canonical + append-only + CI-tested (CN-WAL-01, DC-WAL-01/02/03) — but has **no
rollback mechanism to extend**. There is nothing for B to lean on.

**Q6 — Reuse `materialize_rolled_back_state` + the lockstep reducer, or a second authority?**
The single rollback authority (`commit_rollback` + `materialize_rolled_back_state`,
CN-STORE-07) is **not WAL-durable and not live-wired**. The warm-start orphan-drop is a
**separate, narrower** truncation (not routed through `commit_rollback`/`materialize`).
Neither records a replayable live rollback.

## Why B fails the skeptical bar

B requires PROOF that: live rollback → abandoned tail durably excluded/reconciled → restart
cannot resurrect → replay byte-identical. The existing path proves **none** of this for a
competing-chain rollback. It handles only the narrow orphan-above-tail crash case and assumes
a **linear** WAL; `commit_rollback` leaves no durable record; `replay_from_anchor`
`ChainBreak`s on any non-linear WAL. Per the default-skeptical rule, B is rejected.

## A — the chosen mechanism (shape only; not implementation)

A version-gated additive **`WalEntry::RollBack { to_point, reason, prior_tip, selected_tip }`**
(tag 1 — the reserved RollBackward slot; `AdmitBlock`=0 / tag 2 CaptureSnapshot reserved / `SeedEpochConsensusInputsImported`=3):
- **Append-only** — a new append, never a WAL mutation (CN-WAL-01 preserved).
- **Canonical bytes** — deterministic CBOR; tag-gated; a decoder that does not know the tag
  fails closed (the existing `WalEntry` walks `match` exhaustively — a new variant is a
  compile error in every walk, by design, per the `wal/event.rs` note).
- **NOT a second rollback implementation** — on replay, a `RollBack` entry **re-invokes the
  existing `materialize_rolled_back_state` (CN-STORE-07) + lockstep reducer (DC-CONS-20)** at
  `to_point`. The entry is a durable **marker** ("roll back here via the existing
  authority"), nothing more.
- **Replay fingerprint-chain extension (the BLUE change):** `replay_from_anchor`
  (`wal/replay.rs`) gains a `RollBack` arm that re-anchors `prev_post_fp` to the materialized
  rolled-back fp, so the `AdmitBlock` chain after a `RollBack` links from the rolled-back
  state — replacing today's `ChainBreak` with a faithful linear-with-rollbacks replay.

## Implications for `/cluster-plan`

- **A adds the one BLUE change rung-2 needs:** the additive `WalEntry::RollBack` variant +
  encode/decode + the `replay_from_anchor` `RollBack` arm. This is sanctioned by the existing
  seam doctrine (SEAMS §7 candidate #9: "the append-only WAL schema — `WalEntry` is a
  CE-not-law additively-evolvable surface"). It corrects the planning sketch's "expected new
  BLUE: zero" — the rollback-durability slice is the **one** BLUE touch.
- **Slice ordering:** the BLUE rollback-durability slice (WAL variant + replay arm,
  re-invoking the existing authority) is **foundational** and lands **first** — the live
  apply driver (DC-NODE-25), reconciliation (DC-NODE-26), and replay-equivalence (DC-NODE-27)
  all depend on a WAL-durable rollback.
- **DC-NODE-27** stays `declared`; its OQ-1 mechanism is now **A**. Its `code_locus` narrows
  to the `WalEntry::RollBack` marker + `replay_from_anchor` arm re-invoking
  `materialize_rolled_back_state` / `commit_rollback`.

## Status

OQ-1 **resolved → A**. The cluster is now unblocked for `/cluster-plan`. `d92f9ce8` (the
invariants commit) remains unpushed pending the user's call.
