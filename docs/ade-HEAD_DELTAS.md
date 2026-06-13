# Ade — HEAD Deltas (Changes Since Baseline)

> **Status:** Living architectural document. Regenerated; not hand-edited.
>
> Regenerate with `/head-deltas <baseline>` after every cluster close. Baseline is recorded in `.idd-config.json` `head_deltas_baseline`.

> Baseline: `31efec44` (PHASE4-N-AO S1 — peer-identity restoration, `DC-NODE-34`, 2026-06-11 18:54)
> HEAD: `862cd2cb` (cluster-close — flip `CN-CONS-03` enforced on the natural CE-AO-6 transcript, 2026-06-13 17:05)
> Span: **the PHASE4-N-AO cluster — the live multi-candidate fork-choice SELECT + adopt path (slices S2–S14, with S1 at the baseline tip)**. Ade now DECIDES a fork-choice win among competing live branches, PROVES the replacement branch (fetch → bind → link → validate), COMMITS the adoption, and re-converges — on a NATURAL two-producer partition-and-reconverge venue. **This is the cluster that flipped `CN-CONS-03` `declared → enforced`** (Cardano post-partition convergence) on a committed, sha256-pinned live transcript. New rules `DC-NODE-38..41`, `DC-PUMP-04`, `DC-EVIDENCE-04`, `DC-EVIDENCE-05`; declared→enforced flips `CN-CONS-03` + `DC-NODE-34..37`.
> **42 commits** (no merges), **50 files changed, +9797 / −98 lines**. **This span is GREEN+RED only — ZERO new BLUE canonical type and ZERO BLUE-tree change**: `git diff 31efec44..HEAD` over the configured BLUE `core_paths` trees is **empty** (the fork-choice authority `ade_core::consensus::fork_choice::select_best_chain` and the BLUE header authority `validate_and_apply_header` are REUSED unchanged — the SELECT is built on top of them, never inside them). All production code lands in **`ade_node`** (GREEN projections + RED orchestration: **+4795 / −37**) and a 22-line RED touch in **`ade_runtime::forward_sync::pump`**. **+6 new modules** (all in `ade_node`: `candidate_aggregator.rs`, `selector_state.rs`, `fork_switch.rs`, `lca_walk.rs` GREEN; `fair_merge.rs` RED; `post_switch_continuity.rs` GREEN + its `bin/`), **NO new crate** (still 11). **+11 CI gates, 6 modified in place, 0 removed** (162 → 173). **Registry 365 → 372** (+7 new rules, all enforced at HEAD; +10 strengthenings; **zero removals**). **No `Cargo.toml` change** this span (no new dependency, no feature flag); **no new CLI flag**.

> **Baseline note (load-bearing — read before §0).** This window's baseline is **`31efec44`**, the PHASE4-N-AO **S1**
> commit (peer-identity restoration, `DC-NODE-34`), and it is **valid**: `git rev-parse 31efec44` resolves and
> `git merge-base 31efec44 HEAD == 31efec44` (it is a strict ancestor of HEAD; `31efec44` carries no tag). HEAD is
> **`862cd2cb`** (the PHASE4-N-AO cluster-close that flips `CN-CONS-03`). The cluster's **declare** commits (`a87a4eb5`
> declaring `DC-NODE-34..37`) and the **S1 slice doc + S1 impl** (`301a4932`, `31efec44`) landed in the *preceding*
> grounding-regen window (`b8860b16..31efec44`, opened by the prior `f167a349` regen to `b8860b16`), so they sit at or
> *before* this baseline. **This regen therefore measures the SELECT cluster from the S1 tip forward (`31efec44..HEAD`)**
> — S1 itself (`DC-NODE-34`, `ci_check_peer_identity_preserved.sh`) is already enforced at the baseline and appears here
> only as the **Modified** gate the in-span stale-gate repair touched (§5). The span is **one cluster, end-to-end**:
> S2 (BLUE-safe candidate construction) → S3 (live selector dispatch, decide-only) → S4 (fork-switch apply, prove-then-commit)
> → S5 (reselection replay-equivalence) → S6 (live BlockFetch byte-only bridge) → S7 (LCA fork-anchor walk) → S8 (multi-peer
> wire-pump fairness) → S9 (closed fork-choice evidence + supersession) → S10 (post-switch branch-continuity) → S11
> (post-ForkChoiceWin forward-follow floor) → S12 (bridge-gap fault-injection harness) → S13 (rolled-back branch evidence
> retention) → S14 (missing-bridge range re-fetch) → close (flip `CN-CONS-03`).
>
> **Working-tree note.** At this regen the working tree is **CLEAN** for tracked files — `git status --porcelain` shows
> only untracked scratch (`.mithril-scratch/`, `wire_smoke.jsonl`), neither part of this doc. The PHASE4-N-AO close
> (`862cd2cb` — the `CN-CONS-03`/`DC-NODE-34..37` flips, the 7 new-rule enforcements, the repointed
> `ci_check_convergence_evidence_vocabulary_closed.sh`) is committed. §1 narrates the committed span `31efec44..862cd2cb`
> verbatim from `git log`; §0/§7 read rule **status** from the registry at HEAD `862cd2cb` (`CN-CONS-03` **enforced**,
> **372** rules). The operator bumps `head_deltas_baseline` `b8860b16 → 862cd2cb` as the **post-close step this regen
> performs** (and demotes the N-AM/N-AN paragraph to "PRIOR baseline"). **NB:** the *committed* `.idd-config.json`
> baseline still reads `b8860b16` (the prior window's value); this regen advances it to `862cd2cb`.

This window is **a single cluster — PHASE4-N-AO — that turns the prior single-best-peer rollback-FOLLOW (N-AI/N-AJ,
`DC-NODE-23..30`) into a genuine multi-candidate SELECT**: among two live competing branches, Ade DECIDES a fork-choice
winner, PROVES the replacement branch, COMMITS the adoption, and re-converges. It is the cluster that earned the
**`CN-CONS-03` flip** (the Cardano-specific post-partition convergence rule). The mechanism decomposes into four
authority bands:

1. **Decide (GREEN, BLUE-fed).** `candidate_aggregator` builds a `CandidateFragment` for each competing branch by
   validating every header through the BLUE `validate_and_apply_header` authority; `selector_state` projects Ade's own
   durable tip into the `ChainSelectorState`; the RED `dispatch_competing_fork_choice` calls the *unchanged* BLUE
   `select_best_chain` (`CN-CONS-03`/`DC-CONS-03`) and emits a provisional `PendingForkSwitch` — **decide-only, no
   rollback yet** (`DC-NODE-35`, `DC-NODE-36`).
2. **Prove, then commit (GREEN core + RED apply).** `fork_switch::prevalidate_branch` is a PURE proof that the fetched
   replacement bodies bind to the S3-selected headers, link from the durable fork anchor, and ledger-validate; only on a
   complete proof does the RED `apply_fork_switch` perform the durable rollback + adopt — **a `PendingForkSwitch` is
   authority to *attempt proof*, never to roll back** (`DC-NODE-37`, replay-equivalent reselection `CE-AO-5`).
3. **Reach the anchor + fetch the branch (RED bytes / GREEN walk).** A live competing branch is multi-block, so its
   immediate parent is an intermediate block Ade never stored; `lca_walk` walks the branch's preserved parent links back
   to the durable last-common-ancestor (`DC-NODE-38`), `fair_merge` gives each peer its own bounded lane so a hot peer
   can't starve a competing one (`DC-PUMP-04`), and the S6 live BlockFetch bridge transports the branch **bytes only —
   never truth** (`CE-AO-6`); when the bridge to the winner is missing, the path fails closed (`DC-NODE-39`), retains the
   rolled-back evidence the walk needs (`DC-NODE-40`), and re-fetches the missing range (`DC-NODE-41`).
4. **Prove the SELECT on a committed transcript (GREEN evidence).** S9 adds a **closed, observe-only**
   convergence-evidence vocabulary (`needs_fork_choice` → `lca_discovered` → `candidate_fragment_built` →
   `fork_choice_selected` → `branch_fetch_started/completed` → `branch_prevalidated` →
   `fork_switch_applied|failed|superseded`) so a *committed* JSONL transcript carries the SELECT middle, with every
   fork-choice win paired to exactly one terminal (`DC-EVIDENCE-04`); S10's `post_switch_continuity` is a replayable
   reducer that classifies Ade's own post-switch admitted lineage into a closed verdict (`DC-EVIDENCE-05`). The close ran
   the natural CE-AO-6 two-producer pass against this checker and **flipped `CN-CONS-03`**.

With the full path the **CE-AO-6 multi-candidate SELECT** ran live and is the FIRST `RO`/`CN`-tier convergence flip of
the cluster (`CN-CONS-03 → enforced`), on a NATURAL (no loser-freeze) transcript pinned by sha256 OUTSIDE the repo.

### PHASE4-N-AO — the decide band (`DC-NODE-35`, `DC-NODE-36`, enforced)

> **The SELECT decision is built ON TOP of the BLUE fork-choice authority, never inside it.** `select_best_chain`
> (`crates/ade_core/src/consensus/fork_choice.rs`, `CN-CONS-03`/`DC-CONS-03`) is REUSED byte-for-byte; the BLUE tree is
> untouched this span. The GREEN `candidate_aggregator` (`DC-NODE-35`) is a PURE projection: given a fork anchor, the
> chain-dep AT that anchor, and a peer's candidate header inputs, it validates each header through the BLUE
> `validate_and_apply_header` authority and assembles a `CandidateFragment` — **no minting, no store reads, no durable
> mutation, no nondeterminism** (`OQ-AO-6 → GREEN`). The GREEN `selector_state` (`DC-NODE-36` half) derives the
> `TiebreakerView` of a block **from Ade's own already-admitted durable tip bytes** — local durable authority, never the
> peer tip — and carries a provisional decision toward S4. The RED `dispatch_competing_fork_choice` /
> `decide_fork_switch` route a `NeedsForkChoice` competing block into `select_best_chain` and emit a `PendingForkSwitch`
> on a win — **decide-only**: no rollback, no WAL entry, no ledger/chain-dep/cursor mutation at decide time.

- **PHASE4-N-AO / S2 / `DC-NODE-35` (enforced) (GREEN BLUE-safe candidate construction).** New module
  `crates/ade_node/src/candidate_aggregator.rs` (**+409**, GREEN). A pure projection assembling `CandidateFragment`s
  exclusively from `validate_and_apply_header` output (the BLUE header authority); performs no store reads, no minting,
  no durable mutation, no wall-clock/`rand`/`HashMap`. The hard boundary (`DC-NODE-35`): candidate construction can never
  manufacture a header — fragments come ONLY from the BLUE validator. New gate
  `ci_check_candidate_construction_validated.sh`. (Commit `6bcfc9e5`.)
- **PHASE4-N-AO / S3 / `DC-NODE-36` (enforced) (live selector dispatch — decide-only).** New module
  `crates/ade_node/src/selector_state.rs` (**+167**, GREEN — the projection foundation, `986d8339`); the RED dispatch
  driver lands in `node_lifecycle.rs` (`a8c12327`) as `dispatch_competing_fork_choice` + `decide_fork_switch` + the
  `ForkSwitchDecision` enum. The live `NeedsForkChoice` arm of `run_participant_sync` routes a competing block into the
  BLUE `select_best_chain` and emits a provisional `PendingForkSwitch` (carrying the winner tip) — **the decision is
  Ade's, computed only from local durable authority + the protocol observables `select_best_chain` consumes**; the
  conservative floor (the S3 hard rule) keeps the current chain on any incomplete decision. New gate
  `ci_check_live_selector_dispatch.sh`; the `k` source-of-authority wording was corrected in `cd11c256`. (Commits
  `1939165e`, `04f11013`, `986d8339`, `a8c12327`, `cd11c256`.)

### PHASE4-N-AO — the prove-then-commit band (`DC-NODE-37`, `CE-AO-5`, enforced)

> **A `PendingForkSwitch` is not authority to roll back; it is only authority to *attempt proof* of the selected
> replacement branch.** S4 turns the S3 provisional decision into a durable adoption ONLY by proving the complete
> replacement branch — fetched bodies, bound to the S3-selected headers, linked from the durable fork anchor, and
> ledger-validated — and only then committing the rollback + adopt. `fork_switch::prevalidate_branch` is PURE (no I/O, no
> store reads, no durable mutation); the durable mutation lives in the RED `apply_fork_switch`, which runs the proof
> first and commits second. The reselection that follows is **replay-equivalent** (`CE-AO-5`): the same recovered store +
> the same ordered branch ⇒ the same post-switch durable state and the same evidence verdict.

- **PHASE4-N-AO / S4 / `DC-NODE-37` (enforced) (fork-switch apply — prove, then commit).** New module
  `crates/ade_node/src/fork_switch.rs` (**+555**) shipping the GREEN PURE proof core (`prevalidate_branch`,
  `BranchProofError`) + the RED `prove_fork_switch` / `apply_fork_switch` / `map_branch_proof_failure` in
  `node_lifecycle.rs`. The proof binds the fetched bodies to the S3-selected headers, links them from the durable fork
  anchor, and ledger-validates them via the read-only materialize; only `ForkSwitchOutcome::Adopted` commits the durable
  rollback. New gate `ci_check_fork_switch_never_abandons.sh` (a `PendingForkSwitch` authorizes proof, not a rollback).
  (Commits `cabb94b8`, `d63b5dac`, `5e4807e2`.)
- **PHASE4-N-AO / S5 / `CE-AO-5` (reselection replay-equivalence + fence resolution).** New test file
  `crates/ade_node/tests/reselection_replay_s5.rs` (**+306**) proves the post-switch reselection is replay-equivalent and
  resolves the no-forge-across-pending-reselection fence; the modified gate `ci_check_wal_rollback_replay_equiv.sh` was
  extended for the reselection path. (Commits `2490ef07`, `5b31bf7f`.)

### PHASE4-N-AO — the reach-and-fetch band (`DC-NODE-38`, `DC-PUMP-04`, `CE-AO-6`, `DC-NODE-39..41`, enforced)

> **A live competing branch is multi-block, so the SELECT must reach a DURABLE anchor and transport branch bytes without
> ever transporting truth.** The competing block's immediate parent is an intermediate block on the competing branch Ade
> never stored; the fork anchor is the **last common ancestor (LCA)** — a durable `ChainDb`-stored block — reached by
> walking the competing branch's preserved parent links (`lca_walk`, `DC-NODE-38`; the per-peer cache is NOT authority,
> only an indexed memory of received, preserved headers). When several peers feed one shared bounded channel, a
> continuously-producing peer monopolises it and starves the others, so a competing peer's branch never reaches dispatch;
> `fair_merge` (`DC-PUMP-04`) gives each peer its OWN bounded lane and drains them with a deterministic round-robin merge
> — **scheduling discipline ONLY, never fork-choice** (`select_best_chain` stays arrival-order-independent, `CN-CONS-01`).
> The S6 live BlockFetch bridge fills `PrefetchedBranchBodies` from a live `RequestRange` and carries **bytes only — no
> selection, no admission** (`CE-AO-6`). A winner-descendant whose bridge to the durable adopted tip is missing fails
> closed with a structured `MissingBridge` (`DC-NODE-39`, no silent stall), retains the rolled-back blocks the walk needs
> as walk-visible EVIDENCE (`DC-NODE-40`), and actively re-fetches the missing range (`DC-NODE-41`).

- **PHASE4-N-AO / S6 / `CE-AO-6` (live BlockFetch byte-only bridge).** The byte-only bridge core
  (`PrefetchedBranchBodies` + boundary proofs, `08b2aebc`); `PendingForkSwitch` extended to carry `winner_tip` as the
  BlockFetch endpoint (`9a85ab93`); the live BlockFetch fetch + relay-loop fill + evidence wiring (`3e0a6ad6`, touching
  `node_lifecycle.rs` `prefetch_branch_bodies` + `node_spine_serve_loopback.rs`). New gate
  `ci_check_live_blockfetch_byte_only.sh` (the bridge transports bytes, not truth). (Commits `1f16ff7f`, `08b2aebc`,
  `9a85ab93`, `3e0a6ad6`, `c841f0b5`.)
- **PHASE4-N-AO / S7 / `DC-NODE-38` (enforced) (live LCA fork-anchor walk).** New module
  `crates/ade_node/src/lca_walk.rs` (**+411**, GREEN — read-only `ChainDb` lookups only, no durable mutation). A live
  competing branch is eligible for SELECT only when Ade walks its preserved parent links back to a DURABLE STORED anchor
  under a block-depth `k`-bound, with per-peer branch caching and a multi-header candidate that grows as the branch
  grows. New gate `ci_check_lca_anchor_walk.sh`. (Commits `0cce1668`, `3b03b967`, `cabe61ff`.)
- **PHASE4-N-AO / S8 / `DC-PUMP-04` (enforced) (multi-peer wire-pump fairness).** New module
  `crates/ade_node/src/fair_merge.rs` (**+244**, RED). Per-peer bounded lanes + a deterministic round-robin `fair_merge`
  (rotating cursor, closed-lane retire-in-place; no `HashMap`/wall-clock/`rand`); the per-peer pump is rewired so the
  shared fan-in is gone. **A retry RAN and the channel-fairness layer was found to be the WRONG layer** — the real live
  blocker was an evidence-attribution artifact (see the S8.5 fix below); S8 stays correct + proven (a cleaner per-peer
  arch) but was not the blocker. New gate `ci_check_wire_pump_fairness.sh`. (Commits `fc3db0f5`, `4c64e779`,
  `901650b2`.)
- **PHASE4-N-AO / S8.5 (`6846d252`, the evidence-artifact fix).** `block_received` was mislabelling every block to the
  FIRST peer (`convergence_evidence::emit_block_received` used a fixed `peer_label` instead of the per-block
  `NodeSyncItem::Block.peer`); the fix threads the per-block peer through `emit_block_received(peer, slot, hash)`. This
  overturned the S7-retry "channel starvation" and S8 "fairness/2-pump stall" diagnoses — multi-peer delivery was never
  broken; both pumps always worked. (`convergence_evidence.rs` **+416** carries this + the S9 emitters; `fix` commit
  `6846d252`.)
- **PHASE4-N-AO / S11 / `DC-NODE-39` (enforced) (post-ForkChoiceWin forward-follow floor).** After a `ForkChoiceWin`
  adoption at tip X, a competing descendant whose parent chain cannot connect to the durable adopted tip / a durable
  stored ancestor is a **structured `MissingBridge` fail-closed** — no silent stall. New gate
  `ci_check_missing_bridge_fail_closed.sh`. (Commits `fccebb94`, `eff880aa`, `ab47c338`; run-1 root cause in `08c2bc5b`.)
- **PHASE4-N-AO / S12 / `DC-NODE-39` regression (`66312da0`) (bridge-gap fault-injection harness).** A deterministic
  fault-injection harness exercising the S11 missing-bridge floor. (Delegated test infra per the inline-vs-delegated
  discipline; production fix S11 is inline.)
- **PHASE4-N-AO / S13 / `DC-NODE-40` (enforced) (rolled-back branch evidence retention).** Rolled-back blocks may be
  retained ONLY as walk-visible EVIDENCE: the LCA walk consults the retention on a per-peer-cache MISS to traverse
  non-durable parent links — **fixes the S7 LCA-walk over-fire** (the walk previously failed to reach the anchor across a
  rolled-back segment). New gate `ci_check_rollback_retention_evidence.sh`. (Commits `f1ca350d`, `e80d4226`.)
- **PHASE4-N-AO / S14 / `DC-NODE-41` (enforced) (missing-bridge range re-fetch).** The `DC-NODE-39` floor is SAFE but
  PASSIVE — ChainSync streams each block once, so a winner-descendant whose bridge Ade missed can never be recovered by
  waiting. S14 adds a latent range-recovery admit loop (`recover_missing_range` in `node_lifecycle.rs`, with a closed
  outcome, `bb7ed9dd`) then wires the missing-bridge range re-fetch live (`2a03ac73`). New gate
  `ci_check_missing_bridge_refetch.sh`. (Commits `6369af30`, `bb7ed9dd`, `2a03ac73`.)

### PHASE4-N-AO — the closed-evidence band (`DC-EVIDENCE-04`, `DC-EVIDENCE-05`, enforced)

> **A committed transcript must carry the SELECT middle in a CLOSED, observe-only vocabulary, with every fork-choice win
> paired to exactly one terminal.** The prior `AgreementVerdict` vocabulary (`block_received` / `block_admitted` /
> `agreement_verdict`, N-AJ) recorded only the endpoints; the SELECT (decide → fetch → prove → apply) was stderr
> diagnostics. S9 adds 10 closed `AdmissionLogEvent` variants — `needs_fork_choice`, `lca_discovered`,
> `candidate_fragment_built`, `fork_choice_selected`, `branch_fetch_started`, `branch_fetch_completed`,
> `branch_prevalidated`, `fork_switch_applied`, `fork_switch_failed`, `fork_switch_superseded` — emitted **observe-only**
> (GREEN vocab / RED sink / BLUE unchanged) so a *committed* JSONL transcript ASSERTS the SELECT. Every
> `fork_choice_selected{win}` pairs to EXACTLY ONE terminal of `applied | failed | superseded` (`DC-EVIDENCE-04`):
> because the `fork_switch_id` includes the winner tip, a branch that grows produces a new win per tip, so superseded
> provisional wins emit a `fork_switch_superseded` terminal while only the final pending reaches `applied`. S10's
> `post_switch_continuity` is a PURE replayable reducer over the closed transcript: after a `ForkChoiceWin` adoption at
> tip X it classifies Ade's OWN validated admitted-block lineage into a closed verdict — `ContinuesSelectedBranch`
> requires unbroken `prev_hash` lineage from X across every post-X `block_admitted`, no `diverged` after X, and every win
> paired to a terminal (`DC-EVIDENCE-05`). The peer tip is NEVER an input to the reducer.

- **PHASE4-N-AO / S9 / `DC-EVIDENCE-04` (enforced) (closed fork-choice evidence vocabulary + taps + supersession).**
  Part 1 (`d28d665f`, latent): 10 closed `AdmissionLogEvent` variants + discriminators + writer serialization + the
  `DISCRIMINATORS` allow-list + the closed `ForkChoiceResult` / `ForkChoiceEvidenceFailure` enums
  (`admission_log/event.rs` **+196**, `admission_log/writer.rs` **+236**). Part 2 (`c0bae25e`): the observe-only emit
  taps in `dispatch_competing_fork_choice` (DECIDE half) + the relay loop (APPLY half), with a `fork_switch_id` helper
  (blake2b of peer + fork anchor + winner tip). Supersession (`a3011d71`): the 10th terminal `fork_switch_superseded`,
  so every win pairs to exactly one of `applied | failed | superseded`. The post-switch convergence window for CE-AO-6
  (`028b287a`). New gate `ci_check_fork_choice_evidence_closed.sh`. (Commits `a77cace4`, `d28d665f`, `c0bae25e`,
  `a3011d71`, `028b287a`.)
- **PHASE4-N-AO / S10 / `DC-EVIDENCE-05` (enforced) (post-switch branch-continuity reducer).** New module
  `crates/ade_node/src/post_switch_continuity.rs` (**+676**, GREEN) + its `bin/post_switch_continuity.rs` (**+66**) — a
  pure, total, deterministic reducer producing a closed `PostSwitchContinuity` verdict (closed sum, no free-form
  strings; reads ONLY Ade's own admitted lineage, never the peer tip; replay-equivalent). New gate
  `ci_check_post_switch_convergence_window.sh` (the RELEASE/EVIDENCE-tier transcript checker — NOT a BLUE consensus
  rule). Run-1 (`08c2bc5b`) recorded a real fork-switch fired (not a flip) and surfaced the S11 continuity gap. (Commits
  `4c4b5849`, `811c8114`, `08c2bc5b`.)

### Cluster close — `CN-CONS-03` flip (`862cd2cb`)

- **The close (`862cd2cb`) flipped `CN-CONS-03` `declared → enforced`** (`strengthened_in += PHASE4-N-AO`) on a
  **NATURAL** two-producer multi-candidate SELECT pass (CE-AO-6): both Haskell producers (cn1/cn2, testnet-magic 42)
  live throughout, NO loser-freeze, NO post-fork operator intervention — the SELECT decision is entirely Ade's. The
  `ci_check_post_switch_convergence_window.sh` checker (over the `post_switch_continuity` replayable reducer) PASSED:
  `ContinuesSelectedBranch`, terminal `AgreedAtSwitchTip{slot 391}` (exact agreement, `our_hash == peer_hash`), 25
  admitted descendants chained, 0 diverged, every fork-choice win terminal. The transcript
  (`ao-CN-CONS-03-FLIP-natural-conv.jsonl`, sha256 `6713efe9…ca13f`, captured 2026-06-13) is preserved **OUTSIDE the
  repo** (competition-secrecy / no-credential-leak discipline; sha256-pinned in `CN-CONS-03.evidence_notes`). The close
  also flipped `DC-NODE-34..37 → enforced`, enforced the 7 new rules, and **repointed**
  `ci_check_convergence_evidence_vocabulary_closed.sh` to extend the AJ-era 3-literal allow-list with all 10 new
  fork-choice literals and re-anchor its Guard 5 to the writer `DISCRIMINATORS` allow-list.

**The BLUE tree is UNTOUCHED this span (GREEN+RED only)** — `git diff 31efec44..HEAD` over the configured BLUE
`core_paths` trees is **empty**. The SELECT reuses the BLUE fork-choice authority (`select_best_chain`,
`ade_core::consensus::fork_choice`) and the BLUE header authority (`validate_and_apply_header`) unchanged; all new code
is GREEN (pure projections / reducers) or RED (orchestration / wire) in `ade_node` + one RED touch in
`ade_runtime::forward_sync::pump`. **No `RO-LIVE` rule flipped** — `RO-LIVE-01` stays operator-gated. The headline flip
is **`CN-CONS-03`** (a `CN`-family Cardano-convergence rule), on a committed, sha256-pinned NATURAL transcript.

## 0. Headline

| Count | Baseline (`31efec44`) | HEAD (`862cd2cb`) | Δ |
|---|---|---|---|
| CI gates (`ci/ci_check_*.sh`) | 162 | **173** | **+11** new (S2 `candidate_construction_validated`, S3 `live_selector_dispatch`, S4 `fork_switch_never_abandons`, S6 `live_blockfetch_byte_only`, S7 `lca_anchor_walk`, S8 `wire_pump_fairness`, S9 `fork_choice_evidence_closed`, S11 `missing_bridge_fail_closed`, S13 `rollback_retention_evidence`, S14 `missing_bridge_refetch`, CE-AO-6 `post_switch_convergence_window`). **6 modified in place** (`convergence_evidence_vocabulary_closed`, `live_fork_choice_apply`, `live_fork_choice_wiring`, `peer_identity_preserved`, `wal_rollback_replay_equiv`, `wire_rollback_signal_preserved`). **0 removed** (`--diff-filter=D` over `ci/` is empty; `ls ci/ci_check_*.sh \| wc -l` = 162 → 173). |
| Registry rules (`docs/ade-invariant-registry.toml`) | 365 | **372** | **+7** new rules `DC-NODE-38`, `DC-NODE-39`, `DC-NODE-40`, `DC-NODE-41`, `DC-PUMP-04`, `DC-EVIDENCE-04`, `DC-EVIDENCE-05`. **Zero removed** (`comm -23` of the sorted `id =` lists is empty). |
| Registry status (enforced / enforced_scaffolding / partial / declared) | 227 / 1 / 19 / 118 | **239 / 1 / 19 / 113** | **+12 enforced**, **−5 declared** (`enforced_scaffolding=1`, `partial=19` unchanged). Reconciliation: the 7 new rules close `enforced` (+7 enforced; declared at their slice docs then flipped at close, net 0 declared) **and** 5 prior-`declared` rules flipped `enforced` this span (`CN-CONS-03` + `DC-NODE-34..37`; +5 enforced, −5 declared). Net: +12 enforced, −5 declared. |
| **`CN-CONS-03` (Cardano post-partition convergence)** | `declared` | **`enforced`** | **THE flip.** `strengthened_in` `["PHASE4-N-B","PHASE4-N-AI"]` → `["PHASE4-N-B","PHASE4-N-AI","PHASE4-N-AO"]`; enforced on the natural CE-AO-6 transcript (sha256 `6713efe9…`, OUTSIDE-repo). |
| Registry strengthenings | — | **+10** | `strengthened_in += PHASE4-N-AO` on **CN-CONS-01**, **CN-CONS-03**, **DC-CONS-03**, **DC-CONS-20** (the convergence / arrival-order-independence family — the SELECT exercises them live) and **DC-NODE-24..29** (the N-AI single-best-peer rollback-follow family — the SELECT's prove-then-commit re-exercises the rollback authority). No rule weakened; no rule removed. |
| BLUE canonical types | 462 | **462** | **±0** — the BLUE tree is **untouched** (`git diff 31efec44..HEAD` over the BLUE `core_paths` trees is empty). The SELECT reuses `select_best_chain` + `validate_and_apply_header` unchanged. CODEMAP's BLUE-tree metric **462 → 462**. Still 11 crates (no `Cargo.toml` change this span). |
| Grounding docs | CODEMAP / SEAMS / TRACEABILITY all regenerated to **`b8860b16`** by the prior `f167a349` regen (462 canonical types / 161 CI / 361 rules) | Now **one cluster (PHASE4-N-AO) stale**: none carries the SELECT's 6 new modules or 7 new rules (`grep -c` in each = 0 for `candidate_aggregator` / `fork_switch` / `lca_walk` / `selector_state` / `post_switch_continuity` / `fair_merge` and for `DC-NODE-38..41` / `DC-PUMP-04` / `DC-EVIDENCE-04/05`); their CI pin reads **161** vs. HEAD **173**. | **CODEMAP + SEAMS + TRACEABILITY are now ONE cluster STALE** — they MISS 6 new modules, 7 new rules, the `CN-CONS-03` flip, and 11 new CI gates. The registry holds all of it authoritatively at HEAD (**372 rules**); the refresh to `862cd2cb` is the named follow-on this close. See the cross-reference warnings at the end of §2 and §5. |

> **Grounding-doc state this close (load-bearing).** **CODEMAP, SEAMS, and TRACEABILITY were all regenerated to
> `b8860b16`** (the prior N-AM/N-AN window's HEAD, by `f167a349`), so they pin to `b8860b16` / 462 types / 161 CI / 361
> rules and carry `DC-PUMP-03` + `T-REC-06` but **nothing from PHASE4-N-AO**. They are now **one cluster stale**: the
> SELECT introduced **6 new modules**, **7 new rules**, **10 strengthenings**, the **`CN-CONS-03` flip**, and **11 new CI
> gates** — none of which appear in CODEMAP/SEAMS/TRACEABILITY (`grep -c` for each new module/rule = 0; CI pin 161 vs.
> HEAD 173). This is the **largest grounding-doc refresh debt** of the recent windows (prior windows added 0 modules);
> the invariant registry holds all of it authoritatively at HEAD (**372 rules**). **Action:** regenerate CODEMAP + SEAMS
> + TRACEABILITY to `862cd2cb` as a follow-on this close so the 6 new modules, the 7 new rules with their named gates,
> the 10 strengthenings, and the `CN-CONS-03` flip all appear, and all three docs pin to the N-AO HEAD. Until then the
> registry is authoritative for the new bindings.

The slice↔rule↔gate map for this window (the full verbatim log is §1; S1 sits at the baseline tip):

| Slice | Rule(s) | Gate | What shipped |
|---|---|---|---|
| **N-AO declare + S1** (`a87a4eb5`, `301a4932`, `31efec44`) | declare `DC-NODE-34..37`; **`DC-NODE-34` enforced** | `ci_check_peer_identity_preserved.sh` (S1) | **At/before the baseline.** S1 (peer-identity restoration — `NodeSyncItem` carries `peer`) is the baseline tip; this regen measures from S1 forward, so `DC-NODE-34` is already enforced at `31efec44`. The gate appears as **Modified** in §5 (the stale-gate repair touched it). |
| **S2** (`6bcfc9e5`) | **`DC-NODE-35`** (NEW module, → enforced) | `ci_check_candidate_construction_validated.sh` (NEW) | **GREEN BLUE-safe candidate construction.** New module `candidate_aggregator.rs` (+409) — pure projection from `validate_and_apply_header`. |
| **S3** (`986d8339`, `a8c12327`, `cd11c256`) | **`DC-NODE-36`** (NEW module, → enforced) | `ci_check_live_selector_dispatch.sh` (NEW) | **Live selector dispatch — decide-only.** New module `selector_state.rs` (+167, GREEN) + RED `dispatch_competing_fork_choice` in `node_lifecycle.rs`; calls the unchanged BLUE `select_best_chain`, emits a provisional `PendingForkSwitch`. |
| **S4** (`d63b5dac`, `5e4807e2`) | **`DC-NODE-37`** (NEW module, → enforced) | `ci_check_fork_switch_never_abandons.sh` (NEW) | **Fork-switch apply — prove, then commit.** New module `fork_switch.rs` (+555) — PURE `prevalidate_branch` + RED `apply_fork_switch`. |
| **S5** (`5b31bf7f`) | `CE-AO-5` (reselection replay-equiv) | `ci_check_wal_rollback_replay_equiv.sh` (Modified) | Reselection replay-equivalence + fence resolution; new test file `reselection_replay_s5.rs` (+306). |
| **S6** (`08b2aebc`, `9a85ab93`, `3e0a6ad6`) | `CE-AO-6` (byte-only bridge) | `ci_check_live_blockfetch_byte_only.sh` (NEW) | **Live BlockFetch byte-only bridge.** `PrefetchedBranchBodies` + boundary proofs; `PendingForkSwitch` carries `winner_tip`; relay-loop fill via `prefetch_branch_bodies`. |
| **S7** (`3b03b967`) | **`DC-NODE-38`** (NEW module, → enforced) | `ci_check_lca_anchor_walk.sh` (NEW) | **Live LCA fork-anchor walk.** New module `lca_walk.rs` (+411, GREEN) — read-only ChainDb walk to the durable LCA under a `k`-bound. |
| **S8** (`4c64e779`) | **`DC-PUMP-04`** (NEW module, → enforced) | `ci_check_wire_pump_fairness.sh` (NEW) | **Multi-peer wire-pump fairness.** New module `fair_merge.rs` (+244, RED) — per-peer lanes + round-robin merge. (NOT the live blocker — see S8.5.) |
| **S8.5** (`6846d252`) | (`DC-NODE-34` fidelity) | — | **The evidence-artifact fix.** `block_received` per-block peer attribution; overturned the channel-fairness / 2-pump-stall diagnoses. |
| **S9** (`d28d665f`, `c0bae25e`, `a3011d71`, `028b287a`) | **`DC-EVIDENCE-04`** (NEW, → enforced) | `ci_check_fork_choice_evidence_closed.sh` (NEW) | **Closed fork-choice evidence vocabulary + taps + supersession.** 10 closed `AdmissionLogEvent` variants; observe-only emit taps; `fork_switch_superseded` terminal (every win pairs). |
| **S10** (`811c8114`, `08c2bc5b`) | **`DC-EVIDENCE-05`** (NEW module, → enforced) | `ci_check_post_switch_convergence_window.sh` (NEW) | **Post-switch branch-continuity reducer.** New module `post_switch_continuity.rs` (+676, GREEN) + bin — replayable closed verdict over Ade's own lineage. |
| **S11** (`ab47c338`) | **`DC-NODE-39`** (NEW, → enforced) | `ci_check_missing_bridge_fail_closed.sh` (NEW) | **Post-ForkChoiceWin forward-follow floor.** Structured `MissingBridge` fail-closed; no silent stall. |
| **S12** (`66312da0`) | `DC-NODE-39` regression | — (extends S11 gate) | **Bridge-gap fault-injection harness.** Deterministic regression for the S11 floor. |
| **S13** (`e80d4226`) | **`DC-NODE-40`** (NEW, → enforced) | `ci_check_rollback_retention_evidence.sh` (NEW) | **Rolled-back branch evidence retention.** Walk-visible retention; fixes the S7 LCA-walk over-fire. |
| **S14** (`bb7ed9dd`, `2a03ac73`) | **`DC-NODE-41`** (NEW, → enforced) | `ci_check_missing_bridge_refetch.sh` (NEW) | **Missing-bridge range re-fetch.** `recover_missing_range` admit loop + live wire. |
| **close** (`862cd2cb`) | **`CN-CONS-03` enforced** + `DC-NODE-34..37` enforced + 7 new rules enforced + 10 strengthenings | `ci_check_convergence_evidence_vocabulary_closed.sh` (Modified — repointed) | **Flip `CN-CONS-03`** on the natural CE-AO-6 transcript. |

## 1. Commit Log (newest first)

| Hash | Type | Summary |
|------|------|---------|
| `862cd2cb` | docs | docs(phase4-n-ao): cluster-close -- flip CN-CONS-03 enforced on the natural CE-AO-6 transcript |
| `2a03ac73` | feat | feat(phase4-n-ao): S14 part 2 -- wire the missing-bridge range re-fetch live (DC-NODE-41) |
| `bb7ed9dd` | feat | feat(phase4-n-ao): S14 part 1 -- latent range-recovery admit loop + closed outcome (DC-NODE-41) |
| `6369af30` | docs | docs(phase4-n-ao): scope S14 missing-bridge range re-fetch (declare DC-NODE-41) |
| `e80d4226` | feat | feat(phase4-n-ao): S13 rolled-back branch evidence retention -- fixes the LCA-walk over-fire (DC-NODE-40) |
| `f1ca350d` | docs | docs(phase4-n-ao): scope S13 rolled-back branch evidence retention (declare DC-NODE-40) |
| `66312da0` | test | test(phase4-n-ao): S12 bridge-gap fault-injection harness (deterministic DC-NODE-39 regression) |
| `ab47c338` | feat | feat(phase4-n-ao): S11 DC-NODE-39 floor -- structured MissingBridge fail-closed, no silent stall |
| `eff880aa` | docs | docs(phase4-n-ao): scope S11 post-ForkChoiceWin forward-follow continuity (declare DC-NODE-39) |
| `fccebb94` | docs | docs(phase4-n-ao): run-1 root cause -- post-switch chain HOLE on the winner, not a wire gap (scope S11) |
| `08c2bc5b` | docs | docs(phase4-n-ao): record S10 run 1 -- real fork-switch fired, not a flip (continuity gate) |
| `811c8114` | feat | feat(phase4-n-ao): S10 post-switch branch-continuity reducer + prev_hash evidence (DC-EVIDENCE-05) |
| `4c4b5849` | docs | docs(phase4-n-ao): scope S10 post-switch branch-continuity evidence (declare DC-EVIDENCE-05) |
| `028b287a` | feat | feat(phase4-n-ao): bounded post-switch convergence window for CE-AO-6 (DC-EVIDENCE-04) |
| `a3011d71` | feat | feat(phase4-n-ao): S9 supersession terminal -- every fork-choice win pairs (DC-EVIDENCE-04) |
| `c0bae25e` | feat | feat(phase4-n-ao): S9 part 2 -- observe-only fork-choice evidence taps (DC-EVIDENCE-04) |
| `d28d665f` | feat | feat(phase4-n-ao): S9 part 1 -- closed fork-choice evidence vocabulary (DC-EVIDENCE-04, latent) |
| `a77cace4` | docs | docs(phase4-n-ao): S9 slice doc + DC-EVIDENCE-04 declared (closed fork-choice convergence evidence) |
| `6846d252` | fix | fix(phase4-n-ao): block_received per-block peer attribution -- the evidence artifact that masked working multi-peer SELECT |
| `901650b2` | docs | docs(phase4-n-ao): S8 retry -- channel fairness was the wrong layer; live blocker is a 2-pump concurrency stall |
| `4c64e779` | feat | feat(phase4-n-ao): S8 multi-peer wire-pump fairness -- DC-PUMP-04 (per-peer lanes + fair merge) |
| `fc3db0f5` | docs | docs(phase4-n-ao): S8 slice doc + DC-PUMP-04 declared (multi-peer wire-pump fairness) |
| `cabe61ff` | docs | docs(phase4-n-ao): record S7 live retry -- LCA walk wired, blocked on wire-pump multi-peer fairness |
| `3b03b967` | feat | feat(phase4-n-ao): S7 live LCA fork-anchor walk -- DC-NODE-38 (multi-block branch) |
| `0cce1668` | docs | docs(phase4-n-ao): S7 slice doc + DC-NODE-38 declared (live LCA fork-anchor walk) |
| `c841f0b5` | docs | docs(phase4-n-ao): record the CE-AO-6 live SELECT gap (multi-block competing branch) |
| `3e0a6ad6` | feat | feat(phase4-n-ao): S6 live BlockFetch fetch + relay integration + evidence (CE-AO-6 hermetic) |
| `9a85ab93` | feat | feat(phase4-n-ao): S6 -- PendingForkSwitch carries winner_tip (the BlockFetch endpoint) |
| `08b2aebc` | feat | feat(phase4-n-ao): S6 byte-only bridge core -- PrefetchedBranchBodies + boundary proofs (CE-AO-6) |
| `1f16ff7f` | docs | docs(phase4-n-ao): S6 slice doc -- live BlockFetch bridge + two-producer operator pass (CE-AO-6) |
| `5b31bf7f` | feat | feat(phase4-n-ao): S5 reselection replay-equivalence + fence resolution (CE-AO-5) |
| `2490ef07` | docs | docs(phase4-n-ao): S5 slice doc -- reselection replay-equivalence + fence resolution (CE-AO-5) |
| `d63b5dac` | feat | feat(phase4-n-ao): S4 fork-switch apply -- prove, then commit (DC-NODE-37) |
| `5e4807e2` | ci | ci(gates): repair stale fork-choice gate patterns (PHASE4-N-AO S1/S3 drift) |
| `cabb94b8` | docs | docs(phase4-n-ao): S4 slice doc -- fork-switch apply (prove, then commit) (DC-NODE-37) |
| `cd11c256` | docs | docs(phase4-n-ao): S3 -- correct k source-of-authority wording |
| `a8c12327` | feat | feat(phase4-n-ao): S3 live selector dispatch -- decide-only (DC-NODE-36) |
| `986d8339` | feat | feat(phase4-n-ao): S3 selector-state projection foundation (DC-NODE-36, GREEN half) |
| `04f11013` | docs | docs(phase4-n-ao): S3 doc -- Option A selector-state + conservative-floor hard rule |
| `1939165e` | docs | docs(phase4-n-ao): S3 slice doc -- live selector dispatch (DC-NODE-36) |
| `6bcfc9e5` | feat | feat(phase4-n-ao): S2 -- BLUE-safe candidate construction (DC-NODE-35) |
| `01c94db1` | docs | docs(phase4-n-ao): S2 slice doc -- BLUE-safe candidate construction (DC-NODE-35) |

No merge commits in the span. **42 commits, zero unclassified** — every subject carries an explicit conventional-commits
prefix: **`feat`×19**, **`docs`×20**, **`test`×1** (`66312da0` the S12 fault-injection harness), **`fix`×1**
(`6846d252` the peer-attribution evidence artifact), **`ci`×1** (`5e4807e2` the stale fork-choice gate repair). The
substantive production code lands in the 19 `feat(...)` commits (the SELECT decide/prove/fetch/evidence bands) plus the
one `fix(...)`; the `ci(...)` commit is gate-only and the `test(...)` commit is the delegated fault-injection harness.

> **Note (commit-attribution policy).** Per this repo's `CLAUDE.md` override (vibe-coded-node bounty trailer
> requirement), commits in this repo carry a `Co-Authored-By:` model-attribution trailer; that is an Ade-local override
> of the global no-AI-attribution rule and applies to **commit messages only**. It does not affect this doc's content.

## 2. New Modules

**Six new modules this window — all in `ade_node`** (`git diff --diff-filter=A --name-only 31efec44..HEAD --
'crates/**/*.rs'` lists them plus two new test files and the `post_switch_continuity` bin). There is **no new crate, no
new workspace** (`git diff --diff-filter=A '**/Cargo.toml'` is empty; still **11 crates**), and **no BLUE module** (the
BLUE tree is untouched — the SELECT reuses `select_best_chain` + `validate_and_apply_header`).

| Module | Color | Purpose | Key sub-paths | Added in |
|--------|-------|---------|---------------|----------|
| `ade_node::candidate_aggregator` | **GREEN** | BLUE-safe candidate construction: a PURE projection that validates each competing-branch header through the BLUE `validate_and_apply_header` authority and assembles a `CandidateFragment` for `select_best_chain` — no minting, no store reads, no durable mutation. | `crates/ade_node/src/candidate_aggregator.rs` (+409): `CandidateFragment`, `build_candidate_fragment`, the no-manufacture boundary. | PHASE4-N-AO **S2** (`DC-NODE-35`) |
| `ade_node::selector_state` | **GREEN** | Selector-state projection for live fork-choice dispatch: derives the `TiebreakerView` / `ChainSelectorState` **from Ade's own already-admitted durable tip bytes** (local durable authority, never the peer tip) and carries the provisional decision toward S4. | `crates/ade_node/src/selector_state.rs` (+167): `project_tiebreaker`, `ChainSelectorState` projection. | PHASE4-N-AO **S3** (`DC-NODE-36`) |
| `ade_node::fork_switch` | **GREEN** | Fork-switch prove core: a `PendingForkSwitch` is authority to *attempt proof*, not to roll back. PURE `prevalidate_branch` proves the fetched bodies bind to the S3-selected headers, link from the durable anchor, and ledger-validate; the durable commit is RED (`apply_fork_switch`, in `node_lifecycle`). | `crates/ade_node/src/fork_switch.rs` (+555): `prevalidate_branch` (pure), `BranchProofError`, `ForkSwitchOutcome`. | PHASE4-N-AO **S4** (`DC-NODE-37`) |
| `ade_node::lca_walk` | **GREEN** | Last-common-ancestor fork-anchor walk: walks a multi-block competing branch's preserved parent links back to a DURABLE `ChainDb`-stored LCA under a block-depth `k`-bound. Read-only ChainDb lookups; the per-peer cache is NOT authority. | `crates/ade_node/src/lca_walk.rs` (+411): `walk_to_durable_lca`, per-peer branch cache, multi-header candidate. | PHASE4-N-AO **S7** (`DC-NODE-38`) |
| `ade_node::fair_merge` | **RED** | Multi-peer wire-pump fairness: each peer gets its OWN bounded lane, drained by a deterministic round-robin merge over the configured `--peer` order — **scheduling discipline ONLY, never fork-choice** (`select_best_chain` stays arrival-order-independent). No `HashMap`/wall-clock/`rand`. | `crates/ade_node/src/fair_merge.rs` (+244): per-peer lanes, `fair_merge` (rotating cursor, closed-lane retire-in-place). | PHASE4-N-AO **S8** (`DC-PUMP-04`) |
| `ade_node::post_switch_continuity` | **GREEN** | Replayable post-switch branch-continuity verdict: a pure, total, deterministic reducer over the closed convergence-evidence transcript classifying Ade's OWN admitted-block lineage after a `ForkChoiceWin` into a closed `PostSwitchContinuity` verdict (closed sum; reads only Ade's own lineage; the peer tip is never an input). | `crates/ade_node/src/post_switch_continuity.rs` (+676) + `crates/ade_node/src/bin/post_switch_continuity.rs` (+66): `PostSwitchContinuity`, `ContinuesSelectedBranch`, `AgreedAtSwitchTip`. | PHASE4-N-AO **S10** (`DC-EVIDENCE-05`) |

Two new **test files** were also added (not library modules): `crates/ade_node/tests/reselection_replay_s5.rs` (+306,
S5 reselection replay-equivalence) and the S6/S7/S9 taps extended the existing
`crates/ade_node/tests/live_fork_choice_ai_s4bii.rs`.

> **Cross-reference (CODEMAP) — 6 modules NOT yet registered; CODEMAP is stale.** None of `candidate_aggregator`,
> `selector_state`, `fork_switch`, `lca_walk`, `fair_merge`, or `post_switch_continuity` appears in
> `docs/ade-CODEMAP.md` (`grep -c` for each module name in CODEMAP = **0**) — CODEMAP is pinned to `b8860b16` (the prior
> window, before PHASE4-N-AO). **This is a real staleness flag, not a discipline gap:** this is the first PHASE4-N-AO
> regen, and CODEMAP/SEAMS/TRACEABILITY have not yet been regenerated for the cluster (the registry holds the new module
> bindings authoritatively). **Action:** run `/codemap` (and `/seams`, `/traceability`) to `862cd2cb` so all 6 new
> modules land in CODEMAP §GREEN/§RED with their authority tables, before relying on CODEMAP for the SELECT path.

## 3. Modules Modified

Beyond the six new modules (§2), the production work modified **`ade_node::node_lifecycle`** (the bulk — the RED SELECT
orchestration), the **`ade_node::admission_log`** event/writer (the closed evidence vocabulary), the
**`ade_node::convergence_evidence`** emitter, and a small RED touch in **`ade_runtime::forward_sync::pump`**. The
remaining span churn is the cluster/slice docs and the registry/CI edits.

| Module | Color / scope | Key changes |
|--------|---------------|-------------|
| `ade_node::node_lifecycle` (`node_lifecycle.rs` **+1394 / −~30**) | **RED** `--mode node` orchestration, additive | **The RED SELECT driver.** New fns: `dispatch_competing_fork_choice` + `decide_fork_switch` + the `ForkSwitchDecision` enum (S3 decide), `prove_fork_switch` + `apply_fork_switch` + `map_branch_proof_failure` (S4 prove-then-commit), `prefetch_branch_bodies` (S6 live BlockFetch fill), `recover_missing_range` (S14 range re-fetch), and the S11 `MissingBridge` fail-closed floor. The competing-fork arm of `run_participant_sync` is wired to call the unchanged BLUE `select_best_chain`; the relay loop carries the S9 observe-only emit taps (DECIDE + APPLY halves) keyed by `fork_switch_id`. S8 rewired the per-peer pump to `fair_merge` (the shared fan-in removed). All RED — no BLUE authority is moved here. |
| `ade_node::admission_log` (`event.rs` **+196**, `writer.rs` **+236**, `mod.rs` +3) | **GREEN** closed evidence vocabulary, additive | **S9 closed fork-choice evidence vocabulary.** 10 new closed `AdmissionLogEvent` variants (`NeedsForkChoice`, `LcaDiscovered`, `CandidateFragmentBuilt`, `ForkChoiceSelected`, `BranchFetchStarted`, `BranchFetchCompleted`, `BranchPrevalidated`, `ForkSwitchApplied`, `ForkSwitchFailed`, `ForkSwitchSuperseded`) + their JSONL discriminators + writer serialization + the `DISCRIMINATORS` allow-list extension + the closed `ForkChoiceResult` / `ForkChoiceEvidenceFailure` enums. Latent at part-1; emitted by the part-2 taps. |
| `ade_node::convergence_evidence` (`convergence_evidence.rs` **+416**) | **GREEN** evidence sink, additive | **S8.5 peer-attribution fix + S9 emitters.** `emit_block_received` now takes the per-block `(peer, slot, hash)` and threads the per-block `NodeSyncItem::Block.peer` (the fixed-`peer_label` artifact removed — it had mislabelled every block to the first peer); plus the S9 fork-choice emit helpers + the `fork_switch_id` blake2b helper. |
| `ade_node::node_sync` (`node_sync.rs` +41) | **GREEN/RED** classifier, additive | `NodeSyncItem` peer threading + the competing-branch classification feeding the S3 dispatch. |
| `ade_node::admission::runner` (`runner.rs` +12) | **RED**, additive | Runner wiring for the per-peer lane fan-in (S8). |
| `ade_node::lib` (`lib.rs` +6) | — module wiring | `pub mod` declarations for the 6 new modules. |
| `ade_runtime::forward_sync::pump` (`pump.rs` **+22 / −2**) | **RED** durable-admit chokepoint, additive | Threads the competing-branch / range-recovery hooks into the pump path (the S14 missing-bridge re-fetch admit loop). The BLUE chokepoint reducer it feeds is unchanged. |
| `ade_node::tests::live_fork_choice_ai_s4bii` (+~830 across S3/S4/S6/S7) · `node_spine_serve_loopback` (+60) | **test**, additive | The S3 decide / S4 prove / S6 bridge / S7 walk hermetic tests + the loopback BlockFetch fill test. |

> **The BLUE tree is UNTOUCHED this span (load-bearing).** `git diff 31efec44..HEAD` over the configured BLUE
> `core_paths` trees is **empty** — `select_best_chain` (`ade_core::consensus::fork_choice`, `CN-CONS-03`/`DC-CONS-03`)
> and `validate_and_apply_header` (the BLUE header authority) are REUSED byte-for-byte. The SELECT is composed ON TOP of
> them in GREEN (pure projections / reducers) and RED (orchestration / wire). BLUE canonical types **462 → 462** (no
> `^+\s*(pub )?(struct\|enum)` line in the BLUE-tree diff because there is no BLUE-tree diff). This is why the headline
> flip (`CN-CONS-03`) is enforced **by exercising the existing BLUE authority live**, not by changing it.

## 4. Feature Flags

**No project feature-flag deltas, and no manifest change at all this span.** Ade declares no `[features]` table in any
workspace `Cargo.toml` at either ref (`git grep '^\[features\]'` is empty at both `31efec44` and HEAD). **No
`#[cfg(feature = …)]` gate was introduced** (`git diff 31efec44..HEAD -- 'crates/**/*.rs' | grep -c '^+.*cfg(feature'`
= **0**), **no `compile_error!` coupling was added** (grep = **0**), and **no `Cargo.toml` changed** (`git diff
--name-only 31efec44..HEAD -- '**/Cargo.toml' 'Cargo.toml'` is empty — not even a dev-dependency this window, unlike the
prior N-AM window). **No new CLI flag** — `crates/ade_node/src/cli.rs` is untouched; the SELECT reuses the existing
`--peer` / `--participant-venue` / `--convergence-evidence-path` flags. The new `post_switch_continuity` **bin** is an
offline transcript checker (the `ci_check_post_switch_convergence_window.sh` driver), not a runtime feature gate.

## 5. CI Checks (162 → 173; +11 new, 6 modified, 0 removed)

Seventeen CI scripts changed this span: **11 added**, **6 modified in place**, **0 removed** (`git diff --diff-filter=D`
over `ci/` is empty; `ls ci/ci_check_*.sh | wc -l` = **162 → 173**). The 11 new gates back the SELECT cluster's rules
(7 new rules + the `CE-AO-5` / `CE-AO-6` evidence checkers); the 6 modified gates are the in-span stale fork-choice
gate repair (`5e4807e2`), the S5 reselection extension (`5b31bf7f`), and the close-time vocabulary repoint
(`862cd2cb`).

### PHASE4-N-AO SELECT enforcement (new gates)

| Check | Status | What it checks |
|-------|--------|----------------|
| `ci_check_candidate_construction_validated.sh` | **New** (`DC-NODE-35`, S2) | The candidate aggregator is BLUE-safe + PURE: fragments come ONLY from `validate_and_apply_header` output (no minting), it performs no store reads / durable mutation, and uses no nondeterminism. |
| `ci_check_live_selector_dispatch.sh` | **New** (`DC-NODE-36`, S3) | The live `NeedsForkChoice` dispatch (`run_participant_sync`'s competing arm, via the RED `dispatch_competing_fork_choice` + `decide_fork_switch`) routes a competing block into the unchanged BLUE `select_best_chain` and emits a provisional `PendingForkSwitch` — decide-only (no rollback at decide time). |
| `ci_check_fork_switch_never_abandons.sh` | **New** (`DC-NODE-37`, S4) | A `PendingForkSwitch` authorizes PROOF of the selected replacement branch, not a rollback: the proof (`prove_fork_switch`: fetch + read-only materialize + `prevalidate_branch`) runs first, and only `ForkSwitchOutcome::Adopted` commits the durable rollback + adopt. |
| `ci_check_live_blockfetch_byte_only.sh` | **New** (`CE-AO-6`, S6) | The live BlockFetch bridge transports BYTES, not truth: `PrefetchedBranchBodies` (the relay-loop fill from a live `RequestRange`) carries bytes only — no selection, no admission, no fork-choice on the fetch path. |
| `ci_check_lca_anchor_walk.sh` | **New** (`DC-NODE-38`, S7) | Live multi-block fork-anchor discovery: a live competing branch is eligible for SELECT only when Ade walks its preserved parent links back to a DURABLE STORED LCA (read-only ChainDb, `k`-bound; the per-peer cache is not authority). |
| `ci_check_wire_pump_fairness.sh` | **New** (`DC-PUMP-04`, S8) | Multi-peer wire-pump fairness: each connected peer gets its OWN bounded lane, drained by a fair round-robin merge over a DETERMINISTIC order derived from the configured `--peer` list — scheduling only, never fork-choice (no `HashMap`/wall-clock/`rand`). |
| `ci_check_fork_choice_evidence_closed.sh` | **New** (`DC-EVIDENCE-04`, S9) | The live SELECT path emits a CLOSED, observe-only convergence-evidence sequence (`needs_fork_choice` → `lca_discovered` → `candidate_fragment_built` → `fork_choice_selected` → `branch_fetch_*` → `branch_prevalidated` → `fork_switch_applied|failed|superseded`), with every `fork_choice_selected{win}` paired to exactly one terminal. |
| `ci_check_missing_bridge_fail_closed.sh` | **New** (`DC-NODE-39`, S11) | After a `ForkChoiceWin` adoption at tip X, a competing descendant whose parent chain cannot connect to the durable adopted tip / a durable stored ancestor is a structured `MissingBridge` fail-closed — no silent stall. |
| `ci_check_rollback_retention_evidence.sh` | **New** (`DC-NODE-40`, S13) | Rolled-back blocks may be retained ONLY as walk-visible EVIDENCE: the LCA walk consults the retention on a per-peer-cache MISS to traverse non-durable parent links (fixes the S7 LCA-walk over-fire). |
| `ci_check_missing_bridge_refetch.sh` | **New** (`DC-NODE-41`, S14) | The `DC-NODE-39` floor is SAFE but PASSIVE (ChainSync streams each block once): S14's `recover_missing_range` admit loop actively re-fetches the missing range so a winner-descendant whose bridge Ade missed is recoverable. |
| `ci_check_post_switch_convergence_window.sh` | **New** (`CE-AO-6` / `DC-EVIDENCE-04` refined + `DC-EVIDENCE-05`) | RELEASE/EVIDENCE-tier transcript checker (NOT a BLUE consensus rule): a thin driver over the `post_switch_continuity` replayable reducer asserting `ContinuesSelectedBranch` + a terminal `AgreedAtSwitchTip` + chained admitted descendants + 0 diverged + every win paired. **This is the gate the `CN-CONS-03` flip was proven against.** |

### Modified gates — stale fork-choice repair + reselection + vocabulary repoint

| Check | Status | What changed |
|-------|--------|--------------|
| `ci_check_peer_identity_preserved.sh` | **Modified** (`DC-NODE-34`, S1) | The S1 gate (peer identity restored through the receive feed — `NodeSyncItem` carries `peer`). Touched by the in-span stale-gate repair (`5e4807e2`) for pattern drift after the S3 dispatch landed. (NB: this gate was ADDED at S1, *before* this baseline, so it is Modified-in-range, not new.) |
| `ci_check_live_fork_choice_apply.sh` | **Modified** (`DC-NODE-25`/`DC-NODE-26`, N-AI) | Stale-pattern repair (`5e4807e2`): the apply-driver grep patterns updated for the S3/S4 production-region drift (the rollback-follow apply path is now reached via the SELECT's `apply_fork_switch`). |
| `ci_check_live_fork_choice_wiring.sh` | **Modified** (N-AI rollback-follow routing) | Stale-pattern repair (`5e4807e2`): wiring grep patterns updated for the S1/S3 drift; here-strings (`<<<`) retained (pipefail+SIGPIPE on a large stripped file). |
| `ci_check_wire_rollback_signal_preserved.sh` | **Modified** (AI-S4a) | Stale-pattern repair (`5e4807e2`): the rollback-signal-preservation patterns updated for the dispatch refactor. |
| `ci_check_wal_rollback_replay_equiv.sh` | **Modified** (`DC-NODE-27` / `CE-AO-5`) | Extended by S5 (`5b31bf7f`) for the post-switch reselection replay-equivalence path (same recovered store + same ordered branch ⇒ same post-switch state). |
| `ci_check_convergence_evidence_vocabulary_closed.sh` | **Modified — repointed** (`DC-ADMIT-04` / closed vocab) | The close (`862cd2cb`) extended the AJ-era 3-literal allow-list (`block_received`/`block_admitted`/`agreement_verdict`) with all 10 new fork-choice literals (`needs_fork_choice` … `fork_switch_superseded`) and **repointed Guard 5** from the AI-era schema gate to the writer `DISCRIMINATORS` allow-list (`crates/ade_node/src/admission_log/writer.rs`) — so no free-form / open vocabulary may slip in. |

> **Cross-reference (CODEMAP + SEAMS + TRACEABILITY) — ONE cluster stale this close; the largest refresh debt of the
> recent windows.** The 11 new rule↔enforcement bindings + the 10 strengthenings + the `CN-CONS-03` flip are recorded
> **in the registry at HEAD** (`docs/ade-invariant-registry.toml`, 372 rules). They are **NOT yet in TRACEABILITY,
> SEAMS, or CODEMAP**, all three pinned to `b8860b16` (`grep -c` for each new gate in TRACEABILITY = **0**; their CI-count
> pins read **161** vs. HEAD's **173**). **No gate is orphaned** — each of the 11 new gates binds a registry rule, and
> all 6 modified gates bind their existing rules. **Action:** regenerate CODEMAP + SEAMS + TRACEABILITY to `862cd2cb` as
> the named follow-on this close so the SELECT's 6 modules, 7 new rules with their named gates, 10 strengthenings, and
> the `CN-CONS-03` flip all appear and all three docs pin to the N-AO HEAD; until then the registry is authoritative.

## 6. Canonical Type Registry Delta

**n/a — no separate canonical-type registry is configured** (`canonical_type_registry: null`); canonical-type rules
live inline in the invariant registry under family **T**. **This window added ZERO BLUE canonical types and touched ZERO
BLUE files:** `git diff 31efec44..HEAD` over the BLUE `core_paths` trees is **empty** — the SELECT reuses
`select_best_chain` + `validate_and_apply_header` unchanged. BLUE `pub struct`/`pub enum` over the `core_paths` trees is
unchanged (CODEMAP's BLUE-tree metric **`462 → 462`**). All new types (`CandidateFragment`, `ForkSwitchDecision`,
`ForkSwitchOutcome`, `BranchProofError`, `PostSwitchContinuity`, the 10 `AdmissionLogEvent` variants, the per-peer lane
types, etc.) live in **GREEN/RED `ade_node`**, not in the BLUE core. **Zero BLUE canonical types added; zero removed.**

## 7. Normative / Invariant Rule Delta (365 → 372; +7 rules, +10 strengthenings, the `CN-CONS-03` flip, zero removals)

**Seven rule IDs were added; zero removed** (`365 → 372`; `comm -23` of the sorted `id =` lists is empty — exactly seven
additions, no removal). The status tally moves **227 → 239 enforced** and **118 → 113 declared**
(`enforced_scaffolding = 1`, `partial = 19` unchanged). The +12-enforced / −5-declared reconciles as: the 7 new rules
close `enforced` (+7 enforced — each was `declared` at its slice doc then flipped at close, net 0 declared), **and** 5
prior-`declared` rules flipped `enforced` this span (**`CN-CONS-03`** + **`DC-NODE-34..37`**; +5 enforced, −5 declared).

*(The configured `normative_docs` — the CE-79 tier-gate statement + addendum, the three contract docs, the CE-73
reclassification, and `CLAUDE.md` — were **not** changed this span: `git diff --name-only 31efec44..HEAD` over those
paths is empty. The rule-count delta is entirely the invariant-registry change.)*

**The headline flip — `CN-CONS-03` `declared → enforced`:** "After temporary partition, honest nodes must converge using
only protocol-defined observables and declared emergency procedures." Flipped at PHASE4-N-AO (CE-AO-6) on a NATURAL
two-producer multi-candidate SELECT pass (both Haskell producers live throughout, no loser-freeze, no post-fork operator
intervention — the SELECT decision is entirely Ade's). Checker PASS (`ci_check_post_switch_convergence_window.sh` over
the `post_switch_continuity` reducer): `ContinuesSelectedBranch`, terminal `AgreedAtSwitchTip{slot 391}` (exact
agreement, `our_hash == peer_hash`), 25 admitted descendants chained, 0 diverged, every win terminal. Transcript
`ao-CN-CONS-03-FLIP-natural-conv.jsonl` (sha256 `6713efe96ffd0e0fa304020c7784d6bbf11be0df0e3340e7feeab4d1429ca13f`,
2026-06-13) preserved OUTSIDE the repo (competition-secrecy). **SCOPE (honest):** proves convergence for the EXERCISED
two-producer partition-and-reconverge venue (S1–S14); NOT an unbounded multi-peer ChainSel claim; post-switch
endless-flip-flop survival is out of scope. `strengthened_in` `["PHASE4-N-B","PHASE4-N-AI"]` →
`["PHASE4-N-B","PHASE4-N-AI","PHASE4-N-AO"]`.

**New rules (`+7`, all enforced at HEAD):**

| Rule | Family / Tier · Status | Statement (summary) |
|------|------------------------|---------------------|
| `DC-NODE-38` | DC / `derived` · **enforced** · `introduced_in = "PHASE4-N-AO"` | **Live LCA fork-anchor walk.** A live competing branch (multi-block) is eligible for SELECT only when Ade walks its preserved parent links back to a DURABLE `ChainDb`-stored last-common-ancestor under a block-depth `k`-bound; the per-peer branch cache is an indexed memory of received, preserved headers — NOT authority. Read-only ChainDb lookups; no durable mutation. |
| `DC-NODE-39` | DC / `derived` · **enforced** · `introduced_in = "PHASE4-N-AO"` | **Post-ForkChoiceWin forward-follow floor.** After a `ForkChoiceWin` adoption at tip X, a competing descendant whose parent chain cannot connect to the durable adopted tip / a durable stored ancestor is a structured `MissingBridge` fail-closed — no silent stall (the floor is SAFE but PASSIVE; the active recovery is `DC-NODE-41`). |
| `DC-NODE-40` | DC / `derived` · **enforced** · `introduced_in = "PHASE4-N-AO"` | **Rolled-back branch evidence retention.** Rolled-back blocks may be retained ONLY as walk-visible EVIDENCE: the LCA walk consults the retention on a per-peer-cache MISS to traverse non-durable parent links. Fixes the S7 LCA-walk over-fire (the walk failing to reach the anchor across a rolled-back segment). |
| `DC-NODE-41` | DC / `derived` · **enforced** · `introduced_in = "PHASE4-N-AO"` | **Missing-bridge range re-fetch.** Because ChainSync streams each block once, the `DC-NODE-39` floor cannot recover a missing bridge by waiting; `recover_missing_range` is a latent range-recovery admit loop (with a closed outcome) wired live to actively re-fetch the missing range to the winner. |
| `DC-PUMP-04` | DC / `derived` · **enforced** · `introduced_in = "PHASE4-N-AO"` | **Multi-peer wire-pump fairness.** Each connected peer gets its OWN bounded lane, drained by a deterministic round-robin merge over the configured `--peer` order, so a continuously-producing peer self-backpressures on its own lane and never starves a competing peer's branch from reaching dispatch. **Scheduling discipline ONLY — never fork-choice** (`select_best_chain` stays arrival-order-independent, `CN-CONS-01`); no `HashMap`/wall-clock/`rand`; RED-only. |
| `DC-EVIDENCE-04` | DC / `derived` · **enforced** · `introduced_in = "PHASE4-N-AO"` | **Closed fork-choice convergence evidence.** The live SELECT path emits a CLOSED, observe-only convergence-evidence sequence (10 closed `AdmissionLogEvent` variants: `needs_fork_choice` → `lca_discovered` → `candidate_fragment_built` → `fork_choice_selected` → `branch_fetch_started/completed` → `branch_prevalidated` → `fork_switch_applied|failed|superseded`) to the convergence-evidence sink, GREEN vocab / RED sink / BLUE unchanged, so a *committed* transcript ASSERTS the SELECT middle. Every `fork_choice_selected{win}` pairs to EXACTLY ONE terminal of `applied | failed | superseded` (the per-tip `fork_switch_id` means a growing branch supersedes provisional wins; only the final pending reaches `applied`). |
| `DC-EVIDENCE-05` | DC / `derived` · **enforced** · `introduced_in = "PHASE4-N-AO"` | **Post-switch branch-continuity verdict.** A pure, total, deterministic reducer over the closed convergence-evidence transcript classifies Ade's OWN validated admitted-block lineage after a `ForkChoiceWin` adoption at tip X into a closed `PostSwitchContinuity` verdict; `ContinuesSelectedBranch` requires unbroken `prev_hash` lineage from X across every post-X `block_admitted`, no `diverged` after X, and every `fork_choice_selected{win}` paired to a terminal. The peer tip is NEVER an input (reads only Ade's own lineage); replay-equivalent. |

**Declared → enforced flips (`+5` enforced, −5 declared):** **`CN-CONS-03`** (the headline, above) + **`DC-NODE-34`**
(peer-identity restoration — the S1 gate, already enforced at the baseline tip), **`DC-NODE-35`** (BLUE-safe candidate
construction), **`DC-NODE-36`** (live selector dispatch decide-only), **`DC-NODE-37`** (fork-switch apply
prove-then-commit) — all four `DC-NODE-34..37` were declared in the preceding window (`a87a4eb5`) and flipped to
`enforced` as their slices closed / at the cluster close.

**Strengthenings (`strengthened_in += "PHASE4-N-AO"`) — 10:** **`CN-CONS-01`** (arrival-order-independence over
`select_best_chain` — the SELECT exercises it across competing live branches), **`CN-CONS-03`** (the flip itself),
**`DC-CONS-03`** + **`DC-CONS-20`** (the convergence / chain-selection family), and **`DC-NODE-24`** … **`DC-NODE-29`**
(the N-AI single-best-peer rollback-follow family — the SELECT's prove-then-commit + reselection re-exercise the
durable rollback authority live). No rule weakened; no rule removed.

**No rule was removed (expected: 0).** The registry delta is **7 new rules (all enforced), 5 declared→enforced flips
(incl. the `CN-CONS-03` headline), 10 strengthenings, zero removals** — consistent with append-only registry discipline.
**No anomaly.**

## Honest residual (window scope)

PHASE4-N-AO turned the prior single-best-peer rollback-FOLLOW into a genuine multi-candidate SELECT and **flipped
`CN-CONS-03`** on a NATURAL committed transcript. The honest residual:

- **The flip is scoped to the EXERCISED two-producer partition-and-reconverge venue.** `CN-CONS-03 → enforced` is proven
  for the S1–S14 two-producer venue (both Haskell producers live, no loser-freeze, the SELECT decision entirely Ade's),
  not as an unbounded multi-peer ChainSel claim. **Post-switch endless-flip-flop survival is out of scope.** A
  FREEZE-mode run (loser paused AFTER Ade's decision) is retained OUTSIDE the repo as a SELECT-independence diagnostic
  ONLY — never as flip evidence (a frozen peer also breaks the reducer's peer-observed-ahead terminal).
- **The BLUE authority is REUSED, not changed.** The SELECT is built ON TOP of `select_best_chain` + `validate_and_apply_header`;
  `git diff 31efec44..HEAD` over the BLUE trees is empty. The flip is earned by exercising the existing BLUE authority
  live, which is the stronger claim — but it means the SELECT's correctness rests on the GREEN projections feeding BLUE
  faithfully (`DC-NODE-35` candidate construction is BLUE-validated; `DC-PUMP-04` fairness is scheduling-only and cannot
  reorder the BLUE decision).
- **The evidence is the SELECT middle, not authority.** S9's 10 closed events + S10's reducer are GREEN observe-only
  evidence (RED sink, BLUE unchanged); they ASSERT the SELECT in a committed transcript but do not themselves decide
  anything. The closed-vocabulary discipline (allow-list + the repointed `DISCRIMINATORS` Guard 5) keeps the transcript
  a closed enum.
- **The S8 fairness layer was correct but was NOT the live blocker.** The live multi-peer SELECT was masked by an
  evidence-attribution artifact (`block_received` mislabelling every block to the first peer); the S8.5 fix
  (`6846d252`) threaded the per-block peer and overturned the channel-fairness / 2-pump-stall diagnoses. `DC-PUMP-04`
  stays correct + proven (a cleaner per-peer-lane arch) but is not load-bearing for the flip.
- **No `RO-LIVE` flip.** The CE-AO-6 pass flips `CN-CONS-03` (a `CN`-family Cardano-convergence rule), not a bounty/preprod
  `RO-LIVE` rule. `RO-LIVE-01` stays operator-gated / partial; no `RO-LIVE` registry status changed this span.
- **CODEMAP + SEAMS + TRACEABILITY refresh owed this close — the largest debt of the recent windows.** All three are
  pinned to `b8860b16` and miss **6 new modules**, **7 new rules**, **10 strengthenings**, the **`CN-CONS-03` flip**, and
  **11 new CI gates** (`grep -c` for each new module/rule/gate in all three = 0; CI pin 161 vs. HEAD 173). The registry
  holds all of it authoritatively at HEAD (372 rules); regenerating CODEMAP + SEAMS + TRACEABILITY to `862cd2cb` is the
  named follow-on (surfaced in §2 and §5). No orphan gate (each new gate binds a registry rule).
- **The transcript is OUTSIDE the repo.** Per competition-secrecy / no-credential-leak discipline, the natural CE-AO-6
  flip transcript (`ao-CN-CONS-03-FLIP-natural-conv.jsonl`) lives outside the repo; it is sha256-pinned
  (`6713efe9…ca13f`) in `CN-CONS-03.evidence_notes` so the committed registry binds the exact bytes the flip rests on.

## Working tree at HEAD `862cd2cb` (clean for tracked files)

**The working tree is CLEAN for tracked files at this regen** — `git status --porcelain` shows only untracked scratch
(`.mithril-scratch/`, `wire_smoke.jsonl`), neither part of this doc. The PHASE4-N-AO close (`862cd2cb` — the
`CN-CONS-03`/`DC-NODE-34..37` flips, the 7 new-rule enforcements, the repointed vocabulary gate) is committed; §1
narrates the committed span `31efec44..862cd2cb` verbatim; §0/§7 read rule status from the registry at HEAD
(`CN-CONS-03` enforced, 372 rules). **This regen performs the baseline bump** (`b8860b16 → 862cd2cb` in
`.idd-config.json` `head_deltas_baseline`, with the `_head_deltas_baseline_doc` lead prepended for PHASE4-N-AO and the
N-AM/N-AN paragraph demoted to "PRIOR baseline"), per the task's post-close step. **NB:** the *committed*
`.idd-config.json` baseline still reads `b8860b16`; this regen advances it. The remaining close-pass action is the
CODEMAP + SEAMS + TRACEABILITY refresh to `862cd2cb` (surfaced in §2 and §5).

---

## Historical — PHASE4-N-AM keep-alive client + PHASE4-N-AN rollback-materialize eta0 (`e87e8a43 → b8860b16`)

> The section below is the **previous** HEAD_DELTAS lead, preserved in condensed form. It narrated the
> `e87e8a43 → b8860b16` span (measured from the PHASE4-N-AL AL-S1 close `e87e8a43`): the **PHASE4-N-AL close commit**
> (`35a851b9`, docs/registry only — flipped `DC-NODE-33 → enforced`, regenerated all four grounding docs to
> `e87e8a43`, archived N-AL) + the **PHASE4-N-AM cluster** (`DC-PUMP-03`) + the **PHASE4-N-AN cluster** (`T-REC-06`) + a
> **stale-gate triage** (`89facbea`) + the **cluster-doc archive** (`b8860b16`). **12 commits, 32 files, +2288 / −516.**
> **This span TOUCHED BLUE but added ZERO new canonical type** (462 → 462; the new BLUE surface is the single METHOD
> `PraosChainDepState::overlay_recovered_eta0` + a `recovered_eta0` param/field, not a type). **NO new crate (11), NO
> new module, NO new CLI flag** (the only `Cargo.toml` touch was a `tokio` `test-util` **dev-dependency**). **PHASE4-N-AM
> / `DC-PUMP-03`** (enforced): the N2N keep-alive CLIENT (mini-protocol 8) in `run_admission_wire_pump` (the sole
> per-peer pump, `CN-PUMP-01`) on a ~20s cadence STRICTLY under the peer's ~97s timeout, advancing the REUSED BLUE
> `ade_network::keep_alive` machine; WIRE-ONLY (no `AdmissionPeerEvent`); fail-closed `AdmissionWirePumpError::KeepAlive`;
> RED-only. CE-AM-LIVE PASSED (152s sustain vs. the prior ~96s EOF). **PHASE4-N-AN / `T-REC-06`** (enforced,
> `tier = true`): `materialize_rolled_back_state` (the SOLE rolled-back-state authority, `CN-STORE-07`) overlays the
> recovered seed-epoch eta0 onto the replay `chain_dep` BEFORE the `block_validity` fold, so a rolled-back block
> validates its header VRF against the SAME nonce live admit used, NOT the snapshot `Nonce::ZERO` placeholder; VRF
> strength UNCHANGED (a WRONG eta0 still fails closed). CE-AN-LIVE PASSED (the CE-AI-6 reorg capture: `RollBackward`
> slot regression 371→361, re-converged `agreed` @ 383, 0 diverged, 0 `VrfCert`). **+2 CI gates** (159 → 161;
> `ci_check_keep_alive_wire_only.sh`, `ci_check_rollback_materialize_eta0.sh`; 3 modified by the triage; 0 removed).
> **Registry 359 → 361** (+2 `DC-PUMP-03` + `T-REC-06`; +1 strengthening `DC-PUMP-02 += PHASE4-N-AN`; 0 removed). **NO
> `RO-LIVE` flip; `CN-CONS-03` NOT flipped** (single-best-peer rollback-FOLLOW was the proven scope — flipped the next
> window, PHASE4-N-AO). The full §§0–7 narrative is recoverable from this doc's git history at `b8860b16`. *(Both new CI
> gates here are at HEAD — count 162 at this regen's baseline `31efec44`, having grown 161 → 162 in the intervening
> `b8860b16..31efec44` window for the S1 `ci_check_peer_identity_preserved.sh`.)*

---

## Historical — PHASE4-N-AL participant recovered-anchor rollback no-op (`b4c0983d → e87e8a43`)

> Preserved as a pointer. It narrated the `b4c0983d → e87e8a43` span (measured from the PHASE4-N-AK AK-S2 close): the
> **N-AK close commit** (`efa2a44e`, docs/registry only — flipped `DC-NODE-31`/`DC-NODE-32` `enforced_scaffolding →
> enforced`, regenerated all four grounding docs to `b4c0983d`; registry stayed 358) + a **C2-guide remediation note**
> (`c3ec7466`) + the **PHASE4-N-AL cluster** (single slice AL-S1). **4 commits, 14 files, +1792 / −825.** **This span did
> NOT touch BLUE** (462 → 462); **NO new crate (11), NO new module, NO new canonical type, NO new CI gate (159 → 159).**
> **AL-S1 / `DC-NODE-33`** (enforced): the participant MIRROR of N-AK's single-producer `DC-NODE-32` —
> `run_participant_sync`'s `RollBack` handler accepts a peer `RollBackward` binding EXACTLY (slot AND hash) to the
> persisted recovered anchor as an idempotent NO-OP, evaluated AFTER the `RollBackward(Origin)` fail-close and BEFORE the
> `DC-NODE-29` stored-block resolution. Registry **358 → 359** (+1 `DC-NODE-33`; 0 strengthenings; 0 removals). Live
> CE-AL-3-LIVE PASSED. **NO `RO-LIVE` flip.** The full §§0–7 narrative is recoverable from this doc's git history at
> `e87e8a43`.

---

## Historical — PHASE4-N-AK recovered-anchor live-follow start + rollback boundary (`b1bed361 → b4c0983d`)

> Preserved as a pointer. The **N-AJ close commit** (`bbdc3585`) + the **PHASE4-N-AK cluster** (two slices AK-S1 +
> AK-S2) — a post-N-AH/N-AI/N-AJ live recover→follow regression remediation. **7 commits, 33 files, +2647 / −544.**
> **This span TOUCHED BLUE — +2 canonical types** (the closed version-gated `RecoveredAnchorPoint` record +
> `RecoveredAnchorPointError` + its sole CBOR codec in the NEW BLUE module `crates/ade_ledger/src/recovered_anchor_point.rs`,
> plus the NEW RED module `crates/ade_runtime/src/recovered_anchor.rs`). **AK-S1 / `DC-NODE-31`** (enforced): persist the
> bootstrap anchor POINT as fingerprint-bound recovery provenance + resolve the live-follow FindIntersect start from it.
> **AK-S2 / `DC-NODE-32`** (enforced): the single-producer `run_node_sync` `RollBack` handler accepts `RollBackward(anchor)`
> (exact slot AND hash) as an idempotent no-op. Registry **356 → 358** (+2; `T-REC-05` strengthened; 0 removed); CI **159
> → 159** (both rules `ci_script=""`). **NO `RO-LIVE` flip.** The full §§0–7 narrative is recoverable from this doc's git
> history at `b4c0983d`.

---

## Historical — PHASE4-N-AJ Participant-path convergence evidence emission (`e99a86c7 → b1bed361`)

> Preserved as a pointer. The **PHASE4-N-AJ cluster** — Participant-path convergence evidence emission, the CE-AI-6
> bridge. **9 commits, 19 files, +1813 / −35.** **EVIDENCE-ONLY — ZERO BLUE change.** It added a **deterministic GREEN
> evidence side-output** — the EXISTING closed `AgreementVerdict` vocabulary (`block_received` / `block_admitted` /
> `agreement_verdict` via `verdict::derive`) to a dedicated `--convergence-evidence-path` JSONL sink (the new GREEN/RED
> module `ade_node::convergence_evidence`, now extended by PHASE4-N-AO S8.5/S9). CI **157 → 159** (+2). Registry **354 →
> 356** (+2: `DC-NODE-30` enforced + `DC-EVIDENCE-03` enforced_scaffolding; `DC-ADMIT-04` strengthened; **`CN-CONS-03`
> NOT flipped**; 0 removed). **NO `RO-LIVE` flip.** The full §§0–7 narrative is recoverable from this doc's git history
> at `b1bed361`.

---

## Historical — PHASE4-N-AI live fork-choice rollback-follow wiring (`8e2c3672 → 5ec841c8` / close `e99a86c7`)

> Preserved as a pointer. The **PHASE4-N-AI cluster** (live fork-choice rollback-follow wiring of the EXISTING
> `chain_selector` → BLUE `select_best_chain` into the live `--mode node` receive path — single-best-peer FOLLOW, NOT
> full ChainSel; `DC-NODE-23`…`DC-NODE-29`). **26 commits, 46 files, +5350 / −53.** **FIRST BLUE delta since G-N: +2
> canonical types** (`ade_ledger::wal::event::{RollbackPoint, RollbackReason}`, the payload types of the closed-sum
> `WalEntry::RollBack` durable MARKER). CI **148 → 157** (+9). Registry **347 → 354** (+7: `DC-NODE-23..29` enforced;
> `CN-CONS-01` flipped partial→enforced; 13 strengthenings; 0 removed). The per-cluster security review found **H-1**
> (mixed peer/local rollback target → durable-chain truncation) → remediated by **AI-S6 / `DC-NODE-29`** → re-review
> **H-1 CLOSED**. **`CN-CONS-03` was NOT flipped** — single-best-peer rollback-FOLLOW (the SELECT that flips it is
> PHASE4-N-AO). The `DC-NODE-24..29` family is **strengthened by PHASE4-N-AO** (the SELECT re-exercises the rollback
> authority live). **NO `RO-LIVE` flip.** The full §§0–7 narrative is recoverable from this doc's git history at
> `5ec841c8` / `e99a86c7`.

---

## Historical — PHASE4-N-AG superseded + PHASE4-N-AH local-tip forge-base authority (`f87d0056 → 5858288e`)

> Preserved as a pointer. The **PHASE4-N-AG cluster** (single-producer loop-continuation-after-feed-EOF, `DC-NODE-19`;
> **superseded-close**) + the **PHASE4-N-AH cluster** (local selected durable chain forge-base authority `DC-NODE-20` +
> cert evidence-only `DC-NODE-21` + single-producer warm-start re-entry `DC-NODE-22`). **32 commits, 48 files, +5155 /
> −743.** **RED/GREEN-only — ZERO BLUE change.** CI **143 → 148** (+5; 3 modified; 0 removed). Registry **343 → 347**
> (+4; 9 strengthenings; 0 removed). Headline: Ade sustained **cert-free single-producer block production on C2-LOCAL**
> against a real Haskell relay (`cardano-node 11.0.1`). NOT preprod. No `RO-LIVE` flip. The full §§0–7 narrative is
> recoverable from this doc's git history at `5858288e`.

---

## Historical — earlier windows (`a76672b9 → f87d0056` and before)

> Preserved as pointers. The **PHASE4-N-AF cluster** (single-producer extend-own-durable-spine, `DC-NODE-18`; 343 rules
> / 143 CI at `f87d0056`); the **PHASE4-N-AE.F slice** (`DC-NODE-16` receive idempotency at the durable-admit
> chokepoint; 341 rules / 142 CI at `6363683e`); the **PHASE4-N-AD/N-AE CE-A5 window** (recover→serve continuity +
> forge-on-followed-tip admissibility — the CE-A5 manifest: a real `cardano-node 11.0.1` relay `AddedToCurrentChain` an
> Ade-forged successor block; `DC-NODE-14`/`DC-NODE-15`/`DC-CONS-24`/`DC-PROTO-10`; 336 → 340 rules); the **PHASE4-N-AC**
> (KES key evolves to the current period — `DC-CRYPTO-10`), **PHASE4-N-AB** (outbound mux segmentation — `CN-SESS-05`),
> **PHASE4-N-AA** (bounded peer-driven serve range — `DC-SERVEMEM-01`), **PHASE4-N-U** (forged-block durability —
> `DC-NODE-12`/`DC-CONS-23`/`DC-WAL-04`/`T-REC-05`/`DC-NODE-13`), and the **G-K…G-R + C1 multi-cluster catch-up**. The
> full §§0–7 narrative for each is recoverable from this doc's git history at the respective HEADs.

---

## Generation notes

### Regen `31efec44 → 862cd2cb` (PHASE4-N-AO — live multi-candidate fork-choice SELECT + `CN-CONS-03` flip — current lead)

- **Baseline valid; one cluster, end-to-end.** Run against `31efec44` (the PHASE4-N-AO **S1** commit), which
  `git rev-parse` resolves and `git merge-base 31efec44 HEAD == 31efec44` confirms is a strict ancestor of HEAD
  `862cd2cb` (`31efec44` carries no tag). The cluster's declare (`a87a4eb5`) + S1 slice doc + S1 impl (`301a4932`,
  `31efec44`) landed at/before this baseline (in the preceding `b8860b16..31efec44` grounding-regen window), so this
  regen measures the SELECT from the S1 tip forward; `DC-NODE-34` is already enforced at the baseline and appears only as
  the Modified S1 gate. This regen **performs the baseline bump** `b8860b16 → 862cd2cb` as the task's post-close step
  (the committed `.idd-config.json` baseline still reads `b8860b16`).
- **Counts are mechanical (git/grep/ls):** commit log + `--shortstat` over `31efec44..HEAD` (**42** commits, no merges /
  **50** files / **+9797 / −98**); CI gate count via `git ls-tree -r --name-only <ref> ci/ | grep -c ci_check_.*\.sh` =
  **162** at baseline, **173** at HEAD (`--diff-filter=A` = 11 new, `--diff-filter=M` = 6, `--diff-filter=D` = empty);
  registry rule count via `grep -c '^id = '` at each ref (**365 → 372**; `comm -23` of the sorted `id =` lists is empty —
  exactly seven additions `DC-NODE-38..41` + `DC-PUMP-04` + `DC-EVIDENCE-04/05`, zero removals); registry status via
  `grep '^status = ' | sort | uniq -c` (baseline **227 / 1 / 19 / 118**, HEAD **239 / 1 / 19 / 113**); strengthenings =
  **10** (`strengthened_in` lines gaining `PHASE4-N-AO`: `CN-CONS-01`, `CN-CONS-03`, `DC-CONS-03`, `DC-CONS-20`,
  `DC-NODE-24..29`); BLUE canonical types **462 → 462** (`git diff 31efec44..HEAD` over the BLUE `core_paths` trees is
  empty).
- **BLUE tree UNTOUCHED — GREEN+RED only.** `git diff 31efec44..HEAD` over the configured BLUE `core_paths` trees is
  empty; the SELECT reuses `select_best_chain` (`ade_core::consensus::fork_choice`) + `validate_and_apply_header`
  unchanged. All production code is in `ade_node` (+4795/−37) + a 22-line RED touch in `ade_runtime::forward_sync::pump`.
- **Six new modules, all GREEN/RED in `ade_node`.** `git diff --diff-filter=A --name-only 31efec44..HEAD --
  'crates/**/*.rs'` lists `candidate_aggregator.rs`, `selector_state.rs`, `fork_switch.rs`, `lca_walk.rs` (GREEN),
  `fair_merge.rs` (RED), `post_switch_continuity.rs` (GREEN) + its `bin/`, plus two new test files
  (`reselection_replay_s5.rs`, and the existing `live_fork_choice_ai_s4bii.rs` extended). No new crate / workspace — still
  11 crates. Module colors read from the `//! GREEN`/`//! RED` banners.
- **No manifest change, no feature flag, no CLI flag.** `git diff --name-only 31efec44..HEAD -- '**/Cargo.toml'
  'Cargo.toml'` is empty (not even a dev-dependency); no `[features]` table at either ref; 0 `cfg(feature)` and 0
  `compile_error!` added; `cli.rs` untouched.
- **Registry delta is +7 rules + 5 declared→enforced flips (incl. `CN-CONS-03`) + 10 strengthenings, NOT a removal.**
  The 7 new rules (`DC-NODE-38..41`, `DC-PUMP-04`, `DC-EVIDENCE-04/05`) were declared at their slice docs and flipped to
  `enforced` at close; `CN-CONS-03` + `DC-NODE-34..37` flipped `declared → enforced`. The sorted-id `comm -23` confirms
  zero removals. The `CN-CONS-03` flip is bound in `CN-CONS-03.evidence_notes` to the sha256-pinned OUTSIDE-repo natural
  CE-AO-6 transcript.
- **+11 CI gates, 6 modified, 0 removed.** New: the 10 SELECT-rule gates + `ci_check_post_switch_convergence_window.sh`
  (the CE-AO-6 transcript checker the flip was proven against). Modified: 4 by the stale fork-choice gate repair
  (`5e4807e2` — `live_fork_choice_apply`, `live_fork_choice_wiring`, `peer_identity_preserved`,
  `wire_rollback_signal_preserved`), 1 by S5 (`5b31bf7f` — `wal_rollback_replay_equiv`), 1 by the close (`862cd2cb` —
  `convergence_evidence_vocabulary_closed`, repointed to the writer `DISCRIMINATORS` allow-list).
- **The `CN-CONS-03` flip is the headline; no `RO-LIVE` flip.** `CN-CONS-03` (Cardano post-partition convergence)
  flipped `declared → enforced` on the natural CE-AO-6 two-producer SELECT pass (checker PASS:
  `ContinuesSelectedBranch`, `AgreedAtSwitchTip{slot 391}`, 25 chained descendants, 0 diverged, every win terminal).
  `RO-LIVE-01` stays operator-gated / partial; no `RO-LIVE` status changed.
- **Normative docs unchanged this span.** `git diff --name-only 31efec44..HEAD` over the configured `normative_docs`
  (CE-79 statement + addendum, the three contract docs, CE-73 reclassification, `CLAUDE.md`) is empty — the §7 delta is
  entirely the invariant-registry change.
- **§1 commit log verbatim from `git log` (newest first).** The per-slice synthesis is in §0/§3. All 42 subjects carry a
  conventional-commits prefix (`feat`×19 / `docs`×20 / `test`×1 / `fix`×1 / `ci`×1); zero unclassified.
- **Doc-refresh state — CODEMAP + SEAMS + TRACEABILITY now ONE cluster STALE (the largest refresh debt of the recent
  windows).** All three were regenerated to `b8860b16` (the prior window, by `f167a349`) and carry `DC-PUMP-03` /
  `T-REC-06` but **nothing from PHASE4-N-AO** — they miss the 6 new modules, the 7 new rules, the 10 strengthenings, the
  `CN-CONS-03` flip, and the 11 new CI gates (`grep -c` for each in all three = 0; CI pin 161 vs. HEAD 173).
  **Cross-reference warnings surfaced in §2 and §5.** Regenerate CODEMAP + SEAMS + TRACEABILITY to `862cd2cb` as a
  follow-on this close; the registry holds all of it authoritatively in the interim (372 rules). No orphan gate (each new
  gate binds a registry rule).
- **Working tree CLEAN for tracked files.** This regen runs with all PHASE4-N-AO close artifacts committed
  (`git status --porcelain` = untracked scratch only). **This regen performs the `.idd-config.json` baseline bump**
  `b8860b16 → 862cd2cb`, per the task's post-close step. The remaining close-pass action is the CODEMAP + SEAMS +
  TRACEABILITY refresh to `862cd2cb`.
