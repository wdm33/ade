# Ade — HEAD Deltas (Changes Since Baseline)

> **Status:** Living architectural document. Regenerated; not hand-edited.
>
> Regenerate with `/head-deltas <baseline>` after every cluster close. Baseline is recorded in `.idd-config.json` `head_deltas_baseline`.

> Baseline: `6f848825` (bound live-feed memory before authoritative decode — PHASE4-N-F-G-E S1, 2026-06-02 12:48)
> HEAD: `6bd60c80` (scan archived bounty home in rehearsal leak gate — PHASE4-N-F-G-D S4, 2026-06-02 17:10)
> Cluster: **PHASE4-N-F-G-D — private-testnet accepted-block bounty DRY-RUN fidelity harness on the `--mode node` spine**, slice span closed; close-pass commit to follow.
> 12 commits (no merges), 24 files changed, +2928 / -499 lines.

This window narrates the **PHASE4-N-F-G-D cluster** — a bounty **dry-run fidelity** cluster
that ships a **path-faithful, NON-PROMOTABLE rehearsal HARNESS** for the private-testnet
(C1) accepted-block dry-run, **with NO BLUE crate changed**. G-D answers one question early
— *will a real Haskell node accept an Ade-forged block when Ade has legitimate leader
rights?* — by exercising the **EXACT** preview/preprod `--mode node` accepted-block path in a
venue where the operator controls stake (a private genesis that makes Ade win slots fast).
The cluster is **four slices**, each fail-faithful to the shared path:

- **S1 (`d4d0f456`) — path fidelity.** Proves the C1 private dry-run uses the **same**
  `--mode node` accepted-block path as preview/preprod (`import_live_consensus_inputs` →
  forge → self-accept → sibling-serve → block-fetch → peer log → correlate) with **NO**
  private-only flag, branch, bootstrap authority, or from-genesis constructor — the only
  differences are operator **inputs** (the private genesis stake allocation) and the
  evidence **label** (S2). New gate `ci_check_node_path_fidelity.sh` pins the `cli.rs` flag
  set to a **28-flag closed allow-list** (guard a) and forbids any from-genesis
  consensus-inputs constructor while requiring `node_lifecycle.rs` to source consensus
  inputs only via the shared `import_live_consensus_inputs` (guard b).
- **S2 (`459cf78d`) — rehearsal evidence surface.** New **GREEN** `ade_node::rehearsal_evidence`
  (`PrivateRehearsalManifest` **wraps** a correlate-produced `Ba02Manifest` in a structurally
  distinct, non-promotable envelope; sole ctor `from_correlate_outcome` returns `None` on
  `NoEvidence`; `to_canonical_toml` always emits `is_rehearsal = true` + `not_bounty_evidence
  = true` as **literals**). New **RED** `ade_node::rehearsal_pass` (file I/O that **reuses**
  `ba02_pass::correlate_peer_log_file` verbatim and writes only a `PrivateRehearsalManifest`).
  New gate `ci_check_rehearsal_manifest_schema.sh` — vacuous-until-committed; closed
  **12-field** schema + private-testnet venue + the two non-promotability markers +
  `peer_log_file_sha256` binding; **three** non-promotability barriers.
- **S3 (`076a5af5`) — C1 dry-run runbook + operator scaffold.** A runbook that is a provable
  **strict subset** of the G-C preprod operator-pass runbook, plus the RED test file
  `node_c1_dry_run_rehearsal.rs` (a hermetic correlate→envelope proof + the env-gated
  `node_c1_dry_run_rehearsal_live` operator harness — a RED test skipped in CI, **NOT** a
  runtime node mode). **No** binary wiring, no new flag, no new gate, no synthetic manifest.
- **S4 (`6bd60c80`) — close-surfaced security fix.** The S2 rehearsal leak gate's barrier
  (b) was **dead** after G-C's archival (the pre-S4 `[[ -d ]]` guard skipped the whole check
  because the active home had moved to `docs/clusters/completed/`). S4 repoints the gate to
  scan **both** real bounty homes (active **and** archived) fail-closed and adds a durable
  regression test (`rehearsal_gate_archived_home.rs`).

The new rule `CN-REHEARSAL-FIDELITY-01` (`tier = release`, two coupled clauses — path
fidelity + evidence non-promotability) is **enforced** at this close (in the working-tree
registry; committed as `declared` at the slice-span HEAD — see the sequencing note below).
**G-D closes a NARROW claim: the rehearsal HARNESS is path-faithful and non-promotable.** It
does **NOT** enforce that a C1 run has succeeded (the rehearsal gate is **vacuous until a
real operator-produced manifest is committed**); it flips **NO** RO-LIVE rule; it makes **NO**
bounty / preview / preprod completion claim; the live C1 execution stays
`blocked_until_operator_c1_net_executed`.

The span also carries the **PHASE4-N-F-G-E close tail** (`da205bff`) — docs/registry only —
which lands inside this window because the baseline `6f848825` is the N-F-G-E *slice-span*
HEAD, not its close commit (see §1). This mirrors how the previous HEAD_DELTAS window
carried the **G-C** close tail (`351d46bc`), and the ones before that the **G-B** tail
(`febee120`) and the **G-A** tail (`62cb8718` + `1806584c`), for the same
slice-span-vs-close-commit reason.

## 0. Headline

| Count | Baseline (`6f848825`) | HEAD (`6bd60c80`) | Δ |
|---|---|---|---|
| CI gates (`ci/ci_check_*.sh`) | 119 | **121** | **+2 new** (`node_path_fidelity` G-D S1, `rehearsal_manifest_schema` G-D S2); none removed, none modified |
| Registry rules | 314 | **315** | **+1** (`CN-REHEARSAL-FIDELITY-01`, G-D); no other ID added — G-D records **no** `strengthened_in` bump (it does not advance the bounty deliverable) |
| Test attributes (`#[test]`/`#[tokio::test]`, workspace) | 2234 | **2241** | **+7** (the seven G-D tests, across `node_path_fidelity` + `rehearsal_pass` + `node_c1_dry_run_rehearsal` + `rehearsal_gate_archived_home`) |
| BLUE canonical types | 456 | 456 | **0 — NO BLUE change** (`git diff --name-only 6f848825..6bd60c80` matches **no** BLUE `core_paths` entry; the changed source is all `ade_node` — RED `rehearsal_pass` + GREEN-by-content `rehearsal_evidence` + the `lib.rs` `pub mod` lines) |

> **Registry / grounding-doc sequencing note (load-bearing — read before §7).** This window
> contains **two** distinct close events, and the registry / sibling-doc state differs
> between *committed at HEAD `6bd60c80`* and *the working tree at this regen*:
>
> - **The G-E close-pass (`da205bff`) is committed inside this window.** At HEAD `6bd60c80`
>   the registry therefore already reflects the G-E close: `DC-LIVEMEM-01` is `enforced`, and
>   the G-E gate `ci_check_live_feed_memory_bounds.sh` is committed. This is the close-pass
>   the **previous** HEAD_DELTAS window flagged as owed at its `6f848825` HEAD.
> - **The G-D close-pass is NOT yet committed.** At HEAD `6bd60c80` the new rule
>   `CN-REHEARSAL-FIDELITY-01` is committed as `status = "declared"` with `tests = []` and
>   `ci_script = ""` — but the **flip to `enforced`** and the binding of its `tests` +
>   `ci_script` arrays live only in the **working tree** at this regen (where the rule is
>   already `enforced`, `tests = [node_accepted_block_consensus_inputs_via_shared_import,
>   rehearsal_envelope_wraps_correlate_produced_payload,
>   rehearsal_correlate_no_evidence_writes_nothing,
>   rehearsal_envelope_is_structurally_distinct_from_ba02_manifest,
>   c1_dry_run_correlate_to_rehearsal_envelope, node_c1_dry_run_rehearsal_live,
>   rehearsal_gate_fails_on_archived_home_leak]`, `ci_script = "ci/ci_check_node_path_fidelity.sh,
>   ci/ci_check_rehearsal_manifest_schema.sh"`). The pending close-pass commits the flip and
>   the array bindings — exactly as the N-F-G-E close-pass committed the `DC-LIVEMEM-01` flip
>   after its slice span.
> - **Sibling-doc split (partial — not uniform).** The **registry** and **CODEMAP** working
>   trees have been regenerated to the **G-D** close state (registry: 315 rules,
>   `CN-REHEARSAL-FIDELITY-01` `enforced`; CODEMAP header: "456 canonical types, **2241**
>   tests, **121** CI checks at HEAD (`6bd60c80`, PHASE4-N-F-G-D cluster close)", with the
>   full G-D delta block and **32** `rehearsal_*` mentions). But **SEAMS and TRACEABILITY
>   are still the G-E close docs** — they have **not** been regenerated for G-D: TRACEABILITY
>   still reads "**314 rules** at this working tree" referencing `6f848825` / `DC-LIVEMEM-01`,
>   and neither SEAMS nor TRACEABILITY mentions `CN-REHEARSAL-FIDELITY-01`, the two new G-D
>   gates, or G-D at all. The **committed** CODEMAP / SEAMS / TRACEABILITY at HEAD `6bd60c80`
>   are all still the **G-E close** docs (119 CI checks, 2234 tests, 314 rules, no G-D delta).
>   `docs/ade-HEAD_DELTAS.md`, the **SEAMS/TRACEABILITY G-D regen**, and the `.idd-config.json`
>   baseline bump are what this regen / the close-pass still owe; the `.idd-config.json`
>   baseline has **already** been bumped to `6bd60c80` (the closer's edit), so `/head-deltas`
>   for this window uses the **explicit** `6f848825..6bd60c80` span, not the config value.
>
> **Not a rule removal, not a discipline violation** — a sequencing artifact reconciled by
> the close-pass.

## 1. Commit Log (newest first)

| Hash | Type | Summary |
|------|------|---------|
| `6bd60c80` | fix | scan archived bounty home in rehearsal leak gate (PHASE4-N-F-G-D S4) |
| `a8003dc8` | docs | specify PHASE4-N-F-G-D S4 rehearsal leak gate hardening |
| `076a5af5` | feat | C1 private-testnet dry-run runbook + operator scaffold (PHASE4-N-F-G-D S3) |
| `93472991` | docs | specify PHASE4-N-F-G-D S3 C1 dry-run scaffold |
| `459cf78d` | feat | add non-promotable rehearsal evidence surface (PHASE4-N-F-G-D S2) |
| `e3382532` | docs | specify PHASE4-N-F-G-D S2 rehearsal evidence |
| `d4d0f456` | feat | prove --mode node accepted-block path fidelity (PHASE4-N-F-G-D S1) |
| `c01ff046` | docs | specify PHASE4-N-F-G-D S1 path fidelity |
| `ce313927` | docs | define PHASE4-N-F-G-D bounty dry-run cluster |
| `bc39d431` | docs | plan PHASE4-N-F-G-D bounty dry-run fidelity |
| `07e6a340` | docs | declare PHASE4-N-F-G-D bounty dry-run fidelity |
| `da205bff` | (close) | Close PHASE4-N-F-G-E — peer-driven live-feed memory bounded before authoritative decode/apply |

No merge commits in the span.

The **last-listed** commit (`da205bff`) is the **PHASE4-N-F-G-E close-pass** that lands inside
this window because the baseline `6f848825` is the N-F-G-E *slice-span* HEAD, not its close
commit: `da205bff` flipped `DC-LIVEMEM-01` `declared → enforced`, regenerated the four
grounding docs to the **G-E** close, bumped the `.idd-config.json` baseline `90791691 →
6f848825`, and archived the G-E cluster. It is **docs/registry only** — no source change — and
**count-neutral on CI** (119 → 119; the G-E gate `live_feed_memory_bounds` was already committed
at the baseline `6f848825`, at G-E S1, so the close added no gate). The G-D cluster is then the
**three declare/plan/define docs** (`07e6a340` / `bc39d431` / `ce313927`) followed by **four
doc-then-impl slice pairs** (S1 `c01ff046`+`d4d0f456`, S2 `e3382532`+`459cf78d`, S3
`93472991`+`076a5af5`, S4 `a8003dc8`+`6bd60c80`).

(Plus the pending G-D close-pass commit: the `CN-REHEARSAL-FIDELITY-01` `declared → enforced`
flip + `tests`/`ci_script` array binding, the grounding-doc commit of the working-tree
CODEMAP/registry G-D regen **and** the still-owed SEAMS/TRACEABILITY G-D regen, the
`.idd-config.json` baseline bump confirmation (`6f848825 → 6bd60c80`, already applied) + the
registry-count comment, the G-D cluster-doc archive, and this HEAD_DELTAS.)

## 2. New Modules

Two new source modules, both in `ade_node` (added in **PHASE4-N-F-G-D S2**, `459cf78d`). Both
are confirmed **absent at the baseline** `6f848825` and registered by the two `pub mod` lines
added to `crates/ade_node/src/lib.rs` in the same slice.

| Module | Color | Purpose | Key sub-paths | Added in (cluster/slice) |
|--------|-------|---------|---------------|--------------------------|
| `ade_node::rehearsal_evidence` | **GREEN-by-content** (the file carries the deterministic Core-Contract banner; pure types + serializer, no I/O / clock / rand / float / `HashMap`; `ade_node` is neither a BLUE `core_paths` crate nor RED at crate level — this module is deterministic GREEN evidence by content) | The non-promotable rehearsal **envelope**: `PrivateRehearsalManifest` **wraps** a correlate-produced `Ba02Manifest` payload; closed 1-variant `RehearsalVenue::PrivateTestnetC1`; `RehearsalEnvelope` operator metadata (venue + peer-log filename + sha256). | `crates/ade_node/src/rehearsal_evidence.rs` — `PrivateRehearsalManifest` (`+ ba02 / venue / peer_log_file / peer_log_file_sha256` fields); **sole ctor** `from_correlate_outcome` (returns `None` on `BA02Outcome::NoEvidence` — nothing to wrap); `to_canonical_toml` (pure; emits `is_rehearsal = true` + `not_bounty_evidence = true` as **literals** — the type cannot represent a non-rehearsal); `REHEARSAL_MANIFEST_SCHEMA_VERSION = 1`; `toml_escape` helper. | `PHASE4-N-F-G-D` S2 (`459cf78d`) |
| `ade_node::rehearsal_pass` | **RED** (file I/O; `//! RED` banner; `fs::write` + `io::Result`) | Rehearsal-evidence **I/O**: reads the operator-captured peer-accept JSONL log, runs it through the GREEN correlate **verbatim**, and writes only a `PrivateRehearsalManifest`. Constructs no evidence, synthesizes no acceptance, uses no alternate correlator. | `crates/ade_node/src/rehearsal_pass.rs` — `correlate_peer_log_file_into_rehearsal` (**reuses** `ba02_pass::correlate_peer_log_file`; `Ok(None)` iff `NoEvidence`; missing/unreadable file → `io::Error`, fail closed); `write_private_rehearsal_manifest` (the **argument type is the gate** — only a `PrivateRehearsalManifest` is writable). Hosts the 3 S2 `#[cfg(test)]` proofs. | `PHASE4-N-F-G-D` S2 (`459cf78d`) |

> **Cross-reference warning (CODEMAP):** both modules appear in the **working-tree** CODEMAP
> (32 combined `rehearsal_evidence` / `rehearsal_pass` mentions, in the G-D delta block) — but
> **not** in the CODEMAP **committed at HEAD `6bd60c80`** (still the G-E close doc, 0 mentions).
> The G-D close-pass commits the working-tree CODEMAP regen. See the generation notes.

No new crate, no new workspace, no new BLUE authority, no new WAL/checkpoint/canonical type,
no new `CoordinatorEvent` variant was added. The cluster's only other source change is the
`lib.rs` `pub mod` wiring; S1/S3/S4 add **tests and CI**, not modules.

## 3. Modules Modified

The only modified source file in the window is `crates/ade_node/src/lib.rs` (the module
wiring); all G-D behavior arrives as **new** files (§2) plus tests and CI (§5). There are no
trivial/skipped source changes in this window.

| Module | Color | Scope | Key changes |
|--------|-------|-------|-------------|
| `ade_node` (crate root `lib.rs`) | neither BLUE nor RED at crate level (RED binary/library home) | +2 lines | **G-D S2 (`459cf78d`) — module registration.** Adds `pub mod rehearsal_evidence;` and `pub mod rehearsal_pass;`. No other behavioral change to `lib.rs`. |

The G-A…G-E forge / serve / live-feed / containment source surfaces are **unchanged** in this
window. In particular, no BLUE `ade_network` submodule, no `ade_runtime` shell file, and no
`ade_core` / `ade_ledger` / `ade_codec` / `ade_types` / `ade_crypto` / `ade_plutus` file
appears in `git diff --name-only 6f848825..6bd60c80` — every BLUE count is byte-unchanged.

## 4. Feature Flags

**No project feature-flag deltas.** Ade declares no `[features]` table in any workspace
`Cargo.toml` (confirmed absent at both refs), and no `#[cfg(feature = …)]` gate was introduced
in the span. **No `Cargo.toml` change at all in this window.** No coupling, no `compile_error!`
guard. (The C1 dry-run operator harness is gated by an **environment variable**
`ADE_LIVE_C1_DRY_RUN`, **not** a Cargo feature — it is a `#[test]` skipped in CI, not a
compile-time flag and not a runtime node mode.)

## 5. CI Checks (119 → 121; +2 new, 0 modified, 0 removed)

Two new gates, repo-root-relative and mirroring the existing `ci/ci_check_*.sh` convention.
The only files in `git diff --diff-filter=A 6f848825..6bd60c80 -- ci/` are these two; no `ci/`
file was modified or removed.

### PHASE4-N-F-G-D gates

| Check | Status | Cluster origin | What it checks |
|-------|--------|----------------|----------------|
| `ci_check_node_path_fidelity.sh` | **New** | G-D S1 (`d4d0f456`) | Backs **`CN-REHEARSAL-FIDELITY-01` clause 1 (path fidelity)**. Guard (a): the `cli.rs` argv flag-literal set **equals** a pinned **28-flag closed allow-list** — G-D adds **no** flag, and a private-only / venue flag (`--private-net`, `--from-genesis`, `--devnet`, `--rehearsal`, …) would change the set and trip. Guard (b): **no** from-genesis consensus-inputs constructor exists (a fn whose name carries **both** `genesis` and `consensus`; line comments stripped first so prose naming the forbidden construct cannot self-trip), **and** `node_lifecycle.rs` sources consensus inputs only via the shared `import_live_consensus_inputs` (the same authority the preprod pass uses). Both fail-closed-smoke-verified against an injected `--private-net` flag + a `build_consensus_inputs_from_genesis` ctor. Hermetic. |
| `ci_check_rehearsal_manifest_schema.sh` | **New** (S2; hardened in S4) | G-D S2 (`459cf78d`), S4 fix (`6bd60c80`) | Backs **`CN-REHEARSAL-FIDELITY-01` clause 2 (evidence non-promotability)**. **Vacuous-until-committed**: when no `docs/evidence/phase4-n-f-g-d-private-rehearsal-*.toml` manifest is present (the typical state — the C1 dry-run is operator-gated, only the README is committed) the gate passes. When one is present it verifies the closed **12-field** schema, `schema_version == 1`, the `is_rehearsal = true` + `not_bounty_evidence = true` markers, a `venue = "private-testnet…"` venue, and that `peer_log_file_sha256` **matches** the committed peer-log file (the no-synthetic binding). **Three** non-promotability barriers: (1) distinct `docs/evidence/` home; (2) the rehearsal markers; (3) a cross-check that **no** rehearsal marker (`^is_rehearsal =` / `^not_bounty_evidence =`) appears in any `.toml` under a bounty home. **S4 hardening:** barrier (3) now scans **all** real bounty homes — `docs/clusters/PHASE4-N-F-G-C/` (active) **and** `docs/clusters/completed/PHASE4-N-F-G-C/` (archived) — by building the **existing-homes** list first (no `[[ -d ]]` whole-check skip): *home absent* ⇒ empty contribution (deliberate); a scan error on an **existing** home (`grep` rc ≥ 2) ⇒ **fail closed**, not swallowed. Hermetic. |

The three containment / handoff / memory fences are **byte-unchanged** in this window:
`ci_check_node_run_loop_containment.sh` (relay-loop containment), `ci_check_served_chain_handoff_fence.sh`
(self-accept→serve handoff), and `ci_check_live_feed_memory_bounds.sh` (G-E live-feed memory
bounds) are all untouched. G-D **added** two gates and relaxed **nothing**.

### CI gate lineage (for cross-window readers — explains the headline +2)

The on-disk gate count rose **117 → 118 → 119 → 121** across the G-C…G-D sub-clusters:

- **117 → 118** — G-C **S2** added `ci_check_ba02_evidence_manifest_schema.sh` (narrated in the
  N-F-G-C HEAD_DELTAS window).
- **118 → 119** — G-E S1 added `ci_check_live_feed_memory_bounds.sh` (narrated in the **previous**,
  N-F-G-E, HEAD_DELTAS window `90791691 → 6f848825`). Because that gate was committed **at**
  this window's baseline `6f848825`, it is **not** a window-add here; the G-E close-pass
  `da205bff` was docs/registry only and count-neutral (119 → 119).
- **119 → 121** — G-D S1 + S2 added `ci_check_node_path_fidelity.sh` and
  `ci_check_rehearsal_manifest_schema.sh` (this window).

So **this** window's CI delta is the **two** G-D gates: **119 → 121 (+2)**.

> Cross-reference (TRACEABILITY): in the **working-tree registry**, both new gates are cited by
> `CN-REHEARSAL-FIDELITY-01.ci_script`. But **TRACEABILITY has not been regenerated for G-D** —
> neither the committed (`6bd60c80`) nor the working-tree TRACEABILITY mentions
> `ci_check_node_path_fidelity.sh`, `ci_check_rehearsal_manifest_schema.sh`, or
> `CN-REHEARSAL-FIDELITY-01` (both still report the **G-E** close, 314 rules,
> `DC-LIVEMEM-01`). The G-D close-pass owes the TRACEABILITY regen that renders these rows. See
> the warnings in the generation section.

## 6. Canonical Type Registry Delta

**n/a — no separate canonical-type registry is configured** (`canonical_type_registry: null`),
and **no BLUE crate changed**. The 456 BLUE canonical-type total is **unchanged (Δ0)** across
the span (independently re-verified: `git diff --name-only 6f848825..6bd60c80` matches **no**
`core_paths` (BLUE) entry — every changed source file is under `ade_node`: the new RED
`rehearsal_pass.rs`, the new GREEN-by-content `rehearsal_evidence.rs`, the `lib.rs` `pub mod`
wiring, and the three new test files). The working-tree CODEMAP at `6bd60c80` reports 456
canonical. The new `PrivateRehearsalManifest` / `RehearsalVenue` / `RehearsalEnvelope` types
are **GREEN-by-content and not canonical-counted** (they wrap, but do not extend, the existing
`Ba02Manifest`); no new type was added to any BLUE crate. **No new `CoordinatorEvent` variant
or field was introduced.**

## 7. Normative / Invariant Rule Delta (314 → 315)

**One rule ID added, zero removed** (314 → 315). G-D ships exactly one new rule; this window
also commits the **G-E** close-pass `DC-LIVEMEM-01` `declared → enforced` flip (`da205bff`),
which adds **no** new ID.

### G-D new rule (committed in this span, by S2 `459cf78d`; flip to `enforced` owed at close)

| Rule | Tier | Status (committed at HEAD) | What it pins |
|------|------|---------------------------|--------------|
| `CN-REHEARSAL-FIDELITY-01` | `release` (bounty dry-run fidelity; **NOT** BLUE consensus law) | `declared` at `6bd60c80` (`tests = []`, `ci_script = ""`); **flips to `enforced`** at the close-pass (already `enforced` in the working-tree registry at this regen, with all 7 tests + both gates bound) | **Private-testnet accepted-block bounty dry-run fidelity** — two coupled clauses; if either fails the rehearsal becomes misleading. **(1) PATH FIDELITY:** the C1 private dry-run uses the **SAME** `--mode node` accepted-block path as preview/preprod (N-M-C `import_live_consensus_inputs` → forge → self-accept → sibling-serve → block-fetch → peer log → correlate) with **NO** private-only flag, branch, bootstrap authority, or from-genesis constructor; the only differences are operator-controlled **inputs** (private genesis stake) + the evidence **label**; no private-only helper may make the rehearsal pass if the same condition would fail on preview/preprod. **(2) EVIDENCE NON-PROMOTABILITY:** any private-testnet manifest is clearly marked `rehearsal` / `private-testnet`, stored **only** under the rehearsal home (`docs/evidence/phase4-n-f-g-d-private-rehearsal-*.toml`, never the bounty home `docs/clusters/PHASE4-N-F-G-C/CE-G-C-LIVE_*.toml`), sha256-bound to a real Haskell peer log, correlate-produced (`ba02_evidence::correlate` is the **sole** acceptance-evidence constructor), and flips **NO** RO-LIVE rule — C1 rehearsal evidence is **not** bounty evidence. `cross_ref = RO-LIVE-01, RO-LIVE-06, CN-OPERATOR-EVIDENCE-01, DC-NODE-06, DC-EPOCH-03, CN-NODE-02, CN-CINPUT-03, DC-CINPUT-02b`. Working-tree `tests = [node_accepted_block_consensus_inputs_via_shared_import, rehearsal_envelope_wraps_correlate_produced_payload, rehearsal_correlate_no_evidence_writes_nothing, rehearsal_envelope_is_structurally_distinct_from_ba02_manifest, c1_dry_run_correlate_to_rehearsal_envelope, node_c1_dry_run_rehearsal_live, rehearsal_gate_fails_on_archived_home_leak]`; working-tree `ci_script = ci/ci_check_node_path_fidelity.sh, ci/ci_check_rehearsal_manifest_schema.sh`. |

> **Status sequencing (mirrors the prior G-B/G-C/G-E windows).** At the committed HEAD
> `6bd60c80` (the G-D **S4 slice-span** HEAD), `CN-REHEARSAL-FIDELITY-01.status = "declared"`,
> `tests = []`, `ci_script = ""` — the rule **ID** is committed (so the count rises 314 → 315
> at the slice-span HEAD), but the `declared → enforced` flip **and** the `tests` / `ci_script`
> array bindings are the close-pass edit and live only in the **working-tree** registry at this
> regen (where the rule is already `enforced` with all 7 tests + both gates bound). This is the
> exact pattern the N-F-G-E window documented for `DC-LIVEMEM-01` (`declared` at its slice-span
> HEAD, flipped at close).

### No cross-rule strengthenings recorded by G-D (load-bearing)

Unlike the G-C close-pass (four `strengthened_in` tokens), **G-D records NO `strengthened_in`
bump** — verified by grep over the registry at HEAD: `RO-LIVE-01`, `RO-LIVE-06`, and
`CN-OPERATOR-EVIDENCE-01` gain **no** `PHASE4-N-F-G-D` token. This is **deliberate and
load-bearing**: G-D is a dry-run harness and **does not advance the bounty deliverable**, so it
must not strengthen the release-obligation rules. `RO-LIVE-01` stays `partial`
(`blocked_until_operator_stake_available`); `RO-LIVE-06` stays `enforced` (schema-only); the
single bounty deliverable is preview/preprod acceptance, captured separately.

**No rule was removed (expected: 0).** The 314 → 315 delta is a single additive ID
(`CN-REHEARSAL-FIDELITY-01`); the `DC-LIVEMEM-01` flip carried by the G-E close-pass is a
status change on an existing rule, never a removal.

## 8. Honest residual (cluster scope)

**G-D closes a NARROW claim: the private-testnet (C1) accepted-block dry-run is a
PATH-FAITHFUL, NON-PROMOTABLE rehearsal HARNESS (enforced). It is NOT a live-pass, NOT a
bounty / preview / preprod completion claim, and it does NOT enforce that a C1 run has
succeeded.**

- **Harness, not a successful run.** G-D enforces that *if* a C1 dry-run is executed and an
  acceptance manifest committed, it is path-faithful (clause 1) and non-promotable (clause 2).
  It does **not** enforce that any C1 run has happened: `ci_check_rehearsal_manifest_schema.sh`
  is **vacuous until a real operator-produced manifest is committed** (only the README is
  committed under the rehearsal home — no `.toml` manifest), and the env-gated
  `node_c1_dry_run_rehearsal_live` test is **skipped in CI**. The live C1 execution stays
  **`blocked_until_operator_c1_net_executed`**.
- **NO RO-LIVE flip; no bounty/preview/preprod claim.** G-D flips **no** RO-LIVE rule and
  records **no** `strengthened_in` bump on `RO-LIVE-01` / `RO-LIVE-06` /
  `CN-OPERATOR-EVIDENCE-01`. `RO-LIVE-01` stays `partial`; `RO-LIVE-06` stays schema-only /
  `enforced`. **Private C1 acceptance ≠ bounty completion** — preview/preprod acceptance is the
  single bounty deliverable, captured separately.
- **No BLUE change.** 456 BLUE canonical types unchanged (Δ0); the new modules are RED
  (`rehearsal_pass`) + GREEN-by-content (`rehearsal_evidence`), and the only modified source
  file is `lib.rs` (module wiring). No new `CoordinatorEvent` variant or field; the new
  envelope types wrap, never extend, the BLUE-derived `Ba02Manifest`.
- **Fences byte-unchanged.** The three containment / handoff / memory fences
  (`ci_check_node_run_loop_containment.sh`, `ci_check_served_chain_handoff_fence.sh`,
  `ci_check_live_feed_memory_bounds.sh`) are **byte-unchanged**; the G-A…G-E forge / serve /
  live-feed surfaces are unchanged. G-D **added** two gates and relaxed nothing.
- **S4 close-surfaced security fix (record, do not soften).** S4 fixes a per-cluster
  security-review **HIGH**: the S2 rehearsal leak gate's non-promotability barrier (b) ("no
  rehearsal marker may live in a bounty-evidence home") was **dead** after G-C's archival — the
  pre-S4 `[[ -d ]]` guard checked **only** the active home `docs/clusters/PHASE4-N-F-G-C/`,
  which had moved to `docs/clusters/completed/PHASE4-N-F-G-C/`, so the whole leak scan silently
  skipped. The exact pre-S4 smuggle (a rehearsal-marked `.toml` under the archived home) passed
  with exit 0. S4 repoints the gate to scan **both** real homes fail-closed (existing-homes list
  built first; *home absent* ⇒ empty contribution; scan error on an existing home ⇒ fail closed)
  and adds the durable regression test `rehearsal_gate_fails_on_archived_home_leak` (clean tree
  ⇒ green; archived-home leak ⇒ **fails**; Drop-guarded fixture). Verified: the exact pre-S4
  smuggle now fails with exit 1.
- **Carried follow-ups (explicit, deferred by S4's tight scope — do not treat as closed).**
  Three hardenings are deliberately **out of S4's scope** and remain owed:
  1. **The BA-02 bounty gate's same stale active-home glob.**
     `ci_check_ba02_evidence_manifest_schema.sh` still globs **only**
     `find docs/clusters/PHASE4-N-F-G-C -name "CE-G-C-LIVE_*.toml"` — the *identical* bug S4
     fixed in the rehearsal gate, but on G-C's gate. A separate G-C-gate follow-up.
  2. **`toml_escape` control-character hardening.** `rehearsal_evidence::toml_escape` escapes
     only backslash + double-quote; control characters in operator-supplied fields are not yet
     handled. Deferred — it touches the serializer (left byte-unchanged by S4).
  3. **`pub`-field sole-constructor hardening.** `PrivateRehearsalManifest` / `RehearsalEnvelope`
     expose `pub` fields (so the "sole ctor `from_correlate_outcome`" non-promotability
     guarantee is by-convention on the struct literal, not type-enforced against a hand-built
     value). Inherited from G-C, project-wide; deferred.

---

## Generation notes (regen `6f848825 → 6bd60c80`, PHASE4-N-F-G-D)

- **Explicit span, NOT the config baseline.** This regen was run against the **explicit**
  `6f848825..6bd60c80` span. The `.idd-config.json` `head_deltas_baseline` has **already** been
  bumped to `6bd60c80` (the closer's edit for the **next** cluster), so reading it would
  mis-measure this window; the explicit baseline `6f848825` (the PHASE4-N-F-G-E slice-span HEAD)
  is correct for the G-D close. The `_invariant_registry_doc` comment in `.idd-config.json`
  already reads **"315 entries at HEAD"** (the closer's edit).
- Counts are mechanical (git/grep/ls only, no cargo): commit log + `--shortstat` over
  `6f848825..6bd60c80` (**12** commits, no merges / **24** files / **+2928 / -499**); CI gate
  count via `git ls-tree -r --name-only <ref> ci/ | grep -c 'ci_check_.*\.sh'` at each ref
  (**119 → 121**, **+2 new** — `node_path_fidelity` at G-D S1, `rehearsal_manifest_schema` at
  G-D S2; the only files in `git diff --diff-filter=A 6f848825..6bd60c80 -- ci/` are those two;
  `--diff-filter=M` and `--diff-filter=D` over `ci/` are empty — none modified, none removed);
  registry rule count via `grep -c '^id = '` at each ref (**314 → 315**, the single new ID
  `CN-REHEARSAL-FIDELITY-01`; `diff` of sorted `^id =` lines shows exactly one `>` add and zero
  `<` removals); workspace test attributes via `git grep -hE '#\[(tokio::)?test\]'` over
  `crates/**/*.rs` (**2234 → 2241**, +7 — 1 in `node_path_fidelity.rs`, 3 in `rehearsal_pass.rs`,
  2 in `node_c1_dry_run_rehearsal.rs`, 1 in `rehearsal_gate_archived_home.rs`); BLUE canonical
  types unchanged at 456 (no BLUE `core_paths` file in the diff — every changed source file is
  under `ade_node`).
- **Two close events in one window (NOT an anomaly).** The **G-E** close-pass (`da205bff`) is
  **committed** inside this window (because the baseline `6f848825` is the G-E slice-span HEAD,
  not its close commit) — it flipped `DC-LIVEMEM-01` to `enforced`, refreshed the four grounding
  docs to the **G-E** close, and bumped the baseline `90791691 → 6f848825`. The **G-D** close-pass
  is **not yet committed**. So at HEAD `6bd60c80`: the registry has `DC-LIVEMEM-01` `enforced`
  and `CN-REHEARSAL-FIDELITY-01` as `declared` (the `enforced` flip + array binding are
  working-tree only); and the **committed** CODEMAP / SEAMS / TRACEABILITY are still the **G-E**
  close docs (119 CI, 2234 tests, 314 rules, no `CN-REHEARSAL-FIDELITY-01`).
- **Sibling-doc coherence is PARTIAL at this regen (load-bearing).** The **registry** and
  **CODEMAP** **working trees** have been regenerated to the **G-D** close state — registry: 315
  rules, `CN-REHEARSAL-FIDELITY-01` `enforced` with `tests`/`ci_script` bound; CODEMAP header:
  "456 canonical types, 2241 tests, 121 CI checks at HEAD (`6bd60c80`, PHASE4-N-F-G-D cluster
  close)", with the full G-D delta block and 32 `rehearsal_*` mentions — and they **agree** with
  this doc's counts. But **SEAMS and TRACEABILITY have NOT been regenerated for G-D**:
  TRACEABILITY still reads "314 rules at this working tree" referencing `6f848825` /
  `DC-LIVEMEM-01`, and neither SEAMS nor TRACEABILITY mentions `CN-REHEARSAL-FIDELITY-01`, the
  two new G-D gates, or G-D. `git status` at this regen shows only `.idd-config.json`,
  `docs/ade-CODEMAP.md`, and `docs/ade-invariant-registry.toml` dirty (the SEAMS/TRACEABILITY
  G-D regen is still owed by the close-pass).
- **CI cross-reference warning.** Both new G-D gates are cited by `CN-REHEARSAL-FIDELITY-01`
  only in the **working-tree registry** — **TRACEABILITY does not yet cite them** (it is still
  the G-E close doc, in both the committed and working-tree copies). The close-pass commits the
  `ci_script` binding's enforced state and regenerates TRACEABILITY so these two gates render
  against `CN-REHEARSAL-FIDELITY-01`. (The G-E gate `live_feed_memory_bounds` and the G-C BA-02
  gate are already cited in the committed TRACEABILITY, via `da205bff` / earlier.)
- **Not a rule removal, not a discipline violation** — the working-tree-vs-committed split (and
  the partial sibling-doc regen) is a sequencing artifact reconciled by the close-pass, which
  commits the `CN-REHEARSAL-FIDELITY-01` `declared → enforced` flip + `tests`/`ci_script` array
  binding, the CODEMAP/registry working-tree regen, the **owed** SEAMS/TRACEABILITY G-D regen,
  the `.idd-config.json` baseline bump confirmation (`6f848825 → 6bd60c80`, already applied) +
  the registry-count comment, the G-D cluster-doc archive, and this HEAD_DELTAS. The next gating
  work is the **operator-witnessed live pass** — the C1 dry-run execution
  (`blocked_until_operator_c1_net_executed`) and, for the bounty deliverable, the separate
  preview/preprod acceptance pass — neither advanced by G-D, which only ships the path-faithful
  rehearsal harness ahead of them.
