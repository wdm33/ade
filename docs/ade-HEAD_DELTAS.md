# HEAD Deltas — Ade

> **Status:** Living architectural document. Regenerated; not hand-edited.
> Regenerate via `/head-deltas <baseline>`. Baseline is declared in
> `.idd-config.json` (`head_deltas_baseline`).

> Baseline: `d509f02` (Phase 3 handoff snapshot, 2026-04-15)
> HEAD: `85a50dc` (feat(tx-validity): B2-S5 mempool admission gate — closes CE-B2-5, 2026-05-20)
> 87 commits, 11,180 files changed, +155,533 / −7,233,349 lines

Headline numbers note: the massive negative line count is dominated by
the **corpus relayout** under `corpus/snapshots/` and the deletion of
the multi-MB credentialed-snapshot text files
(`*_registered_creds.txt`, ~7M lines combined). Source-tree deltas are
far smaller — the per-crate breakdown in §3 is the representative view.

> **Commit-hash note.** This regen runs against the current (rebased)
> history. The prior HEAD_DELTAS regen — cut at the then-HEAD
> `0d4457e` (B1-S7) — references commit hashes from a history that has
> since been rewritten; e.g. B1-S7 is now `2630267`, the N-A close is
> `69a2862` (unchanged), and the B1 close is `993f363`. All hashes
> below are verbatim from `git log d509f02..HEAD` at this HEAD.

The delta covers twelve threads of work. The newest thread — the
**PHASE4-B2 tx-validity-agreement arc** — landed on top of the
previous regen's HEAD (B1-S7). The PHASE4-B1 close (`993f363`), the
Cargo.lock sync, and the ledger-state-dump `.gitignore` chore landed
just before it. In rough proportion of the substantive change budget:

1. **Phase 4 cluster B2 (tx validity agreement) — closed at HEAD.** A
   5-slice arc (B2-S1 → B2-S5) shipped as `feat(tx-validity):`
   commits, opened by the planning trio `b79f632` (invariant sketch +
   `DC-TXV` family), `b32fef3` (cluster/slice plan), `7263699`
   (cluster doc). It is the **per-transaction** counterpart to B1's
   per-block verdict: it introduces the new BLUE `ade_ledger::tx_validity`
   submodule (vkey-witness + required-signer closure, phase-1
   composition, closed verdict taxonomy, canonical verdict surface)
   and the BLUE/GREEN `ade_ledger::mempool` admission gate. **All 5 CEs
   closed** (CE-B2-1 vkey-witness/required-signer closure, CE-B2-2
   tx_validity composition + verdict taxonomy, CE-B2-3 positive corpus
   replay with 103/103 real Conway txs Valid, CE-B2-4 adversarial
   corpus no-false-accept, CE-B2-5 mempool admission gate). The arc
   added 5 new `DC-TXV-*` registry rules (all `enforced`), flipped the
   two pre-existing `DC-MEM-*` rules to `enforced`, and — critically —
   **found and fixed a real value-conservation fail-open** (see thread
   2). No `Close PHASE4-B2` commit appears in the log yet; the close
   commit is the upcoming housekeeping step (see Anomalies).
2. **Conway value-conservation fail-open found + fixed by the
   adversarial corpus (`617139f`, B2-S4).** The Conway/Babbage/Alonzo
   state-backed validation path verified input presence, collateral,
   network, and required signers but **never checked coin-level value
   conservation** — so a tx whose `outputs + fee != inputs` was
   accepted as `Valid` at `track_utxo=true`. This is a genuine
   false-accept, surfaced by the family-B synthetic adversarial cases
   (S1 value imbalance). The fix adds
   `ade_ledger::conway::check_conway_coin_conservation`, enforcing
   `sum(inputs) == sum(outputs) + fee + donation` (i128, no float) for
   cert/withdrawal-free txs. **Deposit/refund/withdrawal accounting is
   conservatively DEFERRED** — the guard returns early for txs carrying
   certs or withdrawals, so it can never false-*reject*; the residual
   value-imbalance for deposit/withdrawal-bearing txs is the named
   tx-validity-completeness follow-up (tracked in
   `corpus/tx_validity/adversarial/README.md`). Four Conway happy-path
   fixtures were rebalanced to satisfy the new equation. RESULT: all
   220 adversarial cases → Invalid; no false accept.
3. **Phase 4 cluster B1 (full block validity agreement) — closed
   (`993f363`).** The 7-slice arc (B1-S1 → B1-S7) composes the N-A wire
   layer, the N-B consensus header authority, and the `ade_ledger` body
   authority into a single block verdict. Introduced the BLUE
   `ade_ledger::block_validity` submodule, the BLUE `consensus_view`
   `LedgerView` projection, the RED `consensus_input_extract` snapshot
   tail-scan, the GREEN `validity` testkit harness, the `kes_check`
   fail-closed header crypto guard in `ade_core::consensus`, and the
   `corpus/validity/` positive + (derived) adversarial corpora. **All 5
   CEs closed.** Opened the `DC-VAL-*` registry family (6 rules) and
   added the new crate dependency edge `ade_ledger -> ade_core`. Closed
   by `993f363`; `3552bc2` synced `Cargo.lock` for the new edges and
   `e0af99d` gitignored the multi-GB ledger-state dumps.
4. **Phase 4 cluster N-A (network mini-protocols) — closed.** 10
   slices (S-A1 → S-A10, with S-A8b/S-A8c rework). Introduced the new
   BLUE workspace crate `ade_network` with 11 mini-protocol codecs, 8
   state machines, the Ouroboros mux frame codec, a RED `session`
   substrate. Closed CE-N-A-1 → CE-N-A-5 against pinned cardano-node
   11.0.1, including a real-capture corpus at `corpus/network/{n2n,n2c}/`.
   Three wire-form codec bugs surfaced by real interop were fixed in
   flight, plus an LSQ Acquire/AcquireNoPoint split, a
   LocalTxSubmission/N2N TxSubmission2 inner-tx HFC envelope fix, and
   DoS-hardening on `Vec::with_capacity` in eight codecs.
5. **Phase 4 cluster N-B (consensus runtime) — closed (`a0c73e1`).**
   10 slices (S-B1 → S-B10), opened by `d9f0426` (invariant sketch v2 +
   8 `DC-CONS-*` rules). Built out the BLUE `ade_core::consensus` module
   (15+ source files: closed `PraosChainDepState`, `EraSchedule`,
   fork-choice, rollback, nonce/op-cert/leader-schedule/VRF/header
   validation, `CandidateFragment`, structured event/error taxonomies).
   GREEN `ade_runtime::consensus` shipped the chain-selector
   orchestrator, candidate-fragment builder, and a RED genesis parser.
   New replay corpora under `corpus/consensus/`. All 6 CEs closed.
6. **CE-N-B-6 follow-mode bridge** — `807bcb6` retargeted the N-B
   live-interop pin to cardano-node 11.0.1, then `e5f1f64` added the RED
   `ade_core_interop::follow` bridge plus live preprod tip-agreement
   evidence. Follow mode runs BLUE fork-choice + rollback only — it
   trusts the already-validated peer for header/VRF/leader/nonce/KES, so
   it carries no authoritative validation decision.
7. **Phase 4 cluster N-D (ChainDB persistence) — closed (`436b1d7`).**
   Slices S-33 → S-37. CE-N-D-1 closure evidence (1000/1000 stress-kill
   iterations).
8. **Phase 2C close-out / CE-73 reclassification** — single commit
   (`9b15378`) splitting CE-73 into a Tier-2 semantic gate (enforced via
   `ci_check_hfc_translation.sh`) and an explicit Tier-4 bytes non-goal.
9. **IDD canonicalization** — `chore(idd)` commits that make the repo
   legible to the global IDD slash commands: `.idd-config.json`,
   registry rename (`constitution_registry.toml` →
   `docs/ade-invariant-registry.toml`), cluster N-D moved into
   `docs/clusters/PHASE4-N-D/`, repo-local commit-msg trailer hook.
10. **Grounding-doc generation + ripple** — `a87c3a3` produced the
    first cuts of CODEMAP/SEAMS/HEAD_DELTAS/TRACEABILITY; `f0b0fd6`,
    `a2c7ac8`, and `744ef34` refreshed subsets after the BLUE-scope,
    N-D, and N-A closures respectively. **No grounding-doc refresh
    commit landed for N-B, the follow-bridge, B1, or B2** — those
    threads are reflected in this regen but not yet in CODEMAP / SEAMS /
    TRACEABILITY (see Anomalies).
11. **BLUE-list drift closure** — `5b70bee` extended six CI scripts to
    the full 6-crate BLUE scope; `c8fa37f` refreshed CODEMAP and
    TRACEABILITY to remove 14 `_(scope gap)_` markers across 13 rules.
12. **Corpus relayout** — `corpus/snapshots/*` and the
    `reward_provenance/*_registered_creds.txt` files were removed (they
    carried credential material that does not belong in a public repo);
    12 boundary-block sets were re-extracted at exact era-boundary
    slots; the consensus corpus (`corpus/consensus/*`, N-B), the validity
    corpus (`corpus/validity/*`, B1), and the tx-validity adversarial
    README (`corpus/tx_validity/adversarial/`, B2) were added.

---

## 1. Commit Log

| Hash | Type | Summary |
|------|------|---------|
| `85a50dc` | feat | feat(tx-validity): B2-S5 mempool admission gate (Tier-1) — closes CE-B2-5 |
| `617139f` | feat | feat(tx-validity): B2-S4 adversarial tx corpus — closes CE-B2-4 (no false accept) + fixes a value-conservation fail-open |
| `4cffc2c` | feat | feat(tx-validity): B2-S3 positive tx corpus + replay — closes CE-B2-3 |
| `b24b22c` | feat | feat(tx-validity): B2-S2 tx_validity composition + verdict taxonomy — closes CE-B2-2 |
| `3e24d0b` | feat | feat(tx-validity): B2-S1 Conway vkey-witness + required-signer closure — closes CE-B2-1 |
| `7263699` | docs | docs(phase-4): PHASE4-B2 cluster doc — tx validity agreement |
| `b32fef3` | docs | docs(phase-4): PHASE4-B2 cluster/slice plan — 5-slice tx-validity-agreement arc |
| `b79f632` | docs | docs(phase-4): open PHASE4-B2 — tx validity agreement invariant sketch + DC-TXV family |
| `e0af99d` | chore | chore: gitignore multi-GB ledger-state dumps (belong in S3, not git) |
| `3552bc2` | chore | chore: sync Cargo.lock for PHASE4-B1 dependency edges |
| `993f363` | chore | Close PHASE4-B1 — full block validity agreement (validation core of workstream B) |
| `2630267` | feat | feat(validity): B1-S7 adversarial corpus — closes CE-B1-4 (no false accept) |
| `e394a82` | feat | feat(validity): B1-S6 positive agreement corpus + replay — closes CE-B1-3 |
| `7b95ccd` | feat | feat(validity): B1-S4 block_validity composition — closes CE-B1-2 + CE-B1-5 |
| `500589b` | feat | feat(validity): B1-S5 Praos single-VRF + KES header validation — 14/14 real Conway headers validate |
| `440ac72` | feat | feat(validity): B1-S3 BlockValidity verdict/error taxonomies + canonical surface encoding |
| `97a27cc` | feat | feat(validity): B1-S2 production LedgerView projection — closes CE-B1-1 |
| `a134379` | feat | feat(validity): B1-S1 consensus-input extractor + Conway-576 corpus |
| `b63f554` | docs | docs(phase-4): PHASE4-B1 cluster doc — full block validity agreement |
| `cb8165a` | docs | docs(phase-4): PHASE4-B1 cluster/slice plan — 7-slice full-block-validity arc |
| `c0acd59` | docs | docs(phase-4): open PHASE4-B1 — full block validity agreement invariant sketch + DC-VAL registry family |
| `e5f1f64` | feat | feat(interop): CE-N-B-6 follow-mode bridge + live preprod tip-agreement evidence |
| `807bcb6` | docs | docs(consensus): retarget N-B live-interop pin to cardano-node 11.0.1 |
| `a0c73e1` | chore | Close PHASE4-N-B — consensus runtime (Praos) authority + replay equivalence |
| `ad4d6f6` | feat | feat(consensus): S-B10 stream replay + orchestrator + live interop — closes CE-N-B-5 + CE-N-B-6 |
| `4f5cd7f` | feat | feat(consensus): S-B9 rollback authority — closes CE-N-B-2 |
| `8e991b5` | feat | feat(consensus): S-B8 fork choice + CandidateFragment — closes CE-N-B-1 |
| `e059652` | feat | feat(consensus): S-B7 Praos header validation |
| `f4c8369` | feat | feat(consensus): S-B6 leader schedule — closes CE-N-B-4 |
| `39cc143` | feat | feat(consensus): S-B5 op-cert counter monotonicity |
| `116eb57` | feat | feat(consensus): S-B4 nonce evolution authority |
| `70f60d9` | feat | feat(consensus): S-B3 VRF cert verification wiring + Praos VRF input + leader threshold |
| `ff01fe3` | feat | feat(consensus): S-B2 PraosChainDepState canonical type + closed event/error taxonomies |
| `fe68bb7` | feat | feat(consensus): S-B1 EraSchedule canonical authority + slot/era/time translation |
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
| `ade_ledger::tx_validity` (new submodule of an existing BLUE crate) | BLUE | **Per-transaction verdict authority** — the per-tx counterpart to `block_validity`. Closed `TxValidityVerdict` (`Valid { tx_id, applied: LedgerState }` / `Invalid { class, error }`), closed `TxRejectClass` (Phase1Invalid / WitnessInvalid / MissingRequiredSigner / Phase2Invalid / MalformedField) with a *total* `class()` mapping, and closed `TxValidityError`. `required_signers` enumerates, over a CLOSED era-versioned `SignerSource`, every `Hash28` key hash a Conway tx must witness (grounded in `getConwayWitsVKeyNeeded` / `getVKeyWitnessConwayTxCert`); `witness::verify_required_witnesses` checks each required key has a fail-closed Ed25519 witness over the preserved body hash. `tx_phase_one` composes the witness closure with the state-backed checks; `tx_validity` is the pure `(LedgerState, tx_cbor) → verdict` transition. Canonical `TxVerdictSurface` encode/decode for the replay/comparison surface. | `mod.rs` (re-exports), `verdict.rs` (`TxValidityVerdict`, `TxRejectClass`, `TxValidityError`), `required_signers.rs` (`SignerSource`, `required_signers`, `tx_derived_required_signers`, `ResolvedInputs`/`ResolvedOutput`), `witness.rs` (`verify_required_witnesses`, `WitnessClosureError`, `WitnessField`), `phase1.rs` (`tx_phase_one`, `decode_tx`, `DecodedTx`), `transition.rs` (`tx_validity`, `TxValidityOutcome`), `encoding.rs` (`encode_tx_verdict_surface`, `decode_tx_verdict_surface`, `TxVerdictSurface`) | PHASE4-B2 / B2-S1 (witness/required-signer closure), B2-S2 (composition + taxonomy) |
| `ade_ledger::mempool` (new submodule of an existing BLUE crate) | BLUE (`admit`) / GREEN (`policy`) | Mempool admission gate, two layers strictly separated by TCB color. `admit` (BLUE, Tier-1) admits a tx iff `tx_validity` is `Valid` against the mempool's accumulating `MempoolState`; no false accept, and on Invalid the mempool is unchanged. `policy` (GREEN, Tier-5) does deterministic eviction/ordering over already-admitted tx ids; it never calls `tx_validity` and provably cannot alter an admit verdict (it reads only the admitted-id list). | `mod.rs` (re-exports), `admit.rs` (`admit`, `AdmitOutcome`, `MempoolState`), `policy.rs` (`order`, `OrderPolicy`) | PHASE4-B2 / B2-S5 |
| `ade_testkit::tx_validity` (new submodule of an existing crate) | GREEN | Test-only tx-validity harness. Extracts every on-wire Conway tx from the committed Conway-576 corpus blocks and drives BLUE `tx_validity` over each (positive corpus, 103/103 Valid); supplies synthetic valid txs at controlled UTxO (`valid_synthetic.rs`); derives adversarial txs via deterministic mutators — family A (witness mutations W1–W4 on real corpus txs at `track_utxo=false`) and family B (synthetic value/input/witness mutations S1–S4 at `track_utxo=true`) — plus a judge (`adversarial.rs`). Non-authoritative. | `tx_validity/mod.rs`, `tx_validity/extract.rs`, `tx_validity/valid_synthetic.rs`, `tx_validity/adversarial.rs` | PHASE4-B2 / B2-S3, B2-S4 |
| `ade_ledger::block_validity` (new submodule of an existing BLUE crate) | BLUE | Full-block verdict authority: closed `BlockValidityVerdict`, closed `BlockValidityError` / `BlockRejectClass`, fail-closed `FieldKind` / `FieldError` taxonomy, and the `block_validity(...)` transition composing the N-B header authority with the body authority. Header validated before body (fail-fast). Canonical `VerdictSurface` for replay/comparison. | `mod.rs`, `verdict.rs`, `transition.rs`, `header_input.rs`, `encoding.rs` | PHASE4-B1 / B1-S3 (taxonomy), B1-S4 (composition) |
| `ade_ledger::consensus_view` (new file in an existing BLUE crate) | BLUE | Production `LedgerView` projection. `PoolDistrView` projects a `LedgerState`'s pool-distribution into exactly the four leadership-relevant facts BLUE consensus consumes through the `ade_core::consensus::LedgerView` boundary — total active stake, per-pool active stake, per-pool registered VRF keyhash, active-slots coefficient — and nothing else. | `consensus_view.rs` (`PoolDistrView`) | PHASE4-B1 / B1-S2 |
| `ade_ledger::consensus_input_extract` (new file in an existing BLUE crate) | RED | Tail-scan of a snapshot `state` CBOR for the five `PraosState` nonces. RED because it parses an external dump format; the scan is pure over input bytes and fail-closed (requires exactly five non-neutral nonces). | `consensus_input_extract.rs` | PHASE4-B1 / B1-S1 |
| `ade_core::consensus::kes_check` (new file in an existing BLUE crate) | BLUE | Fail-closed wiring of `ade_crypto::kes` into Praos header validation. `expect_size` rejects wrong-length crypto fields rather than skipping them (DC-VAL-06 fail-closed pattern). Adds the single-VRF + KES header verification path exercised by 14/14 real Conway headers. | `kes_check.rs` | PHASE4-B1 / B1-S5 |
| `ade_testkit::validity` (new submodule of an existing crate) | GREEN | Test-only block-validity harness: positive Conway-576 corpus replay over `block_validity`, corpus-backed `LedgerView`, deterministic adversarial mutators M1–M6. Non-authoritative. | `validity/mod.rs`, `validity/corpus.rs`, `validity/ledger_view.rs`, `validity/replay.rs`, `validity/adversarial.rs` | PHASE4-B1 / B1-S6, B1-S7 |
| `ade_core_interop::follow` (new file in an existing RED crate) | RED | Follow-mode bridge between a peer's ChainSync stream and BLUE fork-choice. Runs BLUE `select_best_chain` + `apply_rollback` ONLY; calls no header/VRF/leader/nonce/KES validation. Asserts tip-selection agreement with an already-validated peer. Carries no authoritative decision. | `follow.rs`, `tests/follow_offline_replay.rs` | CE-N-B-6 follow-bridge (`e5f1f64`) |
| `ade_network` (new workspace crate) | BLUE-majority (per-submodule scoped in `.idd-config.json` `core_paths`) | Ouroboros mini-protocol authority: 11 closed-grammar codecs, 8 pure transition state machines, Ouroboros mux frame codec, RED session/transport substrate. Wire bytes are Tier 1. Sync-only in BLUE submodules (DC-CORE-01); tokio confined to `mux::transport`. | `codec/` (11 codecs); `handshake/`; `chain_sync/`, `block_fetch/`, `tx_submission/`, `keep_alive/`, `peer_sharing/`; `n2c/`; `mux/frame.rs` (BLUE), `mux/transport.rs` (RED); `session/` (RED); RED capture binaries | PHASE4-N-A / S-A1 → S-A10 |
| `ade_core::consensus` (new submodule of an existing BLUE crate) | BLUE | Praos consensus authority: closed `PraosChainDepState`, era-aware slot/time translation, header validation, nonce evolution, op-cert monotonicity, leader schedule, fork choice, rollback. Closed `ChainEvent` / `ChainSelectionReject` taxonomies; flat-data errors. No async, no ChainDb, no floats. | `mod.rs`, `era_schedule.rs`, `header_validate.rs`, `vrf_cert.rs`, `nonce.rs`, `op_cert.rs`, `leader_schedule.rs`, `fork_choice.rs`, `rollback.rs`, `kes_check.rs` (B1), `praos_state.rs`, `candidate.rs`, `events.rs`, `errors.rs`, `encoding.rs`, `ledger_view.rs`, `header_summary.rs` | PHASE4-N-B / S-B1 → S-B9 (`kes_check` from B1) |
| `ade_runtime::consensus` (new submodule of an existing RED crate) | GREEN/RED mix | Imperative-shell composition for consensus: stream-driven orchestrator (GREEN), candidate-fragment builder, RED genesis parser (genesis JSON → BLUE `EraSchedule`). | `mod.rs`, `candidate_fragment.rs`, `chain_selector.rs`, `genesis_parser.rs` | PHASE4-N-B / S-B8, S-B10 |
| `ade_core_interop` (new workspace crate) | RED | Live cardano-node interop driver for CE-N-B-6. No authoritative decisions; readiness probe, `live_consensus_session` binary, and (B1-era) the `follow` bridge. CI does not run it by default (`#[ignore]`-gated / offline-replay only). | `src/lib.rs`, `src/follow.rs`, `src/bin/live_consensus_session.rs`, `tests/` | PHASE4-N-B / S-B10; follow-bridge `e5f1f64` |
| `ade_testkit::consensus` (new submodule of an existing crate) | GREEN | Test-only harness for consensus replay corpora (`corpus/consensus/*`): JSON fixture loader, `LedgerView` stub, `consensus_stream_replay` driver. | `consensus/mod.rs`, `consensus/corpus.rs`, `consensus/ledger_view_stub.rs`, `consensus/stream_replay.rs` | PHASE4-N-B / S-B1, S-B6, S-B8 → S-B10 |
| `ade_runtime::chaindb` | RED | Block-store abstraction and impls. Trait surface Tier 1; backing-store choice and on-disk layout Tier 5. | `mod.rs`, `types.rs`, `error.rs`, `in_memory.rs`, `persistent.rs` (redb), `contract.rs`, `snapshot_contract.rs`, `crash_safety.rs` | PHASE4-N-D / S-33 → S-35 |
| `ade_runtime::recovery` | RED | Composes ChainDb + SnapshotStore into a generic recovery primitive: load latest snapshot, replay forward to tip. | `recovery.rs` | PHASE4-N-D / S-36 |
| `ade_runtime` bin `chaindb_kill_target` | RED | Kill-target child process for the 1,000-kill-9 durability stress harness. | `src/bin/chaindb_kill_target.rs`, `tests/stress_kill_harness.rs` | PHASE4-N-D / S-37 |

Workspace-level membership grew by **two crates** across the full
delta: `ade_network` (PHASE4-N-A) and `ade_core_interop` (PHASE4-N-B).
Both are RED-or-mixed.

Crate dependency shape at HEAD (new deps in this delta):
- `ade_core` gained `ade_types`, `ade_crypto`, `minicbor`, and
  `ade_codec` (B1) as deps; dev deps `ade_testkit`, `serde_json`,
  `cardano-crypto` (`vrf-draft03`).
- **`ade_ledger` gained an `ade_core` dep edge** (PHASE4-B1) so the
  block-validity composition can call the consensus header authority,
  plus `minicbor` (dep) and `ade_testkit` (dev). PHASE4-B2 added no new
  ledger manifest deps — `tx_validity` and `mempool` compose existing
  ledger surfaces (`conway`, `rules`, `shelley`, `witness`,
  `plutus_eval`) plus `ade_types`/`ade_crypto` already on the crate.
- `ade_runtime` gained `ade_core`, `ade_crypto`, `ade_codec`,
  `serde_json` (deps); `ade_testkit`, `cardano-crypto` (dev); the N-D
  deps `redb = "2"` (Tier 5) and `tempfile = "3"` (dev).
- `ade_testkit` gained `ade_core`, `ade_runtime` (deps);
  `cardano-crypto` (dev).
- `ade_core_interop` is new.

Corpora at HEAD: N-A capture corpus under `corpus/network/{n2n,n2c}/`;
N-B replay corpus under `corpus/consensus/`; B1 validity corpus under
`corpus/validity/` (positive Conway-576 blocks; the adversarial half is
*derived at test time*, only a README is committed). The B2
tx-adversarial corpus is likewise *derived* — `corpus/tx_validity/
adversarial/` carries only a README documenting the W1–W4 / S1–S4
mutation table, the per-mutation expected reject class, and the **S1
value-conservation finding** that surfaced the conway fail-open.

Cross-reference: CODEMAP must be regenerated to add per-submodule
entries for the new B2 surfaces (`ade_ledger::tx_validity`,
`ade_ledger::mempool`, `ade_testkit::tx_validity`) in addition to the
still-unrecorded N-B + B1 surfaces. SEAMS must record the tx-validity
seam (closed `TxValidityVerdict` / `TxRejectClass` / `TxValidityError`,
the era-versioned `SignerSource`, the `TxVerdictSurface` comparison
surface, and the BLUE/GREEN mempool admission/policy boundary) plus the
block-validity and N-B consensus seams. TRACEABILITY must add rows for
the 5 `DC-TXV-*`, 2 `DC-MEM-*`, 8 `DC-CONS-*`, and 6 `DC-VAL-*` rules.
**All three grounding docs are stale on N-B + B1 + B2 as of HEAD** —
the last grounding-doc refresh was `744ef34` (N-A only).

---

## 3. Modules Modified

| Module | Scope | Key changes |
|--------|-------|-------------|
| `ade_ledger` | +38 source/test files, +5,645 / −5 lines over the full delta (of which **PHASE4-B2: +24 files, +3,817 / −5 lines**; PHASE4-B1: +13 files, +1,755 lines; CE-73: +73 lines) | **PHASE4-B2 (primary thread of this regen):** the crate gained the `tx_validity/` submodule (7 files: `mod`, `verdict`, `required_signers`, `witness`, `phase1`, `transition`, `encoding`) and the `mempool/` submodule (3 files: `mod`, `admit`, `policy`), plus integration tests `tx_witness_closure.rs` (697 lines), `tx_validity_compose.rs` (472), `tx_validity_positive_corpus.rs`, `tx_validity_adversarial_corpus.rs`, `mempool_admission.rs`. It also touched existing ledger files to wire the closure and the value-conservation fix: **`conway.rs`** (+83/−; added `check_conway_coin_conservation`, the no-false-accept fix), **`rules.rs`** (+187; `verify_conway_witness_closure` body-path wiring + the `track_utxo` phase-1 split), **`shelley.rs`** (+77), `error.rs` (+13; `ConservationError`), `utxo.rs`, `witness.rs`, `phase.rs`, `lib.rs`. **PHASE4-B1:** the `block_validity/` submodule, `consensus_view.rs`, `consensus_input_extract.rs`, the new `ade_core` dep edge. **CE-73:** 10 unit tests for `decode_invalid_tx_indices`, no production change. |
| `ade_core` | +15 source files + 10 tests (N-B, +7,334 lines); +828 / −86 across 16 files (B1) | **PHASE4-N-B:** stub `lib.rs` → substantive BLUE consensus module under `src/consensus/`. **PHASE4-B1:** added `consensus/kes_check.rs` (fail-closed `expect_size` + KES header guard); wired single-VRF + KES header validation across `header_validate.rs`, `vrf_cert.rs`, `leader_schedule.rs`, `fork_choice.rs`, etc. (14/14 real Conway headers validate). New dep `ade_codec` (B1). **No B2 source change** — tx-validity composes only `ade_ledger` surfaces. |
| `ade_crypto` | 1 file, +24 / −81 lines (B1) | Single change in `kes.rs` (`500589b`): **`build_opcert_signable` fixed** as part of B1-S5 KES header validation. No source change in N-A, N-B, N-D, or B2. |
| `ade_core_interop` | +1,426 / −37 across 6 files (B1) | **CE-N-B-6 follow-bridge (`e5f1f64`) + pin retarget (`807bcb6`):** RED `follow.rs` (BLUE fork-choice + rollback only) + `follow_offline_replay.rs`; reworked `lib.rs`, the live-session binary and test. New deps `ade_codec`, `ade_crypto` for offline replay. |
| `ade_network` (existing crate, refined) | 6 source files, +8 / −8 lines | **DoS hardening of 6 codecs** (`744ef34`, post-N-A close): capped untrusted `Vec::with_capacity` hints. No transition-authority change since N-A closure; no change in N-B/B1/B2. |
| `ade_runtime` | +4 source files in `consensus/`, +1,273 lines (N-B); +1 file, +7/−3 (B1) | **PHASE4-N-B:** new `consensus/` submodule (`candidate_fragment.rs`, `chain_selector.rs`, `genesis_parser.rs`) + corpus test. **PHASE4-B1:** one small touch. The N-D `chaindb`/`recovery` submodules + kill-target binary are §2 New Modules. No B2 change. |
| `ade_testkit` | +4 files `consensus/` (N-B); +5 files `validity/`, +1,024/−9 (B1); **+4 files `tx_validity/`, +1,129 lines (B2)** | **PHASE4-N-B:** `consensus/` harness. **PHASE4-B1:** `validity/` harness (M1–M6 mutators). **PHASE4-B2:** new `tx_validity/` submodule (`mod.rs`, `extract.rs` 191 lines, `valid_synthetic.rs` 251, `adversarial.rs` 608) hosting the on-wire-tx extractor, synthetic valid builders, and the W1–W4 / S1–S4 adversarial mutators + judge. New deps `ade_core`, `ade_runtime` (B1). |

No other crate had non-trivial source changes since baseline.
`ade_codec`, `ade_types`, `ade_plutus`, and `ade_node` were untouched
by code commits. `.idd-config.json` had prose edits during the delta;
the `core_paths` array already covers the new `tx_validity` /
`mempool` / `block_validity` / `consensus` surfaces via the
already-listed `ade_ledger` / `ade_core` crate prefixes.

---

## 4. Feature Flags

No Cargo `[features]` tables exist at HEAD in any workspace crate, and
none existed at baseline. The project does not use Cargo feature flags
as a semantic surface — closed semantic surfaces are encoded in the
type system per the IDD core principles, and conditional compilation is
checked out of BLUE code via `ci/ci_check_no_semantic_cfg.sh` (scoped
over the full 6-crate BLUE set; the `ade_ledger::tx_validity`,
`ade_ledger::mempool`, `ade_core::consensus`, and
`ade_ledger::block_validity` surfaces are covered by their crate-level
scope).

No `#[cfg(feature = ...)]` gates appear at either ref. `cardano-crypto`
(`vrf-draft03`) and `minicbor` (`alloc`) feature selections in the
dependency entries are upstream-crate selections, not Ade-side flags.

**Status: unchanged — zero Ade feature flags at baseline, zero at HEAD.**

---

## 5. CI Checks

The CI surface is the shell-script set under `ci/` (no
`.github/workflows` in this repo). At baseline there were 15 scripts.
At HEAD there are **24 scripts plus one git hook**: CE-73 added one
(`ci_check_hfc_translation.sh`), N-D added three, N-A added two, N-B
added four, and one repo-local git hook (`ci/git-hooks/commit-msg`).
**PHASE4-B1 and PHASE4-B2 each added no new CI script** — both reused
and extended the one N-B closed-enums script. Grouped by cluster.

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
| `ci/ci_check_ingress_chokepoints.sh` | Modified — BLUE-scope + registry growth (`5b70bee`) | No raw CBOR decoding outside named chokepoints in BLUE. `T-INGRESS-01`, `DC-INGRESS-01`. |
| `ci/ci_check_dependency_boundary.sh` | Modified — BLUE-scope (`5b70bee`) | BLUE crates must not depend on RED crates. `T-BOUND-02`. |

Follow-up `c8fa37f` re-ran CODEMAP and TRACEABILITY against the new
scope, removing 14 `_(scope gap)_` markers across 13 rules.

### Phase 4 N-D CI gap closure (`78da6c9`)

| Check | Status | What it checks |
|-------|--------|----------------|
| `ci/ci_check_chaindb_contract.sh` | **New** (`78da6c9`) | `cargo test -p ade_runtime --lib chaindb::` — 8 contract tests. `DC-STORE-02/03`, `CN-STORE-04/05`. |
| `ci/ci_check_recovery_contract.sh` | **New** (`78da6c9`) | `cargo test -p ade_runtime --lib recovery::` — 6-test recovery bundle. `T-REC-01/02`, `DC-STORE-05`. |
| `ci/ci_check_chaindb_crash_safety.sh` | **New** (`78da6c9`) | Smoke variant of the subprocess-SIGKILL harness + integrity post-checks. `T-REC-01`, `DC-STORE-01`, `CN-STORE-03`. |

### Phase 4 N-A wire + semantic enforcement (S-A1, S-A10)

| Check | Status | What it checks |
|-------|--------|----------------|
| `ci/ci_check_no_async_in_blue.sh` | **New** (`4fde3a7`, S-A1) | Enforces `DC-CORE-01` — BLUE code is sync-only. Scans every BLUE path in `core_paths`. Covers `ade_core::consensus`, `ade_ledger::block_validity`, `ade_ledger::tx_validity`, and `ade_ledger::mempool` via crate prefixes. |
| `ci/ci_check_ce_n_a_5_proof.sh` | **New** (`56bfa7b`, S-A10) | CE-N-A-5 closure-gate evidence over the real-cardano-node corpus. |

### Phase 4 N-B consensus authority enforcement (S-B1, S-B2, S-B8) — extended by B1 and B2

Four BLUE-scope CI scripts targeting `crates/ade_core/src/consensus/`.
The closed-enums script was **extended in PHASE4-B1** to also scan
`ade_ledger::block_validity`, then **extended again in PHASE4-B2**
(`617139f` / `85a50dc`) to also scan `ade_ledger::tx_validity` and
`ade_ledger::mempool`.

| Check | Status | What it checks |
|-------|--------|----------------|
| `ci/ci_check_consensus_closed_enums.sh` | **New** (N-B); **Modified** (B1, `7b95ccd`); **Modified** (B2) | Four-part scan now over `ade_core/src/consensus/`, `ade_ledger/src/block_validity/`, **`ade_ledger/src/tx_validity/`, and `ade_ledger/src/mempool/`**: no `#[non_exhaustive]`; no open-tail `Other`/`Unknown`; no owned `String` in the error/encoding/verdict files (B2 added `tx_validity/{required_signers,witness,verdict,phase1,transition}.rs` and `mempool/{admit,policy}.rs` to the string-scope list); no `Box<dyn ...>`. Strengthens `DC-CONS-04/10`, `T-DET-01`, the `DC-VAL-*` block-validity rules, **and now `DC-TXV-01/02/04/05` + `DC-VAL-06` (tx-validity: `SignerSource` / `RequiredSignerError` / `WitnessClosureError` / `TxValidityVerdict` / `TxRejectClass` / `TxValidityError` stay closed) and `DC-MEM-01/02` (`AdmitOutcome` / `OrderPolicy` stay closed)**. |
| `ci/ci_check_no_chaindb_in_consensus_blue.sh` | **New** (N-B / S-B1) | No `ChainDb`/`chain_db` token in `consensus/`. Strengthens `DC-CORE-01`, `DC-CONS-07`. |
| `ci/ci_check_no_density_in_fork_choice.sh` | **New** (N-B / S-B8) | No `density` token in `fork_choice.rs` / `candidate.rs`. Strengthens `DC-CONS-03`. |
| `ci/ci_check_no_float_in_consensus.sh` | **New** (N-B / S-B1) | No `f32`/`f64` in `consensus/`. Strengthens `T-CORE-02`, `DC-CONS-07/08/09`. |

TRACEABILITY cross-reference: the four N-B scripts map to the 8
`DC-CONS-*` rules; the closed-enums script now also enforces four
`DC-VAL-*`, four `DC-TXV-*` (01/02/04/05), and both `DC-MEM-*` rules.
**Unlike B1, the B2 registry rows now carry a populated `ci_script`
field** — all five `DC-TXV-*` and both `DC-MEM-*` rules name
`ci/ci_check_consensus_closed_enums.sh` (plus `DC-TXV-05`/`-03` lean
heavily on named integration tests). None of the new N-B / B1 / B2 rows
exist in TRACEABILITY yet (last refresh `744ef34`, N-A only).

---

## 6. Canonical Type Registry Delta

n/a — `.idd-config.json` `canonical_type_registry` is null. Canonical-type
rules live inline in the invariant registry under family `T`.

---

## 7. Normative Rule Delta

The project's invariant registry tracks structured rules (TOML), not
prose normative-doc rules; this section reports on it.

- Rules at baseline: **147** (in `constitution_registry.toml`)
- Rules at HEAD: **168** (in `docs/ade-invariant-registry.toml`)
- Net additions: **+21** (PHASE4-N-A: 2; PHASE4-N-B: 8; PHASE4-B1: 6;
  PHASE4-B2: 5). The two `DC-MEM-*` rules were *introduced earlier*
  (`2047c42`, IDD canonicalization, `status = "declared"`) and were
  not new in B2 — B2 *flipped them to `enforced`* (see Strengthenings).
  - PHASE4-N-A: `DC-CORE-01`, `DC-PROTO-06`.
  - PHASE4-N-B (`d9f0426`): `DC-CONS-03` → `DC-CONS-10` (8 rules:
    Praos `(BlockNo, TiebreakerView)` ordering, `PraosChainDepState`
    ownership, rollback bounded by k, rollback = truncated replay, HFC
    schedule as typed `EraSchedule`, `slot_to_time` purity,
    `OutsideForecastRange`, op-cert monotonicity).
  - PHASE4-B1 (`c0acd59`, `DC-VAL` family): `DC-VAL-01` → `DC-VAL-06`
    (block verdict purity, header∧body composition, header-before-body
    fail-fast, oracle agreement over positive + adversarial corpora,
    Valid→evolved/Invalid→unchanged, fail-closed crypto/field checks).
  - **PHASE4-B2 (`b79f632`, new `DC-TXV` family — 5 rules, all
    `enforced` at HEAD):**
    - **`DC-TXV-01`** — `tx_validity` is a pure function of
      `(LedgerState, tx_cbor)`; no wall-clock / arrival-order /
      HashMap-iteration / float / ambient influence. Tests:
      `tx_validity_is_deterministic`. ci_script
      `ci_check_consensus_closed_enums.sh`. cross_ref `DC-VAL-01`,
      `DC-CORE-01`, `T-DET-01`.
    - **`DC-TXV-02`** — Valid iff *both* phase-1 (structural + UTxO +
      witnesses) and phase-2 (Plutus, when present) accept; no Valid
      verdict may skip either phase (fail-fast: phase-1 decided first).
      Tests: `valid_tx_is_valid_and_applies`,
      `phase1_failure_short_circuits_phase2`.
    - **`DC-TXV-03`** — Ade's verdict (incl. reason class) equals the
      cardano-node verdict over a positive corpus (real on-chain txs)
      and a mandatory adversarial corpus; a false-accept is
      release-blocking. Positive half B2-S3 (`all_corpus_txs_valid`,
      103/103); negative half B2-S4 (family A witness mutations +
      family B synthetic value/input/witness mutations, 220 cases all
      Invalid).
    - **`DC-TXV-04`** — Valid → applied `LedgerState'` (the mempool's
      accumulating view); Invalid → unchanged input state + structured
      reason; no partial/in-place mutation. Tests:
      `invalid_tx_leaves_state_unchanged`, `class_mapping_is_total`.
    - **`DC-TXV-05`** — per-era `required_signers` is a closed,
      explicit, era-versioned function over every signer source;
      incomplete enumeration is a forbidden false-accept path. 13
      named tests in `tx_witness_closure.rs`.
- Removals: **0** (expected under append-only discipline; clean).
- Strengthenings (`declared` → `enforced`, or tightened):
  - **`DC-MEM-01`, `DC-MEM-02`** (`85a50dc`, B2-S5): `declared` →
    `enforced` (`strengthened_in = ["PHASE4-B2"]`). DC-MEM-01: mempool
    acceptance must not contradict block/ledger acceptance — tests
    `valid_tx_admitted_and_accumulates`,
    `invalid_tx_rejected_no_false_accept`,
    `admission_equals_tx_validity_verdict`,
    `dependent_tx_admitted_against_accumulating_state`. DC-MEM-02:
    overload shedding follows deterministic policy.
  - **`DC-TXV-03`, `DC-VAL-06`, `DC-LEDGER-02`** strengthened by B2-S4
    (`617139f`): `code_locus` + `tests` extended for the negative
    adversarial half and the value-conservation fix. The five
    `DC-TXV-*` rules and B2 made registry **`cross_ref` bidirectional**
    and populated `ci_script` on the enforced rules.
  - Earlier-delta strengthenings (unchanged from prior regen):
    `DC-EPOCH-02` (`9b15378`); the N-D bundle
    (`T-REC-01/02`, `DC-STORE-01/02/03/05`, `CN-STORE-03/04/05`,
    `78da6c9`); the N-A real-capture bundle (`T-ENC-03`, `CN-WIRE-07`,
    `DC-PROTO-02/05`); `T-CORE-02` (S-B1).
  - The six `DC-VAL-*` rules each carry `strengthened_in =
    ["PHASE4-B1"]` and the five `DC-TXV-*` rules each carry
    `strengthened_in = ["PHASE4-B2"]` even though each family was
    *created* in its own cluster — recorded faithfully; see Anomalies.

Family counts at HEAD: CN dominates (~64), DC grew most this delta
(now including `DC-CONS` ×8, `DC-VAL` ×6, `DC-TXV` ×5, `DC-MEM` ×2),
T = 30, RO/OP combined ×9.

Normative-doc rule extraction (the `normative_docs` list in
`.idd-config.json`) is approximate and not regenerated here — the
structured registry is the authoritative source.

---

## Anomalies and Cross-Reference Warnings

- **PHASE4-B2 reported closed (all 5 CEs green) but the cluster doc is
  not yet archived and no `Close PHASE4-B2` commit exists.** The
  cluster doc lives at `docs/clusters/PHASE4-B2/` (opened `b79f632` /
  `b32fef3` / `7263699`); `docs/clusters/completed/` exists but does
  not contain `PHASE4-B2/`. CEs: CE-B2-1 `3e24d0b`, CE-B2-2 `b24b22c`,
  CE-B2-3 `4cffc2c`, CE-B2-4 `617139f`, CE-B2-5 `85a50dc`. The
  `Close PHASE4-B2` commit is the upcoming housekeeping step — surface
  for the next commit.
- **Conway value-conservation fail-open found + fixed mid-cluster
  (`617139f`, B2-S4).** A genuine **false-accept** — the
  Conway/Babbage/Alonzo state-backed path never checked coin-level
  value conservation, so a tx with `outputs + fee != inputs` was
  accepted as Valid at `track_utxo=true`. Surfaced by the family-B
  adversarial S1 case, fixed by `check_conway_coin_conservation`. This
  is a BLUE behavioral correctness change; four happy-path conway
  fixtures were rebalanced. Confirm `ci_check_differential_divergence.sh`
  / `ci_check_ledger_determinism.sh` still cover the new equation on the
  next TRACEABILITY pass.
- **Deposit/refund/withdrawal value accounting is deliberately
  DEFERRED.** `check_conway_coin_conservation` returns early for txs
  carrying certs or withdrawals, so it can never false-*reject*, but
  the residual value-imbalance for deposit/withdrawal-bearing txs is
  **not yet caught**. This is a named tx-validity-completeness
  follow-up (tracked in `corpus/tx_validity/adversarial/README.md`),
  not a discipline violation — it is fail-closed in the sense that
  matters (no false reject) but is a known gap in no-false-accept
  coverage for that tx shape. Surface for the next B-series cluster.
- **Grounding docs stale on N-B + follow-bridge + B1 + B2.** CODEMAP,
  SEAMS, and TRACEABILITY were last refreshed at `744ef34` (N-A only).
  They carry **no** entries for: `ade_ledger::tx_validity`,
  `ade_ledger::mempool`, `ade_testkit::tx_validity` (B2);
  `ade_ledger::block_validity`, `consensus_view`,
  `consensus_input_extract`, `ade_testkit::validity` (B1);
  `ade_core::consensus` + `kes_check`, `ade_runtime::consensus`,
  `ade_testkit::consensus`, the `ade_core_interop` crate + `follow`
  bridge (N-B). TRACEABILITY is missing rows for all 5 `DC-TXV-*`, 2
  `DC-MEM-*` (now enforced), 8 `DC-CONS-*`, and 6 `DC-VAL-*` rules. The
  CI-script count in CODEMAP must read 24 scripts + 1 hook. Run
  `/codemap`, `/seams`, `/traceability`.
- **DC-VAL status mismatch vs. closure claim (B1, carried forward).**
  PHASE4-B1 is reported fully closed, but in the registry only
  `DC-VAL-01` is `enforced` — `DC-VAL-02` → `DC-VAL-06` remain
  `declared` despite named tests and the extended closed-enums
  enforcement point. Contrast with B2, where all 5 `DC-TXV-*` *are*
  `enforced`. Flip the DC-VAL statuses on the next `/traceability` pass
  or confirm at the (pending) B1/B2 cluster-close gate.
- **`strengthened_in` records the introducing cluster on freshly-created
  rules.** Each `DC-VAL-*` records `["PHASE4-B1"]` and each `DC-TXV-*`
  records `["PHASE4-B2"]` even though those clusters *created* the
  families. `strengthened_in` ordinarily records *later* clusters that
  tightened a pre-existing rule; harmless (no weakening), but consider
  normalizing on the next registry curation pass.
- **`ade_ledger -> ade_core` dependency edge (B1, carried forward).**
  First ledger→consensus edge. Both BLUE, so the BLUE→RED guard is
  unaffected; CODEMAP's dependency graph and SEAMS' module-addition
  rules should record the intra-BLUE edge.
- **`ade_crypto::kes::build_opcert_signable` fixed in B1-S5
  (`500589b`).** BLUE crypto-surface behavioral change; confirm
  `ci_check_crypto_vectors.sh` still covers it on the next
  TRACEABILITY pass.
- **Adversarial corpora are derived, not committed.** Both
  `corpus/validity/adversarial/` (B1) and
  `corpus/tx_validity/adversarial/` (B2) hold only a README; the
  adversarial blocks/txs are generated deterministically at test time
  by the mutators in `ade_testkit::validity::adversarial` /
  `ade_testkit::tx_validity::adversarial`. Intentional (keeps the
  corpus a pure function of the positive corpus) — the no-false-accept
  evidence lives in test code, not fixtures.
- **PHASE4-N-A / N-B / N-D / B1 closed.** N-A `69a2862` (+`744ef34`);
  N-B `a0c73e1`; N-D `436b1d7`; B1 `993f363`.
- **`ade_core_interop` tests `#[ignore]`-gated / offline-replay by
  design.** Live tip-agreement not run in CI; CE-N-B-6 closure evidence
  is a manual operator pass. By design (RED, no authority).
- **Corpus relayout: credentialed snapshots removed.** Deleted
  `corpus/snapshots/reward_provenance/*_registered_creds.txt` dominates
  the ~7M-line negative line count; replaced by 12 re-extracted
  boundary-block sets.
- No removed canonical types (n/a — no separate registry).
- No removed registry rules (expected: 0; actual: 0).
- **All commit subjects carry a conventional-commits prefix or are
  cluster-close housekeeping.** The three `Close PHASE4-*` commits
  (`69a2862`, `436b1d7`, `a0c73e1`, `993f363`) and the two bare
  `chore:` commits (`3552bc2`, `e0af99d`) are classified `chore` on
  scope grounds. No unclassifiable subjects.

---

## Generation Notes

Regenerate via `/head-deltas <baseline>` or by re-running the
`head-deltas-generator` agent with the same baseline. Baseline lives in
`.idd-config.json` `head_deltas_baseline`. Update on next phase
boundary (Phase 4 close, or when the next cluster closes). Note the
commit-hash rewrite caveat at the top — re-derive hashes from
`git log` at each regen rather than carrying them forward.
