# PHASE4-N-U (OQ-R1) — Forged-block durability — invariant sketch

> **Status:** IDD invariant sketch (Part I), updated at `/cluster-plan` with code-grounded
> resolutions. The ordered slice plan lives in
> `docs/planning/phase4-n-u-cluster-slice-plan.md`. Produced 2026-06-05; grounded in the
> four grounding docs + the invariant registry + the OQ-R1 scope
> (`docs/clusters/PHASE4-N-F-G-R/cluster.md §12`) + the regression target
> (`docs/evidence/c1-genesis-rehearsal-reproduction-README.md`).

## Concept

A self-accepted **own-forged** block becomes a valid **input** to the **same durable
admit chokepoint** that *received* blocks already use
(`run_node_sync → pump_block` / `forward_sync AdmitPlan::durable`). The node **submits its
self-accepted forged block to the durable admit chokepoint, which may advance the ChainDb
tip after extend-only validation and WAL durability** — so the next forge builds block
N+1 (a real growing chain), and the result is recovery-correct (WAL replay, no
`ChainBreak`) and replay-equivalent.

**Central framing (load-bearing):** N-U is **not** "forge mutates the tip." N-U is
"**a self-accepted forged block becomes an input to the existing durable admit
authority**" — the same authority that admits received blocks. This keeps the
color/authority model clean: the forge produces a candidate; `pump_block` /
`AdmitPlan::durable` remains the **sole** durable tip-advance authority; admission stays
validation- + WAL-governed and **extend-only** (the durable admit path runs no
fork-choice — that authority lives in the follow / `chain_selector` paths, not the pump).

**Pure-transformation check (passes):** the authoritative step is
`(durable chain state, self-accepted forged block + its canonical bytes) → (new durable
chain state, ordered durable effects)` — byte-for-byte the same transition shape as a
received-block admit. The only nondeterminism (wall-clock slot, VRF) already enters as
**canonical input** via the clock seam (→ `SlotNo`) and BLUE leadership
(`forge_one_from_recovered`). N-U introduces **no new nondeterminism** in an authoritative
path.

**Mithril-first consistency:** N-U is **post-recovery durable progression**, not a new
bootstrap authority. It routes own-forged blocks through the existing durable
admit / WAL / recovery authority; it does not add a second forge-specific tip path and
does not touch the Mithril/genesis bootstrap composition.

---

## Resolved decisions

- **D-1 (OQ-a — DC-NODE-05 evolution) → supersede-via-new-rule + cross-ref. No
  deprecation.** DC-NODE-05's **deeper invariant is preserved**: *the forge tick advances
  no durable tip **directly***; `pump_block` remains the **sole** durable tip-advance
  authority. What changes is the N-F-E-era **containment consequence** *"a forged block is
  a local self-accept artifact only"* — superseded because N-U **promotes forged blocks
  into the normal durable authority path**. Recorded as: DC-NODE-05 preserved (no direct
  forge-side tip mutation) + **DC-NODE-12** introduced, with a mutual `cross_ref`. (The same
  containment-supersession applies to **CN-NODE-02**'s "no forge tip-advance path" clause.)
- **D-2 (OQ-e — served-view projection) → in N-U scope, as its own (later) slice.** Once
  the node admits own-forged blocks durably, serving **must** become a projection of the
  durable chain, not an independent accumulator — otherwise we preserve the exact
  PHASE4-N-F-G-R class of defect in a new form (durable chain says A → B; served view
  serves only B; follower cannot fetch coherent history). Kept in scope as **DC-NODE-13**,
  sequenced **after** the durable admit path is proven.
- **D-3 (OQ-d — admit semantics) → extend-only, no admit-time fork-choice.** Code
  investigation confirmed the durable admit path
  (`receive_apply → admit_via_block_validity → block_validity`) is **extend-only**;
  `select_best_chain`/`fork_choice` are called **only** in `ade_core_interop::follow` and
  `ade_runtime::consensus::chain_selector`, never in `receive/` or `forward_sync/`. So the
  forge↔feed race is made safe by **fail-closed extend-only validation + `prior_fp`
  chaining**, NOT by an admit-time fork-choice. DC-CONS-23 reframed accordingly.

---

## 1. What must always be true

- **I-1 — One durable tip authority, now covering forged blocks** *(strengthens
  CN-NODE-02 / DC-SYNC-02).* The durable ChainDb tip advances **only** through
  `pump_block` / `AdmitPlan::durable` — for **both** received and forged blocks. The forge
  gets no second tip-advance path.
- **I-2 — Durable-before-tip for forged blocks** *(extends DC-SYNC-01).* A forged block's
  preserved bytes + WAL entry are durable (`StoreBlockBytes` + `AppendWal` acked) **before**
  the tip advances; the pump fail-closes `TipBeforeDurable` on any out-of-order apply —
  forged exactly as received.
- **I-3 — No unvalidated own-bytes at the tip** *(preserves `self_accept` + the BLUE admit
  chokepoint, CN-FORGE-01).* A forged block reaches the durable tip only after the **same**
  extend-only BLUE chokepoint (decode → `validate_and_apply_header` → `block_validity`)
  that received blocks pass. `self_accept` is not bypassed.
- **I-4 — WAL chain integrity across seed + forged + received** *(strengthens T-REC /
  `verify_chain`).* Every WAL entry's `prior_fp == previous post_fp` (anchor's
  `initial_ledger_fingerprint` for the first). A forged block-0 over a seed-only WAL must
  chain to the seed anchor's fingerprint; block N+1 to block N's `post_fp`. **No
  ChainBreak.**
- **I-5 — Own-tip admission is extend-only; a stale-tip forge fails closed** *(preserves
  DC-CONS-03 as the separate fork-choice authority).* The durable admit path is
  **extend-only** — it runs no admit-time `select_best_chain`/fork-choice. A forged block is
  admitted only if it **extends the current durable tip**; if a feed block advanced the tip
  after forge time, the stale-tip forge **fails closed** (header-position/`prev_hash`,
  `TipBeforeDurable`, or WAL `prior_fp` mismatch) and the next tick re-forges on the current
  durable tip. No own-block override.
- **I-6 — The forged tip recovers** *(extends T-REC-01/02/03).* Warm-start recovery (anchor
  + WAL replay) recovers the durable tip including forged blocks, byte-identically;
  recovery reconciles durable block storage and the WAL tail so no un-WAL'd forged orphan
  survives a torn crash.
- **I-7 — Forge-successor derives from the *durable* tip** *(strengthens DC-NODE-10).* The
  next forge's `(block_no, prev_hash)` come from the **durable adopted tip** (`tip.block_no
  + 1`, `Block(tip.hash)`), not a stale recovered base or an in-memory-only spine. (Note:
  `ChainTip` carries no `block_no`; once DC-NODE-12 admits forged blocks durably, the
  evolved `state.receive` spine and the durable ChainDb advance together via `pump_block`,
  so the existing evolved-spine read becomes durable-consistent.)
- **I-8 — Forge-slot discipline preserved** *(DC-NODE-05 permanent clauses, UNCHANGED).*
  ≤1 forge per `SlotNo`; never `slot ≤ last_forged`; slot via the clock seam only;
  leadership in BLUE, not the loop/planner.
- **I-9 — Served view is a projection of the durable chain** *(OQ-R2 / DC-NODE-13;
  supersedes the G-R monotone-serve workaround).* What is served to followers is a
  deterministic projection of the durable adopted chain (**incl. a feed-ingested
  predecessor**), not an independent accumulator — so a follower at N-1 can fetch N.
- **I-10 — Same canonical bytes from `self_accept` → served → durable admit** *(byte-identity
  binding).* The bytes admitted durably (`StoreBlockBytes` + WAL) for an own-forged block are
  **exactly** the bytes `self_accept` validated and the served view serves; **no re-encoding,
  reserialization, or reconstruction** occurs between `self_accept` and durable admit.
  (Resolved: the forged `AcceptedBlock` already holds the canonical `[era, block]` bytes —
  feed `accepted.into_bytes()` into `pump_block` directly.)

## 2. What must never be possible

- A forged block at the durable tip **without** `StoreBlockBytes` + `AppendWal` acked first
  (tip-before-durable).
- A forged block at the tip **without** passing `self_accept` / the BLUE chokepoint
  (unvalidated own-bytes).
- A **second** durable tip-advance path (a forge-specific `put_block` / `AdvanceTip` /
  `rollback_to_slot` bypassing the pump).
- A silent **ChainBreak**: a forged WAL entry whose `prior_fp` ≠ current durable `post_fp`
  admitted (must fail closed; authority-fatal).
- An **un-WAL'd forged orphan** surviving recovery (a torn forge-admit crash leaving
  chaindb tip ahead of WAL).
- A forged block **overriding** the current tip when it does not extend it — a stale-tip
  forge MUST fail closed via extend-only validation / `prior_fp` (N-U adds no admit-time
  fork-choice and no own-block override path).
- **Re-minting** a new block at a height already durably admitted (the forge builds N+1
  from the durable tip, never another N) — this is exactly today's churn, eliminated.
- **Re-encoding / reserializing** a forged block between `self_accept` and durable admit
  (the durably-WAL'd + served bytes must be byte-identical to the self-accepted bytes — I-10).
- **Nondeterministic** forged bytes or admission order across replays.
- The served view **diverging** from the durable chain (serving a non-durable block, or
  omitting a durable predecessor a follower needs).

## 3. What must remain identical across executions (deterministic surface)

- Forged block bytes for fixed (durable tip, slot, leadership inputs, keys) — now a
  function of the **durable** tip, not an in-memory spine.
- The extend-only admit verdict (admit-if-extends / fail-closed) for fixed (current durable
  tip, forged candidate).
- The WAL entry `(prior_fp, post_fp)` + the stored bytes for an admitted forged block.
- The durable tip `(slot, hash, block_no)` + ledger fingerprint after a forged admit.
- The served-chain projection for a fixed durable chain.

## 4. What must be replay-equivalent

- **Loop-as-replay incl. forge-admit** *(extends T-REC-03):* same recovered state + same
  ordered feed + same clock-tick schedule + same leadership/key inputs + same shutdown →
  byte-identical durable outputs (tip, WAL image, checkpoints, halt), now **including
  forged-then-admitted** blocks.
- **Warm-start recovery incl. forged blocks** *(extends T-REC-01/02):* same anchor + same
  WAL (with forged `AdmitBlock` entries) → byte-identical recovered tip + ledger
  fingerprint.
- **The WAL is the replay log of record for forged blocks** — replaying it reproduces the
  exact durable chain.

## 5. State transitions in scope

```
T-A  Admit a self-accepted forged block to the durable chain   (the heart of N-U)
     (DurableChainState{tip, ledger_fp, wal_tail}, ForgedAdmit{accepted_block, canonical_bytes})
       -> Result<(DurableChainState{tip', ledger_fp', wal_tail'},
                  effects=[StoreBlockBytes, AppendWal{prior_fp, post_fp}, AdvanceTip]),  // ORDERED
                 AdmitError{ HeaderPositionInvalid | PrevHashMismatch | TipBeforeDurable
                             | ChainBreak | ValidationFailed }>
     // SAME shape as the received-block admit; N-U makes the forged block a valid INPUT to it.
     // EXTEND-ONLY: a stale-tip forge fails closed (no admit-time fork-choice).
     // canonical_bytes are the exact self_accept'd bytes (I-10): no re-encode before WAL/serve.

T-B  Forge-successor position from the DURABLE tip
     (DurableChainState{tip}, ForgeTick{slot})
       -> Result<ForgePosition{ block_no = tip.block_no + 1, prev_hash = Block(tip.hash) }, ForgeError>
     // cold-start: tip = None -> (0, PrevHash::Genesis)

T-C  Warm-start recovery including forged blocks
     (BootstrapAnchor, Wal{entries incl forged AdmitBlock})
       -> Result<RecoveredState{durable_tip, ledger_fp}, RecoveryError{ WalTailFingerprintMismatch | ChainBreak | BlockBytesMissing }>

T-D  Extend-only durable admit — own forged candidate vs current durable tip
     (current_durable_tip, forged_candidate)
       -> Result<admitted, AdmitReject{ HeaderPositionInvalid | PrevHashMismatch | TipBeforeDurable | ChainBreak }>
     // extend-only; a stale-tip forge fails closed. NO admit-time fork-choice
     // (DC-CONS-03 select_best_chain is the separate follow/chain_selector authority).

T-E  (OQ-R2 / DC-NODE-13) Served-chain projection from the durable chain
     (DurableChain, serve_request) -> ServedChainView   // deterministic projection, not an accumulator
```

## 6. TCB color hypothesis

- **BLUE (authoritative, reused — no new type):** the extend-only admit decision (decode →
  `validate_and_apply_header` → `block_validity`, incl. `block_validity::header_position`)
  for forged blocks — the SAME authority as received (`ade_core::consensus::{header_validate,
  header_summary}`, `ade_ledger::{block_validity, receive::admit_via_block_validity}`); WAL
  semantics (`WalEntry::AdmitBlock`, `prior_fp/post_fp`, `verify_chain` — `ade_ledger::wal`);
  `self_accept` (`ade_ledger::producer`). **`ade_core::consensus::fork_choice` /
  `select_best_chain` is NOT on the durable-admit path** — it stays the follow / `chain_selector`
  authority (DC-CONS-03), untouched by N-U.
- **GREEN (deterministic glue, reused):** the `forward_sync::reducer` `AdmitPlan::durable`
  (durable-before-tip ordering); a constructor-fenced carrier from `self_accept` (the existing
  `SelfAcceptedHandoff`); the served-chain **projection** (I-9).
- **RED (shell — the new wiring):** a new fenced durable-forge-admit driver fn feeding
  `pump_block` from the ForgeTick arm; `pump_block` / `run_node_sync` (reused); `chaindb` /
  `wal` writes; `recovery/restart` + `node_lifecycle` warm_start (recovery slice); the serve
  sibling / `serve_dispatch` (projection slice).
- **Resolved color decision:** the forged block is admitted via a **new fenced RED driver
  fn** (called from the ForgeTick arm, with `pump_block` inside that fn — gate-compatible),
  **not** via a `NodeBlockSource` variant (which would conflate forged with received
  provenance). The admit chokepoint stays BLUE.

## 7. Open questions — RESOLVED at /cluster-plan (code-grounded)

- **OQ-b — RESOLVED (reuse, no new type).** The forged `AcceptedBlock` already holds the
  canonical `[era, block]` bytes (`self_accept` stores `forged_bytes.to_vec()` verbatim);
  `pump_block` takes raw `&[u8]` and `WalEntry::AdmitBlock` stores no bytes (only hash/fp;
  bytes → ChainDb). → feed `accepted.into_bytes()` directly: no re-encode, no new `WalEntry`
  variant; I-10 holds today.
- **OQ-c — RESOLVED.** Not a `prior_fp` mismatch — the seed entry is transparent to
  `verify_chain` and `ForwardSyncState.prior_fp` is anchor-seeded. Today the forge writes no
  WAL `AdmitBlock` and never `put_block`s (DC-NODE-05 → served-view `push_atomic` only), so a
  forged block is non-durable; the README ChainBreak is the *received*-block re-staging
  hazard (`BlockBytesMissing`). Fix = route the forged block through `pump_block` so its
  bytes are `put_block`'d and its `AdmitBlock` (`prior_fp` = current durable `post_fp`) is
  appended durable-before-tip.
- **OQ-d — RESOLVED (correction).** The durable admit path is **extend-only — it calls no
  fork-choice** (verified). The forge↔feed race is handled by fail-closed extend-only
  validation + `prior_fp` chaining (a stale-tip forge is rejected), not by an admit-time
  fork-choice. DC-CONS-23 reframed accordingly. In N-U scope (S1).
- **OQ-f — RESOLVED.** No RO-LIVE flip — durability ≠ peer acceptance; `RO-LIVE-01` stays
  operator-gated.
- **OQ-g — RESOLVED.** Forged admits ride the existing durable cadence **DC-STORE-07**
  (every 100 blocks, `should_snapshot_after_block`) via `pump_block`; immediate restart
  recovery is proven through WAL replay, **not** by forcing a snapshot at every forged tip.
  (Not CN-SNAPSHOT-01/02, which are served-chain `push_atomic` atomicity.)

## 8. Registry entries (appended — declared)

The five N-U rules are appended to `docs/ade-invariant-registry.toml` (`status =
"declared"`, `introduced_in = "PHASE4-N-U"`; `tests`/`ci_script` populated as slices
enforce):

| ID | tier | invariant (one-line) |
|---|---|---|
| **DC-NODE-12** | derived | A self-accepted forged block may become durable **only** through the same `pump_block`/`AdmitPlan::durable` chokepoint as received blocks (durable-before-tip, behind the BLUE extend-only admit authority); the forge has no second tip-advance path and performs no direct tip mutation. Carries the I-10 byte-identity clause. **Supersedes** DC-NODE-05's "forged block is a local self-accept artifact only" while preserving DC-NODE-05's deeper invariant. |
| **DC-WAL-04** | derived | A forged `AdmitBlock` WAL entry's `prior_fp` must equal the current durable `post_fp` (anchor `initial_ledger_fingerprint` for genesis-successor block 0); a forged block that would ChainBreak fails closed (authority-fatal); the WAL binds the exact canonical self-accepted bytes; no un-WAL'd forged orphan survives recovery. |
| **T-REC-05** | true | Same anchor + WAL (incl. forged admits) → byte-identical recovered tip + ledger fp; same recovered state + feed + clock/leadership/shutdown → byte-identical durable outputs **including** forged-then-admitted blocks. Extends T-REC-01/02/03; rides snapshot + forward-replay (no new durability law). |
| **DC-CONS-23** | derived | Own-forged stale-tip race safety by **extend-only durable admit**: a forged candidate is admitted only if it extends the current durable tip; a stale-tip forge fails closed (header-position/`prev_hash`, `TipBeforeDurable`, or WAL `prior_fp`). N-U adds **no admit-time fork-choice** and no own-block override; DC-CONS-03 stays the fork-choice authority in the follow / `chain_selector` paths. |
| **DC-NODE-13** *(OQ-R2)* | derived | The served `ChainView` is a deterministic **projection** of the durable adopted chain (incl. ingested predecessors), not an independent accumulator; supersedes the G-R monotone serve-gate workaround. Sequenced as a later N-U slice, after the durable admit path is proven. |

**Strengthenings** (no new ID; `strengthened_in += "PHASE4-N-U"` recorded at cluster
close): `DC-NODE-05` + `CN-NODE-02` (containment clauses superseded by DC-NODE-12),
`DC-SYNC-01`, `DC-SYNC-02`, `DC-NODE-10`, `DC-CONS-03`, `T-REC-01`, `T-REC-02`, `T-REC-03`,
`DC-STORE-07`.

---

## Next steps

1. The ordered slice plan is in `docs/planning/phase4-n-u-cluster-slice-plan.md` (S1 durable
   admit, S2 recovery/replay, S3 served-view projection).
2. `/cluster-doc PHASE4-N-U` — expand the cluster doc; flip the declared rules to enforced as
   slices land.
3. The C1 genesis-rehearsal reproduction runbook is the **regression target**: N-U must not
   break block-0 acceptance.
