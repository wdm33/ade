# HEAD Deltas — Ade

> **Status:** Living architectural document. Regenerated; not hand-edited.
> Regenerate via `/head-deltas <baseline>`. Baseline is declared in
> `.idd-config.json` (`head_deltas_baseline`).

> Baseline: `d509f02` (Phase 3 handoff snapshot, 2026-04-15)
> HEAD: `7784bf8` (test(tx-validity): PHASE4-B3 conservation corpora — real epoch-576 positive + adversarial, 2026-05-20)
> 91 commits, 11,224 files changed, +162,215 / −7,233,365 lines

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

> **In-flight close note.** This regen is cut at committed HEAD
> `7784bf8` (the third and last PHASE4-B3 commit). The PHASE4-B3
> cluster-close housekeeping is **in flight in the working tree** —
> uncommitted edits to `docs/ade-invariant-registry.toml` and the four
> grounding docs (`CODEMAP`, `SEAMS`, `TRACEABILITY`, this file) plus
> four `docs/clusters/PHASE4-B3/*` slice docs. The committed registry
> at HEAD marks `DC-TXV-06` as `enforced`; the in-flight close edit
> reclassifies it to **`partial`** (the closed-cert-classification
> grep-gate CI is a named follow-up, see §5/§7/Anomalies). This regen
> reports the **in-flight `partial` status** as the authoritative
> intent and flags the committed-vs-working-tree discrepancy in
> Anomalies. No `Close PHASE4-B3` commit exists yet.

The delta covers thirteen threads of work. The newest thread — the
**PHASE4-B3 Conway value-conservation accounting arc** — landed on top
of the PHASE4-B2 close (`c1cba82`). In rough proportion of the
substantive change budget:

1. **Phase 4 cluster B3 (Conway value-conservation accounting) —
   committed; cluster-close in flight.** Three commits since the B2
   close: the planning commit `3aebbe5` (invariants, cluster/slice
   plan, registry rules `DC-TXV-06`/`DC-TXV-07`), the implementation
   `978c222` (full Conway value-conservation accounting — **removes the
   cert/withdrawal early-out** that B2-S4 left as a conservative
   deferral), and the corpora commit `7784bf8` (real epoch-576 positive
   + synthetic + adversarial conservation corpora, no false accept). It
   **closes the deferred value-conservation gap** named at the B2 close:
   the full Conway preservation-of-value equation —
   `Σ(inputs) + Σ(withdrawals) + refunded_deposits == Σ(outputs) + fee
   + donation + new_deposits` (i128, no float) — is now enforced for
   **cert- and withdrawal-bearing txs**, and the release-blocking
   false-accept early-out is gone. New BLUE surfaces: the closed Conway
   cert decoder `ade_codec::conway::cert` (grammar tags 0..18,
   `CodecError::UnknownCertTag`), the withdrawals decoder
   `ade_codec::conway::withdrawals` (`RewardAccount`, i128 sum,
   `CodecError::DuplicateMapKey`), and the closed cert-deposit
   classifier `ade_ledger::cert_classify`. Two new registry rules:
   `DC-TXV-06` (closed cert-deposit classification — status **partial**,
   tests + exhaustive match present, the grep-gate CI is the follow-up)
   and `DC-TXV-07` (canonical deposit-parameter authority — `enforced`
   via the new `ci_check_deposit_param_authority.sh`). Strengthens
   `T-CONSERV-01`/`CN-LEDGER-07`, `DC-VAL-06`, and `DC-TXV-03`.
2. **Phase 4 cluster B2 (tx validity agreement) — closed (`c1cba82`).**
   A 5-slice arc (B2-S1 → B2-S5) shipped as `feat(tx-validity):`
   commits, opened by the planning trio `b79f632` (invariant sketch +
   `DC-TXV` family), `b32fef3` (cluster/slice plan), `7263699`
   (cluster doc), and closed by `c1cba82` (close commit +
   grounding-doc refresh). It is the **per-transaction** counterpart to
   B1's per-block verdict: it introduces the new BLUE
   `ade_ledger::tx_validity` submodule (vkey-witness + required-signer
   closure, phase-1 composition, closed verdict taxonomy, canonical
   verdict surface) and the BLUE/GREEN `ade_ledger::mempool` admission
   gate. **All 5 CEs closed** (CE-B2-1 vkey-witness/required-signer
   closure, CE-B2-2 tx_validity composition + verdict taxonomy, CE-B2-3
   positive corpus replay with 103/103 real Conway txs Valid, CE-B2-4
   adversarial corpus no-false-accept, CE-B2-5 mempool admission gate).
   The arc added 5 `DC-TXV-*` rules, flipped the two `DC-MEM-*` rules to
   `enforced`, and — critically — **found and fixed a real
   value-conservation fail-open** (`617139f`, B2-S4) whose deferred
   deposit/withdrawal residual B3 has now closed (thread 1).
3. **Conway value-conservation: the B2-S4 fail-open and its B3
   completion.** B2-S4 (`617139f`) added
   `ade_ledger::conway::check_conway_coin_conservation`, but with a
   **deliberate early-out** — it returned before checking value for any
   tx carrying certs or withdrawals (so it could never false-*reject*,
   but the deposit/withdrawal residual was uncaught). B3-S4 (`978c222`)
   **removes that early-out** and replaces the deposit-free form with
   the full equation, sourcing every deposit/refund/withdrawal term from
   canonical ledger state (`ProtocolParameters.{key_deposit,pool_deposit}`
   + `LedgerState.conway_deposit_params`), classified by the closed
   `ade_ledger::cert_classify::classify` over the closed `ConwayCert`
   grammar. The named tx-validity-completeness follow-up from the B2
   regen is therefore **discharged** for Conway cert/withdrawal txs.
4. **Phase 4 cluster B1 (full block validity agreement) — closed
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
5. **Phase 4 cluster N-A (network mini-protocols) — closed.** 10
   slices (S-A1 → S-A10, with S-A8b/S-A8c rework). Introduced the new
   BLUE workspace crate `ade_network` with 11 mini-protocol codecs, 8
   state machines, the Ouroboros mux frame codec, a RED `session`
   substrate. Closed CE-N-A-1 → CE-N-A-5 against pinned cardano-node
   11.0.1, including a real-capture corpus at `corpus/network/{n2n,n2c}/`.
   Three wire-form codec bugs surfaced by real interop were fixed in
   flight, plus an LSQ Acquire/AcquireNoPoint split, a
   LocalTxSubmission/N2N TxSubmission2 inner-tx HFC envelope fix, and
   DoS-hardening on `Vec::with_capacity` in eight codecs.
6. **Phase 4 cluster N-B (consensus runtime) — closed (`a0c73e1`).**
   10 slices (S-B1 → S-B10), opened by `d9f0426` (invariant sketch v2 +
   8 `DC-CONS-*` rules). Built out the BLUE `ade_core::consensus` module
   (15+ source files: closed `PraosChainDepState`, `EraSchedule`,
   fork-choice, rollback, nonce/op-cert/leader-schedule/VRF/header
   validation, `CandidateFragment`, structured event/error taxonomies).
   GREEN `ade_runtime::consensus` shipped the chain-selector
   orchestrator, candidate-fragment builder, and a RED genesis parser.
   New replay corpora under `corpus/consensus/`. All 6 CEs closed.
7. **CE-N-B-6 follow-mode bridge** — `807bcb6` retargeted the N-B
   live-interop pin to cardano-node 11.0.1, then `e5f1f64` added the RED
   `ade_core_interop::follow` bridge plus live preprod tip-agreement
   evidence. Follow mode runs BLUE fork-choice + rollback only — it
   trusts the already-validated peer for header/VRF/leader/nonce/KES, so
   it carries no authoritative validation decision.
8. **Phase 4 cluster N-D (ChainDB persistence) — closed (`436b1d7`).**
   Slices S-33 → S-37. CE-N-D-1 closure evidence (1000/1000 stress-kill
   iterations).
9. **Phase 2C close-out / CE-73 reclassification** — single commit
   (`9b15378`) splitting CE-73 into a Tier-2 semantic gate (enforced via
   `ci_check_hfc_translation.sh`) and an explicit Tier-4 bytes non-goal.
10. **IDD canonicalization** — `chore(idd)` commits that make the repo
    legible to the global IDD slash commands: `.idd-config.json`,
    registry rename (`constitution_registry.toml` →
    `docs/ade-invariant-registry.toml`), cluster N-D moved into
    `docs/clusters/PHASE4-N-D/`, repo-local commit-msg trailer hook.
11. **Grounding-doc generation + ripple** — `a87c3a3` produced the
    first cuts of CODEMAP/SEAMS/HEAD_DELTAS/TRACEABILITY; `f0b0fd6`,
    `a2c7ac8`, `744ef34`, and the B2-close refresh in `c1cba82`
    refreshed subsets after the BLUE-scope, N-D, N-A, and B2 closures
    respectively. **No grounding-doc refresh commit landed for N-B, the
    follow-bridge, or B1**, and the B3 refresh is the in-flight working
    tree (see Anomalies).
12. **BLUE-list drift closure** — `5b70bee` extended six CI scripts to
    the full 6-crate BLUE scope; `c8fa37f` refreshed CODEMAP and
    TRACEABILITY to remove 14 `_(scope gap)_` markers across 13 rules.
13. **Corpus relayout** — `corpus/snapshots/*` and the
    `reward_provenance/*_registered_creds.txt` files were removed (they
    carried credential material that does not belong in a public repo);
    12 boundary-block sets were re-extracted at exact era-boundary
    slots; the consensus corpus (`corpus/consensus/*`, N-B), the validity
    corpus (`corpus/validity/*`, B1), the tx-validity adversarial
    README (`corpus/tx_validity/adversarial/`, B2), and the B3
    conservation corpora (`corpus/conway_certs/*`,
    `corpus/validity/conway_epoch576/{resolution_set.txt,resolved_inputs.json}`,
    `corpus/tools/extract_conway_resolved_inputs.md`) were added.

---

## 1. Commit Log

| Hash | Type | Summary |
|------|------|---------|
| `7784bf8` | test | test(tx-validity): PHASE4-B3 conservation corpora — real epoch-576 positive + adversarial no-false-accept |
| `978c222` | feat | feat(tx-validity): PHASE4-B3 full Conway value-conservation accounting — remove the cert/withdrawal early-out |
| `3aebbe5` | docs | docs(phase4-b3): invariants, cluster/slice plan, and registry rules for Conway value-conservation accounting |
| `c1cba82` | chore | chore(phase-4): close PHASE4-B2 — tx-validity agreement + mempool admission, grounding-doc refresh |
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
| `ade_codec::conway::cert` (new file in an existing BLUE crate) | BLUE | **Conway-complete certificate decoder** with a *closed* wire grammar. `decode_conway_certs` decodes the full Conway certificate array over tags `0..18`; tags `5`/`6` (the legacy MIR/genesis-delegation certs removed in Conway) are not valid, and any unrecognized tag is a deterministic `CodecError::UnknownCertTag { tag, offset }` reject — never a silent skip. Decode is replay-deterministic over input bytes. | `conway/cert.rs` (`decode_conway_certs`) | PHASE4-B3 / B3-S1, B3-S2 |
| `ade_codec::conway::withdrawals` (new file in an existing BLUE crate) | BLUE | Conway withdrawals-map decoder. Decodes the `{ RewardAccount => Coin }` map into a canonical ordered form, summing to an `i128` consumed-side term for the value-conservation equation, and rejecting a repeated key with `CodecError::DuplicateMapKey { offset }` (a duplicate-key map is malformed wire, not last-write-wins). | `conway/withdrawals.rs` | PHASE4-B3 / B3-S3 |
| `ade_ledger::cert_classify` (new file in an existing BLUE crate) | BLUE | **Closed cert-deposit classification** — the bridge between the decoded `ConwayCert` grammar and the value-conservation equation. `classify(state, cert)` is a total, era-versioned map resolving every cert variant to exactly one `CertDisposition` over `DepositEffect` (`new_deposit` / `refund` / `neutral` / `explicit_reject`) with the coin sourced via a closed `CoinSource` (from the cert for the Conway explicit-deposit variants, tags 7/8; from the protocol parameter for the legacy variants, tags 0/1). State-dependent cases that cannot be accounted reject with `UnsupportedStateDependentDeposit` rather than guessing. Incomplete classification is a forbidden false-accept path. | `cert_classify.rs` (`classify`) | PHASE4-B3 / B3-S2 |
| `ade_ledger::tx_validity` (new submodule of an existing BLUE crate) | BLUE | **Per-transaction verdict authority** — the per-tx counterpart to `block_validity`. Closed `TxValidityVerdict` (`Valid { tx_id, applied: LedgerState }` / `Invalid { class, error }`), closed `TxRejectClass` (Phase1Invalid / WitnessInvalid / MissingRequiredSigner / Phase2Invalid / MalformedField) with a *total* `class()` mapping, and closed `TxValidityError`. `required_signers` enumerates, over a CLOSED era-versioned `SignerSource`, every `Hash28` key hash a Conway tx must witness (grounded in `getConwayWitsVKeyNeeded` / `getVKeyWitnessConwayTxCert`); `witness::verify_required_witnesses` checks each required key has a fail-closed Ed25519 witness over the preserved body hash. `tx_phase_one` composes the witness closure with the state-backed checks; `tx_validity` is the pure `(LedgerState, tx_cbor) → verdict` transition. Canonical `TxVerdictSurface` encode/decode for the replay/comparison surface. | `mod.rs` (re-exports), `verdict.rs` (`TxValidityVerdict`, `TxRejectClass`, `TxValidityError`), `required_signers.rs` (`SignerSource`, `required_signers`, `tx_derived_required_signers`, `ResolvedInputs`/`ResolvedOutput`), `witness.rs` (`verify_required_witnesses`, `WitnessClosureError`, `WitnessField`), `phase1.rs` (`tx_phase_one`, `decode_tx`, `DecodedTx`), `transition.rs` (`tx_validity`, `TxValidityOutcome`), `encoding.rs` (`encode_tx_verdict_surface`, `decode_tx_verdict_surface`, `TxVerdictSurface`) | PHASE4-B2 / B2-S1 (witness/required-signer closure), B2-S2 (composition + taxonomy) |
| `ade_ledger::mempool` (new submodule of an existing BLUE crate) | BLUE (`admit`) / GREEN (`policy`) | Mempool admission gate, two layers strictly separated by TCB color. `admit` (BLUE, Tier-1) admits a tx iff `tx_validity` is `Valid` against the mempool's accumulating `MempoolState`; no false accept, and on Invalid the mempool is unchanged. `policy` (GREEN, Tier-5) does deterministic eviction/ordering over already-admitted tx ids; it never calls `tx_validity` and provably cannot alter an admit verdict (it reads only the admitted-id list). | `mod.rs` (re-exports), `admit.rs` (`admit`, `AdmitOutcome`, `MempoolState`), `policy.rs` (`order`, `OrderPolicy`) | PHASE4-B2 / B2-S5 |
| `ade_testkit::tx_validity` (new submodule of an existing crate) | GREEN | Test-only tx-validity harness. Extracts every on-wire Conway tx from the committed Conway-576 corpus blocks and drives BLUE `tx_validity` over each (positive corpus, 103/103 Valid); supplies synthetic valid txs at controlled UTxO (`valid_synthetic.rs`); derives adversarial txs via deterministic mutators — family A (witness mutations W1–W4 on real corpus txs at `track_utxo=false`) and family B (synthetic value/input/witness mutations S1–S4 at `track_utxo=true`) — plus a judge (`adversarial.rs`). B3 extended `valid_synthetic.rs` / `adversarial.rs` for the conservation corpora and added the resolved-input intra-corpus resolver to the harness snapshot loader. Non-authoritative. | `tx_validity/mod.rs`, `tx_validity/extract.rs`, `tx_validity/valid_synthetic.rs`, `tx_validity/adversarial.rs`; `harness/snapshot_loader.rs` (B3 resolution helper); B3 example bins `examples/{dump_b3_cert_tags,dump_b3_resolution_set,resolve_b3_intra_corpus}.rs` | PHASE4-B2 / B2-S3, B2-S4; B3 conservation-corpus extensions |
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
Both are RED-or-mixed. **PHASE4-B3 added no new crate** — its new
surfaces are submodules/files of the existing BLUE crates `ade_codec`,
`ade_ledger`, and `ade_types`.

Crate dependency shape at HEAD (new deps in this delta):
- `ade_core` gained `ade_types`, `ade_crypto`, `minicbor`, and
  `ade_codec` (B1) as deps; dev deps `ade_testkit`, `serde_json`,
  `cardano-crypto` (`vrf-draft03`).
- **`ade_ledger` gained an `ade_core` dep edge** (PHASE4-B1) so the
  block-validity composition can call the consensus header authority,
  plus `minicbor` (dep) and `ade_testkit` (dev). PHASE4-B2 and
  PHASE4-B3 added **no new ledger manifest deps** — `tx_validity`,
  `mempool`, and the B3 `cert_classify` / conservation accounting
  compose existing ledger surfaces (`conway`, `cert_classify`, `rules`,
  `shelley`, `delegation`, `pparams`, `state`, `witness`, `plutus_eval`)
  plus the existing `ade_codec` / `ade_types` / `ade_crypto` deps.
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
tx-adversarial corpus is likewise *derived*. **PHASE4-B3 added** the
closed cert-grammar reference (`corpus/conway_certs/{classification_table.md,tags.json}`),
the real epoch-576 resolved-input oracle
(`corpus/validity/conway_epoch576/{resolution_set.txt,resolved_inputs.json}`),
and the extraction tool note (`corpus/tools/extract_conway_resolved_inputs.md`).

Cross-reference: CODEMAP must be regenerated to add per-submodule/file
entries for the new B3 BLUE surfaces (`ade_codec::conway::cert`,
`ade_codec::conway::withdrawals`, `ade_ledger::cert_classify`) in
addition to the still-unrecorded N-B + B1 + B2 surfaces. SEAMS must
record the closed Conway cert grammar (tags 0..18, `UnknownCertTag`
reject), the closed deposit-classification surface
(`CertDisposition` / `DepositEffect` / `CoinSource` in
`ade_types::conway::cert`), and the canonical deposit-param authority
(`ConwayOnlyDepositParams` / `ConwayDepositParams` /
`LedgerState::conway_deposit_view`). TRACEABILITY must add rows for
`DC-TXV-06` and `DC-TXV-07` (and still owes the 5 `DC-TXV-*`, 2
`DC-MEM-*`, 8 `DC-CONS-*`, and 6 `DC-VAL-*` rows). **The B3
grounding-doc refresh is in the working tree, uncommitted.**

---

## 3. Modules Modified

| Module | Scope | Key changes |
|--------|-------|-------------|
| `ade_ledger` | +48 source/test files, +7,701 / −17 lines over the full delta (of which **PHASE4-B3: +12 files, +1,558 / −12 lines**; PHASE4-B2: +24 files, +3,817 / −5; PHASE4-B1: +13 files, +1,755; CE-73: +73) | **PHASE4-B3 (primary thread of this regen):** the crate gained the closed cert-deposit classifier `cert_classify.rs` (`classify`, `CertState` bridge) and substantially reworked the value-conservation accounting. **`conway.rs`** (+130 / −): `check_conway_coin_conservation` rewritten to the **full equation** `Σ(inputs) + Σ(withdrawals) + refunded_deposits == Σ(outputs) + fee + donation + new_deposits` (i128) — **the cert/withdrawal early-out is REMOVED**; certs and withdrawals are now decoded and accounted, never skipped. **`error.rs`** (+86): new `LedgerError::EraInvalidCertificate(EraInvalidCertificateError)` and `LedgerError::UnsupportedStateDependentDeposit(UnsupportedStateDependentDepositAccounting)`, plus `ValidationEnvironmentError::MissingConwayDepositParams`. **`pparams.rs`** (+78): new `ConwayOnlyDepositParams` (Conway-only, structurally `None` for pre-Conway) and the resolved `ConwayDepositParams` view. **`state.rs`** (+28): `LedgerState.conway_deposit_params: Option<ConwayOnlyDepositParams>` field + `conway_deposit_view()` accessor. **`fingerprint.rs`** (+116): Conway deposit-param fold added to the state fingerprint; the pre-Conway fold is **byte-identical** (the new field folds to nothing when `None`). Small wiring touches in `rules.rs`, `phase.rs`, `tx_validity/phase1.rs`, `byron.rs`, `epoch.rs`, `hfc.rs`, `shelley.rs`, `lib.rs` + 3 conservation test suites (`conway_conservation_full.rs`, `conway_conservation_adversarial.rs`, `conway_conservation_positive_synthetic.rs`). **PHASE4-B2:** the `tx_validity/` (7 files) and `mempool/` (3 files) submodules + B2 integration tests; the B2-S4 `check_conway_coin_conservation` first cut (deposit-free form). **PHASE4-B1:** the `block_validity/` submodule, `consensus_view.rs`, `consensus_input_extract.rs`, the `ade_core` dep edge. **CE-73:** 10 unit tests for `decode_invalid_tx_indices`. |
| `ade_codec` | +8 source/test files, +949 lines (all PHASE4-B3) | **PHASE4-B3:** the new BLUE `conway::cert` decoder (`cert.rs`, closed grammar tags 0..18) and `conway::withdrawals` decoder (`withdrawals.rs`, `RewardAccount` map, i128 sum), wired through `conway/mod.rs`; `error.rs` (+13) added `CodecError::UnknownCertTag { tag, offset }` and `CodecError::DuplicateMapKey { offset }`. Two new test suites: `conway_cert_classification.rs` (decode total over tags 0..18, unknown-tag reject, removed-tag-5/6 reject, malformed-CBOR reject, replay-determinism), `conway_withdrawals.rs`. **`ade_codec` was untouched at the prior regen** — this is its first delta since baseline. |
| `ade_types` | +3 files, +109 / −5 lines (all PHASE4-B3) | **PHASE4-B3:** `conway/cert.rs` (+84) gained the closed `ConwayCert` enum plus the classification value types `CertDisposition`, `DepositEffect`, `CoinSource` consumed by `ade_ledger::cert_classify`; `tx.rs` (+6) added `RewardAccount(pub [u8; 29])` for the withdrawals decoder; `lib.rs` re-export wiring. First delta since baseline. |
| `ade_core` | +29 source files + tests (N-B, +8,076 lines); +828 / −86 across 16 files (B1) | **PHASE4-N-B:** stub `lib.rs` → substantive BLUE consensus module under `src/consensus/`. **PHASE4-B1:** added `consensus/kes_check.rs` (fail-closed `expect_size` + KES header guard); wired single-VRF + KES header validation across `header_validate.rs`, `vrf_cert.rs`, `leader_schedule.rs`, `fork_choice.rs`, etc. (14/14 real Conway headers validate). New dep `ade_codec` (B1). **No B2 or B3 source change** — tx-validity and value-conservation compose only `ade_ledger` surfaces. |
| `ade_crypto` | 1 file, +24 / −81 lines (B1) | Single change in `kes.rs` (`500589b`): **`build_opcert_signable` fixed** as part of B1-S5 KES header validation. No source change in N-A, N-B, N-D, B2, or B3. |
| `ade_core_interop` | +1,546 across 6 files (B1) | **CE-N-B-6 follow-bridge (`e5f1f64`) + pin retarget (`807bcb6`):** RED `follow.rs` (BLUE fork-choice + rollback only) + `follow_offline_replay.rs`; reworked `lib.rs`, the live-session binary and test. New deps `ade_codec`, `ade_crypto` for offline replay. |
| `ade_network` (existing crate, refined) | 100 files, +17,861 lines (whole crate is new this delta — see §2; the post-N-A delta is the DoS hardening) | **DoS hardening of 6 codecs** (`744ef34`, post-N-A close): capped untrusted `Vec::with_capacity` hints. No transition-authority change since N-A closure; no change in N-B/B1/B2/B3. |
| `ade_runtime` | +18 files, +3,440 lines (N-B `consensus/` + N-D `chaindb`/`recovery`; B1 one small touch) | **PHASE4-N-B:** new `consensus/` submodule (`candidate_fragment.rs`, `chain_selector.rs`, `genesis_parser.rs`) + corpus test. **PHASE4-B1:** one small touch. The N-D `chaindb`/`recovery` submodules + kill-target binary are §2 New Modules. No B2 or B3 change. |
| `ade_testkit` | +28 files, +3,251 lines: `consensus/` (N-B); `validity/` (B1); `tx_validity/` (B2); **B3 conservation extensions** | **PHASE4-N-B:** `consensus/` harness. **PHASE4-B1:** `validity/` harness (M1–M6 mutators). **PHASE4-B2:** `tx_validity/` submodule (extractor, synthetic builders, W1–W4 / S1–S4 mutators + judge). **PHASE4-B3:** extended `harness/snapshot_loader.rs` (+20, intra-corpus input resolution), `tx_validity/{adversarial,valid_synthetic}.rs` for conservation cases, added the real epoch-576 positive-corpus suite `tests/conway_conservation_positive_corpus.rs` (10 non-Plutus cert/withdrawal txs Valid at `track_utxo=true`; Plutus carved out per CE-88), and three RED example bins that materialize the B3 corpora (`examples/{dump_b3_cert_tags,dump_b3_resolution_set,resolve_b3_intra_corpus}.rs`). New deps `ade_core`, `ade_runtime` (B1). |

No other crate had non-trivial source changes since baseline.
`ade_plutus` and `ade_node` were untouched by code commits.
`.idd-config.json` had prose edits during the delta; the `core_paths`
array already covers the new B3 surfaces (`ade_codec::conway::*`,
`ade_ledger::cert_classify`) via the already-listed `ade_codec` /
`ade_ledger` crate prefixes.

---

## 4. Feature Flags

No Cargo `[features]` tables exist at HEAD in any workspace crate, and
none existed at baseline. The project does not use Cargo feature flags
as a semantic surface — closed semantic surfaces are encoded in the
type system per the IDD core principles, and conditional compilation is
checked out of BLUE code via `ci/ci_check_no_semantic_cfg.sh` (scoped
over the full 6-crate BLUE set; the new B3 surfaces
`ade_codec::conway::cert`/`::withdrawals` and
`ade_ledger::cert_classify` are covered by their crate-level scope, as
are the B2 `tx_validity`/`mempool` and B1 `block_validity` surfaces).

No `#[cfg(feature = ...)]` gates appear at either ref. `cardano-crypto`
(`vrf-draft03`) and `minicbor` (`alloc`) feature selections in the
dependency entries are upstream-crate selections, not Ade-side flags.

**Status: unchanged — zero Ade feature flags at baseline, zero at HEAD.**

---

## 5. CI Checks

The CI surface is the shell-script set under `ci/` (no
`.github/workflows` in this repo). At baseline there were 15 scripts.
At HEAD there are **26 scripts plus one git hook**: CE-73 added one
(`ci_check_hfc_translation.sh`), N-D added three, N-A added two, N-B
added four, **PHASE4-B3 added one** (`ci_check_deposit_param_authority.sh`),
and one repo-local git hook (`ci/git-hooks/commit-msg`). **PHASE4-B1
and PHASE4-B2 each added no new CI script** (both reused/extended the
N-B closed-enums script); B3 is the first cluster since N-B to add a
fresh gate. Grouped by cluster.

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
| `ci/ci_check_no_async_in_blue.sh` | **New** (`4fde3a7`, S-A1) | Enforces `DC-CORE-01` — BLUE code is sync-only. Scans every BLUE path in `core_paths`. Covers `ade_core::consensus`, `ade_ledger::block_validity`, `ade_ledger::tx_validity`, `ade_ledger::mempool`, and the B3 `ade_codec::conway::*` / `ade_ledger::cert_classify` surfaces via crate prefixes. |
| `ci/ci_check_ce_n_a_5_proof.sh` | **New** (`56bfa7b`, S-A10) | CE-N-A-5 closure-gate evidence over the real-cardano-node corpus. |

### Phase 4 N-B consensus authority enforcement (S-B1, S-B2, S-B8) — extended by B1 and B2

Four BLUE-scope CI scripts targeting `crates/ade_core/src/consensus/`.
The closed-enums script was **extended in PHASE4-B1** to also scan
`ade_ledger::block_validity`, then **extended again in PHASE4-B2** to
also scan `ade_ledger::tx_validity` and `ade_ledger::mempool`. B3 added
**no** further scope to this script (its closed-grammar / closed-enum
surfaces — `ConwayCert`, `CertDisposition`, `DepositEffect`,
`CoinSource`, `CodecError::UnknownCertTag`, `CodecError::DuplicateMapKey`
— are guarded by exhaustive-match tests today; the dedicated grep-gate
is the named `DC-TXV-06` follow-up, see §7/Anomalies).

| Check | Status | What it checks |
|-------|--------|----------------|
| `ci/ci_check_consensus_closed_enums.sh` | **New** (N-B); **Modified** (B1, `7b95ccd`); **Modified** (B2) | Four-part scan over `ade_core/src/consensus/`, `ade_ledger/src/block_validity/`, `ade_ledger/src/tx_validity/`, and `ade_ledger/src/mempool/`: no `#[non_exhaustive]`; no open-tail `Other`/`Unknown`; no owned `String` in the error/encoding/verdict files; no `Box<dyn ...>`. Strengthens `DC-CONS-04/10`, `T-DET-01`, the `DC-VAL-*`, `DC-TXV-01/02/04/05`, and `DC-MEM-01/02` rules. |
| `ci/ci_check_no_chaindb_in_consensus_blue.sh` | **New** (N-B / S-B1) | No `ChainDb`/`chain_db` token in `consensus/`. Strengthens `DC-CORE-01`, `DC-CONS-07`. |
| `ci/ci_check_no_density_in_fork_choice.sh` | **New** (N-B / S-B8) | No `density` token in `fork_choice.rs` / `candidate.rs`. Strengthens `DC-CONS-03`. |
| `ci/ci_check_no_float_in_consensus.sh` | **New** (N-B / S-B1) | No `f32`/`f64` in `consensus/`. Strengthens `T-CORE-02`, `DC-CONS-07/08/09`. |

### Phase 4 B3 Conway value-conservation enforcement (`978c222`)

| Check | Status | What it checks |
|-------|--------|----------------|
| `ci/ci_check_deposit_param_authority.sh` | **New** (`978c222`) | **Enforces `DC-TXV-07` (canonical deposit-param authority).** Greps the BLUE crate sources and fails on any non-canonical deposit-parameter read — every `key_deposit` / `pool_deposit` / `drep_deposit` / `gov_action_deposit` term must come from `ProtocolParameters` + `LedgerState.conway_deposit_params`, never from the testkit `ConwayGovParams` RED intermediate, `ProtocolParameters::default()`, a literal constant beside a deposit field, or env/shell config. Comment-only lines are stripped so prose naming a forbidden symbol does not trip the gate. |

TRACEABILITY cross-reference: the four N-B scripts map to the 8
`DC-CONS-*` rules; the closed-enums script also enforces four
`DC-VAL-*`, four `DC-TXV-*` (01/02/04/05), and both `DC-MEM-*` rules;
**the new B3 script `ci_check_deposit_param_authority.sh` is the named
`ci_script` for `DC-TXV-07`.** `DC-TXV-06`'s registry `ci_script` is
empty at HEAD — its grep-gate is the explicit follow-up that keeps the
rule at `partial` (see §7). None of the new N-B / B1 / B2 / B3 rows
exist in the *committed* TRACEABILITY yet (last committed refresh was
the B2 close `c1cba82`, which still owed N-B + B1 + B2); the B3 refresh
is the in-flight working tree.

---

## 6. Canonical Type Registry Delta

n/a — `.idd-config.json` `canonical_type_registry` is null. Canonical-type
rules live inline in the invariant registry under family `T`.

---

## 7. Normative Rule Delta

The project's invariant registry tracks structured rules (TOML), not
prose normative-doc rules; this section reports on it.

- Rules at baseline: **147** (in `constitution_registry.toml`)
- Rules at HEAD: **170** (in `docs/ade-invariant-registry.toml`)
- Net additions: **+23** (PHASE4-N-A: 2; PHASE4-N-B: 8; PHASE4-B1: 6;
  PHASE4-B2: 5; **PHASE4-B3: 2**). The two `DC-MEM-*` rules were
  *introduced earlier* (`2047c42`, `status = "declared"`) and were
  flipped to `enforced` in B2, not counted as new.
  - PHASE4-N-A: `DC-CORE-01`, `DC-PROTO-06`.
  - PHASE4-N-B (`d9f0426`): `DC-CONS-03` → `DC-CONS-10` (8 rules).
  - PHASE4-B1 (`c0acd59`, `DC-VAL` family): `DC-VAL-01` → `DC-VAL-06`.
  - PHASE4-B2 (`b79f632`, `DC-TXV` family): `DC-TXV-01` → `DC-TXV-05`.
  - **PHASE4-B3 (`3aebbe5`, two new `DC-TXV` rules):**
    - **`DC-TXV-06`** — for each era, the certificate-deposit
      classification `map(state, cert)` is a closed, total,
      era-versioned function: every cert variant resolves to exactly one
      of `new_deposit(coin) | refund(coin) | neutral | explicit_reject`,
      coin sourced from the cert (Conway tags 7/8) or the protocol
      parameter (legacy tags 0/1). An unrecognized cert tag, malformed
      cert CBOR, or undecodable withdrawals field is a deterministic
      reject — never a silent neutral. State-dependent cases that cannot
      be accounted reject with `UnsupportedStateDependentDeposit`.
      Incomplete classification is a forbidden false-accept path feeding
      the value-conservation equation.
      Tests: `decode_total_over_tags_0_18`,
      `unknown_cert_tag_is_codec_error`,
      `removed_tag_5_6_is_not_valid_in_conway`,
      `malformed_cert_cbor_rejected`, `decode_is_replay_deterministic`,
      `class_mapping_is_total`,
      `legacy_unregistration_unresolved_is_unsupported_state_dependent`,
      `legacy_unregistration_resolves_recorded_deposit`,
      `pool_reregistration_is_neutral`,
      `pool_new_registration_charges_pool_deposit`.
      code_locus: `ade_codec::conway::cert` (closed `ConwayCert`
      grammar), `ade_codec::error` (`UnknownCertTag`),
      `ade_types::conway::cert` (`ConwayCert`, `CertDisposition`,
      `DepositEffect`, `CoinSource`), `ade_ledger::cert_classify`
      (`classify`), `ade_ledger::error`
      (`UnsupportedStateDependentDepositAccounting`),
      `corpus/conway_certs/*`.
      **Status: `partial`** — the tests and the exhaustive `match`
      enforce totality today, but the dedicated grep-gate CI
      (`ci_script` is empty) is a named follow-up. ci_script: *(none
      yet — follow-up)*.
    - **`DC-TXV-07`** — canonical Conway deposit-parameter authority:
      every deposit/refund amount in the value-conservation equation is
      sourced from canonical ledger state
      (`ProtocolParameters.{key_deposit,pool_deposit}` +
      `LedgerState.conway_deposit_params` / `conway_deposit_view`) and
      never from the testkit RED intermediate, genesis defaults, literal
      constants, or env config. **Status: `enforced`** via
      `ci/ci_check_deposit_param_authority.sh`. cross_ref:
      `T-DET-01`, `T-CONSERV-01`, `CN-LEDGER-07`, `DC-TXV-06`.
- Removals: **0** (expected under append-only discipline; clean).
- Strengthenings (`declared` → `enforced`, or tightened):
  - **`T-CONSERV-01` / `CN-LEDGER-07`** (B3, `978c222`): the
    preservation-of-value invariant is strengthened from the
    deposit-free form to the **full** Conway equation
    (`Σ(inputs)+Σ(withdrawals)+refunded_deposits ==
    Σ(outputs)+fee+donation+new_deposits`); the cert/withdrawal
    early-out is removed. `strengthened_in = ["PHASE4-B3"]`,
    `cross_ref` extended to `DC-TXV-06`/`DC-TXV-07`.
  - **`DC-VAL-06`** (B3): fail-closed deposit/refund accounting added to
    the body-validity path; `cross_ref` extended to `DC-TXV-06`/`07`.
  - **`DC-TXV-03`** (B3): the no-false-accept verdict-agreement rule's
    `tests` extended with the conservation corpora — real epoch-576
    positive (10 cert/withdrawal txs Valid), synthetic positive, and
    adversarial value-imbalance cases; `cross_ref` extended to
    `DC-TXV-06`.
  - **`DC-MEM-01`, `DC-MEM-02`** (B2, `85a50dc`): `declared` →
    `enforced`. **`DC-TXV-03`, `DC-VAL-06`, `DC-LEDGER-02`** previously
    strengthened by B2-S4 (`617139f`).
  - Earlier-delta strengthenings (unchanged): `DC-EPOCH-02`
    (`9b15378`); the N-D bundle (`78da6c9`); the N-A real-capture
    bundle; `T-CORE-02` (S-B1).
  - The six `DC-VAL-*` and five `DC-TXV-01..05` rules each record
    `strengthened_in = ["PHASE4-B1"]` / `["PHASE4-B2"]` respectively
    even though each family was *created* in its own cluster — recorded
    faithfully; see Anomalies.

Family counts at HEAD: CN dominates (~64), DC grew most across the delta
(now including `DC-CONS` ×8, `DC-VAL` ×6, `DC-TXV` ×7, `DC-MEM` ×2),
T = 30, RO/OP combined ×9.

Normative-doc rule extraction (the `normative_docs` list in
`.idd-config.json`) is approximate and not regenerated here — the
structured registry is the authoritative source.

---

## Anomalies and Cross-Reference Warnings

- **PHASE4-B3 committed (3 commits) but cluster-close housekeeping is in
  flight.** No `Close PHASE4-B3` commit exists. The working tree holds
  uncommitted edits to `docs/ade-invariant-registry.toml`, the four
  grounding docs, and four `docs/clusters/PHASE4-B3/*` slice docs. This
  regen reflects the in-flight close intent. Surface for the close
  commit.
- **`DC-TXV-06` status disagrees between committed HEAD and the
  in-flight close edit.** The *committed* registry at `7784bf8` marks
  `DC-TXV-06` `enforced` with `cluster = "PHASE4-B3"`, `strengthened_in
  = []`; the *working-tree* close edit reclassifies it to **`partial`**
  with `cluster = "CL-LEDGER-VERDICT"`, `strengthened_in =
  ["PHASE4-B3"]`, and adds `DC-TXV-07` to its `cross_ref`. This regen
  reports `partial` (the dedicated grep-gate CI is a real, named
  follow-up; the `ci_script` field is empty). Resolve by landing the
  close commit. Note also the `cluster` field flip
  (`PHASE4-B3` → `CL-LEDGER-VERDICT`) — confirm which cluster id is
  canonical for this rule before committing.
- **Conway value-conservation gap CLOSED for cert/withdrawal txs (B3,
  `978c222`).** The deposit/refund/withdrawal accounting that B2-S4
  deliberately deferred (the early-out in `check_conway_coin_conservation`)
  is now removed: the full equation is enforced, sourcing every term
  from canonical ledger state via `cert_classify::classify` over the
  closed `ConwayCert` grammar. The named tx-validity-completeness
  follow-up from the prior regen is **discharged for Conway**. Confirm
  `ci_check_differential_divergence.sh` / `ci_check_ledger_determinism.sh`
  still cover the full equation on the next TRACEABILITY pass; the
  `fingerprint.rs` Conway deposit-param fold is byte-identical for
  pre-Conway state (verify in the determinism replay).
- **`DC-TXV-06` is `partial`, not `enforced` — a known coverage edge.**
  The closed cert-deposit classification is enforced by exhaustive-match
  + named tests today, but has **no dedicated CI grep-gate** (unlike
  `DC-TXV-07`). The grep-gate is the explicit follow-up that flips
  `DC-TXV-06` to `enforced`. Until then, a future refactor reintroducing
  an open-tail or silent-neutral classification arm would be caught by
  tests but not by a standing CI invariant. Surface for the next
  ledger-verdict cluster.
- **Grounding docs stale on N-B + follow-bridge + B1 + B2 + B3
  (committed).** CODEMAP, SEAMS, and TRACEABILITY were last *committed*
  refreshed at the B2 close `c1cba82` (which itself still owed N-B + B1
  + B2 rows). At committed HEAD they carry **no** entries for the B3
  surfaces (`ade_codec::conway::cert`, `ade_codec::conway::withdrawals`,
  `ade_ledger::cert_classify`, the `ConwayCert`/`CertDisposition`/
  `DepositEffect`/`CoinSource` types, `ConwayOnlyDepositParams`/
  `ConwayDepositParams`, `LedgerState::conway_deposit_view`) nor the
  B3 rules `DC-TXV-06`/`07`. The B3 refresh is the **in-flight working
  tree** (this file + the other three docs are `M` in `git status`).
  The CI-script count in CODEMAP must read 26 scripts + 1 hook. Run
  `/codemap`, `/seams`, `/traceability` and commit alongside the close.
- **DC-VAL status mismatch vs. closure claim (B1, carried forward).**
  PHASE4-B1 is reported fully closed, but in the registry only
  `DC-VAL-01` is `enforced` — `DC-VAL-02` → `DC-VAL-06` remain
  `declared` despite named tests and the extended closed-enums
  enforcement point. Flip on the next `/traceability` pass.
- **`strengthened_in` records the introducing cluster on freshly-created
  rules.** Each `DC-VAL-*` records `["PHASE4-B1"]` and each
  `DC-TXV-01..05` records `["PHASE4-B2"]` even though those clusters
  *created* the families; the in-flight close also sets
  `DC-TXV-06.strengthened_in = ["PHASE4-B3"]`. Harmless (no weakening),
  but consider normalizing on the next registry curation pass.
- **`ade_ledger -> ade_core` dependency edge (B1, carried forward).**
  First ledger→consensus edge. Both BLUE, so the BLUE→RED guard is
  unaffected; B3 added no new manifest edge.
- **`ade_crypto::kes::build_opcert_signable` fixed in B1-S5
  (`500589b`).** BLUE crypto-surface behavioral change; confirm
  `ci_check_crypto_vectors.sh` still covers it on the next
  TRACEABILITY pass.
- **B3 positive corpus carves out Plutus per CE-88.** The real
  epoch-576 positive conservation corpus
  (`conway_conservation_positive_corpus.rs`) drives **10 non-Plutus**
  cert/withdrawal txs to `Valid` at `track_utxo=true`; Plutus-witnessing
  txs are excluded because CE-88 (Conway `validity_range` aiken bug) is
  externally blocked. Intentional carve-out, not a coverage gap in the
  conservation equation itself.
- **Adversarial corpora are derived, not committed.** `corpus/validity/`
  (B1), `corpus/tx_validity/` (B2), and the B3 adversarial conservation
  cases are generated deterministically at test time by the mutators in
  `ade_testkit`. The B3 *positive* oracle, by contrast, is committed
  (`corpus/validity/conway_epoch576/resolved_inputs.json`,
  `resolution_set.txt`) because it is a real on-chain input-resolution
  set, not a derivable mutation.
- **PHASE4-N-A / N-B / N-D / B1 / B2 closed.** N-A `69a2862`
  (+`744ef34`); N-B `a0c73e1`; N-D `436b1d7`; B1 `993f363`; B2
  `c1cba82`. B3 committed, close in flight.
- **`ade_core_interop` tests `#[ignore]`-gated / offline-replay by
  design.** Live tip-agreement not run in CI; CE-N-B-6 closure evidence
  is a manual operator pass.
- **Corpus relayout: credentialed snapshots removed.** Deleted
  `corpus/snapshots/reward_provenance/*_registered_creds.txt` dominates
  the ~7M-line negative line count; replaced by 12 re-extracted
  boundary-block sets.
- No removed canonical types (n/a — no separate registry).
- No removed registry rules (expected: 0; actual: 0).
- **All commit subjects carry a conventional-commits prefix or are
  cluster-close housekeeping.** The three `Close PHASE4-*` commits
  (`69a2862`, `436b1d7`, `a0c73e1`, `993f363`) and the bare `chore:`
  commits (`3552bc2`, `e0af99d`) are classified `chore` on scope
  grounds. The B3 trio (`3aebbe5` docs, `978c222` feat, `7784bf8` test)
  are conventional. No unclassifiable subjects.

---

## Generation Notes

Regenerate via `/head-deltas <baseline>` or by re-running the
`head-deltas-generator` agent with the same baseline. Baseline lives in
`.idd-config.json` `head_deltas_baseline` (still `d509f02` — **this is a
cluster close, not a phase boundary, so the baseline is unchanged**).
Update the baseline on the next phase boundary (Phase 4 close). Note the
commit-hash rewrite caveat at the top — re-derive hashes from
`git log` at each regen rather than carrying them forward. This regen
was cut at committed HEAD `7784bf8` with the PHASE4-B3 cluster-close
edits still in the working tree.
