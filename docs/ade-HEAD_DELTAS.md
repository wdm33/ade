# Ade — HEAD Deltas (Changes Since Baseline)

> **Status:** Living architectural document. Regenerated; not hand-edited.
>
> Regenerate with `/head-deltas <baseline>` after every cluster close. Baseline is recorded in `.idd-config.json` `head_deltas_baseline`.

> Baseline: `e99a86c7` (PHASE4-N-AI close — live single-best-peer rollback-follow + replay-equivalent reorg adoption, 2026-06-09 17:58)
> HEAD: `b1bed361` (PHASE4-N-AJ AJ-S3 — convergence runbook correction + DC-EVIDENCE-03 transcript-shape rule, 2026-06-10 09:35)
> Span: **the PHASE4-N-AJ cluster — Participant-path convergence evidence emission (the CE-AI-6 bridge): a GREEN/RED evidence side-output over the EXISTING N-AI rollback-follow receive path, emitting the EXISTING closed `AgreementVerdict` vocabulary to a dedicated `--convergence-evidence-path` JSONL; `DC-NODE-30` + `DC-EVIDENCE-03` + `DC-ADMIT-04` strengthened** — preceded by the N-AI baseline-bump chore and folding **one unrelated docs commit** (a C2-guide sync).
> **9 commits** (no merges), **19 files changed, +1813 / −35 lines**. **This span CHANGES NO BLUE — evidence-only**: `git diff e99a86c7..HEAD` over the BLUE `core_paths` trees (`ade_ledger` / `ade_codec` / `ade_types` / `ade_crypto` / `ade_plutus` / `ade_core` / the BLUE `ade_network` submodules) touches **zero** files and adds **zero** `^+(pub )?(struct|enum)` lines — the BLUE canonical-type count is unchanged at **460** (carried verbatim from the N-AI close; `git diff e99a86c7..HEAD -- '**/Cargo.toml'` is empty — still **11 crates**). Every source change is RED/GREEN on the `ade_node` shell: one NEW non-authority module (`crates/ade_node/src/convergence_evidence.rs`, +417) + small additive wiring in `node_lifecycle.rs` (+107/−12), `cli.rs` (+49), `node_sync.rs` (+2), `lib.rs` (+1), and two test files (+1 each).

> **Baseline note (load-bearing — read before §0).** This window's baseline is **`e99a86c7`**, the
> PHASE4-N-AI close (the prior HEAD_DELTAS HEAD), and it is **valid**: `git rev-parse e99a86c7` resolves and
> `git merge-base e99a86c7 HEAD == e99a86c7` (it is a strict ancestor of HEAD; `e99a86c7` carries no tag).
> HEAD is **`b1bed361`** (the PHASE4-N-AJ AJ-S3 close commit — runbook correction + the `DC-EVIDENCE-03`
> transcript-shape rule). The config baseline at the start of this regen was already `e99a86c7` (the previous
> close's `chore(idd)` bumped it — `c1f4c876`, the first commit in this span), so the window measures cleanly
> from the recorded baseline forward. The span has **three parts**: (1) the **N-AI baseline-bump chore** —
> `c1f4c876` (`chore(idd): bump head_deltas_baseline to the PHASE4-N-AI close (e99a86c7); registry 354`),
> config-only, zero code, zero rule; (2) **one unrelated docs commit** — `c95e2592` (`docs(c2-guide): sync
> §5/§7/§7b to HEAD`), a C2-preprod-tip-guide sync, docs-only, folded into the span but **not** part of N-AJ;
> and (3) the **PHASE4-N-AJ cluster** (`DC-NODE-30` + `DC-EVIDENCE-03`) — cluster/invariants/plan doc
> (`645c0067`, declares both new rules) + the three slices AJ-S1…AJ-S3 (each a `docs(...)` slice doc + a
> `feat(...)` impl). **There is no separate close commit** — AJ-S3 (`b1bed361`) is the last slice and carries
> the final rule flips inline.
>
> **Working-tree note (load-bearing).** At the time of this regen there are **UNCOMMITTED working-tree changes**
> — the N-AJ close artifacts (registry status flips, slice-doc `Merged` flips, a cluster-doc fix, and a further
> c2-guide update). **§1 narrates the COMMITTED span `e99a86c7..b1bed361` verbatim from `git log`.** The rule
> **STATUS** in §0/§7 is read from the **CURRENT working-tree** `docs/ade-invariant-registry.toml` so the prose
> reflects the close state (`DC-NODE-30` **enforced**, `DC-EVIDENCE-03` **enforced_scaffolding**, `DC-ADMIT-04`
> **strengthened** `+= PHASE4-N-AJ`, `CN-CONS-03` **still `declared`** — NOT flipped). The operator bumps
> `head_deltas_baseline` `e99a86c7 → b1bed361` as a separate post-close step so the next cluster measures from
> here.

This window is **led by PHASE4-N-AJ — Participant-path convergence evidence emission (the CE-AI-6 bridge).** It
takes the EXISTING enforced N-AI rollback-follow receive path (the live `--mode node --participant-venue` loop:
`classify_receive` venue-split detector `DC-NODE-23/24`, `apply_chain_event` rollback authority `DC-NODE-25/26`,
`pump_block` roll-forward admit `DC-NODE-05/12`, the durable `WalEntry::RollBack` marker `DC-NODE-27`) and adds a
**deterministic GREEN evidence side-output** — it emits the EXISTING closed `AgreementVerdict` vocabulary to a
dedicated `--convergence-evidence-path` JSONL so that a committed operator transcript can witness Ade + ≥1
Haskell producer converging on the same tip through a real reorg (CE-AI-6). The headline must be read with its
**boundary intact**:

> **PHASE4-N-AJ added a convergence-evidence emit path to the live Participant rollback-follow loop: for each
> peer block considered it emits `block_received`, per `pump_block` admit `block_admitted`, and per outcome
> `agreement_verdict = verdict::derive(outcome, observed_peer_tip)` — the EXISTING closed `AdmissionLogEvent`
> 3-variant subset, no new evidence enum — to a dedicated sink that has NO raw-writer accessor. This is
> EVIDENCE, NOT authority: the emit never gates admission, never triggers/parameterizes a rollback, never
> influences fork-choice, never mutates the durable chain; `pump_block` stays the sole roll-forward admit,
> `apply_chain_event` the sole rollback authority, `classify_receive` unchanged. A sink write failure is
> SURFACED (`EvidenceEmitResult::FailedAndPoisoned`) and marks the transcript incomplete — it is never swallowed
> as success and never halts authority. There is NO BLUE change (460 canonical types unchanged) and NO
> `RO-LIVE` flip. `CN-CONS-03` was NOT flipped (it stays `declared`) — `DC-EVIDENCE-03` is the
> `enforced_scaffolding` transcript-shape rule, vacuous-until-committed.**

The arc is three small slices — the inert sink lands and is proven FIRST, then the emission is wired into the
live loop, then the runbook is corrected and the transcript-shape rule is pinned:

- **PHASE4-N-AJ / AJ-S1 / `DC-ADMIT-04` (strengthened) (dedicated convergence-evidence sink — GREEN selection
  over RED file I/O, INERT).** A new non-authority module `crates/ade_node/src/convergence_evidence.rs` ships
  the `ConvergenceEvidenceSink` — the **only** way to write the convergence transcript: it wraps an optional
  `AdmissionLogWriter<Box<dyn Write>>` and exposes exactly **three** emit methods (`emit_block_received` /
  `emit_block_admitted` / `emit_agreement_verdict`) and **NO accessor to the raw inner writer**, so the file
  cannot become a dumping ground for sched / forge / admission-lifecycle events. The closed convergence
  vocabulary is the **3-variant subset of the REUSED `AdmissionLogEvent`** (`block_received` / `block_admitted`
  / `agreement_verdict`) — **no new evidence enum** (¬AJ-3). TCB split: the event *selection* (which variant +
  the `verdict::derive` mapping) is **GREEN** and generic over `W: Write`; the file-backed instantiation
  (`File::create` + the byte writes) is **RED**. The `--convergence-evidence-path` CLI flag is added but **not
  yet read by the live loop** — the slice is **INERT** (no live behavior flip). New gate
  `ci_check_convergence_evidence_vocabulary_closed.sh` (the file-tree half of the subset closure); `DC-ADMIT-04`
  gains `strengthened_in += PHASE4-N-AJ` (the third physically-isolated closed-vocabulary transcript). *(Without
  the no-accessor wrapper, the convergence file could silently widen past the schema-gate's allow-list.)*
- **PHASE4-N-AJ / AJ-S2 / `DC-NODE-30` (enforced) (participant-path convergence evidence emission — the flip,
  RED+GREEN).** The emission is wired into `node_lifecycle.rs` `run_participant_sync`: the loop now threads a
  `ConvergenceEvidence` (sink + the consensus-inputs-fingerprint binding + the followed-peer label) and emits,
  as a GREEN side-output of already-authoritative outcomes — **`block_received` for EACH peer block considered
  by the receive path** (before drop/admit/refuse), **`block_admitted` per `pump_block` admit**, and
  **`agreement_verdict = verdict::derive(outcome, observed_peer_tip)`** — each carrying
  `consensus_inputs_fingerprint_hex`. The emit is **emit-only on `Diverged`** (a divergence is recorded, never
  acted on). A write failure surfaces as `EvidenceEmitResult::{Written, Disabled, FailedAndPoisoned}` — **never
  swallowed, never halts authority** (the node continues consensus operation; the sink is distinct from the
  authoritative WAL), but the transcript is then marked incomplete/unusable for CE-AI-6 (it MUST NOT silently
  produce a partial transcript that later passes the schema gate). No path supplied ⇒ no file and node behavior
  byte-unchanged. New gate `ci_check_convergence_evidence_emit_only.sh` (the verdict/emit result must never feed
  `classify_receive` / `apply_chain_event` / `pump_block` / fork-choice / forge, and a write error must never
  halt authority). `DC-NODE-30 → enforced` at close. **Honesty:** this emits convergence EVIDENCE over the
  EXISTING single-best-peer rollback-follow path — it does NOT add a second selection authority and does NOT
  broaden the N-AI venue scope.
- **PHASE4-N-AJ / AJ-S3 / `DC-EVIDENCE-03` (enforced_scaffolding) (runbook correction + transcript-shape rule —
  docs + registry).** The CE-AI-6 runbook (`docs/active/phase4-n-ai-convergence-runbook.md`) is corrected to the
  real flag name `--convergence-evidence-path` (it previously named a stale flag). `DC-EVIDENCE-03` pins the
  **convergence-through-reorg transcript shape**: the participant convergence pass produces ONE JSONL transcript
  with **AT LEAST** (a) a strict slot regression in the OBSERVED PEER BLOCK sequence (a peer `RollBackward` was
  actually followed) **and** (b) ≥ 1 `AgreementVerdict { kind: "agreed" }` at the re-converged tip, and **AT
  MOST** 0 `AgreementVerdict { kind: "diverged" }`; the `.md` manifest binds the `.jsonl` sha256. A boring
  same-tip-only run (no regression) is **NOT sufficient.** It is tied to the **existing** schema gate
  `ci_check_convergence_evidence_schema.sh` (which shipped at N-AI AI-S5) — **no new gate this slice.**
  **Vacuous-until-committed** — `DC-EVIDENCE-03` is `enforced_scaffolding`: the gate runs and is closed, but the
  operator-produced transcript (`docs/evidence/phase4-n-ai-convergence-pass.{jsonl,md}`) is not yet committed,
  so the rule is scaffolding-enforced, not satisfied. **`CN-CONS-03` is NOT flipped** — it stays `declared`
  (single-best-peer scope; CE-AI-6 operator-gated). **SINGLE-BEST-PEER scope — NOT full multi-peer Cardano
  ChainSel.**

**No BLUE canonical type added or removed** (460 unchanged — the first window since the G-N span that does NOT
touch BLUE, on the heels of the N-AI +2). **No `RO-LIVE` rule flipped** — `RO-LIVE-01` stays operator-gated;
CE-AI-6 (`DC-EVIDENCE-03`) is the convergence-through-reorg transcript, vacuous-until-committed.

## 0. Headline

| Count | Baseline (`e99a86c7`) | HEAD (`b1bed361` + close working-tree) | Δ |
|---|---|---|---|
| CI gates (`ci/ci_check_*.sh`) | 157 | **159** | **+2** — **two NEW gates** (`--diff-filter=A` over `ci/`): `ci_check_convergence_evidence_vocabulary_closed.sh` (AJ-S1, `DC-ADMIT-04` strengthening / `DC-NODE-30` sink half) and `ci_check_convergence_evidence_emit_only.sh` (AJ-S2, `DC-NODE-30`). The schema gate `ci_check_convergence_evidence_schema.sh` is **NOT new** — it shipped at N-AI AI-S5 and is reused by `DC-EVIDENCE-03`. **Zero gates modified in place; zero removed** (`--diff-filter=M` / `--diff-filter=D` over `ci/` empty). |
| Registry rules (`docs/ade-invariant-registry.toml`) | 354 | **356** | **+2** — two NEW rules `DC-NODE-30` + `DC-EVIDENCE-03`. **Zero removed** (`diff` of the sorted `id =` lists shows exactly the two additions and no removal). |
| Registry status (enforced / enforced_scaffolding / partial / declared) | 221 / 0 / 19 / 114 | **222 / 1 / 19 / 114** | **+1 enforced** (`DC-NODE-30`), **+1 enforced_scaffolding** (`DC-EVIDENCE-03`, a NEW status value introduced this close — vacuous-until-committed). Partial and declared **unchanged**. |
| Registry strengthenings | — | **1** | **`strengthened_in += "PHASE4-N-AJ"`** on **1** existing rule: `DC-ADMIT-04` (the closed `AdmissionLogEvent` vocabulary now extends to the THIRD isolated closed-vocabulary file — the convergence transcript). **`CN-CONS-03` was NOT strengthened or flipped this window** — it stays `declared` (its `strengthened_in` carries `PHASE4-N-B`, `PHASE4-N-AI` from prior windows, not `PHASE4-N-AJ`). No rule weakened. |
| BLUE canonical types | 460 | **460** | **±0** — **no BLUE change.** `git diff e99a86c7..HEAD` over the BLUE `core_paths` trees touches no file and adds zero `^+(pub )?(struct\|enum)` lines. No `Cargo.toml` changed — still 11 crates. |
| Grounding docs | CODEMAP / SEAMS / TRACEABILITY all regenerated to **`5ec841c8`** (the N-AI close — 460 types / 157 CI / 354 rules) | All three still pinned at **`5ec841c8`** — they do **NOT** yet carry `DC-NODE-30`, `DC-EVIDENCE-03`, the `convergence_evidence` module, or the two new gates (grep = 0). CODEMAP's 5 `convergence_evidence` hits are the N-AI AI-S5 **schema-gate** references, not the AJ module. | **CODEMAP + SEAMS + TRACEABILITY are one cluster STALE** — the registry holds the two new rules + their gate bindings authoritatively at HEAD (**356 rules**); the refresh to `b1bed361` is a follow-on item this close. See the cross-reference warnings at the end of §2 and §5. |

> **Grounding-doc state this close (load-bearing).** **CODEMAP, SEAMS, and TRACEABILITY all remain pinned at
> `5ec841c8`** (the N-AI close) and are **one cluster stale** — none carries `DC-NODE-30`, `DC-EVIDENCE-03`, the
> new `ade_node::convergence_evidence` module, or the two new gates
> (`ci_check_convergence_evidence_{vocabulary_closed,emit_only}.sh`); `grep -c` of each in all three is 0 (the
> only `convergence_evidence` hits in CODEMAP are the N-AI AI-S5 `ci_check_convergence_evidence_schema.sh`
> references). The invariant registry holds the two new rules + both new gate bindings + the `DC-ADMIT-04`
> strengthening authoritatively at HEAD (**356 rules**); the CODEMAP + SEAMS + TRACEABILITY refresh to
> `b1bed361` is a follow-on item this close (surfaced in §2 and §5).

The slice↔rule↔gate map for this window:

| Slice | Rule(s) | Gate | What shipped |
|---|---|---|---|
| **cluster doc** (`645c0067`) | `DC-NODE-30` + `DC-EVIDENCE-03` **declared** | — | N-AJ cluster authority doc + invariants + cluster-slice plan; declares both new rules. |
| **AJ-S1** (`69b081c7`) | strengthen **`DC-ADMIT-04`** | **`ci_check_convergence_evidence_vocabulary_closed.sh`** (NEW) | **GREEN selection / RED file I/O — INERT.** New module `ade_node::convergence_evidence` — `ConvergenceEvidenceSink` over `Box<dyn Write>` with exactly 3 emit methods + NO raw-writer accessor; closed 3-variant subset of the REUSED `AdmissionLogEvent` (no new enum). `--convergence-evidence-path` flag added but not yet read (inert). `EvidenceEmitResult { Written \| Disabled \| FailedAndPoisoned }`. |
| **AJ-S2** (`e577bd3b`) | **`DC-NODE-30`** (NEW, enforced) | **`ci_check_convergence_evidence_emit_only.sh`** (NEW) | **RED+GREEN — the flip.** Emission wired into `node_lifecycle.rs` `run_participant_sync`: `block_received` per considered peer block / `block_admitted` per `pump_block` admit / `agreement_verdict` via `verdict::derive`; each carries `consensus_inputs_fingerprint_hex`. Write failure surfaced (`FailedAndPoisoned`), never swallowed, never halts authority. Emit-only on `Diverged`. |
| **AJ-S3** (`b1bed361`) | **`DC-EVIDENCE-03`** (NEW, enforced_scaffolding) | (reuses **`ci_check_convergence_evidence_schema.sh`** — existed at AI-S5) | **docs + registry.** Runbook corrected to `--convergence-evidence-path`; `DC-EVIDENCE-03` pins the convergence-through-reorg transcript shape (≥1 strict observed-peer slot regression + ≥1 `agreed` at re-converged tip; 0 `diverged`; `.md` binds `.jsonl` sha256). Vacuous-until-committed. **Last slice — no separate close commit.** |

The per-commit shape (the full verbatim log is §1):

| Commit | Kind | What it did | Code / CI / registry effect |
|--------|------|-------------|-----------------------------|
| `c1f4c876` | chore (idd) | Bump `head_deltas_baseline` to the PHASE4-N-AI close (`e99a86c7`); registry 354 | **0 code / 0 CI / 0 rule**; `.idd-config.json` only |
| `c95e2592` | docs (c2-guide) | Sync §5/§7/§7b to HEAD — CE-AH-6 + PHASE4-N-AI + ADE1 reg | **0 code / 0 CI / 0 rule** — **UNRELATED to N-AJ**, folded into the span (`docs/active/c2-preprod-tip-guide.md` only) |
| `645c0067` | docs (phase4-n-aj) | Cluster authority doc + invariants/plan; declare `DC-NODE-30` + `DC-EVIDENCE-03` | **0 code / 0 CI**; registry: `DC-NODE-30` + `DC-EVIDENCE-03` declared |
| `21ca9ffd` | docs (AJ-S1) | Slice doc AJ-S1 — dedicated convergence-evidence sink (inert) | **0 code / 0 CI / 0 rule** |
| `69b081c7` | feat (AJ-S1) | Dedicated convergence-evidence sink (inert) | **RED+GREEN code** (NEW `convergence_evidence.rs` + `cli.rs` + `node_lifecycle.rs` + `lib.rs` + `wire_only_loopback.rs` test); **+0 BLUE type**; **+1 CI** (`ci_check_convergence_evidence_vocabulary_closed.sh`); registry: `DC-ADMIT-04` strengthened |
| `33377359` | docs (AJ-S2) | Slice doc AJ-S2 — participant-path convergence evidence emission (the flip) | **0 code / 0 CI / 0 rule** |
| `e577bd3b` | feat (AJ-S2) | Participant-path convergence evidence emission (the flip) | **RED+GREEN code** (`convergence_evidence.rs` + `node_lifecycle.rs` + `node_sync.rs` + `live_fork_choice_ai_s4bii.rs` test); **+1 CI** (`ci_check_convergence_evidence_emit_only.sh`); registry: `DC-NODE-30 → enforced` |
| `9a698b1e` | docs (AJ-S3) | Slice doc AJ-S3 — runbook correction + `DC-EVIDENCE-03` transcript-shape rule | **0 code / 0 CI / 0 rule** |
| `b1bed361` | feat (AJ-S3) | Convergence runbook correction + `DC-EVIDENCE-03` transcript-shape rule | **0 code / 0 CI** (reuses the AI-S5 schema gate); `docs/active/phase4-n-ai-convergence-runbook.md` + registry: `DC-EVIDENCE-03 → enforced_scaffolding` |

## 1. Commit Log (newest first)

| Hash | Type | Summary |
|------|------|---------|
| `b1bed361` | feat | feat(phase4-n-aj): AJ-S3 -- convergence runbook correction + DC-EVIDENCE-03 transcript-shape rule |
| `9a698b1e` | docs | docs(phase4-n-aj): slice doc AJ-S3 -- runbook correction + DC-EVIDENCE-03 transcript-shape rule |
| `e577bd3b` | feat | feat(phase4-n-aj): AJ-S2 -- participant-path convergence evidence emission (the flip) |
| `33377359` | docs | docs(phase4-n-aj): slice doc AJ-S2 -- participant-path convergence evidence emission (the flip) |
| `69b081c7` | feat | feat(phase4-n-aj): AJ-S1 -- dedicated convergence-evidence sink (inert) |
| `21ca9ffd` | docs | docs(phase4-n-aj): slice doc AJ-S1 -- dedicated convergence-evidence sink (inert) |
| `645c0067` | docs | docs(phase4-n-aj): cluster authority doc + invariants/plan; declare DC-NODE-30 + DC-EVIDENCE-03 |
| `c95e2592` | docs | docs(c2-guide): sync §5/§7/§7b to HEAD — CE-AH-6 + PHASE4-N-AI + ADE1 reg |
| `c1f4c876` | chore | chore(idd): bump head_deltas_baseline to the PHASE4-N-AI close (e99a86c7); registry 354 |

No merge commits in the span. **9 commits, zero unclassified.** Every subject carries an explicit
conventional-commits prefix (`chore(...)` / `docs(...)` / `feat(...)`). The `feat(...)` commits (`69b081c7`,
`e577bd3b`, `b1bed361`) are the production RED/GREEN changes (the last carries only docs + registry — the AI-S5
schema gate is reused); the rest are `docs(...)` slice/cluster docs and the `chore(...)` baseline bump.
**`c95e2592`** (`docs(c2-guide): sync …`) is **unrelated to PHASE4-N-AJ** — a C2-preprod-tip-guide sync, folded
into the span but not cluster work. All commits landed 2026-06-09 / 2026-06-10.

> **Note (commit-attribution policy).** Per this repo's `CLAUDE.md` override (vibe-coded-node bounty
> trailer requirement), commits in this repo carry a `Co-Authored-By:` model-attribution trailer; that
> is an Ade-local override of the global no-AI-attribution rule and applies to **commit messages
> only**. It does not affect this doc's content.

## 2. New Modules

**One new GREEN/RED module — `ade_node::convergence_evidence`.** `git diff --diff-filter=A --name-only
e99a86c7..HEAD -- 'crates/**/*.rs'` lists exactly **one** new `.rs` library module
(`crates/ade_node/src/convergence_evidence.rs`, +417, registered in `crates/ade_node/src/lib.rs` as `pub mod
convergence_evidence;`). There is **no new crate, no new `Cargo.toml`, no new workspace** (`git diff
--name-only … '**/Cargo.toml'` is empty; still **11 crates**); the two new integration-test files are not
library modules.

| Module | Color | Purpose | Key sub-paths | Added in (cluster/slice) |
|--------|-------|---------|---------------|--------------------------|
| `ade_node::convergence_evidence` | **GREEN** selection over **RED** file I/O | The single non-authority sink for the CE-AI-6 convergence transcript: emits the closed 3-variant `AdmissionLogEvent` subset (`block_received` / `block_admitted` / `agreement_verdict`) to `--convergence-evidence-path`. Event *selection* is GREEN (generic over `W: Write`); the file-backed instantiation (`File::create` + byte writes) is RED. Evidence, **never** authority. | `ConvergenceEvidenceSink` (3 emit methods, **no** raw-writer accessor; `open` / `with_writer` / `disabled`), `ConvergenceEvidence` (sink + fingerprint binding + peer label; AJ-S2 wiring), `EvidenceEmitResult { Written \| Disabled \| FailedAndPoisoned }` | **PHASE4-N-AJ AJ-S1** (`69b081c7`) — emission wired AJ-S2 (`e577bd3b`) |

> **Cross-reference (CODEMAP) — STALE: new module NOT yet in CODEMAP.** The new GREEN/RED module
> `ade_node::convergence_evidence` and its types (`ConvergenceEvidenceSink`, `ConvergenceEvidence`,
> `EvidenceEmitResult`) are **NOT** in CODEMAP §GREEN/RED — CODEMAP is pinned at `5ec841c8` (the N-AI close) and
> predates N-AJ (`grep -c ConvergenceEvidenceSink` in CODEMAP = 0; the 5 `convergence_evidence` hits are the
> N-AI AI-S5 `ci_check_convergence_evidence_schema.sh` references, not this module). **Action:** regenerate
> CODEMAP to `b1bed361` so the new module appears in the GREEN/RED authority table; until then the registry +
> this doc are authoritative for the module. **This is a refresh-on-this-close item, not a discipline gap** —
> the module ships fully tested and gated; the registry (`DC-NODE-30`, `DC-ADMIT-04`) holds its bindings at HEAD.

## 3. Modules Modified

Five source files across **one crate** changed — **all `ade_node` (RED/GREEN shell)**; **no BLUE file touched,
+0 canonical types**:

| Module | Color / scope | Key changes |
|--------|---------------|-------------|
| `ade_node::node_lifecycle` (`node_lifecycle.rs` +107/−12) | **RED** loop + apply driver, additive | **AJ-S1 (`69b081c7`):** imports `ConvergenceEvidence` / `ConvergenceEvidenceSink`; opens the sink from `cli.convergence_evidence_path` and constructs `ConvergenceEvidence::new(sink, &fp, peer_label)` in `run_participant_sync` (inert — wired but the emit calls land in AJ-S2). **AJ-S2 (`e577bd3b`) — DC-NODE-30 (the flip):** `run_participant_sync` (+ the receive-step helper it calls) now threads an `Option<&mut ConvergenceEvidence>` and emits `block_received` for EACH considered peer block (`ev.emit_block_received(cand_slot, &cand_hash)`), `block_admitted` per `pump_block` admit, and `agreement_verdict` via `verdict::derive(outcome, observed_peer_tip)` — each carrying `consensus_inputs_fingerprint_hex`. The emit is a **GREEN side-output** of already-authoritative outcomes — it never gates `classify_receive` / `apply_chain_event` / `pump_block` / fork-choice / forge; a `FailedAndPoisoned` write error is surfaced but **never halts** the loop. **No new BLUE type.** |
| `ade_node::cli` (`cli.rs` +49) | **RED**, additive | **AJ-S1 (`69b081c7`):** new `--convergence-evidence-path → convergence_evidence_path: Option<PathBuf>` flag — `None` ⇒ no file is opened or written and node behavior is byte-unchanged; `Some(p)` ⇒ the file-backed sink. Parsed into `Cli` alongside the N-AI `--participant-venue`; **not** a Cargo feature or compile-time `cfg`. **No new BLUE type.** |
| `ade_node::node_sync` (`node_sync.rs` +2) | **GREEN**, trivial | **AJ-S2 (`e577bd3b`):** a two-line additive plumb supporting the emission wiring (the considered-block summary the loop emits as `block_received`). `classify_receive` / `resolve_disposition` decision logic **unchanged** — the detector still never references a chain selector (`DC-CONS-03` honored). **No new BLUE type.** |
| `ade_node::lib` (`lib.rs` +1) | RED/GREEN crate root | **AJ-S1 (`69b081c7`):** `pub mod convergence_evidence;` — registers the new module (§2). |
| `ade_node` tests (`tests/wire_only_loopback.rs` +1, `tests/live_fork_choice_ai_s4bii.rs` +1) | test-only | **AJ-S1 / AJ-S2:** one-line additive touches (the new `--convergence-evidence-path` / emission surfaces exercised through the existing live-loop integration tests). Test-only — no production or BLUE effect. |

> **No BLUE change this span (load-bearing).** Unlike the immediately-prior N-AI window (which added +2 BLUE
> types), this span touches **no** BLUE file: `git diff e99a86c7..HEAD` over the BLUE `core_paths` trees
> (`ade_ledger` / `ade_codec` / `ade_types` / `ade_crypto` / `ade_plutus` / `ade_core` / the BLUE `ade_network`
> submodules) lists **zero** files and adds **zero** `^+(pub )?(struct\|enum)` lines — the BLUE canonical-type
> count stays **460** (carried verbatim from the N-AI close). The whole window is `ade_node` shell work
> (GREEN selection + RED file I/O), reusing the EXISTING `AdmissionLogEvent` / `AdmissionLogWriter` vocabulary,
> the EXISTING `verdict::derive`, and the EXISTING N-AI rollback-follow receive path unchanged.

## 4. Feature Flags

**No project feature-flag deltas.** Ade declares no `[features]` table in any workspace `Cargo.toml`, and **no
`Cargo.toml` changed in this window** (`git diff --name-only e99a86c7..HEAD -- '**/Cargo.toml' 'Cargo.toml'`
is empty). No `#[cfg(feature = …)]` gate was introduced and no `compile_error!` coupling was added (`git diff
e99a86c7..HEAD` grep for both is empty). The one CLI-flag delta this span is an **addition**:
`--convergence-evidence-path` (AJ-S1 — an `Option<PathBuf>` selecting the convergence-evidence sink). It is a
CLI flag parsed into `Cli`, **not** a Cargo feature flag, env var, or compile-time `cfg`. **Coupling note:** the
flag is **opt-in and inert by absence** — `None` ⇒ no file is opened or written and node behavior is
byte-unchanged (`DC-NODE-30`); it has no mutual-exclusion or required-with relationship to any other flag (it is
orthogonal to the N-AI `--participant-venue` / `--single-producer-venue` pair, though the convergence transcript
is only emitted on the Participant path). The gates `ci_check_convergence_evidence_vocabulary_closed.sh` (the
sink may construct only the 3-variant subset, no raw-writer accessor) and `ci_check_convergence_evidence_emit_only.sh`
(the emit never feeds authority, a write error never halts) fence the flag's behavior.

## 5. CI Checks (157 → 159; +2 new gates, 0 modified in place, 0 removed)

Two new gates this span; **zero modified in place; zero removed**. `git diff --diff-filter=A e99a86c7..HEAD
-- ci/` lists exactly the two gates below; `--diff-filter=M` and `--diff-filter=D` over `ci/` are **empty**.
The grouping mirrors the AJ-S1 / AJ-S2 slice progression. (AJ-S3's `DC-EVIDENCE-03` reuses the **existing**
`ci_check_convergence_evidence_schema.sh`, which shipped at N-AI AI-S5 — it is **not** new this window.)

### PHASE4-N-AJ gates — convergence-evidence sink + emit-only (AJ-S1 / AJ-S2)

| Check | Status | Origin / change | What it checks |
|-------|--------|-----------------|----------------|
| `ci_check_convergence_evidence_vocabulary_closed.sh` | **New** | AJ-S1 (`69b081c7`); `DC-ADMIT-04` strengthening / `DC-NODE-30` sink half | The `ConvergenceEvidenceSink` module constructs **ONLY** the closed 3-variant subset of the REUSED `AdmissionLogEvent` (`BlockReceived` / `BlockAdmitted` / `AgreementVerdict` ⇒ `block_received` / `block_admitted` / `agreement_verdict`); it exposes **no raw-writer accessor** (which would bypass the subset), carries **no** sched/forge/wire-only literals, and reuses only vocabulary the schema gate already allows — the file-tree half of the subset closure. |
| `ci_check_convergence_evidence_emit_only.sh` | **New** | AJ-S2 (`e577bd3b`); `DC-NODE-30` | The convergence-evidence module is pure GREEN emission — it touches **no** authority surface (it calls **none** of `classify_receive` / `resolve_disposition` / `apply_chain_event` / `pump_block` / `select_best_chain` / `fork_choice` / `commit_rollback`); the verdict / emit result **never** feeds the participant routing; a sink write error is **non-fatal** to authority (surfaced via the incomplete/poisoned flag, the node continues consensus operation). Evidence observes authority; evidence never becomes authority. |

> **Cross-reference (CODEMAP + SEAMS + TRACEABILITY) — STALE this close; refresh owed.** The new rule↔gate
> bindings (`DC-NODE-30 ↔ ci_check_convergence_evidence_{vocabulary_closed,emit_only,schema}.sh`; `DC-EVIDENCE-03
> ↔ ci_check_convergence_evidence_schema.sh`; `DC-ADMIT-04 ↔ ci_check_convergence_evidence_vocabulary_closed.sh`)
> are recorded **in the registry at HEAD** (`docs/ade-invariant-registry.toml`, 356 rules). They are **NOT yet
> in TRACEABILITY, SEAMS, or CODEMAP**, all three of which remain pinned at the N-AI close `5ec841c8` (`grep -c`
> of `DC-NODE-30` / `DC-EVIDENCE-03` / `ci_check_convergence_evidence_emit_only` in each = 0; SEAMS + CODEMAP
> headers still read "460 canonical types / 157 CI checks at HEAD `5ec841c8`"). **Neither new gate is an
> orphan** — each enforces exactly its named rule, recorded in the registry. **Action:** regenerate CODEMAP +
> SEAMS + TRACEABILITY to `b1bed361` as a follow-on this close so the new module appears in CODEMAP §GREEN/RED
> and every §5 gate appears in TRACEABILITY enforcing its named invariant; until then the registry is
> authoritative for the new bindings.

## 6. Canonical Type Registry Delta

**n/a — no separate canonical-type registry is configured** (`canonical_type_registry: null`);
canonical-type rules live inline in the invariant registry under family **T**. **This window added NO BLUE
canonical type:** the BLUE count is **`460 → 460`** (`git diff e99a86c7..HEAD` over the BLUE `core_paths` trees
adds zero `^+(pub )?(struct|enum)` lines and touches no BLUE file). **Zero BLUE canonical types were removed.**
The new GREEN/RED types this window (`ConvergenceEvidenceSink`, `ConvergenceEvidence`, `EvidenceEmitResult`)
live in the `ade_node` shell (`crates/ade_node/src/convergence_evidence.rs`), outside the BLUE `core_paths`, and
the convergence transcript reuses the **existing** closed `AdmissionLogEvent` vocabulary — **no new evidence
enum** (¬AJ-3). No `Cargo.toml` changed.

## 7. Normative / Invariant Rule Delta (354 → 356; +2 rules, 1 strengthening, zero removals)

**Two rule IDs were added; zero removed** (`354 → 356`; `diff` of the sorted `id =` lists shows exactly the two
additions `DC-NODE-30` + `DC-EVIDENCE-03` and no removal). The status tally moves **221 → 222 enforced**
(`DC-NODE-30`) and introduces a **new status value `enforced_scaffolding` (0 → 1)** for `DC-EVIDENCE-03`
(vacuous-until-committed); the 19 partial and 114 declared are **unchanged**.

*(The configured `normative_docs` — the CE-79 tier-gate statement + addendum, the three contract docs, the
CE-73 reclassification, and `CLAUDE.md` — were **not** changed this span: `git diff --name-only
e99a86c7..HEAD` over those paths is empty. The rule-count delta is entirely the invariant-registry change.)*

**New rules (`+2`, both `introduced_in = "PHASE4-N-AJ"`):**

| Rule | Family / Tier · Status | Statement (summary) |
|------|------------------------|---------------------|
| `DC-NODE-30` | DC / `derived` · **enforced** | **Participant-path convergence evidence emission.** The live `--mode node --participant-venue` rollback-follow path emits the EXISTING closed `AgreementVerdict` vocabulary to a dedicated `--convergence-evidence-path` JSONL as a deterministic GREEN side-output of already-authoritative outcomes: `BlockReceived` for EACH peer block considered (before drop/admit/refuse), `BlockAdmitted` per `pump_block` admit, `AgreementVerdict = verdict::derive(outcome, observed_peer_tip)` — each carrying `consensus_inputs_fingerprint_hex`. **MUST NOT become authority:** never gates admission, never triggers/parameterizes a rollback, never influences fork-choice, never mutates the durable chain; `pump_block` stays the sole roll-forward admit, `apply_chain_event` the sole rollback authority, `classify_receive` unchanged. Emit-only on `Diverged`. A write failure is **non-fatal** to authority (the node continues; the sink is distinct from the authoritative WAL) but the transcript is then marked incomplete/unusable for CE-AI-6 — it MUST NOT silently produce a partial transcript that later passes the gate. No path ⇒ no file and node behavior byte-unchanged. **No new evidence enum; no BLUE change.** |
| `DC-EVIDENCE-03` | DC / `derived` · **enforced_scaffolding** (vacuous-until-committed) | **Convergence-through-reorg transcript shape (CE-AI-6).** The participant convergence pass produces ONE JSONL transcript with **AT LEAST** (a) a strict slot regression in the OBSERVED PEER BLOCK sequence (a peer `RollBackward` was actually followed) **and** (b) ≥ 1 `AgreementVerdict { kind: "agreed" }` at the re-converged tip; and **AT MOST** 0 `AgreementVerdict { kind: "diverged" }`. The `.md` manifest binds the `.jsonl` sha256. A boring same-tip-only run (no regression) is **NOT sufficient.** Validated by the existing `ci/ci_check_convergence_evidence_schema.sh` (shipped at N-AI AI-S5). **Vacuous-until-committed** — the operator-produced transcript (`docs/evidence/phase4-n-ai-convergence-pass.{jsonl,md}`) is not yet committed. **SINGLE-BEST-PEER scope — NOT full multi-peer Cardano ChainSel.** |

**Strengthenings (`strengthened_in += "PHASE4-N-AJ"`) — 1:** `DC-ADMIT-04` (the closed `AdmissionLogEvent`
vocabulary is now physically isolated across a **THIRD** file — the convergence-evidence transcript, which via
the `ConvergenceEvidenceSink` wrapper carries ONLY the closed 3-variant convergence subset `{block_received,
block_admitted, agreement_verdict}`, has no inner-writer accessor, and emits none of the excluded
admission-lifecycle / sched / forge / wire-only literals; `ci_check_convergence_evidence_vocabulary_closed.sh`
is the file-tree half). **No rule was weakened.**

> **`CN-CONS-03` was NOT flipped or strengthened this window (load-bearing).** `CN-CONS-03` ("after temporary
> partition, honest nodes must converge using only protocol-defined observables and declared emergency
> procedures") stays **`declared`** — its `strengthened_in` carries `PHASE4-N-B` and `PHASE4-N-AI` from prior
> windows, **not** `PHASE4-N-AJ`. N-AJ adds the convergence-EVIDENCE emit path + the transcript-shape rule
> (`DC-EVIDENCE-03`, the operator-pass bridge), but the broad multi-peer-ChainSel convergence claim that
> `CN-CONS-03` makes is **not** satisfied by single-best-peer rollback-follow evidence — CE-AI-6 is
> operator-gated and the transcript is vacuous-until-committed. The boundary from N-AI holds verbatim: **only the
> single-best-peer venue is exercised; full multi-peer Cardano ChainSel remains out of scope.**

**No rule was removed (expected: 0).** The registry delta is **two new rules (`DC-NODE-30` enforced +
`DC-EVIDENCE-03` enforced_scaffolding), one `strengthened_in += PHASE4-N-AJ` append (`DC-ADMIT-04`), zero
removals** — consistent with append-only registry discipline. **No anomaly.**

## Honest residual (window scope)

PHASE4-N-AJ **wired a convergence-EVIDENCE emit path** into the EXISTING N-AI single-best-peer rollback-follow
loop and made the transcript shape a closed, schema-gated rule. The honest residual:

- **The headline boundary (verbatim).** Ade now **emits the EXISTING closed `AgreementVerdict` vocabulary**
  (`block_received` / `block_admitted` / `agreement_verdict`, via `verdict::derive`) to a dedicated
  `--convergence-evidence-path` sink on the live Participant rollback-follow path. **This is EVIDENCE, NOT
  authority** — it never gates admission, never triggers/parameterizes a rollback, never influences fork-choice,
  never mutates the durable chain.
- **`DC-EVIDENCE-03` is `enforced_scaffolding` — vacuous-until-committed.** The CE-AI-6 transcript-shape rule is
  closed and its schema gate (`ci_check_convergence_evidence_schema.sh`, shipped at N-AI AI-S5) runs, but the
  operator-produced transcript (`docs/evidence/phase4-n-ai-convergence-pass.{jsonl,md}`) is **not yet
  committed** — so the rule is scaffolding-enforced, not satisfied. Producing + committing the convergence-pass
  transcript (a strict observed-peer slot regression + ≥1 `agreed` at the re-converged tip + 0 `diverged`,
  `.md`-binds-`.jsonl`-sha256) is the named follow-on.
- **`CN-CONS-03` was NOT flipped (load-bearing).** It stays `declared` — its broad multi-peer-ChainSel
  convergence claim is not satisfied by single-best-peer rollback-follow evidence; CE-AI-6 is operator-gated and
  vacuous-until-committed. Promoting it requires the committed operator convergence transcript **plus** a later
  multi-peer candidate-set slice.
- **No BLUE change — evidence-only on the `ade_node` shell.** 460 canonical types unchanged (the first window
  since G-N that does not touch BLUE, on the heels of the N-AI +2). The new `ade_node::convergence_evidence`
  module is GREEN selection over RED file I/O; it reuses the EXISTING `AdmissionLogEvent` / `AdmissionLogWriter`
  vocabulary (no new evidence enum) and the EXISTING `verdict::derive`. `pump_block` stays the sole roll-forward
  admit, `apply_chain_event` the sole rollback authority, `classify_receive` unchanged.
- **Write-failure is surfaced, never swallowed, never halts authority.** A sink write error surfaces as
  `EvidenceEmitResult::FailedAndPoisoned` and marks the transcript incomplete/unusable for CE-AI-6 — the node
  continues consensus operation (the sink is distinct from the authoritative WAL), but the transcript MUST NOT
  silently pass the schema gate as partial.
- **No `RO-LIVE` flip.** `RO-LIVE-01` stays operator-gated / partial. No `RO-LIVE` registry status changed this
  span.
- **CODEMAP + SEAMS + TRACEABILITY refresh owed this close.** All three remain pinned at the N-AI close
  `5ec841c8` and do not yet carry `DC-NODE-30`, `DC-EVIDENCE-03`, the new `ade_node::convergence_evidence`
  module, or the two new gates. The registry holds the two new rules + both gate bindings + the `DC-ADMIT-04`
  strengthening authoritatively at HEAD (356 rules) in the interim. Regenerating CODEMAP + SEAMS + TRACEABILITY
  to `b1bed361` is the named follow-on (surfaced in §2 and §5).
- **One unrelated commit folded into the span.** `c95e2592` (`docs(c2-guide): sync §5/§7/§7b to HEAD`,
  `docs/active/c2-preprod-tip-guide.md`) is docs-only and **not** PHASE4-N-AJ work; it sits inside the
  `e99a86c7..HEAD` range and is recorded in §1 for completeness.

## Working tree at HEAD `b1bed361` (close in progress)

**There are UNCOMMITTED working-tree changes at this regen** — the N-AJ close artifacts: registry status flips,
slice-doc `Merged` flips, a cluster-doc fix, and a further c2-guide update. §1 narrates the **committed** span
`e99a86c7..b1bed361` verbatim; §0/§7 read rule **status** from the **current working-tree** registry (so the
prose reflects `DC-NODE-30` enforced / `DC-EVIDENCE-03` enforced_scaffolding / `DC-ADMIT-04` strengthened /
`CN-CONS-03` still declared). The remaining close-pass actions are (1) committing the close artifacts, (2) the
CODEMAP + SEAMS + TRACEABILITY refresh to `b1bed361` (surfaced in §2/§5), and (3) the baseline bump (`e99a86c7 →
b1bed361`) — all separate post-close steps; **this regen does not touch `.idd-config.json` `head_deltas_baseline`.**

> **Cluster-context note.** PHASE4-N-AJ closes with AJ-S3 (`b1bed361`) as the last slice — there is **no
> separate close commit**; the final rule flips (`DC-NODE-30 → enforced`, `DC-EVIDENCE-03 →
> enforced_scaffolding`, `DC-ADMIT-04` strengthened) are carried inline by the slice commits + the in-progress
> working-tree close. Whether the cluster docs are moved to `docs/clusters/completed/PHASE4-N-AJ/` is a
> close-pass bookkeeping decision separate from this HEAD_DELTAS regen.

---

## Historical — PHASE4-N-AI live fork-choice rollback-follow wiring (`8e2c3672 → 5ec841c8` / close `e99a86c7`)

> The section below is the **previous** HEAD_DELTAS lead, preserved in condensed form. It narrated the
> `8e2c3672 → 5ec841c8` span: the **N-AH baseline-bump chore** (`c66fa9a9`) + the **PHASE4-N-AI cluster** (live
> fork-choice rollback-follow wiring of the EXISTING `chain_selector` → BLUE `select_best_chain` into the live
> `--mode node` receive path — single-best-peer FOLLOW, NOT full ChainSel; `DC-NODE-23`…`DC-NODE-29`; close
> `5ec841c8`, docs/baseline `e99a86c7`) + one unrelated docs commit (`cbad2ae3`, a preprod ADE1 pool
> registration manifest). **26 commits, 46 files, +5350 / −53.** **FIRST BLUE delta since G-N: +2 canonical
> types** (`458 → 460` — the `ade_ledger::wal::event::{RollbackPoint, RollbackReason}` payload types of the new
> closed-sum `WalEntry::RollBack` durable MARKER at the reserved RollBackward tag 1; `select_best_chain`
> production byte-identical, the only `fork_choice` change a `#[cfg(test)]` permutation proof). CI gates **148 →
> 157** (+9 new: `wal_rollback_replay_equiv`, `receive_detector_venue_split`, `live_fork_choice_apply`,
> `live_fork_choice_wiring`, `wire_rollback_signal_preserved`, `participant_venue_inert`,
> `chain_selection_arrival_order_independent`, `convergence_evidence_schema`, `rollback_target_canonical_binding`;
> 0 modified, 0 removed). Registry **347 → 354** (+7: `DC-NODE-23..29`, all enforced; `CN-CONS-01` flipped
> partial→enforced; 13 strengthenings, all `PHASE4-N-AI`; 0 removed; status 213 → 221 enforced, 20 → 19 partial,
> 114 declared unchanged). Headline (honest boundary): Ade follows ONE peer's chain-sync `RollBackward` reorg
> end-to-end on a declared Participant venue — durable-point lookup → materialize + lockstep + a version-gated
> `WalEntry::RollBack` durable marker + `pump_block` roll-forward, replay-equivalently and fail-closed.
> **Single-best-peer rollback-FOLLOW, NOT full multi-peer Cardano ChainSel.** **`CN-CONS-03` was NOT flipped**
> (it stayed `declared`, strengthened only). The per-cluster security review found **H-1** (the live RollBack
> path could build a target from mixed peer/local authority — a malicious peer could truncate the durable chain
> and brick the node) → remediated by **AI-S6 / `DC-NODE-29`** (bind the target to the durable stored chain
> point, validated **pre-mutation**, fail-closed) → re-review **H-1 CLOSED, no new findings.** `pump_block` stays
> the SOLE roll-forward admit; SingleProducer stays fail-closed unchanged; **NO `RO-LIVE` flip** (CE-AI-6
> operator-gated/vacuous-until-committed). All four grounding docs were regenerated at the N-AI close to
> `5ec841c8` (460 types / 157 CI / 354 rules). The full §§0–7 narrative is recoverable from this doc's git
> history at `5ec841c8` / `e99a86c7`.

---

## Historical — PHASE4-N-AG superseded + PHASE4-N-AH local-tip forge-base authority (`f87d0056 → 5858288e`)

> Preserved as a pointer. It narrated the `f87d0056 → 5858288e` span: the **PHASE4-N-AF close tail** (`600581e8`
> + `2d99cdf2`, docs/archive) + the **PHASE4-N-AG cluster** (single-producer loop-continuation-after-feed-EOF,
> `DC-NODE-19`; **superseded-close** — hermetic core CE-AG-1..4 complete, live CE re-homed to N-AH) + the
> **PHASE4-N-AH cluster** (local selected durable chain forge-base authority `DC-NODE-20` + cert evidence-only
> `DC-NODE-21` + single-producer warm-start re-entry `DC-NODE-22`). **32 commits, 48 files, +5155 / −743.**
> **RED/GREEN-only — ZERO BLUE change, 458 → 458 canonical types.** CI gates **143 → 148** (+5; 3 modified in
> place; 0 removed). Registry **343 → 347** (+4: `DC-NODE-19` declared + `DC-NODE-20/21/22` enforced; 9
> strengthenings, all `PHASE4-N-AH`; 0 removed). Headline (honest boundary): Ade sustained **cert-free
> single-producer block production on C2-LOCAL** (`cardano-testnet` magic 42) against a real Haskell relay
> (`cardano-node 11.0.1`) — forged on its OWN local durable `ChainDb::tip`, crossed a follow-link EOF, settled
> `> k` immutable, and resumed forging after a hard restart (run-4). NOT preprod. NOT bounty completion. No
> `RO-LIVE` flip. The N-AF operator-adoption certificate had leaked from evidence into forge-loop authority;
> N-AH retired it (deleted the `VenueAdoptionCertificate` type + cert parser + `--adoption-cert-path` flag). All
> four grounding docs were regenerated at the N-AH close (paying the deferred N-AF + N-AG CODEMAP debt). The full
> §§0–7 narrative is recoverable from this doc's git history at `5858288e`.

---

## Historical — PHASE4-N-AF single-producer extend-own-durable-spine (`6363683e → f87d0056`)

> Preserved as a pointer. A **single-slice cluster lead** narrating the `6363683e → f87d0056` span: the
> PHASE4-N-AE.F close grounding-doc refresh (`d3f52e7c`) + a C2-guide doc (`1302417d`) followed by the **OQ-1 /
> DC-NODE-17 investigation** (`bd1a7a73` declared DC-NODE-17 → `dadf4743` live-disproved it as the fix) and the
> **PHASE4-N-AF cluster** (single slice AF.S1 — `DC-NODE-18`, single-producer extend-own-durable-spine). Counts
> at `f87d0056`: 343 rules, 143 CI gates, 458 canonical types. **GREEN+RED only — BLUE 458 → 458.** New gate
> `ci_check_single_producer_extend_own_spine.sh`. `DC-NODE-17` declared then live-disproved (the relay does NOT
> re-announce Ade's own block) and retained safety/observation-only; the actual fix was `DC-NODE-18`. No
> `RO-LIVE` flip. CODEMAP/SEAMS/TRACEABILITY refresh was deferred at the N-AF close and paid at the N-AH close.
> The full §§0–7 narrative is recoverable from this doc's git history at `f87d0056`.

---

## Historical — PHASE4-N-AE.F post-CE-A5 echo-idempotency follow-up (`a76672b9 → 6363683e`)

> Preserved as a pointer. A **single-slice lead** narrating the `a76672b9 → 6363683e` span: the PHASE4-N-AE
> close grounding-doc refresh (`62811a4e`) followed by the **PHASE4-N-AE.F** slice (`DC-NODE-16` receive
> idempotency at the durable-admit chokepoint — a re-announced block Ade already durably holds (same hash, same
> slot) is an idempotent no-op at `pump_block`, so a continuous recover→follow run survives the post-adoption
> echo instead of exiting 43). Counts at `6363683e`: 341 rules, 142 CI gates, 458 canonical types. **RED
> chokepoint only — BLUE 458 → 458.** New gate `ci_check_receive_idempotency.sh`. No `RO-LIVE` flip. The full
> §§0–7 narrative is recoverable from this doc's git history at `6363683e`.

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
> 134 CI gates, the one BLUE canonical type `ArrayHead` 457 → 458). The full §§0–7 narrative for each is
> recoverable from this doc's git history at the respective HEADs.

---

## Generation notes

### Regen `e99a86c7 → b1bed361` (PHASE4-N-AJ Participant-path convergence evidence emission — current lead)

- **Baseline valid; one single-theme cluster + an unrelated docs commit, preceded by the N-AI baseline-bump
  chore.** Run against `e99a86c7` (the PHASE4-N-AI close, the prior HEAD_DELTAS HEAD), which `git rev-parse`
  resolves and `git merge-base e99a86c7 HEAD` confirms is a strict ancestor of HEAD `b1bed361` (`e99a86c7`
  carries no tag). The start-of-regen config baseline was already `e99a86c7`. The operator bumps
  `head_deltas_baseline` `e99a86c7 → b1bed361` as a **separate post-close step** (NOT performed by this regen).
- **Counts are mechanical (git/grep/ls):** commit log + `--shortstat` over `e99a86c7..HEAD` (**9** commits, no
  merges / **19** files / **+1813 / −35**); CI gate count via `git ls-tree -r --name-only <ref> ci/ | grep -c
  ci_check` at each ref (**157 → 159**; `--diff-filter=A` over `ci/ci_check_*.sh` = the two new gates;
  `--diff-filter=M` and `--diff-filter=D` over `ci/` **empty**); registry rule count via `grep -cE
  '^\[\[rules\]\]'` at each ref (**354 → 356**; `comm`/`diff` of the sorted `id =` lists shows exactly the two
  additions `DC-NODE-30` + `DC-EVIDENCE-03`, zero removals); registry status via `grep -cE '^status =
  "<value>"$'` at each ref (**221 → 222 enforced**, **0 → 1 enforced_scaffolding**, 19 partial + 114 declared
  unchanged); strengthening = **1** (`strengthened_in += "PHASE4-N-AJ"` appears once, on `DC-ADMIT-04`); BLUE
  canonical types unchanged at **460** (the `git diff e99a86c7..HEAD` over the BLUE `core_paths` trees is empty).
- **STATUS read from the CURRENT working tree (load-bearing).** There are **uncommitted** close artifacts at this
  regen (registry status flips, slice-doc `Merged` flips, a cluster-doc fix, a further c2-guide update). §1
  narrates the **committed** span `e99a86c7..b1bed361` verbatim; §0/§7 read rule **status** from the **current
  working-tree** `docs/ade-invariant-registry.toml` so the prose reflects the close state (`DC-NODE-30`
  enforced, `DC-EVIDENCE-03` enforced_scaffolding, `DC-ADMIT-04` strengthened, `CN-CONS-03` still declared). The
  registry-count deltas above were verified against the current working-tree registry (356 rules); the
  baseline-side counts via `git show e99a86c7:docs/ade-invariant-registry.toml` (354 rules).
- **No BLUE change — evidence-only on the `ade_node` shell.** `git diff e99a86c7..HEAD` over the BLUE
  `core_paths` trees (`ade_ledger` / `ade_codec` / `ade_types` / `ade_crypto` / `ade_plutus` / `ade_core` / the
  BLUE `ade_network` submodules) touches **no** file and adds **zero** `^+(pub )?(struct|enum)` lines (BLUE
  count **460 → 460**, carried verbatim from N-AI). `git diff --name-only … '**/Cargo.toml' 'Cargo.toml'` is
  empty (no feature-flag delta; the one CLI-flag delta is an **addition**, `--convergence-evidence-path`,
  opt-in/inert-by-absence).
- **One new module — the new `.rs` is a library module, not a test.** `git diff --diff-filter=A --name-only …
  'crates/**/*.rs'` lists exactly one new `.rs`: `crates/ade_node/src/convergence_evidence.rs` (registered in
  `lib.rs`). The two other touched test files (`tests/wire_only_loopback.rs`, `tests/live_fork_choice_ai_s4bii.rs`)
  are one-line additive, not new files. No new crate / `Cargo.toml` / workspace — still 11 crates.
- **Registry delta is +2 rules + 1 strengthening, NOT a removal.** `DC-NODE-30` + `DC-EVIDENCE-03` were declared
  at the cluster doc (`645c0067`) then flipped — `DC-NODE-30 → enforced` at AJ-S2 (`e577bd3b`), `DC-EVIDENCE-03
  → enforced_scaffolding` at AJ-S3 (`b1bed361`); `DC-ADMIT-04` gained `strengthened_in += PHASE4-N-AJ` at AJ-S1.
  The sorted-id `comm` confirms zero removals. **`CN-CONS-03` was NOT flipped** — it stays `declared`
  (single-best-peer scope; CE-AI-6 operator-gated/vacuous-until-committed); its `strengthened_in` carries
  `PHASE4-N-B`, `PHASE4-N-AI` — **not** `PHASE4-N-AJ` (no N-AJ strengthening of `CN-CONS-03`).
- **New status value `enforced_scaffolding`.** `DC-EVIDENCE-03` is the first rule to carry
  `status = "enforced_scaffolding"` — the gate (`ci_check_convergence_evidence_schema.sh`, reused from N-AI
  AI-S5) runs and is closed, but the operator-produced transcript is not yet committed, so the rule is
  scaffolding-enforced (vacuous-until-committed), not satisfied. Recorded faithfully as its own status tier.
- **Classification note (TCB).** The new `ade_node::convergence_evidence` is **GREEN** for the event *selection*
  (which `AdmissionLogEvent` variant + the `verdict::derive` mapping; generic over `W: Write`) and **RED** for
  the file-backed instantiation (`File::create` + byte writes). `ade_node::node_lifecycle` + `cli` are **RED**
  (the loop + CLI); `ade_node::node_sync` is **GREEN** (the unchanged detector/resolver, +2 plumbing lines).
  `ade_node` is neither a BLUE `core_paths` crate nor `ade_runtime` (the RED shell crate); per the project's TCB
  scoping the `ade_node` types are non-BLUE.
- **No `RO-LIVE` flip; CE-AI-6 is operator-gated/vacuous.** `DC-NODE-30` is recorded `enforced` for the
  evidence-emit scope, backed by hermetic enforcement (the two new gates + unit/replay tests). `DC-EVIDENCE-03`
  (the convergence-through-reorg transcript shape) is `enforced_scaffolding` — vacuous-until-committed,
  sha256-bound — **NOT** a bounty/preprod claim. No `RO-LIVE` registry status changed this span (`RO-LIVE-01`
  stays operator-gated / partial).
- **Normative docs unchanged this span.** `git diff --name-only e99a86c7..HEAD` over the configured
  `normative_docs` (CE-79 statement + addendum, the three contract docs, CE-73 reclassification, `CLAUDE.md`)
  is empty — the §7 delta is entirely the invariant-registry change.
- **§1 commit log verbatim from `git log --oneline --no-merges` (newest first).** The per-slice synthesis is in
  §0/§3. Every subject carries a conventional-commits prefix; `c95e2592` (`docs(c2-guide): sync …`) is
  **unrelated to N-AJ** and recorded as such (docs-only, folded into the span by date range).
- **Doc-refresh state — CODEMAP + SEAMS + TRACEABILITY one cluster STALE (refresh owed).** All three remain
  pinned at the N-AI close `5ec841c8` (`grep -c DC-NODE-30 / DC-EVIDENCE-03 / ci_check_convergence_evidence_emit_only`
  in each = 0; CODEMAP's 5 `convergence_evidence` hits are the N-AI AI-S5 schema-gate references, not the AJ
  module; SEAMS + CODEMAP headers still read "460 canonical types / 157 CI checks at `5ec841c8`").
  **Cross-reference warnings surfaced in §2 (new module not in CODEMAP) and §5 (new gates not in
  TRACEABILITY):** regenerate CODEMAP + SEAMS + TRACEABILITY to `b1bed361` as a follow-on this close; the
  registry holds the two new rules + both gate bindings authoritatively in the interim (356 rules). No orphan
  gate — each of the two new gates enforces its named rule.
- **Working tree NOT clean.** This regen runs with the N-AJ close artifacts **uncommitted** (registry status
  flips + slice-doc `Merged` flips + cluster-doc fix + c2-guide update). The remaining close-pass actions are
  committing the close artifacts, the CODEMAP + SEAMS + TRACEABILITY refresh, and the baseline bump (`e99a86c7 →
  b1bed361`) — all separate post-close steps; this regen does **not** touch `.idd-config.json`
  `head_deltas_baseline`.

### Regen `8e2c3672 → 5ec841c8` (PHASE4-N-AI live fork-choice rollback-follow wiring — prior lead)

- **One single-theme cluster + an unrelated docs commit, preceded by the N-AH baseline-bump chore**, measured
  from `8e2c3672` (the PHASE4-N-AH close). **26** commits / **46** files / **+5350 / −53**; CI gates **148 →
  157** (+9 new, 0 modified, 0 removed); registry **347 → 354** (+7: `DC-NODE-23..29` enforced; `CN-CONS-01`
  flipped partial→enforced; 13 strengthenings all `PHASE4-N-AI`; 0 removed; status 213 → 221 enforced, 20 → 19
  partial, 114 declared unchanged); BLUE canonical types **458 → 460** (the +2 in `ade_ledger::wal::event` — the
  first BLUE delta since G-N; `select_best_chain` production byte-identical, one `#[cfg(test)]` permutation
  proof). **`CN-CONS-03` was NOT flipped** (stayed `declared`, strengthened only — single-best-peer scope). H-1
  found at cluster-close (mixed peer/local rollback target) → remediated by AI-S6 / `DC-NODE-29` (durable stored
  point as sole authority, validated pre-mutation, fail-closed) → re-review **H-1 CLOSED.** No `RO-LIVE` flip
  (CE-AI-6 operator-gated/vacuous-until-committed). All four grounding docs were regenerated at the N-AI close to
  `5ec841c8`. Full notes recoverable from this doc's git history at `5ec841c8` / `e99a86c7`.
