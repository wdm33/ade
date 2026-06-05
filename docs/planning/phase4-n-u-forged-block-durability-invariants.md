# PHASE4-N-U (OQ-R1) — Forged-block durability — invariant sketch

> **Status:** IDD invariant sketch (Part I). Pre-`/cluster-plan`. No implementation, no
> cluster/slice decomposition here. Produced 2026-06-05; grounded in the four grounding
> docs + the invariant registry (328 rules @ `65954fa3`) + the OQ-R1 scope
> (`docs/clusters/PHASE4-N-F-G-R/cluster.md §12`) + the regression target
> (`docs/evidence/c1-genesis-rehearsal-reproduction-README.md`).

## Concept

A self-accepted **own-forged** block becomes a valid **input** to the **same durable
admit chokepoint** that *received* blocks already use
(`run_node_sync → pump_block` / `forward_sync AdmitPlan::durable`). The node **submits its
self-accepted forged block to the durable admit chokepoint, which may advance the ChainDb
tip after validation, WAL durability, and fork-choice** — so the next forge builds block
N+1 (a real growing chain), and the result is recovery-correct (WAL replay, no
`ChainBreak`) and replay-equivalent.

**Central framing (load-bearing):** N-U is **not** "forge mutates the tip." N-U is
"**a self-accepted forged block becomes an input to the existing durable admit
authority**" — the same authority that admits received blocks. This keeps the
color/authority model clean: the forge produces a candidate; `pump_block` /
`AdmitPlan::durable` remains the **sole** durable tip-advance authority; admission stays
validation- + WAL- + fork-choice-governed.

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

## Resolved decisions (from the sketch review)

- **D-1 (OQ-a — DC-NODE-05 evolution) → supersede-via-new-rule + cross-ref. No
  deprecation.** DC-NODE-05's **deeper invariant is preserved**: *the forge tick advances
  no durable tip **directly***; `pump_block` remains the **sole** durable tip-advance
  authority. What changes is the N-F-E-era **containment consequence** *"a forged block is
  a local self-accept artifact only"* — appropriate before the durable admit path was
  ready, now superseded because N-U **promotes forged blocks into the normal durable
  authority path**. Recorded as: DC-NODE-05 preserved (no direct forge-side tip mutation)
  + **DC-NODE-12** introduced (self-accepted forged blocks may become durable **only**
  through `pump_block` / `AdmitPlan::durable`), with a mutual `cross_ref`. This is a
  **strengthening** of the architecture, not a weakening of the true invariant.
- **D-2 (OQ-e — served-view projection) → in N-U scope, as its own (later) slice.** Once
  the node admits own-forged blocks durably, serving **must** become a projection of the
  durable chain, not an independent accumulator — otherwise we preserve the exact
  PHASE4-N-F-G-R class of defect in a new form (durable chain says A → B; served view
  serves only B; follower cannot fetch coherent history). Kept in scope as **DC-NODE-13**,
  sequenced **after** the durable admit path is proven.

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
  BLUE chokepoint (decode → `validate_and_apply_header` → `block_validity` → fork-choice)
  that received blocks pass. `self_accept` is not bypassed.
- **I-4 — WAL chain integrity across seed + forged + received** *(strengthens T-REC /
  `verify_chain`).* Every WAL entry's `prior_fp == previous post_fp` (anchor's
  `initial_ledger_fingerprint` for the first). A forged block-0 over a seed-only WAL must
  chain to the seed anchor's fingerprint; block N+1 to block N's `post_fp`. **No
  ChainBreak.**
- **I-5 — Own-tip admission is a fork-choice decision** *(preserves DC-CONS-03).* Admitting
  a forged block is `select_best_chain` (block-no then tiebreaker) against the **current**
  durable tip; if the forged block extends the tip it wins by block-no, otherwise
  fork-choice decides. No "own block always wins."
- **I-6 — The forged tip recovers** *(extends T-REC-01/02/03).* Warm-start recovery (anchor
  + WAL replay) recovers the durable tip including forged blocks, byte-identically;
  recovery reconciles durable block storage and the WAL tail so no un-WAL'd forged orphan survives a torn crash.
- **I-7 — Forge-successor derives from the *durable* tip** *(strengthens DC-NODE-10).* The
  next forge's `(block_no, prev_hash)` come from the **durable adopted tip** (`tip.block_no
  + 1`, `Block(tip.hash)`), not a stale recovered base or an in-memory-only spine.
- **I-8 — Forge-slot discipline preserved** *(DC-NODE-05 permanent clauses, UNCHANGED).*
  ≤1 forge per `SlotNo`; never `slot ≤ last_forged`; slot via the clock seam only;
  leadership in BLUE, not the loop/planner.
- **I-9 — Served view is a projection of the durable chain** *(OQ-R2 / DC-NODE-13;
  supersedes the G-R monotone-serve workaround).* What is served to followers is a
  deterministic projection of the durable adopted chain (**incl. a feed-ingested
  predecessor**), not an independent accumulator — so a follower at N-1 can fetch N.
- **I-10 — Same canonical bytes from `self_accept` → served → durable admit** *(new —
  byte-identity binding).* The bytes admitted durably (`StoreBlockBytes` + WAL) for an
  own-forged block are **exactly** the bytes `self_accept` validated and the served view
  serves; **no re-encoding, reserialization, or reconstruction** occurs between
  `self_accept` and durable admit. (Forged blocks are in-memory first; the WAL and served
  view must bind to the same canonical bytes.)

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
- A forged block **unconditionally overriding** the current tip when it does not extend it
  (must go through fork-choice).
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
- The fork-choice verdict for fixed (current durable tip, forged candidate).
- The WAL entry `(prior_fp, post_fp, preserved bytes)` for an admitted forged block.
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
                 AdmitError{ TipBeforeDurable | ChainBreak | ForkChoiceReject | ValidationFailed }>
     // SAME shape as the received-block admit; N-U makes the forged block a valid INPUT to it.
     // canonical_bytes are the exact self_accept'd bytes (I-10): no re-encode before WAL/serve.

T-B  Forge-successor position from the DURABLE tip
     (DurableChainState{tip}, ForgeTick{slot})
       -> Result<ForgePosition{ block_no = tip.block_no + 1, prev_hash = Block(tip.hash) }, ForgeError>
     // cold-start: tip = None -> (0, PrevHash::Genesis)

T-C  Warm-start recovery including forged blocks
     (BootstrapAnchor, Wal{entries incl forged AdmitBlock})
       -> Result<RecoveredState{durable_tip, ledger_fp}, RecoveryError{ WalTailFingerprintMismatch | ChainBreak }>

T-D  Chain selection — own forged candidate vs current durable tip   (already BLUE; N-U feeds it)
     (current_tip, forged_candidate) -> Result<selected_chain, ChainSelectionReject>
     // evaluated against the CURRENT durable tip at admit time (forge<->feed race safe)

T-E  (OQ-R2 / DC-NODE-13) Served-chain projection from the durable chain
     (DurableChain, serve_request) -> ServedChainView   // deterministic projection, not an accumulator
```

## 6. TCB color hypothesis

- **BLUE (authoritative):** the admit decision (decode → validate → `block_validity` →
  fork-choice) for forged blocks — the SAME authority as received
  (`ade_core::consensus::{fork_choice, select_best_chain, header_validate}`,
  `ade_ledger::{block_validity, receive}`); WAL semantics (`WalEntry::AdmitBlock`,
  `prior_fp/post_fp`, `verify_chain` — `ade_ledger::wal`); `self_accept`; chain selection
  (DC-CONS-03).
- **GREEN (deterministic glue):** the `forward_sync::AdmitPlan` reducer (durable-before-tip
  ordering) extended to accept a forged-block admit input; a constructor-fenced carrier
  from `self_accept` → the admit chokepoint (analogous to `SelfAcceptedHandoff`, now feeding
  `pump_block`); the served-chain **projection** (I-9).
- **RED (shell):** `pump_block` / `run_node_sync` driver (the durable I/O —
  StoreBlockBytes / AppendWal / AdvanceTip); `chaindb` / `wal` / snapshot writes;
  `node_lifecycle` / `node_sync` wiring routing the forged block into the pump;
  `recovery::restart`.
- **Open color:** is the forged block fed in as a **`NodeBlockSource` variant** (closed
  2-variant `{WirePump, InMemory}` → a closed-enum change, but maximal path-reuse) or as a
  **sibling admit input** to `forward_sync`? The *source adapter* is RED/GREEN; the *admit
  chokepoint* stays BLUE either way. → decide at `/cluster-plan`.

## 7. Open questions (for `/cluster-plan`)

- **OQ-b (forged-block preserved bytes / WAL provenance).** Received blocks carry peer
  **wire bytes**; a forged block is in-memory. Is the forge output already byte-canonical so
  `StoreBlockBytes`/WAL/`verify_chain` work unchanged (satisfying I-10 directly), or does
  N-U need a forged provenance variant in `WalEntry` (like the tag-3
  `SeedEpochConsensusInputsImported`)? → needs the forge-output shape.
- **OQ-c (ChainBreak root cause).** Is today's ChainBreak from (i) the forged block-0 WAL
  `prior_fp` ≠ seed anchor `initial_ledger_fingerprint`, (ii) a missing `StoreBlockBytes`
  for the forged block, or (iii) the seed-only WAL (`SeedEpochConsensusInputsImported`, no
  block 0) anchoring contract? → confirm against `verify_chain`.
- **OQ-d (forge↔feed race).** Confirmed direction: the forge-admit re-runs fork-choice
  against the **current** durable tip at admit time (DC-CONS-23). Confirm this is fully in
  N-U scope vs. a follow-on hardening.
- **OQ-f (RO-LIVE / bounty).** Expectation: **no RO-LIVE flip** — durability ≠ peer
  acceptance; `RO-LIVE-01` stays operator-gated. Confirm at plan time.
- **OQ-g (snapshot cadence).** Does forged-block admission ride the existing received-block
  snapshot cadence (CN-SNAPSHOT-01/02)? Expectation: yes (same durable chain). Confirm.

## 8. Candidate registry entries (proposed — append on per-entry approval)

**New rules** (`introduced_in = "PHASE4-N-U"`, `status = "declared"`,
`tests = []` / `ci_script = ""` until slices enforce):

| ID | tier | invariant (one-line) |
|---|---|---|
| **DC-NODE-12** | derived | A self-accepted forged block may become durable **only** through the same `pump_block`/`AdmitPlan::durable` chokepoint as received blocks (durable-before-tip, behind the BLUE admit authority); the forge has no second tip-advance path and performs no direct tip mutation. Carries the I-10 byte-identity clause. **Supersedes** DC-NODE-05's "forged block is a local self-accept artifact only" while preserving DC-NODE-05's deeper invariant. |
| **DC-WAL-04** | derived | A forged `AdmitBlock` WAL entry's `prior_fp` must equal the current durable `post_fp` (anchor `initial_ledger_fingerprint` for genesis-successor block 0); a forged block that would ChainBreak fails closed (authority-fatal); the WAL binds the exact canonical self-accepted bytes; no un-WAL'd forged orphan survives recovery. |
| **T-REC-05** | true | Same anchor + WAL (incl. forged admits) → byte-identical recovered tip + ledger fp; same recovered state + feed + clock/leadership/shutdown → byte-identical durable outputs **including** forged-then-admitted blocks. Extends T-REC-01/02/03; rides snapshot + forward-replay (no new durability law). |
| **DC-CONS-23** | derived | An own-forged candidate is admitted to the durable tip **only** via `select_best_chain` (DC-CONS-03: block-no then tiebreaker) against the **current** durable tip at admit time; it never unconditionally overrides the tip (forge↔feed race safe). |
| **DC-NODE-13** *(OQ-R2)* | derived | The served `ChainView` is a deterministic **projection** of the durable adopted chain (incl. ingested predecessors), not an independent accumulator; supersedes the G-R monotone serve-gate workaround. Sequenced as a later N-U slice, after the durable admit path is proven. |

**Strengthenings** (no new ID; `strengthened_in += "PHASE4-N-U"` recorded at cluster
close): `DC-NODE-05`, `DC-SYNC-01`, `DC-SYNC-02`, `CN-NODE-02`, `DC-NODE-10`,
`DC-CONS-03`, `T-REC-01`, `T-REC-02`, `T-REC-03`.

---

## Next steps

1. Append the five declared rules to `docs/ade-invariant-registry.toml` (328 → 333) on
   per-entry approval.
2. `/cluster-plan PHASE4-N-U` — order the slices around the invariant authority clusters
   (durable admit chokepoint → WAL/recovery → fork-choice/race → served-view projection),
   resolving OQ-b…OQ-g.
3. The C1 genesis-rehearsal reproduction runbook is the **regression target**: N-U must not
   break block-0 acceptance.
