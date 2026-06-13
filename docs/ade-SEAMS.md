# Seams — Where New Work Can Attach (Ade)

> **Status:** Living architectural document. Regenerated; not hand-edited.
> Per-project instance of `~/.claude/methodology/templates/seams.md`.

> 11 crates, **462 canonical types**, **173 CI checks**, **372 registry rules** at HEAD (`862cd2cb`, PHASE4-N-AO CLOSE — live multi-candidate fork-choice **SELECT + adopt** (rung-2), the half PHASE4-N-AI's single-best-peer FOLLOW did not do). This regeneration folds the entire `b8860b16..862cd2cb` PHASE4-N-AO span (47 commits, slices S1…S14 + close): the cluster wired the live `NeedsForkChoice` arm to the EXISTING BLUE `select_best_chain` over an aggregated candidate **set**, proved the winning replacement branch (fetched bodies, bound to S3-selected headers, parent-linked from the durable fork anchor, BLUE `block_validity`-validated) **STRICTLY BEFORE** any irreversible `commit_rollback` (`DC-NODE-37`), and — on the committed CE-AO-6 transcript — **flipped `CN-CONS-03` → `enforced`** (live multi-candidate convergence in the exercised two-producer venue; NOT full Cardano ChainSel).
>
> ### What PHASE4-N-AO added to the seam surface (the load-bearing summary)
>
> **No new ingress surface, no new openly-extensible / plugin / negotiated / runtime-registered registry, and no second chain-selection authority.** `select_best_chain` (`ade_core::consensus::fork_choice`, `DC-CONS-03`) stays the **SINGLE selector** — routed-to, never duplicated; a competing Participant candidate set is routed into it, and `pump_block` stays the **sole roll-forward durable admit** (a fork-choice win is *provisional* until its bodies validate + apply). The span ADDED:
>
> - **One RED fetch seam (a trait — the only `dyn`/extension-shaped surface this cluster introduced):** `ade_node::fork_switch::BranchBodySource` — a **byte-only** body-fetch abstraction with two impls (`NullBranchBodySource` = the fence placeholder; `PrefetchedBranchBodies` = the live `BlockFetch RequestRange` bytes). **It carries BYTES, never adoption authority** (see §2 / §3-extensible — this is the one place a `Box<dyn …>` is allowed, and it is fenced read-only / proof-gated).
> - **Four new RED `ade_node` modules** (all closed-by-content; none is a plugin host): `fork_switch` (the prove core), `selector_state` (the GREEN selector-state projection + `PendingForkSwitch` / `ForkAnchor`), `lca_walk` (the durable-LCA fork-anchor walk), `fair_merge` (the deterministic per-peer round-robin wire-pump merge).
> - **The closed `AdmissionLogEvent` convergence vocabulary broadened from 8 → 22 variants** (S9 added the 9 fork-choice events + supersession; S11 added `missing_bridge`; S14 added `range_refetch_started` / `range_refetch_completed`). It stays a **CLOSED enum (no open/wildcard variant)** enforced by the repointed `ci_check_convergence_evidence_vocabulary_closed.sh` + `ci_check_fork_choice_evidence_closed.sh` + the `ci_check_missing_bridge_*` pair + the writer's `DISCRIMINATORS` allow-list.
> - **The `ForgeActivation` fork-switch lifecycle state** gained five RED **recovery** fields (`pending_fork_switch` / `pending_missing_bridge` / `post_switch_follow` / `pending_range_refetch` / `rollback_retention`) joining the carried `pending_reselection` / `last_forge_refused`. **These are RED recovery state, never selection authority** — they hold the forge fence and sequence the prove→commit→follow→re-fetch lifecycle; none of them decides which branch wins.
>
> Every PHASE4-N-AO surface is CLOSED / additive (closed discriminants, deterministic ordering, fail-closed). `select_best_chain` is byte-unchanged. `RO-LIVE-01` stays operator-gated (preprod rung-3 ADE1 stake ~epoch 295; full N>2-peer adversarial ChainSel is preprod rung-3, out of scope here).
>
> ### Counts (mechanical, with sources)
>
> | Count | Value | Source |
> |---|---|---|
> | Crates | **11** | `grep -cE '"crates/' Cargo.toml`. No new crate this span. |
> | Canonical types | **462** | structural grep over the 6 BLUE crate `src/` + 9 BLUE `ade_network` submodule paths. **Δ vs the N-AN baseline (462): 0** — PHASE4-N-AO's hypothesis of **zero new BLUE canonical type** HELD (`RollbackReason::ForkChoiceWin` pre-existed; the new types — `BranchBodySource` / `PendingForkSwitch` / `MissingBridgeReason` / `PostSwitchFollow` / `RangeRefetch` / `CachedHeader` / `LcaResult` / the new `AdmissionLogEvent` variants — all live in RED/GREEN `ade_node`, NOT canonical-counted). |
> | CI checks | **173** | `ls ci/ci_check_*.sh \| wc -l`. **Δ vs the N-AN baseline (161): +12** — verified via `git diff --name-status b8860b16..HEAD -- 'ci/ci_check_*.sh'`: **12 `A`** (`ci_check_peer_identity_preserved.sh`, `ci_check_candidate_construction_validated.sh`, `ci_check_live_selector_dispatch.sh`, `ci_check_fork_switch_never_abandons.sh`, `ci_check_lca_anchor_walk.sh`, `ci_check_wire_pump_fairness.sh`, `ci_check_fork_choice_evidence_closed.sh`, `ci_check_live_blockfetch_byte_only.sh`, `ci_check_post_switch_convergence_window.sh`, `ci_check_rollback_retention_evidence.sh`, `ci_check_missing_bridge_fail_closed.sh`, `ci_check_missing_bridge_refetch.sh`), **6 `M`** (`ci_check_convergence_evidence_vocabulary_closed.sh`, `ci_check_live_fork_choice_apply.sh`, `ci_check_live_fork_choice_wiring.sh`, `ci_check_node_path_fidelity.sh`, `ci_check_wal_rollback_replay_equiv.sh`, `ci_check_wire_rollback_signal_preserved.sh` — extended in place), **0 `D`**. |
> | Registry rules | **372** | `grep -cE '^id = ' docs/ade-invariant-registry.toml`. **Status:** 239 `enforced` / 19 `partial` / 113 `declared` / 1 `enforced_scaffolding`. **Δ vs the N-AN baseline (361): +11** — `DC-NODE-34/35/36/37/38/39/40/41` + `DC-EVIDENCE-04` + `DC-EVIDENCE-05` + `DC-PUMP-04`, **all `enforced`** at close. **`CN-CONS-03` flipped `declared` → `enforced`** on the committed CE-AO-6 transcript. |
>
> ### CODEMAP cross-reference (read honestly — load-bearing)
>
> This SEAMS reads the CODEMAP (`docs/ade-CODEMAP.md`) for the module list + TCB colors. **The on-disk CODEMAP is pinned at `b8860b16` (the PHASE4-N-AN close — 462 / 161 / 361) and is ONE cluster stale vs this HEAD**: it does NOT yet describe PHASE4-N-AO. Specifically, the CODEMAP is missing the four NEW RED `ade_node` modules this span added (`fork_switch`, `lca_walk`, `fair_merge`, `selector_state` — all registered in `crates/ade_node/src/lib.rs`), the broadened `AdmissionLogEvent` vocabulary (8 → 22), the `ForgeActivation` fork-switch fields, the +12 CI gates (161 → 173), and the `CN-CONS-03` flip. **The registry is the canonical count source at this HEAD** (372 rules, incl. the 11 new AO rules + the `CN-CONS-03` flip). The CODEMAP picks up the four new modules + the AO rows on its next regen; this SEAMS is authoritative on the AO seam surface in the interim. The module COLORS the CODEMAP records are still accurate — every AO module lives in the already-RED `ade_node` host crate, and `select_best_chain` stays the already-BLUE sole selector.
>
> **TCB-color note (per CODEMAP, still accurate + the AO additions):** `node_sync` + `node_lifecycle` are **RED** (they drive the apply path + own the persistent-store reads + the forge wrap + the fork-switch driver `apply_fork_switch`). Their forge/receive-decision fns (`classify_receive` / `resolve_disposition` / `forge_mode_after_admit` / `single_producer_forge_decision` / `venue_policy` / `forge_followed_tip_admission` / `pending_reselection_forge_refusal`) are **GREEN-by-function** (pure / total / deterministic) inside the RED host. The AO modules: `fork_switch::prevalidate_branch` is **GREEN/BLUE-reused and pure** (no I/O, no store, no mutation — it folds BLUE `block_validity`); `selector_state` is **GREEN** (a pure projection of validated data into `PendingForkSwitch`); `lca_walk::walk_to_durable_lca` is **GREEN-by-function** (a pure read over a `&dyn ChainDb`); `fair_merge` is **GREEN** (deterministic round-robin, no HashMap/wall-clock/rand). The RED driver `node_lifecycle::apply_fork_switch` does the fetch + the read-only materialize + the irreversible `commit_rollback`. `BranchBodySource` is **RED** (the body bytes come from the winning peer over the wire).

This document describes **the closure surface of the system** — where new work can attach safely, where it cannot, and what shape attachments must take. It is the architectural complement to CODEMAP: CODEMAP says what each module *is*, SEAMS says where the system *opens*.

---

## 1. Surface Reduction Rules

> External inputs reduce to canonical form before entering authoritative pipelines. Ade's external surfaces are the N2N/N2C wire, operator files, and `argv`. Each reduces to a canonical type before any BLUE authority sees it.

**Rule:** New ingress surfaces attach by producing the canonical type's bytes and entering the same authoritative pipeline. They **may not** introduce new pipeline steps, reorder existing steps, or shortcut into the core via a back door.

### Surface: N2N inbound wire (received blocks/headers/txs/rollbacks — the LIVE `--mode node` feed)

```
Surface: N2N inbound wire (TCP + mux + handshake; ChainSync RollForward / RollBackward + BlockFetch)
Reduces to: AdmissionPeerEvent (RED, ade_runtime::admission::wire_pump) → NodeSyncItem { Block(Vec<u8>, peer) | RollBack(Point, peer) } (RED, ade_node::node_sync; PEER-TAGGED since N-AO S1 / DC-NODE-34) → BLUE DecodedBlock (ade_codec) → BLUE BlockVerdict (ade_ledger::block_validity)
Pipeline (fixed; steps cannot be reordered):
  1. mux frame decode                          (BLUE ade_network::mux::frame — single authority)
  2. session reassembly / segmentation         (GREEN ade_network::session — one DeliverPeerFrame per complete CBOR item)
  3. tag-24 unwrap                              (BLUE ade_codec::unwrap_tag24 — the SOLE tag-24 authority; CN-WIRE-12)
  4. AdmissionPeerEvent emission                (RED wire_pump — Block / TipUpdate / RollBackward {peer, point, tip} / Disconnected; the keep-alive client emits NONE — DC-PUMP-01/03)
  5. fair per-peer merge                        (GREEN ade_node::fair_merge — deterministic round-robin; N-AO S8 / DC-PUMP-04)
  6. peer-tagged NodeSyncItem                   (RED node_sync — N-AO S1 / DC-NODE-34 threads the origin peer through)
  7. classify_receive → resolve_disposition     (GREEN-by-fn — Admit | NeedsForkChoice | RollbackFollow; DC-NODE-23/24)
  8a. (Admit)          BLUE decode + block_validity → pump_block durable admit   (DC-NODE-05/12)
  8b. (NeedsForkChoice) per-peer candidate aggregation → select_best_chain → S4 prove-then-commit (N-AO; §2)
  8c. (RollbackFollow) materialize_rolled_back_state (eta0-overlaid) + commit_rollback (DC-NODE-25/26/29, T-REC-06)
Cross-surface state sharing: the per-peer wire-pump lanes share one deterministic fair-merge cursor (N-AO S8); the rollback target is bound to the durable ChainDb stored slot+hash (DC-NODE-29), never peer-supplied.
```

**N-AO note:** the live feed is the EXISTING N2N inbound wire — **no new ingress surface**. Multi-candidate SELECT rides it: competing blocks from distinct peers (preserved by the S1 peer tag) aggregate into a candidate set that is routed to the single `select_best_chain`. The winner's replacement-branch bodies are range-fetched over the SAME BlockFetch wire (`BranchBodySource` / `PrefetchedBranchBodies`; §2) — a fetch, not a new ingress.

### Surface: argv (closed mode set)

```
Surface: argv
Reduces to: Cli / Mode (closed: WireOnly | Admission | KeyGenKes | Produce | Node) — ade_node::cli
Pipeline: parse → closed Mode dispatch in main() → per-mode driver
Cross-surface state sharing: none (a CLI flag set is a CLOSED allow-list, mirrored by ci_check_node_path_fidelity.sh — N-AO added no new --mode node flag)
```

### Surface: operator file ingress (KES skey / opcert / Shelley genesis / UTxO seed dump / recovered-anchor sidecar)

```
Surface: operator files
Reduces to: KesSecret / OperationalCert / ConwayGenesisConfig / canonical seed entries / SeedEpochConsensusInputs / RecoveredAnchorPoint — via the single RED parsers in ade_runtime (each fail-closed)
Pipeline: read bytes → RED parser → canonical BLUE type → bootstrap_initial_state (the single lifecycle owner; CN-NODE-01)
Cross-surface state sharing: the recovered seed-epoch eta0 sidecar is read once by bootstrap and overlaid onto BOTH the WarmStart and rollback-materialize chain_dep (T-REC-04 / T-REC-06)
```

> Full per-surface detail (BA-02 operator-pass evidence, Mithril provenance binding, the forge-constant/operator-key/run-loop surfaces, the seed-epoch sidecar warm-start) is carried in the §2 domain tables and the §3 registries. N-AO touched none of these ingress surfaces.

---

## 2. Data-Only vs. Authoritative Layers

> For every domain where a tooling/transport layer and an authoritative layer coexist, the boundary is named. **The compilation/enforcement chokepoint never moves.**

### Domain: live multi-candidate fork-choice SELECT + adopt — RED fetch/driver vs. GREEN sequencing vs. BLUE `select_best_chain` + `block_validity` (N-AO; the cluster's defining domain)

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **Authoritative selector** | `ade_core::consensus::fork_choice::select_best_chain` | **BLUE** | **The SINGLE, sole `DC-CONS-03` fork-choice authority.** k-bounded, density-free, arrival-order-independent (`CN-CONS-01`). N-AO routes a candidate **set** into it; **byte-unchanged** by the cluster. No second selector exists (verified: one `pub fn`; all callers — `node_lifecycle.rs:2857`, `ade_runtime::consensus::chain_selector:168`, `ade_core_interop::follow:454` — route to it). |
| **Authoritative branch proof** | `ade_core::consensus` `block_validity` (via `ade_node::fork_switch::prevalidate_branch`) | **BLUE-reused / GREEN-pure** | `prevalidate_branch` is PURE (no I/O, no store, no mutation): it (1) binds each fetched body to its S3-selected `ValidatedHeaderSummary` (re-derived header + recomputed body hash — trusts nothing peer-asserted), (2) parent-links from the durable `ForkAnchor`, (3) folds BLUE `block_validity` from the materialized anchor. **A non-`Valid` verdict fails closed HERE, BEFORE the caller's `commit_rollback` (`DC-NODE-37`).** |
| **Authoritative apply** | `ade_ledger::rollback::materialize_rolled_back_state` (+ eta0 overlay) + `receive::reducer` `commit_rollback` + `pump_block` | **BLUE** | The EXISTING, reused adoption authorities. A proven branch adopts via `RolledBack(fork_anchor) + ChainSelected(body)×N` through `apply_chain_event`, recorded `WalEntry::RollBack{ForkChoiceWin}`. The fork anchor binds Ade's durable stored slot+hash (`DC-NODE-29`). `pump_block` stays the SOLE roll-forward durable admit. |
| **Selector-state projection** | `ade_node::selector_state` (`project_tiebreaker`, `PendingForkSwitch`, `ForkAnchor`) | **GREEN** | Pure projection of S2-validated header summaries into a `PendingForkSwitch` (the provisional decision S3 emits, S4 consumes). |
| **Durable-LCA fork-anchor walk** | `ade_node::lca_walk::walk_to_durable_lca` | **GREEN-by-fn** | Pure read over a `&dyn ChainDb`: walks a competing branch's cached headers down to a durable stored ancestor (the fork anchor), k-bounded (block depth), cache self-binding-checked (map key == re-derived hash, else fail closed). `DC-NODE-38`. |
| **Fork-switch driver** | `ade_node::node_lifecycle::apply_fork_switch` | **RED** | Does the body fetch (`BranchBodySource`), the read-only anchor materialize, calls `prevalidate_branch`, and — only on success — adopts via `apply_chain_event`. |
| **Body fetch seam** | `ade_node::fork_switch::BranchBodySource` (`NullBranchBodySource` / `PrefetchedBranchBodies`) | **RED** | The byte-only fetch abstraction. See §3-extensible. |
| **Per-peer wire-pump fairness** | `ade_node::fair_merge::fair_merge` | **GREEN** | Deterministic round-robin merge of per-peer bounded lanes (`DC-PUMP-04`) — no HashMap / wall-clock / rand; closed-lane retire-in-place. |

**Rule:** New fork-choice work adds candidate-construction / proof / sequencing logic in RED/GREEN `ade_node`; it routes into the EXISTING BLUE `select_best_chain` and adopts via the EXISTING BLUE `materialize` / `commit_rollback` / `pump_block`. **The selector and the adoption chokepoints never move.** No second selector, no parallel preference, no density ordering, no operator heuristic. The current durable chain is NEVER abandoned until the replacement branch is fetched, linked, and validated as a complete candidate branch (`DC-NODE-37`).

### Domain: live fork-choice rollback-FOLLOW — single-best-peer (N-AI; the precedent N-AO builds the SELECT half atop)

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **Detector / resolver** | `ade_node::node_sync` (`classify_receive` / `resolve_disposition`) | GREEN-by-fn | Classifies a competing Participant block as `NeedsForkChoice` (`DC-NODE-23/24`). |
| **Selector** | `ade_core::consensus::fork_choice::select_best_chain` | BLUE | Same single authority (`DC-CONS-03`). |
| **Durable apply** | `ade_ledger::rollback::materialize_rolled_back_state` + `commit_rollback` | BLUE | Bound to the durable stored slot+hash (`DC-NODE-29`); eta0-overlaid (`T-REC-06`). |

### Domain: rollback-materialization replay-equivalence — BLUE overlay authority (N-AN; reused by N-AO)

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **Single eta0-overlay authority** | `ade_core::consensus::praos_state::PraosChainDepState::overlay_recovered_eta0` | **BLUE** | The SOLE eta0-overlay site — shared by WarmStart bootstrap AND rollback materialize, so live admit and rollback replay validate the header VRF against the SAME nonce (`T-REC-06`). VRF strength UNCHANGED (a WRONG eta0 still fails closed). N-AO's `prevalidate_branch` materialize path inherits this. |

> The carried domains — N2N tag-24 wire envelope (N-X), block codec, leader-eligibility VRF input (N-W), KES signing-key custody, forged-block serving (data-only serve vs. authoritative admit), network forward-sync (durable-before-tip), crash recovery, bootstrap seed provenance, recovered seed-epoch consensus inputs (N-F-A), recovered-anchor live-follow start (N-AK), the self-accept→serve handoff (N-F-G-B, `--mode node` usage superseded by N-U S3 / `DC-NODE-13`), the live `--mode node` block feed (REUSED dial/pump, N-F-G-C), BA-02 operator-pass evidence, private-testnet rehearsal evidence, forge-constant fidelity, operator-key ingress + forge-on flip, the live relay run-loop, node lifecycle + BA-02 evidence — are unchanged by N-AO. Each: the RED tooling/transport layer parses/packs/moves bytes; the BLUE authority enforces; the chokepoint never moves.

---

## 3. Closed vs. Extensible Registries

The system is partitioned into closed (frozen, version-gated) and extensible (open within constraints) registries.

### Closed (frozen — version-gated changes only)

> **PHASE4-N-AO additions are at the top.** Every AO closed surface is a closed-discriminant enum / struct with deterministic ordering and fail-closed behavior; none is openly extensible.

| Registry | Location | Count | Change Rule |
|----------|----------|-------|-------------|
| `AdmissionLogEvent` (the closed convergence-evidence vocabulary) *(N-M-B 8 variants; N-AO broadened to 22 — S9 fork-choice + supersession, S11 missing_bridge, S14 range_refetch)* | `ade_node::admission_log::event` (GREEN) + `writer` `DISCRIMINATORS` allow-list (GREEN) | **22 variants** | **CLOSED sum, NO open/wildcard variant.** The base 8 (`admission_started` / `snapshot_imported` / `bootstrap_complete` / `block_received` / `block_admitted` / `agreement_verdict` / `admission_halted` / `admission_shutdown`); **S9 (`DC-EVIDENCE-04`)** added `needs_fork_choice` / `lca_discovered` / `candidate_fragment_built` / `fork_choice_selected` / `branch_fetch_started` / `branch_fetch_completed` / `branch_prevalidated` / `fork_switch_applied` / `fork_switch_failed` / `fork_switch_superseded`; **S11 (`DC-NODE-39`)** added `missing_bridge`; **S14 (`DC-NODE-41`)** added `range_refetch_started` / `range_refetch_completed`. New variant = a code change in `event.rs` (variant + `discriminator()` arm) **AND** a matching `DISCRIMINATORS` allow-list entry in `writer.rs` **AND** an allow-list update in `ci_check_convergence_evidence_vocabulary_closed.sh` + `ci_check_fork_choice_evidence_closed.sh`. **Observe-only — emitting it affects no authority** (`block_received` is per-block peer-attributed since S1's fix `6846d252`; every fork-choice WIN pairs to EXACTLY ONE terminal of `fork_switch_applied` \| `fork_switch_failed` \| `fork_switch_superseded`, `DC-EVIDENCE-04`). |
| `BranchProofError` *(NEW, N-AO S4 / DC-NODE-37)* | `ade_node::fork_switch` (RED/GREEN-pure) | closed 7-variant | `EmptyBranch` / `BodyUnavailable{slot}` / `BodyHeaderMismatch{index}` / `BrokenParentLink{index}` / `BodyInvalid{index}` / `AnchorUnreachable`. The closed proof-failure surface of `prevalidate_branch`. Any path is fail-closed: the current durable chain is unchanged. New variant = a proof-step addition + a `DC-NODE-37` strengthening (`ci_check_fork_switch_never_abandons.sh`). |
| `ForkSwitchOutcome` *(NEW, N-AO S4 / DC-NODE-37; DC-EVIDENCE-05)* | `ade_node::fork_switch` (RED) | closed 2-variant | `Adopted{new_tip, new_tip_prev}` \| `ProofFailed{error}`. `new_tip_prev` is **capture-only evidence fidelity** for the post-switch branch-continuity verdict (`DC-EVIDENCE-05`) — **NOT read by any authority path.** |
| `MissingBridgeReason` *(NEW, N-AO S11 / DC-NODE-39)* | `ade_node::fork_switch` (RED) | closed 5-variant | `BranchGap` / `NoDurableAncestorWithinK` / `ExceededK` / `CacheSelfBindingViolation` / `LcaUnreachable`. **A `MissingBridge` is ONLY a structured fail-closed outcome — never an adoption path, rollback target, candidate anchor, fence-clear, skip-the-missing-parent, or trust-the-later-block.** It HOLDS the forge fence; no durable mutation ever occurs on this path. Closed `as_str()` discriminator (no free-form strings). New variant = a fail-closed cause + a `DC-NODE-39` strengthening (`ci_check_missing_bridge_fail_closed.sh`). |
| `RangeRefetchOutcome` *(NEW, N-AO S14 / DC-NODE-41)* | `ade_node::fork_switch` (RED) | closed sum | `Admitted` (the ONLY forward progress — clears the hold) \| `Unavailable` \| `ShortRange` \| `BodyHeaderMismatch` \| (broken parent link / other) — every non-`Admitted` path LEAVES the structured `MissingBridge` hold (the `DC-NODE-39` floor fallback). Closed discriminant for the transcript. |
| `PendingForkSwitch` + `ForkAnchor` *(NEW, N-AO S3/S6 / DC-NODE-36/37)* | `ade_node::selector_state` (GREEN) | closed structs | `PendingForkSwitch{fork_anchor, winning_peer, winning_candidate, winner_tip}` is the PROVISIONAL S3 decision (S3 sets it + the `DC-NODE-28` fence but **applies nothing**). `ForkAnchor{slot, hash, block_no}` binds Ade's durable stored point (`DC-NODE-29`), never peer-supplied. `winner_tip` is a `BlockFetch RequestRange` **endpoint ONLY — NOT adoption authority** (a peer serving a different body for it is rejected by S4 `BodyHeaderMismatch`). |
| `PostSwitchFollow` + `RangeRefetch` *(NEW, N-AO S14 / DC-NODE-41)* | `ade_node::fork_switch` (RED) | closed structs | `PostSwitchFollow{winning_peer, adopted_tip, fork_switch_id}` is recorded on a `ForkChoiceWin` adoption to decide whether a `MissingBridge` is ELIGIBLE for active range re-fetch. `RangeRefetch{peer, from_tip, to_descendant, fork_switch_id, reason}` is the pending re-fetch (winning peer ONLY, `from_tip(+1)..to_descendant`). **RECOVERY state, NOT selection authority** — consulted only to decide *whether* to re-fetch, never which branch wins (S3 already decided). |
| `LcaError` + `LcaResult` + `CachedHeader` *(NEW, N-AO S7 / DC-NODE-38)* | `ade_node::lca_walk` (GREEN-by-fn) | closed 4-variant + structs | `LcaError`: `BranchGap` / `NoDurableAncestorWithinK` / `ExceededK` / `CacheSelfBindingViolation` (mapped 1:1 into `MissingBridgeReason` by `map_lca_error`). The durable-LCA walk result + per-branch cached header. Fail-closed, k-bounded (block depth), cache self-binding-checked. |
| `WalEntry::RollBack` + `RollbackPoint` + `RollbackReason` *(N-AI / DC-NODE-25/27; `ForkChoiceWin` arm LIVE since N-AO)* | `ade_ledger::wal::event` (BLUE) | closed sum (tag 1) | The DURABLE rollback MARKER. `RollbackReason::ForkChoiceWin` (which N-AO drives live for the first time) + `PeerRollBackward`. Replay re-invokes the EXISTING `materialize_rolled_back_state` + lockstep `commit_rollback`; append-only (tag 1 reserved, tag 2 stays reserved). New variant/reason = a versioned WAL change + replay corpus (`ci_check_wal_rollback_replay_equiv.sh`, extended for `ForkChoiceWin`). |
| `WalError::RollbackTargetNotInChain` *(N-AI / DC-NODE-27)* | `ade_ledger::wal::error` (BLUE) | closed variant | Fail-closed when a rollback target is not in the durable chain. |
| `RecoveredAnchorPoint` + `RecoveredAnchorPointError` *(N-AK / DC-NODE-31)* | `ade_ledger::recovered_anchor_point` (BLUE) | closed, `RECOVERED_ANCHOR_POINT_SCHEMA_VERSION = 1` | Version-gated `array(4) [version, anchor_fp, slot, block_hash]` + SOLE codec. Separate additive record from `SeedEpochConsensusInputs`. New field = a schema-version bump. |
| `ServedChainSource` / `ServeRangeOutcome` / `CappedSlotRange` / `MAX_SERVE_RANGE_BLOCKS=256` *(N-U/N-AA / DC-NODE-13 / DC-SERVEMEM-01)* | `ade_runtime::network::{serve_dispatch, served_chain_projection}` + `chaindb::types` (RED) | closed | The READ-ONLY durable-ChainDb serve projection, bounded per request. |
| `NodeSyncItem` *(N-AI / N-AO S1 — PEER-TAGGED, DC-NODE-34)* | `ade_node::node_sync` (RED) | closed sum | `Block(Vec<u8>, peer)` \| `RollBack(Point, peer)` — N-AO S1 threaded the origin `peer` through `from_wire_pump` / `next_item` (was discarded). Transient feed type (not persisted/hashed) — no canonical/replay obligation. |
| `ReceiveClass` / `ReceiveDisposition` / `NodeSyncError` (4-variant) / `ForgeRefused::ReselectionPending` *(N-AI / DC-NODE-23/24/28/29)* | `ade_node::node_sync` (RED) | closed | The detector/resolver vocabulary; the fail-closed forge refusal while reselection pends. |
| `AdmissionPeerEvent` (incl. `RollBackward {peer, point, tip}`) *(N-AI / DC-PUMP-01)* | `ade_runtime::admission::wire_pump` (RED) | closed | The wire-pump event set — the rollback POINT preserved. The keep-alive client (N-AM) emits NONE of these (`DC-PUMP-03`). |
| `ArrayHead` / `PrevHash` / `TagEnvelopeError` / `BlockValidityError::HeaderPositionInvalid` / `SeedEpochConsensusInputs` (`epoch_nonce`, schema 2) / `LeaderCheckVerdict` / `ExpectedVrfInput` *(N-F-G / N-X / N-W / N-R-A)* | various BLUE | closed | The carried BLUE wire/forge/recovery closed surfaces. |
| `Mode` / `ForgeMode` / `VenuePolicy` / `VenueRole` / `ForgeActivation` / `NodeBlockSource` / `LoopStep` / `ForgeIntent` / `OperatorForgeMaterial` / the `live_log` + `convergence_evidence` + `rehearsal_evidence` + `ba02_evidence` vocabularies *(N-F-C … N-AJ)* | `ade_node::*` (RED + GREEN) | closed | The carried `--mode node` lifecycle / forge / evidence closed surfaces (full detail in the carried §3 history + the CODEMAP). `ForgeActivation` gained the five N-AO fork-switch RED fields — see §5. |
| Network message taxonomies (`AcceptedMiniProtocol`, per-protocol message enums, `KeepAliveMessage`) / `CardanoEra` (Byron=0…Conway=7) / `OutboundCommand` / `DispatchError` *(network / N-S-B)* | `ade_network::*` (BLUE/GREEN) + `ade_runtime::network` (RED) | closed | The frozen wire grammars. N-AM reused `KeepAliveMessage` unchanged (a wire-only client). |

### Extensible (open within constraints)

> Ade has **very few** extensible registries — the BLUE core is closed by construction. The PHASE4-N-AO `BranchBodySource` trait is the one new extension-shaped surface, and it is deliberately fenced to byte-transport only.

| Registry | Location | Extension Rule |
|----------|----------|---------------|
| **`BranchBodySource`** (the RED branch-body fetch seam) *(NEW, N-AO S4/S6 / DC-NODE-37)* | `ade_node::fork_switch` (RED) | **The ONE `Box<dyn …>` extension point this cluster introduced — and it is BYTE-ONLY, never adoption authority.** `trait BranchBodySource { fn fetch_body(&self, peer, slot) -> Result<Vec<u8>, FetchError> }`. Two impls: **`NullBranchBodySource`** (the relay-loop placeholder — serves nothing; a fork-choice win fails proof closed, the fence stays set, nothing is adopted — never a half-switch) and **`PrefetchedBranchBodies`** (CE-AO-6 — bytes the relay loop pre-fetched live via `BlockFetch RequestRange` from the winning peer; carries BYTES and nothing else — no verdict, no selection, no fence, no authority). **A new impl may supply body bytes from a new transport, but it MUST NOT short-circuit `prevalidate_branch`**: a lying / short / truncated / Byzantine fetch is rejected by `prove_fork_switch` / `prevalidate_branch` BEFORE any `commit_rollback` (`BodyHeaderMismatch` / `BodyUnavailable` / `BrokenParentLink` / `BodyInvalid`). Fenced by `ci_check_live_blockfetch_byte_only.sh` + `ci_check_fork_switch_never_abandons.sh`. **BlockFetch transports bytes; it does not grant truth.** |
| Mempool tx admission (sorted, deduplicated) | `ade_ledger::mempool` (BLUE) | New txs enter at runtime via the single `mempool_ingress` chokepoint; sort/dedup invariants preserved. |
| Peer set (`--peer`, repeatable `Vec<String>`) | `ade_node::cli` (RED) | New peers added at runtime via the CLI flag → N pumps → the deterministic `fair_merge` (N-AO S8). A peer is a transport endpoint, never an authority. |
| Served-chain read projection | `ade_runtime::network::served_chain_projection` (RED) | Bounded read-only projection of the durable ChainDb; per-request cap `MAX_SERVE_RANGE_BLOCKS=256`. Not openly extensible — a fixed bound. |

> Note: `ade_plutus` ports the `aiken_uplc` evaluator behind a quarantine boundary (pinned tag `v1.1.21`) — a frozen vendored dependency, NOT a runtime plugin registry. There are no HSM-plugin / scenario-template / federation-contract style runtime registries in Ade.

---

## 4. Version-Gated vs. Frozen Contracts

### Frozen (immutable at current version — change = new major version)

- **Wire format / encoding**: Cardano-canonical CBOR via `minicbor` + the `ade_codec` canonical primitives — field order = wire order for hash-bearing structures; **wire bytes are preserved, never re-encoded for hashing** (`ci_check_hash_uses_wire_bytes.sh`). `postcard`-style anchors do not apply; the wire format is Cardano's.
- **Tag-24 envelope**: `0xd8 0x18` CBOR-in-CBOR — the SOLE `ade_codec::cbor::tag24::{wrap_tag24, unwrap_tag24}` authority (`CN-WIRE-08/12`); no second/hand-rolled tag-24 parse anywhere.
- **Hash algorithms**: `blake2b_256` / `blake2b_224` (`ade_crypto::hash`) — immutable per version; the single body-hash recipe `block_body_hash`.
- **The header VRF / KES / DSIGN verification recipes**: `ade_crypto` (Praos VRF draft-03, the Ade-owned Sum6KES matching `cardano-base` byte-for-byte, Ed25519). **VRF strength is FROZEN — N-AN's eta0 overlay is the correct nonce, NOT a bypass; N-AO's `prevalidate_branch` reuses `block_validity` unchanged.**
- **`select_best_chain`** (the `DC-CONS-03` fork-choice contract): **FROZEN — byte-unchanged by N-AO.** k-bounded, density-free, arrival-order-independent. The single selector.
- **All 462 canonical types**: existing wire formats frozen; new types may be added (N-AO added ZERO).
- **The closed era enum** `CardanoEra` (Byron=0 … Conway=7); the closed `PrevHash = Genesis | Block(Hash32)`.
- **The durable WAL grammar**: `WalEntry` closed sum (tag 0 `AdmitBlock`, tag 1 `RollBack`, tag 3 `SeedEpochConsensusInputsImported`); tag 2 reserved. Append-only.

### Version-gated (can evolve across major versions)

- New variants in the closed `AdmissionLogEvent` convergence vocabulary (N-AO took it 8 → 22): require a `DISCRIMINATORS` allow-list entry + the two closed-vocabulary CI gates. **Observe-only — never an authority surface.**
- New closed `BranchProofError` / `MissingBridgeReason` / `RangeRefetchOutcome` discriminants: require the matching fail-closed CI gate + a `DC-NODE-37/39/41` strengthening.
- Canonical type schema additions (new fields appended; sort/dedup + version-byte invariants preserved — e.g. `SeedEpochConsensusInputs` schema 1→2, `RecoveredAnchorPoint` schema 1).
- New `WalEntry` variants / `RollbackReason` arms: a versioned WAL change + a replay-equivalence corpus.
- New `--mode node` CLI flags: must be path-PRESERVING and added to the `ci_check_node_path_fidelity.sh` allow-list (N-AO added none).
- New CI checks (existing checks may be TIGHTENED, never relaxed — the AO span repointed/extended 6 in place and added 12).

---

## 5. Module Addition Rules

How new modules enter the workspace.

| Color | Naming convention | Build-config flags | May depend on | MUST NOT depend on |
|-------|-------------------|--------------------|----------------|--------------------|
| **BLUE** | crate prefixes `ade_codec` / `ade_types` / `ade_crypto` / `ade_core` / `ade_ledger` / `ade_plutus`; plus the 9 BLUE `ade_network` submodule paths (`mux/frame.rs`, `codec/`, `handshake/`, `chain_sync/`, `block_fetch/`, `tx_submission/`, `keep_alive/`, `peer_sharing/`, `n2c/`) | `// Core Contract:` banner; `#![deny(unsafe_code, clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::float_arithmetic)]`; **no `cfg(feature)` semantic gates** (`ci_check_no_semantic_cfg.sh`) | Other BLUE modules (downward only) | Any RED/GREEN crate; `ade_runtime`/`ade_node`/`ade_core_interop`; std runtime I/O; tokio/async (`ci_check_no_async_in_blue.sh`); `pallas_*` outside `ade_plutus` |
| **GREEN** | `ade_testkit` (whole crate); GREEN-by-content sub-trees inside RED crates carry a `//! GREEN …` banner + the BLUE deny attributes | same deny attributes; purity CI gate per sub-tree | BLUE modules | RED modules in non-test deps; nondeterminism (wall-clock / rand / float / HashMap) |
| **RED** | `ade_runtime`, `ade_node`, `ade_core_interop`, `ade_network::mux::transport` | `//! RED …` banner; tokio/std/I/O allowed; key custody confined to `ProducerShell`; the `Clock` seam is the SOLE wall-clock observation reachable from a relay/orchestrator driver | Any module | — (RED is the leaf) |

**The four NEW RED `ade_node` modules PHASE4-N-AO added** (registered in `crates/ade_node/src/lib.rs`) all follow the RED rule — they live in the already-RED `ade_node` host crate, carry the `// Core Contract:` banner, and either are RED (`fork_switch`'s fetch/driver surface) or GREEN-by-content/by-function (`fork_switch::prevalidate_branch`, `selector_state`, `lca_walk`, `fair_merge` — pure, deterministic, no nondeterminism):

- `ade_node::fork_switch` — the prove core (`BranchBodySource` seam + `prevalidate_branch` pure prove + `ForkSwitchOutcome` / `BranchProofError` / `MissingBridgeReason` / `PostSwitchFollow` / `RangeRefetch`).
- `ade_node::selector_state` — the GREEN selector-state projection (`PendingForkSwitch` / `ForkAnchor` / `project_tiebreaker`).
- `ade_node::lca_walk` — the GREEN-by-fn durable-LCA fork-anchor walk (`walk_to_durable_lca` / `LcaResult` / `LcaError` / `CachedHeader`).
- `ade_node::fair_merge` — the GREEN deterministic per-peer round-robin merge.

**The `ForgeActivation` fork-switch lifecycle state (RED, `ade_node::node_lifecycle`).** `pub struct ForgeActivation<'a>` (the opt-in forge-activation bundle threaded into `run_relay_loop` as `forge: Option<&mut ForgeActivation>`) gained five N-AO RED **recovery** fields joining the carried `last_forge_refused` / `pending_reselection`:

- `pending_fork_switch: Option<PendingForkSwitch>` — the provisional S3 decision awaiting S4 prove+apply.
- `pending_missing_bridge: Option<MissingBridgeReason>` — the structured fail-closed hold (S11) — HOLDS the forge fence, refuses the silent no-op.
- `post_switch_follow: Option<PostSwitchFollow>` — the post-`ForkChoiceWin` follow target (S14) — decides re-fetch eligibility.
- `pending_range_refetch: Option<RangeRefetch>` — the pending active range re-fetch (S14), consumed by the async relay loop.
- `rollback_retention: BTreeMap<Hash32, CachedHeader>` — rolled-back branch evidence retention (S13) — fixes the LCA-walk over-fire.

**These are RED recovery / sequencing state, NEVER selection authority.** The forge never builds on a stale pre-resolution local tip (`pending_reselection` / `pending_fork_switch` / `pending_missing_bridge` all HOLD the `DC-NODE-28` fence via `pending_reselection_forge_refusal` → `ForgeRefused::ReselectionPending`). None of these fields decides which branch wins — S3's routed `select_best_chain` call already did. `Some` on any of them refuses forging; the fence is cleared ONLY after `apply_chain_event` (or, for `pending_range_refetch`, only on `RangeRefetchOutcome::Admitted`). Fenced by `ci_check_live_fork_choice_wiring.sh` + `ci_check_fork_switch_never_abandons.sh` + `ci_check_post_switch_convergence_window.sh` + `ci_check_missing_bridge_refetch.sh` + `ci_check_rollback_retention_evidence.sh`.

### New module checklist

1. Add to the Cargo workspace `[workspace] members` (N-AO added NO crate — the four new modules are sub-modules of the existing `ade_node` crate).
2. Apply color-specific banner + lints (BLUE: `// Core Contract:` + deny attributes + no-semantic-cfg; GREEN: `//! GREEN` + deny attributes + purity gate; RED: `//! RED`).
3. `ci_check_dependency_boundary.sh` rejects forbidden cross-color imports; `ci_check_module_headers.sh` enforces the banner.
4. New canonical types: structural-grep-counted from the BLUE trees; add round-trip tests (N-AO added none — its new types are RED/GREEN, not canonical-counted).
5. A new selector / adoption / rollback path is **forbidden** — route into the existing `select_best_chain` / `pump_block` / `materialize_rolled_back_state`.

### CI gates that enforce the boundary (**173 total**)

Cross-cutting BLUE gates (scope the full BLUE set): `ci_check_module_headers.sh`, `ci_check_forbidden_patterns.sh`, `ci_check_dependency_boundary.sh`, `ci_check_no_signing_in_blue.sh`, `ci_check_no_semantic_cfg.sh`, `ci_check_hash_uses_wire_bytes.sh`, `ci_check_ingress_chokepoints.sh`, `ci_check_pallas_quarantine.sh`, `ci_check_no_async_in_blue.sh`. Fork-choice / selector gates (carried + N-AO): `ci_check_no_density_in_fork_choice.sh`, `ci_check_chain_selection_arrival_order_independent.sh` (`CN-CONS-01`), `ci_check_consensus_closed_enums.sh`, `ci_check_rollback_materialize_closure.sh`, `ci_check_rollback_target_canonical_binding.sh` (`DC-NODE-29`), `ci_check_wal_rollback_replay_equiv.sh` (extended for `ForkChoiceWin`), `ci_check_live_fork_choice_apply.sh`, `ci_check_live_fork_choice_wiring.sh`, `ci_check_wire_rollback_signal_preserved.sh`. **PHASE4-N-AO added 12 gates** (161 → 173): `ci_check_peer_identity_preserved.sh` (`DC-NODE-34`), `ci_check_candidate_construction_validated.sh` (`DC-NODE-35`), `ci_check_live_selector_dispatch.sh` (`DC-NODE-36`), `ci_check_fork_switch_never_abandons.sh` (`DC-NODE-37`), `ci_check_lca_anchor_walk.sh` (`DC-NODE-38`), `ci_check_wire_pump_fairness.sh` (`DC-PUMP-04`), `ci_check_fork_choice_evidence_closed.sh` (`DC-EVIDENCE-04`), `ci_check_live_blockfetch_byte_only.sh` (the `BranchBodySource` byte-only fence), `ci_check_post_switch_convergence_window.sh` (`DC-EVIDENCE-04/05`), `ci_check_rollback_retention_evidence.sh` (`DC-NODE-40`), `ci_check_missing_bridge_fail_closed.sh` (`DC-NODE-39`), `ci_check_missing_bridge_refetch.sh` (`DC-NODE-41`). The convergence-vocabulary gate `ci_check_convergence_evidence_vocabulary_closed.sh` was repointed/extended for the broadened vocabulary.

---

## 6. Forbidden Patterns (per color)

Universal IDD prohibitions per color (from `~/.claude/methodology/idd.md` Part IV):

- **BLUE:** no clock, rand, raw HashMap/HashSet, float, env access, network/filesystem, async runtime, locale-dependent ops, OS-dependent ordering.
- **GREEN:** no nondeterminism; no participation in authoritative outputs.
- **RED:** no direct mutation of BLUE state; no unsafe construction of semantic types; no bypassing canonical validation.

### Project-specific additions (Ade) — PHASE4-N-AO

- **No second chain selector.** `select_best_chain` is routed-to, never duplicated; no parallel preference, density ordering, or operator heuristic. A competing candidate set goes into the ONE selector.
- **No RED-minted candidate summary may reach `select_best_chain`.** Candidate fragments are derived ONLY from `validate_and_apply_header` output (the `chain_selector::process_header_arrival` validate-then-fragment pattern), never the `follow.rs` mint (`DC-NODE-35`).
- **NEVER commit a rollback of the current durable chain until the replacement branch's bodies are fetched, linked, and validated as a complete candidate branch** (`DC-NODE-37` — the H-1 class at fork-choice scale). A failed / lying / incomplete / Byzantine winner leaves ChainDb / ledger / chain_dep byte-unchanged.
- **`pump_block` stays the sole roll-forward durable admit** — no header-only tip advance; a fork-choice win is provisional until bodies apply.
- **The fork-anchor rollback target binds Ade's durable stored slot+hash** (`DC-NODE-29`) — never peer-supplied, never mixed peer/local authority. `winner_tip` is a fetch endpoint only, not adoption authority.
- **`BranchBodySource` carries BYTES only.** No impl may short-circuit `prevalidate_branch`; a lying/short/truncated fetch is rejected before any `commit_rollback`. `NullBranchBodySource` is the fail-closed fence.
- **A `MissingBridge` is a structured fail-closed outcome only** — never an adoption path, rollback target, candidate anchor, fence-clear, skip-the-missing-parent, or trust-the-later-block (`DC-NODE-39`). No durable mutation on that path.
- **The `AdmissionLogEvent` convergence vocabulary is a CLOSED enum** — no open/wildcard variant; observe-only; emitting it affects no authority; every fork-choice win pairs to exactly one terminal (`DC-EVIDENCE-04`).
- **Venue stays explicit + closed** — only `Participant` reaches SELECT; `SingleProducer` / `Unknown` fail closed.
- Carried: no second block-envelope encoder; no second `leader_vrf_input` authority; no second `wrap_tag24`/`unwrap_tag24` or hand-rolled tag-24 parse; no forward-sync `AdvanceTip` before durability; no Mithril/genesis bootstrap bypassing the single `bootstrap_initial_state`; no internal-fingerprint-vs-Haskell-hash equality assertion; no registry rule citing a non-existent `code_locus`; no second eta0-overlay authority (only `PraosChainDepState::overlay_recovered_eta0`); no rollback-replay VRF bypass / skip / loosening.

---

## 7. Candidate & Not-Yet-Wired Seams (declared follow-ons — NOT closed)

> Surfaced for confirmation, not asserted wired. Items the user should confirm.

- **`chain_selector::process_rollback` (the orchestrator rollback path) — was SEAMS candidate #10 (test-only).** PHASE4-N-AO wired the live multi-candidate SELECT path through `node_lifecycle::apply_fork_switch` + the EXISTING `select_best_chain` + `apply_chain_event` (`RolledBack + ChainSelected`). **CONFIRM with the user whether `chain_selector::process_rollback` is now reachable live or remains test-only** — if S3/S4 reuse/extend it live it must carry the `DC-NODE-29` binding (the N-AI forward-OQ). The slice docs route through `apply_chain_event`, suggesting `process_rollback` stayed test-only, but this is a candidate to verify against the final S3/S4 wiring.
- **Full Cardano ChainSel (N>2 peers, adversarial load)** — preprod rung-3, explicitly out of scope here. PHASE4-N-AO proved the hermetic SELECT mechanism (CE-AO-1…5) and, on the committed CE-AO-6 transcript, the exercised two-producer live venue (flipping `CN-CONS-03`). It does NOT prove full Cardano ChainSel. `RO-LIVE-01` stays operator-gated (ADE1 stake ~epoch 295).
- **The keep-alive SERVER/responder** (N-AM shipped the CLIENT only) — a CE-AM-LIVE-gated follow-on.
- **Multi-epoch rollback nonce-evolution** — N-AN's `T-REC-06` covers the recovered SEED epoch (no epoch-boundary crossing in the follow window); a multi-epoch rollback is a named out-of-scope follow-on.

### Operator-pass execution gates (schema enforced, execution context-gated)

- **CE-AO-6 / `CN-CONS-03`** — FLIPPED to `enforced` on the committed natural CE-AO-6 transcript at the cluster close (`862cd2cb`). The transcript proves multi-candidate SELECT + adopt in the exercised two-producer venue (full chain → `fork_switch_applied{fork_choice_win}` → `agreement_verdict{agreed}` at the adopted tip, 0 diverged). It does NOT claim full Cardano ChainSel.
- **`RO-LIVE-01`** — stays operator-gated (preprod, ADE1 stake ~epoch 295). N-AO did NOT flip it.

---

## Generation notes

- Generated by `/seams` at HEAD `862cd2cb` (PHASE4-N-AO CLOSE), reading `docs/ade-CODEMAP.md` (pinned `b8860b16` — **ONE cluster stale**, missing the four new AO modules + the AO rows; module COLORS still accurate) and `docs/ade-invariant-registry.toml` (**372 rules** at HEAD — the canonical count source, holding all 11 new AO rules + the `CN-CONS-03` flip).
- This regeneration FOLDS the prior 5785-line append-stratified document into a tight AO-current structure rather than appending another stratum (the methodology: "regenerated, not hand-edited"). All load-bearing closed-registry rows, the §2 domain boundaries, and the §4–§6 contracts are preserved; the deep per-surface narrative history of the N-F-* / N-Q / N-R-* / N-S-* / N-U … N-AN spans remains recoverable from the CODEMAP + the invariant registry + the archived cluster docs under `docs/clusters/completed/`.
- **Counts at this HEAD:** 11 crates / 462 canonical types / 173 CI checks / 372 registry rules (239 enforced / 19 partial / 113 declared / 1 enforced_scaffolding).
- **Candidates surfaced for human confirmation** (NOT auto-included as wired): the live reachability of `chain_selector::process_rollback` (§7); whether any future `BranchBodySource` impl beyond `PrefetchedBranchBodies`/`NullBranchBodySource` is planned.
