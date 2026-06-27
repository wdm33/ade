# LIVE-LEDGER-EPOCH-TRANSITION — the continuous self-sustaining ledger evolution loop

> **Status:** Planning Artifact (Non-Normative). Describes how the work is organized and sequenced;
> introduces no new requirements beyond the invariant it declares (DC-EPOCH-19). If this conflicts
> with a normative document, the normative document wins.

**Central rule:** DC-EPOCH-19 (declared by this cluster). **Builds on:** DC-EPOCH-13..18 (automatic
activation, the MARK bridge, the live eta0 evolution, the per-boundary authority advance, the
bootstrap reward update), the MEM-OPT architecture (the disk-backed reduced UTxO checkpoint;
`track_utxo=false`), DC-EVIEW-* (the window-replay substrate).

---

## 1. The invariant (blunt)

> **DC-EPOCH-19.** After every durable selected-chain block, Ade has enough durable, replayable state
> to derive EVERY future epoch transition — rewards, stake snapshots, pool/cert lifecycle, and
> leadership authority — WITHOUT another Mithril import, an external CLI oracle, or a manually injected
> authority.

This is an *inside-out* invariant. Every prior epoch cluster validated the system *outside-in* ("can it
cross boundary 1340?"), which is exactly why each boundary exposed the next special case. The thing that
must be made impossible to violate is: **the node never runs out of self-derived future authority.**

---

## 2. The gap — the authority-surface trace (verified at the file level, 2026-06-27)

For one admitted selected-chain block, every authoritative state object and how it is handled today:

| State object | Status | Evidence |
|---|---|---|
| UTxO → reduced checkpoint | ① live | `advance_reduced_checkpoint_to_durable_tip` per sync pass (node_lifecycle.rs:2200) |
| Praos chain-dep nonce (eta0) | ① live | per-block contribution + `EpochBoundary` tick (node_sync.rs:642, DC-EPOCH-16) |
| WAL / recovery snapshot | ① live | activation records + `PersistentSnapshotCache.capture` (node_sync.rs:731) |
| Cert state (delegations / withdrawals) | ② replay-only | `advance_cert_state` called ONLY in the window-replay (reduced_window_driver.rs:113) |
| Pool lifecycle / POOLREAP | ② replay-only | `apply_pool_reap` per crossed boundary (reduced_window_driver.rs:99) |
| Leadership stake aggregate | ② replay-only | `aggregate_pool_stake` — the go-for-E via replay(E-2) |
| Protocol / governance params | ③ bootstrap-import | snapshot pparams; no live enactment on the follow |
| Seed snapshots (mark/set/go) | ③ bootstrap-import | snapshot esSnapshots; `rotate_snapshots` only in full-ledger rules.rs:1130 |
| **block_production accounting** | **④ MISSING** | incremented NOWHERE; fed only by the snapshot decode |
| **epoch_fees accumulation** | **④ MISSING (live)** | accumulated only in full-ledger phase.rs:199, never on the follow |
| **Reward update (RUPD)** | **④ MISSING** | `apply_epoch_boundary_with_registrations` has ZERO live callers — only inside full-ledger `apply_block`, which the node never invokes |
| **Snapshot rotation mark→set→go** | **④ MISSING (live)** | only in full-ledger rules.rs |

`pump_block` (the sole tip-advancing call, forward_sync/pump.rs:83) delegates to `forward_sync_step` —
header-validate + store + chain-dep, nothing else. **The live node never applies a ledger transition.**

**The finding.** Categories ② and ④ are the gap: the cert state is only ever *reconstructed* by a
seed-anchored replay, and rewards / block-counts / fees / snapshot-rotation are *not produced at all*.
Seed+1 (the MARK bridge, DC-EPOCH-15) and seed+2 (Mithril's precomputed nesRu, DC-EPOCH-18) are
**bootstrap-transient borrows**. There is no authoritative source for the seed+3 reward update because
nothing on the follow path produces it. Ade is a header-validating follower with a bounded stake-window
replay — not yet a continuously self-sustaining ledger.

---

## 3. The design — a maintained bounded accumulator beside the disk-backed checkpoint

Preserve the low-memory architecture. Split the state by size:

```
durable selected blocks
  → reduced UTxO checkpoint (disk-backed, EXISTING)   ── the large stake-bearing substrate
  + EpochAccumulator (small, durable, incremental)    ── the non-UTxO consensus facts
  → ONE deterministic boundary transition
  → rewards + snapshot rotation + next leadership authority
  → repeat forever
```

The **EpochAccumulator** carries ONLY the non-UTxO facts the node must evolve to compute future
authority: rewards / reward accounts, delegations, pool registrations + retirement/future-pool maps,
block-production counters, epoch fees, the reward/treasury/reserves pots, the relevant
protocol/governance state, and the mark/set/go snapshot distributions. The reduced checkpoint stays the
UTxO/stake substrate — **no permanent full UTxO map in RAM, ever.**

The load-bearing artifact is **one authoritative transition contract** (defined in S1), not a struct:

```
apply_selected_block(prior: EpochAccumulator, block_bytes, selected_ctx)
  -> next: EpochAccumulator | StructuredError
```

— total, deterministic, replay-equivalent, with the protocol boundary order (withdrawals → rewards →
snapshot rotation) and rollback/re-materialization defined up front. The epoch boundary becomes one
*scheduled consequence* of this contract, not another special-case seam.

---

## 4. The memory constraint (a HARD exit criterion, not an afterthought)

The accumulator is the missing *small* state machine beside the disk-backed checkpoint; it must NOT undo
the MEM-OPT work. Binding:

- **no permanent full UTxO map in RAM** (the reduced checkpoint remains the UTxO/stake substrate);
- **no full-chain replay during ordinary block follow** (per-block work touches only affected entries);
- **no full-accumulator clone per block** (apply a compact deterministic delta; never `state.clone()` per block);
- **no full UTxO rescan per block**;
- **bounded, measured memory under long-running follow** (RssAnon ceiling, like BA-08);
- **restart reconstructs from the persisted accumulator + checkpoint** — never a full-chain rebuild;
- **epoch-boundary peak memory is measured and capped** (the boundary transition is the only naturally
  heavier step; it must be bounded + disk-backed, not a "load everything" spike).

The accumulator's larger maps (delegations, reward accounts, pool state, snapshot distributions) are
substantial but far smaller and more structured than a full UTxO set, and are persisted incrementally.

---

## 5. Slice decomposition

> Ordering reflects dependency and safety. A slice is not complete until it meets the cluster exit
> criteria incrementally and is replay-verifiable end-to-end.

- **S1 — Authority transition contract + accumulator state + canonical persistence format.** Defines
  `apply_selected_block` (the total contract: certs/withdrawals/delegations, issuer block-production,
  epoch fees, governance/protocol-state effects, the boundary order withdrawals→rewards→snapshot
  rotation, rollback/re-materialization), the BLUE `EpochAccumulator` type, and its canonical durable
  encoding. **The contract — not the type — is the center.** Declares DC-EPOCH-19.
- **S2 — Per-block non-UTxO evolution on selected-chain admit.** Wire the within-epoch half of the
  contract onto the live follow: advance cert state + `block_production[issuer]` + `epoch_fees` per
  admitted block, persisted as a compact delta; decode the snapshot's `nesBcur` so epoch-N counts are
  complete. Strengthens DC-EPOCH-19.
- **S3 — Boundary transition: RUPD, pots, fees, block production, snapshot rotation.** Wire the existing
  byte-exact `apply_epoch_boundary_with_registrations` from the accumulator (counts/fees/reserves/go),
  in protocol order; rotate mark→set→go. Byte-exact gate vs the live cardano-node. Strengthens DC-EPOCH-19.
- **S4 — Leadership authority derivation from reduced UTxO + accumulator snapshots.** Derive the next
  authority from the maintained accumulator + the reduced checkpoint; retire the seed-anchored
  re-derive past the bootstrap-transient boundaries (the bridge + Option-B stay as the seed-2 seeds).
- **S5 — Restart and rollback recovery equivalence.** The accumulator re-derives identically from
  durable blocks after restart, in each epoch phase; a rollback re-materializes it through the same path.
- **S6 — Live multi-epoch proof: N → N+1 → N+2 → N+3**, with a restart in each phase and a controlled
  rollback case — all self-derived from S3's transition, no Mithril re-import / oracle / injection.

**Operational / release gates (separate, alongside S6 — must not obscure the semantic question):** peer
disconnect→reconnect as an outer lifecycle; keyed forge + Haskell-peer adoption after self-derived epochs.

---

## 6. Exit criteria (CE) — the cluster is NOT closed until ALL are mechanically green

- **CE-1 (contract):** `apply_selected_block` is total + deterministic + replay-equivalent; a hermetic
  test drives a multi-block + boundary sequence and asserts byte-identical accumulators on re-run.
- **CE-2 (within-epoch evolution):** block_production + epoch_fees + cert effects accumulate live; a gate
  asserts the accumulated counts == the counts derived from replaying the same durable blocks.
- **CE-3 (boundary, byte-exact):** the self-computed RUPD + the rotated go-snapshot == the live
  cardano-node's reward update + stake snapshot at ≥2 self-derived boundaries (the live differential gate).
- **CE-4 (self-sufficiency, DC-EPOCH-19):** N → N+1 → N+2 → N+3 crossed with NO Mithril re-import, no CLI
  oracle, no injected authority; every block in N+3 validates against the self-derived authority.
- **CE-5 (restart/rollback equivalence):** restart in each epoch phase + one controlled rollback
  re-derive the IDENTICAL accumulator + authority.
- **CE-6 (memory, HARD):** the §4 memory criteria — bounded RssAnon under long follow, no per-block full
  clone/rescan, restart-from-persisted, measured + capped epoch-boundary peak — all mechanically checked.

**Entry:** DC-EPOCH-18 enforced (the seed+2 stake is byte-exact, c4e0413b — done).
**Exit:** CE-1..CE-6 green in CI + the CE-4 live proof captured.
