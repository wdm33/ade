# HEAD Deltas — Ade

> **Status:** Living architectural document. Regenerated; not hand-edited.
> Regenerate via `/head-deltas <baseline>`. Baseline is declared in
> `.idd-config.json` (`head_deltas_baseline`).

> Baseline: `d509f02` (Phase 3 handoff snapshot, 2026-04-15)
> HEAD: `0d4457e` (feat(validity): B1-S7 adversarial corpus — closes CE-B1-4, 2026-05-20)
> 76 commits, 11,153 files changed, +148,623 / −1,449,058 lines

Headline numbers note: the negative line count is dominated by the
**corpus relayout** under `corpus/snapshots/` and the deletion of two
multi-MB credentialed-snapshot text files (`*_tick_registered_creds.txt`
~1.4M lines, `*_final_registered_creds.txt` ~1.5K lines). Source-tree
deltas are far smaller — the per-crate breakdown in §3 is the
representative view.

The delta covers eleven threads of work. The newest threads — the
PHASE4-N-B close, the CE-N-B-6 follow-mode bridge, and the entire
PHASE4-B1 full-block-validity arc — landed after the previous
HEAD_DELTAS regen (which was cut at `744ef34`). In rough proportion of
the substantive change budget:

1. **Phase 4 cluster N-A (network mini-protocols)** — the largest
   code drop. 10 slices (S-A1 through S-A10, with S-A8b / S-A8c
   rework slices) shipped end-to-end as `feat(phase-4):` commits.
   Introduced the new BLUE workspace crate `ade_network` with 11
   mini-protocol codecs, 8 state machines, the Ouroboros mux frame
   codec, and a RED `session` substrate. Closed CE-N-A-1 through
   CE-N-A-5 against pinned cardano-node 11.0.1, including a
   real-capture corpus at `corpus/network/{n2n,n2c}/`. Three wire-form
   codec bugs surfaced by real interop were fixed in flight
   (chain-sync RollForward era-wrap, block-fetch flat RequestRange
   triple, N2C version 0x8000 wire flag), plus an LSQ Acquire /
   AcquireNoPoint split and a LocalTxSubmission / N2N TxSubmission2
   inner-tx HFC envelope fix, plus DoS-hardening on
   `Vec::with_capacity` in eight codecs.
2. **Phase 4 cluster N-B (consensus runtime) — closed.** 10 slices
   (S-B1 through S-B10) shipped as `feat(consensus):` commits, opened
   by `d9f0426` (invariant sketch v2 + 8 new `DC-CONS-*` registry
   rules) and **closed by `fb7f6dc`** (`Close PHASE4-N-B`). Built out
   the BLUE `ade_core::consensus` module (15 source files: closed
   `PraosChainDepState`, `EraSchedule`, fork-choice, rollback,
   nonce/op-cert/leader-schedule/VRF/header validation, plus
   `CandidateFragment` and structured event/error taxonomies). GREEN
   `ade_runtime::consensus` shipped the chain-selector orchestrator
   (`process_stream_input`), candidate-fragment builder, and a RED
   genesis parser. New replay corpora landed under
   `corpus/consensus/{hfc_schedule, nonce_evolution, leader_schedule,
   fork_choice, rollback, stream, op_cert}` (13 fixtures). All
   **6 CEs closed** (CE-N-B-1 fork choice, CE-N-B-2 rollback, CE-N-B-3
   header validation, CE-N-B-4 leader schedule, CE-N-B-5 stream
   replay, CE-N-B-6 live interop).
3. **CE-N-B-6 follow-mode bridge** — `c828449` retargeted the N-B
   live-interop pin to cardano-node 11.0.1, then `8b7e19e` added the
   RED `ade_core_interop::follow` follow-mode bridge plus live preprod
   tip-agreement evidence. Follow mode runs BLUE fork-choice
   (`select_best_chain`) and rollback (`apply_rollback`) only — it
   trusts the already-validated peer for header/VRF/leader/nonce/KES,
   so it carries no authoritative validation decision.
4. **Phase 4 cluster B1 (full block validity agreement) — closed at
   HEAD.** A 7-slice arc (S-B1 through S-B7, plus the cluster open /
   plan / doc commits) shipped as `feat(validity):` commits. This is
   the first thread to compose the N-A wire layer, the N-B consensus
   header authority, and the existing `ade_ledger` body authority into
   a single block verdict. It introduced the new BLUE `block_validity`
   submodule in `ade_ledger`, a BLUE `consensus_view` `LedgerView`
   projection, a RED `consensus_input_extract` snapshot tail-scan, a
   GREEN `validity` testkit harness, the `kes_check` fail-closed header
   crypto guard in `ade_core::consensus`, and the `corpus/validity/`
   positive + adversarial corpora. **All 5 CEs closed** (CE-B1-1
   LedgerView projection, CE-B1-2 header∧body composition, CE-B1-3
   positive-agreement replay, CE-B1-4 no-false-accept adversarial,
   CE-B1-5 fail-fast / unchanged-state-on-invalid). The arc opened the
   new **`DC-VAL-*` registry family** (6 rules) and added the new
   crate dependency edge **`ade_ledger -> ade_core`**.
5. **Phase 4 cluster N-D (ChainDB persistence)** — closed earlier in
   the delta (`436b1d7`). Slices S-33 through S-37 shipped end-to-end.
   CE-N-D-1 closure evidence (1000/1000 stress-kill iterations) at
   `docs/clusters/completed/PHASE4-N-D/CE-N-D-1_2026-05-19.log`.
6. **Phase 2C close-out / CE-73 reclassification** — single commit
   splitting CE-73 into a Tier-2 semantic gate (now enforced via new
   `ci_check_hfc_translation.sh`) and an explicit Tier-4 bytes non-goal.
7. **IDD canonicalization** — four `chore(idd)` commits that make the
   repo legible to the global IDD slash commands: `.idd-config.json`,
   registry rename (`constitution_registry.toml` → `docs/ade-invariant-registry.toml`),
   cluster N-D moved into `docs/clusters/PHASE4-N-D/`, repo-local
   commit-msg trailer hook.
8. **Grounding-doc generation + ripple** — `a87c3a3` produced the first
   cuts of CODEMAP, SEAMS, HEAD_DELTAS, and TRACEABILITY at the
   canonical `docs/ade-*.md` paths; `f0b0fd6` refreshed HEAD_DELTAS
   and SEAMS after the BLUE-scope closure; `a2c7ac8` refreshed all
   three after the N-D CI closure; `744ef34` refreshed CODEMAP /
   SEAMS / TRACEABILITY for the N-A close. **No grounding-doc refresh
   commit landed for N-B, the follow-bridge, or B1** — those three
   threads are reflected in this regen but not yet in CODEMAP / SEAMS /
   TRACEABILITY (see Anomalies).
9. **BLUE-list drift closure** — `5b70bee` extended six CI scripts from
   a 4-crate (or 5-crate for `dependency_boundary`) BLUE scope to the
   full 6-crate scope declared in `.idd-config.json`, then `c8fa37f`
   refreshed CODEMAP and TRACEABILITY to remove 14 `_(scope gap)_`
   markers across 13 rules.
10. **Phase 4 N-D CI gap closure** — `78da6c9` added three new RED-scope
    CI scripts (`ci_check_chaindb_contract.sh`,
    `ci_check_recovery_contract.sh`, `ci_check_chaindb_crash_safety.sh`)
    for the N-D recovery surface and flipped nine registry rules from
    `declared` → `enforced`. DC-STORE-04 was left `declared` with an
    explanatory Tier-5-divergence comment block — a comment edit, not a
    rule edit, per the IDD no-weakening discipline.
11. **Corpus relayout** — `corpus/snapshots/*` and the
    `reward_provenance/*_registered_creds.txt` files were removed
    (they carried credential material that does not belong in a
    public repo); 12 boundary-block sets were re-extracted at exact
    era-boundary slots and committed under `corpus/boundary_blocks/`;
    the consensus corpus (`corpus/consensus/*`) was added by N-B and
    the validity corpus (`corpus/validity/*`) by B1.

---

## 1. Commit Log

| Hash | Type | Summary |
|------|------|---------|
| `0d4457e` | feat | feat(validity): B1-S7 adversarial corpus — closes CE-B1-4 (no false accept) |
| `37498f2` | feat | feat(validity): B1-S6 positive agreement corpus + replay — closes CE-B1-3 |
| `c4723d4` | feat | feat(validity): B1-S4 block_validity composition — closes CE-B1-2 + CE-B1-5 |
| `62d25a3` | feat | feat(validity): B1-S5 Praos single-VRF + KES header validation — 14/14 real Conway headers validate |
| `2197355` | feat | feat(validity): B1-S3 BlockValidity verdict/error taxonomies + canonical surface encoding |
| `ee694f4` | feat | feat(validity): B1-S2 production LedgerView projection — closes CE-B1-1 |
| `bf353c4` | feat | feat(validity): B1-S1 consensus-input extractor + Conway-576 corpus |
| `af4a4db` | docs | docs(phase-4): PHASE4-B1 cluster doc — full block validity agreement |
| `c85cb7a` | docs | docs(phase-4): PHASE4-B1 cluster/slice plan — 7-slice full-block-validity arc |
| `2e58189` | docs | docs(phase-4): open PHASE4-B1 — full block validity agreement invariant sketch + DC-VAL registry family |
| `8b7e19e` | feat | feat(interop): CE-N-B-6 follow-mode bridge + live preprod tip-agreement evidence |
| `c828449` | docs | docs(consensus): retarget N-B live-interop pin to cardano-node 11.0.1 |
| `fb7f6dc` | chore | Close PHASE4-N-B — consensus runtime (Praos) authority + replay equivalence |
| `b9ff041` | feat | feat(consensus): S-B10 stream replay + orchestrator + live interop — closes CE-N-B-5 + CE-N-B-6 |
| `ecfaf70` | feat | feat(consensus): S-B9 rollback authority — closes CE-N-B-2 |
| `795a9ef` | feat | feat(consensus): S-B8 fork choice + CandidateFragment — closes CE-N-B-1 |
| `d924a22` | feat | feat(consensus): S-B7 Praos header validation |
| `0ed4568` | feat | feat(consensus): S-B6 leader schedule — closes CE-N-B-4 |
| `7c31a6b` | feat | feat(consensus): S-B5 op-cert counter monotonicity |
| `5bc4088` | feat | feat(consensus): S-B4 nonce evolution authority |
| `059e5e2` | feat | feat(consensus): S-B3 VRF cert verification wiring + Praos VRF input + leader threshold |
| `23d360e` | feat | feat(consensus): S-B2 PraosChainDepState canonical type + closed event/error taxonomies |
| `418ba9e` | feat | feat(consensus): S-B1 EraSchedule canonical authority + slot/era/time translation |
| `744ef34` | chore | chore(phase-4): complete PHASE4-N-A close — DoS hardening + grounding doc refreshes |
| `d9f0426` | docs | docs(phase-4): PHASE4-N-B invariant sketch v2 + 8 new DC-CONS-* registry rules |
| `69a2862` | chore | Close PHASE4-N-A — Ouroboros mini-protocols (11) wire-grammar conformance + state-machine determinism + real-interop validation |
| `56bfa7b` | feat | feat(phase-4): close CE-N-A-5 — 4 N2C real captures + LSQ/LTS/TxSubmission2 wire-form fixes + condition 4 + 5 + S-A10 evidence script |
| `d977640` | docs | docs(registry): wire S-A9 real-capture tests into PHASE4-N-A invariants |
| `b7cd39d` | feat | feat(phase-4): S-A9 N2C handshake + N2N keep-alive + peer-sharing real captures (3 more protocols + N2C 0x8000 wire-flag fix) |
| `a1b47ec` | feat | feat(phase-4): S-A9 block-fetch real interop + flat-range wire-form fix |
| `ef38212` | feat | feat(phase-4): S-A9 block-fetch codec wrapping fix + capture binary |
| `84d3eab` | feat | feat(phase-4): S-A9 chain-sync real capture + ChainSync codec wrapped-header fix |
| `98d0abe` | feat | feat(phase-4): S-A9 partial — real-capture corpus + handshake against mainnet relays |
| `1ba2d95` | feat | feat(phase-4): S-A8c — version table alignment with cardano-node 11.0.1 |
| `679491f` | docs | docs(phase-4): S-A8c entry obligation discharge — version table alignment with cardano-node 11.0.1 |
| `b7fade3` | feat | feat(phase-4): S-A8b — LocalTxMonitor wire-grammar rework (corrects S-A2/S-A8 misimpl) |
| `affa624` | docs | docs(phase-4): S-A8b entry obligation discharge — LocalTxMonitor wire-grammar rework |
| `9b7b96d` | docs | docs(phase-4): S-A9 + S-A10 entry obligation discharge — corpus replay harness + live interop closure gate |
| `77a02dd` | feat | feat(phase-4): S-A8 — N2C transition authority (4 state machines; structural completion) |
| `20b3554` | docs | docs(phase-4): S-A8 entry obligation discharge — N2C transition authority (4 state machines) |
| `b16329b` | feat | feat(phase-4): S-A7 — keep-alive + peer-sharing transition authority (structural completion) |
| `2cb0e86` | docs | docs(phase-4): S-A7 entry obligation discharge — keep-alive + peer-sharing transition authority |
| `844ae95` | feat | feat(phase-4): S-A6 — tx-submission2 transition authority (closes CE-N-A-4 state-machine portion) |
| `10659d5` | docs | docs(phase-4): S-A6 entry obligation discharge — tx-submission2 transition authority |
| `d702772` | feat | feat(phase-4): S-A5 — block-fetch transition authority (closes CE-N-A-3 state-machine portion) |
| `7078b9b` | docs | docs(phase-4): S-A5 entry obligation discharge — block-fetch transition authority |
| `787da55` | feat | feat(phase-4): S-A4 — chain-sync transition authority (closes CE-N-A-2 state-machine portion) |
| `7fef3a4` | docs | docs(phase-4): S-A4 entry obligation discharge — chain-sync transition authority |
| `ba02f71` | feat | feat(phase-4): S-A3 — handshake version negotiation authority (closes CE-N-A-1 state-machine portion) |
| `6faacd0` | docs | docs(phase-4): S-A3 entry obligation discharge — handshake version negotiation authority |
| `d1d47e9` | feat | feat(phase-4): S-A2 — protocol message codec authority for all 11 mini-protocols |
| `a4aabb9` | docs | docs(phase-4): S-A2 entry obligation discharge — protocol codec authority for all 11 mini-protocols |
| `4fde3a7` | feat | feat(phase-4): S-A1 — ade_network substrate + DC-CORE-01 mechanical gate |
| `22023be` | docs | docs(phase-4): S-A1 entry obligation discharge — mux/framing + sync-only CI gate |
| `6942674` | docs | docs(phase-4): open PHASE4-N-A cluster doc — wire+semantic Tier 1, 10 slices |
| `6ca2ba8` | docs | docs(phase-4): ratify PHASE4-N-A cluster plan (10 slices, authority-aligned) |
| `ae9c473` | docs | docs(phase-4): close N-A invariants §7 decisions + add DC-PROTO-06 |
| `492de56` | docs | docs(phase-4): open PHASE4-N-A — invariant sketch + DC-CORE-01 sync-only rule |
| `436b1d7` | chore | Close PHASE4-N-D — chain DB persistence with crash-equivalent recovery |
| `a3a083a` | docs | docs(phase-4): CE-N-D-1 closure evidence — 1000/1000 stress kill iterations green |
| `27960fd` | docs | docs(phase-4): lock N-A scope decisions before cluster opens |
| `a2c7ac8` | chore | chore(idd): refresh CODEMAP + TRACEABILITY + HEAD_DELTAS after N-D CI closure |
| `78da6c9` | chore | chore(ci): close Phase 4 N-D CI gap — 3 new scripts, 9 rules enforced |
| `f0b0fd6` | chore | chore(idd): refresh HEAD_DELTAS + SEAMS to align with BLUE-scope closure |
| `c8fa37f` | chore | chore(idd): refresh CODEMAP + TRACEABILITY after BLUE-list drift closure |
| `5b70bee` | chore | chore(ci): close BLUE-list drift — extend 6 CI scripts to full BLUE scope |
| `a87c3a3` | chore | chore(idd): generate four grounding docs (CODEMAP, SEAMS, HEAD_DELTAS, TRACEABILITY) |
| `3eddcbb` | chore | chore(idd): add .idd-config.json — opt the repo into IDD enforcement |
| `76c1f64` | chore | chore(idd): move in-flight cluster N-D into canonical clusters layout |
| `39865f6` | chore | chore(idd): update active-doc + CI refs to canonical registry path |
| `2047c42` | chore | chore(idd): commit-msg hook + CLAUDE.md trailer-override note |
| `5eecc8a` | feat | feat(phase-4): snapshot + forward-replay recovery (S-36) |
| `e52fe9f` | feat | feat(phase-4): SnapshotStore trait + impls (S-35) |
| `fb4a5d4` | feat | feat(phase-4): persistent ChainDb backed by redb (S-34) |
| `994203b` | feat | feat(phase-4): begin cluster N-D — ChainDb trait + InMemoryChainDb (S-33) |
| `9b15378` | feat | feat(phase-2c): reclassify CE-73 — semantic enforced, bytes Tier 4 non-goal |

Verbatim from `git log d509f02..HEAD` (`--no-merges`; history is
linear, no merge commits in range). Aggregation is in §3 and §5.

---

## 2. New Modules

| Module | Color | Purpose | Key sub-paths | Added in (cluster/slice) |
|--------|-------|---------|---------------|--------------------------|
| `ade_ledger::block_validity` (new submodule of an existing BLUE crate) | BLUE | Full-block verdict authority: closed `BlockValidityVerdict` (Valid / Invalid), closed `BlockValidityError` / `BlockRejectClass` reject taxonomy, fail-closed `FieldKind` / `FieldError` field-size taxonomy, and the `block_validity(...)` transition that composes the N-B header authority (`validate_and_apply_header`) with the body authority and returns evolved `(LedgerState', PraosChainDepState')` on Valid or unchanged input states + structured reason on Invalid. Header is validated before body (fail-fast). Canonical CBOR `VerdictSurface` encode/decode for the replay/comparison surface. Types-only `mod.rs` (no transition logic); composition lives in `transition.rs`. | `mod.rs` (re-exports), `verdict.rs` (`BlockValidityVerdict`, `BlockValidityError`, `BlockRejectClass`, `FieldError`, `FieldKind`, `MissingInput`), `transition.rs` (`block_validity`, `BlockValidityOutcome`), `header_input.rs` (`decode_block`, `DecodedBlock`), `encoding.rs` (`encode_verdict_surface`, `decode_verdict_surface`, `VerdictSurface`, `SurfaceDecodeError`) | PHASE4-B1 / S-B3 (taxonomy), S-B4 (composition) |
| `ade_ledger::consensus_view` (new file in an existing BLUE crate) | BLUE | Production `LedgerView` projection. `PoolDistrView` projects a `LedgerState`'s pool-distribution (`nesPd` / `stakeDistrib.unPoolDistr`) into exactly the four leadership-relevant facts BLUE consensus consumes through the `ade_core::consensus::LedgerView` boundary — total active stake, per-pool active stake, per-pool registered VRF keyhash, active-slots coefficient — and nothing else. | `consensus_view.rs` (`PoolDistrView`) | PHASE4-B1 / S-B2 |
| `ade_ledger::consensus_input_extract` (new file in an existing BLUE crate) | RED | Tail-scan of a snapshot `state` CBOR (a UTxO-HD `utxohd-mem` `ExtLedgerState` dump) for the five `PraosState` nonces. Classified RED because it parses an external dump format rather than an authoritative canonical type; the scan itself is pure over the input bytes and fail-closed (requires exactly five non-neutral nonces). | `consensus_input_extract.rs` | PHASE4-B1 / S-B1 |
| `ade_core::consensus::kes_check` (new file in an existing BLUE crate) | BLUE | Fail-closed wiring of `ade_crypto::kes` into Praos header validation. `expect_size` rejects wrong-length crypto fields rather than skipping them (DC-VAL-06 fail-closed pattern). Adds the single-VRF + KES header verification path exercised by 14/14 real Conway headers. | `kes_check.rs` (`expect_size` + KES header guard) | PHASE4-B1 / S-B5 |
| `ade_testkit::validity` (new submodule of an existing crate) | GREEN | Test-only block-validity harness. Loads the committed positive Conway-576 corpus and drives `block_validity` over all blocks (`replay.rs`); supplies a corpus-backed `LedgerView` (`ledger_view.rs`); derives adversarial blocks from the real corpus via deterministic mutators M1–M6 (`adversarial.rs`); fixture loader (`corpus.rs`). Non-authoritative. | `validity/mod.rs`, `validity/corpus.rs`, `validity/ledger_view.rs`, `validity/replay.rs`, `validity/adversarial.rs` | PHASE4-B1 / S-B6, S-B7 |
| `ade_core_interop::follow` (new file in an existing RED crate) | RED | Follow-mode bridge between a peer's ChainSync stream and BLUE fork-choice. Runs BLUE `select_best_chain` + `apply_rollback` ONLY; does **not** call `validate_and_apply_header`, builds no `LedgerView`, verifies no VRF/leader/nonce/KES. Asserts tip-selection agreement with an already-validated peer; trusts the peer for body/header validity (those need workstream-B ledger stake state). Carries no authoritative decision. | `follow.rs`, `tests/follow_offline_replay.rs` | CE-N-B-6 follow-bridge (`8b7e19e`) |
| `ade_network` (new workspace crate) | BLUE-majority (per-submodule scoped in `.idd-config.json` `core_paths`) | Ouroboros mini-protocol authority: 11 closed-grammar codecs, 8 pure transition state machines, Ouroboros mux frame codec, RED session/transport substrate. Wire bytes are Tier 1 — no Tier 5 latitude. Sync-only in BLUE submodules (DC-CORE-01); tokio is confined to `mux::transport`. | `codec/` (11 protocol message codecs); `handshake/`; `chain_sync/`, `block_fetch/`, `tx_submission/`, `keep_alive/`, `peer_sharing/`; `n2c/{local_chain_sync,local_state_query,local_tx_monitor,local_tx_submission}/`; `mux/frame.rs` (BLUE), `mux/transport.rs` (RED), `mux/mod.rs` (GREEN); `session/` (RED); 8 RED capture binaries under `src/bin/capture_*.rs` | PHASE4-N-A / S-A1 → S-A10 |
| `ade_core::consensus` (new submodule of an existing BLUE crate) | BLUE | Praos consensus authority: closed `PraosChainDepState`, era-aware slot/time translation, header validation, nonce evolution, op-cert counter monotonicity, leader schedule, fork choice, rollback. Closed-grammar `ChainEvent` / `ChainSelectionReject` taxonomies; flat-data error enums. No async, no ChainDb, no floats. | `mod.rs`, `candidate.rs`, `encoding.rs`, `era_schedule.rs`, `errors.rs`, `events.rs`, `fork_choice.rs`, `header_summary.rs`, `header_validate.rs`, `kes_check.rs` (B1), `leader_schedule.rs`, `ledger_view.rs`, `nonce.rs`, `op_cert.rs`, `praos_state.rs`, `rollback.rs`, `vrf_cert.rs` | PHASE4-N-B / S-B1 → S-B9 (kes_check from PHASE4-B1) |
| `ade_runtime::consensus` (new submodule of an existing RED crate) | GREEN/RED mix | Imperative-shell composition for consensus: stream-driven orchestrator (GREEN), candidate-fragment builder, and a RED genesis parser turning genesis JSON into the BLUE-consumed `EraSchedule`. | `mod.rs`, `candidate_fragment.rs`, `chain_selector.rs`, `genesis_parser.rs` | PHASE4-N-B / S-B8, S-B10 |
| `ade_core_interop` (new workspace crate) | RED | Live cardano-node interop driver for CE-N-B-6. Carries no authoritative decisions; `fresh_orchestrator` readiness probe, `live_consensus_session` binary, and (B1-era) the `follow` bridge. CI does not run this crate by default — tests are `#[ignore]`-gated or offline-replay only. | `src/lib.rs`, `src/follow.rs` (follow-bridge), `src/bin/live_consensus_session.rs`, `tests/live_consensus_session.rs`, `tests/follow_offline_replay.rs` | PHASE4-N-B / S-B10; follow-bridge `8b7e19e` |
| `ade_testkit::consensus` (new submodule of an existing crate) | GREEN | Test-only harness for consensus replay corpora. Loads JSON fixtures from `corpus/consensus/*`, provides a `LedgerView` stub, and the `consensus_stream_replay` driver. | `consensus/mod.rs`, `consensus/corpus.rs`, `consensus/ledger_view_stub.rs`, `consensus/stream_replay.rs` | PHASE4-N-B / S-B1, S-B6, S-B8, S-B9, S-B10 |
| `ade_runtime::chaindb` | RED | Block-store abstraction and impls. Trait surface is Tier 1; backing-store choice and on-disk layout are Tier 5. | `mod.rs`, `types.rs`, `error.rs`, `in_memory.rs`, `persistent.rs` (redb-backed), `contract.rs`, `snapshot_contract.rs`, `crash_safety.rs` | PHASE4-N-D / S-33, S-34, S-35 |
| `ade_runtime::recovery` | RED | Composes ChainDb + SnapshotStore into a generic recovery primitive: load latest snapshot, replay blocks forward to chain tip. | `recovery.rs` (Recoverable, RecoveryReport, RecoveryError, `recover<C, S, R>`) | PHASE4-N-D / S-36 |
| `ade_runtime` bin `chaindb_kill_target` | RED | Kill-target child process driver for the 1,000-kill-9 durability stress harness. | `src/bin/chaindb_kill_target.rs`, `tests/stress_kill_harness.rs` | PHASE4-N-D / S-37 |

Workspace-level membership grew by **two crates** across the full delta:
`ade_network` (PHASE4-N-A) and `ade_core_interop` (PHASE4-N-B). Both
are RED-or-mixed; `ade_core_interop` tests are `#[ignore]`-gated or
offline-replay only.

Crate dependency shape at HEAD (new deps in this delta):
- `ade_core` gained `ade_types`, `ade_crypto`, `minicbor` (deps);
  `ade_codec` (dep, added in B1 arc); `ade_testkit`, `serde_json`,
  `cardano-crypto` with `vrf-draft03` (dev).
- **`ade_ledger` gained a new `ade_core` dep edge** (PHASE4-B1) so the
  block-validity composition can call the consensus header authority,
  plus `minicbor` (dep) and `ade_testkit` (dev). This is the first time
  the ledger crate depends on the consensus crate.
- `ade_runtime` gained `ade_core`, `ade_crypto`, `ade_codec`,
  `serde_json` (deps); `ade_testkit`, `cardano-crypto` (dev). Carries
  the N-D deps `redb = "2"` (Tier 5) and `tempfile = "3"` (dev).
- `ade_testkit` gained `ade_core`, `ade_runtime` (deps);
  `cardano-crypto` (dev).
- `ade_core_interop` is new (deps: `ade_core`, `ade_runtime`,
  `ade_network`, `ade_testkit`, `ade_types`, `tokio`); gained
  `ade_codec`, `ade_crypto` in the B1 arc for the follow-bridge
  offline-replay path.

The N-A capture corpus shipped under `corpus/network/{n2n,n2c}/` —
11 protocol directories. The N-B replay corpus shipped under
`corpus/consensus/` — 13 JSON fixtures across seven sub-paths.

The B1 validity corpus shipped under `corpus/validity/` (16 files):
- `conway_epoch576/blocks/` (13 block CBOR files), plus
  `conway_epoch576/consensus_inputs.json` and a README — the positive
  Conway-576 corpus that all blocks validate against (the commit
  subjects frame this as "14/14 real Conway headers").
- `adversarial/README.md` — the adversarial corpus is **not** committed
  as fixture blocks; it is derived deterministically at test time by
  the M1–M6 mutators in `ade_testkit::validity::adversarial`, which
  mutate the real positive corpus (truncated VRF proof, flipped KES
  sig, slot-beyond-horizon, etc.).

Cross-reference: CODEMAP must be regenerated to add per-submodule
entries for `ade_ledger::block_validity`, `ade_ledger::consensus_view`,
`ade_ledger::consensus_input_extract`, `ade_core::consensus::kes_check`,
`ade_testkit::validity`, `ade_core_interop::follow`, plus the N-B
surfaces (`ade_core::consensus`, `ade_runtime::consensus`,
`ade_testkit::consensus`, the `ade_core_interop` crate). SEAMS must
record the block-validity seam (closed `BlockValidityVerdict` /
`BlockValidityError` / `BlockRejectClass`, the `VerdictSurface`
comparison surface, the `LedgerView` projection boundary) plus the N-B
consensus seam. TRACEABILITY must add rows for the 8 `DC-CONS-*` rules
and the 6 `DC-VAL-*` rules. **All three are stale on N-B + B1 as of
HEAD** — the last grounding-doc refresh was `744ef34` (N-A only).

---

## 3. Modules Modified

| Module | Scope | Key changes |
|--------|-------|-------------|
| `ade_ledger` | +13 source/test files, +1,755 lines (B1 arc); +1 file, +73 lines (CE-73 reclassification) | **PHASE4-B1 (primary):** the crate gained the `block_validity/` submodule (5 files), the BLUE `consensus_view.rs` `LedgerView` projection, and the RED `consensus_input_extract.rs` snapshot tail-scan, plus integration tests `block_validity_types.rs`, `block_validity_compose.rs`, `block_validity_positive_corpus.rs`, `block_validity_adversarial_corpus.rs`, `consensus_view.rs`, `consensus_input_extract.rs`. New manifest deps: **`ade_core`** (first ledger→consensus edge), `minicbor`; dev dep `ade_testkit`. This is where the full-block verdict is composed from the header authority and body authority. **CE-73 (earlier):** 10 unit tests for `decode_invalid_tx_indices` in `plutus_eval.rs` (piggybacked on `9b15378`), no production-code change. |
| `ade_core` | +15 source files + 10 tests (N-B, +7,334 lines); +828 / −86 across 16 files (B1 arc) | **PHASE4-N-B:** crate went from a stub `lib.rs` to a substantive BLUE consensus module — 15 source files under `src/consensus/` (`PraosChainDepState`, `EraSchedule`, header validation, nonce evolution, op-cert monotonicity, VRF threshold, leader schedule, fork choice, rollback, canonical event/error encodings) plus 10 integration tests. **PHASE4-B1:** added `consensus/kes_check.rs` (fail-closed `expect_size` + KES header guard) and `praos_header_validate.rs` test; touched `header_validate.rs`, `vrf_cert.rs`, `leader_schedule.rs`, `fork_choice.rs`, `header_summary.rs`, `ledger_view.rs`, `errors.rs`, `encoding.rs`, `mod.rs` to wire single-VRF + KES header validation (62d25a3, "14/14 real Conway headers validate"). New dep `ade_codec` (B1). |
| `ade_crypto` | 1 file, +24 / −81 lines (B1 arc) | Single change in `kes.rs` (`62d25a3`): **`build_opcert_signable` fixed** as part of the B1-S5 KES header-validation path; the function was reworked (net 57 fewer lines) to produce the correct KES op-cert signable bytes consumed by `ade_core::consensus::kes_check`. No other source change to this crate across the delta. |
| `ade_core_interop` | +1,426 / −37 across 6 files (B1 arc) | **CE-N-B-6 follow-bridge (`8b7e19e`) + pin retarget (`c828449`):** added the RED `follow.rs` follow-mode bridge (BLUE fork-choice + rollback only, no validation) and `follow_offline_replay.rs` test; reworked `lib.rs`, `bin/live_consensus_session.rs`, and `live_consensus_session.rs` test for the live preprod tip-agreement evidence path. New deps `ade_codec`, `ade_crypto` for the offline-replay path. |
| `ade_network` (existing crate, refined) | 6 source files, +8 / −8 lines | **DoS hardening of 6 codecs** (`744ef34`, post-N-A close): `chain_sync.rs`, `handshake.rs`, `local_chain_sync.rs`, `n2c_handshake.rs`, `peer_sharing.rs`, `tx_submission.rs` each capped untrusted `Vec::with_capacity` allocator hints at the remaining wire-bytes budget. No transition-authority changes since N-A closure. |
| `ade_runtime` | +4 source files in `consensus/`, +1,273 lines (N-B); +1 file, +7/−3 (B1 arc) | **PHASE4-N-B:** new `consensus/` submodule — `candidate_fragment.rs`, `chain_selector.rs`, `genesis_parser.rs` (RED genesis JSON → BLUE `EraSchedule`), plus `genesis_parser_corpus.rs` test. New deps `ade_core`, `ade_crypto`, `ade_codec`, `serde_json`; dev `ade_testkit`, `cardano-crypto`. **PHASE4-B1:** one small file touch (+7/−3). The N-D submodules (`chaindb`, `recovery`) and `chaindb_kill_target` binary are §2 New Modules. |
| `ade_testkit` | +4 files in `consensus/` (N-B); +5 files in `validity/`, +1,024 / −9 (B1 arc) | **PHASE4-N-B:** `consensus/` submodule (`mod.rs`, `corpus.rs`, `ledger_view_stub.rs`, `stream_replay.rs`) + `consensus_stream_replay.rs` test. **PHASE4-B1:** new `validity/` submodule (`mod.rs`, `corpus.rs`, `ledger_view.rs`, `replay.rs`, `adversarial.rs`) hosting the positive-corpus replay driver and the M1–M6 adversarial mutators. New deps `ade_core`, `ade_runtime`; dev `cardano-crypto`. |

No other crate had non-trivial source changes since baseline.
`ade_codec`, `ade_types`, `ade_plutus`, and `ade_node` were untouched
by code commits. `ade_plutus`'s `evaluator.rs` is **referenced** by the
BLUE-scope CI extension (see §5) as the named chokepoint for
`PlutusScript::from_cbor` but the source file itself was not modified.

`.idd-config.json` had prose edits during the delta (the
`_core_paths_doc` and the registry-count note); the `core_paths` array
itself was extended only for the `ade_network` per-submodule scope —
`ade_core` and `ade_ledger` were already listed, so the new
`block_validity` / `consensus` surfaces are covered automatically.

---

## 4. Feature Flags

No Cargo `[features]` tables exist at HEAD in any workspace crate, and
none existed at baseline. The project does not use Cargo feature flags
as a semantic surface — closed semantic surfaces are encoded in the
type system per the IDD core principles, and conditional compilation is
checked out of BLUE code via `ci/ci_check_no_semantic_cfg.sh` (scoped
over the full 6-crate BLUE set plus all `ade_network` BLUE submodules;
the new `ade_core::consensus` and `ade_ledger::block_validity` surfaces
are covered by their crate-level scope).

No `#[cfg(feature = ...)]` gates appear at either ref. `cardano-crypto`
is declared `default-features = false, features = ["vrf-draft03"]` in
the dev-dependency entries (and `minicbor` with
`default-features = false, features = ["alloc"]` in the new
`ade_ledger` / `ade_core` deps) — these are upstream-crate feature
selections, not Ade-side flags.

**Status: unchanged — zero Ade feature flags at baseline, zero at HEAD.**

---

## 5. CI Checks

The CI surface is the shell-script set under `ci/` (no
`.github/workflows` in this repo). At baseline there were 15 scripts.
At HEAD there are **24 scripts plus one git hook**: CE-73
reclassification added one (`ci_check_hfc_translation.sh`), N-D added
three, N-A added two, N-B added four, and one repo-local git hook
(`ci/git-hooks/commit-msg`) shipped. PHASE4-B1 added **no new CI
script** — instead it **extended one existing N-B script**
(`ci_check_consensus_closed_enums.sh`) to also cover
`ade_ledger::block_validity`. Six scripts had BLUE-scope arrays
extended; one had a path-only registry edit. Grouped by cluster.

### CE-73 reclassification (Phase 2C close-out)

| Check | Status | What it checks |
|-------|--------|----------------|
| `ci/ci_check_hfc_translation.sh` | **New** (`9b15378`) | CE-73-semantic gate: runs the three HFC ledger-side translation proof surfaces. Authoritative test for invariant `DC-EPOCH-02`. |

### IDD canonicalization (post-Phase-4-N-D)

| Check | Status | What it checks |
|-------|--------|----------------|
| `ci/ci_check_constitution_coverage.sh` | Modified (`39865f6`) | Path-only edit: registry path now `docs/ade-invariant-registry.toml`. Coverage enforcement unchanged. |
| `ci/git-hooks/commit-msg` | **New** (`2047c42`) | Local git hook: rejects commit messages lacking a `Co-Authored-By: Claude ...` trailer. Repo-local exception to the global no-AI-attribution rule, commit-messages only. |

### BLUE-list drift closure (`5b70bee`)

Six CI scripts were extended to the full 6-crate BLUE set
(`ade_codec`, `ade_types`, `ade_crypto`, `ade_core`, `ade_ledger`,
`ade_plutus`).

| Check | Status | What it checks |
|-------|--------|----------------|
| `ci/ci_check_module_headers.sh` | Modified — BLUE-scope (`5b70bee`) | `// Core Contract:` header on every `.rs` in BLUE crates. `T-BUILD-01`. |
| `ci/ci_check_no_semantic_cfg.sh` | Modified — BLUE-scope (`5b70bee`) | No semantic `#[cfg(...)]` in BLUE `src/`. `T-BUILD-01`. |
| `ci/ci_check_no_signing_in_blue.sh` | Modified — BLUE-scope (`5b70bee`) | No signing primitives in BLUE crates. `T-KEY-01`. |
| `ci/ci_check_hash_uses_wire_bytes.sh` | Modified — BLUE-scope (`5b70bee`) | All BLUE hashing via wire-byte fingerprint surfaces. `T-ENC-01`, `DC-CBOR-02`. |
| `ci/ci_check_ingress_chokepoints.sh` | Modified — BLUE-scope + registry growth (`5b70bee`) | No raw CBOR decoding outside named chokepoints in BLUE. Registry grew 10 → 11 (`PlutusScript::from_cbor`). `T-INGRESS-01`, `DC-INGRESS-01`. |
| `ci/ci_check_dependency_boundary.sh` | Modified — BLUE-scope (`5b70bee`) | BLUE crates must not depend on RED crates. `T-BOUND-02`. |

Follow-up `c8fa37f` re-ran CODEMAP and TRACEABILITY against the new
scope, removing 14 `_(scope gap)_` markers across 13 rules.

### Phase 4 N-D CI gap closure (`78da6c9`)

| Check | Status | What it checks |
|-------|--------|----------------|
| `ci/ci_check_chaindb_contract.sh` | **New** (`78da6c9`) | `cargo test -p ade_runtime --lib chaindb::` — 8 contract tests. `DC-STORE-02`, `DC-STORE-03`, `CN-STORE-04`, `CN-STORE-05`. |
| `ci/ci_check_recovery_contract.sh` | **New** (`78da6c9`) | `cargo test -p ade_runtime --lib recovery::` — 6-test recovery bundle. `T-REC-01`, `T-REC-02`, `DC-STORE-05`. |
| `ci/ci_check_chaindb_crash_safety.sh` | **New** (`78da6c9`) | Smoke variant of the subprocess-SIGKILL harness + integrity post-checks. `T-REC-01`, `DC-STORE-01`, `CN-STORE-03`. |

### Phase 4 N-A wire + semantic enforcement (S-A1, S-A10)

| Check | Status | What it checks |
|-------|--------|----------------|
| `ci/ci_check_no_async_in_blue.sh` | **New** (`4fde3a7`, S-A1) | Enforces `DC-CORE-01` — BLUE code is sync-only. Scans every BLUE path in `.idd-config.json` `core_paths` for async/tokio/futures. Covers `ade_core::consensus` and `ade_ledger::block_validity` via crate prefixes. |
| `ci/ci_check_ce_n_a_5_proof.sh` | **New** (`56bfa7b`, S-A10) | CE-N-A-5 closure-gate evidence over the real-cardano-node corpus; logs to `docs/active/CE-N-A-5_evidence.toml`. |

### Phase 4 N-B consensus authority enforcement (S-B1, S-B2, S-B8) — extended by B1

Four BLUE-scope CI scripts targeting `crates/ade_core/src/consensus/`.
The closed-enums script was **extended in PHASE4-B1** to also scan
`crates/ade_ledger/src/block_validity/`.

| Check | Status | What it checks |
|-------|--------|----------------|
| `ci/ci_check_consensus_closed_enums.sh` | **New** (N-B); **Modified** (B1, `c4723d4`) | Four-part scan now over **both** `ade_core/src/consensus/` and `ade_ledger/src/block_validity/`: no `#[non_exhaustive]`; no open-tail `Other`/`Unknown` variant; no owned `String` in the error/encoding/events files (`verdict.rs` and `encoding.rs` added to the string-scope list); no `Box<dyn ...>`. Strengthens `DC-CONS-04`, `DC-CONS-10`, `T-DET-01` (consensus) **and now `DC-VAL-02`/`-04`/`-05`/`-06`** (block-validity) — every reject reason a structured flat-data value. |
| `ci/ci_check_no_chaindb_in_consensus_blue.sh` | **New** (N-B / S-B1) | No `ChainDb`/`chain_db` token in `consensus/`. Strengthens `DC-CORE-01`, `DC-CONS-07`. |
| `ci/ci_check_no_density_in_fork_choice.sh` | **New** (N-B / S-B8) | No `density` token in `fork_choice.rs` / `candidate.rs`. Praos fork-choice ordering is `(BlockNo, TiebreakerView)` only. Strengthens `DC-CONS-03`. |
| `ci/ci_check_no_float_in_consensus.sh` | **New** (N-B / S-B1) | No `f32`/`f64` in `consensus/`. Strengthens `T-CORE-02`, `DC-CONS-07/08/09`. |

TRACEABILITY cross-reference: the four N-B scripts map to the 8
`DC-CONS-*` rules; the extended closed-enums script now also enforces
four of the six `DC-VAL-*` rules. The six `DC-VAL-*` rules are
otherwise **test-enforced, not script-enforced** — their registry
`ci_script` field is empty and enforcement is carried by named
integration tests in `ade_ledger/tests/` and `ade_testkit`. None of
the new N-B or B1 rows exist in TRACEABILITY yet (last refresh
`744ef34`, N-A only).

---

## 6. Canonical Type Registry Delta

n/a — `.idd-config.json` `canonical_type_registry` is null. Canonical-type
rules live inline in the invariant registry under family `T`.

---

## 7. Normative Rule Delta

The project's invariant registry tracks structured rules (TOML), not
prose normative-doc rules; this section reports on it.

- Rules at baseline: **147** (in `constitution_registry.toml`)
- Rules at HEAD: **163** (in `docs/ade-invariant-registry.toml`)
- Net additions: **+16** (PHASE4-N-A: 2; PHASE4-N-B: 8; PHASE4-B1: 6)
  - PHASE4-N-A: `DC-CORE-01` (BLUE sync-only); `DC-PROTO-06` (pure
    transitions).
  - PHASE4-N-B (`d9f0426`): `DC-CONS-03` (Praos `(BlockNo,
    TiebreakerView)` ordering, no density), `DC-CONS-04`
    (`PraosChainDepState` owned by consensus), `DC-CONS-05` (rollback
    never exceeds k), `DC-CONS-06` (rollback = truncated replay /
    `ForkBeforeImmutableTip`), `DC-CONS-07` (HFC schedule consumed as
    typed `EraSchedule`), `DC-CONS-08` (`slot_to_time` pure, no BLUE
    wall clock), `DC-CONS-09` (`OutsideForecastRange` beyond safe
    zone), `DC-CONS-10` (op-cert counter monotonicity).
  - **PHASE4-B1 (`2e58189`, new `DC-VAL` family):**
    - **`DC-VAL-01`** — A block's verdict is a pure function of
      `(LedgerState, PraosChainDepState, EraSchedule, LedgerView,
      block_cbor)`; no wall-clock/arrival-order/HashMap/float/ambient
      influence. Status **`enforced`**. Tests in
      `consensus_input_extract` / `consensus_view`; cross-ref
      `DC-CORE-01`, `T-DET-01`, `DC-CONSENSUS-01`.
    - **`DC-VAL-02`** — Valid iff *both* the consensus header authority
      and the ledger body authority accept; no Valid verdict may skip
      either. Status `declared`. Enforced via
      `ci_check_consensus_closed_enums.sh` (B1 extension) + composition
      tests.
    - **`DC-VAL-03`** — Header validated before body; body never runs
      on a header-invalid block; first failing authority sets the
      reason (fail-fast). Status `declared`. Test
      `header_before_body_fail_fast`.
    - **`DC-VAL-04`** — Ade's verdict equals the reference
      cardano-node verdict (incl. reject class), over both a positive
      corpus and a mandatory adversarial corpus. Status `declared`.
      Tests: `corpus_block_count_is_14`, `all_corpus_blocks_valid`,
      `verdict_stream_replays_identically`, `no_mutation_is_ever_valid`,
      `each_mutation_maps_to_expected_class`.
    - **`DC-VAL-05`** — Valid → evolved `(LedgerState',
      PraosChainDepState')`; Invalid → unchanged input states +
      structured reason; no partial/in-place mutation. Status
      `declared`.
    - **`DC-VAL-06`** — Every crypto-input / field-size / structural
      check rejects on wrong size/shape and never silently skips;
      `if X.len() == K { check } else { skip }` forbidden in BLUE; no
      defined-but-unwired or tautological guard. Status `declared`.
      Tests: `expect_size_rejects_wrong_length`,
      `praos_malformed_kes_sig_rejected`, plus adversarial mutators.
- Removals: **0** (expected under append-only discipline; clean)
- Strengthenings (`declared` / `partial` → `enforced`):
  - `DC-EPOCH-02` (`9b15378`): `partial` → `enforced`.
  - `T-REC-01`, `T-REC-02`, `DC-STORE-01`, `DC-STORE-02`,
    `DC-STORE-03`, `DC-STORE-05`, `CN-STORE-03`, `CN-STORE-04`,
    `CN-STORE-05` (all `78da6c9`): `declared` → `enforced`.
  - `T-ENC-03`, `CN-WIRE-07`, `DC-PROTO-02`, `DC-PROTO-05`
    (PHASE4-N-A): real-capture corpus + `ci_check_ce_n_a_5_proof.sh`.
  - `T-CORE-02` (S-B1, N-B): `declared` → `enforced` for the consensus
    surface.
  - The six `DC-VAL-*` rules each carry `strengthened_in =
    ["PHASE4-B1"]` in the registry even though the family was *created*
    in PHASE4-B1 — recorded faithfully here; see Anomalies.

Family counts at HEAD (per `^id = "<FAM>-"`): CN dominates (~64),
DC = 53 (now including `DC-CONS` ×8, `DC-VAL` ×6), T = 30, RO/OP ×9
combined. The `DC` family grew most this delta (+2 N-A, +8 N-B, +6 B1
= +16 DC-family additions net of the families they sit in).

Normative-doc rule extraction (the `normative_docs` list in
`.idd-config.json`) is approximate and not regenerated here — the
structured registry is the authoritative source.

---

## Anomalies and Cross-Reference Warnings

- **PHASE4-B1 closed at HEAD; cluster doc not yet archived.** All five
  CEs closed (CE-B1-1 `ee694f4`, CE-B1-2 + CE-B1-5 `c4723d4`, CE-B1-3
  `37498f2`, CE-B1-4 `0d4457e`). The cluster doc lives at
  `docs/clusters/PHASE4-B1/` (opened `2e58189`/`af4a4db`); no
  `Close PHASE4-B1` commit appears in the log and it is **not** yet
  moved to `docs/clusters/completed/PHASE4-B1/`. Surface for the next
  housekeeping commit.
- **PHASE4-N-B closed.** `fb7f6dc` (`Close PHASE4-N-B`) archived the
  cluster; the prior HEAD_DELTAS regen predated this commit and called
  N-B "closed at HEAD but not archived" — that is now resolved. The
  CE-N-B-6 live-interop pin was retargeted to cardano-node 11.0.1
  (`c828449`) and the follow-mode bridge landed at `8b7e19e`.
- **DC-VAL status mismatch vs. closure claim.** PHASE4-B1 is reported
  as fully closed (all CEs green), but in the registry only
  **`DC-VAL-01` is `enforced`** — `DC-VAL-02` through `DC-VAL-06`
  remain `status = "declared"` despite having named tests and (for
  02/04/05/06) the extended `ci_check_consensus_closed_enums.sh`
  enforcement point. Either the registry statuses need flipping to
  `enforced` on the next `/traceability` pass, or the cluster-close
  gate should confirm the declared rules are mechanically green before
  archiving. Flagged — this is the kind of premature-status-flip-vs-
  reality gap the complete-work-only discipline targets.
- **`DC-VAL-*` `strengthened_in = ["PHASE4-B1"]` on freshly-created
  rules.** The six DC-VAL rules were *introduced* in PHASE4-B1 yet each
  records `strengthened_in = ["PHASE4-B1"]`. `strengthened_in`
  ordinarily records *later* clusters that tightened a pre-existing
  rule; using it for the introducing cluster is unusual but harmless
  (no weakening). Reported faithfully; consider normalizing on the
  next registry curation pass.
- **Grounding docs stale on N-B + follow-bridge + B1.** CODEMAP, SEAMS,
  and TRACEABILITY were last refreshed at `744ef34` (N-A only). They
  carry **no** entries for: `ade_core::consensus` + `kes_check`,
  `ade_runtime::consensus`, `ade_testkit::consensus`, the
  `ade_core_interop` crate + `follow` bridge, `ade_ledger::block_validity`,
  `ade_ledger::consensus_view`, `ade_ledger::consensus_input_extract`,
  or `ade_testkit::validity`. TRACEABILITY is missing rows for all 8
  `DC-CONS-*` and all 6 `DC-VAL-*` rules. The CI-script count in
  CODEMAP must tick to 24 scripts + 1 hook. Run `/codemap`, `/seams`,
  `/traceability`.
- **New `ade_ledger -> ade_core` dependency edge.** First time the
  ledger crate depends on the consensus crate. Both are BLUE, so the
  `ci_check_dependency_boundary.sh` BLUE→RED guard is unaffected, but
  CODEMAP's dependency graph and SEAMS' module-addition rules should
  record the new intra-BLUE edge (consensus header authority is now a
  ledger-side composition dependency).
- **`ade_crypto::kes::build_opcert_signable` fixed in B1-S5.** A
  correctness fix (`62d25a3`) to the KES op-cert signable-bytes
  derivation, validated by 14/14 real Conway headers. This is a BLUE
  crypto-surface behavioral change; confirm crypto-vector CI
  (`ci_check_crypto_vectors.sh`) still covers it on the next
  TRACEABILITY pass.
- **Adversarial corpus is derived, not committed.** `corpus/validity/
  adversarial/` holds only a README; the adversarial blocks are
  generated deterministically at test time by the M1–M6 mutators in
  `ade_testkit::validity::adversarial`. This is intentional (keeps the
  corpus a pure function of the positive corpus), but means the
  CE-B1-4 no-false-accept evidence lives in test code, not fixtures.
- **PHASE4-N-A / N-D closed.** N-A archived (`69a2862`, DoS-hardening
  follow-up `744ef34`); N-D archived (`436b1d7`, CE-N-D-1 evidence
  1000/1000 stress-kill green).
- **`ade_core_interop` tests are `#[ignore]`-gated / offline-replay by
  design.** Live tip-agreement is not run in CI; the follow-bridge has
  an offline-replay test (`follow_offline_replay.rs`). CE-N-B-6 closure
  evidence is a manual operator pass. By design (RED, no authority).
- **`ade_node` MUST NOT list is forward-looking.** Binary is a stub;
  no authority surface exercised. Not new in this delta.
- **Corpus relayout: credentialed snapshots removed.** Deleted
  `corpus/snapshots/reward_provenance/*_registered_creds.txt`
  dominates the large negative line count; replaced by 12 re-extracted
  boundary-block sets under `corpus/boundary_blocks/`.
- No removed canonical types (n/a — no separate registry).
- No removed registry rules (expected: 0; actual: 0).
- No commit subjects lack a conventional-commits prefix except the two
  `Close PHASE4-N-*` chore commits (`69a2862`, `436b1d7`, `fb7f6dc`) —
  cluster-close housekeeping, classified `chore` on scope grounds.

---

## Generation Notes

Regenerate via `/head-deltas <baseline>` or by re-running the
`head-deltas-generator` agent with the same baseline. Baseline lives
in `.idd-config.json` `head_deltas_baseline`. Update on next phase
boundary (Phase 4 close, or when the next cluster — N-C / N-E / N-F /
the next B-series — closes).
