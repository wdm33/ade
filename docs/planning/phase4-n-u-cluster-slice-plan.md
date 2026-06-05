# Cluster/Slice Plan — Ade · PHASE4-N-U (forged-block durability)

> **Status:** IDD cluster plan (Part IV). Overall ordered plan only — full cluster doc is
> `/cluster-doc PHASE4-N-U`. Produced 2026-06-05; code-grounded by two read-only
> investigations of the durable-admit/WAL/recovery + forge/self-accept/served surfaces.
> Invariants sketch: `docs/planning/phase4-n-u-forged-block-durability-invariants.md`
> (5 declared rules). Regression target: `docs/evidence/c1-genesis-rehearsal-reproduction-README.md`.

## Resolution of the open questions (from code investigation)

- **OQ-b → REUSE, no new type.** The forged `AcceptedBlock` already holds the canonical
  `[era, block]` bytes (`self_accept` stores `forged_bytes.to_vec()` verbatim). `pump_block`
  takes raw `&[u8]`; `WalEntry::AdmitBlock` stores no bytes (hash/fp only; bytes → ChainDb via
  `StoreBlockBytes`). → feed `accepted.into_bytes()`: **no re-encode, no new `WalEntry`
  variant; I-10 holds today.**
- **OQ-c → route through the pump.** Not a `prior_fp` mismatch (the seed entry is transparent
  to `verify_chain`; `ForwardSyncState.prior_fp` is anchor-seeded). Today the forge writes no
  WAL `AdmitBlock` and never `put_block`s; the README ChainBreak is the *received*-block
  re-staging hazard (`BlockBytesMissing`). Fix = route the forged block through `pump_block`.
- **OQ-d → extend-only, NO admit-time fork-choice (correction).** Verified: the durable admit
  path (`receive_apply → admit_via_block_validity → block_validity`) has no
  `select_best_chain`/`fork_choice` — those live only in `ade_core_interop::follow` +
  `ade_runtime::consensus::chain_selector`. The forge↔feed race is made safe by **fail-closed
  extend-only validation + `prior_fp` chaining**. DC-CONS-23 reframed accordingly.
- **OQ-f → no RO-LIVE flip.** Durability ≠ peer acceptance; `RO-LIVE-01` stays operator-gated.
- **OQ-g → DC-STORE-07.** Durable cadence is DC-STORE-07 (every 100 blocks,
  `should_snapshot_after_block`), not CN-SNAPSHOT-01/02 (served-chain `push_atomic`
  atomicity). Forged admits ride DC-STORE-07 via `pump_block`.

**Structural constraint (load-bearing):** the durable-admit step **cannot inline** in
`run_relay_loop` (fenced: no `pump_block(`/`put_block`/`AdvanceTip` in the loop body) nor in
`run_node_sync` (fenced: no forge tokens). It is a **new fenced RED driver fn** called from the
ForgeTick arm (with `pump_block` inside *that* fn — gate-compatible, mirroring the loop's call
to `forge_one_from_recovered`), and the containment gate's allow-list gains that one call.
**No `NodeBlockSource` variant** (the driver feeds `pump_block` directly — avoids conflating
forged with received provenance).

## Cluster Index (single cluster)

1. **PHASE4-N-U — forged-block durability** — primary invariant: **DC-NODE-12** — a
   self-accepted forged block becomes durable *only* by being submitted to the existing
   `pump_block`/`AdmitPlan::durable` chokepoint (durable-before-tip); the forge advances no
   tip directly; `pump_block` stays the sole durable tip-advance authority, now feeding both
   received and forged blocks.

---

## PHASE4-N-U — forged-block durability

- **Primary invariant (DC-NODE-12):** the forge submits its self-accepted block as an *input*
  to the single durable admit authority; the forge advances no tip directly; admission is
  extend-only, durable-before-tip, byte-identical.
- **TCB partition:**
  - **BLUE (reused, unchanged — no new authority/type):** `ade_ledger::{block_validity (incl.
    header_position), receive::admit_via_block_validity, wal (WalEntry/verify_chain),
    producer::{forge, self_accept}}`, `ade_core::consensus::{header_validate, header_summary}`.
    **N-U adds no BLUE canonical type.** `ade_core::consensus::fork_choice`/`select_best_chain`
    is **not** on the durable-admit path (stays the follow/`chain_selector` authority).
  - **GREEN (reused):** `ade_runtime::forward_sync::reducer` (`AdmitPlan::durable` ordering);
    the existing `SelfAcceptedHandoff` carrier; the clock/loop planner.
  - **RED (new wiring + changes):** `ade_node::{node_sync (new fenced durable-forge-admit
    driver fn), node_lifecycle (ForgeTick arm wiring; serve sibling)}`,
    `ade_runtime::{forward_sync::pump (reused), recovery/restart + node_lifecycle warm_start
    (S2), chaindb (reused), serve_dispatch / serve sibling (S3)}`.
- **Cluster Exit Criteria:**
  - **CE-1** [S1]: a self-accepted forged block advances the durable ChainDb tip **only**
    through the new fenced driver → `pump_block` (durable-before-tip); the next forge derives
    `(block_no, prev_hash)` from the advanced durable tip and builds N+1; no direct forge-side
    tip mutation; no second tip-advance path. (DC-NODE-12; strengthens DC-NODE-05, CN-NODE-02,
    DC-NODE-10)
  - **CE-2** [S1]: the bytes `put_block`'d for a forged block are byte-identical to the
    self-accepted + served bytes; no re-encode; no new `WalEntry` variant. (I-10)
  - **CE-3** [S1]: a stale-tip forge (a feed block advanced the tip after forge time) fails
    closed at admit (header-position/`prev_hash` or `TipBeforeDurable`), never overrides the
    durable tip; admit is extend-only; no new fork-choice. (DC-CONS-23)
  - **CE-4** [S1]: forged `AdmitBlock` WAL entries chain correctly (`prior_fp` == current
    durable `post_fp`); a silent ChainBreak is authority-fatal. (DC-WAL-04, chaining clause)
  - **CE-5** [S2]: production `warm_start_recovery` recovers a forged-block durable tip
    byte-identically — WAL-tail reconciliation drops an un-WAL'd forged orphan (torn
    forge-admit crash), and forward-replay from a sub-tip snapshot is supported; kill-then-
    recover yields the same tip + ledger fp. Recovery is proven through WAL replay (riding the
    existing DC-STORE-07 cadence), not by forcing a snapshot at every forged tip.
    (DC-WAL-04 no-orphan + T-REC-05 recovery)
  - **CE-6** [S2]: two clean forge-runs over identical inputs → byte-identical durable outputs
    (tip, WAL image, checkpoints) including forged blocks. (T-REC-05 replay)
  - **CE-7** [S3]: the served ChainView is a deterministic projection of the durable chain
    (incl. a feed-ingested predecessor); a follower fetches coherent history (A→B, never B
    without A); the G-R accumulator + `serve_gate_admits` workaround is retired. (DC-NODE-13)
- **Slices:**
  - **S1 — own-forged durable admit through the pump** — invariant: a new fenced RED driver
    feeds `accepted.into_bytes()` into `pump_block` from the ForgeTick arm (extend-only
    validate → StoreBlockBytes → AppendWal → AdvanceTip); the forge-successor reads the
    now-durable-consistent tip; a stale-tip forge fails closed; forged admits ride the existing
    DC-STORE-07 snapshot cadence (the existing snapshot policy can eventually cover forged
    admits — **no eager per-tip snapshot**; immediate restart recovery is proven through WAL
    replay in S2, not by forcing a snapshot at every forged tip) — addresses: CE-1, CE-2, CE-3,
    CE-4 — TCB: **RED** (driver + ForgeTick wiring; reuses BLUE `block_validity` + GREEN
    `AdmitPlan` + RED `pump`); updates `ci_check_node_run_loop_containment.sh` allow-list.
  - **S2 — forged-tip crash recovery + replay equivalence** — invariant: wire the
    `recover_node_state`-style WAL-tail reconciliation + forward-replay-from-sub-tip-snapshot
    into the production `warm_start_recovery`, fail-closed on
    ChainBreak/`WalTailFingerprintMismatch`/`BlockBytesMissing`; prove kill-then-recover +
    two-run byte-identical — addresses: CE-5, CE-6 — TCB: **RED** (`recovery/restart` +
    `node_lifecycle` warm_start; reuses BLUE `replay_from_anchor`/`verify_chain`). *(May split
    S2a reconciliation / S2b forward-replay at `/cluster-doc` if forward-replay is large — both
    independently mergeable.)*
  - **S3 — serve-as-durable-chain projection** — invariant: replace the `ServedChainSnapshot`
    accumulator + `serve_gate_admits` with a serve projection over the durable ChainDb
    (`tip`/`get_block_by_*`/`iter_from_slot` — already exposed; no new ChainDb surface); a
    follower fetches coherent history incl. ingested predecessors; retire the G-R workaround —
    addresses: CE-7 — TCB: **RED** (serve sibling / `serve_dispatch`; reuses BLUE served bytes).
- **Replay obligations:** N-U brings **forged blocks into the existing durable ChainDb + WAL**
  (same stores as received) — **no new canonical type** (reuse `AcceptedBlock` bytes +
  `WalEntry::AdmitBlock`). New replay corpus: a forged-admit → kill → recover transcript + a
  two-clean-run byte-identical forge transcript (T-REC-05). The durable-before-tip ordering
  (DC-SYNC-01) + WAL fingerprint chain (T-REC-01/02) + the DC-STORE-07 snapshot cadence extend
  to forged admits unchanged. Replay command: `cargo test -p ade_testkit` (replay suites) +
  the `ade_node` forge-recovery tests.

## Mergeability / complete-work-only notes

- **S1** leaves a fully-correct *running* node: the forge builds a growing durable chain;
  forged blocks become first-class equal to received blocks (same durability, same recovery
  semantics — the pre-existing forward-replay limitation applies equally and is *not newly
  introduced*). S1 makes no restart-recovery claim (that is S2). It keeps the existing serve
  path (G-R accumulator) intact until S3.
- **S2** closes the recovery gap for forged tips (and, as a side effect, the pre-existing
  forward-replay limitation for received tips); after S2, kill-then-recover is byte-identical.
- **S3** unifies serve onto the durable chain and retires the redundant accumulator + G-R gate.
- Each slice independently leaves the system in a correct state; none temporarily weakens an
  invariant.

## Registry consequences (applied)

- **DC-CONS-23 reframed** (declared) from "admitted via `select_best_chain`" to the
  **extend-only / fail-closed** semantics (OQ-d). DC-NODE-12's admit-chain phrase corrected
  (`block_validity, extend-only`, not `→ fork-choice`).
- **Strengthenings to record at close** (`strengthened_in += "PHASE4-N-U"`): `DC-NODE-05` +
  `CN-NODE-02` (containment clauses — "forged block is a local artifact only" / "no forge
  tip-advance path" — superseded by DC-NODE-12 via cross-ref), `DC-SYNC-01`, `DC-SYNC-02`,
  `DC-NODE-10`, `DC-CONS-03`, `T-REC-01/02/03`, `DC-STORE-07`.
- **Slice → rule flip:** S1 flips DC-NODE-12 + DC-CONS-23 (+ DC-WAL-04 chaining clause) to
  `enforced`; S2 flips T-REC-05 (+ DC-WAL-04 no-orphan clause); S3 flips DC-NODE-13.

## Next steps

1. `/cluster-doc PHASE4-N-U` — expand the full cluster doc (TCB color map, per-slice CE,
   replay obligations) from this plan.
2. `/slice-doc PHASE4-N-U S1`, then implement S1 (the durable admit) first — it is the
   foundation S2 and S3 depend on.
3. Re-run the C1 genesis-rehearsal reproduction after S1 as the regression check (must not
   break block-0 acceptance; should now show a growing chain past block 0).
