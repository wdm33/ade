# Seams — Where New Work Can Attach (Ade)

> **Status:** Living architectural document. Regenerated; not hand-edited.
> Per-project instance of `~/.claude/methodology/templates/seams.md`.

> 11 crates, **464 canonical types**, **190 CI checks**, **378 registry rules** at HEAD (`233644f7`, MEM-OPT-OPS cluster-close refresh). This regeneration folds the `862cd2cb..233644f7` span on top of the prior PHASE4-N-AO SEAMS. The active cluster — **MEM-OPT-OPS** (`DC-MEM-06` / `OP-MEM-02`) — is **RED+GREEN ONLY**: the BLUE consensus + ledger core is reused byte-unchanged (`ade_core` 49→49, `ade_ledger` 181→181; `git diff 388d8073..HEAD` over the BLUE trees is empty for the MEM-OPT-OPS commits). It adds **NO new module/crate, NO new CLI flag, and NO new BLUE canonical type** — every MEM-OPT-OPS seam below is a **field / enum / error-surface addition behind an EXISTING closed vocabulary**.
>
> ### What MEM-OPT-OPS added to the seam surface (the load-bearing summary)
>
> **No new ingress surface, no new openly-extensible / plugin / negotiated / runtime-registered registry, and no second authority of any kind.** The three slices add memory profile around the unchanged authority path:
>
> - **S1 — a process-wide `#[global_allocator]` (`mimalloc`) at the RED binary entry** (`crates/ade_node/src/main.rs`). A build-time, **determinism-neutral** seam: the allocator type is INVISIBLE to BLUE (allocation addresses/sizes never enter a fingerprint or replay output), gated by `ci_check_alloc_determinism_neutral.sh`. The `mimalloc` dep lives on the `ade_node` binary crate ONLY (no BLUE/GREEN crate takes it). `VmRSS −29.8 %` on preprod.
> - **S2 — a STREAMING seed import** (`crates/ade_runtime/src/seed_import/importer.rs`). `import_cardano_cli_json_utxo` is now a `CanonicalUtxoSink` serde `Visitor` over `serde_json::Deserializer::from_reader(BufReader<File>)` — it never materializes the whole-file buffer (import peak halved). It is **byte-identical** to the retained whole-buffer oracle `import_cardano_cli_json_utxo_from_bytes` on identical-by-canonical-key inputs. The closed `JsonSeedError` error surface gained a **`DuplicateTxIn { key }`** variant: ANY canonical-`TxIn` duplicate is **fail-closed**, never an order-dependent survivor (the whole-buffer oracle is now test-only). See §1 (operator file ingress) + §3 closed.
> - **S3 — owned-footprint samplers + honest cross-node comparison** threaded through the **closed** `AdmissionLogEvent::{MemoryMeasure, MemorySummary}` evidence variants — gross fields (`rss_kib` / `rss_hwm_kib`) joined by OWNED fields (`rss_anon_kib` / `private_dirty_kib` per-point + `owned_rss_anon_{p50,peak}_kib` / `owned_private_dirty_{p50,peak}_kib` summary percentiles). The closed measurement-`point` set gained **`seed_import`** (allow-listed in `ci_check_mem_measure_evidence.sh`). **Observe-only — no `*_kib` field gates any authority.** Honest verdict recorded `ade_heavier`; `OP-MEM-02` stays `declared`.
> - **The mem-measure closed enums** `BoundedOutcome` / `ShedReason` (`mem_measure::bounded_admission`), `ReplayVerdict` / `EvidenceDefect` (`mem_measure::evidence`) — the MEM-MEASURE substrate MEM-OPT-OPS builds on (carried, all in the RED `ade_node` crate, not canonical-counted).
>
> **Continuity from the broader stale span (separate from MEM-OPT-OPS, the +2 BLUE canonical-type delta lives here — the pre-preprod local streams):** `ade_network::codec::tx_submission::TxSubmissionTxId` + the tx-submission2 message set (`DC-PROTO-11`, `enforced`, real-capture-locked — the last N2N mini-protocol surface) and `ade_plutus::tx_eval::RedeemerFields` (Stream-1 per-script ex_units cap, `CN-PLUTUS-01/04`). These flipped a set of BLUE/network rules to `enforced`; they are reflected in §3-closed and the §5 counts.
>
> Every MEM-OPT-OPS surface is CLOSED / additive (closed discriminants, deterministic ordering, fail-closed). The PHASE4-N-AO live multi-candidate fork-choice SELECT surface (the prior active cluster) is **carried verbatim** — its modules live unchanged in the codebase per the CODEMAP. `select_best_chain` is byte-unchanged; `RO-LIVE-01` stays operator-gated.
>
> ### Counts (mechanical, with sources)
>
> | Count | Value | Source |
> |---|---|---|
> | Crates | **11** | `grep -cE '"crates/' Cargo.toml`. **Δ vs the prior SEAMS (11): 0** — MEM-MEASURE + MEM-OPT-OPS add five modules INSIDE the existing `ade_node` crate + one capture bin in `ade_network`; no `[workspace] members` entry added. |
> | Canonical types | **464** | structural grep over the 6 BLUE crate `src/` trees + the 9 BLUE `ade_network` submodule paths at `233644f7`. **Δ vs the prior SEAMS (462): +2 — NOT from MEM-OPT-OPS.** `ade_plutus` 8→9 (`tx_eval::RedeemerFields`, Stream-1 `ed408410`) + `ade_network::codec` 39→40 (`tx_submission::TxSubmissionTxId`, Stream-2 `92b855c4`). The mem_measure module types (`BoundedOutcome`, `ShedReason`, `ReplayVerdict`, `EvidenceDefect`, `MemEvidenceRecord`, `RssSampleKib`, `RssWindow`, …) live in the RED `ade_node` crate and are NOT canonical-counted. |
> | CI checks | **190** | `ls ci/ci_check_*.sh \| wc -l` at `233644f7`. **Δ vs the prior SEAMS (173): +17** — verified `git diff --name-status 388d8073..HEAD -- 'ci/ci_check_*.sh'`: **17 `A`** (7 MEM-MEASURE/MEM-OPT-OPS: `ci_check_alloc_determinism_neutral.sh`, `ci_check_bounded_inbound_admission.sh`, `ci_check_mem_compare_evidence.sh`, `ci_check_mem_measure_evidence.sh`, `ci_check_mem_opt_s1_reduction.sh`, `ci_check_mem_opt_s2_import_peak.sh`, `ci_check_mem_opt_s3_owned.sh`; + 10 Stream-1/2/3), **2 `M`** (`ci_check_admission_log_vocabulary_closed.sh`, `ci_check_convergence_evidence_vocabulary_closed.sh` — both extended for `memory_measure` / `memory_summary`), **0 `D`**. |
> | Registry rules | **378** | `grep -cE '^id = ' docs/ade-invariant-registry.toml`. **Status:** 250 `enforced` / 22 `partial` / 105 `declared` / 1 `enforced_scaffolding` / 0 `deprecated`. **Δ vs the prior SEAMS (372): +6** — `DC-MEM-05` (`declared`), `DC-MEM-06` (`partial`), `DC-MEM-07` (`declared`), `DC-MEM-08` (`declared`), `OP-MEM-02` (`declared`), `DC-PROTO-11` (`enforced`). **Flips this span (Streams 1/2/3):** `DC-PROTO-02`, `DC-PROTO-01/03/04/06`, `CN-CONS-04`, `CN-WIRE-07`, `CN-PLUTUS-01/04`, `DC-CORE-01` → `enforced`. **No rule weakened.** |
>
> ### CODEMAP cross-reference (read honestly — load-bearing)
>
> This SEAMS reads the CODEMAP (`docs/ade-CODEMAP.md`, **at the SAME HEAD `233644f7`** — the MEM-OPT-OPS cluster-close refresh) for the module list + TCB colors, and `docs/ade-invariant-registry.toml` (378 rules) for the canonical count source. The CODEMAP and SEAMS are now in lockstep at `233644f7` (the prior SEAMS at `862cd2cb` was the stale party — CODEMAP led this refresh). The CODEMAP's "Closed enums / registries (for SEAMS cross-reference)" §, the per-module BLUE/GREEN/RED tables, and the cross-module rules are the upstream source for every row below.
>
> **TCB-color note (per CODEMAP at `233644f7`):** MEM-OPT-OPS touched ONLY RED/GREEN. `main.rs`'s `#[global_allocator]` is **RED** (binary entry). `seed_import::importer` is **GREEN-by-content** inside the RED `ade_runtime` (rewritten streaming, same canonical output; `JsonSeedError` is its closed error surface). `mem_measure::{bounded_admission, evidence}` are **GREEN** (the bounded fold + the evidence record/validator/replay-pairing — promotable to BLUE, never demotable); `mem_measure::rss_sampler` (the `/proc` reader) is **RED**. `admission_log::event` + `convergence_evidence` (the `MemoryMeasure`/`MemorySummary` taps) are **GREEN observe-only**. The PHASE4-N-AO SELECT modules (`fork_switch` / `lca_walk` / `selector_state` / `fair_merge` / `candidate_aggregator` / `post_switch_continuity`) are carried verbatim — every one lives in the already-RED `ade_node` host crate, and `select_best_chain` stays the already-BLUE sole selector.

This document describes **the closure surface of the system** — where new work can attach safely, where it cannot, and what shape attachments must take. It is the architectural complement to CODEMAP: CODEMAP says what each module *is*, SEAMS says where the system *opens*.

---

## 1. Surface Reduction Rules

> External inputs reduce to canonical form before entering authoritative pipelines. Ade's external surfaces are the N2N/N2C wire, operator files, and `argv`. Each reduces to a canonical type before any BLUE authority sees it. **MEM-OPT-OPS added NO new ingress surface** — it changed the operator-file UTxO-dump reduction MECHANISM (whole-buffer → streaming) without changing its canonical OUTPUT, and tightened it fail-closed (`DuplicateTxIn`).

**Rule:** New ingress surfaces attach by producing the canonical type's bytes and entering the same authoritative pipeline. They **may not** introduce new pipeline steps, reorder existing steps, or shortcut into the core via a back door.

### Surface: N2N inbound wire (received blocks/headers/txs/rollbacks — the LIVE `--mode node` feed)

```
Surface: N2N inbound wire (TCP + mux + handshake; ChainSync RollForward / RollBackward + BlockFetch + the tx-submission2 grammar)
Reduces to: AdmissionPeerEvent (RED, ade_runtime::admission::wire_pump) → NodeSyncItem { Block { peer, bytes } | RollBack(Point) } (RED, ade_node::node_sync; PEER-TAGGED since N-AO S1 / DC-NODE-34) → BLUE DecodedBlock (ade_codec) → BLUE BlockVerdict (ade_ledger::block_validity)
Pipeline (fixed; steps cannot be reordered):
  1. mux frame decode                          (BLUE ade_network::mux::frame — single authority)
  2. session reassembly / segmentation         (GREEN ade_network::session — one DeliverPeerFrame per complete CBOR item)
  3. tag-24 unwrap                              (BLUE ade_codec::unwrap_tag24 — the SOLE tag-24 authority; CN-WIRE-12)
  4. AdmissionPeerEvent emission                (RED wire_pump — Block / TipUpdate / RollBackward {peer, point, tip} / Disconnected; the keep-alive client emits NONE — DC-PUMP-01/03)
  5. fair per-peer merge                        (GREEN ade_node::fair_merge — deterministic round-robin; N-AO S8 / DC-PUMP-04)
  6. peer-tagged NodeSyncItem                   (RED node_sync — N-AO S1 / DC-NODE-34 threads the origin peer through)
  7. classify_receive → resolve_disposition     (GREEN-by-fn — Admit | NeedsForkChoice | RollbackFollow; DC-NODE-23/24)
  8a. (Admit)          bounded inbound fold → BLUE decode + block_validity → pump_block durable admit  (DC-MEM-07 fold fronts mempool_ingress; DC-NODE-05/12)
  8b. (NeedsForkChoice) per-peer candidate aggregation → select_best_chain → S4 prove-then-commit (N-AO; §2)
  8c. (RollbackFollow) materialize_rolled_back_state (eta0-overlaid) + commit_rollback (DC-NODE-25/26/29, T-REC-06)
Cross-surface state sharing: the per-peer wire-pump lanes share one deterministic fair-merge cursor (N-AO S8); the rollback target is bound to the durable ChainDb stored slot+hash (DC-NODE-29), never peer-supplied.
```

**MEM-MEASURE note (step 8a):** the GREEN `mem_measure::bounded_admission::replay_bounded_ingress_trace` fold gates inbound bytes BEFORE the BLUE `mempool_ingress`, within the closed non-configurable `MAX_INBOUND_ADMISSION_{BYTES,COUNT}` (`DC-MEM-07`) — memory pressure cannot grow the inbound working set unboundedly, and the bound never changes an authoritative output. A shed event (`BoundedOutcome::Shed(ShedReason)`) is NEVER an acceptance. **Stream-2 note (step 1–3):** the tx-submission2 grammar (`codec::tx_submission`, `DC-PROTO-11`) is now real-capture-locked — the last N2N mini-protocol surface.

### Surface: argv (closed mode set)

```
Surface: argv
Reduces to: Cli / Mode (closed: WireOnly | Admission | KeyGenKes | Produce | Node) — ade_node::cli
Pipeline: parse → closed Mode dispatch in main() → per-mode driver
Cross-surface state sharing: none (a CLI flag set is a CLOSED allow-list, mirrored by ci_check_node_path_fidelity.sh — MEM-OPT-OPS added NO new --mode node flag)
```

### Surface: operator file ingress (KES skey / opcert / Shelley genesis / UTxO seed dump / recovered-anchor sidecar)

```
Surface: operator files
Reduces to: KesSecret / OperationalCert / ConwayGenesisConfig / canonical seed entries (UTxOState + UtxoFingerprint) / SeedEpochConsensusInputs / RecoveredAnchorPoint — via the single RED parsers in ade_runtime (each fail-closed)
Pipeline: read bytes → RED parser → canonical BLUE type → bootstrap_initial_state (the single lifecycle owner; CN-NODE-01)
Cross-surface state sharing: the recovered seed-epoch eta0 sidecar is read once by bootstrap and overlaid onto BOTH the WarmStart and rollback-materialize chain_dep (T-REC-04 / T-REC-06)
```

**MEM-OPT-OPS S2 note (the UTxO seed dump reduction):** `seed_import::import_cardano_cli_json_utxo` is now **STREAMING** — a `CanonicalUtxoSink` serde `Visitor` over `serde_json::Deserializer::from_reader(BufReader<File>)` converts each cardano-cli `query utxo` JSON entry to canonical form and inserts into the `BTreeMap<TxIn, TxOut>` AS IT IS PARSED, so neither the whole-file buffer nor an intermediate `RawUtxoMap` is materialized (import peak halved, ~6.8 GB removed). The canonical OUTPUT `(UTxOState, UtxoFingerprint)` is **byte-identical** to the retained whole-buffer oracle `import_cardano_cli_json_utxo_from_bytes` on identical-by-canonical-key inputs (the SAME `parse_txin_key` + `build_canonical_tx_out` + the SAME canonical `BTreeMap`; the fingerprint is over canonical `TxIn` keys, allocator/parse-order independent — `DC-MEM-06`). **The streaming path is fail-closed on duplicates:** distinct JSON key strings can collide on one canonical `TxIn` (uppercase/lowercase hex; `#0` vs `#00`), so ANY duplicate yields `JsonSeedError::DuplicateTxIn { key }` — never an order-dependent survivor. The whole-buffer `_from_bytes` is now the **equivalence oracle + in-memory test helper only**, not a second production authority. **This is a reduction-MECHANISM change with an unchanged reduction TARGET** — it adds no pipeline step and no new ingress.

> Full per-surface detail (BA-02 operator-pass evidence, Mithril provenance binding, the forge-constant/operator-key/run-loop surfaces, the seed-epoch sidecar warm-start) is carried in the §2 domain tables and the §3 registries. MEM-OPT-OPS touched only the UTxO-dump reduction mechanism above; N-AO touched none of these ingress surfaces.

---

## 2. Data-Only vs. Authoritative Layers

> For every domain where a tooling/transport layer and an authoritative layer coexist, the boundary is named. **The compilation/enforcement chokepoint never moves.**

### Domain: memory measurement + optimization — RED/GREEN observational vs. BLUE authority untouched (MEM-OPT-OPS; the active cluster's defining domain)

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **Authoritative core (UNTOUCHED)** | `ade_ledger::mempool::ingress::mempool_ingress` / `block_validity` / `select_best_chain` / the UTxO canonical fingerprint authority | **BLUE** | **Reused byte-unchanged.** MEM-OPT-OPS changes the runtime memory profile AROUND these; it changes no authoritative output. `ade_core` 49→49, `ade_ledger` 181→181. |
| **Process allocator (build-time seam)** | `ade_node::main` `#[global_allocator] mimalloc::MiMalloc` | **RED** | The single process-wide allocator declaration. **Determinism-neutral:** the allocator type is INVISIBLE to BLUE — allocation addresses/sizes never enter a fingerprint or replay output (`ci_check_alloc_determinism_neutral.sh`, `DC-MEM-06`). mimalloc returns freed pages to the OS, so the transient seed-import peak no longer pins RSS. Dep on `ade_node` binary crate ONLY. |
| **Streaming seed-import (data-only conversion)** | `ade_runtime::seed_import::importer::import_cardano_cli_json_utxo` (`CanonicalUtxoSink`) | **GREEN-by-content** | Streams the cardano-cli JSON UTxO dump into the canonical `BTreeMap<TxIn, TxOut>` byte-identically to the whole-buffer oracle; `JsonSeedError::DuplicateTxIn` fail-closed. **Produces canonical bytes; interprets no semantics; the fingerprint authority it feeds is the BLUE `UTxOState` fingerprint, unchanged.** |
| **Equivalence oracle (test-only)** | `ade_runtime::seed_import::importer::import_cardano_cli_json_utxo_from_bytes` | **GREEN-by-content** | The retained whole-buffer path — the equivalence ORACLE + in-memory test helper. NOT a second production authority. |
| **Bounded inbound-admission fold** | `ade_node::mem_measure::bounded_admission::replay_bounded_ingress_trace` (`BoundedOutcome` / `ShedReason` / `MAX_INBOUND_ADMISSION_{BYTES,COUNT}`) | **GREEN** | Deterministic fold that FRONTS the BLUE `mempool_ingress` (`CN-MEM-01` / `DC-MEM-07`): admits within the closed caps, sheds the rest with a closed `ShedReason`. Gates bytes BEFORE `mempool_ingress`, never inside/bypassing it. A shed event is never an acceptance. |
| **Memory-evidence discipline** | `ade_node::mem_measure::evidence` (`MemEvidenceRecord` / `validate_evidence` / `pair_replay` / `ReplayVerdict` / `EvidenceDefect`) | **GREEN** | Pairs a memory measurement (RED RSS observations) with a REPLAY FINGERPRINT; `validate_evidence` returns `EvidenceDefect::VerdictNotAgreed` if the replay verdict is not `Agreed`. **The verdict is computed ONLY from the fingerprint pairing, never from RSS** — a low-memory run that silently changed the authoritative output is INVALID evidence. |
| **RSS / owned-footprint sampler** | `ade_node::mem_measure::rss_sampler` (`sample_vm_rss_kib` / `sample_vm_hwm_kib` / `sample_rss_anon_kib` / `sample_private_dirty_kib`) | **RED** | The single `/proc/self/{status,smaps_rollup}` reader. `RssAnon` (owned anonymous heap, excludes the mmap'd `chain.db` — the cross-node `OP-MEM-02` metric) + `Private_Dirty` (ptrace-protected, self-informational). Fail-soft (`None` off-Linux / ptrace-denied). **Numbers only — never enter a fingerprint, verdict, or validator pass/fail.** |
| **Observe-only evidence sink** | `ade_node::convergence_evidence` + `admission_log::event` (`AdmissionLogEvent::{MemoryMeasure, MemorySummary}`) | **GREEN** | Emits the gross + owned `*_kib` fields alongside the existing convergence/admission evidence. **OBSERVE-ONLY — no `*_kib` field is read by any authority path; RSS magnitude never gates the run's replay verdict (`OP-MEM-02`).** |

**Rule:** New memory / measurement work adds RED sampling + GREEN evidence/folds AROUND the unchanged BLUE core; it routes admission through the EXISTING `mempool_ingress` (behind the bounded fold), produces canonical seed bytes through the EXISTING `seed_import` canonical conversion, and emits `*_kib` numbers as observe-only evidence. **An RSS / owned-footprint magnitude MUST NOT enter a fingerprint, verdict, validator pass/fail, or any authoritative comparison.** A storage / memory-representation / allocator change is NEVER a consensus or replay change (`DC-MEM-05/06`). A measurement whose replay verdict is not `Agreed` is INVALID evidence. The honest comparison may not claim a bound the evidence does not support (S3 recorded `ade_heavier`).

### Domain: live multi-candidate fork-choice SELECT + adopt — RED fetch/driver vs. GREEN sequencing vs. BLUE `select_best_chain` + `block_validity` (N-AO; carried verbatim)

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **Authoritative selector** | `ade_core::consensus::fork_choice::select_best_chain` | **BLUE** | **The SINGLE, sole `DC-CONS-03` fork-choice authority.** k-bounded, density-free, arrival-order-independent (`CN-CONS-01`). N-AO routes a candidate **set** into it; **byte-unchanged** by N-AO AND by MEM-OPT-OPS. No second selector exists. |
| **Authoritative branch proof** | `ade_core::consensus` `block_validity` (via `ade_node::fork_switch::prevalidate_branch`) | **BLUE-reused / GREEN-pure** | `prevalidate_branch` is PURE: it binds each fetched body to its S3-selected `ValidatedHeaderSummary`, parent-links from the durable `ForkAnchor`, and folds BLUE `block_validity`. **A non-`Valid` verdict fails closed HERE, BEFORE the caller's `commit_rollback` (`DC-NODE-37`).** |
| **Authoritative apply** | `ade_ledger::rollback::materialize_rolled_back_state` (+ eta0 overlay) + `receive::reducer` `commit_rollback` + `pump_block` | **BLUE** | The EXISTING, reused adoption authorities. The fork anchor binds Ade's durable stored slot+hash (`DC-NODE-29`). `pump_block` stays the SOLE roll-forward durable admit. |
| **Selector-state projection** | `ade_node::selector_state` (`project_tiebreaker`, `PendingForkSwitch`, `ForkAnchor`) | **GREEN** | Pure projection of S2-validated header summaries into a provisional `PendingForkSwitch` (S3 sets it but applies nothing). |
| **Durable-LCA fork-anchor walk** | `ade_node::lca_walk::walk_to_durable_lca` | **GREEN-by-fn** | Pure read over a `&dyn ChainDb`: walks a competing branch's cached headers to a durable stored ancestor, k-bounded (block depth), cache self-binding-checked. `DC-NODE-38`. |
| **Fork-switch driver** | `ade_node::node_lifecycle::apply_fork_switch` | **RED** | Does the body fetch (`BranchBodySource`), the read-only anchor materialize, calls `prevalidate_branch`, and — only on success — adopts via `apply_chain_event`. |
| **Body fetch seam** | `ade_node::fork_switch::BranchBodySource` (`NullBranchBodySource` / `PrefetchedBranchBodies`) | **RED** | The byte-only fetch abstraction. See §3-extensible. |
| **Per-peer wire-pump fairness** | `ade_node::fair_merge::fair_merge` | **GREEN** | Deterministic round-robin merge of per-peer bounded lanes (`DC-PUMP-04`) — no HashMap / wall-clock / rand; closed-lane retire-in-place. |

**Rule:** New fork-choice work adds candidate-construction / proof / sequencing logic in RED/GREEN `ade_node`; it routes into the EXISTING BLUE `select_best_chain` and adopts via the EXISTING BLUE `materialize` / `commit_rollback` / `pump_block`. **The selector and the adoption chokepoints never move.** No second selector, no parallel preference, no density ordering, no operator heuristic. The current durable chain is NEVER abandoned until the replacement branch is fetched, linked, and validated as a complete candidate branch (`DC-NODE-37`).

### Domain: tx-submission2 transport vs. ledger admission — RED/BLUE codec vs. BLUE mempool (Stream-2; carried + completed this span)

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **Wire grammar (data-only)** | `ade_network::codec::tx_submission` (`TxSubmissionTxId`) + `tx_submission::{transition, event}` | **BLUE** | The SINGLE tx-submission2 codec — era-tagged `TxSubmissionTxId` + definite/indefinite array forms, validated against a real preprod capture corpus (`DC-PROTO-11`). Rejects a bare txid / wrong txid-hash length / unterminated indefinite sequence (fail-closed). Carries bytes; interprets no ledger semantics. |
| **Authoritative admission** | `ade_ledger::mempool::ingress::mempool_ingress` | **BLUE** | The single BLUE wire-ingress chokepoint (behind the MEM-MEASURE bounded fold). Unchanged. |

### Domain: live fork-choice rollback-FOLLOW + rollback-materialization replay-equivalence (N-AI / N-AN; reused by N-AO, unchanged by MEM-OPT-OPS)

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **Detector / resolver** | `ade_node::node_sync` (`classify_receive` / `resolve_disposition`) | GREEN-by-fn | Classifies a competing Participant block as `NeedsForkChoice` (`DC-NODE-23/24`). |
| **Selector + durable apply** | `ade_core::consensus::fork_choice::select_best_chain` + `ade_ledger::rollback::materialize_rolled_back_state` + `commit_rollback` | BLUE | Same single authority (`DC-CONS-03`); bound to the durable stored slot+hash (`DC-NODE-29`); eta0-overlaid (`T-REC-06`). |
| **Single eta0-overlay authority** | `ade_core::consensus::praos_state::PraosChainDepState::overlay_recovered_eta0` | **BLUE** | The SOLE eta0-overlay site — shared by WarmStart bootstrap AND rollback materialize. VRF strength UNCHANGED (a WRONG eta0 still fails closed). N-AO's `prevalidate_branch` materialize path inherits this. |

> The carried domains — N2N tag-24 wire envelope (N-X), block codec, leader-eligibility VRF input (N-W), KES signing-key custody, forged-block serving (data-only serve vs. authoritative admit), network forward-sync (durable-before-tip), crash recovery, bootstrap seed provenance, recovered seed-epoch consensus inputs (N-F-A), recovered-anchor live-follow start (N-AK), the live `--mode node` block feed, BA-02 operator-pass evidence, forge-constant fidelity, the live relay run-loop — are unchanged by MEM-OPT-OPS. Each: the RED tooling/transport layer parses/packs/moves bytes; the BLUE authority enforces; the chokepoint never moves.

---

## 3. Closed vs. Extensible Registries

The system is partitioned into closed (frozen, version-gated) and extensible (open within constraints) registries.

### Closed (frozen — version-gated changes only)

> **MEM-OPT-OPS additions are at the top.** Every MEM-OPT-OPS closed surface is a field/enum/error-surface addition behind an EXISTING closed vocabulary — closed discriminants, deterministic ordering, fail-closed; none is openly extensible.

| Registry | Location | Count | Change Rule |
|----------|----------|-------|-------------|
| `JsonSeedError` (the closed, fail-closed seed-import error surface) *(MEM-OPT-OPS S2 added `DuplicateTxIn`; `DC-MEM-06`)* | `ade_runtime::seed_import::importer` (GREEN-by-content) | **closed sum** | The streaming `import_cardano_cli_json_utxo` rejects ANY canonical-`TxIn` duplicate with `DuplicateTxIn { key }` — distinct JSON key strings colliding on one canonical `TxIn` is fail-closed, never an order-dependent survivor. The other variants are the carried parse/canonicalization failures. New variant = a new fail-closed parse cause + a `DC-MEM-06` / seed-import strengthening (`ci_check_mem_opt_s2_import_peak.sh` + the `streaming_*` equivalence tests). **The whole-buffer `_from_bytes` oracle is test-only.** |
| `AdmissionLogEvent::{MemoryMeasure, MemorySummary}` (the closed memory-evidence variants) *(MEM-OPT-OPS S3; `OP-MEM-02`)* | `ade_node::admission_log::event` (GREEN) + `writer` `DISCRIMINATORS` allow-list (GREEN) | **2 evidence variants** (within the broader closed `AdmissionLogEvent` sum) | `MemoryMeasure { point, slot, …, rss_kib, rss_hwm_kib, rss_anon_kib, private_dirty_kib }` (gross VmRSS/VmHWM + S3 OWNED RssAnon/Private_Dirty) and `MemorySummary { …, rss_p50/p95/peak_kib, rss_hwm_kib, owned_rss_anon_{p50,peak}_kib, owned_private_dirty_{p50,peak}_kib }`. Discriminators `memory_measure` / `memory_summary`. **CLOSED — no open `*_kib` variant; OBSERVE-ONLY — emitting it affects no authority.** The closed measurement-`point` set is `{seed_import, idle_recovered_tip, chain_sync_follow, block_fetch_serve, mempool_admission, wal_checkpoint_recovery, sustained}` — S3 added **`seed_import`** (allow-listed in `ci_check_mem_measure_evidence.sh`). New field/variant/point = a code change + matching allow-list entries in `ci_check_admission_log_vocabulary_closed.sh` + `ci_check_convergence_evidence_vocabulary_closed.sh` + `ci_check_mem_measure_evidence.sh`. |
| `BoundedOutcome` + `ShedReason` (the bounded inbound-admission outcome) *(MEM-MEASURE A1; `DC-MEM-07`)* | `ade_node::mem_measure::bounded_admission` (GREEN) | `BoundedOutcome` 2-variant; `ShedReason` 2-variant | `BoundedOutcome`: `Forwarded(AdmitOutcome)` (passed the bound; carries the unchanged BLUE verdict) \| `Shed(ShedReason)` (dropped BEFORE the authoritative path — never an acceptance). `ShedReason`: `CountBudgetExhausted` \| `ByteBudgetExhausted`. Caps `MAX_INBOUND_ADMISSION_{BYTES,COUNT}` are closed + non-configurable. New variant = a `DC-MEM-07` strengthening (`ci_check_bounded_inbound_admission.sh`). |
| `ReplayVerdict` + `EvidenceDefect` (the memory-evidence verdict/defect vocabulary) *(MEM-MEASURE A1; `OP-MEM-01`)* | `ade_node::mem_measure::evidence` (GREEN) | `ReplayVerdict` 2-variant; `EvidenceDefect` 5-variant | `ReplayVerdict`: `Agreed` (fingerprints byte-identical — valid evidence) \| `Diverged` (the measurement perturbed an authoritative output — INVALID). `EvidenceDefect`: `EmptyScenarioId` / `EmptyWorkloadHash` / `EmptyFinalFingerprint` / `VerdictNotAgreed` / `PercentileShapeViolated`. **The verdict is from the fingerprint pairing, never the RSS.** New variant = an `OP-MEM-01` strengthening (`ci_check_mem_measure_evidence.sh`). |
| `TxSubmissionTxId` + the tx-submission2 message set *(Stream-2; `DC-PROTO-11`, real-capture-locked — the +1 BLUE canonical type on `ade_network::codec`)* | `ade_network::codec::tx_submission` (BLUE) + `tx_submission::{transition, event}` (BLUE) | closed; codec 39→**40** | The SINGLE tx-submission2 codec — the era-tagged txid + definite/indefinite sequence decode + indefinite encode, validated against the real preprod capture corpus. Never a second/hand-rolled tx-submission codec; rejects a bare txid / wrong txid-hash length / unterminated indefinite sequence (fail-closed). The last N2N mini-protocol surface. New form = a versioned codec change + a real-capture corpus extension (`ci_check_tx_submission2_real_capture.sh`). |
| `RedeemerFields` (the per-script ex_units cap helper) *(Stream-1; `CN-PLUTUS-01/04` — the +1 BLUE canonical type on `ade_plutus`)* | `ade_plutus::tx_eval` (BLUE) | closed; `ade_plutus` 8→**9** | The private helper backing the per-script declared-`ex_units` cap that closes a Plutus false-accept. Phase-two evaluation fails closed on a script exceeding its declared budget. New field = a `CN-PLUTUS-01/04` strengthening (`ci_check_plutus_budget_cap.sh` + the conformance gates). |
| `AdmissionLogEvent` (the closed convergence-evidence vocabulary) *(N-M-B 8; N-AO 22; + MEM-OPT-OPS S3 `MemoryMeasure`/`MemorySummary` = **24 variants**)* | `ade_node::admission_log::event` (GREEN) + `writer` `DISCRIMINATORS` (GREEN) | **24 variants** | **CLOSED sum, NO open/wildcard variant.** The base 8 + the N-AO fork-choice/supersession/missing-bridge/range-refetch 14 (`needs_fork_choice` … `range_refetch_completed`) + the MEM-OPT-OPS 2 (`memory_measure` / `memory_summary`). New variant = a code change in `event.rs` (variant + `discriminator()` arm) AND a `DISCRIMINATORS` allow-list entry in `writer.rs` AND allow-list updates in the closed-vocabulary CI gates. **Observe-only** — every fork-choice WIN pairs to EXACTLY ONE terminal (`fork_switch_applied` \| `fork_switch_failed` \| `fork_switch_superseded`, `DC-EVIDENCE-04`); `block_received` is per-block peer-attributed (S1 `6846d252`). |
| `BranchProofError` *(N-AO S4 / DC-NODE-37)* | `ade_node::fork_switch` (RED/GREEN-pure) | closed 7-variant | `EmptyBranch` / `BodyUnavailable{slot}` / `BodyHeaderMismatch{index}` / `BrokenParentLink{index}` / `BodyInvalid{index}` / `AnchorUnreachable`. The closed proof-failure surface of `prevalidate_branch`; any path is fail-closed (the durable chain unchanged). New variant = a proof-step addition + a `DC-NODE-37` strengthening (`ci_check_fork_switch_never_abandons.sh`). |
| `ForkSwitchOutcome` *(N-AO S4 / DC-NODE-37; DC-EVIDENCE-05)* | `ade_node::fork_switch` (RED) | closed 2-variant | `Adopted{new_tip, new_tip_prev}` \| `ProofFailed{error}`. `new_tip_prev` is capture-only evidence fidelity — NOT read by any authority path. |
| `MissingBridgeReason` *(N-AO S11 / DC-NODE-39)* | `ade_node::fork_switch` (RED) | closed 5-variant | `BranchGap` / `NoDurableAncestorWithinK` / `ExceededK` / `CacheSelfBindingViolation` / `LcaUnreachable`. A `MissingBridge` is ONLY a structured fail-closed outcome — never an adoption path, rollback target, candidate anchor, fence-clear, or trust-the-later-block. Closed `as_str()` discriminator. New variant = a fail-closed cause + a `DC-NODE-39` strengthening (`ci_check_missing_bridge_fail_closed.sh`). |
| `RangeRefetchOutcome` *(N-AO S14 / DC-NODE-41)* | `ade_node::fork_switch` (RED) | closed sum | `Admitted` (the ONLY forward progress) \| `Unavailable` \| `ShortRange` \| `BodyHeaderMismatch` \| (broken parent link / other) — every non-`Admitted` path LEAVES the structured `MissingBridge` hold. Closed discriminant for the transcript. |
| `PendingForkSwitch` + `ForkAnchor` + `SelectorProjectError` *(N-AO S3/S6 / DC-NODE-36/37)* | `ade_node::selector_state` (GREEN) | closed structs + 1-variant error | `PendingForkSwitch{fork_anchor, winning_peer, winning_candidate, winner_tip}` is the PROVISIONAL S3 decision (applies nothing). `ForkAnchor{slot, hash, block_no}` binds Ade's durable stored point (`DC-NODE-29`). `winner_tip` is a `BlockFetch RequestRange` endpoint ONLY. `SelectorProjectError::UnsupportedTpraosTip` fails a legacy TPraos tip closed. |
| `PostSwitchFollow` + `RangeRefetch` *(N-AO S14 / DC-NODE-41)* | `ade_node::fork_switch` (RED) | closed structs | RECOVERY state, NOT selection authority — consulted only to decide *whether* to re-fetch (winning peer only, bounded retry), never which branch wins. |
| `LcaError` + `LcaResult` + `CachedHeader` *(N-AO S7 / DC-NODE-38; retention S13 DC-NODE-40)* | `ade_node::lca_walk` (GREEN-by-fn) | closed 4-variant + structs | `LcaError`: `BranchGap` / `NoDurableAncestorWithinK` / `ExceededK` / `CacheSelfBindingViolation` (mapped 1:1 into `MissingBridgeReason`). Fail-closed, k-bounded (block depth), cache self-binding-checked. A retained rolled-back block is walk-visible EVIDENCE only — never the LCA anchor. |
| `PostSwitchContinuity` *(N-AO S10 / DC-EVIDENCE-05)* | `ade_node::post_switch_continuity` (GREEN) | closed 5-variant | `ContinuesSelectedBranch{…}` / `Diverged` / `BrokenLineage` / `DanglingForkChoiceWin` / `InsufficientEvidence`. OBSERVE-ONLY replay verdict over Ade's OWN admitted lineage (the peer tip is NEVER an input). |
| `WalEntry::RollBack` + `RollbackPoint` + `RollbackReason` *(N-AI / DC-NODE-25/27; `ForkChoiceWin` arm LIVE since N-AO)* | `ade_ledger::wal::event` (BLUE) | closed sum (tag 1) | The DURABLE rollback MARKER. `RollbackReason::ForkChoiceWin` + `PeerRollBackward`. Append-only (tag 0 `AdmitBlock`, tag 1 `RollBack`, tag 3 `SeedEpochConsensusInputsImported`; tag 2 reserved). New variant/reason = a versioned WAL change + replay corpus (`ci_check_wal_rollback_replay_equiv.sh`). |
| `RecoveredAnchorPoint` + `RecoveredAnchorPointError` *(N-AK / DC-NODE-31)* | `ade_ledger::recovered_anchor_point` (BLUE) | closed, `RECOVERED_ANCHOR_POINT_SCHEMA_VERSION = 1` | Version-gated `array(4) [version, anchor_fp, slot, block_hash]` + SOLE codec. New field = a schema-version bump. |
| `ServedChainSource` / `ServeRangeOutcome` / `CappedSlotRange` / `MAX_SERVE_RANGE_BLOCKS=256` *(N-U/N-AA / DC-NODE-13 / DC-SERVEMEM-01)* | `ade_runtime::network::{serve_dispatch, served_chain_projection}` + `chaindb::types` (RED) | closed | The READ-ONLY durable-ChainDb serve projection, bounded per request. |
| `NodeSyncItem` *(N-AI / N-AO S1 — PEER-TAGGED, DC-NODE-34)* | `ade_node::node_sync` (RED) | closed sum | `Block { peer, bytes }` \| `RollBack(Point)`. Transient feed type (not persisted/hashed) — no canonical/replay obligation. |
| `ReceiveClass` / `ReceiveDisposition` / `NodeSyncError` / `ForgeRefused::ReselectionPending` *(N-AI / DC-NODE-23/24/28/29)* | `ade_node::node_sync` (RED) | closed | The detector/resolver vocabulary + the fail-closed forge refusal while reselection pends. |
| `AdmissionPeerEvent` (incl. `RollBackward {peer, point, tip}`) + `AdmissionWirePumpError` *(N-AI / DC-PUMP-01)* | `ade_runtime::admission::wire_pump` (RED) | closed | The wire-pump event set — the rollback POINT preserved. The keep-alive client (N-AM) emits NONE (`DC-PUMP-03`). |
| `ArrayHead` / `PrevHash` / `TagEnvelopeError` / `BlockValidityError::HeaderPositionInvalid` / `SeedEpochConsensusInputs` (`epoch_nonce`, schema 2) / `LeaderCheckVerdict` / `ExpectedVrfInput` *(N-F-G / N-X / N-W / N-R-A)* | various BLUE | closed | The carried BLUE wire/forge/recovery closed surfaces. |
| `Mode` / `ForgeMode` / `VenuePolicy` / `VenueRole` / `ForgeActivation` / `NodeBlockSource` / `LoopStep` / `ForgeIntent` / `OperatorForgeMaterial` / the `live_log` + `convergence_evidence` + `rehearsal_evidence` + `ba02_evidence` vocabularies *(N-F-C … N-AJ)* | `ade_node::*` (RED + GREEN) | closed | The carried `--mode node` lifecycle / forge / evidence closed surfaces. `ForgeActivation` carries the five N-AO fork-switch RED fields — see §5. |
| Network message taxonomies (`AcceptedMiniProtocol`, per-protocol message enums, `KeepAliveMessage`) / `CardanoEra` (Byron=0…Conway=7) / `OutboundCommand` / `DispatchError` *(network / N-S-B)* | `ade_network::*` (BLUE/GREEN) + `ade_runtime::network` (RED) | closed | The frozen wire grammars. |

### Extensible (open within constraints)

> Ade has **very few** extensible registries — the BLUE core is closed by construction. **MEM-OPT-OPS introduced NO new extension point** (no allocator-plugin registry, no sampler registry — the allocator is a single fixed `#[global_allocator]` declaration, the samplers are fixed functions). The PHASE4-N-AO `BranchBodySource` trait remains the one extension-shaped surface, fenced to byte-transport only.

| Registry | Location | Extension Rule |
|----------|----------|---------------|
| **`BranchBodySource`** (the RED branch-body fetch seam) *(N-AO S4/S6 / DC-NODE-37)* | `ade_node::fork_switch` (RED) | **The ONE `Box<dyn …>` extension point — BYTE-ONLY, never adoption authority.** `trait BranchBodySource { fn fetch_body(&self, peer, slot) -> Result<Vec<u8>, FetchError> }`. Impls: `NullBranchBodySource` (the fail-closed fence — serves nothing) and `PrefetchedBranchBodies` (live `BlockFetch RequestRange` bytes). A new impl may supply body bytes from a new transport but **MUST NOT short-circuit `prevalidate_branch`** — a lying/short/truncated/Byzantine fetch is rejected before any `commit_rollback`. Fenced by `ci_check_live_blockfetch_byte_only.sh` + `ci_check_fork_switch_never_abandons.sh`. **BlockFetch transports bytes; it does not grant truth.** |
| Mempool tx admission (sorted, deduplicated) | `ade_ledger::mempool` (BLUE) | New txs enter at runtime via the single `mempool_ingress` chokepoint (behind the MEM-MEASURE bounded fold within `MAX_INBOUND_ADMISSION_{BYTES,COUNT}`); sort/dedup invariants preserved. |
| Peer set (`--peer`, repeatable `Vec<String>`) | `ade_node::cli` (RED) | New peers added at runtime via the CLI flag → N pumps → the deterministic `fair_merge` (N-AO S8). A peer is a transport endpoint, never an authority. |
| Served-chain read projection | `ade_runtime::network::served_chain_projection` (RED) | Bounded read-only projection of the durable ChainDb; per-request cap `MAX_SERVE_RANGE_BLOCKS=256`. Not openly extensible — a fixed bound. |

> Note: `ade_plutus` ports the `aiken_uplc` evaluator behind a quarantine boundary (pinned tag `v1.1.21`) — a frozen vendored dependency, NOT a runtime plugin registry. `mimalloc` is a process allocator dependency, NOT a runtime-registered allocator plugin. There are no HSM-plugin / scenario-template / federation-contract style runtime registries in Ade.

---

## 4. Version-Gated vs. Frozen Contracts

### Frozen (immutable at current version — change = new major version)

- **Wire format / encoding**: Cardano-canonical CBOR via `minicbor` + the `ade_codec` canonical primitives — field order = wire order for hash-bearing structures; **wire bytes are preserved, never re-encoded for hashing** (`ci_check_hash_uses_wire_bytes.sh`). The wire format is Cardano's.
- **Tag-24 envelope**: `0xd8 0x18` CBOR-in-CBOR — the SOLE `ade_codec::cbor::tag24::{wrap_tag24, unwrap_tag24}` authority (`CN-WIRE-08/12`); no second/hand-rolled tag-24 parse anywhere.
- **Hash algorithms**: `blake2b_256` / `blake2b_224` (`ade_crypto::hash`) — immutable per version; the single body-hash recipe `block_body_hash`. **The MEM-OPT-OPS streaming-import `UtxoFingerprint` and the MEM-MEASURE evidence `workload_hash` / `final_fingerprint` reuse `blake2b_256` over the SAME canonical bytes the whole-buffer oracle hashes** — the fingerprint input is allocator/parse-order-independent, never the allocator's allocation addresses.
- **The UTxO canonical fingerprint**: the canonical-CBOR-over-keys `UtxoFingerprint` (`ade_runtime::seed_import` over the BLUE `UTxOState`) — **computed by the canonical CBOR encoder over canonically-encoded `TxIn` keys, NEVER from a storage backend's native iteration order, AND independent of the process allocator** (`DC-MEM-05/06`). The streaming and whole-buffer paths agree byte-for-byte on identical-by-canonical-key inputs.
- **The header VRF / KES / DSIGN verification recipes**: `ade_crypto` (Praos VRF draft-03, the Ade-owned Sum6KES matching `cardano-base` byte-for-byte, Ed25519). **VRF strength is FROZEN.**
- **`select_best_chain`** (the `DC-CONS-03` fork-choice contract): **FROZEN — byte-unchanged by N-AO AND MEM-OPT-OPS.** k-bounded, density-free, arrival-order-independent. The single selector.
- **The process allocator is determinism-neutral, not a frozen contract**: the `#[global_allocator]` (mimalloc) MAY be swapped — what is FROZEN is that the allocator is INVISIBLE to BLUE (allocation addresses/sizes never enter a fingerprint or replay output; `DC-MEM-06`, `ci_check_alloc_determinism_neutral.sh`).
- **All 464 canonical types**: existing wire formats frozen; new types may be added (the +2 this span — `TxSubmissionTxId` wire-locked, `RedeemerFields` private — are closed-by-construction; MEM-OPT-OPS added ZERO).
- **The closed era enum** `CardanoEra` (Byron=0 … Conway=7); the closed `PrevHash = Genesis | Block(Hash32)`.
- **The durable WAL grammar**: `WalEntry` closed sum (tag 0 `AdmitBlock`, tag 1 `RollBack`, tag 3 `SeedEpochConsensusInputsImported`); tag 2 reserved. Append-only.

### Version-gated (can evolve across major versions)

- New variants in the closed `AdmissionLogEvent` convergence/evidence vocabulary (MEM-OPT-OPS took it 22 → 24 with `memory_measure` / `memory_summary`): require a `DISCRIMINATORS` allow-list entry + the closed-vocabulary CI gates (`ci_check_admission_log_vocabulary_closed.sh`, `ci_check_convergence_evidence_vocabulary_closed.sh`, `ci_check_mem_measure_evidence.sh`). **Observe-only — never an authority surface.**
- New closed measurement `point` values in the `AdmissionLogEvent::MemoryMeasure` set (S3 added `seed_import`): require an allow-list entry in `ci_check_mem_measure_evidence.sh`. Observe-only.
- New `JsonSeedError` / `BranchProofError` / `MissingBridgeReason` / `RangeRefetchOutcome` / `ShedReason` / `EvidenceDefect` discriminants: require the matching fail-closed CI gate + a `DC-MEM-06/07`, `OP-MEM-01`, or `DC-NODE-37/39/41` strengthening.
- Canonical type schema additions (new fields appended; sort/dedup + version-byte invariants preserved — e.g. `SeedEpochConsensusInputs` schema 1→2, `RecoveredAnchorPoint` schema 1).
- New `WalEntry` variants / `RollbackReason` arms: a versioned WAL change + a replay-equivalence corpus.
- New `--mode node` CLI flags: must be path-PRESERVING and added to the `ci_check_node_path_fidelity.sh` allow-list (MEM-OPT-OPS added none).
- New CI checks (existing checks may be TIGHTENED, never relaxed — the span added 17 and extended 2 in place).

---

## 5. Module Addition Rules

How new modules enter the workspace.

| Color | Naming convention | Build-config flags | May depend on | MUST NOT depend on |
|-------|-------------------|--------------------|----------------|--------------------|
| **BLUE** | crate prefixes `ade_codec` / `ade_types` / `ade_crypto` / `ade_core` / `ade_ledger` / `ade_plutus`; plus the 9 BLUE `ade_network` submodule paths (`mux/frame.rs`, `codec/`, `handshake/`, `chain_sync/`, `block_fetch/`, `tx_submission/`, `keep_alive/`, `peer_sharing/`, `n2c/`) | `// Core Contract:` banner; `#![deny(unsafe_code, clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::float_arithmetic)]`; **no `cfg(feature)` semantic gates** (`ci_check_no_semantic_cfg.sh`) | Other BLUE modules (downward only) | Any RED/GREEN crate; `ade_runtime`/`ade_node`/`ade_core_interop`; std runtime I/O; tokio/async (`ci_check_no_async_in_blue.sh`); `pallas_*` outside `ade_plutus`; **`mimalloc` (the allocator stays on the `ade_node` binary crate only — `DC-MEM-06`)** |
| **GREEN** | `ade_testkit` (whole crate); GREEN-by-content sub-trees inside RED crates carry a `//! GREEN …` banner + the BLUE deny attributes | same deny attributes; purity CI gate per sub-tree | BLUE modules | RED modules in non-test deps; nondeterminism (wall-clock / rand / float / HashMap); **letting an RSS / owned-footprint magnitude enter a fingerprint / verdict / validator (`OP-MEM-01/02`)** |
| **RED** | `ade_runtime`, `ade_node`, `ade_core_interop`, `ade_network::mux::transport` | `//! RED …` banner; tokio/std/I/O allowed; key custody confined to `ProducerShell`; the `Clock` seam is the SOLE wall-clock observation; the `mem_measure::rss_sampler` is the SOLE `/proc` memory reader | Any module | — (RED is the leaf) |

**MEM-OPT-OPS added NO new module** — its three slices are field/enum/error-surface additions to EXISTING modules, all color-preserving:

- `ade_node::main` (RED binary) gained a single `#[global_allocator] mimalloc::MiMalloc` declaration (S1). The `mimalloc` dep is on the `ade_node` binary crate ONLY.
- `ade_runtime::seed_import::importer` (GREEN-by-content) was rewritten streaming (S2) — same canonical output, fail-closed `DuplicateTxIn`.
- `ade_node::mem_measure::rss_sampler` (RED) gained `sample_rss_anon_kib` / `sample_private_dirty_kib` (S3); `admission_log::event` / `convergence_evidence` (GREEN) gained the `MemoryMeasure`/`MemorySummary` owned-memory fields.

**The MEM-MEASURE substrate (MEM-OPT-OPS builds on it; carried)** is the `ade_node::mem_measure` sub-tree — split-color by file inside the RED `ade_node` crate: GREEN `bounded_admission` (the bounded fold + `BoundedOutcome`/`ShedReason`/`MAX_INBOUND_ADMISSION_{BYTES,COUNT}`), GREEN `evidence` (`MemEvidenceRecord`/`validate_evidence`/`pair_replay`/`ReplayVerdict`/`EvidenceDefect`), RED `rss_sampler` (the `/proc` reader), GREEN/RED `runner` (the measurement seam). The GREEN cores are promotable to BLUE on demand, never demotable to RED.

**The four PHASE4-N-AO RED `ade_node` modules** (`fork_switch` / `selector_state` / `lca_walk` / `fair_merge`) + the GREEN `candidate_aggregator` / `post_switch_continuity` are carried verbatim — all in the already-RED `ade_node` host crate.

**The `ForgeActivation` fork-switch lifecycle state (RED, `ade_node::node_lifecycle`).** `pub struct ForgeActivation<'a>` carries the five N-AO RED **recovery** fields (`pending_fork_switch` / `pending_missing_bridge` / `post_switch_follow` / `pending_range_refetch` / `rollback_retention`) joining the carried `last_forge_refused` / `pending_reselection`. **These are RED recovery / sequencing state, NEVER selection authority** — they hold the forge fence; none decides which branch wins. Fenced by `ci_check_live_fork_choice_wiring.sh` + `ci_check_fork_switch_never_abandons.sh` + `ci_check_post_switch_convergence_window.sh` + `ci_check_missing_bridge_refetch.sh` + `ci_check_rollback_retention_evidence.sh`. (Unchanged by MEM-OPT-OPS.)

### New module checklist

1. Add to the Cargo workspace `[workspace] members` (MEM-OPT-OPS added NO crate — its work is field/enum/error-surface additions to existing modules).
2. Apply color-specific banner + lints (BLUE: `// Core Contract:` + deny attributes + no-semantic-cfg; GREEN: `//! GREEN` + deny attributes + purity gate; RED: `//! RED`).
3. `ci_check_dependency_boundary.sh` rejects forbidden cross-color imports; `ci_check_module_headers.sh` enforces the banner.
4. New canonical types: structural-grep-counted from the BLUE trees; add round-trip tests (the +2 this span are Stream-1/2 BLUE types; MEM-OPT-OPS added none — its new types are RED/GREEN, not canonical-counted).
5. A new selector / adoption / rollback / allocator-visible-to-BLUE / second-seed-import-authority path is **forbidden** — route into the existing `select_best_chain` / `pump_block` / `materialize_rolled_back_state` / `mempool_ingress` / `import_cardano_cli_json_utxo`; the allocator stays determinism-neutral.

### CI gates that enforce the boundary (**190 total**)

Cross-cutting BLUE gates (scope the full BLUE set): `ci_check_module_headers.sh`, `ci_check_forbidden_patterns.sh`, `ci_check_dependency_boundary.sh`, `ci_check_no_signing_in_blue.sh`, `ci_check_no_semantic_cfg.sh`, `ci_check_hash_uses_wire_bytes.sh`, `ci_check_ingress_chokepoints.sh`, `ci_check_pallas_quarantine.sh`, `ci_check_no_async_in_blue.sh`. Fork-choice / selector gates (carried + N-AO): `ci_check_no_density_in_fork_choice.sh`, `ci_check_chain_selection_arrival_order_independent.sh` (`CN-CONS-01`), `ci_check_consensus_closed_enums.sh`, `ci_check_rollback_materialize_closure.sh`, `ci_check_rollback_target_canonical_binding.sh` (`DC-NODE-29`), `ci_check_wal_rollback_replay_equiv.sh`, `ci_check_live_fork_choice_apply.sh`, `ci_check_live_fork_choice_wiring.sh`, the 12 N-AO gates (`ci_check_peer_identity_preserved.sh` … `ci_check_missing_bridge_refetch.sh`).

**The span added 17 gates** (173 → 190). The **seven MEM-MEASURE / MEM-OPT-OPS gates:**
- `ci_check_alloc_determinism_neutral.sh` (`DC-MEM-06`) — the process `#[global_allocator]` is determinism-neutral; allocation addresses/sizes never enter a fingerprint/replay output.
- `ci_check_mem_opt_s1_reduction.sh` (`CE-OPS-1`) — the committed S1 mimalloc `VmRSS −29.8 %` evidence.
- `ci_check_mem_opt_s2_import_peak.sh` (`DC-MEM-06`) — the streaming seed import is byte-identical to the whole-buffer oracle; import peak halved; `DuplicateTxIn` fail-closed.
- `ci_check_mem_opt_s3_owned.sh` (`OP-MEM-02`) — the OWNED-footprint samplers + the honest comparison; RSS magnitude never gates a verdict.
- `ci_check_bounded_inbound_admission.sh` (`DC-MEM-07`) — the bounded fold stays within the closed `MAX_INBOUND_ADMISSION_{BYTES,COUNT}`; the bound never changes an authoritative output.
- `ci_check_mem_measure_evidence.sh` (`OP-MEM-01`) — a `MemEvidenceRecord` is valid evidence only if its replay verdict is `Agreed`; enforces the closed event vocabulary + the closed measurement-`point` set (incl. `seed_import`).
- `ci_check_mem_compare_evidence.sh` (`OP-MEM-01/02`) — the committed Haskell-vs-Ade gross + owned-footprint comparison schema.

The **two extended in place** (no rule weakened): `ci_check_admission_log_vocabulary_closed.sh` + `ci_check_convergence_evidence_vocabulary_closed.sh` — both extended to assert the closed `memory_measure` / `memory_summary` evidence variants stay in the closed allow-list. The **ten Stream-1/2/3 gates** (separate from MEM-OPT-OPS): `ci_check_plutus_{conformance,budget_cap,eval_purity,oracle_no_false_accept}.sh`, `ci_check_required_signer_closure.sh` (Stream 1); `ci_check_tx_submission2_real_capture.sh` (Stream 2); `ci_check_codec_message_closed.sh`, `ci_check_header_body_binding.sh`, `ci_check_mini_protocol_{surface,transition_purity}.sh` (Stream 3).

---

## 6. Forbidden Patterns (per color)

Universal IDD prohibitions per color (from `~/.claude/methodology/idd.md` Part IV):

- **BLUE:** no clock, rand, raw HashMap/HashSet, float, env access, network/filesystem, async runtime, locale-dependent ops, OS-dependent ordering.
- **GREEN:** no nondeterminism; no participation in authoritative outputs.
- **RED:** no direct mutation of BLUE state; no unsafe construction of semantic types; no bypassing canonical validation.

### Project-specific additions (Ade)

**MEM-OPT-OPS / MEM-MEASURE:**

- **A storage / memory-representation / allocator change is NEVER a consensus or replay change** (`DC-MEM-05/06`). The `#[global_allocator]` MUST be determinism-neutral — allocation addresses/sizes never enter a fingerprint or replay output. The `mimalloc` dep stays on the `ade_node` binary crate ONLY; no BLUE/GREEN crate may take it.
- **The streaming seed import MUST be byte-identical to the whole-buffer oracle** on identical-by-canonical-key inputs — the `(UTxOState, UtxoFingerprint)` agree byte-for-byte; the fingerprint is over canonical `TxIn` keys, allocator/parse-order independent (`DC-MEM-06`). **ANY canonical-`TxIn` duplicate fails closed** (`JsonSeedError::DuplicateTxIn`) — never a silent order-dependent survivor. The whole-buffer `_from_bytes` is the retained equivalence ORACLE, not a second production authority.
- **An RSS / owned-footprint magnitude MUST NOT enter a fingerprint, verdict, validator pass/fail, or any authoritative comparison** (`OP-MEM-01/02`) — RSS is release-tier evidence ONLY; the run's REPLAY VERDICT, not RSS, decides validity. A measurement whose replay verdict is not `Agreed` is INVALID evidence and MUST NOT be reported as a pass.
- **The bounded inbound-admission working set MUST stay within the closed non-configurable `MAX_INBOUND_ADMISSION_{BYTES,COUNT}`** (`DC-MEM-07`) — memory pressure cannot grow it unboundedly; the bound never changes an authoritative output; the fold gates inbound bytes BEFORE the BLUE `mempool_ingress`, never inside/bypassing it; a shed event is never an acceptance.
- **The `AdmissionLogEvent::{MemoryMeasure, MemorySummary}` vocabulary stays CLOSED** — no open `*_kib` variant; observe-only; emitting it affects no authority. The measurement-`point` set is closed.
- **No `mem_measure` over-claim of semantic truth** — a low-memory or `lagging` run is not "success"; the honest comparison may not claim a bound the evidence does not support (S3 recorded `ade_heavier`).

**Streams 1/2/3 (separate from MEM-OPT-OPS):**

- **No Plutus false-accept past the per-script declared `ex_units`** and no impure evaluator host env (`CN-PLUTUS-01/04`).
- **No second / hand-rolled tx-submission2 codec**, and no tx-submission2 codec that diverges from the real capture corpus (`DC-PROTO-11`).

**PHASE4-N-AO (carried):**

- **No second chain selector.** `select_best_chain` is routed-to, never duplicated; no parallel preference, density ordering, or operator heuristic.
- **No RED-minted candidate summary may reach `select_best_chain`** — candidate fragments are derived ONLY from `validate_and_apply_header` output, never the `follow.rs` mint (`DC-NODE-35`).
- **NEVER commit a rollback of the current durable chain until the replacement branch's bodies are fetched, linked, and validated as a complete candidate branch** (`DC-NODE-37`). A failed / lying / incomplete / Byzantine winner leaves ChainDb / ledger / chain_dep byte-unchanged.
- **`pump_block` stays the sole roll-forward durable admit** — no header-only tip advance; a fork-choice win is provisional until bodies apply.
- **The fork-anchor rollback target binds Ade's durable stored slot+hash** (`DC-NODE-29`); `winner_tip` is a fetch endpoint only, not adoption authority.
- **`BranchBodySource` carries BYTES only.** No impl may short-circuit `prevalidate_branch`; `NullBranchBodySource` is the fail-closed fence.
- **A `MissingBridge` is a structured fail-closed outcome only** (`DC-NODE-39`); no durable mutation on that path.
- **The convergence/continuity evidence vocabulary is a CLOSED enum** — no open/wildcard variant; observe-only; every fork-choice win pairs to exactly one terminal (`DC-EVIDENCE-04`).
- **Venue stays explicit + closed** — only `Participant` reaches SELECT; `SingleProducer` / `Unknown` fail closed.
- Carried: no second block-envelope encoder; no second `leader_vrf_input` authority; no second `wrap_tag24`/`unwrap_tag24` or hand-rolled tag-24 parse; no forward-sync `AdvanceTip` before durability; no Mithril/genesis bootstrap bypassing the single `bootstrap_initial_state`; no internal-fingerprint-vs-Haskell-hash equality assertion; no registry rule citing a non-existent `code_locus`; no second eta0-overlay authority (only `PraosChainDepState::overlay_recovered_eta0`); no rollback-replay VRF bypass / skip / loosening.

---

## 7. Candidate & Not-Yet-Wired Seams (declared follow-ons — NOT closed)

> Surfaced for confirmation, not asserted wired. Items the user should confirm.

- **`OP-MEM-02` owned-footprint bound — `declared`, honestly recorded `ade_heavier`.** S3 committed the OWNED-footprint (`RssAnon`) comparison; on the apples-to-apples cross-node metric Ade is still heavier than the reference Haskell node. The bound is an OPERATIONAL obligation NOT yet met — `OP-MEM-02` stays `declared` (not a wired pass). **CONFIRM with the user the intended follow-on:** further memory reduction toward the bound, or a re-scope of the target. `DC-MEM-05` / `DC-MEM-07` / `DC-MEM-08` also remain `declared`.
- **`DC-MEM-06` — `partial`.** S1 (allocator determinism-neutral) + S2 (streaming-import byte-equivalence + `DuplicateTxIn` fail-closed) are gated; the rule is `partial` rather than `enforced`. **CONFIRM with the user** which sub-obligation of `DC-MEM-06` is not yet `enforced` (the registry's `DC-MEM-06` entry is the source) — likely the compact/lazily-decoded UTxO-representation half (`DC-MEM-05` cross-ref) that S1–S3 did not ship.
- **`RO-LIVE-01` — operator-gated.** Stays operator-gated (preprod, ADE1 stake ~epoch 295). MEM-OPT-OPS did NOT touch it; the memory work reduces the runtime profile of the operator-pass path without changing its authority surface.
- **`chain_selector::process_rollback` (the orchestrator rollback path) — carried N-AO candidate.** Whether it is reachable live or remains test-only is still a candidate to verify against the final S3/S4 wiring (the slice docs route through `apply_chain_event`, suggesting it stayed test-only). Unchanged by MEM-OPT-OPS.
- **Full Cardano ChainSel (N>2 peers, adversarial load)** — preprod rung-3, out of scope. PHASE4-N-AO flipped `CN-CONS-03` on the committed two-producer CE-AO-6 transcript; it does NOT prove full Cardano ChainSel.
- **The keep-alive SERVER/responder** (N-AM shipped the CLIENT only) — a CE-AM-LIVE-gated follow-on.
- **Any future `BranchBodySource` impl beyond `PrefetchedBranchBodies` / `NullBranchBodySource`** — none is planned; a new impl must stay byte-only behind `prevalidate_branch` (§3-extensible). Confirm before adding.

---

## Generation notes

- Generated by `/seams` at HEAD `233644f7` (MEM-OPT-OPS cluster-close refresh), reading `docs/ade-CODEMAP.md` (**at the SAME HEAD `233644f7`** — CODEMAP led this refresh; the prior SEAMS was the stale party at `862cd2cb`) and `docs/ade-invariant-registry.toml` (**378 rules** — the canonical count source, holding the 6 new MEM/PROTO rules).
- This regeneration FOLDS the `862cd2cb..233644f7` span onto the prior PHASE4-N-AO SEAMS: the MEM-OPT-OPS additive closed surfaces (the `JsonSeedError::DuplicateTxIn` fail-closed seed-import surface; the `AdmissionLogEvent::{MemoryMeasure, MemorySummary}` gross+owned `*_kib` evidence variants + the `seed_import` measurement point; the mem-measure closed enums `BoundedOutcome`/`ShedReason`/`ReplayVerdict`/`EvidenceDefect`; the determinism-neutral `#[global_allocator]` seam) + the continuity `TxSubmissionTxId` tx-submission2 surface. The PHASE4-N-AO SELECT seam is carried verbatim (the modules live unchanged in the codebase per the CODEMAP).
- **MEM-OPT-OPS is RED+GREEN only — it added NO new module/crate, NO new CLI flag, and NO new BLUE canonical type.** Every MEM-OPT-OPS seam is a field/enum/error-surface addition behind an EXISTING closed vocabulary. The BLUE consensus + ledger core is reused byte-unchanged (`ade_core` 49→49, `ade_ledger` 181→181). The +2 BLUE canonical-type delta (`RedeemerFields`, `TxSubmissionTxId`) is the separate Stream-1/2 local-streams work.
- **Counts at this HEAD:** 11 crates / 464 canonical types / 190 CI checks / 378 registry rules (250 enforced / 22 partial / 105 declared / 1 enforced_scaffolding).
- **Candidates surfaced for human confirmation** (NOT auto-included as wired): the `OP-MEM-02` owned-footprint bound (honestly `declared` / `ade_heavier`); which sub-obligation of `DC-MEM-06` (`partial`) is not yet `enforced`; the live reachability of `chain_selector::process_rollback` (carried §7); whether any future `BranchBodySource` impl is planned.
