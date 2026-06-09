# Ade — HEAD Deltas (Changes Since Baseline)

> **Status:** Living architectural document. Regenerated; not hand-edited.
>
> Regenerate with `/head-deltas <baseline>` after every cluster close. Baseline is recorded in `.idd-config.json` `head_deltas_baseline`.

> Baseline: `8e2c3672` (PHASE4-N-AH close — DC-NODE-20/21/22 enforced; local-tip forge-base authority, 2026-06-08 21:58)
> HEAD: `5ec841c8` (PHASE4-N-AI AI-S6 — rollback-target slot/hash canonical binding, H-1 remediation, 2026-06-09 16:26)
> Span: **the PHASE4-N-AI cluster — live fork-choice rollback-follow wiring of the EXISTING `ade_runtime::consensus::chain_selector` orchestrator + BLUE `select_best_chain` into the live `--mode node` receive path (single-best-peer FOLLOW, NOT full ChainSel; `DC-NODE-23`…`DC-NODE-29`)** — preceded by the N-AH baseline-bump chore and folding **one unrelated docs commit** (a preprod pool-registration evidence manifest).
> **26 commits** (no merges), **46 files changed, +5350 / −53 lines**. **This span CHANGES BLUE — the FIRST BLUE delta since the G-N span: +2 canonical types** (`458 → 460`, verified mechanically: `git grep -hE '^(pub )?(struct|enum) '` over the BLUE `core_paths` trees is `458 → 460`; the +2 are the NEW `ade_ledger::wal::event::{RollbackPoint, RollbackReason}` payload types of the new closed-sum variant `WalEntry::RollBack`; `ade_ledger` `177 → 179`; `git diff 8e2c3672..HEAD -- '**/Cargo.toml' 'Cargo.toml'` empty — still **11 crates**). The span touches exactly **three** BLUE files — `ade_ledger::wal::{event, replay}` (the new version-gated durable MARKER + replay arm) + `ade_ledger::wal::{error, mod, store_trait}` (closed error + accessor plumbing) + `ade_core::consensus::fork_choice` (**production UNCHANGED — `select_best_chain` byte-identical; the only hunk is `@@ -340,4 +340,89 @@ mod tests`, a `#[cfg(test)]`-only arrival-order-permutation determinism proof for `CN-CONS-01`**). Everything else is RED/GREEN on the `ade_node` + `ade_runtime` shells.

> **Baseline note (load-bearing — read before §0).** This window's baseline is **`8e2c3672`**, the
> PHASE4-N-AH close (the prior HEAD_DELTAS HEAD), and it is **valid**: `git rev-parse 8e2c3672` resolves and
> `git merge-base 8e2c3672 HEAD == 8e2c3672` (it is a strict ancestor of HEAD; `8e2c3672` carries no tag).
> HEAD is **`5ec841c8`** (the PHASE4-N-AI AI-S6 H-1 remediation — the last cluster commit). The config
> baseline at the start of this regen was already `8e2c3672` (the previous close bumped it), so the window
> measures cleanly from the recorded baseline forward. The span has **three parts**: (1) the **N-AH
> baseline-bump chore** — `c66fa9a9` (`chore(idd): bump head_deltas_baseline to the PHASE4-N-AH close`),
> config-only, zero code, zero rule; (2) the **PHASE4-N-AI cluster** (`DC-NODE-23`…`DC-NODE-29`) — invariants
> sketch (`d92f9ce8`) + OQ-1 resolution (`80862c7f`) + cluster/slice docs + the eight slices AI-S1…AI-S6 +
> close (`5ec841c8`); and (3) **one unrelated docs commit** — `cbad2ae3` (`docs(evidence): add preprod ADE1
> pool registration manifest`), docs-only, folded into the span but **not** part of N-AI. The closer bumps
> `head_deltas_baseline` `8e2c3672 → <close SHA>` after this regen (a separate post-close step) so the next
> cluster measures from here.

This window is **led by PHASE4-N-AI — live fork-choice rollback-follow wiring (single-best-peer FOLLOW).** It
takes the EXISTING enforced BLUE fork-choice / rollback core (`select_best_chain` `DC-CONS-03`,
`materialize_rolled_back_state` `CN-STORE-07`, the lockstep `receive::reducer` `DC-CONS-20`, `pump_block`
`DC-NODE-05/12`) and **wires it into the live `--mode node` receive path** so that, on a declared Participant
venue, Ade **follows ONE peer's chain-sync `RollBackward` reorg end-to-end** — durable-point lookup →
`ChainEvent::RolledBack` → materialize + lockstep + a durable `WalEntry::RollBack` marker + `pump_block`
roll-forward — replay-equivalently. The headline must be read with its **boundary intact**:

> **PHASE4-N-AI wired single-best-peer rollback-FOLLOW into the live receive path: Ade detects a peer-origin
> non-spine candidate ONCE (venue-blind), and on a Participant venue follows that ONE peer's `RollBackward`
> reorg through the EXISTING `chain_selector` → BLUE `select_best_chain` durable-apply path, recording the
> rollback as a version-gated `WalEntry::RollBack` durable marker that re-invokes the existing rollback
> authority on replay. This is NOT full multi-peer Cardano ChainSel. `CN-CONS-03` was NOT flipped to enforced
> (it stays `declared`, strengthened only — only the single-best-peer venue was exercised). `pump_block` stays
> the SOLE roll-forward durable admit; the SingleProducer venue stays fail-closed unchanged
> (`DC-NODE-20`/`DC-NODE-24` `RefuseSingleProducer`). There is NO `RO-LIVE` flip — CE-AI-6 is the
> operator-gated convergence transcript, vacuous-until-committed.**

The arc to that result is load-bearing — the rollback-durability foundation lands and is proven **first**,
then the detector, then the apply driver, then the wire-signal preservation, then the go-live flip, then the
convergence evidence, then a cluster-close security remediation:

- **PHASE4-N-AI / AI-S1 / `DC-NODE-27` (rollback durability foundation — the OQ-1 mechanism, BLUE).** OQ-1
  ("how is a live rollback made replay-equivalent?") **resolved → A**: a version-gated additive `WalEntry::RollBack
  { to_point, reason, prior_tip, selected_tip }` at the **reserved RollBackward tag 1** + a `replay_from_anchor`
  `RollBack` arm that **re-invokes** the existing `materialize_rolled_back_state` (`CN-STORE-07`) + lockstep
  reducer (`DC-CONS-20`) and re-anchors the fp chain to `to_point`'s in-chain `post_fp` — replacing today's
  `ChainBreak` with a faithful linear-with-rollbacks replay. **It is a durable MARKER, NOT a second rollback
  implementation.** Two new BLUE canonical payload types ship with it: `RollbackPoint` (slot + hash + block_no)
  and the closed `RollbackReason` (uint wire code; rung-1 sole variant `PeerRollBackward`). Append-only
  preserved. Option B (WAL-tail reconciliation) was **rejected**. New gate
  `ci_check_wal_rollback_replay_equiv.sh` (CE-AI-1). *(Without this, a live rollback either `ChainBreak`s on
  restart or resurrects the abandoned branch.)* `DC-NODE-27 → enforced` at close.
- **PHASE4-N-AI / AI-S2 / `DC-NODE-23` + `DC-NODE-24` (shared detector + venue-split resolver — GREEN).** A
  pure total venue-blind classifier `(durable_tip, candidate_header_summary) → ReceiveDisposition { AlreadyHave
  | LinearExtend | RefuseSingleProducer | NeedsForkChoice }` (`DC-NODE-23`) — an already-known peer echo is
  `AlreadyHave`, **never** a competing candidate merely because it is not a fresh extension; it observes no
  venue, no wall-clock, no network state, and **never** calls `select_best_chain`. The venue-split resolver
  (`DC-NODE-24`) is **total over the closed venue set**: `VenueRole::SingleProducer ⇒ fail closed` (the
  `DC-NODE-20` rung-1 behavior, byte-unchanged); `VenueRole::Participant ⇒ NeedsForkChoice ⇒` the EXISTING
  `chain_selector` orchestrator → BLUE `select_best_chain` (`DC-CONS-03`); an undeclared/unknown venue takes the
  conservative `SingleProducer` refuse arm (**no silent inference** either direction). New gate
  `ci_check_receive_detector_venue_split.sh` (CE-AI-2). `DC-NODE-23/24 → enforced` at close.
- **PHASE4-N-AI / AI-S3 / `DC-NODE-25` + `DC-NODE-26` (live apply driver + reconciliation — RED+GREEN).** A
  `ChainSelected`/`RolledBack` outcome from the orchestrator is applied to the durable stores **ONLY** via the
  already-enforced authorities (`materialize_rolled_back_state` `CN-STORE-07` + lockstep `roll_backward`
  `DC-CONS-20` + `WalEntry::RollBack` append (S1) + `pump_block` roll-forward `DC-NODE-05/12`) — **no second
  apply path, no second tip-advance, no second rollback-materialize** (`DC-NODE-25`). A fork-choice win is
  **provisional**: durably adopted **only** when its BODIES validate and apply through `pump_block` (no
  header-only tip advance); a `TiebreakerLossKeepCurrent` outcome makes no durable change. After **every** applied
  decision, the orchestrator's `selector.current_tip == ChainDb::tip` (and chain_dep == durable
  `PraosChainDepState`) — the in-memory decision state never diverges from the persisted authority (`DC-NODE-26`).
  New gates `ci_check_live_fork_choice_apply.sh` + `ci_check_live_fork_choice_wiring.sh` (CE-AI-3). `DC-NODE-25/26
  → enforced` at close.
- **PHASE4-N-AI / AI-S4a / `DC-PUMP-01` (strengthened) (wire rollback signal preservation — RED).** The
  admission wire pump now **preserves** the peer's chain-sync `RollBackward` **point** as a closed
  `AdmissionPeerEvent::RollBackward { peer, point, tip }` variant — **previously the point was discarded and a
  `TipUpdate` emitted only**; "a rollback is NEVER represented as a `TipUpdate` only." A `RollBackward(Origin)`
  (rollback-to-genesis) is **unsupported for single-best-peer within k** and **fails closed**
  (`UnsupportedRollbackPoint` — drop the peer). `Block`/`TipUpdate`/`Disconnected` unchanged. The event **merges
  latent** — the live loop does not consume it until AI-S4b-ii. This is wire-signal preservation, **NOT**
  fork-choice wiring. New gate `ci_check_wire_rollback_signal_preserved.sh`; `DC-PUMP-01` gains
  `strengthened_in += PHASE4-N-AI`.
- **PHASE4-N-AI / AI-S4b-i / `DC-NODE-28` (Participant venue declaration — RED, inert).** An explicit, closed
  venue declaration (`--participant-venue → VenueRole::Participant`, mutually exclusive with
  `--single-producer-venue` → `CliError::ConflictingVenue`); `Unknown`/absent stays the conservative
  non-fork-choice path; **no silent inference** either direction. **Truly inert** — it recognizes the mode value
  only: it does NOT route to fork-choice, does NOT change any forge decision, does NOT alter `SingleProducer`
  behavior. New gate `ci_check_participant_venue_inert.sh` (CE-AI-2 venue precursor / OQ-5). Merges with **no
  live behavior flip**.
- **PHASE4-N-AI / AI-S4b-ii / `DC-NODE-28` (enforced) (live rollback-follow routing + forge gate — RED, THE
  GO-LIVE FLIP).** The live receive loop now classifies every block (S2 detector + venue resolver). **RollBack
  path (Participant):** `RollBackward(point)` → verify `point` is in the durable ChainDb → compute
  `to_block_no`+`depth` from the durable chain → construct `ChainEvent::RolledBack` → `apply_chain_event` (S3:
  materialize + lockstep + `WalEntry::RollBack`, replay-equivalent). A point not in the durable chain /
  beyond-k / crossing the immutable tip **fails closed** (no fabricated block_no). **This is NOT
  `process_stream_input`** — the orchestrator's in-memory rollback ring is header-arrival-populated and empty on
  the live block/pump follow path; the loop never calls `select_best_chain` (`DC-CONS-03` honored). **Block
  path:** `AlreadyHave → drop`, `LinearExtend → pump_block`, a bare competing block (non-linear `RollForward`
  without a prior `RollBackward`) → **fail closed (all venues)** (no safe fork point); `SingleProducer`/`Unknown`
  keep the `DC-NODE-20`/existing fail-closed. **Forge gate (`DC-NODE-28`, lands together — irreducible):** while
  a rollback/apply is pending, forging refuses (`ForgeRefused::ReselectionPending`) — **never forges on the stale
  pre-resolution tip**. New gate `ci_check_live_fork_choice_wiring.sh` (CE-AI-3 live + CE-AI-4). **Honesty:**
  this proves single-best-peer rollback *following* (replay-equivalent peer-branch adoption), **NOT**
  multi-candidate live `select_best_chain` selection.
- **PHASE4-N-AI / AI-S5 / `CN-CONS-01` (flipped) (convergence evidence + operator pass — RED + hermetic).**
  **Hermetic (CE-AI-5, `CN-CONS-01`):** for a fixed competing-candidate set, `select_best_chain` converges on
  the fork-choice-maximal chain regardless of arrival order — a `#[cfg(test)]` permutation proof over
  `fork_choice` (distinct-heights + tiebreaker). New gate
  `ci_check_chain_selection_arrival_order_independent.sh`. **Operator-gated (CE-AI-6, `CN-CONS-03`):** a closed
  derived-tier evidence vocabulary for a committed transcript
  (`docs/active/phase4-n-ai-convergence-runbook.md` + a `convergence-pass.{md,jsonl}`) where Ade + ≥1 Haskell
  producer on a competing-producer venue converge on the same tip; new schema gate
  `ci_check_convergence_evidence_schema.sh` (**vacuous-until-committed, sha256-bound**). `CN-CONS-01 →
  enforced`; **`CN-CONS-03` NOT flipped** — its statement is broad and CE-AI-6 is operator-gated, so it stays
  `declared`, strengthened only.
- **PHASE4-N-AI / AI-S6 / `DC-NODE-29` (NEW, enforced) (rollback-target slot/hash canonical binding — H-1
  security remediation, found at cluster-close).** The per-cluster security review surfaced **H-1**: the live
  Participant `RollBackward` path could build a rollback target from **mixed peer/local authority**
  (peer-supplied slot + locally-verified hash), which a malicious peer could exploit to truncate the durable
  chain and brick the node. `DC-NODE-29` makes the rollback target resolve against the durable `ChainDb` and use
  the **stored chain point (stored slot + hash) as the SOLE authority**: the peer-supplied slot MUST equal the
  stored slot for that hash; on **any** mismatch (or unknown hash, or Origin) the path **fails closed with a
  typed error BEFORE `commit_rollback`, BEFORE `WalEntry::RollBack`, BEFORE any ChainDb/LedgerState/PraosChainDepState
  mutation**. New variants `NodeSyncError::{UnexpectedRollback, RollbackPointSlotMismatch}`. Reconciliation
  (`DC-NODE-26`) remains the post-apply backstop but is **NOT** the only defense. New gate
  `ci_check_rollback_target_canonical_binding.sh`. Security re-review: **H-1 CLOSED, no new findings.**

**+2 BLUE canonical types** (`RollbackPoint` + `RollbackReason` — the first BLUE delta since G-N), one new
closed BLUE WAL variant (`WalEntry::RollBack`), and one new closed BLUE error (`WalError::RollbackTargetNotInChain`).
**No `RO-LIVE` rule flipped** this span — `RO-LIVE-01` stays operator-gated; CE-AI-6 is the convergence
transcript, vacuous-until-committed.

## 0. Headline

| Count | Baseline (`8e2c3672`) | HEAD (`5ec841c8`) | Δ |
|---|---|---|---|
| CI gates (`ci/ci_check_*.sh`) | 148 | **157** | **+9** — **nine NEW gates** (`--diff-filter=A` over `ci/`): `ci_check_wal_rollback_replay_equiv.sh` (AI-S1, `DC-NODE-27`), `ci_check_receive_detector_venue_split.sh` (AI-S2, `DC-NODE-23/24`), `ci_check_live_fork_choice_apply.sh` (AI-S3, `DC-NODE-25/26`), `ci_check_live_fork_choice_wiring.sh` (AI-S3/S4b-ii, `DC-NODE-25/26/28`), `ci_check_wire_rollback_signal_preserved.sh` (AI-S4a, `DC-PUMP-01`), `ci_check_participant_venue_inert.sh` (AI-S4b-i, `DC-NODE-28`), `ci_check_chain_selection_arrival_order_independent.sh` (AI-S5, `CN-CONS-01`), `ci_check_convergence_evidence_schema.sh` (AI-S5, CE-AI-6), `ci_check_rollback_target_canonical_binding.sh` (AI-S6, `DC-NODE-29`). **Zero gates modified in place; zero removed** (`--diff-filter=M` / `--diff-filter=D` over `ci/` empty). |
| Registry rules (`docs/ade-invariant-registry.toml`) | 347 | **354** | **+7** — seven NEW rules `DC-NODE-23`…`DC-NODE-29`, all `enforced`. **Zero removed** (`diff` of the sorted `id =` lists shows exactly the seven `DC-NODE-23..29` additions and no removal). |
| Registry status (enforced / partial / declared) | 213 / 20 / 114 | **221 / 19 / 114** | **+8 enforced** (the 7 new `DC-NODE-23..29` + **`CN-CONS-01` flipped `partial → enforced`**), **−1 partial** (`CN-CONS-01`). Declared **unchanged** (114). |
| Registry strengthenings | — | **13** | **`strengthened_in += "PHASE4-N-AI"`** on **13** existing rules: `CN-CONS-01`, **`CN-CONS-03`** (strengthened **but NOT flipped** — stays `declared`), `CN-STORE-07`, `DC-CONS-03`, `DC-CONS-05`, `DC-CONS-06`, `DC-CONS-20`, `DC-NODE-05`, `DC-NODE-12`, `DC-NODE-20`, `DC-PUMP-01`, `T-REC-03`, `T-REC-05`. No rule weakened. |
| BLUE canonical types | 458 | **460** | **+2** — **the FIRST BLUE delta since the G-N span.** `git grep -hE '^(pub )?(struct\|enum) '` over the BLUE `core_paths` trees is `458 → 460`; the +2 are `ade_ledger::wal::event::{RollbackPoint, RollbackReason}` (the payload types of the new closed-sum `WalEntry::RollBack`). `ade_ledger` `177 → 179`; `ade_core::consensus::fork_choice` production is **byte-identical** (the only hunk is a `#[cfg(test)]` permutation proof). No `Cargo.toml` changed — still 11 crates. |
| Grounding docs | CODEMAP **regenerated to `5ec841c8`** this close (460 types / 157 CI / 354 rules); SEAMS + TRACEABILITY last regenerated to **`5858288e`** (the N-AH close) | CODEMAP **current** — carries `DC-NODE-23` ×14, `DC-NODE-24` ×3, `DC-NODE-25` ×13, `DC-NODE-26` ×3, `DC-NODE-27` ×10, `DC-NODE-28` ×11, `DC-NODE-29` ×16, `DC-PUMP-01` ×13, `RollbackPoint`/`RollbackReason`/`WalEntry::RollBack`, the BLUE count `460`, and all nine new gates. **SEAMS + TRACEABILITY are one cluster STALE** — they do **not** yet carry `DC-NODE-23..29` (grep = 0). | CODEMAP cross-reference **verified, no staleness**. **SEAMS + TRACEABILITY are refresh-on-this-close items** (the registry holds the seven new rules + their gate bindings authoritatively at HEAD, 354 rules). See the cross-reference warning at the end of §5. |

> **Grounding-doc state this close (load-bearing).** **CODEMAP was regenerated to `5ec841c8`** and is current
> for the whole `8e2c3672..5ec841c8` span (it carries all seven `DC-NODE-23..29` rules, `DC-PUMP-01`, the new
> BLUE types `RollbackPoint`/`RollbackReason`/`WalEntry::RollBack`, the BLUE count `460`, and all nine new
> gates). **SEAMS + TRACEABILITY remain pinned at `5858288e`** (the N-AH close) and are **one cluster stale** —
> they do not yet carry `DC-NODE-23..29` (grep count 0). The invariant registry holds the seven new rules + all
> nine gate bindings + the thirteen strengthenings authoritatively at HEAD (**354 rules**); the SEAMS +
> TRACEABILITY refresh to `5ec841c8` is a follow-on item this close (surfaced in §5).

The slice↔rule↔gate map for this window:

| Slice | Rule(s) | Gate | What shipped |
|---|---|---|---|
| **AI-S1** (`cced0214`) | **`DC-NODE-27`** (NEW, enforced) | **`ci_check_wal_rollback_replay_equiv.sh`** (NEW) | **BLUE.** Version-gated additive `WalEntry::RollBack { to_point, reason, prior_tip, selected_tip }` (reserved tag 1) + the two BLUE payload types `RollbackPoint` / `RollbackReason` + the closed `WalError::RollbackTargetNotInChain`; `replay_from_anchor` `RollBack` arm re-invokes the existing `materialize_rolled_back_state` + lockstep reducer (a MARKER, not a reimpl). OQ-1 → A (option B rejected). **Lands + proven FIRST.** |
| **AI-S2** (`47c0f487`) | **`DC-NODE-23`** + **`DC-NODE-24`** (NEW, enforced) | **`ci_check_receive_detector_venue_split.sh`** (NEW) | **GREEN** (`ade_node::node_sync`). Pure total venue-blind detector (4 dispositions; `AlreadyHave` for a known echo; never calls `select_best_chain`) + venue-split resolver (`SingleProducer → refuse`, `Participant → NeedsForkChoice`; unknown venue fails closed). New closed types `NodeSyncItem` / `ReceiveClass` / `ReceiveDisposition` / `CandidateSummary`. |
| **AI-S3** (`7f78dd98`) | **`DC-NODE-25`** + **`DC-NODE-26`** (NEW, enforced) | **`ci_check_live_fork_choice_apply.sh`** + **`ci_check_live_fork_choice_wiring.sh`** (NEW) | **RED+GREEN** (`ade_node::node_lifecycle` apply driver). Durable apply via materialize + lockstep + `WalEntry::RollBack` + `pump_block` (no second apply path; provisional until bodies apply); `selector.current_tip == ChainDb::tip` after every decision. New types `AppliedTip` / `ApplyError`. |
| **AI-S4a** (`30b5727c`) | strengthen **`DC-PUMP-01`** | **`ci_check_wire_rollback_signal_preserved.sh`** (NEW) | **RED** (`ade_runtime::admission::wire_pump`). `AdmissionPeerEvent::RollBackward { peer, point, tip }` preserved as a closed event (was discarded → `TipUpdate` only); `Origin` fails closed `UnsupportedRollbackPoint`. Merges **latent**. |
| **AI-S4b-i** (`04f358bd`) | (`DC-NODE-28` precursor / OQ-5) | **`ci_check_participant_venue_inert.sh`** (NEW) | **RED** (`ade_node::cli` + `main` + `node_lifecycle`). `--participant-venue → VenueRole::Participant` (mutually exclusive with `--single-producer-venue` → `CliError::ConflictingVenue`); **truly inert** — recognizes the mode value only, no live behavior flip. |
| **AI-S4b-ii** (`af51b3c8`) | **`DC-NODE-28`** (NEW, enforced) | **`ci_check_live_fork_choice_wiring.sh`** (NEW; CE-AI-4 folded in) | **RED** (`ade_node::node_lifecycle` + `node_sync`). **The go-live flip.** Live loop classifies every block; Participant `RollBackward(point)` → durable-point lookup → `ChainEvent::RolledBack` → `apply_chain_event` (NOT `process_stream_input`; never calls `select_best_chain`); bare competing block fails closed (all venues); forge gate `ForgeRefused::ReselectionPending` while a decision is pending. Single-best-peer FOLLOW, **not** multi-candidate selection. |
| **AI-S5** (`a8d93ba4`) | **`CN-CONS-01`** (`partial → enforced`); `CN-CONS-03` strengthen (**NOT flipped**) | **`ci_check_chain_selection_arrival_order_independent.sh`** + **`ci_check_convergence_evidence_schema.sh`** (NEW) | **RED + hermetic.** CE-AI-5: `#[cfg(test)]` arrival-order-permutation proof for `select_best_chain` in `fork_choice` (`CN-CONS-01`). CE-AI-6: closed derived-tier convergence-evidence vocabulary + runbook (operator-gated, **vacuous-until-committed**, sha256-bound) — proves the exercised venue, NOT full multi-peer ChainSel. **Last slice.** |
| **AI-S6** (`5ec841c8`) | **`DC-NODE-29`** (NEW, enforced) | **`ci_check_rollback_target_canonical_binding.sh`** (NEW) | **RED** (`ade_node::node_sync` + `node_lifecycle`; `ade_runtime::recovery::restart` WAL-tail scan). **H-1 security remediation** (found at cluster-close). Rollback target uses the durable stored chain point (stored slot + hash) as the SOLE authority; peer slot must equal stored slot for the hash; **any** mismatch / unknown hash / Origin fails closed with a typed error **before** any mutation, WAL append, or `commit_rollback`. New errors `NodeSyncError::{UnexpectedRollback, RollbackPointSlotMismatch}`. |
| **close** (CE-AI-7) | `DC-NODE-23..29` enforced; `CN-CONS-01` partial→enforced; 13 strengthenings | — | Registry close (354 rules); CODEMAP regenerated to `5ec841c8`; SEAMS + TRACEABILITY refresh owed. |

The per-commit shape (selected — the full verbatim log is §1):

| Commit | Kind | What it did | Code / CI / registry effect |
|--------|------|-------------|-----------------------------|
| `c66fa9a9` | chore (idd) | Bump `head_deltas_baseline` to the PHASE4-N-AH close (`8e2c3672`); registry 347 | **0 code / 0 CI / 0 rule**; `.idd-config.json` only |
| `d92f9ce8` | docs (invariants) | DC-NODE-23..28 invariants sketch (declared) | **0 code / 0 CI**; registry: `DC-NODE-23..28` declared |
| `80862c7f` | docs | OQ-1 resolved → A (`WalEntry::RollBack` marker) | **0 code / 0 CI / 0 rule** |
| `cced0214` | feat (AI-S1) | Rollback WAL durability foundation (`WalEntry::RollBack`) | **BLUE code** (`ade_ledger::wal::{event, replay, error, mod, store_trait}`); **+2 BLUE types**; **+1 CI** (`ci_check_wal_rollback_replay_equiv.sh`) |
| `47c0f487` | feat (AI-S2) | Shared detector + venue-split resolver (`DC-NODE-23/24`) | **GREEN code** (`node_sync.rs`); **+1 CI** (`ci_check_receive_detector_venue_split.sh`) |
| `7f78dd98` | feat (AI-S3) | Live fork-choice apply driver + reconciliation (`DC-NODE-25/26`) | **RED+GREEN code** (`node_lifecycle.rs` + `node_sync.rs`); **+2 CI** (`ci_check_live_fork_choice_apply.sh` + `ci_check_live_fork_choice_wiring.sh`) |
| `30b5727c` | feat (AI-S4a) | Wire rollback signal preservation (`DC-PUMP-01`) | **RED code** (`ade_runtime::admission::wire_pump` + `admission/bootstrap.rs` non-consuming note); **+1 CI** (`ci_check_wire_rollback_signal_preserved.sh`) |
| `04f358bd` | feat (AI-S4b-i) | Participant venue declaration (inert) (OQ-5) | **RED code** (`cli.rs` + `main.rs` + `node_lifecycle.rs`); **+1 CI** (`ci_check_participant_venue_inert.sh`) |
| `af51b3c8` | feat (AI-S4b-ii) | Live rollback-follow routing + forge gate (the go-live flip) | **RED code** (`node_lifecycle.rs` + `node_sync.rs`); **+1 CI** (`ci_check_live_fork_choice_wiring.sh`) |
| `a8d93ba4` | feat (AI-S5) | Convergence evidence + operator pass (CE-AI-5 + CE-AI-6) | **RED + hermetic** (`fork_choice.rs` `#[cfg(test)]`; runbook); **+2 CI** (`ci_check_chain_selection_arrival_order_independent.sh` + `ci_check_convergence_evidence_schema.sh`); registry: `CN-CONS-01 → enforced` |
| `cbad2ae3` | docs (evidence) | Add preprod ADE1 pool registration manifest | **0 code / 0 CI / 0 rule** — **UNRELATED to N-AI**, folded into the span |
| `5ec841c8` | fix (AI-S6) | Rollback-target slot/hash canonical binding (H-1 remediation) | **RED code** (`node_sync.rs` + `node_lifecycle.rs` + `restart.rs`); **+1 CI** (`ci_check_rollback_target_canonical_binding.sh`); registry: `DC-NODE-29` introduced+enforced + `DC-NODE-23..28` enforced + 13 strengthenings |

## 1. Commit Log (newest first)

| Hash | Type | Summary |
|------|------|---------|
| `5ec841c8` | fix | fix(phase4-n-ai): AI-S6 -- rollback-target slot/hash canonical binding (H-1 remediation) |
| `197f6332` | docs | docs(phase4-n-ai): slice doc AI-S6 -- rollback-target slot/hash binding (H-1 remediation) |
| `cbad2ae3` | docs | docs(evidence): add preprod ADE1 pool registration manifest |
| `a8d93ba4` | feat | feat(phase4-n-ai): AI-S5 -- convergence evidence + operator pass (CE-AI-5 + CE-AI-6) [last slice] |
| `23ce2697` | docs | docs(phase4-n-ai): slice doc AI-S5 -- convergence evidence + operator pass (last slice) |
| `af51b3c8` | feat | feat(phase4-n-ai): AI-S4b-ii -- live rollback-follow routing + forge gate (the go-live flip) |
| `a52df6c4` | docs | docs(phase4-n-ai): slice doc AI-S4b-ii -- live rollback-follow routing + forge gate |
| `fbe33112` | docs | docs(phase4-n-ai): correct AI-S4b-ii live authority path (direct RolledBack, not orchestrator) |
| `04f358bd` | feat | feat(phase4-n-ai): AI-S4b-i -- Participant venue declaration (inert) (OQ-5) |
| `8cbf1242` | docs | docs(phase4-n-ai): slice doc AI-S4b-i -- Participant venue declaration (inert) |
| `4d1bc4cc` | docs | docs(phase4-n-ai): sub-split AI-S4b into S4b-i (venue, inert) + S4b-ii (flip) |
| `30b5727c` | feat | feat(phase4-n-ai): AI-S4a -- wire rollback signal preservation (DC-PUMP-01) |
| `60ad1c25` | docs | docs(phase4-n-ai): slice doc AI-S4a -- wire rollback signal preservation |
| `471835a2` | docs | docs(phase4-n-ai): split AI-S4 into S4a (wire rollback signal) + S4b (live loop) |
| `7f78dd98` | feat | feat(phase4-n-ai): AI-S3 -- live fork-choice apply driver + reconciliation (DC-NODE-25/26) |
| `3a9bdcc5` | docs | docs(phase4-n-ai): slice doc AI-S3 -- live fork-choice apply driver + reconciliation |
| `47c0f487` | feat | feat(phase4-n-ai): AI-S2 -- shared detector + venue-split resolver (DC-NODE-23/24) |
| `0090987c` | docs | docs(phase4-n-ai): slice doc AI-S2 -- shared detector + venue-split resolver |
| `cced0214` | feat | feat(phase4-n-ai): AI-S1 -- rollback WAL durability foundation (WalEntry::RollBack) |
| `7b37c6fd` | docs | docs(phase4-n-ai): correct rollback WAL tag 4 -> 1 (reserved RollBackward slot) |
| `8fab31bc` | docs | docs(phase4-n-ai): slice doc AI-S1 -- rollback WAL durability foundation |
| `3ed9e67a` | docs | docs(phase4-n-ai): cluster doc -- live fork-choice wiring (AI-S1..S5) |
| `19676365` | docs | docs(phase4-n-ai): cluster-slice plan -- AI-S1..S5 (rollback-foundation-first) |
| `80862c7f` | docs | docs(phase4-n-ai): OQ-1 resolved -> A (WalEntry::RollBack marker) |
| `d92f9ce8` | docs | docs(phase4-n-ai): invariants sketch + DC-NODE-23..28 declared |
| `c66fa9a9` | chore | chore(idd): bump head_deltas_baseline to the PHASE4-N-AH close (8e2c3672); registry 347 |

No merge commits in the span. **26 commits, zero unclassified.** Every subject carries an explicit
conventional-commits prefix (`chore(...)` / `docs(...)` / `feat(...)` / `fix(...)`). The `feat(...)` commits
(`cced0214`, `47c0f487`, `7f78dd98`, `30b5727c`, `04f358bd`, `af51b3c8`, `a8d93ba4`) are the production
BLUE/RED/GREEN changes; `5ec841c8` is the `fix(...)` H-1 remediation; the rest are `docs(...)` slice/cluster/
invariants docs and the `chore(...)` baseline bump. **`cbad2ae3`** (`docs(evidence): add preprod ADE1 pool
registration manifest`) is **unrelated to PHASE4-N-AI** — a docs-only preprod pool-registration evidence
manifest, folded into the span but not cluster work. All commits landed 2026-06-08 / 2026-06-09.

> **Note (commit-attribution policy).** Per this repo's `CLAUDE.md` override (vibe-coded-node bounty
> trailer requirement), commits in this repo carry a `Co-Authored-By:` model-attribution trailer; that
> is an Ade-local override of the global no-AI-attribution rule and applies to **commit messages
> only**. It does not affect this doc's content.

## 2. New Modules

**None.** `git diff --diff-filter=A --name-only 8e2c3672..HEAD -- 'crates/**/*.rs'` lists **five** new `.rs`
files, but **all five are integration tests** (`crates/*/tests/`), not library modules:
`crates/ade_ledger/tests/wal_rollback_ai_s1.rs`, `crates/ade_node/tests/{apply_driver_ai_s3,
live_fork_choice_ai_s4bii, participant_venue_ai_s4bi, receive_detector_ai_s2}.rs`. There is **no new crate, no
new `Cargo.toml`, no new library module, no new workspace** (`git diff --name-only … '**/Cargo.toml'` is
empty; still **11 crates**). The whole span is **modification only** — the new BLUE authority is a new *variant*
+ payload types **inside the existing** `crates/ade_ledger/src/wal/event.rs`, and the new GREEN/RED machinery is
added **inside the existing** `ade_node` + `ade_runtime` source files (§3). The other added files this span are
the **nine CI gates** (§5), the N-AI **cluster + eight slice docs** (`docs/clusters/PHASE4-N-AI/`), the N-AI
**plan + invariants + OQ docs** (`docs/planning/`), the **convergence runbook**
(`docs/active/phase4-n-ai-convergence-runbook.md`), and the **unrelated preprod pool-registration evidence**
(`docs/evidence/preprod-pool-registration.md`).

> **Cross-reference (CODEMAP) — no new module; new BLUE *types* carried by CODEMAP.** The span adds **no
> module** — the new BLUE canonical types (`RollbackPoint`, `RollbackReason`) and the new closed WAL variant
> (`WalEntry::RollBack`) live **inside** the existing `ade_ledger::wal::event` module, and the new GREEN/RED
> types (`NodeSyncItem`, `ReceiveClass`, `ReceiveDisposition`, `CandidateSummary`, `AppliedTip`, `ApplyError`)
> live **inside** `ade_node::{node_sync, node_lifecycle}`. **CODEMAP was regenerated this close** and carries
> all of them (`RollbackPoint` ×26, `RollbackReason` ×15, `WalEntry::RollBack` ×27, the BLUE count `460`). No
> cross-reference warning for §2.

## 3. Modules Modified

Thirteen source files across **three crates** changed — **`ade_ledger::wal` (BLUE, the new durable rollback
marker) + `ade_core::consensus::fork_choice` (BLUE, `#[cfg(test)]`-only) + `ade_node` (GREEN + RED) +
`ade_runtime` (RED shell)**, **+2 BLUE canonical types**:

| Module | Color / scope | Key changes |
|--------|---------------|-------------|
| `ade_ledger::wal` (`event.rs` +109/−3, `replay.rs` +75, `error.rs` +6, `store_trait.rs` +28/−1, `mod.rs` +2/−2) | **BLUE** authority, +2 canonical types | **AI-S1 (`cced0214`) — DC-NODE-27:** new version-gated additive `WalEntry::RollBack { to_point: RollbackPoint, reason: RollbackReason, prior_tip: RollbackPoint, selected_tip: RollbackPoint }` at the **reserved RollBackward tag 1** (`TAG_ROLLBACK = 1`; tag 2 `CaptureSnapshot` stays reserved); the **+2 new BLUE canonical payload types** `RollbackPoint` (slot + hash + block_no) and the closed `RollbackReason` (uint wire code; sole rung-1 variant `PeerRollBackward`, `from_code`/code round-trip); a new closed `WalError::RollbackTargetNotInChain` + accessor plumbing (`store_trait`). `replay.rs` gains a `RollBack` arm that **re-invokes** the existing `materialize_rolled_back_state` (CN-STORE-07) + lockstep `commit_rollback` (DC-CONS-20) and re-anchors `prev_post_fp` to `to_point`'s in-chain `post_fp` — replacing today's `ChainBreak` with a faithful linear-with-rollbacks replay. **A durable MARKER, NOT a second rollback implementation.** Append-only preserved. `ade_ledger` BLUE type count `177 → 179`. |
| `ade_core::consensus::fork_choice` (`fork_choice.rs` +85) | **BLUE** — `#[cfg(test)]` only | **AI-S5 (`a8d93ba4`) — CN-CONS-01:** the **production `select_best_chain` is byte-identical** — the only hunk is `@@ -340,4 +340,89 @@ mod tests`, a `#[cfg(test)]` arrival-order-permutation determinism proof (`select_best_chain_arrival_order_independent_distinct_heights`, `select_best_chain_arrival_order_independent_tiebreaker`) showing the converged tip is the fork-choice-maximal chain regardless of candidate arrival order. **+0 production change; +0 canonical type** (the +2 BLUE types are in `ade_ledger::wal`, not here). |
| `ade_node::node_sync` (`node_sync.rs` +263/−25) | **GREEN** detector/resolver, heavy | **AI-S2 (`47c0f487`) — DC-NODE-23/24:** the pure total venue-blind detector (`classify_receive` → `ReceiveClass`/`ReceiveDisposition { AlreadyHave \| LinearExtend \| RefuseSingleProducer \| NeedsForkChoice }`) + the venue-split resolver (`resolve_disposition`: `SingleProducer → refuse`, `Participant → NeedsForkChoice`; unknown venue fails closed); new closed types `NodeSyncItem { Block \| RollBack(RollbackPoint) }`, `ReceiveClass`, `ReceiveDisposition`, `CandidateSummary`; the detector **never** calls `select_best_chain`. **AI-S4b-ii (`af51b3c8`) — DC-NODE-28:** `pending_reselection_forge_refusal` → `Some(ForgeRefused::ReselectionPending)` while a decision is pending (a pure/total producer-race fence). **AI-S6 (`5ec841c8`) — DC-NODE-29:** the new closed `NodeSyncError::{UnexpectedRollback, RollbackPointSlotMismatch}`; the SingleProducer arm returns `UnexpectedRollback` on any `NodeSyncItem::RollBack` (never adopts a peer rollback); the rollback-target slot is bound to the durable stored slot for the hash and fails closed on mismatch. **The GREEN decision never references a chain selector (`DC-CONS-03` honored).** |
| `ade_node::node_lifecycle` (`node_lifecycle.rs` +406/−13) | **RED** apply driver + loop, heavy | **AI-S3 (`7f78dd98`) — DC-NODE-25/26:** the RED `apply_chain_event` durable-apply driver (materialize + lockstep `roll_backward` + `WalEntry::RollBack` append + `pump_block` roll-forward — **no second apply path**; provisional until bodies apply) + the reconciliation assert (`selector.current_tip == ChainDb::tip` after every decision); new types `AppliedTip` / `ApplyError`. **AI-S4b-ii (`af51b3c8`) — DC-NODE-28 (the flip):** `run_participant_sync` classifies every block; the RollBack path resolves the durable point → `ChainEvent::RolledBack` → `apply_chain_event` (**NOT `process_stream_input`**; the loop never calls `select_best_chain`); a bare competing block fails closed; `ForgeActivation.pending_reselection` gates the forge. **AI-S4b-i (`04f358bd`):** `declare_participant_venue` (inert venue recognition). **AI-S6 (`5ec841c8`) — DC-NODE-29:** the rollback-target canonical binding wired through the live RollBack arm (stored slot+hash sole authority; fails closed pre-mutation). **No new BLUE type.** |
| `ade_runtime::admission::wire_pump` (`wire_pump.rs` +133/−6) | **RED** shell, heavy | **AI-S4a (`30b5727c`) — DC-PUMP-01 (strengthened):** `AdmissionPeerEvent` gains a closed `RollBackward { peer, point: Point, tip: Tip }` variant — the peer's chain-sync `RollBackward` **point** is **preserved as a closed event** (was discarded → `TipUpdate` only; "a rollback is NEVER represented as a `TipUpdate` only"). `RollBackward(Origin)` is **unsupported for single-best-peer within k** and **fails closed** (`UnsupportedRollbackPoint`). `Block`/`TipUpdate`/`Disconnected` unchanged. Merges **latent** (the live loop consumes it at AI-S4b-ii). **No new BLUE type.** |
| `ade_node::cli` (`cli.rs` +19) + `ade_node::main` (`main.rs` +5) | **RED**, additive | **AI-S4b-i (`04f358bd`):** `--participant-venue → VenueRole::Participant`, **mutually exclusive** with `--single-producer-venue` (a new `CliError::ConflictingVenue` rendered in `main.rs`); `Unknown`/absent stays the conservative non-fork-choice path; **no silent inference**. Truly inert — recognizes the mode value only. **No new BLUE type.** |
| `ade_runtime::recovery::restart` (`restart.rs` +8) + `ade_node::admission::bootstrap` (`bootstrap.rs` +5) | **RED** shell, additive | **AI-S6 (`5ec841c8`):** the WAL-tail reverse scan skips `WalEntry::RollBack` (a `RollBack` is not an `AdmitBlock` and does not define the WAL-tail slot — safe because the recovery floor is the durable ChainDb trim + the rollback-aware T-REC-05 fingerprint check in `replay_from_anchor`, not this scan). **AI-S4a (`30b5727c`):** `bootstrap.rs` documents the admission-mode runner as a **non-consuming** rollback path (`RollBackward { .. } => continue` — not a rollback→`TipUpdate` downgrade; the live `--mode node` path consumes via `node_sync`). **No new BLUE type.** |

> **BLUE change this span (load-bearing).** Unlike the prior several windows, this span **does** touch BLUE —
> `git grep -hE '^(pub )?(struct\|enum) '` over the BLUE `core_paths` trees is **`458 → 460`** (the +2 being
> `ade_ledger::wal::event::{RollbackPoint, RollbackReason}`). It is the **first BLUE delta since the G-N span**.
> Three BLUE files change: `ade_ledger::wal::{event, replay}` (production — the new closed `WalEntry::RollBack`
> durable MARKER + its replay arm) + `ade_ledger::wal::{error, mod, store_trait}` (closed error + accessors) +
> `ade_core::consensus::fork_choice` (**`#[cfg(test)]`-only** — `select_best_chain` production is byte-identical).
> The new BLUE authority is a durable MARKER that **re-invokes the existing rollback authority on replay** — it
> is **NOT** a second rollback implementation; `pump_block` stays the SOLE roll-forward durable admit
> (`DC-NODE-05`/`DC-NODE-12` reused unchanged); `materialize_rolled_back_state` (`CN-STORE-07`), the lockstep
> `receive::reducer` (`DC-CONS-20`), and `select_best_chain` (`DC-CONS-03`) production are **reused, not built**.
> **Two test files** (`crates/ade_node/tests/wire_only_loopback.rs` +1,
> `crates/ade_node/tests/phase4_n_ae_recover_serve_continuity_diag.rs` +3) and one runtime test
> (`crates/ade_runtime/tests/wal_replay_from_anchor.rs` +1) were touched additively — test-only.

## 4. Feature Flags

**No project feature-flag deltas.** Ade declares no `[features]` table in any workspace `Cargo.toml`, and **no
`Cargo.toml` changed in this window** (`git diff --name-only 8e2c3672..HEAD -- '**/Cargo.toml' 'Cargo.toml'`
is empty). No `#[cfg(feature = …)]` gate was introduced and no `compile_error!` coupling was added (`git diff
8e2c3672..HEAD` grep for both is empty). The notable CLI-flag delta this span is an **addition**:
`--participant-venue` (AI-S4b-i — declares `VenueRole::Participant`), **mutually exclusive** with the
N-AF-introduced `--single-producer-venue` (a new `CliError::ConflictingVenue` is raised if both are passed).
These are CLI flags parsed into `Cli`, **not** Cargo feature flags, env vars, or compile-time `cfg`. **Coupling
note:** the two venue flags are **mutually exclusive** (enforced at CLI parse, fail-closed); an absent/unknown
venue takes the conservative non-fork-choice (`SingleProducer`/`Unknown`) path — **no silent inference** of
`Participant` from network traffic and no silent inference of `SingleProducer` for a configured participant
node (`DC-NODE-24`). The gates `ci_check_participant_venue_inert.sh` (venue recognized but inert) and
`ci_check_receive_detector_venue_split.sh` (unknown venue fails closed) fence the coupling.

## 5. CI Checks (148 → 157; +9 new gates, 0 modified in place, 0 removed)

Nine new gates this span; **zero modified in place; zero removed**. `git diff --diff-filter=A 8e2c3672..HEAD
-- ci/` lists exactly the nine gates below; `--diff-filter=M` and `--diff-filter=D` over `ci/` are **empty**.
The grouping mirrors the AI-S1…AI-S6 slice progression.

### PHASE4-N-AI gates — rollback durability + detector (AI-S1 / AI-S2)

| Check | Status | Origin / change | What it checks |
|-------|--------|-----------------|----------------|
| `ci_check_wal_rollback_replay_equiv.sh` | **New** | AI-S1 (`cced0214`); `DC-NODE-27` | A WAL with a `RollBack` entry replays **byte-identically** (the rolled-back-then-reselected chain recovers the *selected* tip, never the abandoned branch); the `RollBack` replay arm invokes `materialize_rolled_back_state` / the lockstep reducer (**not** a reimpl); append-only preserved; the version-gated tag-1 marker decodes closed. |
| `ci_check_receive_detector_venue_split.sh` | **New** | AI-S2 (`47c0f487`); `DC-NODE-23` / `DC-NODE-24` | The classifier is **total** over the 4 dispositions; `SingleProducer → refuse`, `Participant → NeedsForkChoice`; `AlreadyHave` for a known echo; the detector **never calls `select_best_chain`**; an unknown/invalid venue **fails closed** (no silent inference). |

### PHASE4-N-AI gates — live apply + wire signal (AI-S3 / AI-S4a)

| Check | Status | Origin / change | What it checks |
|-------|--------|-----------------|----------------|
| `ci_check_live_fork_choice_apply.sh` | **New** | AI-S3 (`7f78dd98`); `DC-NODE-25` / `DC-NODE-26` | A `ChainSelected`/`RolledBack` requiring rollback applies via materialize + lockstep + `WalEntry::RollBack` + `pump_block`; **no header-only tip advance**; `selector.current_tip == ChainDb::tip` after apply; **no second apply / tip-advance / rollback-materialize path**. |
| `ci_check_live_fork_choice_wiring.sh` | **New** | AI-S3 / AI-S4b-ii (`7f78dd98` / `af51b3c8`); `DC-NODE-25/26/28` | The live Participant path **follows a peer's `RollBackward` reorg end-to-end** (single-best-peer: durable-point lookup → `ChainEvent::RolledBack` → `apply_chain_event`; bare competing blocks fail closed — **not** multi-candidate selection); the forge gate `ForgeRefused::ReselectionPending` fires while a decision is pending (CE-AI-4 folded in). |
| `ci_check_wire_rollback_signal_preserved.sh` | **New** | AI-S4a (`30b5727c`); `DC-PUMP-01` (strengthening) | The admission wire pump preserves the peer's chain-sync `RollBackward` **point** as a closed `AdmissionPeerEvent` variant (was discarded → `TipUpdate` only); a rollback is **never** a `TipUpdate` only; `Origin` fails closed (`UnsupportedRollbackPoint`); `Block`/`TipUpdate`/`Disconnected` unchanged. |

### PHASE4-N-AI gates — venue, convergence, H-1 remediation (AI-S4b-i / AI-S5 / AI-S6)

| Check | Status | Origin / change | What it checks |
|-------|--------|-----------------|----------------|
| `ci_check_participant_venue_inert.sh` | **New** | AI-S4b-i (`04f358bd`); `DC-NODE-28` (venue precursor / OQ-5) | The `--participant-venue` declaration is recognized as a closed mode value but is **inert** — it does NOT route to fork-choice, does NOT change any forge decision, does NOT alter `SingleProducer` behavior; mutually exclusive with `--single-producer-venue` (`CliError::ConflictingVenue`); no silent inference either direction. |
| `ci_check_chain_selection_arrival_order_independent.sh` | **New** | AI-S5 (`a8d93ba4`); `CN-CONS-01` | For a fixed competing-candidate set, the converged tip is the fork-choice-maximal chain **regardless of arrival order** (a `#[cfg(test)]` permutation proof over `select_best_chain` — distinct heights + tiebreaker). |
| `ci_check_convergence_evidence_schema.sh` | **New** | AI-S5 (`a8d93ba4`); CE-AI-6 (`CN-CONS-03` operator-gated) | The convergence-evidence transcript is a **closed derived-tier vocabulary** (allow-listed event kinds), **vacuous-until-committed**, sha256-bound; Ade + ≥1 Haskell producer converge on the same tip; **proves the exercised venue, NOT full multi-peer ChainSel.** |
| `ci_check_rollback_target_canonical_binding.sh` | **New** | AI-S6 (`5ec841c8`); `DC-NODE-29` (H-1 remediation) | The live Participant rollback target uses the durable `ChainDb` stored chain point (stored slot + hash) as the **SOLE authority**; the peer-supplied slot **must equal** the stored slot for that hash; **any** mismatch / unknown hash / Origin **fails closed with a typed error BEFORE `commit_rollback`, BEFORE `WalEntry::RollBack`, BEFORE any ChainDb/LedgerState/PraosChainDepState mutation**; no mixed peer/local authority. |

> **Cross-reference (TRACEABILITY + SEAMS) — STALE this close; refresh owed.** The nine new rule↔gate bindings
> are recorded **in the registry at HEAD** (`docs/ade-invariant-registry.toml`, 354 rules) and in the
> **regenerated CODEMAP** (`5ec841c8`). They are **NOT yet in TRACEABILITY or SEAMS**, which remain pinned at
> the N-AH close `5858288e` (`grep -c` of `DC-NODE-23`/`DC-NODE-27`/`DC-NODE-29` in TRACEABILITY = 0; SEAMS
> header still reads "458 canonical types, 148 CI checks at HEAD `5858288e`"). **None of the nine new gates is
> an orphan** — each enforces exactly its named rule, recorded in the registry. **Action:** regenerate SEAMS +
> TRACEABILITY to `5ec841c8` as a follow-on this close so every §5 gate appears in TRACEABILITY enforcing its
> named invariant; until then the registry is authoritative for the new bindings. (`DC-PUMP-01` already appears
> ×1 in TRACEABILITY from a prior pass; its new AI-S4a gate `ci_check_wire_rollback_signal_preserved.sh` is the
> refresh delta.)

## 6. Canonical Type Registry Delta

**n/a — no separate canonical-type registry is configured** (`canonical_type_registry: null`);
canonical-type rules live inline in the invariant registry under family **T**. **This window ADDED +2 BLUE
canonical types — the first BLUE delta since the G-N span:** the BLUE count is `458 → 460` (`git grep -hE
'^(pub )?(struct|enum) '` over the BLUE `core_paths` trees), the +2 being `ade_ledger::wal::event::{RollbackPoint,
RollbackReason}` — the payload types of the new closed-sum WAL variant `WalEntry::RollBack` (`ade_ledger` `177
→ 179`). **Zero BLUE canonical types were removed.** The variant itself (`WalEntry::RollBack`) and the new
closed error (`WalError::RollbackTargetNotInChain`) are additive extensions of existing closed enums — not new
top-level types and not removals. No `Cargo.toml` changed.

## 7. Normative / Invariant Rule Delta (347 → 354; +7 rules, 13 strengthenings, zero removals)

**Seven rule IDs were added; zero removed** (`347 → 354`; `diff` of the sorted `id =` lists shows exactly the
seven additions `DC-NODE-23` … `DC-NODE-29` and no removal). The status tally moves **213 → 221 enforced**
(the 7 new `DC-NODE-23..29` + `CN-CONS-01` flipped `partial → enforced`) and **20 → 19 partial** (`CN-CONS-01`);
the 114 declared **unchanged**.

*(The configured `normative_docs` — the CE-79 tier-gate statement + addendum, the three contract docs, the
CE-73 reclassification, and `CLAUDE.md` — were **not** changed this span: `git diff --name-only
8e2c3672..HEAD` over those paths is empty. The rule-count delta is entirely the invariant-registry change.)*

**New rules (`+7`, all `enforced`, all `introduced_in = "PHASE4-N-AI"`):**

| Rule | Family / Tier · Status | Statement (summary) |
|------|------------------------|---------------------|
| `DC-NODE-23` | DC / `derived` · **enforced** | **Shared receive-side fork-choice detector (rung-2).** A peer-origin candidate **not** already part of Ade's admitted durable spine / own-served lineage — incl. a header that does not build on `ChainDb::tip` — is classified **ONCE, venue-blind**, by a pure total predicate `(durable_tip, candidate_header_summary) → ReceiveDisposition { AlreadyHave \| LinearExtend \| RefuseSingleProducer \| NeedsForkChoice }`. A duplicate/already-known peer echo is `AlreadyHave`, never a competing candidate merely for not being a fresh extension. Observes no venue / wall-clock / network state; the single classification point both the SingleProducer fail-closed arm (`DC-NODE-20`) and the Participant fork-choice arm (`DC-NODE-24`) consume. **Never selects/reorders/prefers chains** (that is `select_best_chain` / `DC-CONS-03`). |
| `DC-NODE-24` | DC / `derived` · **enforced** | **Venue-split fork-choice resolver (rung-2).** The `DC-NODE-23` non-spine consequent is gated by venue, **total over the closed venue set**: `SingleProducer ⇒ fail closed` (the `DC-NODE-20` rung-1 behavior, byte-unchanged — never adopt a peer candidate); `Participant ⇒ NeedsForkChoice ⇒` the existing `chain_selector` orchestrator (`process_stream_input → select_best_chain`, `DC-CONS-03`). Venue input is explicit and **fail-safe** (undeclared/unknown ⇒ conservative SingleProducer refuse). In Participant mode the peer's **VALIDATED** header summary (post `validate_and_apply_header`) becomes a candidate; a raw `followed_peer_tip` signal MUST NOT reach `select_best_chain`. |
| `DC-NODE-25` | DC / `derived` · **enforced** | **Live fork-choice durable application authority (rung-2).** A `ChainSelected`/`RolledBack` outcome is applied to the durable stores **ONLY** via the existing enforced authorities: the lockstep receive reducer (`DC-CONS-20`) + `materialize_rolled_back_state` (`CN-STORE-07`) for the rollback target + `pump_block` (`DC-NODE-05/12`) for roll-forward. **No second apply path, no second tip-advance, no second rollback-materialize.** A fork-choice win is **provisional** — durably adopted ONLY when its BODIES validate and apply through `pump_block` (no header-only tip advance). A `TiebreakerLossKeepCurrent` outcome makes no durable change. |
| `DC-NODE-26` | DC / `derived` · **enforced** | **Decision / durable reconciliation (rung-2).** After any applied receive decision, `selector.current_tip == durable ChainDb::tip` (and orchestrator `chain_dep == durable PraosChainDepState`). The in-memory decision state never diverges from the persisted authority: the orchestrator decides, the durable lockstep path applies, and the two are reconciled **every decision**. No applied decision leaves the selector ahead of / behind / forked from the durable spine. |
| `DC-NODE-27` | DC / `derived` · **enforced** | **Rollback+reselection replay-equivalence (rung-2).** The ordered live receive-event sequence (RollForward headers, RollBackward points, body deliveries) replayed against the same bootstrap anchor + durable log produces a **BYTE-IDENTICAL** durable tip + ledger fingerprint + `PraosChainDepState` — **including any rollback+reselection**. A live rollback is recorded durably (append-only canonical bytes — `CN-WAL-01`) such that replay re-invokes the SAME materialize/reducer authority (`CN-STORE-07`/`DC-CONS-20`); the durable record is **NOT** a second rollback implementation. **OQ-1 RESOLVED → A** (the version-gated `WalEntry::RollBack` marker; option B WAL-tail reconciliation rejected). |
| `DC-NODE-28` | DC / `derived` · **enforced** | **No forge across unresolved re-selection (rung-2).** Once a peer-origin candidate is classified `NeedsForkChoice` (`DC-NODE-23`) in a Participant venue, forging is **DISABLED** until the outcome is either (a) durably applied and reconciled (`DC-NODE-25/26`) or (b) rejected with durable state unchanged. The forge base is **NEVER** selected from a stale pre-resolution `ChainDb::tip` while a decision is pending — a producer tick during a pending decision **fails closed** (`ForgeRefused::ReselectionPending`), never forges on the old local tip. A producer-race fence distinct from `pump_block` admit / rollback replay / reconciliation, so it carries its own evidence. |
| `DC-NODE-29` | DC / `derived` · **enforced** (AI-S6 H-1 remediation) | **Live rollback target canonical binding (rung-2).** For a peer `RollBackward(point)` on the live Participant path, the rollback target MUST be resolved against the durable `ChainDb` and use the **stored chain point (stored slot + hash) as the SOLE authority**. The peer-supplied slot MUST equal the stored slot for that hash; on **any** mismatch (or unknown hash, or Origin) the path **fails closed with a typed error BEFORE `commit_rollback`, BEFORE `WalEntry::RollBack`, BEFORE any ChainDb/LedgerState/PraosChainDepState mutation**. No rollback target may be built from mixed peer/local authority (peer slot + locally-verified hash). Reconciliation (`DC-NODE-26`) remains the post-apply backstop but is **NOT** the only defense. |

**Strengthenings (`strengthened_in += "PHASE4-N-AI"`) — 13:** `CN-CONS-01` (also flipped `partial →
enforced` — the arrival-order determinism of `select_best_chain` now has a permutation proof + gate),
**`CN-CONS-03`** (strengthened **but NOT flipped** — only the single-best-peer venue was exercised; its broad
multi-peer ChainSel statement stays `declared`, with CE-AI-6 operator-gated/vacuous-until-committed),
`CN-STORE-07` (the rollback-materialize authority is now re-invoked from the live apply driver **and** from the
`WalEntry::RollBack` replay arm), `DC-CONS-03` (`select_best_chain` is now the live Participant fork-choice
authority, reused unchanged — production byte-identical), `DC-CONS-05` + `DC-CONS-06` (rollback apply +
replay-equivalence now span live rollback+reselection), `DC-CONS-20` (the lockstep receive reducer is the live
durable-apply authority for both roll-forward and roll-backward), `DC-NODE-05` + `DC-NODE-12` (`pump_block`
stays the SOLE roll-forward durable admit on the live fork-choice path), `DC-NODE-20` (the SingleProducer venue
stays fail-closed unchanged under the new detector/resolver — `RefuseSingleProducer`), `DC-PUMP-01` (the
admission wire pump now preserves the rollback **point** as a closed event, no longer downgraded to a
`TipUpdate`), `T-REC-03` + `T-REC-05` (replay-equivalence now covers rollback+reselection — two-runs +
kill/warm-start byte-identical incl. a `RollBack` WAL entry). **No rule was weakened.**

> **The boundary — what PHASE4-N-AI closes (and what it does not).** PHASE4-N-AI wired **single-best-peer
> rollback-FOLLOW** into the live `--mode node` receive path: Ade detects a peer-origin non-spine candidate
> ONCE (venue-blind, `DC-NODE-23`), and on a Participant venue follows that ONE peer's `RollBackward` reorg
> through the EXISTING `chain_selector` → BLUE `select_best_chain` (`DC-CONS-03`) durable-apply path
> (`DC-NODE-24/25/26`), recording the rollback as a version-gated `WalEntry::RollBack` durable marker that
> re-invokes the existing rollback authority on replay (`DC-NODE-27`), refusing to forge across a pending
> decision (`DC-NODE-28`), with the rollback target bound to the durable stored point as the sole authority
> (`DC-NODE-29`). **This is NOT full multi-peer Cardano ChainSel.** It maintains **no** multi-peer candidate
> set, adds **no** second selection authority, and **`CN-CONS-03` was NOT flipped to enforced** (it stays
> `declared`, strengthened only, with the explicit honesty note that only the single-best-peer venue was
> exercised). The sole new BLUE authority is `WalEntry::RollBack` (a durable MARKER, not a second rollback
> impl); `pump_block` stays the SOLE roll-forward durable admit (`DC-NODE-05`/`DC-NODE-12` unchanged); the
> SingleProducer venue stays **fail-closed unchanged** (`run_node_sync` returns `NodeSyncError::UnexpectedRollback`
> on any peer rollback; `DC-NODE-20`/`DC-NODE-24` `RefuseSingleProducer`); and there is **NO `RO-LIVE` flip**
> (`RO-LIVE-01` stays operator-gated; CE-AI-6 is the operator-pass convergence transcript,
> **vacuous-until-committed**).

**No rule was removed (expected: 0).** The registry delta is **seven new rules (`DC-NODE-23..29`, all
enforced), one existing rule flipped (`CN-CONS-01` partial → enforced), thirteen `strengthened_in +=
PHASE4-N-AI` appends, zero removals** — consistent with append-only registry discipline. **No anomaly.**

## Working tree at HEAD `5ec841c8`

Clean of tracked changes from this span — the N-AH baseline-bump chore, the N-AI cluster (invariants → OQ-1 →
cluster/slice docs → AI-S1…AI-S6 → close), and the unrelated `cbad2ae3` preprod evidence commit are all
committed. `git status --short` shows only an untracked `.mithril-scratch/` (operator scratch, ignored). **This
regen runs *after* all 26 span commits** (the AI-S6 close `5ec841c8` is HEAD for this window); the registry
records `DC-NODE-23..29` + their nine gate bindings authoritatively at HEAD (**354 rules**), and **CODEMAP was
regenerated this close to `5ec841c8`**. The remaining close-pass actions are (1) the SEAMS + TRACEABILITY
refresh to `5ec841c8` (surfaced in §5) and (2) the baseline bump (`8e2c3672 → 5ec841c8`), both separate
post-close steps.

> **Cluster-context note.** PHASE4-N-AI is **formally closed** — its cluster doc (`docs/clusters/PHASE4-N-AI/cluster.md`)
> carries the §11 close record (CE-AI-7: `DC-NODE-23..28` declared→enforced, `DC-NODE-29` introduced+enforced
> at AI-S6, `CN-CONS-01` partial→enforced, **`CN-CONS-03` NOT flipped**, 13 strengthenings) and the H-1
> security-remediation record (H-1 CLOSED, no new findings). Whether the cluster docs are moved to
> `docs/clusters/completed/PHASE4-N-AI/` is a close-pass bookkeeping decision separate from this HEAD_DELTAS
> regen.

## Honest residual (window scope)

PHASE4-N-AI **wired single-best-peer rollback-FOLLOW into the live receive path** and made it
replay-equivalent + fail-closed. The honest residual:

- **The headline boundary (verbatim).** Ade now **follows ONE peer's chain-sync `RollBackward` reorg
  end-to-end** on a declared Participant venue — durable-point lookup → `ChainEvent::RolledBack` → materialize
  + lockstep + a version-gated `WalEntry::RollBack` durable marker + `pump_block` roll-forward,
  replay-equivalently and fail-closed. **This is single-best-peer rollback-following, NOT full multi-peer
  Cardano ChainSel.**
- **`CN-CONS-03` was NOT flipped (load-bearing).** The cluster plan intended `CN-CONS-03` (live convergence) to
  flip declared→enforced, but the **close held it `declared`, strengthened only**: its statement is broad
  (full multi-peer ChainSel) and the live convergence evidence (CE-AI-6) is **operator-gated and
  vacuous-until-committed** — only the single-best-peer venue was exercised. Promoting it requires the
  committed operator convergence transcript **plus** a later multi-peer candidate-set slice.
- **`pump_block` stays the sole admit; SingleProducer unchanged.** The roll-forward durable admit authority is
  unchanged (`DC-NODE-05`/`DC-NODE-12`); the SingleProducer venue stays **fail-closed unchanged**
  (`NodeSyncError::UnexpectedRollback` on any peer rollback; `DC-NODE-20`/`DC-NODE-24` `RefuseSingleProducer`).
  N-AI adds a **follow** path for a Participant venue; it does not change the producer path.
- **First BLUE delta since G-N — a durable marker, not a second rollback impl.** +2 BLUE canonical types
  (`RollbackPoint`, `RollbackReason`), one new closed BLUE WAL variant (`WalEntry::RollBack`), one new closed
  BLUE error (`WalError::RollbackTargetNotInChain`). The marker **re-invokes the existing
  `materialize_rolled_back_state` + lockstep reducer on replay** — it is not a second rollback implementation.
  `select_best_chain` production is **byte-identical** (the only `fork_choice` change is a `#[cfg(test)]`
  permutation proof). BLUE count `458 → 460`.
- **H-1 remediated at cluster-close (AI-S6 / `DC-NODE-29`).** The per-cluster security review found that the
  live RollBack path could build a rollback target from mixed peer/local authority (a malicious peer could
  truncate the durable chain and brick the node); AI-S6 bound the target to the durable stored chain point
  (stored slot + hash, validated **pre-mutation**, fail-closed). Security re-review: **H-1 CLOSED, no new
  findings.** A forward OQ remains: the orchestrator's `chain_selector::process_rollback` path should also be
  aligned with the `DC-NODE-29` canonical target binding (it is not on the live block/pump follow path today).
- **No `RO-LIVE` flip.** CE-AI-6 is the operator-gated convergence transcript (vacuous-until-committed,
  sha256-bound). `RO-LIVE-01` stays operator-gated / partial. No `RO-LIVE` registry status changed this span.
- **SEAMS + TRACEABILITY refresh owed this close.** CODEMAP was regenerated to `5ec841c8` and is current;
  SEAMS + TRACEABILITY remain pinned at the N-AH close `5858288e` and do not yet carry `DC-NODE-23..29`. The
  registry holds the seven new rules + nine gate bindings + thirteen strengthenings authoritatively at HEAD
  (354 rules) in the interim. Regenerating SEAMS + TRACEABILITY to `5ec841c8` is the named follow-on.
- **One unrelated commit folded into the span.** `cbad2ae3` (`docs(evidence): add preprod ADE1 pool
  registration manifest`,`docs/evidence/preprod-pool-registration.md`) is docs-only and **not** PHASE4-N-AI
  work; it sits inside the `8e2c3672..HEAD` range and is recorded in §1 for completeness.

---

## Historical — PHASE4-N-AG superseded + PHASE4-N-AH local-tip forge-base authority (`f87d0056 → 5858288e`)

> The section below is the **previous** HEAD_DELTAS lead, preserved in condensed form. It narrated the
> `f87d0056 → 5858288e` span: the **PHASE4-N-AF close tail** (`600581e8` + `2d99cdf2`, docs/archive only) +
> the **PHASE4-N-AG cluster** (single-producer loop-continuation-after-feed-EOF, `DC-NODE-19`;
> **superseded-close** — hermetic core CE-AG-1..4 complete, live CE re-homed to N-AH) + the **PHASE4-N-AH
> cluster** (local selected durable chain forge-base authority `DC-NODE-20` + cert evidence-only `DC-NODE-21`
> + single-producer warm-start re-entry `DC-NODE-22`). **32 commits, 48 files, +5155 / −743.** **RED/GREEN-only
> — ZERO BLUE change, 458 → 458 canonical types** (six source files in `ade_node` + `ade_runtime`). CI gates
> **143 → 148** (+5: `single_producer_loop_continuation`, `local_durable_forge_base`, `cert_evidence_only`,
> `warm_start_re_entry`, `live_transcript_forge_base`; 3 modified in place; 0 removed). Registry **343 → 347**
> (+4: `DC-NODE-19` declared + `DC-NODE-20/21/22` enforced; 9 strengthenings, all `PHASE4-N-AH`; 0 removed).
> Headline (honest boundary): Ade sustained **cert-free single-producer block production on C2-LOCAL**
> (`cardano-testnet` magic 42) against a real Haskell relay (`cardano-node 11.0.1`) — forged on its OWN local
> durable `ChainDb::tip`, crossed a follow-link EOF, settled `> k` immutable, and resumed forging after a hard
> restart (run-4, `docs/evidence/phase4-n-ah-ce-ah-6-close.{md,jsonl}`). NOT preprod. NOT bounty completion.
> No `RO-LIVE` flip. The N-AF operator-adoption certificate had leaked from evidence into forge-loop authority;
> N-AH retired it (folded out the `FirstOwnBlockServed` cert-wait `ForgeMode` state + **deleted** the
> `VenueAdoptionCertificate` type + cert parser + `--adoption-cert-path` flag). All four grounding docs were
> regenerated at the N-AH close (paying the deferred N-AF + N-AG CODEMAP debt). The full §§0–7 narrative is
> recoverable from this doc's git history at `5858288e`.

---

## Historical — PHASE4-N-AF single-producer extend-own-durable-spine (`6363683e → f87d0056`)

> Preserved as a pointer. A **single-slice cluster lead** narrating the `6363683e → f87d0056` span: the
> PHASE4-N-AE.F close grounding-doc refresh (`d3f52e7c`, span head) + a C2-guide doc (`1302417d`) followed by
> the **OQ-1 / DC-NODE-17 investigation** (`bd1a7a73` declared DC-NODE-17 → `dadf4743` live-disproved it as the
> fix) and the **PHASE4-N-AF cluster** (single slice AF.S1 — `DC-NODE-18`, single-producer
> extend-own-durable-spine). Counts at `f87d0056`: 343 rules, 143 CI gates, 458 canonical types. **GREEN+RED
> only — BLUE 458 → 458.** New gate `ci_check_single_producer_extend_own_spine.sh`. `DC-NODE-17` declared then
> live-disproved (the relay does NOT re-announce Ade's own block) and retained safety/observation-only; the
> actual fix was `DC-NODE-18`. No `RO-LIVE` flip. CODEMAP/SEAMS/TRACEABILITY refresh was deferred at the N-AF
> close and paid at the N-AH close. The full §§0–7 narrative is recoverable from this doc's git history at
> `f87d0056`.

---

## Historical — PHASE4-N-AE.F post-CE-A5 echo-idempotency follow-up (`a76672b9 → 6363683e`)

> Preserved as a pointer. A **single-slice lead** narrating the `a76672b9 → 6363683e` span: the PHASE4-N-AE
> close grounding-doc refresh (`62811a4e`, span head) followed by the **PHASE4-N-AE.F** slice (`DC-NODE-16`
> receive idempotency at the durable-admit chokepoint — a re-announced block Ade already durably holds (same
> hash, same slot) is an idempotent no-op at `pump_block`, so a continuous recover→follow run survives the
> post-adoption echo instead of exiting 43). Counts at `6363683e`: 341 rules, 142 CI gates, 458 canonical
> types. **RED chokepoint only — BLUE 458 → 458.** New gate `ci_check_receive_idempotency.sh`. No `RO-LIVE`
> flip. The full §§0–7 narrative is recoverable from this doc's git history at `6363683e`.

---

## Historical — PHASE4-N-AD durability proof + C2-LOCAL run + PHASE4-N-AE CE-A5 cluster (`25ddeebd → a76672b9`)

> Preserved as a pointer. A **multi-part lead** narrating the `25ddeebd → a76672b9` span: the PHASE4-N-AC
> grounding-doc-refresh tail (`25ddeebd`), the **test-only PHASE4-N-AD** tip-successor durability cluster, a
> **docs-only C2-LOCAL** preprod-tip / cardano-testnet venue guide-and-finding run, and the closing **CE-A5
> cluster PHASE4-N-AE** — **Recover→Serve Continuity and Forge Admissibility** (the CE-A5 manifest: a real
> `cardano-node 11.0.1` relay `AddedToCurrentChain` an Ade-forged successor block). Counts at `a76672b9`: 340
> rules, 141 CI gates, 458 canonical types. +4 enforced rules (`DC-NODE-14`, `DC-NODE-15`, `DC-CONS-24`,
> `DC-PROTO-10`) + 9 strengthenings; CI 138 → 141 (+3). **BLUE-additive, +0 canonical type.** **CE-A5 is
> recorded as `enforced`-backing evidence, NOT a `RO-LIVE` flip.** The full §§0–7 narrative is recoverable
> from this doc's git history at `a76672b9` / `62811a4e`.

---

## Historical — earlier windows (`c6e7fafe → 1d54abb4` and before)

> Preserved as pointers. The **PHASE4-N-AC cluster** (`c6e7fafe → 1d54abb4`, KES signing evolves the
> operator key to the current period before signing — `DC-CRYPTO-10`; 335 → 336 rules / 137 → 138 CI gates
> at `1d54abb4`; RED-only, 458 types unchanged); the **PHASE4-N-AB cluster** (`b0365df0 → c6e7fafe`,
> outbound mux segmentation — `CN-SESS-05`; 334 → 335 rules / 136 → 137 CI gates at `c6e7fafe`; GREEN-only);
> the **PHASE4-N-AA cluster** (`999199f8 → b0365df0`, bounded peer-driven serve range — `DC-SERVEMEM-01`;
> 333 → 334 rules / 135 → 136 CI gates at `b0365df0`; RED-only); the **PHASE4-N-U cluster CLOSE +
> gate-hygiene tail** (`4e358e92 → 999199f8`, 333 rules / 135 CI gates at `999199f8` — 11 gates repaired in
> place, 0 added/removed, 0 invariants weakened); the **PHASE4-N-U cluster** (`65954fa3 → 4e358e92`,
> forged-block durability — `DC-NODE-12`, `DC-CONS-23`, `DC-WAL-04`, `T-REC-05`, `DC-NODE-13`; one new RED
> module `served_chain_projection`; 328 → 333 rules); and the **G-K…G-R + C1 multi-cluster catch-up**
> (`550eec3a → 65954fa3`, eight clusters toward a live genesis-successor follower — 319 → 328 rules, 126 →
> 134 CI gates, the one BLUE canonical type `ArrayHead` 457 → 458 — the **last BLUE delta before this
> window's `WalEntry::RollBack` types**). The full §§0–7 narrative for each is recoverable from this doc's
> git history at the respective HEADs.

---

## Generation notes

### Regen `8e2c3672 → 5ec841c8` (PHASE4-N-AI live fork-choice rollback-follow wiring — current lead)

- **Baseline valid; one single-theme cluster + an unrelated docs commit, preceded by the N-AH baseline-bump
  chore.** Run against `8e2c3672` (the PHASE4-N-AH close, the prior HEAD_DELTAS HEAD), which `git rev-parse`
  resolves and `git merge-base 8e2c3672 HEAD` confirms is a strict ancestor of HEAD `5ec841c8` (`8e2c3672`
  carries no tag). The start-of-regen config baseline was already `8e2c3672`. The closer bumps
  `head_deltas_baseline` `8e2c3672 → 5ec841c8` as a **separate post-close step** (NOT performed by this regen).
- **Counts are mechanical (git/grep/ls):** commit log + `--shortstat` over `8e2c3672..HEAD` (**26** commits, no
  merges / **46** files / **+5350 / −53**); CI gate count via `ls ci/ci_check_*.sh | wc -l` + `git ls-tree -r
  --name-only <ref> ci/ | grep -c` at each ref (**148 → 157**; `--diff-filter=A` over `ci/ci_check_*.sh` = the
  nine new gates; `--diff-filter=M` and `--diff-filter=D` over `ci/` **empty**); registry rule count via `grep
  -cE '^\[\[rules\]\]'` at each ref (**347 → 354**; `comm`/`diff` of the sorted `id =` lists shows the seven
  additions `DC-NODE-23..29`, zero removals); registry status via `grep -E '^status = ' | sort | uniq -c`
  (**213 → 221 enforced**, **20 → 19 partial**, 114 declared unchanged); the enforced/partial delta reconciled
  by an `awk` id↔status `comm` showing the newly-enforced set is exactly `{CN-CONS-01, DC-NODE-23..29}` and the
  de-partialed set is exactly `{CN-CONS-01}`; strengthenings = **13** (an `awk` scan of `strengthened_in` lines
  shows `PHASE4-N-AI` appended to `CN-CONS-01`, `CN-CONS-03`, `CN-STORE-07`, `DC-CONS-03`, `DC-CONS-05`,
  `DC-CONS-06`, `DC-CONS-20`, `DC-NODE-05`, `DC-NODE-12`, `DC-NODE-20`, `DC-PUMP-01`, `T-REC-03`, `T-REC-05`);
  BLUE canonical types via `git grep -hE '^(pub )?(struct|enum) '` over the BLUE `core_paths` trees at each ref
  (**458 → 460**; the +2 in `ade_ledger::wal::event`).
- **First BLUE delta since G-N — +2 canonical types, three BLUE files (one `#[cfg(test)]`-only).** `git diff
  --name-status 8e2c3672..HEAD` over the BLUE trees shows exactly three BLUE files: `ade_ledger::wal::{event,
  replay, error, mod, store_trait}` (production — the new closed `WalEntry::RollBack` durable MARKER + its
  replay arm + the two BLUE payload types + the closed error) and `ade_core::consensus::fork_choice` (**only a
  `#[cfg(test)]` permutation proof — `@@ -340,4 +340,89 @@ mod tests`; production `select_best_chain`
  byte-identical**). `git diff --name-only … '**/Cargo.toml' 'Cargo.toml'` is empty (no feature-flag delta;
  the notable CLI-flag delta is an **addition**, `--participant-venue`, mutually exclusive with
  `--single-producer-venue`).
- **No new module — the five new `.rs` files are all tests.** `git diff --diff-filter=A --name-only … 'crates/**/*.rs'`
  lists `wal_rollback_ai_s1.rs` + four `ade_node` test files, all under `crates/*/tests/`. The new BLUE types +
  variant live **inside** the existing `ade_ledger::wal::event`; the new GREEN/RED types live **inside** the
  existing `ade_node::{node_sync, node_lifecycle}`.
- **Registry delta is +7 rules (all enforced) + 1 flip + 13 strengthenings, NOT a removal.** `DC-NODE-23..28`
  were declared at `/invariants` (`d92f9ce8`) then flipped declared→enforced at the close; `DC-NODE-29` was
  introduced **and** enforced by AI-S6 (`5ec841c8`); `CN-CONS-01` flipped `partial → enforced` (AI-S5); the
  sorted-id `comm` confirms zero removals. **`CN-CONS-03` was NOT flipped** — it stays `declared` (strengthened
  only), recorded faithfully against the cluster doc's close record (the plan intended a flip; the close held
  it, single-best-peer scope).
- **Classification note (TCB).** `ade_node::node_sync` is **GREEN** (pure/total/deterministic detector +
  resolver, no I/O); `ade_node::node_lifecycle` + `cli` + `main` are **RED** (the loop + CLI);
  `ade_runtime::{admission::wire_pump, recovery::restart}` and `ade_node::admission::bootstrap` are the **RED**
  shell; `ade_ledger::wal` + `ade_core::consensus::fork_choice` are **BLUE** (`core_paths`). `ade_node` is
  neither a BLUE `core_paths` crate nor `ade_runtime` (the RED shell crate); per the project's TCB scoping the
  new `ade_node` GREEN/RED types are non-BLUE.
- **No `RO-LIVE` flip; CE-AI-6 is operator-gated/vacuous.** N-AI's `DC-NODE-23..29` are recorded `enforced` for
  the single-best-peer rung-2 follow scope, backed by hermetic enforcement (the nine gates + unit/replay
  tests). CE-AI-6 (live convergence) is operator-gated, vacuous-until-committed, sha256-bound — **NOT** a
  bounty/preprod claim. No `RO-LIVE` registry status changed this span (`RO-LIVE-01` stays operator-gated /
  partial).
- **Normative docs unchanged this span.** `git diff --name-only 8e2c3672..HEAD` over the configured
  `normative_docs` (CE-79 statement + addendum, the three contract docs, CE-73 reclassification, `CLAUDE.md`)
  is empty — the §7 delta is entirely the invariant-registry change.
- **§1 commit log verbatim from `git log --oneline --no-merges` (newest first).** The per-slice synthesis is in
  §0/§3. Every subject carries a conventional-commits prefix; `cbad2ae3` (`docs(evidence): … pool registration
  manifest`) is **unrelated to N-AI** and recorded as such (docs-only, folded into the span by date range).
- **Doc-refresh state — CODEMAP current, SEAMS + TRACEABILITY one cluster STALE (refresh owed).** CODEMAP was
  regenerated to `5ec841c8` (verified by `grep -c`: it carries `DC-NODE-23` ×14 / `DC-NODE-24` ×3 / `DC-NODE-25`
  ×13 / `DC-NODE-26` ×3 / `DC-NODE-27` ×10 / `DC-NODE-28` ×11 / `DC-NODE-29` ×16 / `DC-PUMP-01` ×13 +
  `RollbackPoint`/`RollbackReason`/`WalEntry::RollBack` + BLUE count `460` + all nine new gates). **SEAMS +
  TRACEABILITY remain pinned at `5858288e`** (`grep -c DC-NODE-23/27/29` in TRACEABILITY = 0; SEAMS header still
  "458 canonical types, 148 CI checks at `5858288e`"). **Cross-reference warning surfaced in §5:** regenerate
  SEAMS + TRACEABILITY to `5ec841c8` as a follow-on this close; the registry holds the new bindings
  authoritatively in the interim (354 rules). No orphan gate — each of the nine enforces its named rule.
- **Working tree clean.** This regen runs *after* all 26 span commits (the AI-S6 close `5ec841c8` is HEAD for
  this window); `git status --short` shows only an untracked `.mithril-scratch/` (operator scratch, ignored).
  The remaining close-pass actions are the SEAMS + TRACEABILITY refresh and the baseline bump `8e2c3672 →
  5ec841c8` (both separate post-close steps; this regen does **not** touch `.idd-config.json` `head_deltas_baseline`).

### Regen `f87d0056 → 5858288e` (PHASE4-N-AG superseded + PHASE4-N-AH local-tip forge-base authority — prior lead)

- **Two pivoting single-theme clusters preceded by the N-AF close tail**, measured from `f87d0056` (the
  PHASE4-N-AF S1 close). **32** commits / **48** files / **+5155 / −743**; CI gates **143 → 148** (+5 new, 3
  modified in place, 0 removed); registry **343 → 347** (+4: `DC-NODE-19` declared + `DC-NODE-20/21/22`
  enforced; 9 strengthenings all `PHASE4-N-AH`; 0 removed; status 210 → 213 enforced, 113 → 114 declared, 20
  partial unchanged); BLUE canonical types **458 → 458** (GREEN+RED only — no BLUE file touched). N-AG was
  superseded-closed (hermetic core CE-AG-1..4 complete; live CE re-homed to N-AH CE-AH-6); the N-AF cert
  mechanism was retired (folded out `FirstOwnBlockServed`, deleted the parser + flag). All four grounding docs
  were regenerated at the N-AH close (paying the deferred N-AF + N-AG CODEMAP debt). No `RO-LIVE` flip. Full
  notes recoverable from this doc's git history at `5858288e`.
