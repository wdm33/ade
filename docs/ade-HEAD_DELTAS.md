# Ade — HEAD Deltas (Changes Since Baseline)

> **Status:** Living architectural document. Regenerated; not hand-edited.
>
> Regenerate with `/head-deltas <baseline>` after every cluster close. Baseline is recorded in `.idd-config.json` `head_deltas_baseline`.

> Baseline: `4e358e92` (refresh stale G-R serve-handoff comment in containment gate (post-N-U-S3), 2026-06-05 17:17)
> HEAD: `999199f8` (repair 10 pre-existing gate-vs-code drifts (gate hygiene; 0 invariants weakened), 2026-06-05 19:28)
> Span: **PHASE4-N-U cluster CLOSE + a gate-hygiene / close-correction tail** — the close commit (docs-only: archive + 4-grounding-doc refresh + baseline bump) followed by three CI-only commits that make the full `ci/ci_check_*.sh` sweep trustworthy as release evidence.
> 4 commits (no merges), 23 files changed, +1063 / -658 lines.

> **Baseline note (load-bearing — read before §0).** This window's baseline is **`4e358e92`**, the
> `.idd-config.json` `head_deltas_baseline` set by the *previous* (PHASE4-N-U) regen — and it is
> **valid**: `git rev-parse 4e358e92` resolves and `git merge-base 4e358e92 HEAD == 4e358e92` (it is a
> strict ancestor of HEAD; `4e358e92` carries no tag). HEAD is **`999199f8`** (the real working HEAD).
> The span is **not a feature cluster** — it is the **PHASE4-N-U close pass** (commit `7f00e75d`,
> docs-only) plus a **gate-hygiene / close-correction tail** of three CI-only commits (`60deecf3`,
> `e92b40b7`, `999199f8`). It ships **NO new code authority, NO new canonical type, NO new rule, and NO
> new CI gate**; its product is a CI sweep that is now **135 passed / 0 failed**. The closer bumps
> `head_deltas_baseline` `4e358e92 → 999199f8` after this regen so the next cluster measures from here.

This window is a **close + gate-hygiene span, not a cluster lead.** It answers one operational
question that the PHASE4-N-U close left open: *is the `ci/ci_check_*.sh` sweep trustworthy as release
evidence — does **GREEN actually mean GREEN**?* At the N-U close the answer was **no**: the close
commit (`7f00e75d`) itself recorded **"12 pre-existing gate failures remain (gate drift in files N-U
never touched)"** — the gate sweep was red for reasons unrelated to the cluster's work, so a green-vs-red
sweep could not be used as a release signal. This span closes that gap by **repairing every failing
gate in place** — adding no gate, removing no gate, weakening no invariant:

- **The close itself (`7f00e75d`, docs-only).** Archives the PHASE4-N-U cluster doc to
  `docs/clusters/completed/PHASE4-N-U/`, refreshes all four grounding docs at the close (CODEMAP /
  SEAMS / TRACEABILITY / HEAD_DELTAS — to the mutually-consistent **458 BLUE types / 135 CI checks /
  333 rules** figures), flips the S2/S3 slice docs `in progress → done`, and bumps
  `head_deltas_baseline` `65954fa3 → 4e358e92`. **No code, no CI, no registry change.**
- **The N-U-stranded gate, reconciled (`60deecf3`).** N-U S3 retired the `--mode node` spine
  `SelfAcceptedHandoff → push_atomic` accumulator (`DC-NODE-13`) — but the DC-NODE-06 handoff-fence
  gate `ci_check_served_chain_handoff_fence.sh` kept fencing the *retired* mechanism (its "no
  push_atomic on the node spine" check inverted once N-U removed the very push it required). The N-U
  `/cluster-close` set-based gate-diff **masked** this (the gate was already failing for an unrelated
  same-line technicality, so it appeared in both pre- and post-impl failing sets and was mis-classified
  pre-existing). Corrected post-close: the gate is **repointed** (not retired — CI count stays 135,
  avoiding churn in the just-refreshed docs) to fence the **evolved durable-provenance serve**
  (node-spine serve sources ONLY `ServedChainSource::DurableChainDb`); `DC-NODE-06 strengthened_in +=
  PHASE4-N-U`. **No code change.**
- **The silent secret-scan, made to actually run (`e92b40b7`).** `ci_check_no_secrets.sh` was exiting
  **126** ("Argument list too long") and therefore **silently not scanning** — a security gate
  providing zero protection. Root cause: the 6756-entry `git ls-files` list was exported as an env var,
  and `execve` packs args + env together against `ARG_MAX`, failing `E2BIG` before the scan ran. Fixed
  by passing the file list via a **temp file** (small path in the env); the scan now runs (**6756 files,
  0 secrets**), with IPv4 false-positive tuning for version-shaped tokens + synthetic placeholder IPs
  (real AKIA / PEM / hostname / routable-IP patterns still fail closed).
- **Ten pre-existing gate-vs-code drifts repaired (`999199f8`).** Nine gate scripts with stale grep
  patterns / allow-lists / paths that lagged later cluster work, plus two source comment/header edits.
  Each fix only stops a false positive or repoints a stale path; **the protected invariant holds in
  code in every case** (triaged + confirmed no genuine code regression).

The triage verdict is the load-bearing claim: of the 12 gate failures the N-U close recorded,
**0 were genuine code-invariant regressions** — **11 were stale-gate drift** (gate scripts that lagged
behind earlier cluster work in files N-U never touched) and **1 was the N-U-stranded DC-NODE-06 gate**
(reconciled in `60deecf3`). After this span the **full `ci/ci_check_*.sh` sweep is 135 passed / 0
failed** (verified by running every gate at HEAD). The sweep is now usable as release evidence:
green means green. This window **flips no `RO-LIVE` rule**, makes no bounty-accept claim, and changes
no authoritative behavior; it is pure **enforcement-trustworthiness** work.

## 0. Headline

| Count | Baseline (`4e358e92`) | HEAD (`999199f8`) | Δ |
|---|---|---|---|
| CI gates (`ci/ci_check_*.sh`) | 135 | **135** | **0** — **11 gates repaired IN PLACE** (no gate added, no gate removed): `no_secrets` made-to-run (`e92b40b7`), `served_chain_handoff_fence` repointed (`60deecf3`), + **9** drift fixes (`999199f8`). The count is held at 135 deliberately to avoid churn in the just-refreshed grounding docs. |
| **Full gate sweep result** | 12 failing (per N-U close record) | **135 passed / 0 failed** | **the headline.** Every `ci/ci_check_*.sh` exits 0 at HEAD (verified by running the sweep). GREEN-MEANS-GREEN: the sweep is now trustworthy as release evidence. |
| Registry rules (`docs/ade-invariant-registry.toml`) | 333 | **333** | **0** — identical ID set (`comm` of sorted id lists is empty). The lone registry edit (`60deecf3`) is the **`DC-NODE-06` strengthening** (`strengthened_in += "PHASE4-N-U"`; `ci_script` gains the projection gate; `tests`/`code_locus`/`cross_ref`/`evidence_notes` reconciled to the N-U S3 supersession) — **not a new rule**. |
| Registry status (enforced / partial / declared) | 201 / 20 / 112 | **201 / 20 / 112** | **0** — no status flip; the DC-NODE-06 strengthening does not change its `enforced` status. |
| BLUE canonical types | 458 | **458** | **0** — per the CODEMAP header at both refs. No BLUE type added or removed; the only BLUE-file touch this span is a **comment-only** Core-Contract header prepend (`block_validity/mod.rs`, +7/−0). |
| Grounding docs (CODEMAP / SEAMS / TRACEABILITY) | pinned at G-K…G-R catch-up | **refreshed to N-U HEAD `4e358e92`** | All four advanced to **458 / 135 / 333** by the close commit `7f00e75d`. **CODEMAP was intentionally NOT regenerated again** in the post-close hygiene commits (it is structurally current — the hygiene span added no module, type, or rule). |

The span has **no slice↔rule↔gate map** — it introduces no slice, no rule, and no gate. Its product is
a **trustworthy gate sweep**. The per-commit shape:

| Commit | Kind | What it did | Code / CI / registry effect |
|---|---|---|---|
| `7f00e75d` | docs-only (close) | Archive N-U cluster doc; refresh 4 grounding docs; flip S2/S3 slice status; bump baseline `65954fa3 → 4e358e92` | **0 code / 0 CI / 0 registry** (only the 4 docs + `.idd-config.json` + the archived cluster/slice docs) |
| `60deecf3` | fix(ci) (close correction) | Repoint the N-U-stranded `served_chain_handoff_fence.sh` to the evolved durable-provenance serve; record the DC-NODE-06 strengthening + supersession note | **0 code**; 1 gate repointed; `DC-NODE-06 strengthened_in += "PHASE4-N-U"` (no new rule) |
| `e92b40b7` | fix(ci) (gate hygiene 1/2) | Make `no_secrets` actually run (file-list via temp file past `ARG_MAX`) + IPv4 false-positive tuning | **0 code**; 1 gate (security) made-to-run; 0 registry |
| `999199f8` | fix(ci) (gate hygiene 2/2) | Repair 9 stale gate scripts + 2 source comment/header edits | **2 comment-only source edits**; 9 gates repaired; 0 registry |

> **Cross-reference (other grounding docs) — all consistent at HEAD, no expected catch-up.** Unlike the
> N-U regen (which was the first of the four docs to advance), this span's close commit `7f00e75d`
> refreshed **all four** grounding docs together to the N-U HEAD `4e358e92` (CODEMAP / SEAMS /
> TRACEABILITY headers all read **458 types / 135 CI / 333 rules**, mutually consistent). The post-close
> hygiene commits touched **only `ci/`, two source files (comment-only), and the registry (the
> DC-NODE-06 strengthening)** — they introduced no module, type, or rule, so **CODEMAP / SEAMS were
> deliberately left unregenerated** (structurally current). The only doc this span re-touches is
> **TRACEABILITY-adjacent**: the registry's DC-NODE-06 `ci_script` now lists three gates (see §5/§7),
> which a TRACEABILITY refresh would reflect — but no rule or row count changed (still 333).

## 1. Commit Log (newest first)

| Hash | Type | Summary |
|------|------|---------|
| `999199f8` | fix(ci) | repair 10 pre-existing gate-vs-code drifts (gate hygiene; 0 invariants weakened) |
| `e92b40b7` | fix(ci) | make no_secrets actually run (batch file list past ARG_MAX) + tune IP false positives |
| `60deecf3` | fix(ci) | repoint DC-NODE-06 handoff fence to durable-provenance serve (N-U S3 close correction) |
| `7f00e75d` | chore (close) | Close PHASE4-N-U — forged-block durability (full producer own-tip advance) |

No merge commits in the span. **4 commits, zero unclassified** — three carry the `fix(ci):`
conventional prefix; the close commit `7f00e75d` is a `/cluster-close`-style record (docs-only, no
conventional prefix by convention — its diff scope is exclusively `docs/` + `.idd-config.json`, so it
classifies `docs`/`chore`). The shape is **close-then-correct-then-harden**: the docs-only close
(`7f00e75d`), then the single close-correction (`60deecf3`, the N-U-stranded gate), then the two-part
gate-hygiene follow-up (`e92b40b7` security-scan + `999199f8` the nine drift fixes). All four commits
landed the same day (2026-06-05, 17:58 → 19:28).

> **Note (commit-attribution policy).** Per this repo's `CLAUDE.md` override (vibe-coded-node bounty
> trailer requirement), commits in this repo carry a `Co-Authored-By:` model-attribution trailer
> (`git show` confirms it on all three `fix(ci)` commits and the close); that is an Ade-local override
> of the global no-AI-attribution rule and applies to **commit messages only**. It does not affect this
> doc's content.

## 2. New Modules

**None.** `git diff --name-status 4e358e92..999199f8` shows **no `A` (added) source file** — the only
non-`M` entries are the four `R` renames that archive the PHASE4-N-U cluster doc
(`docs/clusters/PHASE4-N-U/ → docs/clusters/completed/PHASE4-N-U/`). No new crate, no new workspace, no
new `Cargo.toml`, no new `.rs` file. The span is **modification only** — CI scripts (§5), two
comment-only source edits (§3), the four grounding docs, and the DC-NODE-06 registry strengthening
(§7).

> **Cross-reference (CODEMAP) — current, no addition owed.** This span adds no module, so CODEMAP
> requires no new §RED/§GREEN/§BLUE row. The N-U module (`ade_runtime::network::served_chain_projection`)
> was already added to CODEMAP by the close commit `7f00e75d` (the N-U refresh). CODEMAP is structurally
> current at HEAD.

## 3. Modules Modified

Only **two source files** changed this span, both in the gate-hygiene commit `999199f8`, and **both are
comment/header-only** (zero logic, zero signature change, zero type change). They are recorded here for
completeness; neither is a behavioral modification.

| Module | Scope | Key changes |
|--------|-------|-------------|
| `ade_ledger::block_validity` (`crates/ade_ledger/src/block_validity/mod.rs`) | +7 / −0, **comment-only** (BLUE `core_paths`) | **`999199f8`.** Prepends the canonical `// Core Contract:` determinism header (same-inputs+seed ⇒ byte-identical; no wall-clock / rand / `HashMap` / float; encode invariants in types; explicit transitions; canonical serialization). The module **predated the header convention**; `ci_check_module_headers.sh` requires it. **No code, no type, no signature change** — the BLUE canonical-type count stays 458. |
| `ade_runtime::seed_import` (`crates/ade_runtime/src/seed_import/importer.rs`) | +2 / −1, **doc-comment-only** (RED shell) | **`999199f8`.** Refreshes a **stale module-doc line**: `//! Reference scripts → fail-fast UnsupportedTxOutFeature` → `//! Reference scripts → supported (A1.1): matched + encoded via encode_script_ref, fail-closed on malformed via BadReferenceScript`. The doc line lagged the **A1.1** change (reference scripts became *supported*, not fail-fast); this aligns the comment to code reality. Paired with the `ci_check_admission_no_refscript_skip.sh` Guard-2 repoint (§5). **No code, no signature change.** |

> **No BLUE-authority change (load-bearing).** The one BLUE-file touch this span is a **comment-only
> Core-Contract header prepend** — it adds no logic, no function, no type, and changes no wire byte.
> The BLUE canonical-type count is **458 → 458** (CODEMAP header at both refs). There is no `ade_core` /
> `ade_codec` / `ade_types` / `ade_crypto` / `ade_plutus` / `ade_network`-BLUE source change at all.

## 4. Feature Flags

**No project feature-flag deltas.** Ade declares no `[features]` table in any workspace `Cargo.toml`,
and **no `Cargo.toml` changed in this window** (`git diff --name-only 4e358e92..999199f8 -- '**/Cargo.toml'
'Cargo.toml'` is empty). No `#[cfg(feature = …)]` gate was introduced; no coupling, no `compile_error!`
guard. Two gate scripts (`ci_check_scheduler_closure.sh`, `ci_check_no_signing_in_blue.sh`) were taught
to **strip `#[cfg(test)]` blocks** before scanning (so a test-only `std::time` / `.sign(` reference no
longer trips a production-purity gate), but that is a **gate-scoping** change, not a feature-flag
change — see §5.

## 5. CI Checks (135 → 135; 11 gates repaired in place, 0 added, 0 removed)

The product of this span. **No gate was added or removed** — the count is held at **135** to avoid
churn in the just-refreshed grounding docs — and **eleven gates were repaired in place** so the full
sweep passes. `git diff --diff-filter=A 4e358e92..999199f8 -- ci/` and `--diff-filter=D` are both
**empty**; `--diff-filter=M` lists exactly the eleven `ci_check_*.sh` below. Running the full sweep at
HEAD yields **135 passed / 0 failed**.

> **Sweep result (load-bearing, verified).** Running every `ci/ci_check_*.sh` at HEAD `999199f8`:
> **135 gates, 135 passed, 0 failed.** At the N-U close the close record stated **"12 pre-existing gate
> failures remain (gate drift in files N-U never touched)."** Triage of those 12: **0 genuine
> code-invariant regressions**; **11 stale-gate drift** (gate scripts lagging earlier cluster work) +
> **1 N-U-stranded gate** (`DC-NODE-06` / `served_chain_handoff_fence.sh`). All resolved in this span.
> (The 12th of the original tally, `ci_check_registry_code_locus_exists.sh`, was already **net-fixed by
> N-U itself** per the close record, so the surviving repair set is the 11 below.) The sweep is now
> usable as release evidence: a green sweep means the gated invariants hold.

### Close-correction gate (N-U-stranded — `60deecf3`)

| Check | Status | Origin / change | What it checks |
|-------|--------|-----------------|----------------|
| `ci_check_served_chain_handoff_fence.sh` | **Repaired in place (repointed)** | PHASE4-N-F-G-B origin (`DC-NODE-06`); **N-U S3 stranded → repointed** | N-U S3 (`DC-NODE-13`) retired the `--mode node` spine `SelfAcceptedHandoff → push_atomic` accumulator, so the gate's "no push_atomic on the node spine" check **inverted** (N-U removed the very push the gate required). Repointed (not retired — count stays 135) to fence the **evolved durable-provenance serve**: the node-spine serve sources **only** `ServedChainSource::DurableChainDb` (the `ChainDbServedSource` projection over `Arc<dyn ChainDb>`); **no** retired non-durable serve ingress (`push_atomic` / `served_chain_admit` / `ServedChainHandle` / `SelfAcceptedHandoff` channel on the node spine), and the `--mode produce` serve path (`CN-PROD-04`) legitimately retains the handoff carrier. The renamed `relay_loop_containment` test reference is fixed. **The DC-NODE-06 deeper invariant — only validated/admitted bytes may be served on the node spine — is preserved + strengthened** (durable-provenance; now survives restart). Mirrors the DC-NODE-11 supersession treatment. **No code change.** |

### Security gate made-to-run (`e92b40b7`)

| Check | Status | Origin / change | What it checks |
|-------|--------|-----------------|----------------|
| `ci_check_no_secrets.sh` | **Repaired in place (made to actually run)** | secret-hygiene gate; **fixed silent exit-126** | Was exiting **126** (`Argument list too long`) → **silently not scanning** (a security gate with zero protection). Root cause: the 6756-entry `git ls-files` list was exported as an **env var**; `execve` packs args + env against `ARG_MAX`, failing `E2BIG` before the `python3` scan ran. Fix: pass the file list via a **temp file** (small path in the env), not the list itself. The scan now runs — **6756 files, 0 secrets**. IPv4 false-positive tuning (the now-running scan surfaced pre-existing data, zero real secrets): version-shaped tokens in a version context (OpenSSL `3.0.14.4` in `.uplc` vectors, `cardano-cli 11.0.0.0` in doc comments) and synthetic placeholder IPs (`1.1.1.1`/`2.2.2.2`/`3.3.3.3`/`1.2.3.4` in CLI + log tests). **Tuning is IPv4-only** — real `AKIA…` keys, PEM/`.key`/`id_rsa`, `BEGIN PRIVATE KEY`, `amazonaws.com`/`ec2` hosts, ssh strings, and any non-placeholder routable IP still **fail closed**. |

### Drift-repaired gates (`999199f8` — nine stale gate-vs-code drifts)

Each fix only stops a **false positive** or **repoints a stale path**; the protected invariant holds in
code in every case (triaged, no genuine regression), and each gate exits 0 after the fix.

| Check | Status | Drift repaired | Invariant (still enforced) |
|-------|--------|----------------|----------------------------|
| `ci_check_server_paths_corpus_present.sh` | **Repaired in place** | PROCEDURE path repointed `→ docs/clusters/completed/PHASE4-N-G/` (PHASE4-N-G was archived) | Server-path corpus presence. |
| `ci_check_forge_purity.sh` | **Repaired in place** | Allow-list `leader_check.rs` (`is_leader_for_vrf_output` relocated there in N-R-A; the single-authority property is owned by the sibling `ci_check_leader_check_authority.sh`) | Forge-path purity. |
| `ci_check_ingress_chokepoints.sh` | **Repaired in place** | Exclude `minicbor::decode::Error` **type** refs (the `impl From<…>` error-conversion in 3 internal encoders is not a decode call) | Ingress decode chokepoints (real `minicbor::decode(` / `Decode` derives still fail closed). |
| `ci_check_mempool_ingress_closure.sh` | **Repaired in place** | Allow-list `producer/forge.rs` (reject-only admit-prefix re-validation; cannot false-accept) + exclude `rollback/*_cache.rs` | Mempool ingress closure. |
| `ci_check_scheduler_closure.sh` | **Repaired in place** | `EXTRA_TIME_HITS` sub-check now strips `//` comments + `#[cfg(test)]` (was matching `std::time` inside a test-fn comment); timing-integration-test whitelist preserved | Scheduler determinism / no-wall-clock closure. |
| `ci_check_wire_only_event_vocabulary_closed.sh` | **Repaired in place** | Narrow Rule 1's scan to the wire-only surface (`live_log/` + `wire_only.rs`); `agreement_verdict` is a legitimately-registered admission event owned by the sibling `ci_check_admission_log_vocabulary_closed.sh` | Closed wire-only event vocabulary. |
| `ci_check_admission_no_refscript_skip.sh` | **Repaired in place** | Guard 2 repointed: A1.1 replaced the A1 refscript fail-fast with full **support**, so verify the current fail-closed surface (`match &entry.reference_script` + `BadReferenceScript`) instead of the retired A1 literals (no-silent-skip stays enforced by Guard 1a). Paired with the `seed_import/importer.rs` doc-line refresh (§3). | No silent reference-script skip on admission. |
| `ci_check_no_signing_in_blue.sh` | **Repaired in place** | Strip `//` comments + `#[cfg(test)]` and allow-list the **BLUE Sum6KES algorithm** (`ade_crypto/src/kes_sum/`, PHASE4-N-P — deterministic crypto defining KES `SigningKey` **types**, not custody; custody stays RED, `OP-OPS-04`) | No production signing in BLUE (verified: surviving `.sign(`/`SigningKey` hits are test fixtures feeding the verifier, or the `kes_sum` algorithm). |
| `ci_check_self_accept_gate.sh` | **Repaired in place** (`CN-CONS-07`) | Guard 1a now matches struct-literal **construction** only (excludes fn-return types `-> [&]AcceptedBlock {`); Guard 1b counts the fallible ctor signature `-> Result<AcceptedBlock, SelfAcceptError>` (the sole `self_accept`), so the G-B handoff accessors (`into_accepted`/`accepted`, returning an already-built token) are no longer mis-counted | `AcceptedBlock` private-ctor-only (a real struct literal anywhere still fails Guard 1a — sole surviving hit is `self_accept.rs`). |

> **Triage honesty (load-bearing).** This span is **gate hygiene, not invariant change**: **0 invariants
> were weakened.** Of the 12 gate failures the N-U close recorded, **0 were genuine code-invariant
> regressions** — 11 were stale-gate drift (gate scripts lagging earlier cluster work in files N-U never
> touched) + 1 was the N-U-stranded `DC-NODE-06` gate. Each drift fix narrows a false-positive or
> repoints a stale path; the security gate (`no_secrets`) went from *silently not running* to
> *running + passing* (a **strengthening** of effective enforcement, not a weakening). After the span the
> full `ci/ci_check_*.sh` sweep is **135 passed / 0 failed**.

> **Cross-reference (TRACEABILITY).** No rule↔gate binding was added or removed this span; all 11
> repaired gates were already bound to their rules. The one binding **change** is `DC-NODE-06`, whose
> `ci_script` now lists three gates — `ci_check_served_chain_handoff_fence.sh` (repointed) +
> `ci_check_node_run_loop_containment.sh` + `ci_check_served_chain_projection.sh` — reflecting the N-U S3
> supersession. A TRACEABILITY refresh would update DC-NODE-06's CI cell to the three gates; the row
> count stays 333.

## 6. Canonical Type Registry Delta

**n/a — no separate canonical-type registry is configured** (`canonical_type_registry: null`);
canonical-type rules live inline in the invariant registry under family **T**. **No canonical type was
added or removed in this window** (BLUE count unchanged, **458 → 458**, per the CODEMAP header at both
refs). The only BLUE-file touch is a **comment-only** Core-Contract header prepend
(`block_validity/mod.rs`); it adds no `struct`/`enum`/`fn`. No `Cargo.toml` changed.

## 7. Normative / Invariant Rule Delta (333 → 333; one strengthening, zero adds, zero removals)

**No rule ID was added or removed** (333 → 333; `comm` of the sorted id lists at both refs is empty —
identical ID sets). The status tally is unchanged (**201 enforced / 20 partial / 112 declared**). The
**only** registry edit this span (commit `60deecf3`) is a **strengthening of `DC-NODE-06`** — recorded
honestly as the N-U S3 close correction.

**Strengthening (`strengthened_in += "PHASE4-N-U"`):**

| Rule | Family / Tier | Strengthening |
|------|---------------|---------------|
| `DC-NODE-06` | DC / `derived` (`enforced`, unchanged) | **Mechanism superseded by N-U S3, deeper invariant preserved + strengthened.** N-U S3 (`DC-NODE-13`) replaced the `--mode node` spine `SelfAcceptedHandoff → push_atomic` accumulator with **serve-as-projection** of the durable ChainDb. DC-NODE-06's gate `ci_check_served_chain_handoff_fence.sh` was **stranded** (it fenced the retired mechanism); it is **repointed** to fence the evolved durable-provenance serve (node-spine serves only `ServedChainSource::DurableChainDb`). The DC-NODE-06 **deeper invariant — only validated/admitted bytes may be served on the node spine — is preserved + strengthened** (durable-provenance per the CN-CONS-07 restatement; now survives restart). Registry edits: `strengthened_in` `["PHASE4-N-F-G-C"] → ["PHASE4-N-F-G-C", "PHASE4-N-U"]`; `ci_script` now lists three gates (handoff-fence + run-loop-containment + the new projection gate); `cross_ref += DC-NODE-13, DC-NODE-12`; `code_locus`/`tests`/`source`/`evidence_notes` reconciled to the supersession (the G-B `served_chain_handle.rs` carrier is now `--mode produce`-only). **Not a weakening, not a removal** — the deny-list-based mechanism is superseded by structure, and the deeper invariant is enforced more strongly. |

> **Close-correction honesty (load-bearing).** The DC-NODE-06 strengthening lands **post-close** because
> the N-U `/cluster-close` set-based gate-diff **masked** the stranding (the gate was already failing for
> an unrelated same-line technicality, so it sat in both the pre- and post-impl failing sets and was
> mis-classified pre-existing — the close commit's "0 N-U-introduced gate regressions" claim was
> therefore *incomplete* for this gate). This is **surfaced in the DC-NODE-06 `evidence_notes`** (a full
> "N-U S3 SUPERSESSION + CLOSE CORRECTION" paragraph) and the archived cluster doc, **not hidden**. It
> mirrors the DC-NODE-11 supersession treatment N-U applied to the retired `served_chain_stability` gate.

**No rule was removed (expected: 0); no rule was added.** The registry delta is **one
`strengthened_in` append + the supersession reconciliation of one rule's metadata** — purely a
strengthening. The N-U *cluster's* rule additions (`DC-NODE-12`, `DC-CONS-23`, `DC-WAL-04`, `T-REC-05`,
`DC-NODE-13`; 328 → 333) landed **before this baseline** (in the N-U slice commits + the prior regen)
and are narrated in the §"Historical — PHASE4-N-U" section below.

## Working tree at HEAD `999199f8`

Clean of tracked changes from this span — the close + corrections are all committed. `git status
--short` shows only an untracked `.mithril-scratch/` (operator scratch, ignored). **This regen runs
*after* all four span commits**, so unlike the N-U regen there is no close-in-progress working tree;
the baseline bump (`4e358e92 → 999199f8`) is the only follow-on action.

## Honest residual (window scope)

This span made the **gate sweep trustworthy** — and nothing more. The honest boundary:

- **Enforcement trustworthiness ≠ new capability — NO `RO-LIVE` flip, NO behavior change.** No
  `RO-LIVE` rule was flipped; `RO-LIVE-01` stays operator-gated. No authoritative behavior changed: the
  two source edits are comment-only, no canonical type / rule was added, no gate was added or removed.
  The span makes **green mean green**; it does not advance the bounty.
- **The N-U cluster's honest residual still stands.** PHASE4-N-U made a forged block durable,
  crash-recoverable, and coherently servable through the same gate received blocks use — but flipped no
  `RO-LIVE` rule and demonstrated no operator-witnessed peer acceptance. The two N-U follow-ons remain
  open (and unaffected by this span): **[MEDIUM]** `ChainDb::iter_from_slot` full-range materialization
  + O(N²) hash recovery with no per-request serve range cap; **[LOW]** > 64 KB block bodies not served
  (fail-closed) + unbounded inbound serve accept. Both are pre-existing and gate any large-chain live
  serve.
- **Triage is the claim, not a guess.** "0 genuine code-invariant regressions / 11 stale-gate drift +
  1 N-U-stranded" is the recorded triage of the 12 close-record gate failures; the security gate went
  from silently-off to on+passing (a strengthening). Every repaired gate was confirmed to exit 0 at
  HEAD, and the full sweep is **135 passed / 0 failed** (verified by running it).
- **CODEMAP intentionally not regenerated this pass.** The post-close hygiene commits added no module,
  type, or rule, so CODEMAP / SEAMS are structurally current at the N-U HEAD `4e358e92` the close
  refreshed them to; re-running `/codemap` would be a no-op churn. The only doc this span makes
  marginally stale is TRACEABILITY's DC-NODE-06 CI cell (now three gates) — a refresh-on-next-cluster
  item, not a discipline gap (the rule count is unchanged at 333).

---

## Historical — PHASE4-N-U window (`65954fa3 → 4e358e92`)

> The section below is the **previous** HEAD_DELTAS lead, preserved in condensed form. It was the
> single-cluster lead **PHASE4-N-U — forged-block durability**, narrating the `65954fa3..4e358e92` span.
> Counts in this Historical section are the figures **at `4e358e92`** (333 rules, 135 CI gates, 458
> canonical types); the current window measures **forward** from `4e358e92`. The full N-U §§0–7 narrative
> is recoverable from this doc's git history at `4e358e92`.

> Baseline: `65954fa3` (G-K…G-R + C1 catch-up close, 2026-06-04 23:32)
> HEAD: `4e358e92` (refresh stale G-R serve-handoff comment in containment gate (post-N-U-S3), 2026-06-05 17:17)
> Span: **PHASE4-N-U — forged-block durability** (own-forged durable admit → forged-tip crash recovery + replay-equivalence → serve-as-durable-chain projection) — 14 commits, 28 files, +3726 / −1802.

PHASE4-N-U answered one structural question every prior forge/serve cluster left open: *once Ade forges
its own block, does that block become part of the **durable** chain — survive a crash, replay
byte-identically, and get served to a follower as durable history — through the SAME gate received
blocks use, with NO second tip-advance path?* Before N-U a forged block was a **local self-accept
artifact only** (`DC-NODE-05`): the forge tick advanced no durable tip, and the served view was an
in-memory accumulator that did not survive restart. N-U closed that gap across three slices:

- **S1 — own-forged durable admit through the pump (`DC-NODE-12` + `DC-CONS-23` + `DC-WAL-04` prior-fp
  clause).** A new fenced RED driver `ade_node::node_sync::admit_forged_block_durably` feeds the
  self-accepted bytes (`accepted.into_bytes()`, no re-encode — I-10) into the **same**
  `forward_sync::pump_block` chokepoint received blocks use (`StoreBlockBytes → AppendWal → AdvanceTip`,
  durable-before-tip, behind the BLUE admit authority, **extend-only** — a stale-tip re-forge fails
  closed). The forge gains **no** second tip-advance path; new gate
  `ci_check_forged_durable_admit_via_pump.sh`.
- **S2 — forged-tip crash recovery + replay-equivalence (`T-REC-05`, `DC-WAL-04` no-orphan clause).**
  Production `warm_start_recovery` now forward-replays from the nearest snapshot ≤ tip and reconciles
  the WAL tail, so a forge-then-kill recovers the same durable tip byte-identically; an un-WAL'd forged
  orphan above the WAL tail is dropped. `T-REC-05` is **test-enforced** (`ci_script = ""`).
- **S3 — serve-as-durable-chain projection (`DC-NODE-13`; strengthens `CN-CONS-07`, `DC-NODE-11`).**
  The `--mode node` served view became a deterministic read-only **projection of the durable ChainDb**
  (the NEW RED module `ade_runtime::network::served_chain_projection` / `ChainDbServedSource`, plus the
  closed `ServedChainSource { Snapshot | DurableChainDb }` selector in `serve_dispatch`), retiring the
  in-memory accumulator + the G-R monotone serve gate. New gate `ci_check_served_chain_projection.sh`;
  retired gate `ci_check_served_chain_stability.sh` (mechanism superseded).

**N-U headline (at `4e358e92`):** Registry **328 → 333** (+5 `enforced`: `DC-NODE-12`, `DC-CONS-23`,
`DC-WAL-04`, `T-REC-05`, `DC-NODE-13`; +2 strengthenings: `CN-CONS-07`, `DC-NODE-11`; 0 removed; 196 →
201 enforced). CI gates **134 → 135** (**+1 net**: +2 new (`forged_durable_admit_via_pump` S1,
`served_chain_projection` S3); −1 retired (`served_chain_stability`, G-R mechanism superseded by S3); +3
modified in place (`node_run_loop_containment`, `node_serve_lifetime`, `feed_tag24_unwrap`)). **One new
RED module** (`served_chain_projection`). **BLUE canonical types 458 → 458** — the lone BLUE touch
factored `block_header_bytes(&[u8])` out of `accepted_block_header_bytes` (same `DC-CONS-18` recipe; one
new fn, no new type). **No `RO-LIVE` flip** — durability + coherent serve ≠ operator-witnessed peer
acceptance; `RO-LIVE-01` stayed operator-gated. **Honest S2 drift (recorded at close):** the §8-named
CE-5 gate `ci_check_forged_tip_recovery.sh` + CE-6 test `forge_two_clean_runs_byte_identical` were not
created literally — `T-REC-05` + `DC-WAL-04`(no-orphan) were enforced via the kill-recover
fingerprint-equality tests instead.

> **Connecting note to the current window.** The N-U *close pass + its gate-hygiene tail* is the
> **current** window (`4e358e92 → 999199f8`): the close commit `7f00e75d` refreshed the four grounding
> docs and bumped the baseline; the post-close correction `60deecf3` repointed the **N-U-stranded
> DC-NODE-06 gate** (`served_chain_handoff_fence.sh`, masked by the close's set-based gate-diff) and
> strengthened `DC-NODE-06`; and the two-part gate-hygiene follow-up (`e92b40b7` + `999199f8`) repaired
> the remaining pre-existing gate drift so the full sweep is **135 passed / 0 failed**. See §§0–7 above.

---

## Historical — PHASE4-N-F-G-K … G-R + C1 window (`550eec3a → 65954fa3`)

> Preserved in condensed form. A **multi-cluster catch-up** narrating the `550eec3a..65954fa3` span —
> the PHASE4-N-F-G-J close-pass + eight clusters (G-K through G-R) + the C1 genesis-successor rehearsal
> reproduction evidence. Counts here are the figures **at `65954fa3`** (328 rules, 134 CI gates, 458
> canonical types). The full G-K…C1 §§0–7 narrative (and the G-J window before it) is recoverable from
> this doc's git history at `65954fa3` / `4e358e92`.

> Baseline: `550eec3a` (PHASE4-N-F-G-J close, 2026-06-03 22:02)
> HEAD: `65954fa3` (run-2 genesis-rehearsal reproduction + runbook flag fixes + gate now covers c1 manifests, 2026-06-04 23:32)
> Span: **G-J close-pass → G-K, G-L, G-M, G-N, G-O, G-P, G-Q, G-R → C1 genesis-successor rehearsal evidence** — 28 commits, 73 files, +4967 / −243.

Ade closed **eight clusters** (G-K through G-R) plus a G-J **close-pass** and a C1 **genesis-successor
rehearsal evidence** pass, each peeling off the next concrete blocker on the path to a live C1
genesis-successor follower adopting an Ade-forged block 0 over a real `cardano-node` peer:
serve-listener lifetime (G-K, `DC-NODE-09`) → real-node handshake compat (G-L, `CN-WIRE-10`) →
real-node ChainSync FindIntersect compat (G-M, `CN-WIRE-11`, + the closed BLUE enum
`ArrayHead = Definite(u64) | Indefinite`, the window's only +1 canonical type, 457 → 458) →
recovered-eta0 WarmStart (G-N, `T-REC-04` + `DC-CINPUT-03`) → feed-side tag-24 unwrap (G-O,
`CN-WIRE-12`) → feed-side leader-threshold view (G-P, `DC-CINPUT-04`) → forge-successor position from
the evolved admitted spine (G-Q, `DC-NODE-10`) → stable served block 0 via a monotone serve gate (G-R,
`DC-NODE-11`, gate `ci_check_served_chain_stability.sh`) → and finally the C1 reproduction evidence.

**G-K…C1 headline (at `65954fa3`):** CI gates **126 → 134** (+8 new, one per cluster G-K…G-R, +
`ci_check_rehearsal_manifest_schema.sh` modified for C1, 0 removed); registry **319 → 328** (+9 new, all
`enforced`; 0 strengthenings; 0 removed); BLUE canonical types **457 → 458** (+1 `ArrayHead`); no new
module in that window. **Note:** the G-R gate `ci_check_served_chain_stability.sh` introduced in that
window was **retired in PHASE4-N-U** (mechanism superseded by serve-as-projection), and `DC-NODE-11` was
strengthened there — and `DC-NODE-11`'s stranded sibling `DC-NODE-06` was reconciled in the **current**
window (`60deecf3`).

> *(The G-E…G-I leads were never re-led in HEAD_DELTAS — each was closed with its own grounding-doc
> refresh and lives in its own close-pass commit + the registry. The G-J lead before that is recoverable
> from this doc's git history at `65954fa3`.)*

---

## Generation notes

### Regen `4e358e92 → 999199f8` (PHASE4-N-U close + gate-hygiene tail — current lead)

- **Baseline valid; close-plus-hygiene span, NOT a feature cluster.** Run against the config baseline
  `4e358e92` (the PHASE4-N-U slice-span HEAD), which `git rev-parse` resolves and `git merge-base
  4e358e92 HEAD` confirms is a strict ancestor of HEAD `999199f8` (`4e358e92` carries no tag). The span
  is the **N-U close commit** (`7f00e75d`, docs-only) + a **gate-hygiene / close-correction tail**
  (`60deecf3`, `e92b40b7`, `999199f8`) — **no slice, no new rule, no new gate, no new module, no new
  canonical type.** The closer bumps `head_deltas_baseline` `4e358e92 → 999199f8` after this regen.
- **Counts are mechanical (git/grep/ls + a gate sweep, no cargo build):** commit log + `--shortstat`
  over `4e358e92..999199f8` (**4** commits, no merges / **23** files / **+1063 / −658**); CI gate count
  via `git ls-tree -r --name-only <ref> ci/ | grep -c 'ci_check_.*\.sh$'` at each ref (**135 → 135**;
  `--diff-filter=A` and `--diff-filter=D` over `ci/` both **empty**; `--diff-filter=M` = exactly **11**
  `ci_check_*.sh`); registry rule count via `grep -cE '^\s*id\s*='` at each ref (**333 → 333**; `comm`
  of sorted id lists **empty** — identical ID sets, zero adds, zero removals); registry status via
  `grep -E '^status = ' | sort | uniq -c` at both refs (**201 / 20 / 112**, unchanged); BLUE canonical
  types via the CODEMAP header at both refs (**458 → 458**).
- **Headline verified by running the sweep.** Running every `ci/ci_check_*.sh` at HEAD `999199f8`:
  **135 gates, 135 passed, 0 failed.** This is the load-bearing claim — the gate sweep is now
  trustworthy as release evidence.
- **No new module, +0 canonical type, no Cargo.toml change.** `git diff --name-status 4e358e92..999199f8`
  shows **no `A` source file** (only four `R` renames archiving the N-U cluster doc to
  `docs/clusters/completed/PHASE4-N-U/`). The only source touches are **two comment-only edits** in
  `999199f8` (`block_validity/mod.rs` Core-Contract header +7/−0; `seed_import/importer.rs` stale
  reference-script doc line +2/−1) — no logic, no type, no signature. `git diff --name-only …
  '**/Cargo.toml' 'Cargo.toml'` is empty (no feature-flag delta; no `[features]` table workspace-wide).
- **Registry delta is one strengthening, NOT an add/remove.** The only registry edit (`60deecf3`) is
  `DC-NODE-06`: `strengthened_in += "PHASE4-N-U"`, `ci_script` → three gates, `cross_ref`/`code_locus`/
  `tests`/`source`/`evidence_notes` reconciled to the N-U S3 supersession. `comm` confirms the ID set is
  identical at both refs (333 → 333). Recorded honestly as the N-U-stranded-gate close correction
  (masked by N-U's set-based gate-diff), surfaced in the rule's `evidence_notes`.
- **Triage: 0 genuine regressions.** Of the 12 gate failures the N-U close record named, triage found
  **0 genuine code-invariant regressions** — **11 stale-gate drift** (gate scripts lagging earlier
  cluster work) + **1 N-U-stranded** (`DC-NODE-06` / `served_chain_handoff_fence.sh`). All 11 repaired in
  place across the three hygiene commits (9 in `999199f8`, `no_secrets` in `e92b40b7`,
  `served_chain_handoff_fence` in `60deecf3`); the 12th of the original tally
  (`ci_check_registry_code_locus_exists.sh`) was already net-fixed by N-U itself. **0 invariants
  weakened** — each fix narrows a false positive or repoints a stale path; the security gate
  (`no_secrets`) went from silently-not-running (exit 126, `ARG_MAX`) to running + passing (a
  strengthening of effective enforcement).
- **CODEMAP / SEAMS deliberately NOT regenerated this pass.** All four grounding docs were refreshed to
  the N-U HEAD `4e358e92` by the close commit `7f00e75d` (458 / 135 / 333, mutually consistent). The
  post-close hygiene commits added no module / type / rule, so CODEMAP + SEAMS are structurally current
  — re-running `/codemap` would be no-op churn. The only refresh-on-next-cluster item is TRACEABILITY's
  DC-NODE-06 CI cell (now three gates); the rule/row count is unchanged at 333.
- **Working tree clean.** This regen runs *after* all four span commits, so there is no
  close-in-progress working tree (unlike the N-U regen). `git status --short` shows only an untracked
  `.mithril-scratch/` (operator scratch, ignored). The only follow-on action is the baseline bump
  `4e358e92 → 999199f8`.
