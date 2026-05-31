# Ade — HEAD Deltas (Changes Since Baseline)

> **Status:** Living architectural document. Regenerated; not hand-edited.
>
> Regenerate with `/head-deltas <baseline>` after every cluster close. Baseline is recorded in `.idd-config.json` `head_deltas_baseline`.

> Baseline: `a3a0636` (no tag, 2026-05-30 13:25:21 +0700)
> HEAD: `71a6c80` (PHASE4-N-F-C L6 — BA-02 peer-acceptance evidence manifest, 2026-05-31 13:36:51 +0700)
> 23 commits, 39 files changed, +6713 / -495 lines

This window narrates the **PHASE4-N-F-C cluster** — *"Build the real Ade node lifecycle"* — preceded by a short **two-commit post-N-F-A tail** (the N-F-A archive commit + a CI-workflow removal). The prior baseline (`a3a0636`) was the PHASE4-N-F-A close point; per the per-cluster bump model the N-F-A cluster body is now archived in git history + its cluster docs, and this doc narrates only the post-`a3a0636` span.

The window is two pieces:

1. The **post-N-F-A tail** (`4b0eed6`, `1d29597`, 2 commits) — `4b0eed6` archives the PHASE4-N-F-A capability cluster (moves `docs/clusters/PHASE4-N-F-A/` → `docs/clusters/completed/PHASE4-N-F-A/`); `1d29597` removes the non-gating `.github/workflows/notify-atlas.yml` dispatch workflow. **No `crates/**/*.rs` behavior change, no new invariant rule.** Narrated in **§8**.
2. The **PHASE4-N-F-C cluster** (`5403468` invariant sketch → `71a6c80` L6, 21 commits) — the BA-02 wiring the N-F-A capability cluster deferred. One new `--mode node` lifecycle owner that composes the existing closed seams end-to-end: first-run **Mithril-only** bootstrap → persist → **WAL-only** warm-start recovery → forward-sync → **forge-from-recovered** → real-peer acceptance evidence. **Three new RED/GREEN modules in `ade_node`, one new `ade_runtime::chaindb` accessor, three new CI gates (+ three modified), three new invariant rules. No BLUE crate changed.** Narrated in §§2–7.

> **Generation state — this is a near-complete close, the inverse of the N-F-A dry-run.** At HEAD `71a6c80`, **all source, all CI scripts, and all cluster/slice/planning docs are committed**. The working tree carries only the staged close-pass for the grounding docs + registry:
> - `docs/ade-invariant-registry.toml` — the **registry promotion** (the three new rules `CN-CINPUT-03` / `DC-CINPUT-02b` / `RO-LIVE-06` + ten carry-forward `strengthened_in` appends + two status flips — `CN-STORE-02` `declared`→`partial` and `DC-CINPUT-01` `partial`→`enforced`): **303 → 306**.
> - `docs/ade-CODEMAP.md` — the CODEMAP regeneration cataloguing the three new modules.
> - This `HEAD_DELTAS` and the SEAMS / TRACEABILITY refreshes are part of the same staged close-pass.
>
> `git show HEAD:docs/ade-invariant-registry.toml` is still **303** (committed HEAD has **no** N-F-C rules); the committed CODEMAP / SEAMS / TRACEABILITY carry **no** N-F-C content. The committed-vs-working-tree counts are reconciled inline in §7 and flagged in the §0 anomaly box. **The cluster close should commit the staged registry + four grounding docs together**, then re-bump `.idd-config.json` `head_deltas_baseline` from `a3a0636` to **`71a6c80`** (the orchestrator is handling the config edit separately — see §0).

> **Cluster framing — lifecycle mechanics proven through evidence closure; NOT a live BA-02 claim (load-bearing; do not over-read).** **PHASE4-N-F-C proves the Ade node lifecycle mechanics through evidence closure. It does not claim live BA-02. RO-LIVE-01 remains partial/operator-gated. RO-LIVE-06 is only schema/correlation mechanics.** The cluster ships the single `--mode node` owner and mechanically fences that (a) first-run bootstrap is Mithril-only with no genesis/bundle/cold fallback, (b) warm-start recovery is WAL-only, (c) the forward-sync path drives durable validated apply through the one `pump_block` seam, (d) the producer forges from the WAL-recovered `SeedEpochConsensusInputs` (CN-CINPUT-03 / DC-CINPUT-02b), not an operator bundle, and (e) the BA-02 evidence manifest is a closed, correlation-correct, lie-proof schema (RO-LIVE-06). What it does **not** do is execute a live operator pass against a real peer — that remains `RO-LIVE-01` partial/operator-gated.

---

## 0. Anomalies & Cross-Reference Warnings (surface prominently)

Recorded so a reader does not mistake an intentional change for a defect.

| Item | Class | Disposition |
|------|-------|-------------|
| `.github/workflows/notify-atlas.yml` **removed** (42 lines) | Intentional tail cleanup (`1d29597`) | A non-gating grounding-doc → `ade-atlas` rebuild **dispatch** workflow, retired because *"ade-atlas now polls every 10 min."* It was never an Ade invariant gate (it is not under `ci/`, and `.idd-config.json` `ci_dirs = ["ci"]` excludes `.github/workflows` — the config's `_ci_dirs_doc` already noted it "is not an Ade invariant gate"). **Net `ci/ci_check_*.sh` enforcement-gate count is unaffected by this removal** (that count moves separately, **105 → 108**, on the three new N-F-C gates — see §5). Not a lost Ade gate. |
| Registry rule count **303 (committed HEAD) → 306 (working tree)** | Expected (staged close-pass) | The three N-F-C rules (`CN-CINPUT-03`, `DC-CINPUT-02b`, `RO-LIVE-06`) + the ten `strengthened_in` appends + the two status flips are **uncommitted in the working tree** and land in the cluster-close commit. `git show HEAD:…registry.toml` = 303 by design. Reconciled in §7. |
| `DC-CINPUT-01` status flips `partial` → **`enforced`** | Status strengthening (not a removal) | N-F-A left `DC-CINPUT-01` `partial` precisely because the *production* restart path was unwired (its open obligation, deferred to N-F-C). N-F-C's `--mode node` warm-start arm threads exactly that path (`node_lifecycle.rs::warm_start_recovery` → `bootstrap_initial_state(RequiredFromRecoveredProvenance)`), so the open obligation is cleared and the rule promotes to `enforced`. A strengthening, surfaced here because it is a status change (expected, not anomalous). |
| Committed HEAD grounding docs (`CODEMAP` / `SEAMS` / `TRACEABILITY`) carry **no** N-F-C content | Expected (in-flight close) | The CODEMAP refresh is **staged** in the working tree (committed: 0 hits for `node_lifecycle`; working-tree: 18). The SEAMS / TRACEABILITY refreshes + this `HEAD_DELTAS` are part of the same close-pass. Once committed together, all four docs + the registry are coherent at the close SHA. **Until then the committed docs are stale w.r.t. N-F-C** — the normal pre-close state, not a defect. |
| `head_deltas_baseline` in `.idd-config.json` still reads `a3a0636` (the *prior* baseline) | **Recommended config bump (orchestrator action; NOT edited by this doc)** | This refresh narrates `a3a0636 → 71a6c80`; at close the config should bump `head_deltas_baseline` `a3a0636` → **`71a6c80`** and update its `_head_deltas_baseline_doc`. Also: the `_invariant_registry_doc` count string still says **"303 entries at HEAD"** — stale; should read **306** once the staged registry is committed. This doc only **recommends** the bumps; it writes no config. |
| **N-F-A append-only concern (prior window) — REPAIRED this window** | Resolved | The prior HEAD_DELTAS flagged that the staged N-F-A registry had **replaced** `T-REC-01`/`T-REC-02` `strengthened_in` `["PHASE4-N-R-A"]` with `["PHASE4-N-F-A"]` (dropping N-R-A). The committed registry now reads `["PHASE4-N-R-A", "PHASE4-N-F-A"]`, and the N-F-C close **appends** `"PHASE4-N-F-C"` → `["PHASE4-N-R-A", "PHASE4-N-F-A", "PHASE4-N-F-C"]`. Every N-F-C `strengthened_in` change this window is a verified append (no list replaced). Append-only discipline is intact; the earlier concern is closed. |

No canonical-type removals. No invariant-rule removals (the registry is `+3 / −0`; both status flips are *strengthenings*, not removals). Zero commits without a conventional-commits prefix.

---

## 1. Commit Log

Verbatim from `git log --oneline --no-merges a3a0636..HEAD`, newest-first. Type is the conventional-commits prefix on the subject; no editorial. (History uses no merge commits in this span — `--merges` is empty.)

| Hash | Type | Summary |
|------|------|---------|
| `71a6c80` | feat | PHASE4-N-F-C L6 — BA-02 peer-acceptance evidence manifest |
| `9e3fae2` | docs | add PHASE4-N-F-C L6 slice doc |
| `f32598b` | feat | PHASE4-N-F-C L5 — forge from recovered state |
| `866f2c9` | docs | add PHASE4-N-F-C L5 slice doc |
| `0df63db` | feat | PHASE4-N-F-C L4c — recover synced tip via warm-start |
| `7f6ce62` | docs | amend L4 doc for L4c warm-start block-bytes extension |
| `de9c6b5` | feat | drive durable validated apply via pump_block |
| `263264d` | feat | add verdict-decoupled block source |
| `bc10618` | docs | add PHASE4-N-F-C L4 slice doc |
| `450cd46` | fix | PHASE4-N-F-C — add --mithril-manifest-path CLI parse arm |
| `c79c4a8` | feat | PHASE4-N-F-C L3 — production warm-start recovery |
| `214715b` | docs | add PHASE4-N-F-C L3 slice doc |
| `ddc84be` | feat | PHASE4-N-F-C L2 — Mithril-only first-run bootstrap |
| `80498f9` | docs | add PHASE4-N-F-C L2 slice doc (Mithril-only first-run bootstrap) |
| `4b761e0` | feat | PHASE4-N-F-C L1 — --mode node lifecycle owner skeleton + branch |
| `9dac0f8` | docs | align PHASE4-N-F-C invariant sketch with the revised plan |
| `0f4a8f1` | docs | replace PHASE4-N-F-C plan with the real Ade node lifecycle |
| `555d064` | docs | add PHASE4-N-F-C C1 slice doc |
| `fef652a` | docs | add PHASE4-N-F-C cluster doc |
| `abeafdb` | docs | add PHASE4-N-F-C cluster/slice plan |
| `5403468` | docs | add PHASE4-N-F-C invariant sketch |
| `1d29597` | ci | remove notify-atlas (ade-atlas now polls every 10 min) |
| `4b0eed6` | docs | archive PHASE4-N-F-A capability cluster |

Type histogram: **docs ×13, feat ×8, ci ×1, fix ×1**. **Unclassified by prefix: 0** — every commit carries a conventional-commits prefix. The eight `feat` + one `fix` commits are the only source-bearing commits (L1 `4b761e0`, L2 `ddc84be` + the L2 CLI fix `450cd46`, L3 `c79c4a8`, L4a `263264d` / L4b `de9c6b5` / L4c `0df63db`, L5 `f32598b`, L6 `71a6c80`); all `docs` are cluster/slice/planning docs (plus the `4b0eed6` N-F-A archive move); the one `ci` is the notify-atlas removal.

(`4b0eed6` and `1d29597` are the post-N-F-A tail — they sit at the very start of this window because the baseline `a3a0636` is their predecessor. `4b0eed6` touches only `docs/clusters/.../PHASE4-N-F-A/` (a rename to `completed/`) and carries no N-F-C content; `1d29597` removes the non-gating dispatch workflow. See §8.)

---

## 2. New Modules

Three modules added this window — all new files in `ade_node`, all PHASE4-N-F-C. Colors per the module doc-comment self-classification and the project TCB vocabulary. **No BLUE crate changed this window** — every authoritative step routes through an *existing* closed seam (`bootstrap_initial_state`, `forward_sync::pump_block`, the BLUE `PoolDistrView::from_seed_epoch_consensus_inputs` projection, the existing `run_real_forge` engine); the new modules are the RED orchestration + GREEN evidence that compose them.

| Module | Color | Purpose | Key sub-paths | Added in (cluster/slice) |
|--------|-------|---------|---------------|--------------------------|
| `ade_node::node_lifecycle` | **RED** | The single `--mode node` **lifecycle owner** (the `PHASE4-N-F-C-LIFECYCLE-OWNER` marker module) — the BA-02 wiring N-F-A deferred. It opens a persistent `ChainDb` + `FileWalStore`, classifies **first-run** (empty store) vs **warm-start** (non-empty) as a **pure function of on-disk state** (`classify_start`), then: first-run → **Mithril-only** bootstrap (`bootstrap_from_mithril_snapshot`, its first non-test caller — fail-closes on `verify_mithril_binding` before any state is admitted, persists the seed-epoch sidecar + WAL provenance under one `BootstrapAnchor`); warm-start → production recovery (`warm_start_recovery` → `replay_from_anchor` → `bootstrap_initial_state(RequiredFromRecoveredProvenance)`). It *orchestrates* but does not *define* truth: initial state flows **only** through the single `bootstrap_initial_state` authority. **No genesis branch, no `--consensus-inputs-path`-as-forge-input, no tip-bundle, no cold-`produce_mode` fallback, no native Mithril UTXO-HD/LedgerDB decode** — fully fail-closed (CN-NODE-01 strengthened; `ci_check_lifecycle_owner_uses_bootstrap_initial_state.sh`). | `node_lifecycle.rs` (`run_node_lifecycle`, `classify_start`, the FirstRun/WarmStart branch, `warm_start_recovery`, `EXIT_NODE_LIFECYCLE_UNWIRED`) | PHASE4-N-F-C / `4b761e0` (L1) → `c79c4a8` (L3) → `0df63db` (L4c) |
| `ade_node::node_sync` | **RED** | The **forward-sync driver + forge-from-recovered handoff** for the lifecycle owner (L4 + L5). `run_node_sync` consumes a **verdict-decoupled** block-bytes source (L4a — one ordered source: a single peer's `run_admission_wire_pump` event stream, or a deterministic in-memory feed; it yields ONLY block bytes, never an agreement/tip/follow verdict) and applies each through the SAME `forward_sync::pump_block(...)` authority the wire layer uses (L4b — durable `StoreBlockBytes` + `AppendWal` **before** `AdvanceTip`), failing closed on an undecodable block. **No second apply path; not a follower** (`ade_core_interop::follow` is not validating sync). L5's `forge_one_from_recovered` projects the leadership `PoolDistrView` via `PoolDistrView::from_seed_epoch_consensus_inputs(recovered)` and drives the reused `run_real_forge` engine with eta0 from the recovered `chain_dep` — failing closed (`MissingRecoveredConsensusInputs`) with **no** operator-bundle / cold-`InMemoryChainDb` / `--consensus-inputs-path` fallback (CN-CINPUT-03 / DC-CINPUT-02b). | `node_sync.rs` (`run_node_sync`, the verdict-decoupled `BlockSource`/`NodeBlockSource`, the `pump_block`-driven sync loop, `forge_one_from_recovered`) | PHASE4-N-F-C / `263264d` (L4a) / `de9c6b5` (L4b) / `0df63db` (L4c) / `f32598b` (L5) |
| `ade_node::ba02_evidence` | **GREEN** | The closed, versioned **BA-02 peer-acceptance evidence manifest + correlator** (L6). GREEN *evidence*, not authority: `correlate` is the **sole** `Ba02Manifest` constructor; it COMPARES two already-authoritative outputs — the BLUE-minted forged-block hash (read from `ForgedBlockArtifact.{hash,slot}`, **never recomputed**; block bytes never parsed) and an operator-captured peer-accept signal — and emits a closed `BA02Outcome`. **Hash-primary, lie-proof:** the forged-block hash is the required correlation key; a peer signal's slot is optional context that must AGREE when present (present-but-different → `NoEvidence`); conflicting peer signals → `NoEvidence`; and the peer-accept-log parser is **allow-list only** (`peer_served_block` / `peer_chain_tip`), dropping every weaker/self/unknown/malformed line — `ForgeSucceeded` / `self_accept` / `block_received` / a lagging-or-diverged `agreement_verdict` are **never** coerced to acceptance. Two ranked signal forms: `PeerAcceptEvent::PeerServedBlock` (strongest — peer re-served the forged block) and `PeerAcceptEvent::PeerChainTip` (corroborating). **Schema + correlation mechanics ONLY — does NOT assert a live BA-02 pass occurred** (RO-LIVE-06; live BA-02 stays operator-gated, RO-LIVE-01 partial). | `ba02_evidence.rs` (`Ba02Manifest`, `BA02Outcome`, `PeerAcceptEvent` {`PeerServedBlock`, `PeerChainTip`}, `NoEvidenceReason`, `parse_peer_accept_events`, `correlate`) | PHASE4-N-F-C / `71a6c80` (L6) |

In addition, **`ade_runtime::chaindb` gained one new accessor** — `list_seed_epoch_consensus_anchor_fps(&self) -> Result<Vec<Hash32>, ChainDbError>` (enumerate the anchor fingerprints under which seed-epoch consensus-input sidecars are keyed; reads the `SEED_CINPUTS_BY_ANCHOR_FP` table), added across `mod.rs` (trait) + `in_memory.rs` + `persistent.rs` impls for the L3 warm-start anchor-lineage discovery. This is a new surface on an existing module, catalogued in §3.

**Cross-reference (CODEMAP):** the **committed** `docs/ade-CODEMAP.md` at HEAD does **not** yet catalogue any of these (0 hits for `node_lifecycle` / `node_sync` / `ba02_evidence`). The **working-tree** CODEMAP (staged close-pass) **does** — 18 hits `node_lifecycle`, 26 `node_sync`, 26 `ba02_evidence`. **Action:** commit the staged CODEMAP refresh with the cluster close; until then the committed CODEMAP is stale w.r.t. N-F-C (expected pre-close state). When committed, verify `node_lifecycle` + `node_sync` appear in CODEMAP §RED, `ba02_evidence` in §GREEN, and that the `ade_runtime::chaindb` row notes the new `list_seed_epoch_consensus_anchor_fps()` accessor.

No new corpus / non-source artifacts this window (no BA-02 live transcript is committed — consistent with RO-LIVE-01 remaining operator-gated; RO-LIVE-06 covers only the manifest *mechanics*, and `ci_check_ba02_evidence_closed.sh` actively asserts no `docs/evidence/*ba02*` manifest is committed).

---

## 3. Modules Modified

Modules that existed at baseline with non-trivial changes. Grouped by cluster/slice; commit-by-commit paraphrase is avoided.

| Module | Scope | Key changes |
|--------|-------|-------------|
| `ade_node::cli` | +104 / −3 (`cli.rs`) | **N-F-C L1 + L2 + the L2 fix:** adds the fifth closed CLI mode `Mode::Node` (`--mode node`) and its dispatch, plus the `--mithril-manifest-path` parse arm (first-run bootstrap Mithril-provenance input — added in the dedicated CLI fix `450cd46` after L2 left the field present but unreachable from real argv, with a `node_cli_parses_mithril_manifest_path_from_argv` test). The closed `--mode` set is now exactly `{ wire_only, admission, key_gen_kes, produce, node }`, dispatch total (no catch-all). Fenced by the repaired `ci_check_node_mode_closure.sh` (§5). (Commits: `4b761e0`, `ddc84be`, `450cd46`.) |
| `ade_node` crate wiring (`lib.rs`, `main.rs`) | +13 / −1 | **N-F-C L1/L6:** `lib.rs` declares the three new modules (`pub mod node_lifecycle; pub mod node_sync; pub mod ba02_evidence;`) and re-exports `run_node_lifecycle` + `EXIT_NODE_LIFECYCLE_UNWIRED`; `main.rs` routes `Mode::Node` to `run_node_lifecycle(cli, shutdown_rx)` (dropping the wire-only JSONL writer first so no stray empty log lingers). Pure wiring. |
| `ade_runtime::chaindb` | +36 / −0 across `mod.rs` (+12), `persistent.rs` (+18), `in_memory.rs` (+6) | **N-F-C L3:** adds the `list_seed_epoch_consensus_anchor_fps()` accessor (enumerate anchor-fp-keyed seed-epoch sidecars) across the `ChainDb`/`SnapshotStore` trait + persistent + in-memory impls — the warm-start recovery path uses it to discover the seed-epoch anchor lineage and **fail closed on multiple/mismatched anchors**. Additive `SnapshotStore` surface; the keyed sidecar remains disjoint from slot-keyed snapshots (the N-F-A `CN-STORE-02` disjointness invariant, now strengthened `declared` → `partial` — see §7). The doc-comment is explicit that finding an `anchor_fp` here is *not* proof the sidecar is valid (that is the warm-start verify chain's job). (Commit: `c79c4a8`.) |
| `ade_node::tests::wire_only_loopback` | +1 / −0 | **N-F-C:** one-line test touch (loopback wiring follow-through for the verdict-decoupled `BlockSource`). Trivial; noted only for completeness. |

> **`ade_node::produce_mode` is deliberately untouched this window** (no diff). N-F-C does **not** reroute the standalone `--mode produce` cold-start path (which still consumes `--consensus-inputs-path` — the N-F-A `CN-CINPUT-02` forge-time fence; `produce_mode` stays a diagnostic mode passing `SeedEpochConsensusSource::NotRequired`). The producer **consumption** of the recovered surface (CN-CINPUT-03 / DC-CINPUT-02b) is added on the **new** `--mode node` forge-from-recovered path inside `node_sync.rs::forge_one_from_recovered`, leaving the operator-bundle `produce` path intact and separately fenced. That is why §3 shows no `produce_mode` diff.

### Strengthenings recorded this window (staged registry `strengthened_in`)

Not new rules — **ten** cross-cutting invariant strengthenings PHASE4-N-F-C carries forward (full list + evidence in §7). **All ten are in the *uncommitted* close-pass registry, not committed HEAD, and every one is a verified append** (no `strengthened_in` list replaced; append-only discipline intact). In brief: `T-REC-01` / `T-REC-02` (warm-start synced-tip recovery is replay-equivalent and replay-derivable), `CN-STORE-02` (recovered-artifact anchor-lineage binding now mechanically exercised), `CN-NODE-01` (the lifecycle owner is the single first-run-vs-warm-start owner routing through the one bootstrap authority), `DC-WAL-03` (`replay_from_anchor` now has its first production caller), `DC-FORGE-01` / `DC-SYNC-01` (the lifecycle is the first production driver of `run_real_forge` from recovered state and of `forward_sync::pump_block`), `CN-CINPUT-02` (its consume-side open obligation is now CLOSED by CN-CINPUT-03), and `DC-CINPUT-01` / `DC-CINPUT-02a` (the warm-start verification capability and the projection are now driven from the production path — `DC-CINPUT-01` additionally flips `partial` → `enforced`).

---

## 4. Feature Flags

No feature-flag deltas this window. **No `Cargo.toml`** (workspace root or any member) was modified between `a3a0636` and `71a6c80`, so no `[features]` table, `optionalDependencies`, build tag, or `extras_require` changed. No `compile_error!`-coupled flag was introduced or removed.

---

## 5. CI Checks

Every CI check added or materially modified since baseline. Enforcement gates live as `ci/ci_check_*.sh`. Count of `ci/ci_check_*.sh`: **105 → 108** (net **+3**: three new N-F-C gates; three modified N-F-C gates; the `notify-atlas.yml` removal is a non-gating `.github/workflows` dispatch workflow and is **not** in this count — see §0/§8).

### PHASE4-N-F-C checks (new)

| Check | Status | What it checks |
|-------|--------|----------------|
| `ci_check_lifecycle_owner_uses_bootstrap_initial_state.sh` | **New** (`4b761e0` L1, extended L3 `c79c4a8`) | **CN-NODE-01 (single-authority) + DC-CINPUT-01 (production warm-start).** Asserts exactly **one** module carries the `PHASE4-N-F-C-LIFECYCLE-OWNER` marker, and that the owner obtains initial state **solely** via the single `bootstrap_initial_state` authority — the FirstRun arm indirectly (via `bootstrap_from_mithril_snapshot(`), the WarmStart arm directly (calling `bootstrap_initial_state(` and constructing `RequiredFromRecoveredProvenance`). Fences out every fallback (no `InMemoryChainDb`, no `materialize_rolled_back_state(`, no `bootstrap_from_conway_genesis`, no cold `genesis_initial: Some`, no `seed_graft`, no `tip_bundle`), contains the `RequiredFromRecoveredProvenance` constructor to owner+authority, and forbids the L3 overclaim `recover_node_state(` (which would pass `NotRequired` and not recover the sidecar). Comments + `#[cfg(test)]` stripped before the negative greps. |
| `ci_check_node_sync_via_pump.sh` | **New** (`263264d` L4a / `de9c6b5` L4b) | **DC-SYNC-01 (sync-driver containment).** Asserts the `--mode node` sync path advances the recoverable tip **only** via `pump_block` — not merely that `pump_block` exists somewhere. Isolates the `run_node_sync` function body (so it does not see L5's `forge_one_from_recovered` handoff in the same file), then: (pos) the body calls `pump_block(`; (neg) no follower-as-sync (`ade_core_interop` / `follow(`), no verdict-as-sync (`derive_verdict` / `run_admission(`), no manual tip advance (`.put_block(` / `AdvanceTip` / `rollback_to_slot(`), and no forge/cold/bundle on the sync path (`run_real_forge` / `InMemoryChainDb` / `consensus_inputs_path`). So peer tip-agreement can never masquerade as validating sync. |
| `ci_check_ba02_evidence_closed.sh` | **New** (`71a6c80` L6) | **RO-LIVE-06 (evidence honesty).** A `Ba02Manifest` must be constructible **only** from `correlate`'s exact forged-hash ↔ peer-accept match. Production code only (the `#[cfg(test)]` module + line/doc comments — which legitimately name the forbidden signals while explaining their exclusion — are stripped first). Guards: (pos) the module defines both peer-accept signal forms (`PeerServedBlock` + `PeerChainTip`) and a `correlate` fn; (g1) the `Ba02Manifest { … }` struct-literal constructor appears **exactly once** in production, inside `correlate`; (g2) no self-evidence token (`ForgeSucceeded` / `self_accept` / `block_received` / `agreement_verdict` / `"agreed"`) is used as an acceptance source; (g3) **no committed `docs/evidence/*ba02*` manifest exists** — a real manifest requires a real operator-captured peer log; L6 commits none. |

### PHASE4-N-F-C checks (modified)

| Check | Status | What it checks |
|-------|--------|----------------|
| `ci_check_node_mode_closure.sh` | **Modified** (`4b761e0`/`ddc84be`, +47/−40) | **Repaired to a 5-variant closed set** (it had gone stale-RED on `main`, still pinning the old two-variant `{ WireOnly, Admission }`). Now asserts the `Mode` sum's variant set is **exactly** `{ WireOnly, Admission, KeyGenKes, Produce, Node }` (doc-comments between variants ignored), that `Mode` is not `#[non_exhaustive]`, and that `main.rs`'s `match cli.mode` covers every variant by name with **no wildcard arm**. Backs CN-NODE-01. |
| `ci_check_bootstrap_closure.sh` | **Modified** (`c79c4a8`/L1, +15/−4) | **Repaired for the N-F-A `BootstrapState` return shape** (it had gone stale-RED against the old `(LedgerState, PraosChainDepState, Option<ChainTip>)` triple). Now asserts `bootstrap_initial_state` returns `Result<BootstrapState, _>` and that `BootstrapState` still carries the `ledger` / `chain_dep` / `tip` fields (no silent field drop) plus the warm-start branch calling the materialize authority. Backs CN-NODE-01 (the bootstrap single-authority rule). |
| `ci_check_consensus_input_provenance.sh` | **Modified** (`f32598b`/L5, +36/−0) | **Extended with the consume-side fence (guard (d)).** N-F-A's gate fenced *population* + the *forge-time* (`produce_mode`) path; N-F-C adds **guard (d)** scoped to `node_sync.rs`: (pos) the forge path projects via `from_seed_epoch_consensus_inputs(`; (neg) no bundle/cold token (`import_live_consensus_inputs` / `pool_distr_view_from_consensus_inputs` / `consensus_inputs_path` / `InMemoryChainDb`); and **no shape-swap** — the forge path must not CONSTRUCT a `SeedEpochConsensusInputs { … }` literal (it must receive the recovered record via `BootstrapState` and project it). Backs CN-CINPUT-03 + DC-CINPUT-02b. |

**Cross-reference (TRACEABILITY):** none of the three new gates is in committed HEAD TRACEABILITY (0 N-F-C bindings committed); the working-tree TRACEABILITY refresh (part of this close-pass) binds them per the registry `ci_script` fields: `ci_check_lifecycle_owner_uses_bootstrap_initial_state.sh` → **DC-CINPUT-01** (and CN-NODE-01's evidence cites it), `ci_check_node_sync_via_pump.sh` → **DC-SYNC-01**, `ci_check_ba02_evidence_closed.sh` → **RO-LIVE-06**; and the guard-(d) extension of `ci_check_consensus_input_provenance.sh` → **CN-CINPUT-02 / CN-CINPUT-03 / DC-CINPUT-02b**. **Action:** commit the staged TRACEABILITY refresh with the cluster close so every gate listed here maps to a named invariant.

---

## 6. Canonical Type Registry Delta

n/a — `.idd-config.json` `canonical_type_registry` is `null`. Canonical-type rules live inline in the invariant registry under family **T**; the only family-T changes this window are `strengthened_in`-list appends on `T-REC-01` / `T-REC-02` (see §7), not type add/removes.

For reference, N-F-C introduces no new canonical *type*: the lifecycle owner composes the existing N-F-A `SeedEpochConsensusInputs` (CN-CINPUT-01 sole codec) and the existing forge/bootstrap types. The new closed `Ba02Manifest` / `BA02Outcome` / `PeerAcceptEvent` shapes are GREEN evidence types governed by family-RO invariant rule `RO-LIVE-06` (§7), not a separate canonical-type registry file, so there is nothing to delta against here.

---

## 7. Normative / Invariant Rule Delta

Source: `docs/ade-invariant-registry.toml` (the project's canonical append-only invariant registry; `invariant_registry` in `.idd-config.json`). Counts by `^[[rules]]` entries.

> **Committed-vs-staged reconciliation.** `git show HEAD:…registry.toml` = **303** (committed HEAD has **no** N-F-C rules and the pre-N-F-C `strengthened_in` / status values). The **working-tree** registry = **306** (the staged close-pass: +3 N-F-C rules + the 10 carry-forward strengthenings + the 2 status flips). The deltas below describe the **staged** registry — the state the cluster close will commit.

- Rules at baseline (`a3a0636`): **303**
- Rules at HEAD (committed `71a6c80`): **303** (no registry promotion committed yet)
- Rules at HEAD (**staged working tree**): **306**
- Net additions (staged): **+3** (`CN-CINPUT-03`, `DC-CINPUT-02b`, `RO-LIVE-06`)
- Removals: **0** (append-only rule discipline upheld — no rule ID dropped).

### New rules (staged)

| ID | Tier | Status | Cluster | One-line summary |
|----|------|--------|---------|------------------|
| `CN-CINPUT-03` | constraint | **enforced** | N-F-C | **Consume-side anti-laundering fence.** On the node-lifecycle forge path the leadership view MUST be projected from the recovered `SeedEpochConsensusInputs` surface (`PoolDistrView::from_seed_epoch_consensus_inputs` over the recovered `BootstrapState`) and MUST NOT be built from a forge-time operator bundle; no `SeedEpochConsensusInputs` value may be **constructed** on the production forge path (no shape-swap), and the forge-time tokens (`import_live_consensus_inputs` / `pool_distr_view_from_consensus_inputs` / `--consensus-inputs-path`) must not appear in the node-sync/forge driver. Data-flow-resistant containment gate (guard (d) of `ci_check_consensus_input_provenance.sh`). Closes CN-CINPUT-02's consume-side open obligation. (L5) |
| `DC-CINPUT-02b` | derived | **enforced** | N-F-C | **Producer CONSUMPTION (closes CE-A-4b).** The node-lifecycle forge base is built from the recovered selected tip + the recovered `SeedEpochConsensusInputs`: `forge_one_from_recovered` projects the leadership `PoolDistrView` via `from_seed_epoch_consensus_inputs(recovered)` and drives the reused `run_real_forge` engine with eta0 from the recovered `chain_dep`, failing closed (`MissingRecoveredConsensusInputs`) with no operator-bundle / cold / `--consensus-inputs-path` fallback. The consumption half of DC-CINPUT-02a's projection (A4 proved the projection; this binds the producer to consume it). Deterministic across runs. (L5) |
| `RO-LIVE-06` | release | **enforced** | N-F-C | **BA-02 peer-acceptance evidence closure (SCHEMA + CORRELATION MECHANICS ONLY).** Closed versioned `Ba02Manifest` + a pure/total/deterministic/hash-primary `correlate` (the sole `Ba02Manifest` constructor): the forged-block hash is the required key; a peer signal's slot must agree when present (else `NoEvidence`); conflicting peer signals → `NoEvidence`; the parser is allow-list only (`peer_served_block` / `peer_chain_tip`), never coercing `ForgeSucceeded` / `self_accept` / `block_received` / `agreement_verdict` to acceptance; forged fields come only from `ForgedBlockArtifact.{hash,slot}` (hash never recomputed, bytes never parsed). **Enforced for schema + correlation mechanics; explicitly NOT a claim that BA-02 was achieved live** (open_obligation records the live capture is operator-gated, distinct from RO-LIVE-01). (L6; `ci_check_ba02_evidence_closed.sh`) |

### Modified rules — status flips + strengthenings (staged)

**Two status flips (both strengthenings, not removals):**

- **`CN-STORE-02`** (WAL/checkpoint/recovered-artifact must bind to exactly one anchor/bootstrap lineage) — `status` **`declared` → `partial`**, and `strengthened_in` gains `"PHASE4-N-F-C"`. N-F-C L3's warm-start now *exercises* the recovered-artifact clause: `warm_start_recovery` discovers the seed-epoch anchor lineage (via the new `list_seed_epoch_consensus_anchor_fps()`) and **fails closed** on multiple anchor lineages / anchor mismatch / duplicate provenance. `partial` (not `enforced`): only the recovered-artifact anchor-lineage side is gated; the WAL-entry + checkpoint binding clauses are not yet covered by a dedicated CI check.
- **`DC-CINPUT-01`** (warm-start verification capability) — `status` **`partial` → `enforced`**, `strengthened_in` gains `"PHASE4-N-F-C"`, `ci_script` gains `ci_check_lifecycle_owner_uses_bootstrap_initial_state.sh`, `open_obligation` cleared. N-F-A left this `partial` because the *production* restart path was unwired; N-F-C L3 wires it (`node_lifecycle.rs::warm_start_recovery` opens `PersistentChainDb` + `FileWalStore`, replays via `replay_from_anchor`, calls `bootstrap_initial_state(RequiredFromRecoveredProvenance)`), and L4c (`node_sync_kill_then_warm_start_recovers_same_tip`) proves a synced+advanced tip recovers byte-identically through it.

**Ten carry-forward `strengthened_in` appends, all append-only (no list replaced), no statement weakened** (the two status-flip rules above also appear here because they gained the append as well as a status change):

- **`T-REC-01`** — `["PHASE4-N-R-A", "PHASE4-N-F-A"]` → `+ "PHASE4-N-F-C"` (✓ prior N-F-A drop repaired; production warm-start drives `replay_from_anchor → bootstrap_initial_state`, L4c proves byte-identical synced-tip recovery).
- **`T-REC-02`** — `["PHASE4-N-R-A", "PHASE4-N-F-A"]` → `+ "PHASE4-N-F-C"` (✓ same; the synced tip is fully replay-derivable through the production path).
- **`CN-STORE-02`** — `[]` → `["PHASE4-N-F-C"]` (recovered-artifact anchor-lineage binding; see status flip above).
- **`CN-NODE-01`** — `[…, "PHASE4-N-F-A"]` → `+ "PHASE4-N-F-C"` (the `--mode node` lifecycle owner is the single first-run-vs-warm-start owner; `ci_check_node_mode_closure.sh` now pins the 5-variant closed set; new gate `ci_check_lifecycle_owner_uses_bootstrap_initial_state.sh`).
- **`DC-WAL-03`** (replay-from-anchor byte-identity) — `[…, "PHASE4-N-Y"]` → `+ "PHASE4-N-F-C"` (the lifecycle owner is the first **production** caller of `replay_from_anchor`).
- **`DC-FORGE-01`** (leader-check determinism / replay-equivalence anchor) — `[]` → `["PHASE4-N-F-C"]` (L5 `forge_from_recovered_is_deterministic_across_two_runs` extends the anchor to the recovered-state forge handoff).
- **`DC-SYNC-01`** (forward-sync durable-before-tip + WAL-tail reconciliation) — `[]` → `["PHASE4-N-F-C"]`; `ci_script` gains `ci_check_node_sync_via_pump.sh`; `tests` gain `node_sync_pump_advances_recoverable_tip` / `node_sync_fails_closed_on_undecodable_block` / `node_sync_kill_then_warm_start_recovers_same_tip`. The lifecycle is the first **production** driver of `forward_sync::pump_block`.
- **`CN-CINPUT-02`** — `[]` → `["PHASE4-N-F-C"]`; its consume-side `open_obligation` rewritten to **CLOSED by PHASE4-N-F-C** (the consume-side fence landed as CN-CINPUT-03 on the node-lifecycle forge path; `produce_mode` stays diagnostic, `NotRequired`).
- **`DC-CINPUT-01`** — `[]` → `["PHASE4-N-F-C"]` (see status flip `partial` → `enforced` above).
- **`DC-CINPUT-02a`** — `[]` → `["PHASE4-N-F-C"]` (the A4 projection is now consumed on the production forge path; DC-CINPUT-02b is its consumption half).

> **`DC-CINPUT-02b` is the promotion of the ID N-F-A reserved.** N-F-A introduced `DC-CINPUT-02a` with the `a`-suffix deliberately reserving `02b` for the deferred producer-consumption rule; N-F-C promotes it here.

### Honest residual (cluster scope)

**PHASE4-N-F-C proves the Ade node lifecycle mechanics through evidence closure. It does not claim live BA-02. RO-LIVE-01 remains partial/operator-gated. RO-LIVE-06 is only schema/correlation mechanics.** Mechanically, the cluster proves: the `--mode node` owner first-run-bootstraps Mithril-only with no fallback (`CN-NODE-01` strengthened, `ci_check_lifecycle_owner_uses_bootstrap_initial_state.sh` + repaired `ci_check_bootstrap_closure.sh` / `ci_check_node_mode_closure.sh`); warm-start recovers WAL-only, replay-equivalently, and fail-closed on anchor-lineage ambiguity (`T-REC-01`/`T-REC-02`/`DC-WAL-03`/`CN-STORE-02` strengthened, `DC-CINPUT-01` promoted to `enforced`); the forward-sync path drives durable validated apply through the one `pump_block` seam (`DC-SYNC-01` strengthened, `ci_check_node_sync_via_pump.sh`); the producer **consumes** the WAL-recovered `SeedEpochConsensusInputs` rather than an operator bundle (`CN-CINPUT-03` / `DC-CINPUT-02b`, the consume-side guard (d); `CN-CINPUT-02`'s consume-side obligation thereby CLOSED); and the BA-02 evidence manifest is a closed, correlation-correct, lie-proof schema (`RO-LIVE-06`). It does **NOT** prove — and does not claim — that a **live operator pass against a real peer** has occurred: that is `RO-LIVE-01`, which remains **partial / operator-gated**, and `RO-LIVE-06`'s own `open_obligation` records that a real BA-02 result needs (1) an Ade-forged hash, (2) genuine cardano-node peer accept evidence naming that exact hash, and (3) a reviewed manifest from `correlate` over the captured log — synthetic fixtures prove the mechanics only.

---

## 8. Post-N-F-A Tail (`4b0eed6`, `1d29597`)

**Not a cluster.** Two housekeeping commits after the PHASE4-N-F-A close, before the N-F-C cluster body. **No `crates/**/*.rs` behavior change, no new invariant rule** (registry unchanged across the tail).

| Hash | Type | Summary |
|------|------|---------|
| `1d29597` | ci | `ci:` remove notify-atlas (ade-atlas now polls every 10 min) |
| `4b0eed6` | docs | `docs(close):` archive PHASE4-N-F-A capability cluster |

### `4b0eed6` — archive PHASE4-N-F-A capability cluster

The N-F-A close-pass cluster-doc move: `docs/clusters/PHASE4-N-F-A/` → `docs/clusters/completed/PHASE4-N-F-A/` (cluster.md + the A1/A2/A3a/A3b/A4 slice docs + the A5-SCOPING handoff). A pure `docs/` rename (the diffstat shows the `{ => completed}/PHASE4-N-F-A/` path moves with near-zero content delta apart from a ~17-line cluster.md closure annotation). **No source, no rule delta.**

### `1d29597` — remove notify-atlas dispatch workflow

Removes `.github/workflows/notify-atlas.yml` (42 lines) — a non-gating grounding-doc → `ade-atlas` rebuild **dispatch** workflow (its own header: *"Tells the ade-atlas dashboard to rebuild when the grounding docs / registry change"*), retired because *"ade-atlas now polls every 10 min."* The workflow was **never an Ade invariant gate**: it lives under `.github/workflows`, which `.idd-config.json` `ci_dirs = ["ci"]` deliberately excludes (the config's `_ci_dirs_doc` already noted the notify-atlas workflow "is not an Ade invariant gate"). **The `ci/ci_check_*.sh` enforcement-gate count is unaffected** by this removal (it moves separately, 105 → 108, on the three new N-F-C gates — §5). Intentional cleanup, not a lost Ade gate.

> **Anomaly check (tail):** no rule count change across the tail (registry stays 303 committed); the only "removal" is the **non-gating** `notify-atlas.yml` dispatch workflow (intentional; not an enforcement gate). No discipline violation in the tail.
