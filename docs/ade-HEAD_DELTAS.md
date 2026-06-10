# Ade — HEAD Deltas (Changes Since Baseline)

> **Status:** Living architectural document. Regenerated; not hand-edited.
>
> Regenerate with `/head-deltas <baseline>` after every cluster close. Baseline is recorded in `.idd-config.json` `head_deltas_baseline`.

> Baseline: `b1bed361` (PHASE4-N-AJ AJ-S3 — convergence runbook correction + DC-EVIDENCE-03 transcript-shape rule, 2026-06-10 09:35)
> HEAD: `b4c0983d` (PHASE4-N-AK AK-S2 — recovered-anchor rollback no-op completes live follow, DC-NODE-32, 2026-06-10 17:34)
> Span: **the PHASE4-N-AK cluster — a post-N-AH/N-AI/N-AJ live recover→follow regression remediation: persist the bootstrap anchor POINT as durable recovery provenance and resolve the live-follow FindIntersect START from it (so a bare-anchor recovery starts at the anchor, not Origin — `DC-NODE-31`), then accept the peer's post-intersection `RollBackward(anchor)` as an idempotent boundary no-op so the single-producer follow loop catches up to the relay tip (`DC-NODE-32`)** — preceded by the **N-AJ close commit** (`bbdc3585`, the prior-window grounding/registry close + baseline bump that this regen measures forward from).
> **7 commits** (no merges), **33 files changed, +2647 / −544 lines**. **This span DOES touch BLUE — +2 canonical types**: one NEW BLUE module `crates/ade_ledger/src/recovered_anchor_point.rs` (+318) ships `RecoveredAnchorPoint` (the closed, version-gated, byte-canonical anchor-point record) + its closed error sum `RecoveredAnchorPointError` + the sole canonical CBOR codec — the BLUE `pub struct`/`pub enum` count over the `core_paths` trees moves **456 → 458** (`git diff b1bed361..HEAD` over the BLUE trees adds exactly those two `^+(pub )?(struct|enum)` lines and no removal). No `Cargo.toml` changed (`git diff b1bed361..HEAD -- '**/Cargo.toml'` empty — still **11 crates**); **CI gates unchanged at 159** (`--diff-filter=A/M/D` over `ci/` all empty — no new/modified/removed gate); **registry 356 → 358** (+2 rules `DC-NODE-31` + `DC-NODE-32`, both enforced; `T-REC-05` strengthened `+= PHASE4-N-AK`; zero removals). The rest of the change is the RED `ade_runtime` recovery/bootstrap/storage shell (`bootstrap.rs`, `recovered_anchor.rs` NEW, `chaindb/{mod,in_memory,persistent}.rs`, `seed_epoch_lineage.rs`, `forward_sync/reducer.rs`) and the `ade_node` shell (`node_lifecycle.rs`, `node_sync.rs`, `node.rs`), plus mechanical `recovered_anchor: None` struct-init touches at every existing construction site.

> **Baseline note (load-bearing — read before §0).** This window's baseline is **`b1bed361`**, the
> PHASE4-N-AJ AJ-S3 close (the prior HEAD_DELTAS HEAD), and it is **valid**: `git rev-parse b1bed361` resolves and
> `git merge-base b1bed361 HEAD == b1bed361` (it is a strict ancestor of HEAD; `b1bed361` carries no tag). HEAD is
> **`b4c0983d`** (the PHASE4-N-AK AK-S2 impl — the recovered-anchor rollback no-op that completes the live
> follow). The config baseline at the start of this regen was already `b1bed361` (the previous close's commit
> `bbdc3585` bumped it `e99a86c7 → b1bed361` — and `bbdc3585` is the **first** commit in this span), so the window
> measures cleanly from the recorded baseline forward. The span has **two parts**: (1) the **PHASE4-N-AJ close
> commit** — `bbdc3585` (`Close PHASE4-N-AJ — participant-path convergence evidence emission`), which committed
> the N-AJ close artifacts the previous regen ran against as uncommitted working-tree changes (registry status
> flips `DC-NODE-30 → enforced` + `DC-EVIDENCE-03 → enforced_scaffolding`, the HEAD_DELTAS refresh, the cluster
> archive, the c2-guide sync, the baseline bump) — **docs/registry/config only, 0 code, 0 net new rule** (registry
> 354→356 was its delta, already reflected at this baseline); and (2) the **PHASE4-N-AK cluster** (`DC-NODE-31` +
> `DC-NODE-32`) — cluster authority doc + invariants sketch (`c8e44386`, declares `DC-NODE-31`) + the AK-S1 slice
> doc (`f3f1e7ac`) + the AK-S1 impl (`8bb1c402` and its correction `7b3b6779`) + the AK-S2 authority doc
> (`f14dee20`, declares `DC-NODE-32`) + the AK-S2 impl (`b4c0983d`). **The cluster-close registry flips + grounding
> refresh + archive are an uncommitted working-tree close-pass at this regen** (see the working-tree note below).
>
> **Working-tree note (load-bearing).** At the time of this regen there are **UNCOMMITTED working-tree changes** —
> the N-AK close artifacts (registry status flips to `enforced` for `DC-NODE-31`/`DC-NODE-32`, the `T-REC-05`
> strengthening, slice-doc `Merged` flips, the cluster-doc archive move, and this HEAD_DELTAS refresh).
> **§1 narrates the COMMITTED span `b1bed361..b4c0983d` verbatim from `git log`.** The rule **STATUS** in §0/§7 is
> read from the **CURRENT working-tree** `docs/ade-invariant-registry.toml` so the prose reflects the close state
> (`DC-NODE-31` **enforced**, `DC-NODE-32` **enforced**, `T-REC-05` **strengthened** `+= PHASE4-N-AK`). The
> operator bumps `head_deltas_baseline` `b1bed361 → b4c0983d` as a separate post-close step so the next cluster
> measures from here.

This window is **led by PHASE4-N-AK — the recovered-anchor live-follow start + rollback boundary.** It is a
**regression remediation** of the live `recover → follow` path that N-AH/N-AI/N-AJ stood up: after recovery from a
**bare bootstrap anchor** (a recovery snapshot captured at the anchor slot with **no servable post-anchor block**
durable in the `ChainDb`), the prior code had no durable record of WHERE the anchor was, so warm-start resolved the
live-follow FindIntersect start to **Origin** — and the peer's post-intersection `RollBackward(anchor)` then failed
closed (`UnsupportedRollbackPoint` / `UnexpectedRollback`), stalling the follow before it could catch up. N-AK
fixes this **end-to-end** in two slices, the first BLUE-touching and the second a fail-closed RED follow-loop
boundary:

> **PHASE4-N-AK persists the bootstrap anchor POINT `(slot, hash)` as a closed, version-gated, byte-canonical,
> fingerprint-bound recovery-provenance record (`RecoveredAnchorPoint`, a NEW BLUE type) and resolves the
> live-follow FindIntersect start from it whenever the `ChainDb` has no servable post-anchor block — so a
> bare-anchor recovery starts the follow AT THE ANCHOR, not Origin (`DC-NODE-31`). It then accepts the peer's
> post-intersection `RollBackward` whose target binds EXACTLY (slot AND hash) to that persisted anchor as an
> IDEMPOTENT NO-OP boundary rewind on the single-producer follow loop (no WAL, no `ChainDb`/ledger/`chain_dep`/
> cursor mutation), so the loop continues and the forward blocks admit through the EXISTING `pump_block`,
> catching up to the relay tip (`DC-NODE-32`). The anchor is a recovery snapshot BOUNDARY, **never** synthesized
> into a servable block (`ChainDb::tip()` / `last_block_bytes` / serve never return it); `RollBackward(Origin)`
> and **every** non-anchor rollback still FAIL CLOSED (AI-S4a unchanged); the accepted point binds to the
> PERSISTED anchor on slot AND hash, never peer-supplied alone. The persisted anchor point is the durable restart
> authority — NOT CLI re-supply. Same recovered store + same ordered peer feed ⇒ byte-identical post-state and
> admit sequence (extends `T-REC-05` to the recovered tip + follow surface). +2 BLUE canonical types; NO new CI
> gate; NO `RO-LIVE` flip.**

The arc is two slices — the persisted anchor-point provenance + start resolver lands FIRST (BLUE type + codec +
store surface + bootstrap resolver), then the follow-loop rollback boundary completes the catch-up:

- **PHASE4-N-AK / AK-S1 / `DC-NODE-31` (enforced) (recovered-anchor live-follow start authority — BLUE type +
  codec + store surface + bootstrap resolver, RED load).** A new BLUE module
  `crates/ade_ledger/src/recovered_anchor_point.rs` ships **`RecoveredAnchorPoint`** — the closed record
  `{ anchor_fp: Hash32, slot: SlotNo, block_hash: Hash32 }` (all fields required at construction; no `Default`,
  no `#[non_exhaustive]`) — and its closed error sum **`RecoveredAnchorPointError`** (`MalformedCbor` /
  `UnknownVersion { expected, found }` / `Structural { reason }` / `TrailingBytes { extra }`), plus the **sole**
  canonical CBOR codec (`encode_recovered_anchor_point` / `decode_recovered_anchor_point`, a 4-element array
  `[ RECOVERED_ANCHOR_POINT_SCHEMA_VERSION=1, anchor_fp, slot, block_hash ]`; decode rejects unknown version /
  short hash / trailing bytes and verifies a byte-canonical round-trip — a CN-CINPUT-01 analog). The anchor-fp-keyed
  durable surface is the new `SnapshotStore::{put,get}_recovered_anchor_point` (redb table
  `recovered_anchor_point_by_anchor_fp`, in `chaindb/{mod,in_memory,persistent}.rs`). The write site is the shared
  `seed_epoch_lineage::persist_seed_epoch_consensus_inputs` (which already persists the seed-epoch sidecar — the
  anchor-point record is a **separate** additive record sharing the same `anchor_fp` key, NOT touching the
  sidecar's shape/schema/hash). On warm-start, the new RED `crates/ade_runtime/src/recovered_anchor.rs`
  (`load_recovered_anchor_point` — load + fail-closed verify, kept OUT of `bootstrap.rs` to preserve the
  `CN-NODE-01` single-`pub fn` bootstrap closure) loads the record and binds it; `bootstrap.rs` gains a new
  `BootstrapInputs.recovered_anchor` field and the private total resolver `resolve_live_follow_start(tip,
  recovered_anchor)` (resolution order: **servable `ChainDb` tip → persisted non-Origin anchor point → Origin/None
  only if truly cold-start**; a zero/null-hash anchor is treated as Origin). Three new fail-closed `BootstrapError`
  variants — `RecoveredAnchorPointMissing { anchor_fp }`, `RecoveredAnchorPointDecode(RecoveredAnchorPointError)`,
  `RecoveredAnchorPointBindingMismatch { expected_anchor_fp, actual_anchor_fp }` — make a non-Origin recovered store
  whose anchor-point record is missing / malformed / fingerprint-mismatched a deterministic halt **before** live
  follow starts (never a silent Origin fallback). `ChainDb::tip()` semantics unchanged; no servable block
  synthesized; the wire-pump consumer (`spawn_live_wire_pump_source`) unchanged. **`DC-NODE-31 → enforced`** (11
  hermetic tests across `bootstrap.rs` / `seed_epoch_lineage.rs` / `recovered_anchor_point.rs` /
  `node_lifecycle.rs`; **no dedicated CI gate** — enforced by the unit/integration suite + the existing
  `ci_check_bootstrap_closure.sh` single-`pub fn` fence). *(The `7b3b6779` correction — "OQ-AK-1 corrected" —
  refined the persisted provenance within AK-S1 before the AK-S2 doc; same slice.)*
- **PHASE4-N-AK / AK-S2 / `DC-NODE-32` (enforced) (recovered-anchor rollback boundary completes the live follow —
  RED follow-loop no-op + fail-closed fence).** `forward_sync/reducer.rs` gains a `ForwardSyncState.recovered_anchor:
  Option<ChainTip>` field (default `None`; the recover path sets it to `BootstrapState.tip`). The single-producer
  `node_sync::run_node_sync` `RollBack` handler now matches `(&state.recovered_anchor, &point)`: a `RollBackward`
  whose target binds **EXACTLY (slot AND hash)** to the persisted recovered anchor is an **idempotent NO-OP**
  (`continue` — no `commit_rollback`, no `WalEntry::RollBack`, no `ChainDb`/ledger/`chain_dep`/cursor mutation);
  **every other point still `Err(NodeSyncError::UnexpectedRollback)`** — `RollBackward(Origin)` fails closed
  (AI-S4a unchanged), every non-anchor non-Origin rollback fails closed, and slot-only / hash-only near-misses fail
  closed (the bind is slot **AND** hash, never peer-supplied alone). The anchor point consumed by the loop is the
  single authority (`BootstrapState.tip`), threaded in — **NEVER re-read from the store inside the loop**. The
  first forward block after the anchor admits through the **EXISTING** sole `pump_block` path (its `prev_hash` binds
  the recovered `chain_dep`) — **AK-S2 adds NO forward-link code**. `node_lifecycle.rs` sets `fwd.recovered_anchor =
  BootstrapState.tip` on the ON arm; **`run_participant_sync` is UNCHANGED** (a separate follow-on). **`DC-NODE-32 →
  enforced`** (6 hermetic CEs `ak_s2_*` in `live_fork_choice_ai_s4bii.rs` + a `node_sync.rs` unit test:
  idempotent-no-op; Origin-fails-closed-even-with-anchor; non-anchor-fails-closed-slot-and-hash-bound;
  no-recovered-anchor-still-fails-closed; forward-block-reaches-pump-block-after-no-op; single-producer rollback
  refused; **no new CI gate**). **Honesty:** SCOPE is the **single-producer `run_node_sync` recovered-anchor
  rollback-to-intersection ONLY** — it does NOT add general stored-block rollback-follow on the single-producer
  path, does NOT touch the participant path, and does NOT claim full ChainSel convergence.

**+2 BLUE canonical types** (`RecoveredAnchorPoint` + `RecoveredAnchorPointError`, both in the new BLUE module
`ade_ledger::recovered_anchor_point`; 456 → 458 BLUE `pub struct`/`pub enum` lines). **No `RO-LIVE` rule flipped** —
`RO-LIVE-01` stays operator-gated. The live **CE-AK-3** end-to-end pass IS recorded as `enforced`-backing evidence
for `DC-NODE-31` + `DC-NODE-32` (2026-06-10, frozen c2-relay venue: re-recover → FindIntersect at the persisted
anchor → `RollBackward(anchor)` idempotent no-op → caught up to `forge_base_block_no=13` == the frozen relay tip;
**0 `UnsupportedRollbackPoint` + 0 `UnexpectedRollback`**), NOT a bounty/preprod completion claim.

## 0. Headline

| Count | Baseline (`b1bed361`) | HEAD (`b4c0983d` + close working-tree) | Δ |
|---|---|---|---|
| CI gates (`ci/ci_check_*.sh`) | 159 | **159** | **±0** — **no gate added, modified, or removed** (`--diff-filter=A` / `--diff-filter=M` / `--diff-filter=D` over `ci/` all **empty**). `DC-NODE-31` + `DC-NODE-32` carry **`ci_script = ""`** — they are enforced by the unit/integration test suite (11 + 7 named tests) plus the EXISTING `ci_check_bootstrap_closure.sh` single-`pub fn` fence (the new RED `recovered_anchor.rs` was deliberately kept OUT of `bootstrap.rs` to preserve `CN-NODE-01`). |
| Registry rules (`docs/ade-invariant-registry.toml`) | 356 | **358** | **+2** — two NEW rules `DC-NODE-31` + `DC-NODE-32`. **Zero removed** (`diff` of the sorted `id =` lists shows exactly the two additions and no removal). |
| Registry status (enforced / enforced_scaffolding / partial / declared) | 221 / 1 / 19 / 116 | **224 / 1 / 19 / 114** | **+3 enforced**, **−2 declared** (the `enforced_scaffolding=1` and `partial=19` unchanged). The +3 enforced = the two NEW N-AK rules (`DC-NODE-31`, `DC-NODE-32`) **plus `DC-NODE-30`**, which was `declared` at baseline `b1bed361` (the committed AJ-S3 state) and flipped to `enforced` by the **N-AJ close commit `bbdc3585`** (the first commit in this span); the two declared exits are `DC-NODE-30` (→ enforced) and `DC-EVIDENCE-03` (→ enforced_scaffolding, also by `bbdc3585`). |
| Registry strengthenings | — | **1** | **`strengthened_in += "PHASE4-N-AK"`** on **1** existing rule: **`T-REC-05`** (replay-equivalence now extends to the recovered tip surface AND the single-producer follow: same recovered store + same WAL ⇒ same anchor point ⇒ same `BootstrapState.tip` ⇒ same FindIntersect start AND same admit sequence). No rule weakened; no rule removed. |
| BLUE canonical types | 456 | **458** | **+2** — `RecoveredAnchorPoint` (struct) + `RecoveredAnchorPointError` (enum), both in the NEW BLUE module `crates/ade_ledger/src/recovered_anchor_point.rs`. `git diff b1bed361..HEAD` over the BLUE `core_paths` trees adds exactly those two `^+(pub )?(struct\|enum)` lines and removes none. No `Cargo.toml` changed — still 11 crates. |
| Grounding docs | CODEMAP / SEAMS / TRACEABILITY all pinned at **`5ec841c8`** (the N-AI close — 460 types-by-old-count / 157 CI / 354 rules), already **one cluster stale at baseline** (they do not carry the N-AJ `DC-NODE-30` / `DC-EVIDENCE-03` / `convergence_evidence` module / the two N-AJ gates) | Still pinned at **`5ec841c8`** — now **two clusters stale**: they additionally do not carry `DC-NODE-31`, `DC-NODE-32`, the NEW BLUE `ade_ledger::recovered_anchor_point` module, the NEW RED `ade_runtime::recovered_anchor` module, the new `SnapshotStore` `recovered_anchor_point` surface, or the `BootstrapInputs.recovered_anchor` / `ForwardSyncState.recovered_anchor` fields (grep = 0). | **CODEMAP + SEAMS + TRACEABILITY are now two clusters STALE** — the registry holds the four new rules (N-AJ's two + N-AK's two) + their bindings authoritatively at HEAD (**358 rules**); the refresh to `b4c0983d` is a follow-on item this close. See the cross-reference warnings at the end of §2 and §5. |

> **Grounding-doc state this close (load-bearing).** **CODEMAP, SEAMS, and TRACEABILITY all remain pinned at
> `5ec841c8`** (the N-AI close) and are now **two clusters stale** (N-AJ + N-AK). None carries `DC-NODE-31`,
> `DC-NODE-32`, the new BLUE `ade_ledger::recovered_anchor_point` module, the new RED `ade_runtime::recovered_anchor`
> module, or the new `recovered_anchor_point` store surface; `grep -c` of `RecoveredAnchorPoint` / `DC-NODE-31` /
> `DC-NODE-32` in all three is 0. The invariant registry holds the two new rules + the `T-REC-05` strengthening
> authoritatively at HEAD (**358 rules**); the CODEMAP + SEAMS + TRACEABILITY refresh to `b4c0983d` is a follow-on
> item this close (surfaced in §2 and §5). **The N-AJ refresh debt is now folded into the same follow-on.**

The slice↔rule↔gate map for this window:

| Slice | Rule(s) | Gate | What shipped |
|---|---|---|---|
| **N-AJ close** (`bbdc3585`) | flip `DC-NODE-30 → enforced`; `DC-EVIDENCE-03 → enforced_scaffolding`; `DC-ADMIT-04` strengthened (all PHASE4-N-AJ) | — (no new gate) | **docs/registry/config only — 0 code.** Committed the N-AJ close artifacts the previous regen ran against as uncommitted working-tree (HEAD_DELTAS refresh `e99a86c7..b1bed361`, registry 354→356, cluster archive, c2-guide sync, baseline bump `e99a86c7 → b1bed361`). Folded into this span because it sits inside `b1bed361..HEAD`; it is **not** N-AK work. |
| **cluster doc** (`c8e44386`) | `DC-NODE-31` **declared** | — | N-AK cluster authority doc + invariants sketch; declares `DC-NODE-31`. Also declares `DC-NODE-32` placeholder ahead of AK-S2's authority doc. |
| **AK-S1** (`8bb1c402` + correction `7b3b6779`) | **`DC-NODE-31`** (NEW, → enforced at close) | (none — `ci_script=""`; reuses `ci_check_bootstrap_closure.sh`) | **BLUE type + codec + store surface + bootstrap resolver, RED load.** NEW BLUE `recovered_anchor_point.rs` (`RecoveredAnchorPoint` + `RecoveredAnchorPointError` + sole CBOR codec); NEW RED `recovered_anchor.rs` (`load_recovered_anchor_point`); `SnapshotStore::{put,get}_recovered_anchor_point` (redb `recovered_anchor_point_by_anchor_fp`); `BootstrapInputs.recovered_anchor` + `resolve_live_follow_start` + 3 new fail-closed `BootstrapError` variants; write at `persist_seed_epoch_consensus_inputs`. |
| **AK-S2** (`f14dee20` doc, `b4c0983d` impl) | **`DC-NODE-32`** (NEW, → enforced at close); `T-REC-05` strengthened | (none — `ci_script=""`) | **RED follow-loop no-op + fail-closed fence.** `ForwardSyncState.recovered_anchor` field; `run_node_sync` `RollBack` handler accepts `RollBackward(anchor)` (exact slot AND hash) as idempotent no-op, all else `UnexpectedRollback`; forward block admits via EXISTING `pump_block`. **Last slice — the cluster-close flips/archive are the in-progress working-tree close.** |

The per-commit shape (the full verbatim log is §1):

| Commit | Kind | What it did | Code / CI / registry effect |
|--------|------|-------------|-----------------------------|
| `bbdc3585` | (close) | Close PHASE4-N-AJ — participant-path convergence evidence emission (the CE-AI-6 bridge) | **0 code / 0 CI**; docs/registry/config: HEAD_DELTAS `e99a86c7..b1bed361`, registry 354→356 (`DC-NODE-30 → enforced`, `DC-EVIDENCE-03 → enforced_scaffolding`, `DC-ADMIT-04` strengthened), cluster archive, c2-guide sync, baseline bump `e99a86c7 → b1bed361`. **N-AJ, not N-AK** |
| `c8e44386` | docs (phase4-n-ak) | Cluster authority doc + invariants sketch; declare `DC-NODE-31` | **0 code / 0 CI**; registry: `DC-NODE-31` declared |
| `f3f1e7ac` | docs (AK-S1) | Slice doc AK-S1 — recovered-anchor live-follow start | **0 code / 0 CI / 0 rule** |
| `7b3b6779` | revise (AK-S1) | Persist recovered anchor-point provenance (OQ-AK-1 corrected) | **BLUE+RED code** (correction within AK-S1 — refines the persisted provenance); part of the AK-S1 impl |
| `8bb1c402` | feat (AK-S1) | Persist recovered anchor point for live follow start (`DC-NODE-31`) | **BLUE+RED code** (NEW BLUE `recovered_anchor_point.rs` + NEW RED `recovered_anchor.rs` + `bootstrap.rs` + `chaindb/{mod,in_memory,persistent}.rs` + `seed_epoch_lineage.rs` + the `recovered_anchor: None` struct-init touches); **+2 BLUE types**; **+0 CI**; registry: `DC-NODE-31` → enforced at close |
| `f14dee20` | docs (AK-S2) | AK-S2 authority — recovered-anchor follow-rollback boundary (`DC-NODE-32`) | **0 code / 0 CI**; registry: `DC-NODE-32` declared |
| `b4c0983d` | feat (AK-S2) | AK-S2 — recovered-anchor rollback no-op completes live follow (`DC-NODE-32`) | **RED code** (`forward_sync/reducer.rs` `ForwardSyncState.recovered_anchor` + `node_sync.rs` `run_node_sync` rollback no-op + `node_lifecycle.rs` ON-arm wiring); **+0 BLUE type**; **+0 CI**; registry: `DC-NODE-32` → enforced + `T-REC-05` strengthened at close |

## 1. Commit Log (newest first)

| Hash | Type | Summary |
|------|------|---------|
| `b4c0983d` | feat | feat(phase4-n-ak): AK-S2 -- recovered-anchor rollback no-op completes live follow (DC-NODE-32) |
| `f14dee20` | docs | docs(phase4-n-ak): AK-S2 authority -- recovered-anchor follow-rollback boundary (DC-NODE-32) |
| `8bb1c402` | feat | feat(phase4-n-ak): persist recovered anchor point for live follow start |
| `7b3b6779` | revise | revise(phase4-n-ak): persist recovered anchor-point provenance (OQ-AK-1 corrected) |
| `f3f1e7ac` | docs | docs(phase4-n-ak): slice doc AK-S1 -- recovered-anchor live-follow start |
| `c8e44386` | docs | docs(phase4-n-ak): cluster authority doc + invariants sketch; declare DC-NODE-31 |
| `bbdc3585` | (close) | Close PHASE4-N-AJ — participant-path convergence evidence emission (the CE-AI-6 bridge) |

No merge commits in the span. **7 commits, zero unclassified.** Six subjects carry an explicit
conventional-commits prefix (`docs(...)` / `feat(...)` / `revise(...)`); the seventh (`bbdc3585`, `Close
PHASE4-N-AJ …`) is the prior-window **close commit** (no prefix — the project's close-commit convention), folded
into this span because it sits inside `b1bed361..HEAD`. The production code lands in the two `feat(...)` commits
(`8bb1c402` AK-S1 + `b4c0983d` AK-S2) and the AK-S1 `revise(...)` correction (`7b3b6779`); the rest are `docs(...)`
slice/cluster docs. **`bbdc3585`** is **N-AJ close work, not PHASE4-N-AK** (docs/registry/config only — 0 code). All
commits landed 2026-06-10.

> **Note (commit-attribution policy).** Per this repo's `CLAUDE.md` override (vibe-coded-node bounty
> trailer requirement), commits in this repo carry a `Co-Authored-By:` model-attribution trailer; that
> is an Ade-local override of the global no-AI-attribution rule and applies to **commit messages
> only**. It does not affect this doc's content.

## 2. New Modules

**Two new modules — one BLUE (`ade_ledger::recovered_anchor_point`) + one RED (`ade_runtime::recovered_anchor`).**
`git diff --diff-filter=A --name-only b1bed361..HEAD -- 'crates/**/*.rs'` lists exactly **two** new `.rs` library
modules: `crates/ade_ledger/src/recovered_anchor_point.rs` (+318, registered in `crates/ade_ledger/src/lib.rs`)
and `crates/ade_runtime/src/recovered_anchor.rs` (+72, registered in `crates/ade_runtime/src/lib.rs`). There is
**no new crate, no new `Cargo.toml`, no new workspace** (`git diff --name-only … '**/Cargo.toml'` is empty; still
**11 crates**).

| Module | Color | Purpose | Key sub-paths | Added in (cluster/slice) |
|--------|-------|---------|---------------|--------------------------|
| `ade_ledger::recovered_anchor_point` | **BLUE** | The closed, version-gated, byte-canonical record of the bootstrap anchor POINT `(slot, hash)` the recovered store was seeded at — persisted as replayable recovery provenance, bound to the recovered anchor fingerprint (`anchor_fp`). The durable restart authority for the live-follow FindIntersect start tip (`DC-NODE-31`). Separate additive record from `SeedEpochConsensusInputs`; shares the `anchor_fp` key. | `RecoveredAnchorPoint { anchor_fp: Hash32, slot: SlotNo, block_hash: Hash32 }` (no `Default`, no `#[non_exhaustive]`), `RecoveredAnchorPointError { MalformedCbor \| UnknownVersion \| Structural \| TrailingBytes }`, `encode_recovered_anchor_point` / `decode_recovered_anchor_point` (sole codec pair), `RECOVERED_ANCHOR_POINT_SCHEMA_VERSION = 1` | **PHASE4-N-AK AK-S1** (`8bb1c402`) |
| `ade_runtime::recovered_anchor` | **RED** (store-read I/O of a BLUE record; the decode + binding check are BLUE) | The recover-time load + fail-closed verify of the persisted anchor-point record. Companion to `bootstrap::resolve_live_follow_start` — kept OUT of `bootstrap.rs` so that module stays the single-`pub fn` bootstrap authority (`CN-NODE-01`, `ci_check_bootstrap_closure.sh`). Mirrors `bootstrap::restore_seed_epoch_consensus_inputs`. | `load_recovered_anchor_point<S: SnapshotStore>(store, anchor_fp) -> Result<ChainTip, BootstrapError>` — one `SnapshotStore::get_recovered_anchor_point` read + `decode_recovered_anchor_point` + the `anchor_fp` binding check; missing/malformed/mismatched ⇒ fail-closed, never silent Origin | **PHASE4-N-AK AK-S1** (`8bb1c402`) |

> **Cross-reference (CODEMAP) — STALE: new modules NOT yet in CODEMAP.** Neither new module
> (`ade_ledger::recovered_anchor_point` BLUE, `ade_runtime::recovered_anchor` RED) nor its types
> (`RecoveredAnchorPoint`, `RecoveredAnchorPointError`) is in CODEMAP — CODEMAP is pinned at `5ec841c8` (the N-AI
> close) and predates both N-AJ and N-AK (`grep -c RecoveredAnchorPoint` in CODEMAP = 0). **Action:** regenerate
> CODEMAP to `b4c0983d` so the new BLUE module appears in CODEMAP §BLUE (with its `Creates`/`Interprets`/`MUST
> NOT` rows) and the new RED module in §RED; until then the registry (`DC-NODE-31` `code_locus`) + this doc are
> authoritative for the modules. **This is a refresh-on-this-close item, not a discipline gap** — both modules ship
> fully tested; the registry holds their bindings at HEAD. (The N-AJ `ade_node::convergence_evidence` module is
> ALSO still absent from CODEMAP — fold both into the one CODEMAP regen.)

## 3. Modules Modified

Beyond the two new modules (§2), **eleven existing source files across three crates** changed — `ade_ledger`
(crate root only), `ade_runtime` (bootstrap + storage + recovery + seed-epoch), and `ade_node` (lifecycle + sync +
node), plus mechanical struct-init touches. The substantive changes group cleanly by slice:

| Module | Color / scope | Key changes |
|--------|---------------|-------------|
| `ade_runtime::bootstrap` (`bootstrap.rs` +373/−…) | **RED** shell with **BLUE** resolver, additive | **AK-S1 (`8bb1c402`):** new `BootstrapInputs.recovered_anchor: Option<ChainTip>` canonical input; new **pure, total** private resolver `resolve_live_follow_start(tip, recovered_anchor)` (resolution order servable `ChainDb` tip → persisted non-Origin anchor point → Origin/None; a zero/null-hash anchor ⇒ Origin); `bootstrap_initial_state` now resolves the live-follow start tip from it. **Three new fail-closed `BootstrapError` variants** — `RecoveredAnchorPointMissing { anchor_fp }`, `RecoveredAnchorPointDecode(RecoveredAnchorPointError)`, `RecoveredAnchorPointBindingMismatch { expected_anchor_fp, actual_anchor_fp }`. `ChainDb::tip()` semantics unchanged; no servable block synthesized. Hosts 8 of the `DC-NODE-31` tests. |
| `ade_runtime::chaindb` (`mod.rs` +26, `persistent.rs` +67, `in_memory.rs` +32) | **RED** store surface, additive | **AK-S1 (`8bb1c402`):** new `SnapshotStore::{put,get}_recovered_anchor_point` trait methods (anchor-fp-keyed; the record's bytes are the canonical `ade_ledger::recovered_anchor_point` form). `persistent.rs` adds the redb `TableDefinition` `recovered_anchor_point_by_anchor_fp` (a distinct key-space — cannot collide with existing tables; a missing table on `get` returns `Ok(None)`). `in_memory.rs` adds the in-memory analog. Existing tables/methods untouched. |
| `ade_runtime::seed_epoch_lineage` (`seed_epoch_lineage.rs` +64) | **RED** persist driver, additive | **AK-S1 (`8bb1c402`):** `persist_seed_epoch_consensus_inputs` now ALSO writes the `RecoveredAnchorPoint` record (`put_recovered_anchor_point(anchor_fp, encode_recovered_anchor_point(&ap))`) at seed/recover — a **separate** additive record sharing the `anchor_fp` key, NOT touching the seed-epoch sidecar's shape / schema version / `sidecar_hash`. New error path via `SeedEpochLineagePersistError::Persist`. Hosts the `bootstrap_recover_persists_anchor_point_sidecar` test (CE-AK-1). |
| `ade_runtime::forward_sync::reducer` (`reducer.rs` +15/−…) | **GREEN/RED** state carrier, additive | **AK-S2 (`b4c0983d`):** `ForwardSyncState` gains `recovered_anchor: Option<ChainTip>` (default `None`; the recover path sets it to `BootstrapState.tip`). Documented as a recovery snapshot boundary — NOT a servable block, never synthesized. No reducer decision logic changed. |
| `ade_node::node_lifecycle` (`node_lifecycle.rs` +162/−…) | **RED** loop + recover driver, additive | **AK-S1 (`8bb1c402`):** `warm_start_recovery` calls `load_recovered_anchor_point(chaindb, &anchor_fp)` once the recovered `anchor_fp` is discovered and threads the result into `BootstrapInputs.recovered_anchor`, so `resolve_live_follow_start(chaindb.tip(), recovered_anchor)` sets the live-follow start tip. **AK-S2 (`b4c0983d`):** the ON arm sets `fwd.recovered_anchor = state.tip.clone()` (the `BootstrapState.tip` single authority). **`run_participant_sync` UNCHANGED** (a separate follow-on). Hosts the `recovered_bare_anchor_findintersect_starts_at_anchor_not_origin` test. |
| `ade_node::node_sync` (`node_sync.rs` +107/−…) | **GREEN/RED** single-producer follow loop, additive | **AK-S2 (`b4c0983d`):** the `run_node_sync` `RollBack` handler now matches `(&state.recovered_anchor, &point)` — a `RollBackward` binding EXACTLY (slot AND hash) to the persisted recovered anchor is an idempotent NO-OP (`continue` — no `commit_rollback` / `WalEntry::RollBack` / `ChainDb`/ledger/`chain_dep`/cursor mutation); every other point ⇒ `Err(NodeSyncError::UnexpectedRollback)` (Origin + non-anchor + slot-only + hash-only all fail closed). The forward block after the anchor admits via the EXISTING `pump_block`. Hosts the `ak_s2_valid_forward_block_admits_after_recovered_anchor_noop` unit test. |
| `ade_node::node` (`node.rs` +13), `ade_node::produce_mode` (`produce_mode.rs` +4), `ade_runtime::genesis_bootstrap` (+4), `ade_runtime::mithril_bootstrap` (+4), `ade_runtime::recovery::restart` (+6), `ade_testkit::consensus::genesis_pinning` (+1), `ade_ledger::lib` / `ade_runtime::lib` (+1 each) | mechanical / crate-root | **AK-S1/S2:** mechanical `recovered_anchor: None` struct-init additions at every existing `BootstrapInputs` / `ForwardSyncState` construction site (the new field's default — `None` ⇒ pre-AK behavior verbatim; first-run genesis/Mithril bootstrap supplies no recovered anchor, only the warm-start recover path does). `lib.rs` of each new module's crate adds the `pub mod` registration (§2). Behavior-preserving; no new type. |

> **BLUE change this span (load-bearing).** Unlike the immediately-prior N-AJ window (evidence-only, 0 BLUE), this
> span **adds a BLUE module**: `git diff b1bed361..HEAD` over the BLUE `core_paths` trees lists
> `crates/ade_ledger/src/recovered_anchor_point.rs` (new) and adds exactly **two** `^+(pub )?(struct\|enum)` lines
> (`RecoveredAnchorPoint`, `RecoveredAnchorPointError`) — BLUE count **456 → 458**. The BLUE surface is the closed
> anchor-point record + its sole canonical codec; everything else (the load, the store surface, the bootstrap
> resolver wiring, the follow-loop no-op) is the RED `ade_runtime` / `ade_node` shell. The decode + `anchor_fp`
> binding check inside the RED `recovered_anchor.rs` load are BLUE-authoritative (the codec is the sole authority);
> the `SnapshotStore` read itself is RED I/O.

## 4. Feature Flags

**No project feature-flag deltas.** Ade declares no `[features]` table in any workspace `Cargo.toml`, and **no
`Cargo.toml` changed in this window** (`git diff --name-only b1bed361..HEAD -- '**/Cargo.toml' 'Cargo.toml'` is
empty). No `#[cfg(feature = …)]` gate was introduced and no `compile_error!` coupling was added. **No new CLI flag
this span either** — the new behavior is a NEW typed struct field (`BootstrapInputs.recovered_anchor` /
`ForwardSyncState.recovered_anchor`, default `None`), not a flag: it is populated by the recover path itself
(`warm_start_recovery` → `load_recovered_anchor_point`), and a `None` recovered anchor reproduces pre-AK behavior
verbatim (first-run genesis/Mithril bootstrap supplies `None`). The durable restart authority is the **persisted
anchor-point record**, explicitly NOT CLI re-supply (`DC-NODE-31`: "the persisted anchor point is the durable
restart authority — NOT CLI re-supply").

## 5. CI Checks (159 → 159; no gate added, modified, or removed)

**Zero CI-script changes this span.** `git diff --diff-filter=A b1bed361..HEAD -- ci/`,
`--diff-filter=M`, and `--diff-filter=D` over `ci/` are **all empty** — no gate was added, modified in place, or
removed; the count holds at **159**. Both new rules carry **`ci_script = ""`**: they are enforced by the
unit/integration **test suite** (the `code_locus`-named tests below) plus the EXISTING
`ci_check_bootstrap_closure.sh` single-`pub fn` fence (which is **why** `load_recovered_anchor_point` was placed in
the NEW `ade_runtime::recovered_anchor` module rather than in `bootstrap.rs` — to keep `bootstrap.rs`'s single-pub-fn
closure intact under that gate, preserving `CN-NODE-01`).

### PHASE4-N-AK enforcement (AK-S1 / AK-S2) — test-suite + existing-gate-backed, no new gate

| Rule | Enforced by | What it checks |
|------|-------------|----------------|
| `DC-NODE-31` | `ci_script=""`; 11 named tests + EXISTING `ci_check_bootstrap_closure.sh` | The persisted anchor point round-trips byte-identically (`recovered_anchor_point_round_trips_byte_identical`); a zero-hash anchor resolves to Origin; a bare-anchor recovery surfaces the anchor as the live-follow tip; a true-Origin recovery surfaces `None`; a servable `ChainDb` tip wins over the anchor; warm-start loads the persisted record; missing / fingerprint-mismatched record fails closed; same store ⇒ same FindIntersect start; persist writes the anchor-point sidecar; FindIntersect starts at the anchor not Origin. `bootstrap.rs` stays single-`pub fn` (the load lives in `recovered_anchor.rs`). |
| `DC-NODE-32` | `ci_script=""`; 7 named tests | `RollBackward(anchor)` (exact slot AND hash) is an idempotent no-op; `RollBackward(Origin)` fails closed even with a recovered anchor present; a non-anchor rollback fails closed (slot AND hash bound); no-recovered-anchor still fails closed; the forward block after the no-op reaches `pump_block` and admits (tip advances); the single-producer path refuses a generic rollback via `run_node_sync`. |

> **Cross-reference (CODEMAP + SEAMS + TRACEABILITY) — STALE this close; refresh owed.** The new rule↔enforcement
> bindings (`DC-NODE-31` ↔ its 11 tests + `ci_check_bootstrap_closure.sh`; `DC-NODE-32` ↔ its 7 tests) are recorded
> **in the registry at HEAD** (`docs/ade-invariant-registry.toml`, 358 rules). They are **NOT yet in TRACEABILITY,
> SEAMS, or CODEMAP**, all three of which remain pinned at the N-AI close `5ec841c8` (`grep -c` of `DC-NODE-31` /
> `DC-NODE-32` in each = 0). **No gate is orphaned** (no gate was added). **TRACEABILITY note:** because both new
> rules carry `ci_script=""`, TRACEABILITY's enforcement column for them is the **test suite**, not a `ci_check_*`
> script — this is intentional (the existing `ci_check_bootstrap_closure.sh` fences the single-pub-fn closure that
> makes the placement safe, but does not itself check the anchor-point semantics). **Action:** regenerate CODEMAP +
> SEAMS + TRACEABILITY to `b4c0983d` as a follow-on this close (folding in the still-pending N-AJ refresh) so the
> two new modules appear in CODEMAP and every N-AJ + N-AK rule appears in TRACEABILITY with its named enforcement;
> until then the registry is authoritative for the new bindings.

## 6. Canonical Type Registry Delta

**n/a — no separate canonical-type registry is configured** (`canonical_type_registry: null`);
canonical-type rules live inline in the invariant registry under family **T**. **This window ADDED 2 BLUE
canonical types:** the BLUE `pub struct`/`pub enum` count over the `core_paths` trees is **`456 → 458`** —
`RecoveredAnchorPoint` (struct) + `RecoveredAnchorPointError` (enum), both in the NEW BLUE module
`crates/ade_ledger/src/recovered_anchor_point.rs` (the `ade_ledger` crate is the first BLUE `core_paths` entry).
`git diff b1bed361..HEAD` over the BLUE trees adds exactly those two `^+(pub )?(struct|enum)` lines and removes
none. **Zero BLUE canonical types were removed.** No `Cargo.toml` changed (still 11 crates). The new RED types this
window are confined to method signatures / struct fields (`SnapshotStore::{put,get}_recovered_anchor_point`,
`BootstrapInputs.recovered_anchor`, `ForwardSyncState.recovered_anchor`, the 3 new `BootstrapError` variants) — RED
shell surface, outside the BLUE canonical-type count.

## 7. Normative / Invariant Rule Delta (356 → 358; +2 rules, 1 strengthening, zero removals)

**Two rule IDs were added; zero removed** (`356 → 358`; `diff` of the sorted `id =` lists shows exactly the two
additions `DC-NODE-31` + `DC-NODE-32` and no removal). The status tally moves **221 → 224 enforced** and **116 →
114 declared** (the `enforced_scaffolding = 1` and `partial = 19` unchanged). The +3 enforced / −2 declared
reconciles as: the two NEW N-AK rules land **enforced** (+2 enforced), **and** the **N-AJ close commit `bbdc3585`**
(the first commit in this span) flipped `DC-NODE-30 declared → enforced` (+1 enforced, −1 declared) and
`DC-EVIDENCE-03 declared → enforced_scaffolding` (−1 declared, +1 enforced_scaffolding — but that tier was already
1 at baseline because the committed AJ-S3 baseline had it `declared`; net the two declared exits are `DC-NODE-30` +
`DC-EVIDENCE-03`).

*(The configured `normative_docs` — the CE-79 tier-gate statement + addendum, the three contract docs, the
CE-73 reclassification, and `CLAUDE.md` — were **not** changed this span: `git diff --name-only b1bed361..HEAD`
over those paths is empty. The rule-count delta is entirely the invariant-registry change.)*

**New rules (`+2`, both `introduced_in = "PHASE4-N-AK"`, both enforced):**

| Rule | Family / Tier · Status | Statement (summary) |
|------|------------------------|---------------------|
| `DC-NODE-31` | DC / `derived` · **enforced** | **Recovered-anchor live-follow start authority.** After recovery from a non-Origin bootstrap anchor, the recovered store PERSISTS the bootstrap anchor point `(slot, hash)` as replayable recovery provenance, bound to the recovered `anchor_fp`. On warm-start, `BootstrapState` resolves the live-follow start tip from that persisted anchor point whenever `ChainDb` has no servable post-anchor block; resolution order = servable `ChainDb` tip → persisted recovered anchor point (non-Origin + provenance-bound) → Origin/None only if truly cold-start. A non-Origin recovered store whose anchor-point record is missing / malformed / fingerprint-mismatched **FAILS CLOSED before live follow starts**. Same recovered store + same WAL ⇒ same anchor point ⇒ same `BootstrapState.tip` ⇒ same FindIntersect start (replay-equivalent; extends `T-REC-05` to the recovered tip surface). **MUST NOT:** the persisted anchor point is the durable restart authority, **NOT** CLI re-supply (CLI seed-point is first-run input only); does **not** change `ChainDb::tip()` semantics and does **not** synthesize a servable block; AI-S4a `RollBackward(Origin)` fail-close unchanged; the wire-pump consumer (`spawn_live_wire_pump_source`) UNCHANGED. |
| `DC-NODE-32` | DC / `derived` · **enforced** | **Recovered-anchor rollback boundary on the single-producer live-follow path (AK-S2).** After recovery to a bare bootstrap anchor, `run_node_sync` accepts a peer `RollBackward` whose target binds EXACTLY (slot AND hash) to the persisted recovered anchor point (`DC-NODE-31` / `BootstrapState.tip`) as an IDEMPOTENT NO-OP boundary rewind: no WAL, no `ChainDb` mutation, no ledger mutation, no cursor. **MUST NOT:** the anchor is a recovery snapshot boundary, **NOT** a stored servable block, and is **NEVER** synthesized into one (`ChainDb::tip()` / `last_block_bytes` / serve never return it); `RollBackward(Origin)` still fails closed (AI-S4a unchanged); every non-anchor, non-Origin rollback fails closed; the accepted point must bind to the PERSISTED anchor on slot AND hash, **never peer-supplied alone**; the anchor point consumed by the loop is the single authority (`BootstrapState.tip`), threaded in — **NEVER re-read from the store inside the loop**. The first forward block after the anchor admits through the EXISTING sole `pump_block` path (AK-S2 adds **no** forward-link code). Recover→follow on the single-producer path is replay-equivalent (extends `T-REC-05`/`DC-NODE-31` to the follow). **SCOPE:** does NOT add general stored-block rollback-follow on the single-producer path, and does NOT touch the participant path (`run_participant_sync` — a separate follow-on). |

**Strengthenings (`strengthened_in += "PHASE4-N-AK"`) — 1:** `T-REC-05` (replay-equivalence now extends to the
recovered tip surface AND the single-producer follow: same recovered store + same WAL ⇒ same persisted anchor point
⇒ same `BootstrapState.tip` ⇒ same FindIntersect start, and same store + same ordered peer feed ⇒ byte-identical
post-state and admit sequence across the recovered-anchor rollback no-op + forward catch-up). **No rule was
weakened.**

**No rule was removed (expected: 0).** The registry delta is **two new rules (`DC-NODE-31` + `DC-NODE-32`, both
enforced), one `strengthened_in += PHASE4-N-AK` append (`T-REC-05`), zero removals** — consistent with append-only
registry discipline. **No anomaly.** (The +3-enforced / −2-declared tally includes the `DC-NODE-30` /
`DC-EVIDENCE-03` flips carried by the in-span N-AJ close commit `bbdc3585`, accounted above.)

## Honest residual (window scope)

PHASE4-N-AK **remediated the live recover→follow regression** by giving the recovered store a durable record of its
bootstrap anchor point and resolving the live-follow start (and the follow's first rollback) from it. The honest
residual:

- **The headline boundary (verbatim).** Ade now **persists the bootstrap anchor point** as fingerprint-bound,
  version-gated, byte-canonical recovery provenance (`RecoveredAnchorPoint`), resolves the live-follow FindIntersect
  start from it (bare-anchor recovery starts AT the anchor, not Origin — `DC-NODE-31`), and accepts the peer's
  post-intersection `RollBackward(anchor)` (exact slot AND hash) as an idempotent boundary no-op so the
  single-producer follow loop catches up (`DC-NODE-32`). **The anchor is a recovery BOUNDARY, never a servable
  block** — `ChainDb::tip()` / serve never return it; `pump_block` stays the sole roll-forward admit.
- **CE-AK-3 is `enforced`-backing evidence, NOT a `RO-LIVE` flip.** The live end-to-end pass (2026-06-10, frozen
  c2-relay venue: re-recover → FindIntersect at the persisted anchor → `RollBackward(anchor)` idempotent no-op →
  caught up to `forge_base_block_no=13` == the frozen relay tip; **0 `UnsupportedRollbackPoint` + 0
  `UnexpectedRollback`**) backs `DC-NODE-31` + `DC-NODE-32` as enforced. It is **NOT** preprod, **NOT** bounty
  completion. `RO-LIVE-01` stays operator-gated / partial; no `RO-LIVE` registry status changed this span.
- **Scope is deliberately narrow (load-bearing).** `DC-NODE-32` covers the **single-producer `run_node_sync`
  recovered-anchor rollback-to-intersection ONLY**. It does **not** add general stored-block rollback-follow on the
  single-producer path, does **not** touch the participant path (`run_participant_sync` is a **separate
  follow-on**, explicitly NOT proven here), and does **not** claim full multi-peer ChainSel convergence. Restoring
  the participant path's recover→follow (to resume N-AJ / CE-AI-6 there) is the named follow-on.
- **+2 BLUE canonical types — a BLUE-touching window.** `RecoveredAnchorPoint` + `RecoveredAnchorPointError`
  (456 → 458) in the new BLUE `ade_ledger::recovered_anchor_point` module, with the sole canonical CBOR codec. The
  rest is RED `ade_runtime` / `ade_node` shell. The new BLUE surface is a closed, version-gated record + codec; it
  does not widen any existing BLUE authority.
- **Missing/malformed/mismatched anchor record fails closed — never a silent Origin fallback.** Three new
  fail-closed `BootstrapError` variants (`RecoveredAnchorPointMissing` / `RecoveredAnchorPointDecode` /
  `RecoveredAnchorPointBindingMismatch`) make a non-Origin recovered store with a bad anchor-point record a
  deterministic halt **before** live follow starts.
- **No new CI gate; enforced by the test suite + an existing closure fence.** Both rules carry `ci_script=""`; the
  18 named tests + `ci_check_bootstrap_closure.sh` (the single-pub-fn fence that motivated placing the load in a
  separate module) are the mechanical enforcement.
- **CODEMAP + SEAMS + TRACEABILITY refresh owed this close — now TWO clusters behind.** All three remain pinned at
  the N-AI close `5ec841c8` and do not yet carry the N-AJ rules/module/gates **or** the N-AK rules + the two new
  modules. The registry holds all four new rules + the `T-REC-05` strengthening authoritatively at HEAD (358 rules)
  in the interim. Regenerating CODEMAP + SEAMS + TRACEABILITY to `b4c0983d` (folding in the pending N-AJ refresh) is
  the named follow-on (surfaced in §2 and §5).
- **One in-span commit is prior-window close work.** `bbdc3585` (`Close PHASE4-N-AJ …`) is docs/registry/config
  only (0 code) — it committed the N-AJ close artifacts the previous regen ran against as uncommitted working-tree
  (registry 354→356, the `DC-NODE-30`/`DC-EVIDENCE-03` flips, the baseline bump). It sits inside `b1bed361..HEAD`
  and is recorded in §1/§0 for completeness, but it is **not** PHASE4-N-AK work.

## Working tree at HEAD `b4c0983d` (close in progress)

**There are UNCOMMITTED working-tree changes at this regen** — the N-AK close artifacts: registry status flips
(`DC-NODE-31` → enforced, `DC-NODE-32` → enforced, `T-REC-05` strengthened `+= PHASE4-N-AK`), slice-doc `Merged`
flips, the cluster-doc archive move to `docs/clusters/completed/PHASE4-N-AK/`, and this HEAD_DELTAS refresh. §1
narrates the **committed** span `b1bed361..b4c0983d` verbatim; §0/§7 read rule **status** from the **current
working-tree** registry (so the prose reflects `DC-NODE-31` enforced / `DC-NODE-32` enforced / `T-REC-05`
strengthened). The remaining close-pass actions are (1) committing the close artifacts, (2) the CODEMAP + SEAMS +
TRACEABILITY refresh to `b4c0983d` (surfaced in §2/§5, folding in the pending N-AJ refresh), and (3) the baseline
bump (`b1bed361 → b4c0983d`) — all separate post-close steps; **this regen does not touch `.idd-config.json`
`head_deltas_baseline`.**

> **Cluster-context note.** PHASE4-N-AK closes with AK-S2 (`b4c0983d`) as the last slice — the final rule flips
> (`DC-NODE-31 → enforced`, `DC-NODE-32 → enforced`, `T-REC-05` strengthened) are carried by the in-progress
> working-tree close, alongside moving the cluster docs to `docs/clusters/completed/PHASE4-N-AK/`.

---

## Historical — PHASE4-N-AJ Participant-path convergence evidence emission (`e99a86c7 → b1bed361`)

> The section below is the **previous** HEAD_DELTAS lead, preserved in condensed form. It narrated the
> `e99a86c7 → b1bed361` span (measured from the PHASE4-N-AI close `e99a86c7`): the **N-AI baseline-bump chore**
> (`c1f4c876`) + **one unrelated docs commit** (`c95e2592`, a C2-guide sync) + the **PHASE4-N-AJ cluster** —
> Participant-path convergence evidence emission, the CE-AI-6 bridge. **9 commits, 19 files, +1813 / −35.**
> **EVIDENCE-ONLY — ZERO BLUE change, 460 canonical types unchanged** (the first window since G-N not to touch
> BLUE; counted by the old whole-tree metric). It took the EXISTING N-AI single-best-peer rollback-follow receive
> path and added a **deterministic GREEN evidence side-output** — emitting the EXISTING closed `AgreementVerdict`
> vocabulary (`block_received` / `block_admitted` / `agreement_verdict` via `verdict::derive`) to a dedicated
> `--convergence-evidence-path` JSONL sink (the new GREEN/RED module `ade_node::convergence_evidence` —
> `ConvergenceEvidenceSink` over `Box<dyn Write>` with exactly 3 emit methods + NO raw-writer accessor;
> `EvidenceEmitResult { Written | Disabled | FailedAndPoisoned }`). CI gates **157 → 159** (+2:
> `ci_check_convergence_evidence_{vocabulary_closed,emit_only}.sh`; the schema gate reused from N-AI AI-S5; 0
> modified, 0 removed). Registry **354 → 356** (+2: `DC-NODE-30` enforced + `DC-EVIDENCE-03` enforced_scaffolding;
> `DC-ADMIT-04` strengthened; **`CN-CONS-03` NOT flipped** — stays `declared`, single-best-peer scope; 0 removed).
> Headline (honest boundary): the live `--mode node --participant-venue` rollback-follow path now emits
> convergence EVIDENCE — **NOT authority** (never gates admission, never triggers a rollback, never influences
> fork-choice, never mutates the durable chain; `pump_block` stays the sole roll-forward admit). `DC-EVIDENCE-03`
> (the convergence-through-reorg transcript shape) is **vacuous-until-committed** — the operator-produced
> transcript is not yet committed. **NO `RO-LIVE` flip.** *(The N-AJ close artifacts — registry flips, HEAD_DELTAS
> refresh, archive, baseline bump — were committed by `bbdc3585`, the first commit of the SUCCEEDING N-AK window.)*
> The full §§0–7 narrative is recoverable from this doc's git history at `b1bed361`.

---

## Historical — PHASE4-N-AI live fork-choice rollback-follow wiring (`8e2c3672 → 5ec841c8` / close `e99a86c7`)

> Preserved as a pointer. It narrated the `8e2c3672 → 5ec841c8` span: the **N-AH baseline-bump chore**
> (`c66fa9a9`) + the **PHASE4-N-AI cluster** (live fork-choice rollback-follow wiring of the EXISTING
> `chain_selector` → BLUE `select_best_chain` into the live `--mode node` receive path — single-best-peer FOLLOW,
> NOT full ChainSel; `DC-NODE-23`…`DC-NODE-29`; close `5ec841c8`, docs/baseline `e99a86c7`) + one unrelated docs
> commit (`cbad2ae3`). **26 commits, 46 files, +5350 / −53.** **FIRST BLUE delta since G-N: +2 canonical types**
> (`458 → 460` by the old whole-tree metric — the `ade_ledger::wal::event::{RollbackPoint, RollbackReason}` payload
> types of the new closed-sum `WalEntry::RollBack` durable MARKER). CI gates **148 → 157** (+9; 0 modified, 0
> removed). Registry **347 → 354** (+7: `DC-NODE-23..29` enforced; `CN-CONS-01` flipped partial→enforced; 13
> strengthenings; 0 removed). Headline (honest boundary): Ade follows ONE peer's chain-sync `RollBackward` reorg
> end-to-end on a declared Participant venue — replay-equivalently and fail-closed. **Single-best-peer
> rollback-FOLLOW, NOT full multi-peer Cardano ChainSel.** **`CN-CONS-03` was NOT flipped.** The per-cluster
> security review found **H-1** (mixed peer/local rollback target → durable-chain truncation) → remediated by
> **AI-S6 / `DC-NODE-29`** (durable stored point as sole authority, validated pre-mutation, fail-closed) →
> re-review **H-1 CLOSED.** **NO `RO-LIVE` flip.** The full §§0–7 narrative is recoverable from this doc's git
> history at `5ec841c8` / `e99a86c7`.

---

## Historical — PHASE4-N-AG superseded + PHASE4-N-AH local-tip forge-base authority (`f87d0056 → 5858288e`)

> Preserved as a pointer. It narrated the `f87d0056 → 5858288e` span: the **PHASE4-N-AF close tail** (`600581e8`
> + `2d99cdf2`) + the **PHASE4-N-AG cluster** (single-producer loop-continuation-after-feed-EOF, `DC-NODE-19`;
> **superseded-close**) + the **PHASE4-N-AH cluster** (local selected durable chain forge-base authority
> `DC-NODE-20` + cert evidence-only `DC-NODE-21` + single-producer warm-start re-entry `DC-NODE-22`). **32 commits,
> 48 files, +5155 / −743.** **RED/GREEN-only — ZERO BLUE change, 458 → 458 (old metric).** CI gates **143 → 148**
> (+5; 3 modified in place; 0 removed). Registry **343 → 347** (+4; 9 strengthenings; 0 removed). Headline (honest
> boundary): Ade sustained **cert-free single-producer block production on C2-LOCAL** (`cardano-testnet` magic 42)
> against a real Haskell relay (`cardano-node 11.0.1`) — forged on its OWN local durable `ChainDb::tip`, crossed a
> follow-link EOF, settled `> k` immutable, and resumed forging after a hard restart (run-4). NOT preprod. NOT
> bounty completion. No `RO-LIVE` flip. The full §§0–7 narrative is recoverable from this doc's git history at
> `5858288e`.

---

## Historical — PHASE4-N-AF single-producer extend-own-durable-spine (`6363683e → f87d0056`)

> Preserved as a pointer. A **single-slice cluster lead** narrating the `6363683e → f87d0056` span: the
> PHASE4-N-AE.F close grounding-doc refresh (`d3f52e7c`) + a C2-guide doc (`1302417d`) followed by the **OQ-1 /
> DC-NODE-17 investigation** (`bd1a7a73` declared DC-NODE-17 → `dadf4743` live-disproved it as the fix) and the
> **PHASE4-N-AF cluster** (single slice AF.S1 — `DC-NODE-18`, single-producer extend-own-durable-spine). Counts
> at `f87d0056`: 343 rules, 143 CI gates, 458 canonical types (old metric). **GREEN+RED only — BLUE 458 → 458.**
> New gate `ci_check_single_producer_extend_own_spine.sh`. No `RO-LIVE` flip. The full §§0–7 narrative is
> recoverable from this doc's git history at `f87d0056`.

---

## Historical — PHASE4-N-AE.F post-CE-A5 echo-idempotency follow-up (`a76672b9 → 6363683e`)

> Preserved as a pointer. A **single-slice lead** narrating the `a76672b9 → 6363683e` span: the PHASE4-N-AE
> close grounding-doc refresh (`62811a4e`) followed by the **PHASE4-N-AE.F** slice (`DC-NODE-16` receive
> idempotency at the durable-admit chokepoint — a re-announced block Ade already durably holds (same hash, same
> slot) is an idempotent no-op at `pump_block`). Counts at `6363683e`: 341 rules, 142 CI gates, 458 canonical types
> (old metric). **RED chokepoint only — BLUE 458 → 458.** New gate `ci_check_receive_idempotency.sh`. No `RO-LIVE`
> flip. The full §§0–7 narrative is recoverable from this doc's git history at `6363683e`.

---

## Historical — earlier windows (`25ddeebd → a76672b9` and before)

> Preserved as pointers. The **PHASE4-N-AD/N-AE CE-A5 window** (`25ddeebd → a76672b9`, recover→serve continuity +
> forge-on-followed-tip admissibility — the CE-A5 manifest: a real `cardano-node 11.0.1` relay
> `AddedToCurrentChain` an Ade-forged successor block; `DC-NODE-14`/`DC-NODE-15`/`DC-CONS-24`/`DC-PROTO-10`; 336 →
> 340 rules at `a76672b9`); the **PHASE4-N-AC cluster** (KES signing evolves the operator key to the current period
> — `DC-CRYPTO-10`; 335 → 336 rules); the **PHASE4-N-AB cluster** (outbound mux segmentation — `CN-SESS-05`; 334 →
> 335 rules); the **PHASE4-N-AA cluster** (bounded peer-driven serve range — `DC-SERVEMEM-01`; 333 → 334 rules);
> the **PHASE4-N-U cluster + gate-hygiene tail** (forged-block durability — `DC-NODE-12`/`DC-CONS-23`/`DC-WAL-04`/
> `T-REC-05`/`DC-NODE-13`; 328 → 333 rules); and the **G-K…G-R + C1 multi-cluster catch-up** (`550eec3a →
> 65954fa3`, 319 → 328 rules, 126 → 134 CI gates). The full §§0–7 narrative for each is recoverable from this
> doc's git history at the respective HEADs.

---

## Generation notes

### Regen `b1bed361 → b4c0983d` (PHASE4-N-AK recovered-anchor live-follow start + rollback boundary — current lead)

- **Baseline valid; one single-theme cluster + the prior-window close commit.** Run against `b1bed361` (the
  PHASE4-N-AJ AJ-S3 close, the prior HEAD_DELTAS HEAD), which `git rev-parse` resolves and `git merge-base
  b1bed361 HEAD` confirms is a strict ancestor of HEAD `b4c0983d` (`b1bed361` carries no tag). The start-of-regen
  config baseline was already `b1bed361` (bumped by the previous close's `bbdc3585`, which is the **first** commit
  in this span). The operator bumps `head_deltas_baseline` `b1bed361 → b4c0983d` as a **separate post-close step**
  (NOT performed by this regen).
- **Counts are mechanical (git/grep/ls):** commit log + `--shortstat` over `b1bed361..HEAD` (**7** commits, no
  merges / **33** files / **+2647 / −544**); CI gate count via `git ls-tree -r --name-only <ref> ci/ | grep -c
  ci_check_.*\.sh` at each ref (**159 → 159**; `--diff-filter=A/M/D` over `ci/` all **empty**); registry rule count
  via `grep -cP '^id = "'` at each ref (**356 → 358**; `comm`/`diff` of the sorted `id =` lists shows exactly the
  two additions `DC-NODE-31` + `DC-NODE-32`, zero removals); registry status via `grep -oP '^status = "\K[^"]+' |
  sort | uniq -c` at each ref (**221 → 224 enforced**, **116 → 114 declared**, `enforced_scaffolding=1` + `partial=19`
  unchanged); strengthening = **1** (`strengthened_in += "PHASE4-N-AK"` appears once, on `T-REC-05`); BLUE
  canonical types **456 → 458** (the `git diff b1bed361..HEAD` over the BLUE `core_paths` trees adds exactly two
  `^+(pub )?(struct|enum)` lines in the new `recovered_anchor_point.rs`).
- **BLUE count metric note.** This regen counts BLUE canonical types as `pub struct`/`pub enum` declarations over
  the configured BLUE `core_paths` trees (`ade_ledger` / `ade_codec` / `ade_types` / `ade_crypto` / `ade_plutus` /
  `ade_core` / the BLUE `ade_network` submodules) — **456 → 458** here. Prior windows in this doc reported a
  whole-tree "458/460 canonical types" figure (a coarser count); the historical pointers retain that number with an
  "(old metric)" note. The **delta** (+2) is the load-bearing fact and is metric-independent: exactly two new BLUE
  `pub struct`/`pub enum` (`RecoveredAnchorPoint`, `RecoveredAnchorPointError`), zero removed.
- **STATUS read from the CURRENT working tree (load-bearing).** There are **uncommitted** N-AK close artifacts at
  this regen (registry status flips, slice-doc `Merged` flips, the cluster-doc archive, this HEAD_DELTAS refresh).
  §1 narrates the **committed** span `b1bed361..b4c0983d` verbatim; §0/§7 read rule **status** from the **current
  working-tree** `docs/ade-invariant-registry.toml` so the prose reflects the close state (`DC-NODE-31` enforced,
  `DC-NODE-32` enforced, `T-REC-05` strengthened). The registry-count deltas above were verified against the
  current working-tree registry (358 rules); the baseline-side counts via `git show
  b1bed361:docs/ade-invariant-registry.toml` (356 rules).
- **BLUE change this span (NOT evidence-only).** Unlike the prior N-AJ window, `git diff b1bed361..HEAD` over the
  BLUE `core_paths` trees lists the new file `crates/ade_ledger/src/recovered_anchor_point.rs` and adds two BLUE
  `pub struct`/`pub enum` (`RecoveredAnchorPoint`, `RecoveredAnchorPointError`). `git diff --name-only …
  '**/Cargo.toml' 'Cargo.toml'` is empty (no manifest/feature-flag delta; no new CLI flag — the new behavior is a
  typed struct field defaulting to `None`).
- **Two new modules — both new `.rs` are library modules, not tests.** `git diff --diff-filter=A --name-only …
  'crates/**/*.rs'` lists exactly two new `.rs`: `crates/ade_ledger/src/recovered_anchor_point.rs` (BLUE) and
  `crates/ade_runtime/src/recovered_anchor.rs` (RED), each registered in its crate's `lib.rs`. No new crate /
  `Cargo.toml` / workspace — still 11 crates.
- **Registry delta is +2 rules + 1 strengthening, NOT a removal.** `DC-NODE-31` declared at the cluster doc
  (`c8e44386`) then flipped to enforced at close; `DC-NODE-32` declared at the AK-S2 authority doc (`f14dee20`)
  then flipped to enforced at close; `T-REC-05` gained `strengthened_in += PHASE4-N-AK`. The sorted-id `comm`
  confirms zero removals. The +3-enforced / −2-declared status tally additionally reflects the `DC-NODE-30 →
  enforced` + `DC-EVIDENCE-03 → enforced_scaffolding` flips carried by the in-span **N-AJ close commit**
  (`bbdc3585`).
- **No new CI gate — enforced by tests + an existing closure fence.** Both new rules carry `ci_script=""`. They are
  enforced by the unit/integration suite (11 named tests for `DC-NODE-31`, 7 for `DC-NODE-32`) plus the EXISTING
  `ci_check_bootstrap_closure.sh` single-`pub fn` fence — which is **why** the load was placed in a separate
  `ade_runtime::recovered_anchor` module (to keep `bootstrap.rs`'s single-pub-fn closure intact, preserving
  `CN-NODE-01`). No `--diff-filter=A/M/D` change over `ci/`.
- **Classification note (TCB).** The new `ade_ledger::recovered_anchor_point` is **BLUE** (the closed record + sole
  canonical codec; `ade_ledger` is the first BLUE `core_paths` entry). The new `ade_runtime::recovered_anchor` is
  **RED** for the `SnapshotStore` read I/O but the decode (via the sole codec) + the `anchor_fp` binding check are
  BLUE-authoritative. `ade_runtime::bootstrap`'s `resolve_live_follow_start` is a pure/total BLUE resolver inside
  the RED shell crate; the `chaindb` store surface + `node_lifecycle`/`node_sync` wiring are RED/GREEN shell.
- **No `RO-LIVE` flip; CE-AK-3 is enforced-backing evidence.** `DC-NODE-31` + `DC-NODE-32` are recorded `enforced`
  (hermetic CEs + the live CE-AK-3 end-to-end pass at the frozen c2-relay venue: 0 `UnsupportedRollbackPoint` + 0
  `UnexpectedRollback`, caught up to relay tip 13). This is **NOT** a bounty/preprod claim. No `RO-LIVE` registry
  status changed (`RO-LIVE-01` stays operator-gated / partial).
- **Normative docs unchanged this span.** `git diff --name-only b1bed361..HEAD` over the configured `normative_docs`
  (CE-79 statement + addendum, the three contract docs, CE-73 reclassification, `CLAUDE.md`) is empty — the §7
  delta is entirely the invariant-registry change.
- **§1 commit log verbatim from `git log` (newest first).** The per-slice synthesis is in §0/§3. Six subjects carry
  a conventional-commits prefix; the seventh (`bbdc3585`, `Close PHASE4-N-AJ …`) is the prior-window **close
  commit** (no prefix, per the project's close-commit convention) and is **N-AJ work, not N-AK** (docs/registry/
  config only, 0 code), folded into the span by commit range.
- **Doc-refresh state — CODEMAP + SEAMS + TRACEABILITY now TWO clusters STALE (refresh owed).** All three remain
  pinned at the N-AI close `5ec841c8` (`grep -c DC-NODE-31 / DC-NODE-32 / RecoveredAnchorPoint` in each = 0; they
  also still lack the N-AJ `DC-NODE-30` / `DC-EVIDENCE-03` / `convergence_evidence` module). **Cross-reference
  warnings surfaced in §2 (two new modules not in CODEMAP) and §5 (new rules not in TRACEABILITY; note that both
  carry `ci_script=""`, so TRACEABILITY's enforcement column is the test suite).** Regenerate CODEMAP + SEAMS +
  TRACEABILITY to `b4c0983d` as a follow-on this close (folding in the pending N-AJ refresh); the registry holds the
  four new rules + the `T-REC-05` strengthening authoritatively in the interim (358 rules). No orphan gate (no gate
  was added).
- **Working tree NOT clean.** This regen runs with the N-AK close artifacts **uncommitted** (registry status flips
  + slice-doc `Merged` flips + cluster-doc archive + this HEAD_DELTAS refresh). The remaining close-pass actions are
  committing the close artifacts, the CODEMAP + SEAMS + TRACEABILITY refresh, and the baseline bump (`b1bed361 →
  b4c0983d`) — all separate post-close steps; this regen does **not** touch `.idd-config.json`
  `head_deltas_baseline`.
