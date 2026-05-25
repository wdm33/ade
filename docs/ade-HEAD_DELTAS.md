# HEAD Deltas — Ade

> **Status:** Living architectural document. Regenerated; not hand-edited.
> Regenerate via `/head-deltas <baseline>`. Baseline is declared in
> `.idd-config.json` (`head_deltas_baseline`).

> Baseline: `d509f02` (Phase 3 handoff snapshot, 2026-04-15)
> HEAD: `694dd74` (feat(producer): mechanical cross-impl adapter + live_block_production_session binary (PHASE4-N-C S7), 2026-05-25)
> 155 commits, 11,365 files changed, +191,010 / −7,233,633 lines

Headline numbers note: the massive negative line count is dominated by
the **corpus relayout** under `corpus/snapshots/` and the deletion of
the multi-MB credentialed-snapshot text files
(`*_registered_creds.txt`, ~7M lines combined). Source-tree deltas are
far smaller — the per-crate breakdown in §3 is the representative view.

> **Commit-hash note.** This regen runs against the current (rebased)
> history. Earlier HEAD_DELTAS regens referenced commit hashes from a
> history that has since been rewritten; all hashes below are verbatim
> from `git log d509f02..HEAD` at this HEAD.

> **PHASE4-N-C cluster close note (newest thread).** This regen is cut
> at working HEAD `694dd74`. Since the prior grounding-doc refresh
> commit `96d043c` (which committed the PROPOSAL-PROCEDURES-DECODE
> cluster close + archive + post-PPD grounding ripple at HEAD
> `928c2be`), **nine new commits have landed** — the **PHASE4-N-C
> cluster** (S1 → S7 + a small registry follow-up + a Cargo.lock
> follow-up) closing the last Tier-1 bounty deliverable
> (block-production authority — the validation→producer leap).
> Sequence: `ea9770e` (S1, **RED** signing primitives + cardano-cli
> skey loader — `ade_runtime::producer::{signing, keys}` greenfield
> ~976 LOC + `ade_crypto::kes` additions (`KesSignature`,
> `verify_kes_signature`) + new BLUE rules `DC-CRYPTO-03/04/05` +
> `OP-OPS-04`, all flipped to `enforced` + new CI gate
> `ci_check_private_key_custody.sh` + new closed registry rules
> `DC-CONS-11/12/13/14/15/16`, `DC-LEDGER-12`, `CN-CONS-06/07`,
> `OP-OPS-05` introduced at `declared`); `9727bd9` (S1 follow-up,
> `OP-OPS-04.open_obligation` records the two real-skey loader
> obligations); `4cf4b65` (S2, **BLUE** `opcert_validate` +
> closed-grammar opcert encoder — new `ade_core::consensus::opcert_validate`
> (~234 LOC) + new `ade_codec::shelley::opcert` (~375 LOC) + new CI
> gate `ci_check_opcert_closed.sh` + `DC-CONS-11/12` flipped to
> `enforced`); `8312690` (S3, **BLUE** `forge_block` + `ProducerTick`
> — new `ade_ledger::producer::{forge, state}` + tx-admissibility
> prefix gate over `mempool::admit` + new CI gates
> `ci_check_forge_purity.sh` + `ci_check_no_private_keys_in_corpus.sh`
> + first cut of `ade_testkit::producer::{fixtures, replay}` + 3 in-code
> synthetic ProducerTick fixtures + `DC-CONS-13/14/15`, `DC-LEDGER-12`
> flipped to `enforced`); `4fd714c` (S4, **BLUE refactor** — unifies the
> body-hash recipe into a single canonical authority: new
> `ade_ledger::block_body_hash` (~147 LOC: `block_body_hash`,
> `block_body_hash_from_buckets`) consumed by both `forge_block` and
> `block_validity::header_input`; new CI gate
> `ci_check_no_producer_body_encoder.sh`; `DC-CONS-16` flipped to
> `enforced`. `aa7a7dd` (S5, **BLUE** `self_accept` bridge +
> `AcceptedBlock` type-level broadcast gate — new
> `ade_ledger::producer::self_accept` (~364 LOC) wrapping N-B's
> `validate_and_apply_header` + B1's `block_validity` + new CI gate
> `ci_check_self_accept_gate.sh` + `CN-CONS-07` flipped to
> `enforced`); `58678af` (S6, **RED** `scheduler` + **GREEN**
> `tick_assembler` + **RED** `broadcast` queue — new
> `ade_runtime::producer::{scheduler, tick_assembler, broadcast}`
> (~478 + ~211 + ~265 LOC) + slot-deadline integration test +
> new CI gate `ci_check_scheduler_closure.sh` + new Cargo dep edge
> `ade_runtime -> ade_ledger` + `OP-OPS-05` flipped to `enforced`);
> `52b77c5` (S6 Cargo.lock follow-up); `694dd74` (S7, **mechanical
> cross-impl adapter** — new `ade_testkit::producer::cross_impl_adapter`
> (~184 LOC) asserting decode round-trip + S4 body-hash binding +
> structural decoder/encoder field agreement over the S3 fixture
> corpus + new **RED** binary `ade_core_interop::live_block_production_session`
> (~247 LOC) + new CI gate `ci_check_producer_corpus_present.sh` +
> new operator-action procedure doc + `CN-CONS-06` flipped to
> `enforced` for its mechanical half, with CE-N-C-8's live half
> tracked as `blocked_until_operator_stake_available` on the rule's
> `open_obligation`). **Two new BLUE submodules** (`ade_ledger::producer`
> with `forge`/`state`/`self_accept` files and the
> `ade_core::consensus::opcert_validate` + `ade_codec::shelley::opcert`
> pair), **one new GREEN file** (`ade_runtime::producer::tick_assembler`),
> **four new RED files** (`ade_runtime::producer::{signing, keys,
> scheduler, broadcast}`), **one new RED binary**
> (`live_block_production_session`), **one new BLUE module under an
> existing crate** (`ade_ledger::block_body_hash` — the single
> canonical body-hash authority), **one new test-harness submodule**
> (`ade_testkit::producer::{fixtures, replay, reference_vectors,
> cross_impl_adapter}`), **14 new registry rules** (all
> `introduced_in = "PHASE4-N-C"` and all flipped to `enforced` over
> the cluster: `DC-CRYPTO-03/04/05`, `DC-CONS-11/12/13/14/15/16`,
> `DC-LEDGER-12`, `CN-CONS-06/07`, `OP-OPS-04/05`), **8 new CI
> scripts** (the 33rd → 40th: `ci_check_private_key_custody.sh`,
> `ci_check_opcert_closed.sh`, `ci_check_forge_purity.sh`,
> `ci_check_no_private_keys_in_corpus.sh`,
> `ci_check_no_producer_body_encoder.sh`,
> `ci_check_self_accept_gate.sh`, `ci_check_scheduler_closure.sh`,
> `ci_check_producer_corpus_present.sh`), one Cargo dep edge added
> (`ade_runtime -> ade_ledger`, S6). **Two `open_obligation` entries
> recorded**: `OP-OPS-04` (real cardano-cli Sum6KES skey loader — the
> upstream-fork-or-document call) and `CN-CONS-06` (CE-N-C-8 live
> evidence — blocked_until_operator_stake_available). **Cluster
> status at HEAD: closed mechanically; CE-N-C-8 conditional half open
> as a registry obligation.** Cluster directory remains in place at
> `docs/clusters/PHASE4-N-C/` (8 files: `cluster.md` + S1 → S7 +
> `CE-N-C-8_PROCEDURE.md`); archival to
> `docs/clusters/completed/PHASE4-N-C/` is deferred to a separate
> `/cluster-close` commit. **No CODEMAP/SEAMS/TRACEABILITY refresh
> yet** for the N-C cluster — those three docs are stale relative to
> this HEAD_DELTAS regen and must be regenerated in the grounding
> ripple immediately following.

> **PROPOSAL-PROCEDURES-DECODE cluster close note (prior thread,
> carried forward).** Closed at HEAD `928c2be` and archived to
> `docs/clusters/completed/PROPOSAL-PROCEDURES-DECODE/` by `96d043c`.
> Two slices PP-S1 + PP-S2 introduced the new BLUE module
> `ade_codec::conway::governance` (~580 LOC) + new closed type
> `ProposalProcedure` + body-codec rewire at key 20 + new CI gate
> `ci_check_proposal_procedures_closed.sh` + new rule
> `DC-LEDGER-11` + a GREEN canonical synthetic-corpus replay
> harness `ade_testkit::governance::proposal_procedures_replay`.
> Type change at the boundary: `ConwayTxBody.proposal_procedures`
> went from `Option<Vec<u8>>` (opaque pass-through at key 20) to
> `Option<Vec<ProposalProcedure>>` (typed through the closed
> decoder at the canonical entry slice). **Scope locks held by
> parent declaration**: `voting_procedures` stays opaque (OQ-1);
> `ParameterChange.update` stays opaque (OQ-2); `NewConstitution.raw`
> stays opaque (OQ-3); `proposal_procedure.return_addr` stays
> `Vec<u8>` (OQ-4); `Anchor` stays opaque struct.

> **PHASE4-N-E cluster close note (prior thread, carried forward).**
> Closed at HEAD `caa5ce8` and archived to
> `docs/clusters/completed/PHASE4-N-E/` (10 files) by `3af9e2b`.
> Tier-1 authority closed; CE-N-E-6 live N2N evidence captured at
> `CE-N-E-6_2026-05-25.log`. CE-N-E-7 deferred as cross-cluster
> obligation `CE-NODE-N2C-LTX`. Six implementation commits shipped
> the BLUE `mempool::ingress` chokepoint, the GREEN
> `mempool::canonicalize` ordering function, the GREEN
> `testkit::mempool::ingress_replay` harness, the two GREEN bridges
> (`ade_core_interop::tx_submission`, `ade_core_interop::local_tx_submission`),
> and the RED `live_tx_submission_session` probe binary. Two
> registry rules (`DC-MEM-03`, `DC-MEM-04`); two CI scripts; one
> new Cargo dep edge (`ade_core_interop -> ade_ledger`).

> **Testkit follow-up note (prior thread, carried forward).** Four
> GREEN-scope commits between WRITEBACK refresh `3d94c22` and refresh
> `52642e5` — bounded to `ade_testkit` / corpus tooling, no BLUE
> source change, no new rule, no new CI script. `DC-EPOCH-01` and
> `DC-LEDGER-10` each gained one oracle test.

> **ENACTMENT-COMMITTEE-WRITEBACK cluster note (prior thread, carried
> forward).** Three implementation commits + close-hardening +
> grounding refresh; live committee write-back without a new
> module/rule/CI script.

> **ENACTMENT-COMMITTEE-FIDELITY / DREP-VOTE-FIDELITY /
> COMMITTEE-CRED-FIDELITY / OQ5-CREDENTIAL-FIDELITY cluster notes
> (prior threads, carried forward).** All structural changes,
> fingerprint surfaces, and credential-discriminant ripples unchanged
> at this HEAD.

> **B5 / B4 / B3F / B3 / B2 / B1 / N-D / N-B / N-A / PHASE4-N-E /
> PROPOSAL-PROCEDURES-DECODE cluster notes (carried forward).** All
> closed and archived at `docs/clusters/completed/<NAME>/`.

The delta now covers twenty-six threads of work. The newest thread —
the **PHASE4-N-C cluster** (`ea9770e` → `694dd74`, 9 commits) — sits
on the post-PPD grounding refresh `96d043c`, which closed +
archived PROPOSAL-PROCEDURES-DECODE. In rough proportion of the
substantive change budget:

0. **PHASE4-N-C (last Tier-1 bounty deliverable — block-production
   authority, the validation→producer leap) — closed in 7 slices +
   2 follow-ups.** S1 (`ea9770e`, **RED**) introduces the producer
   crypto-substrate: `ade_runtime::producer::signing` (~600 LOC:
   `VrfSigningKey`, `KesSecret`, `ColdSigningKey`, `vrf_prove`,
   `kes_sign`, `kes_update`, `SigningError`; zeroize-on-drop) +
   `ade_runtime::producer::keys` (~376 LOC: cardano-cli `*.skey`
   text-envelope loader with `load_{vrf,kes,cold}_signing_key_skey`
   + `VRF/KES/POOL_SIGNING_KEY_TYPE` constants + `KeyLoadError`),
   plus additions to `ade_crypto::kes` (`KesSignature` +
   `verify_kes_signature` re-exposing the existing depth-6 verify
   path under the canonical name S5/S6 consume), plus first cut of
   `ade_testkit::producer::reference_vectors` (~130 LOC: VRF/KES
   reference vector sets + KES update chain). Registry: 11 rules
   land in this slice — `DC-CRYPTO-03/04/05` + `OP-OPS-04` flipped
   to `enforced`; `DC-CONS-11/12/13/14/15/16`, `DC-LEDGER-12`,
   `CN-CONS-06/07`, `OP-OPS-05` introduced at `declared`. New CI
   gate `ci_check_private_key_custody.sh` (~207 LOC, 5+ mechanical
   guards forbidding `*SigningKey`/`KesSecret`/cold-key types
   outside `ade_runtime::producer/` and forbidding direct
   cardano-crypto signing calls outside the same scope; production
   code only — test modules excluded for self-consistency checks).
   `OP-OPS-04.open_obligation` records the cardano-crypto-1.0.8
   Sum6KES `raw_deserialize_signing_key_kes` gap (synthetic-seed
   round-trip works; real 612-byte expanded-tree cardano-cli skey
   loading is the upstream-fork-or-document call). S1 follow-up
   `9727bd9` formalizes this in the registry.
   S2 (`4cf4b65`, **BLUE**) introduces the opcert authority pair —
   new closed-grammar codec module `ade_codec::shelley::opcert`
   (~375 LOC: `encode_opcert`, `decode_opcert`,
   `write_opcert_fields_into`, `read_opcert_fields_from`,
   `OpCertCodecError`; cardano-cli byte-identical) and new BLUE
   validator module `ade_core::consensus::opcert_validate`
   (~234 LOC: `opcert_validate`, `OpCertError::{CounterRepeat,
   CounterRegression, PeriodMismatch, ShortHotVkey, BadColdSignature}`;
   counter monotonicity gate + period-at-slot check + ed25519
   cold-signature verify). Header path in
   `ade_codec::shelley::block` re-pointed at the new opcert codec.
   New CI gate `ci_check_opcert_closed.sh` (~220 LOC, 6+ guards:
   forbids parallel `encode_*opcert` / `write_opcert` outside the
   canonical authority pair). Registry: `DC-CONS-11/12` flipped to
   `enforced`.
   S3 (`8312690`, **BLUE**) introduces the producer core — new
   submodule `ade_ledger::producer` with `state.rs` (~74 LOC:
   `ProducerTick` canonical type carrying `slot`, `epoch_nonce`,
   `vrf_proof`, `kes_signature`, `opcert`, `mempool_snapshot`,
   `pparams`, `era_anchor`) and `forge.rs` (~534 LOC at HEAD after
   S4 refactor: `forge_block`, `ForgedBlock`, `ForgeError`,
   `ForgeEffects` — leader-check gate via the validator's
   `is_leader_for_vrf_output` (no producer-side fork per NC-VRF-3),
   tx-admissibility prefix gate via `mempool::admit::admit` over
   the snapshot's accumulating order, empty-mempool→empty-body per
   OQ-7). The Conway tx-body shape `ade_codec::shelley::tx_components`
   gains 152 lines for the producer assembly path. New BLUE
   additions to `ade_ledger::mempool::admit` (+32) expose the admit
   prefix helper. New CI gates `ci_check_forge_purity.sh` (forbids
   `SystemTime`, `rand`, `HashMap` iteration, `std::fs`, `std::env`,
   `println!`, `async` in `forge.rs`/`state.rs`) and
   `ci_check_no_private_keys_in_corpus.sh` (forbids `*.skey` /
   `*.sk` / `*.signingkey` files under producer fixtures and forbids
   private-key type names in fixture sources). First cut of the
   replay harness lands here: `ade_testkit::producer::fixtures`
   (~274 LOC: `fixture_empty_mempool_leader`,
   `fixture_non_leader`, `fixture_two_tx_mempool_leader`,
   `all_fixtures`) + `ade_testkit::producer::replay` (~216 LOC:
   `ProducerReplayFixture`, `producer_replay_fixtures`) + regen
   binary `crates/ade_testkit/tests/regen_producer_fixtures.rs`.
   Fixtures are in-code synthetic (canonical by construction; no
   on-disk corpus files — the corpus tree exists in code so replay
   never touches RED signing). Registry: `DC-CONS-13/14/15`,
   `DC-LEDGER-12` flipped to `enforced`.
   S4 (`4fd74fc` — see §1 row 5 for verbatim hash, **BLUE refactor**)
   unifies body-hash recipe into a single canonical authority. New
   module `ade_ledger::block_body_hash` (~147 LOC: `block_body_hash`,
   `block_body_hash_from_buckets` — the dual entry points S4 lifts
   to single-authority status). Both `forge_block` (producer) and
   `block_validity::header_input::computed_body_hash` (validator)
   now go through this module — no producer/validator encoder
   bifurcation. New CI gate `ci_check_no_producer_body_encoder.sh`
   (~132 LOC; exactly two `pub fn block_body_hash{,_from_buckets}`
   definitions allowed in BLUE; both must be in `block_body_hash.rs`).
   Registry: `DC-CONS-16` flipped to `enforced`. **No new dependency
   edge.**
   S5 (`aa7a7dd`, **BLUE**) introduces the self-acceptance bridge —
   new module `ade_ledger::producer::self_accept` (~364 LOC:
   `self_accept`, `AcceptedBlock`, `SelfAcceptError` — closed
   8-variant error taxonomy covering header reject / body reject /
   body-hash drift / KES drift / VRF drift / opcert reject /
   ledger-state drift / context mismatch). `AcceptedBlock` is the
   load-bearing **type-level broadcast gate**: it has no public
   constructor outside `self_accept.rs`; `RED Broadcast::send`
   accepts only `AcceptedBlock` (consume-only). Wraps N-B's
   `header_validate::validate_and_apply_header` and B1's
   `block_validity::transition::block_validity` — single closed
   validator authority self_accept invokes (no parallel validator
   path). New CI gate `ci_check_self_accept_gate.sh` (~174 LOC, 3+
   guards: `AcceptedBlock {` struct-literal matches ONLY in
   `self_accept.rs`; exactly one `pub fn .* -> AcceptedBlock`;
   `broadcast` argument type is `AcceptedBlock`, not raw bytes).
   `ci_check_constitution_coverage.sh` extended (+16/-…) to
   recognize the new closed types. Registry: `CN-CONS-07` flipped
   to `enforced`.
   S6 (`58678af`, **RED + GREEN**) introduces the slot-driven
   pipeline — new submodules in `ade_runtime::producer`:
   `scheduler.rs` (~478 LOC, **RED**: `scheduler_step<L: LedgerView>`,
   `SchedulerInput`, `SchedulerEffect`, `SchedulerHaltReason`,
   `SchedulerState`; slot-wakeup loop, RED→GREEN→BLUE→BLUE→RED
   call sequence, deterministic halt on `self_accept` failure),
   `tick_assembler.rs` (~211 LOC, **GREEN**: `assemble_tick`,
   `TickInputs`, `TickAssemblyError`; composes canonical
   `ProducerTick` from captured RED outputs — must be observably
   deterministic so two replays over the same RED outputs yield
   byte-identical ticks), and `broadcast.rs` (~265 LOC, **RED**:
   `BroadcastQueue`, `BroadcastError`; outbound queue handing
   `AcceptedBlock` bytes to N2N block-fetch / chain-sync server
   path). New `crates/ade_runtime/tests/producer_pipeline_slot_deadline.rs`
   (~190 LOC) integration test measuring full-path wall-clock under
   the slot deadline. New CI gate
   `ci_check_scheduler_closure.sh` (~197 LOC, 5+ guards:
   `scheduler_step` + `assemble_tick` are I/O-pure; broadcast
   accepts only `AcceptedBlock`; scheduler halts deterministically
   on self-accept failure; no cycles in the dep graph). **New
   Cargo dep edge**: `ade_runtime -> ade_ledger` (was transitive
   only; now direct because broadcast must consume the
   `AcceptedBlock` token type from `ade_ledger::producer`).
   `52b77c5` records the resulting Cargo.lock changes. Registry:
   `OP-OPS-05` flipped to `enforced`.
   S7 (`694dd74`, **mechanical cross-impl + operator-action evidence**)
   ships the cluster's bounty-facing surface. New
   `ade_testkit::producer::cross_impl_adapter` (~184 LOC) drives
   every fixture in `producer_replay_fixtures()` through the full
   pipeline and asserts three honest structural properties: (1)
   `decode_shelley_block_inner(forged.bytes)` round-trips OK;
   (2) `block_body_hash(&decoded)` re-binds the emitted
   `header.body_hash` (S4's body-hash recipe surviving the
   decode/encode round-trip); (3) decoded `body_hash`,
   `operational_cert.sequence_number`, and
   `operational_cert.kes_period` match the in-memory `ForgedBlock`.
   New **RED** binary `ade_core_interop::live_block_production_session`
   (~247 LOC) modeled on `live_consensus_session` and
   `live_tx_submission_session`: connects to a private cardano-node
   over N2N, hands off Ade-forged bytes via block-fetch /
   chain-sync server, captures
   `docs/clusters/PHASE4-N-C/CE-N-C-LIVE_<date>.log`. New
   operator-action procedure doc `CE-N-C-8_PROCEDURE.md`. New CI
   gate `ci_check_producer_corpus_present.sh` (~177 LOC, 5+ guards:
   `producer_replay_fixtures()` is wired and exposes the three S3
   fixtures; expected_forged outputs non-empty for leader cases;
   cross-impl adapter test names present; live binary present;
   procedure doc present; `CN-CONS-06.code_locus` references the
   cross-impl adapter). Registry: `CN-CONS-06` flipped to
   `enforced` for its mechanical half; `CN-CONS-06.open_obligation`
   records CE-N-C-8's live half as
   `blocked_until_operator_stake_available` with re-open criteria,
   per the OP-OPS-04 precedent. **No new dep edge from S7** —
   `ade_core_interop` already had its `ade_ledger` edge from N-E S4.
1. **PROPOSAL-PROCEDURES-DECODE (last open governance-domain decode
   seam) — closed in 2 slices.** PP-S1 (`70bc85b`, **BLUE**) introduces
   the new closed decoder module `crates/ade_codec/src/conway/governance.rs`,
   the new closed canonical type `ProposalProcedure`, rewires
   `ConwayTxBody.proposal_procedures` from `Option<Vec<u8>>` to
   `Option<Vec<ProposalProcedure>>`, introduces `DC-LEDGER-11` +
   `ci_check_proposal_procedures_closed.sh`. PP-S2 (`928c2be`,
   **GREEN**) adds canonical synthetic-corpus replay harness in
   `ade_testkit::governance`.
2. **PHASE4-N-E S6 (live N2N tx-submission2 evidence binary) —
   cluster close.** Sustained-window RED probe binary
   `live_tx_submission_session`; retry-on-`TimedOut` hardening folded
   into the close commit.
3. **PHASE4-N-E S1–S5 (wire-level mempool ingress, Tier 1).**
   `IngressEvent` / `IngressSource::{N2N, N2C}` closed chokepoint;
   canonicalizer; N2N + N2C bridges; ingress-replay harness.
   `DC-MEM-03` + `DC-MEM-04`; `DC-MEM-01` strengthened.
4. **Post-WRITEBACK testkit follow-ups (four commits, GREEN-scope).**
   Real-chain committee oracle; corpus alignment; `reward_provenance`
   generator; snapshot-loader follow-ups.
5. **ENACTMENT-COMMITTEE-WRITEBACK — closed.** Live committee
   write-back; `DC-EPOCH-01` + `DC-LEDGER-10` strengthened.
6. **ENACTMENT-COMMITTEE-FIDELITY — closed.**
7. **DREP-VOTE-FIDELITY — closed.**
8. **COMMITTEE-CRED-FIDELITY — closed.**
9. **OQ5-CREDENTIAL-FIDELITY — closed.** `DC-LEDGER-10` introduced +
   `enforced`.
10. **Phase 4 cluster B5 (Conway gov-cert accumulation) — closed.**
    `DC-LEDGER-09` introduced + `enforced`.
11. **Phase 4 cluster B4 (Conway cert-state accumulation,
    fail-closed) — closed.** `DC-LEDGER-08` introduced + `enforced`.
12. **Phase 4 cluster B3F (follow-up hardening) — committed.**
    `DC-TXV-06` `partial` → `enforced`.
13. **Phase 4 cluster B3 (Conway value-conservation accounting) —
    closed.**
14. **Phase 4 cluster B2 (tx validity agreement) — closed.**
15. **Phase 4 cluster B1 (full block validity agreement) — closed.**
16. **Phase 4 cluster N-A (network mini-protocols) — closed.** New
    BLUE crate `ade_network`.
17. **Phase 4 cluster N-B (consensus runtime) — closed.** New BLUE
    `ade_core::consensus` module.
18. **CE-N-B-6 follow-mode bridge.**
19. **Phase 4 cluster N-D (ChainDB persistence) — closed.**
20. **Phase 2C close-out / CE-73 reclassification.**
21. **IDD canonicalization.**
22. **Grounding-doc generation + ripple.** Successive refreshes,
    including `52642e5`, `350130e`, `3af9e2b`, `96d043c`.
23. **BLUE-list drift closure.** Six CI scripts extended to full
    BLUE scope.
24. **Corpus relayout.** Credentialed `*_registered_creds.txt`
    removed (~7M-line negative); `corpus/snapshots/` now
    `.gitignore`-d.

---

## 1. Commit Log

| Hash | Type | Summary |
|------|------|---------|
| `694dd74` | feat | feat(producer): mechanical cross-impl adapter + live_block_production_session binary (PHASE4-N-C S7) |
| `52b77c5` | chore | chore(lock): record Cargo.lock changes from N-C-S6 ade_runtime -> ade_ledger dep |
| `58678af` | feat | feat(producer): RED scheduler + GREEN tick-assembler + RED broadcast queue (PHASE4-N-C S6) |
| `aa7a7dd` | feat | feat(producer): BLUE self_accept bridge + AcceptedBlock type-level broadcast gate (PHASE4-N-C S5) |
| `4fd714c` | refactor | refactor(ledger): unify body-hash recipe into single canonical authority (PHASE4-N-C S4) |
| `8312690` | feat | feat(producer): BLUE forge core + ProducerTick + tx-admissibility prefix (PHASE4-N-C S3) |
| `4cf4b65` | feat | feat(consensus): BLUE opcert_validate + closed-grammar opcert encoder authority (PHASE4-N-C N-C-S2) |
| `9727bd9` | docs | docs(registry): record OP-OPS-04 open obligations from N-C-S1 closure |
| `ea9770e` | feat | feat(producer): RED signing primitives + cardano-cli skey loader (PHASE4-N-C S1) |
| `96d043c` | docs | docs(grounding): close PROPOSAL-PROCEDURES-DECODE — archive cluster + refresh CODEMAP/SEAMS/HEAD_DELTAS/TRACEABILITY |
| `928c2be` | test | test(testkit): proposal_procedures canonical corpus + replay harness (PROPOSAL-PROCEDURES-DECODE PP-S2) |
| `70bc85b` | feat | feat(codec): close proposal_procedures opacity at the Conway tx-body boundary (PROPOSAL-PROCEDURES-DECODE PP-S1) |
| `3af9e2b` | docs | docs(grounding): close PHASE4-N-E — archive cluster + refresh CODEMAP/SEAMS/HEAD_DELTAS/TRACEABILITY |
| `caa5ce8` | fix | fix(interop): retry-on-timeout + elapsed-time logging; CE-N-E-6 live evidence (PHASE4-N-E) |
| `d1068b3` | feat | feat(interop): live N2N tx-submission2 session binary (PHASE4-N-E S6) |
| `350130e` | docs | docs(grounding): refresh CODEMAP/SEAMS/HEAD_DELTAS/TRACEABILITY for PHASE4-N-E (partial close) |
| `43fcc31` | feat | feat(interop): N2C local-tx-submission -> mempool_ingress bridge (PHASE4-N-E S5) |
| `ca3f23a` | feat | feat(interop): N2N tx-submission2 -> mempool_ingress bridge (PHASE4-N-E S4) |
| `509d714` | feat | feat(ledger): per-peer ingress canonicalizer (PHASE4-N-E S3) |
| `2d0c918` | test | test(testkit): mempool ingress-replay harness + B-track corpus reuse (PHASE4-N-E S2) |
| `32c1ee6` | feat | feat(ledger): IngressEvent + mempool_ingress closed chokepoint (PHASE4-N-E S1) |
| `52642e5` | docs | docs(grounding): refresh CODEMAP/SEAMS/HEAD_DELTAS/TRACEABILITY + archive 7 closed cluster dirs |
| `168ac02` | fix | fix(testkit): snapshot-loader follow-ups (tip slot + Conway UMElem) |
| `c78ec76` | test | test(corpus): add reward_provenance generator (re-runnable, ignored) |
| `396664a` | test | test(corpus): align previously-blocked ade_testkit tests + ade_plutus compile with regenerated corpus |
| `b9cfaf9` | test | test(ledger): real-chain committee oracle, mainnet 575->576 (strengthens DC-EPOCH-01 + DC-LEDGER-10) |
| `3d94c22` | docs | docs(grounding): refresh CODEMAP/SEAMS/HEAD_DELTAS/TRACEABILITY + strengthen DC-EPOCH-01/DC-LEDGER-10 for ENACTMENT-COMMITTEE-WRITEBACK close |
| `69e2d4b` | test | test(ledger): harden update_committee decode + extend credential gate (ENACTMENT-COMMITTEE-WRITEBACK close) |
| `3180e27` | feat | feat(ledger): wire committee enactment write-back (ENACTMENT-COMMITTEE-WRITEBACK-S2) |
| `f2f15f9` | feat | feat(ledger): structured UpdateCommittee gov action (ENACTMENT-COMMITTEE-WRITEBACK-S1) |
| `ea25dd9` | docs | docs(ledger): ENACTMENT-COMMITTEE-WRITEBACK plan (wire committee enactment) |
| `3706534` | docs | docs(grounding): refresh CODEMAP/SEAMS/HEAD_DELTAS/TRACEABILITY for ENACTMENT-COMMITTEE-FIDELITY close |
| `a6b8de7` | feat | feat(ledger): discriminate EnactmentEffects.committee_changes (ENACTMENT-COMMITTEE-FIDELITY-S1) |
| `5d64fee` | docs | docs(ledger): ENACTMENT-COMMITTEE-FIDELITY plan (strengthens DC-LEDGER-10) |
| `06f517f` | docs | docs(grounding): refresh CODEMAP/SEAMS/HEAD_DELTAS/TRACEABILITY for DREP-VOTE-FIDELITY close |
| `62c9020` | test | test(ledger): DRep cross-resolve negative + CI gate, strengthen DC-LEDGER-10 (DREP-VOTE-FIDELITY-S2) |
| `ba4ff37` | feat | feat(ledger): discriminate drep_votes; exact-variant DRep stake resolution (DREP-VOTE-FIDELITY-S1) |
| `ecb0b92` | docs | docs(ledger): DREP-VOTE-FIDELITY plan (strengthens DC-LEDGER-10) |
| `a157c92` | docs | docs(grounding): refresh CODEMAP/SEAMS/HEAD_DELTAS/TRACEABILITY for COMMITTEE-CRED-FIDELITY close |
| `2aeea16` | test | test(ledger): committee cross-resolve negative + CI gate, strengthen DC-LEDGER-10 (COMMITTEE-CRED-FIDELITY-S2) |
| `2303a60` | feat | feat(ledger): discriminate committee member + vote credentials (COMMITTEE-CRED-FIDELITY-S1) |
| `32d7a2e` | docs | docs(ledger): COMMITTEE-CRED-FIDELITY plan (strengthens DC-LEDGER-10) |
| `676af5a` | docs | docs(grounding): refresh CODEMAP/SEAMS/HEAD_DELTAS/TRACEABILITY for OQ5 close |
| `a3ee2da` | test | test(ledger): credential-fidelity corpus + CI gate, enforce DC-LEDGER-10 (OQ5-S2) |
| `4187330` | feat | feat(types): discriminated StakeCredential end-to-end — preserve key/script tag (OQ5-S1) |
| `007b0e8` | docs | docs(ledger): OQ5-CREDENTIAL-FIDELITY cluster plan + cluster doc |
| `959e16c` | docs | docs(ledger): OQ-5 credential-fidelity invariants + DC-LEDGER-10 (declared) |
| `f81f815` | docs | docs(grounding): refresh CODEMAP/SEAMS/HEAD_DELTAS/TRACEABILITY for PHASE4-B5 close |
| `651adc9` | fix | fix(ledger): checked DRep-expiry arithmetic, deterministic fail-closed on overflow (PHASE4-B5-S5) |
| `06385d0` | test | test(ledger): gov-state accumulation corpus + CI gate, enforce DC-LEDGER-09 (PHASE4-B5-S4) |
| `d63c700` | feat | feat(ledger): apply gov-cert accumulation in block path, carry gov_state forward (PHASE4-B5-S3) |
| `7a48727` | feat | feat(ledger): native Conway gov-cert apply model — apply_conway_gov_cert (PHASE4-B5-S2) |
| `9c8d118` | feat | feat(ledger): gov-cert env infrastructure — drep_activity + GovCertEnv fail-fast (PHASE4-B5-S1) |
| `fdb6601` | docs | docs(gov): PHASE4-B5 invariants + cluster plan + DC-LEDGER-09 (Conway gov-cert accumulation) |
| `644eb03` | docs | docs(grounding): refresh CODEMAP/SEAMS/HEAD_DELTAS/TRACEABILITY for PHASE4-B4 close |
| `ee35493` | test | test(ledger): Conway cert-state accumulation corpus (PHASE4-B4-S5) |
| `302d22c` | feat | feat(ledger): era-dispatched fail-closed cert-state accumulation (PHASE4-B4-S3/S4) |
| `da30706` | feat | feat(ledger): native owner-tagged Conway cert apply model (PHASE4-B4-S2) |
| `228415b` | feat | feat(codec): owner-complete Conway certificate decoder (PHASE4-B4-S1) |
| `ae1300a` | docs | docs(planning): PHASE4-B4 grounding — invariants, cluster plan, cluster doc, B4-S1 slice (DC-LEDGER-08) |
| `1d989de` | docs | docs(grounding): refresh CODEMAP/TRACEABILITY/SEAMS/HEAD_DELTAS for PHASE4-B3F |
| `193d2fc` | feat | feat(codec): Conway cert decoder strictness — reject trailing bytes, bound preallocation (PHASE4-B3F) |
| `d6c1993` | feat | feat(ci): DC-TXV-06 cert-classification closure gate — flip partial to enforced (PHASE4-B3F) |
| `d766eb0` | chore | Close PHASE4-B3 — full Conway tx value-conservation accounting |
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
| `ade_runtime::producer::signing` (new file in an existing RED crate) | RED | **Producer crypto-substrate — RED-confined private-key custody and signing.** `VrfSigningKey([u8; 64])`, `KesSecret { ... }`, `ColdSigningKey { ... }` hold in-memory secret material with zeroize-on-drop. `vrf_prove(sk, alpha) -> (VrfProof, VrfOutput)` produces Praos VRF output byte-identical to cardano-node references; `kes_sign(sk, period, msg) -> KesSignature` produces Sum6KES output; `kes_update(sk, to) -> KesSecret` evolves forward only (rejects `to < from` or `to > from + evolutions_remaining`). Closed `SigningError`. No reads of wall-clock, env, fs. Private-key types do NOT appear in any public BLUE API surface. Enforced by `ci_check_private_key_custody.sh` and `DC-CRYPTO-03/04/05` + `OP-OPS-04`. | `producer/signing.rs` (~600 LOC) | PHASE4-N-C / S1 (`ea9770e`) |
| `ade_runtime::producer::keys` (new file in an existing RED crate) | RED | **cardano-cli `*.skey` text-envelope loader.** `load_vrf_signing_key_skey(path)`, `load_kes_signing_key_skey(path)`, `load_cold_signing_key_skey(path)` parse cardano-cli's text-envelope format, validate the type field against the canonical constants `VRF_SIGNING_KEY_TYPE`/`KES_SIGNING_KEY_TYPE`/`POOL_SIGNING_KEY_TYPE`, and produce RED in-memory secrets. Closed `KeyLoadError` taxonomy (Io, MalformedJson, WrongType, MalformedCborHex, …; error variants carry no path bytes — operator-PII discipline). **Open obligation** (`OP-OPS-04.open_obligation`): cardano-crypto 1.0.8 exposes only `gen_key_kes_from_seed_bytes` for Sum6Kes; real cardano-cli's 612-byte expanded-tree skey loading needs an upstream-fork-or-document call; CE-N-C-8 operator-action evidence must exercise whichever path closes. | `producer/keys.rs` (~376 LOC) | PHASE4-N-C / S1 (`ea9770e`) |
| `ade_core::consensus::opcert_validate` (new file in an existing BLUE crate) | BLUE | **BLUE op-cert validator (counter monotonicity + period gate + cold-sig verify).** `opcert_validate(opcert, prev_counter, slot, era_anchor, cold_vkey) -> Result<(), OpCertError>` rejects period mismatch (`period_at_slot(slot, anchor) = (slot - anchor) / slots_per_kes_period`), counter regression / repeat against `prev_counter`, short hot vkey (≠ 32 bytes), and bad ed25519 signature over the cold key. Closed `OpCertError::{CounterRepeat, CounterRegression, PeriodMismatch, ShortHotVkey, BadColdSignature}`. Re-used unchanged by S5's `self_accept`. Enforced by `ci_check_opcert_closed.sh` and `DC-CONS-11`/`DC-CONS-12`. | `consensus/opcert_validate.rs` (~234 LOC) | PHASE4-N-C / S2 (`4cf4b65`) |
| `ade_codec::shelley::opcert` (new file in an existing BLUE crate) | BLUE | **Closed-grammar op-cert encoder/decoder.** `encode_opcert`, `decode_opcert`, plus in-place helpers `write_opcert_fields_into` and `read_opcert_fields_from` used by the Shelley header codec. Cardano-cli byte-identical (S2 fixture `opcert_encoder_matches_cardano_cli_byte_identical`). Closed `OpCertCodecError`. The header codec in `shelley/block` was re-pointed at this module; the old inline opcert encode/decode block was removed. Enforced by `ci_check_opcert_closed.sh` (forbids parallel `encode_*opcert` / `write_opcert` outside this module). | `shelley/opcert.rs` (~375 LOC); registered in `shelley/mod.rs` | PHASE4-N-C / S2 (`4cf4b65`) |
| `ade_ledger::producer` (new submodule of an existing BLUE crate) | BLUE | **Producer authority core — the validation→producer leap.** Three files: `state.rs` (`ProducerTick` — canonical input bundling slot, epoch_nonce, vrf_proof, kes_signature, opcert, mempool_snapshot, pparams, era_anchor), `forge.rs` (the BLUE `forge_block` transition: leader-check gate via the validator's `is_leader_for_vrf_output` — no producer-side fork — plus tx-admissibility prefix gate via `mempool::admit::admit`, plus header assembly + body assembly + `header.body_hash` via S4's `block_body_hash`), and `self_accept.rs` (S5's BLUE bridge — `self_accept(forged) -> Result<AcceptedBlock, SelfAcceptError>` wraps N-B's `validate_and_apply_header` + B1's `block_validity`; `AcceptedBlock` is the load-bearing type-level broadcast gate). Originally planned at `ade_core::consensus::forge` but relocated to `ade_ledger::producer` because `ade_ledger` already depends on `ade_core` and the forge body needs `ade_ledger::{state::LedgerState, mempool::admit::*}`. BLUE classification unchanged. Closed `ForgeError`, `ForgedBlock`, `ForgeEffects`, `SelfAcceptError` (8-variant). Enforced by `ci_check_forge_purity.sh`, `ci_check_self_accept_gate.sh`, and `ci_check_no_private_keys_in_corpus.sh`; rules `DC-CONS-13/14/15`, `DC-LEDGER-12`, `CN-CONS-07`. | `producer/mod.rs`, `producer/state.rs` (~74 LOC), `producer/forge.rs` (~534 LOC at HEAD), `producer/self_accept.rs` (~364 LOC) | PHASE4-N-C / S3 + S4 + S5 (`8312690`, `4fd714c`, `aa7a7dd`) |
| `ade_ledger::block_body_hash` (new file in an existing BLUE crate) | BLUE | **Single canonical body-hash authority** consumed by both `forge_block` (producer) and `block_validity::header_input::computed_body_hash` (validator). `block_body_hash(&ShelleyBlock) -> Hash32` and `block_body_hash_from_buckets(...) -> Hash32` are the only two `pub fn block_body_hash{,_from_buckets}` definitions in BLUE. The S4 refactor eliminated all parallel body-hash recipes — no producer/validator encoder bifurcation is structurally possible. Enforced by `ci_check_no_producer_body_encoder.sh` and `DC-CONS-16`. | `block_body_hash.rs` (~147 LOC); registered in `ade_ledger/src/lib.rs` | PHASE4-N-C / S4 (`4fd714c`) |
| `ade_runtime::producer::tick_assembler` (new file in an existing RED crate) | GREEN | **Composes canonical `ProducerTick` from captured RED outputs.** `assemble_tick(inputs: TickInputs) -> Result<ProducerTick, TickAssemblyError>` is pure — no I/O, no clock, no rand, no async. Observably deterministic: two replays over the same `TickInputs` yield byte-identical `ProducerTick`. Closed `TickAssemblyError`. Bridges the RED scheduler / signing outputs into the BLUE `forge_block` input shape. Enforced by `ci_check_scheduler_closure.sh`. | `producer/tick_assembler.rs` (~211 LOC) | PHASE4-N-C / S6 (`58678af`) |
| `ade_runtime::producer::scheduler` (new file in an existing RED crate) | RED | **Slot-wakeup RED loop driving the producer pipeline.** `scheduler_step<L: LedgerView>(state, input) -> (state', Vec<SchedulerEffect>)` is the slot-driven transition driving the RED→GREEN→BLUE→BLUE→RED sequence (slot wakeup → RED signing → GREEN tick assembly → BLUE forge → BLUE self_accept → RED broadcast). Closed `SchedulerInput`, `SchedulerEffect`, `SchedulerState`, `SchedulerHaltReason` taxonomies. Self-accept failure → deterministic halt (`SchedulerHaltReason::SelfAcceptFailed`); the scheduler never emits a broadcast effect without a freshly-acquired `AcceptedBlock`. Enforced by `ci_check_scheduler_closure.sh` and `OP-OPS-05`. | `producer/scheduler.rs` (~478 LOC) | PHASE4-N-C / S6 (`58678af`) |
| `ade_runtime::producer::broadcast` (new file in an existing RED crate) | RED | **Outbound queue handing self-accepted bytes to `ade_network`'s N2N server path.** `BroadcastQueue::send(&mut self, block: AcceptedBlock)` is the only entry point; the argument type `AcceptedBlock` cannot be constructed outside `self_accept` (type-level broadcast gate, CN-CONS-07). Closed `BroadcastError`. Scope: enough delivery for cardano-node to fetch the block via block-fetch / chain-sync server; full relay-mesh behaviour is N-A successor scope. | `producer/broadcast.rs` (~265 LOC) | PHASE4-N-C / S6 (`58678af`) |
| `ade_testkit::producer` (new submodule of an existing crate) | GREEN | **In-code synthetic producer corpus + replay + cross-impl adapter.** `fixtures.rs` (~274 LOC: 3 canonical synthetic `ProducerTick` fixtures — `fixture_empty_mempool_leader`, `fixture_non_leader`, `fixture_two_tx_mempool_leader`; `all_fixtures()`). `replay.rs` (~216 LOC: `ProducerReplayFixture`, `producer_replay_fixtures() -> Vec<ProducerReplayFixture>` with captured `expected_forged: Vec<Vec<u8>>` per tick — drives `forge_block_replay_byte_identical`). `reference_vectors.rs` (~130 LOC: `VrfReferenceVector`, `KesReferenceVector`, `vrf_reference_set`, `kes_reference_set`, `kes_update_reference_chain`). `cross_impl_adapter.rs` (~184 LOC, S7: drives every fixture through the full pipeline and asserts three honest structural properties — decode round-trip, S4 body-hash binding, decoder/encoder structural field agreement on `body_hash`/`sequence_number`/`kes_period`). All synthetic — canonical by construction. **No on-disk corpus files** per OQ-12: corpus lives in code so replay never invokes RED signing and replay corpora never carry private keys. Regen binary `crates/ade_testkit/tests/regen_producer_fixtures.rs`. | `producer/mod.rs`, `producer/fixtures.rs`, `producer/replay.rs`, `producer/reference_vectors.rs`, `producer/cross_impl_adapter.rs`; registered in `ade_testkit/src/lib.rs` | PHASE4-N-C / S1 + S3 + S4 + S7 (`ea9770e`, `8312690`, `4fd714c`, `694dd74`) |
| `ade_core_interop` bin `live_block_production_session` (new RED binary in an existing RED crate) | RED | **Sustained-window operator-action live-evidence probe for CE-N-C-8.** Modeled on `live_consensus_session` (CE-N-B-6) and `live_tx_submission_session` (CE-N-E-6). Connects to a private cardano-node over N2N, negotiates handshake, opens block-fetch / chain-sync server roles, hands off Ade-forged bytes, and observes cardano-node's accept / reject verdict. Captures `docs/clusters/PHASE4-N-C/CE-N-C-LIVE_<date>.log`. **Conditional on testnet SPO stake**: at HEAD, status is `blocked_until_operator_stake_available`, tracked as `CN-CONS-06.open_obligation`. Operator procedure documented at `docs/clusters/PHASE4-N-C/CE-N-C-8_PROCEDURE.md`. Honest scope: structural cross-impl agreement lives in S7's mechanical adapter; this binary closes the crypto-level cross-impl claim (real KES/VRF signatures accepted over N2N). | `src/bin/live_block_production_session.rs` (~247 LOC); `[[bin]]` entry in `crates/ade_core_interop/Cargo.toml`; operator procedure at `docs/clusters/PHASE4-N-C/CE-N-C-8_PROCEDURE.md` | PHASE4-N-C / S7 (`694dd74`) |
| `ade_codec::conway::governance` (new file in an existing BLUE crate) | BLUE | **Closed-grammar Conway proposal-procedures decoder + encoder** — see prior-thread carry-forward narrative. `decode_proposal_procedures` is the single sanctioned entry from the Conway tx-body codec at key 20. Enforced by `ci_check_proposal_procedures_closed.sh` and `DC-LEDGER-11`. | `conway/governance.rs` (~856 lines incl. 14 inline unit tests); registered in `conway/mod.rs` | PROPOSAL-PROCEDURES-DECODE / PP-S1 (`70bc85b`) |
| `ade_testkit::governance::proposal_procedures_replay` (new submodule + file in an existing crate) | GREEN | **Canonical synthetic replay harness for the closed `proposal_procedures` decoder.** 9 entries; per OQ-5 corpus is synthetic-canonical. | `governance/mod.rs`, `governance/proposal_procedures_replay.rs` (~232 lines) | PROPOSAL-PROCEDURES-DECODE / PP-S2 (`928c2be`) |
| `ade_core_interop` bin `live_tx_submission_session` (new RED binary in an existing RED crate) | RED | Sustained-window N2N tx-submission2 live-evidence probe. CE-N-E-6 closure-gate. | `src/bin/live_tx_submission_session.rs` (~552 LOC); operator procedure + live log under `docs/clusters/completed/PHASE4-N-E/` | PHASE4-N-E / S6 (`d1068b3` + `caa5ce8`) |
| `ade_ledger::mempool::ingress` (new file in an existing BLUE crate) | BLUE | Single closed wire-level ingress chokepoint. `IngressEvent`/`IngressSource::{N2N,N2C}`/`mempool_ingress`. | `mempool/ingress.rs` | PHASE4-N-E / S1 (`32c1ee6`) |
| `ade_ledger::mempool::canonicalize` (new file in an existing BLUE crate) | GREEN | Deterministic per-peer ingress canonicalizer. | `mempool/canonicalize.rs` | PHASE4-N-E / S3 (`509d714`) |
| `ade_testkit::mempool::ingress_replay` (new submodule of an existing crate) | GREEN | Single-step ingress-replay harness over B-track corpus. | `mempool/mod.rs`, `mempool/ingress_replay.rs` | PHASE4-N-E / S2 (`2d0c918`) |
| `ade_core_interop::tx_submission` (new file in an existing RED crate) | GREEN | N2N tx-submission2 → `mempool_ingress` bridge. | `src/tx_submission.rs`; `tests/tx_submission_ingress.rs` (7 integration tests) | PHASE4-N-E / S4 (`ca3f23a`) |
| `ade_core_interop::local_tx_submission` (new file in an existing RED crate) | GREEN | N2C local-tx-submission → `mempool_ingress` bridge. | `src/local_tx_submission.rs`; `tests/local_tx_submission_ingress.rs` (8 integration tests) | PHASE4-N-E / S5 (`43fcc31`) |
| `ade_codec::conway::cert` (new file in an existing BLUE crate) | BLUE | Conway-complete certificate decoder with a closed wire grammar; trailing-byte reject + bounded preallocation in B3F-S2. | `conway/cert.rs` | PHASE4-B3 / B3-S1, B3-S2; strictness PHASE4-B3F / B3F-S2 |
| `ade_codec::conway::withdrawals` (new file in an existing BLUE crate) | BLUE | Conway withdrawals-map decoder. | `conway/withdrawals.rs` | PHASE4-B3 / B3-S3 |
| `ade_ledger::cert_classify` (new file in an existing BLUE crate) | BLUE | Closed cert-deposit classification — `classify(state, cert)` total, era-versioned. | `cert_classify.rs` | PHASE4-B3 / B3-S2; closure gate B3F / B3F-S1 |
| `ade_ledger::gov_cert` (new file in an existing BLUE crate) | BLUE | Native Conway governance-certificate accumulation. | `gov_cert.rs` | PHASE4-B5 / B5-S2; B5-S1 env; B5-S3 block-path; B5-S5 checked arithmetic |
| `ade_ledger::tx_validity` (new submodule of an existing BLUE crate) | BLUE | Per-transaction verdict authority. | `mod.rs`, `verdict.rs`, `required_signers.rs`, `witness.rs`, `phase1.rs`, `transition.rs`, `encoding.rs` | PHASE4-B2 / B2-S1, B2-S2 |
| `ade_ledger::mempool` (new submodule of an existing BLUE crate) | BLUE (`admit` + `ingress`) / GREEN (`policy` + `canonicalize`) | Two-layer mempool: BLUE `admit` requires `tx_validity` Valid; GREEN `policy` does eviction/ordering. PHASE4-N-E added the BLUE `ingress` chokepoint + GREEN `canonicalize`. | `mod.rs`, `admit.rs`, `policy.rs`, `ingress.rs` (N-E S1), `canonicalize.rs` (N-E S3) | PHASE4-B2 / B2-S5; PHASE4-N-E / S1, S3 |
| `ade_testkit::tx_validity` (new submodule of an existing crate) | GREEN | Test-only tx-validity harness. | `tx_validity/mod.rs`, etc. | PHASE4-B2 / B2-S3, B2-S4; B3 extensions |
| `ade_ledger::block_validity` (new submodule of an existing BLUE crate) | BLUE | Full-block verdict authority. | `mod.rs`, `verdict.rs`, `transition.rs`, `header_input.rs`, `encoding.rs` | PHASE4-B1 / B1-S3, B1-S4 |
| `ade_ledger::consensus_view` (new file in an existing BLUE crate) | BLUE | Production `LedgerView` projection. | `consensus_view.rs` | PHASE4-B1 / B1-S2 |
| `ade_ledger::consensus_input_extract` (new file in an existing BLUE crate) | RED | Tail-scan of a snapshot `state` CBOR for the five `PraosState` nonces. | `consensus_input_extract.rs` | PHASE4-B1 / B1-S1 |
| `ade_core::consensus::kes_check` (new file in an existing BLUE crate) | BLUE | Fail-closed wiring of `ade_crypto::kes` into Praos header validation. | `kes_check.rs` | PHASE4-B1 / B1-S5 |
| `ade_testkit::validity` (new submodule of an existing crate) | GREEN | Test-only block-validity harness. | `validity/mod.rs`, etc. | PHASE4-B1 / B1-S6, B1-S7 |
| `ade_core_interop::follow` (new file in an existing RED crate) | RED | Follow-mode bridge. | `follow.rs`, `tests/follow_offline_replay.rs` | CE-N-B-6 (`e5f1f64`) |
| `ade_network` (new workspace crate) | BLUE-majority (per-submodule scoped) | Ouroboros mini-protocol authority. | `codec/`, `handshake/`, `chain_sync/`, `block_fetch/`, `tx_submission/`, `keep_alive/`, `peer_sharing/`, `n2c/`, `mux/frame.rs` (BLUE), `mux/transport.rs` (RED), `session/` (RED) | PHASE4-N-A / S-A1 → S-A10 |
| `ade_core::consensus` (new submodule of an existing BLUE crate) | BLUE | Praos consensus authority. **N-C S2 added `opcert_validate.rs` to this submodule.** | `mod.rs`, `era_schedule.rs`, `header_validate.rs`, `vrf_cert.rs`, `nonce.rs`, `op_cert.rs`, `leader_schedule.rs`, `fork_choice.rs`, `rollback.rs`, `kes_check.rs` (B1), `praos_state.rs`, `candidate.rs`, `events.rs`, `errors.rs`, `encoding.rs`, `ledger_view.rs`, `header_summary.rs`, `opcert_validate.rs` (N-C S2) | PHASE4-N-B / S-B1 → S-B9; PHASE4-N-C / S2 |
| `ade_runtime::consensus` (new submodule of an existing RED crate) | GREEN/RED mix | Imperative-shell composition for consensus runtime. | `mod.rs`, `candidate_fragment.rs`, `chain_selector.rs`, `genesis_parser.rs` | PHASE4-N-B / S-B8, S-B10 |
| `ade_runtime::producer` (new submodule of an existing RED crate) | RED + GREEN mix | **Imperative-shell composition for producer runtime** — signing/keys (S1, RED), scheduler/broadcast (S6, RED), tick_assembler (S6, GREEN). | `mod.rs`, `signing.rs`, `keys.rs`, `scheduler.rs`, `broadcast.rs`, `tick_assembler.rs` | PHASE4-N-C / S1, S6 |
| `ade_core_interop` (new workspace crate) | RED | Live cardano-node interop driver. **N-C S7 added `live_block_production_session.rs`.** | `src/lib.rs`, `src/follow.rs`, `src/tx_submission.rs` (N-E S4), `src/local_tx_submission.rs` (N-E S5), `src/bin/live_consensus_session.rs`, `src/bin/live_tx_submission_session.rs` (N-E S6), `src/bin/live_block_production_session.rs` (**N-C S7**), `tests/` | PHASE4-N-B / S-B10; follow-bridge `e5f1f64`; PHASE4-N-E / S4–S6; PHASE4-N-C / S7 |
| `ade_testkit::consensus` (new submodule of an existing crate) | GREEN | Test-only harness for consensus replay corpora. | `consensus/mod.rs`, `consensus/corpus.rs`, `consensus/ledger_view_stub.rs`, `consensus/stream_replay.rs` | PHASE4-N-B / S-B1, S-B6, S-B8 → S-B10 |
| `ade_runtime::chaindb` | RED | Block-store abstraction and impls. | `mod.rs`, `types.rs`, `error.rs`, `in_memory.rs`, `persistent.rs` (redb), `contract.rs`, `snapshot_contract.rs`, `crash_safety.rs` | PHASE4-N-D / S-33 → S-35 |
| `ade_runtime::recovery` | RED | Composes ChainDb + SnapshotStore. | `recovery.rs` | PHASE4-N-D / S-36 |
| `ade_runtime` bin `chaindb_kill_target` | RED | Kill-target child process for the 1,000-kill-9 stress harness. | `src/bin/chaindb_kill_target.rs`, `tests/stress_kill_harness.rs` | PHASE4-N-D / S-37 |

Workspace-level membership grew by **two crates** across the full
delta: `ade_network` (PHASE4-N-A) and `ade_core_interop` (PHASE4-N-B).
Both are RED-or-mixed. **PHASE4-N-C added no new crate** — S1's
RED signing/keys, S2's BLUE opcert pair, S3+S4+S5's BLUE producer
submodule, S6's RED/GREEN scheduler/tick-assembler/broadcast, and
S7's mechanical cross-impl adapter + RED binary all live as new
files / submodules under the existing 8 workspace crates.
**PROPOSAL-PROCEDURES-DECODE added no new crate; PHASE4-N-E
collectively added no new crate either.**

Crate dependency shape at HEAD: **PHASE4-N-C S6 added one new dep
edge** — `ade_runtime` now depends directly on `ade_ledger` (was
transitive only). The edge is required because the RED scheduler /
broadcast must consume the `AcceptedBlock` token type from
`ade_ledger::producer`. Dependency direction RED → BLUE is permitted
by `ci_check_dependency_boundary.sh`. **PHASE4-N-C S7 added no new
dep edge** — `ade_core_interop` already had its `ade_ledger` edge
from N-E S4. The prior **PHASE4-N-E S4** edge
(`ade_core_interop -> ade_ledger`) is carried forward.
**PROPOSAL-PROCEDURES-DECODE added no new dep edge.** No edge from
a BLUE crate to a RED crate was introduced.

Corpora at HEAD: N-A capture corpus, N-B replay corpus, B1 validity
corpus, B3 conservation corpora, B4/B5 README-only synthetic notes,
the credential-fidelity corpus from OQ5-S2, the PPD in-code
synthetic-canonical corpus, and `corpus/snapshots/` under
`.gitignore` (canonical home `s3://ade-corpus-snapshots`).
**PHASE4-N-C added no external corpus** — the producer corpus lives
in-code under `ade_testkit::producer::fixtures` (3 canonical
synthetic `ProducerTick` fixtures + captured `expected_forged`
bytes per tick), per the OQ-12 discipline that replay corpora never
carry private-key material. Real-chain cross-impl corpus extraction
is the operator-action evidence captured by CE-N-C-8's live binary,
not a committed corpus.

Cross-reference: **The `ade-CODEMAP.md` regenerated in parallel with
this HEAD_DELTAS will record the new BLUE submodule
`ade_ledger::producer` (with `forge`/`state`/`self_accept`), the new
BLUE module `ade_ledger::block_body_hash`, the new BLUE modules
`ade_core::consensus::opcert_validate` and
`ade_codec::shelley::opcert`, the new RED submodule
`ade_runtime::producer` (with `signing`/`keys`/`scheduler`/`broadcast`),
the new GREEN file `ade_runtime::producer::tick_assembler`, the new
test-harness `ade_testkit::producer`, and the new RED binary
`ade_core_interop::live_block_production_session`** as rows under
their respective crates' BLUE/GREEN/RED listings; the prior CODEMAP
at `96d043c` does NOT yet contain any of those. SEAMS will pick up
`forge_block` as the canonical producer entry slice, `AcceptedBlock`
as the type-level broadcast gate, `block_body_hash` as the single
canonical body-hash authority, and the new RED→BLUE edge
`ade_runtime -> ade_ledger`. TRACEABILITY will pick up the 14 new
registry rules (`DC-CRYPTO-03/04/05`, `DC-CONS-11/12/13/14/15/16`,
`DC-LEDGER-12`, `CN-CONS-06/07`, `OP-OPS-04/05`) with their 8 new
`ci_script ↔ rule` edges; the prior TRACEABILITY at `96d043c` does
NOT contain any of them. All three rewrites are in flight in the
grounding ripple immediately following this HEAD_DELTAS regen; the
four docs will be self-consistent at the next grounding-doc commit.

---

## 3. Modules Modified

| Module | Scope | Key changes |
|--------|-------|-------------|
| `ade_ledger` | +75 source/test files over the full delta; **PHASE4-N-C touched 8 files**: `producer/{mod.rs, forge.rs, state.rs, self_accept.rs}` (new), `block_body_hash.rs` (new), `block_validity/header_input.rs` (re-pointed at S4 authority, +19/-…), `mempool/admit.rs` (+32: prefix-helper for tx-admissibility), `lib.rs` (+2: register new modules). | **PHASE4-N-C (S3 + S4 + S5):** new BLUE producer submodule (`forge_block`, `ProducerTick`, `self_accept`, `AcceptedBlock`, `SelfAcceptError`); new single canonical body-hash authority `block_body_hash.rs`; `block_validity::header_input::computed_body_hash` re-pointed at it; `mempool::admit` exposes a prefix helper for the producer's tx-admissibility gate. **Carried forward:** PHASE4-N-E mempool ingress chokepoint, B-series tx_validity/block_validity/cert_classify/gov_cert/conway value-conservation, OQ5/FIDELITY/WRITEBACK credential discriminant work, PROPOSAL-PROCEDURES-DECODE OQ-8 test-fixture scrub. |
| `ade_runtime` | +24 files, +5,840 lines (N-B `consensus/` + N-D `chaindb`/`recovery`; B1 one small touch; **PHASE4-N-C added 6 files / +1,953 LOC** under the new `producer/` submodule). | **PHASE4-N-C (S1 + S6):** new RED `producer/signing.rs` (~600 LOC), new RED `producer/keys.rs` (~376 LOC), new RED `producer/scheduler.rs` (~478 LOC), new GREEN `producer/tick_assembler.rs` (~211 LOC), new RED `producer/broadcast.rs` (~265 LOC), new `producer/mod.rs`. New integration test `tests/producer_pipeline_slot_deadline.rs` (~190 LOC). `Cargo.toml` gains an `ade_ledger` dep + crypto features; `lib.rs` registers `producer`. **Carried forward:** N-B consensus runtime, N-D chaindb/recovery (§2 New Modules). |
| `ade_core` | +30 source files + tests (N-B); +828 / −86 across 16 files (B1); **PHASE4-N-C added 1 new file** (`consensus/opcert_validate.rs`, ~234 LOC, S2) plus `consensus/mod.rs` (+2). | **PHASE4-N-C (S2):** new BLUE `opcert_validate` module — closed `OpCertError` taxonomy; counter monotonicity + period-at-slot gate + ed25519 cold-signature verify. `Cargo.toml` adds the dependency the validator needs. **Carried forward:** the N-B consensus authority. |
| `ade_codec` | +14 source/test files over the full delta (B3 + B3F + B4 + OQ5 + PROPOSAL-PROCEDURES-DECODE PP-S1 + **PHASE4-N-C S2 + S3**). | **PHASE4-N-C (S2):** new BLUE module `shelley/opcert.rs` (~375 LOC: `encode_opcert`, `decode_opcert`, `write_opcert_fields_into`, `read_opcert_fields_from`, `OpCertCodecError`); `shelley/mod.rs` (+1) registers it; `shelley/block.rs` (47-line net rewrite) re-points header path at the new module. **PHASE4-N-C (S3):** `shelley/tx_components.rs` (+152) adds the producer assembly path's body-shape helpers. **Carried forward:** PPD PP-S1 `conway::governance`; B3 `conway::cert` + `conway::withdrawals`; B4 owner-complete `decode_conway_certs`; OQ5 era decoder discriminant work. |
| `ade_crypto` | 2 files: `kes.rs` (+122 / −81 across S1 + B1) and `lib.rs` (+5). | **PHASE4-N-C (S1):** `kes.rs` (+122) adds `KesSignature` (newtype wrapper with `from_bytes` round-trip) and `verify_kes_signature` (re-exposes the depth-6 verify under the canonical name S5/S6 consume); `lib.rs` (+5) re-exports both. **Carried forward (B1-S5):** `build_opcert_signable` fix. Note that the existing `verify_*` paths in `ade_crypto::vrf` / `ade_crypto::kes` are otherwise **unchanged in PHASE4-N-C** per OQ-12 (zero validator regressions). |
| `ade_testkit` | +33 files across the full delta (N-B/B1/B2/B3/OQ5/N-E/PPD/**PHASE4-N-C** layers). PHASE4-N-C added 6 files / +864 LOC under `producer/` + 1 regen binary. | **PHASE4-N-C:** new GREEN submodule `producer/{mod.rs, fixtures.rs, replay.rs, reference_vectors.rs, cross_impl_adapter.rs}` (S1: reference_vectors; S3 + S4: fixtures + replay; S7: cross_impl_adapter); `lib.rs` (+1) registers it; `Cargo.toml` (+6) adds the new crate deps; new test `tests/regen_producer_fixtures.rs` (~61 LOC) supports re-deriving the captured `expected_forged` bytes. **Carried forward:** PPD PP-S2 `governance/proposal_procedures_replay`; N-E `mempool/ingress_replay`; B3 snapshot-loader extensions; OQ5 → WRITEBACK ripples. |
| `ade_core_interop` | +1,793 across 9 files (B1/CE-N-B-6 + N-E S4/S5/S6 + **PHASE4-N-C S7**). | **PHASE4-N-C (S7):** new RED binary `src/bin/live_block_production_session.rs` (~247 LOC) modeled on `live_consensus_session` and `live_tx_submission_session`; new `[[bin]]` entry in `Cargo.toml` (+4). **Carried forward (N-E S4/S5/S6):** N2N + N2C tx-submission bridges + sustained-window live binary. **Carried forward (CE-N-B-6):** follow-bridge + pin retarget. |
| `ade_network` | 100 files, +17,861 lines (full N-A). **No PHASE4-N-C source change** — the new producer modules consume `ade_network`'s codecs through existing crate edges. | DoS hardening of 6 codecs (`744ef34`, post-N-A close). |

No other crate had non-trivial source changes since baseline.
`ade_plutus` and `ade_node` were untouched by code commits.
**PHASE4-N-C touched 7 of 8 workspace crates** (`ade_ledger`,
`ade_runtime`, `ade_core`, `ade_codec`, `ade_crypto`, `ade_testkit`,
`ade_core_interop`) — every crate except `ade_network`, `ade_plutus`,
and `ade_node`. **No `.idd-config.json` change.** **No BLUE
authority-path semantics changed apart from the S2 + S3 + S4 + S5
new closed surfaces** — the prior validator authorities
(`ade_core::consensus::*` and `ade_ledger::block_validity::*`) were
re-used unchanged by S5's `self_accept` per OQ-12.

---

## 4. Feature Flags

No Cargo `[features]` tables exist at HEAD in any workspace crate, and
none existed at baseline. The project does not use Cargo feature flags
as a semantic surface — closed semantic surfaces are encoded in the
type system per the IDD core principles, and conditional compilation
is checked out of BLUE code via `ci/ci_check_no_semantic_cfg.sh`
(scoped over the full 6-crate BLUE set, covering all surfaces
introduced through the PHASE4-N-C producer chokepoint, body-hash
authority, opcert codec/validator, and self-accept bridge).

No `#[cfg(feature = ...)]` gates appear at either ref. `cardano-crypto`
(`vrf-draft03`, `kes-sum`, `dsign`) and `minicbor` (`alloc`) feature
selections in the dependency entries are upstream-crate selections,
not Ade-side flags. **PHASE4-N-C S1 widened the `cardano-crypto`
feature set on `ade_runtime` to include `kes-sum` + `dsign`** (the
producer's signing primitives need them); this is an upstream-crate
selection, not a new Ade feature flag.

**Status: unchanged — zero Ade feature flags at baseline, zero at HEAD.**

---

## 5. CI Checks

The CI surface is the shell-script set under `ci/` (no
`.github/workflows` in this repo). At baseline there were 15 scripts.
At HEAD there are **40 scripts plus one git hook**
(`ci/git-hooks/commit-msg`). Across the full delta: CE-73 added one,
N-D added three, N-A added two, N-B added four, B3 added one, B3F
added one, B5 added one, OQ5 added one (the 29th), PHASE4-N-E S1/S2
added two (the 30th and 31st), PROPOSAL-PROCEDURES-DECODE PP-S1 added
one (the 32nd), and **PHASE4-N-C added eight (the 33rd through 40th)**:
`ci_check_private_key_custody.sh` (S1),
`ci_check_opcert_closed.sh` (S2),
`ci_check_forge_purity.sh` (S3),
`ci_check_no_private_keys_in_corpus.sh` (S3),
`ci_check_no_producer_body_encoder.sh` (S4),
`ci_check_self_accept_gate.sh` (S5),
`ci_check_scheduler_closure.sh` (S6),
`ci_check_producer_corpus_present.sh` (S7).
Grouped by cluster.

### CE-73 reclassification (Phase 2C close-out)

| Check | Status | What it checks |
|-------|--------|----------------|
| `ci/ci_check_hfc_translation.sh` | **New** (`9b15378`) | CE-73-semantic gate: runs the three HFC ledger-side translation proof surfaces. Authoritative test for invariant `DC-EPOCH-02`. |

### IDD canonicalization (post-Phase-4-N-D)

| Check | Status | What it checks |
|-------|--------|----------------|
| `ci/ci_check_constitution_coverage.sh` | Modified (`39865f6`, `aa7a7dd`) | Path-only edit (`39865f6`): registry path now `docs/ade-invariant-registry.toml`. **Extended** (`aa7a7dd`, N-C S5, +16/-…) to recognize new closed types under `producer/`. |
| `ci/git-hooks/commit-msg` | **New** (`2047c42`) | Local git hook: rejects commit messages lacking a `Co-Authored-By: Claude ...` trailer. |

### BLUE-list drift closure (`5b70bee`)

| Check | Status | What it checks |
|-------|--------|----------------|
| `ci/ci_check_module_headers.sh` | Modified — BLUE-scope (`5b70bee`) | `// Core Contract:` header on every `.rs` in BLUE crates. |
| `ci/ci_check_no_semantic_cfg.sh` | Modified — BLUE-scope (`5b70bee`) | No semantic `#[cfg(...)]` in BLUE `src/`. |
| `ci/ci_check_no_signing_in_blue.sh` | Modified — BLUE-scope (`5b70bee`) | No signing primitives in BLUE crates. |
| `ci/ci_check_hash_uses_wire_bytes.sh` | Modified — BLUE-scope (`5b70bee`) | All BLUE hashing via wire-byte fingerprint surfaces. |
| `ci/ci_check_ingress_chokepoints.sh` | Modified — BLUE-scope + registry growth (`5b70bee`) | No raw CBOR decoding outside named chokepoints in BLUE. |
| `ci/ci_check_dependency_boundary.sh` | Modified — BLUE-scope (`5b70bee`) | BLUE crates must not depend on RED crates. Continues to PASS at HEAD: the N-C S6 new edge is RED → BLUE (`ade_runtime -> ade_ledger`), which is permitted. |

### Phase 4 N-D CI gap closure (`78da6c9`)

| Check | Status | What it checks |
|-------|--------|----------------|
| `ci/ci_check_chaindb_contract.sh` | **New** (`78da6c9`) | `cargo test -p ade_runtime --lib chaindb::` — 8 contract tests. |
| `ci/ci_check_recovery_contract.sh` | **New** (`78da6c9`) | `cargo test -p ade_runtime --lib recovery::` — 6-test recovery bundle. |
| `ci/ci_check_chaindb_crash_safety.sh` | **New** (`78da6c9`) | Smoke variant of the subprocess-SIGKILL harness + integrity post-checks. |

### Phase 4 N-A wire + semantic enforcement (S-A1, S-A10)

| Check | Status | What it checks |
|-------|--------|----------------|
| `ci/ci_check_no_async_in_blue.sh` | **New** (`4fde3a7`, S-A1) | Enforces `DC-CORE-01` — BLUE code is sync-only. |
| `ci/ci_check_ce_n_a_5_proof.sh` | **New** (`56bfa7b`, S-A10) | CE-N-A-5 closure-gate evidence over the real-cardano-node corpus. |

### Phase 4 N-B consensus authority enforcement (S-B1, S-B2, S-B8) — extended by B1, B2, and N-C S3/S6

| Check | Status | What it checks |
|-------|--------|----------------|
| `ci/ci_check_consensus_closed_enums.sh` | **New** (N-B); **Modified** (B1, `7b95ccd`); **Modified** (B2); **Modified** (N-C, implicit via new closed enums under `producer/`) | Closed-enum scan over `ade_core/src/consensus/`, `ade_ledger/src/block_validity/`, `ade_ledger/src/tx_validity/`, `ade_ledger/src/mempool/`, and (now) `ade_ledger/src/producer/`. |
| `ci/ci_check_no_chaindb_in_consensus_blue.sh` | **New** (N-B / S-B1) | No `ChainDb`/`chain_db` token in `consensus/`. |
| `ci/ci_check_no_density_in_fork_choice.sh` | **New** (N-B / S-B8) | No `density` token in `fork_choice.rs` / `candidate.rs`. |
| `ci/ci_check_no_float_in_consensus.sh` | **New** (N-B / S-B1) | No `f32`/`f64` in `consensus/`. |

### Phase 4 B3 Conway value-conservation enforcement (`978c222`)

| Check | Status | What it checks |
|-------|--------|----------------|
| `ci/ci_check_deposit_param_authority.sh` | **New** (`978c222`) | Enforces `DC-TXV-07` (canonical deposit-param authority). |

### Phase 4 B3F cert-classification closure enforcement (`d6c1993`)

| Check | Status | What it checks |
|-------|--------|----------------|
| `ci/ci_check_conway_cert_classification_closed.sh` | **New** (`d6c1993`, B3F-S1) | Enforces `DC-TXV-06` — flips `partial` → `enforced`. |

### Phase 4 B4 cert-state-accumulation fail-closed enforcement (`302d22c`)

| Check | Status | What it checks |
|-------|--------|----------------|
| `ci/ci_check_forbidden_patterns.sh` | **Modified** (`302d22c`, B4-S3/S4) | Enforces `DC-LEDGER-08` — no `non-fatal during replay` rationale; no `Err(_) =>` swallow arm in `accumulate_tx_certs`. |

### Phase 4 B5 governance-cert-accumulation fail-closed enforcement (`06385d0`)

| Check | Status | What it checks |
|-------|--------|----------------|
| `ci/ci_check_gov_cert_accumulation_closed.sh` | **New** (`06385d0`, B5-S4) | Enforces `DC-LEDGER-09` — four-part grep-gate over `apply_conway_gov_cert` totality, `checked_add` arithmetic, observe-and-drop removal, env fail-fast wiring. |

### OQ5 / FIDELITY / WRITEBACK credential discriminant gate (single script, extended six times)

| Check | Status | What it checks |
|-------|--------|----------------|
| `ci/ci_check_credential_discriminant_closed.sh` | **New** (`a3ee2da`, OQ5-S2) | Enforces `DC-LEDGER-10`. Three OQ5 clauses + 2 committee + 2 DRep + 1 enactment-effect + section 7 (WRITEBACK). **Unmodified by PHASE4-N-C.** |

### PHASE4-N-E wire-level mempool ingress closure (`32c1ee6`, `2d0c918`, `509d714`)

| Check | Status | What it checks |
|-------|--------|----------------|
| `ci/ci_check_mempool_ingress_closure.sh` | **New** (`32c1ee6`, S1) — the **30th** script | Enforces `DC-MEM-03` via 5 mechanical guards. |
| `ci/ci_check_mempool_ingress_replay.sh` | **New** (`2d0c918`, S2); **Modified** (`509d714`, S3, +2 clauses) — the **31st** script | Enforces `DC-MEM-04` via 6 mechanical guards. |

### PROPOSAL-PROCEDURES-DECODE closure enforcement (`70bc85b`)

| Check | Status | What it checks |
|-------|--------|----------------|
| `ci/ci_check_proposal_procedures_closed.sh` | **New** (`70bc85b`, PP-S1) — the **32nd** script | Enforces `DC-LEDGER-11` via 5 mechanical guards. |

### PHASE4-N-C block-production closure (`ea9770e`, `4cf4b65`, `8312690`, `4fd714c`, `aa7a7dd`, `58678af`, `694dd74`) — 8 new scripts (the 33rd → 40th)

| Check | Status | What it checks |
|-------|--------|----------------|
| `ci/ci_check_private_key_custody.sh` | **New** (`ea9770e`, S1) — the **33rd** script | Enforces `DC-CRYPTO-03/04/05` + `OP-OPS-04` via 5+ mechanical guards: (1) no private-key types (`*SigningKey`, `KesSecret`, cold-key types) defined outside `crates/ade_runtime/src/producer/`; (2) no `cardano_crypto::vrf::prove` / `kes::sign_kes` / `kes::update_kes` call outside `producer/` in production code (test modules are excluded for verify-path self-consistency tests); (3) no private-key types in BLUE public APIs (`ade_core`, `ade_codec`, `ade_types`, `ade_ledger`, `ade_crypto`); (4) `KeyLoadError` carries no path bytes (operator-PII discipline); (5) custody is RED-shell only — no signing primitives reachable from BLUE. |
| `ci/ci_check_opcert_closed.sh` | **New** (`4cf4b65`, S2) — the **34th** script | Enforces `DC-CONS-11` + `DC-CONS-12` via 6+ mechanical guards: (1) no parallel opcert encoders — the only `pub fn .*encode.*opcert` / `pub fn .*write_opcert` definitions in BLUE source live in `crates/ade_codec/src/shelley/opcert.rs`; (2) opcert codec round-trips byte-identical against the cardano-cli fixture; (3) `opcert_validate` covers all five `OpCertError` variants; (4) header path's opcert encode goes through `write_opcert_fields_into`, not an inline body; (5) no `*_opcert` definitions in `ade_codec::shelley::block.rs` (the inline path was removed by S2); (6) `OpCertError` is a closed enum with no `#[non_exhaustive]`. |
| `ci/ci_check_forge_purity.sh` | **New** (`8312690`, S3) — the **35th** script | Enforces `DC-CONS-13/14/15` + `DC-LEDGER-12` via 5+ mechanical guards: (1) no I/O / clock / rand / HashMap iteration / floating-point / println / async in `producer/forge.rs`, `producer/state.rs`, or `shelley/tx_components.rs`; (2) `forge_block` calls `is_leader_for_vrf_output` (not a producer-side fork); (3) `forge_block` calls `mempool::admit::admit` over the snapshot's accumulating order — no out-of-order or fabricated txs; (4) `ProducerTick` is a closed canonical struct, no `Option<...>` flag-fields, no `#[non_exhaustive]`; (5) `ForgeError` is a closed taxonomy. |
| `ci/ci_check_no_private_keys_in_corpus.sh` | **New** (`8312690`, S3) — the **36th** script | Enforces `DC-CONS-14` + `DC-CRYPTO-03` via 3+ mechanical guards: (1) no `*.skey` / `*.sk` / `*.signingkey` files under producer fixtures; (2) no private-key type names (`VrfSigningKey`, `KesSecret`, `KesSigningKey`, `ColdSigningKey`) in fixture sources; (3) replay corpora are in-code synthetic only — no committed binary blobs. Closure of the OQ-12 discipline (replay never invokes RED signing; corpora never carry private-key material). |
| `ci/ci_check_no_producer_body_encoder.sh` | **New** (`4fd714c`, S4) — the **37th** script | Enforces `DC-CONS-16` via 3+ mechanical guards: (1) EXACTLY two `pub fn block_body_hash{,_from_buckets}` definitions in BLUE crates, both in `crates/ade_ledger/src/block_body_hash.rs`; (2) `forge_block` and `block_validity::header_input::computed_body_hash` both import from `block_body_hash` (single canonical authority); (3) no parallel body-hash recipes anywhere in BLUE. |
| `ci/ci_check_self_accept_gate.sh` | **New** (`aa7a7dd`, S5) — the **38th** script | Enforces `CN-CONS-07` via 3+ mechanical guards: (1) `AcceptedBlock {` struct-literal matches ONLY in `self_accept.rs` (no public constructor outside `self_accept`); (2) EXACTLY one `pub fn .* -> AcceptedBlock` definition across crates; (3) `Broadcast::send`'s argument type is `AcceptedBlock`, not raw bytes — type-level broadcast gate. |
| `ci/ci_check_scheduler_closure.sh` | **New** (`58678af`, S6) — the **39th** script | Enforces `OP-OPS-05` via 5+ mechanical guards: (1) `scheduler_step` + `assemble_tick` are I/O-pure (grep for `SystemTime`/`fs`/`env`/`println`/`async`); (2) broadcast accepts only `AcceptedBlock`; (3) scheduler halts deterministically on self-accept failure (`SchedulerHaltReason::SelfAcceptFailed` referenced); (4) no cycles in dep graph (RED → BLUE only); (5) full pipeline integration test exists under `tests/producer_pipeline_slot_deadline.rs`. |
| `ci/ci_check_producer_corpus_present.sh` | **New** (`694dd74`, S7) — the **40th** script | Enforces `CN-CONS-06` (mechanical half) via 5+ mechanical guards: (1) `producer_replay_fixtures()` is wired and exposes the three S3 fixtures (`fixture_empty_mempool_leader`, `fixture_non_leader`, `fixture_two_tx_mempool_leader`); (2) `expected_forged` outputs are non-empty for leader cases (and empty for the non-leader case); (3) the three cross-impl adapter test names are present in `cross_impl_adapter.rs`; (4) the live binary `live_block_production_session` is present + the procedure doc `CE-N-C-8_PROCEDURE.md` exists; (5) `CN-CONS-06.code_locus` references the cross-impl adapter. |

TRACEABILITY cross-reference: every script listed above appears as a
`ci_script` for at least one rule in `docs/ade-invariant-registry.toml`,
re-traced via `ci/ci_check_constitution_coverage.sh`. **PHASE4-N-C
added 8 new `ci_script ↔ rule` edges** (one per new script;
see §7). The constitution-coverage gate continues to PASS at HEAD
(`aa7a7dd` extended the coverage script's recognized-type list to
include the new closed types under `producer/`).

---

## 6. Canonical Type Registry Delta

n/a — `.idd-config.json` `canonical_type_registry` is null. Canonical-type
rules live inline in the invariant registry under family `T`.

**PHASE4-N-C introduced 22 new closed types** in support of the
producer authority surface: `ProducerTick`, `ForgedBlock`,
`ForgeError`, `ForgeEffects`, `AcceptedBlock`, `SelfAcceptError`,
`OpCertError`, `OpCertCodecError`, `KeyLoadError`, `SigningError`,
`KesSignature`, `SchedulerInput`, `SchedulerEffect`, `SchedulerState`,
`SchedulerHaltReason`, `BroadcastError`, `BroadcastQueue`,
`TickInputs`, `TickAssemblyError`, `VrfSigningKey`, `KesSecret`,
`ColdSigningKey`. Of those, `AcceptedBlock` is the load-bearing
type-level broadcast gate; `ProducerTick` is the canonical
input boundary; the rest are closed error taxonomies + capability
tokens. Exact whole-project recount belongs to the TRACEABILITY
regen that follows.

**Removals: 0** (expected under append-only discipline).

---

## 7. Normative Rule Delta

The project's invariant registry tracks structured rules (TOML), not
prose normative-doc rules; this section reports on it.

- Rules at baseline (`d509f02:constitution_registry.toml`): **147**
- Rules at prior refresh (`96d043c:docs/ade-invariant-registry.toml`): **176**
- Rules at HEAD (`694dd74:docs/ade-invariant-registry.toml`): **190**
- Net additions vs baseline: **+43** (PHASE4-N-A: 2; PHASE4-N-B: 8;
  PHASE4-B1: 6; PHASE4-B2: 5; PHASE4-B3: 2; PHASE4-B3F: 0; PHASE4-B4: 1
  (`DC-LEDGER-08`); PHASE4-B5: 1 (`DC-LEDGER-09`); OQ5: 1
  (`DC-LEDGER-10`); COMMITTEE-CRED-FIDELITY / DREP-VOTE-FIDELITY /
  ENACTMENT-COMMITTEE-FIDELITY / ENACTMENT-COMMITTEE-WRITEBACK /
  post-3d94c22 testkit thread: 0 each; PHASE4-N-E S1–S5: 2
  (`DC-MEM-03`, `DC-MEM-04`); PHASE4-N-E S6: 0;
  PROPOSAL-PROCEDURES-DECODE: 1 (`DC-LEDGER-11`); **PHASE4-N-C: 14**
  — `DC-CRYPTO-03/04/05`, `DC-CONS-11/12/13/14/15/16`, `DC-LEDGER-12`,
  `CN-CONS-06/07`, `OP-OPS-04/05`, all introduced at `declared` in
  S1 (`ea9770e`) and all flipped to `enforced` over S1 → S7).
- Net additions vs prior refresh: **+14** — the full N-C 14-rule
  family.
- Removals: **0** (expected under append-only discipline; clean).

- **Strengthenings recorded by PHASE4-N-C:**
  - **`T-DET-01`** (cluster cross-references it via every new
    `DC-CONS-*` rule; the producer authority surface joins the list
    of byte-deterministic transformations: canonical `ProducerTick`
    → forged block bytes).
  - **`T-ENC-01`** (cluster cross-references via `DC-CONS-16`; the
    producer's `header.body_hash` computation joins the hash-critical
    byte paths through the single canonical `block_body_hash` authority).
  - Both strengthenings are recorded as cross-references on the new
    `DC-CONS-*` rules rather than as `strengthened_in` entries on
    `T-DET-01` / `T-ENC-01` — consider normalizing on the next
    registry curation pass.
- **Strengthenings carried forward unchanged**: `DC-MEM-01`
  (PHASE4-N-E S2/S3); `DC-MEM-02` (B2); `DC-EPOCH-01` (WRITEBACK +
  post-3d94c22 oracle); `DC-LEDGER-10` (OQ5 → COMMITTEE-CRED →
  DREP-VOTE → ENACTMENT-COMMITTEE-FIDELITY → WRITEBACK →
  post-3d94c22 oracle → PPD cross_ref; 20 tests at HEAD);
  `DC-LEDGER-08` (B5, via `cross_ref`); `T-DET-01` / `T-ENC-03`
  (OQ5); `DC-TXV-06` (B3F: `partial` → `enforced`); `DC-VAL-06`
  (B3F + B4); `T-CONSERV-01` / `CN-LEDGER-07` (B3); `DC-MEM-01,02`
  (B2, `declared` → `enforced`); `DC-EPOCH-02` (CE-73
  reclassification); the N-D bundle; the N-A real-capture bundle;
  `T-CORE-02` (S-B1).

- **Open obligations recorded by PHASE4-N-C:**
  - **`OP-OPS-04.open_obligation`** — cardano-crypto 1.0.8 exposes
    `gen_key_kes_from_seed_bytes` but no `raw_deserialize_signing_key_kes`
    for Sum6Kes; the keys loader currently round-trips synthetic
    seed-encoded skeys but does not yet load real cardano-cli's
    612-byte expanded-tree Sum6KES serialization. Resolution
    candidates: upstream a Sum6Kes deserializer, fork the parsing
    locally, or document the cardano-cli kes-skey conversion step in
    the operator workflow. CE-N-C-8 operator-action evidence must
    exercise whichever path closes. Rule remains `enforced` — the
    obligation is for a follow-on artifact, not a closure regression.
  - **`CN-CONS-06.open_obligation`** — CE-N-C-8 live half is
    `blocked_until_operator_stake_available`. The mechanical half
    (structural cross-impl agreement: decode round-trip + S4
    body-hash binding + decoder/encoder field agreement) is fully
    enforced via the cross_impl_adapter tests +
    `ci_check_producer_corpus_present.sh`, satisfying the
    bytes-shape claim against the synthetic S3 corpus. The
    crypto-level cross-impl claim (real KES/VRF signatures
    accepted by cardano-node over N2N) is by-design out of reach of
    any CI gate against this corpus: synthetic fixtures carry
    all-zero KES/VRF artifacts so the BLUE forge can be replayed
    without invoking RED signing primitives. The crypto-level claim
    therefore lives in CE-N-C-8's operator-action live evidence:
    testnet (preview/preprod) SPO stake must be provisioned and
    `live_block_production_session` run against a private
    cardano-node to capture
    `docs/clusters/PHASE4-N-C/CE-N-C-LIVE_<date>.log`. Reopen
    criteria: when stake is registered, run the binary per
    `CE-N-C-8_PROCEDURE.md` and append the captured log path to
    this entry's `evidence_notes`. Follows the OP-OPS-04 precedent.

Family counts at HEAD: registry total **190** (= 176 + 14 from the
N-C family). The `DC` family grew by 10 (3 CRYPTO + 6 CONS + 1
LEDGER); the `CN` family entries grew by 2 (CONS-06/07); the `OP`
family grew by 2 (OPS-04/05). Per the constitution coverage gate,
`ci_check_constitution_coverage.sh` PASSES at HEAD with the 14 new
rules' `ci_script` and `tests` arrays populated; rule status at HEAD
breaks down to **65 enforced, 15 partial, 110 declared** across the
190-entry registry.

Normative-doc rule extraction (the `normative_docs` list in
`.idd-config.json`) is approximate and not regenerated here — the
structured registry is the authoritative source.

---

## Anomalies and Cross-Reference Warnings

- **PHASE4-N-C cluster mechanically closed; CE-N-C-8 live half is a
  registry `open_obligation`, not a regression.** All 7 implementing
  slices (S1 → S7) land their CE — every CE-N-C-1..7 is mechanically
  enforced by a named CI script + named tests. CE-N-C-8 follows the
  cluster doc's explicit conditional-closure pattern (case b):
  `CN-CONS-06.open_obligation` records `blocked_until_operator_stake_available`
  + names the specific blocker (testnet SPO registration unavailable
  at HEAD `694dd74`) + records the re-open criteria in
  `CE-N-C-8_PROCEDURE.md`. This is the documented conditional pattern
  established by N-E's `CE-NODE-N2C-LTX` and N-B's CE-N-B-6 evidence
  surface — not a discipline gap.
- **OP-OPS-04 carries an open obligation on a real cardano-cli skey
  loader, but is `enforced`.** Structurally the same pattern as
  `CN-CONS-06`: the rule's mechanical claim (RED-only custody) is
  fully gated by `ci_check_private_key_custody.sh`; the open
  obligation is for a follow-on parsing artifact (Sum6Kes
  raw_deserialize for 612-byte expanded-tree skeys). The synthetic
  seed-encoded round-trip path is fully implemented and tested;
  real-cardano-cli loading is the upstream-fork-or-document call.
- **CODEMAP / SEAMS / TRACEABILITY are stale at this HEAD — expected
  drift between cluster close and grounding ripple.** This regen
  refreshes HEAD_DELTAS only. Prior CODEMAP (`96d043c`) does NOT
  contain any of the N-C new modules; prior SEAMS does NOT contain
  the `AcceptedBlock` type-level broadcast gate, `ProducerTick`
  canonical input, `forge_block` chokepoint, `block_body_hash` single
  authority, or the new RED → BLUE `ade_runtime -> ade_ledger` edge;
  prior TRACEABILITY does NOT contain the 14 new rules + 8 new
  `ci_script ↔ rule` edges. The grounding ripple immediately
  following this HEAD_DELTAS regen will bring all four docs to
  self-consistency.
- **One new RED → BLUE Cargo dep edge in PHASE4-N-C S6.**
  `ade_runtime -> ade_ledger` is direct (was transitive); required so
  RED scheduler / broadcast can consume the `AcceptedBlock` token
  type from `ade_ledger::producer`. RED → BLUE direction is allowed
  by `ci_check_dependency_boundary.sh`. No BLUE → RED edge was
  introduced; the cluster's CI gates explicitly forbid the reverse
  direction (`ci_check_no_signing_in_blue.sh`,
  `ci_check_private_key_custody.sh`).
- **Producer corpus is in-code synthetic only (per OQ-12 / NC-FORGE-2
  / DC-CONS-14).** `ade_testkit::producer::fixtures` ships 3
  canonical synthetic `ProducerTick` fixtures with captured
  `expected_forged: Vec<Vec<u8>>` derived once by the regen binary
  `regen_producer_fixtures.rs`. Replay drives BLUE only; corpora
  carry no private-key material — `ci_check_no_private_keys_in_corpus.sh`
  enforces this. The cross-impl adapter's "all-zero KES/VRF
  artifacts" choice is the load-bearing design decision separating
  the mechanical and crypto-level cross-impl claims (the latter is
  CE-N-C-8 operator-action evidence, not CI).
- **PHASE4-N-C cluster directory is NOT yet archived.**
  `docs/clusters/PHASE4-N-C/` (8 files: `cluster.md` + N-C-S1.md
  through N-C-S7.md + `CE-N-C-8_PROCEDURE.md`) remains at the active
  cluster location. Archival to `docs/clusters/completed/PHASE4-N-C/`
  is deferred to a separate `/cluster-close` commit. Active
  `docs/clusters/` at HEAD now contains `completed/`, `PHASE4-N-B/`
  (carried-forward stray log directory), and **`PHASE4-N-C/`** (the
  newly closed but unarchived cluster).
- **Pre-existing `boundary_fingerprint_matches_pins` failure on
  `byron_pre_hfc` predates this cluster** (verified during N-C-S2
  acceptance per the commit message). Out-of-scope for PHASE4-N-C;
  not introduced by any N-C slice. Tracked under a separate future
  cluster.
- **`strengthened_in` records the introducing cluster on
  freshly-created rules.** Each of the 14 new N-C rules records
  `introduced_in = "PHASE4-N-C"` and `strengthened_in = []` (no
  strengthenings yet). Harmless.
- **PROPOSAL-PROCEDURES-DECODE / PHASE4-N-E / WRITEBACK / FIDELITY /
  OQ5 / B-cluster anomalies (carried forward unchanged).** All
  cluster-specific anomalies from prior threads are unchanged at this
  HEAD. See prior HEAD_DELTAS snapshot at `/tmp/head-deltas-pre-N-C.md`
  for the verbatim record.
- **DC-VAL status mismatch vs. closure claim (B1, carried forward).**
  Only `DC-VAL-01` is `enforced`; `DC-VAL-02` → `DC-VAL-05` remain
  `declared` despite named tests and the extended closed-enums
  enforcement point. Flip on the next `/traceability` pass.
- **All commit subjects in this regen carry a conventional-commits
  prefix.** The 9 PHASE4-N-C commits are `feat(producer)` ×5,
  `refactor(ledger)` ×1, `feat(consensus)` ×1, `docs(registry)` ×1,
  `chore(lock)` ×1. **All 9 commits in the `96d043c..694dd74` span
  carry the repo-required `Co-Authored-By: Claude Opus 4.7 (1M context)`
  model-attribution trailer** (per the CLAUDE.md project override for
  the bounty trailer ratio). The project hook
  `ci/git-hooks/commit-msg` is active in this clone and enforces the
  trailer mechanically.
- **Cluster docs archived as of this HEAD.** Sixteen cluster
  directories archived under `docs/clusters/completed/`:
  COMMITTEE-CRED-FIDELITY, DREP-VOTE-FIDELITY,
  ENACTMENT-COMMITTEE-FIDELITY, ENACTMENT-COMMITTEE-WRITEBACK,
  OQ5-CREDENTIAL-FIDELITY, PHASE4-B1, PHASE4-B2, PHASE4-B3,
  PHASE4-B3F, PHASE4-B4, PHASE4-B5, PHASE4-N-A, PHASE4-N-B,
  PHASE4-N-D, PHASE4-N-E, PROPOSAL-PROCEDURES-DECODE. **PHASE4-N-C is
  closed mechanically but its cluster directory is not yet archived.**
- **B5 / B4 / B3F / B3 / B2 / B1 / N-D / N-B / N-A / PHASE4-N-E /
  PROPOSAL-PROCEDURES-DECODE closures — carried forward unchanged.**
- **B3 positive corpus carves out Plutus per CE-88 (carried
  forward).**
- **Adversarial corpora are derived, not committed (carried forward).**
  N-E reuses the B2 B-track corpus verbatim; PPD PP-S2 ships its
  corpus in code; **PHASE4-N-C ships its corpus in code (in-code
  synthetic)** under `ade_testkit::producer::fixtures`. The
  PHASE4-N-C corpus pattern goes one step further than PPD PP-S2 by
  using only `Vec<u8>` literals (zero on-disk artifacts) — required
  by the no-private-keys-in-corpus discipline.
- **Corpus relayout: credentialed snapshots removed, then regenerated
  off-repo (carried forward).** `corpus/snapshots/` `.gitignore`-d;
  canonical home `s3://ade-corpus-snapshots`.
- **No removed canonical types** (n/a — no separate registry;
  canonical types at HEAD grew by 22 from the N-C cluster + 1 from
  PPD PP-S1 since the prior baseline-snapshot count).
- **No removed registry rules** (expected: 0; actual: 0).
  **PHASE4-N-C added 14 new rules; PROPOSAL-PROCEDURES-DECODE PP-S1
  added 1.** Registry total: **190** at HEAD (was 176 at prior
  refresh).

---

## Generation Notes

Regenerate via `/head-deltas <baseline>` or by re-running the
`head-deltas-generator` agent with the same baseline. Baseline lives
in `.idd-config.json` `head_deltas_baseline` (still `d509f02` —
**this is a cluster-close grounding refresh, not a phase boundary,
so the baseline is unchanged**). Update the baseline on the next
phase boundary (Phase 4 close, which PHASE4-N-C brings within reach:
the bounty's Tier-1 validation→producer leap is mechanically closed
at this HEAD; PHASE4-N-F and live operator evidence are the
remaining Phase-4 closure work). Note the commit-hash rewrite caveat
at the top — re-derive hashes from `git log` at each regen rather
than carrying them forward. This regen is cut at working HEAD
`694dd74` (PHASE4-N-C S7). The prior regen narrated HEAD `928c2be`
(PROPOSAL-PROCEDURES-DECODE PP-S2); the new span is `96d043c..694dd74`
— 9 commits: `ea9770e` (S1 RED signing + keys), `9727bd9` (S1
follow-up OP-OPS-04 obligation), `4cf4b65` (S2 BLUE opcert validator
+ codec), `8312690` (S3 BLUE forge + ProducerTick + tx-admissibility),
`4fd714c` (S4 single canonical body-hash authority), `aa7a7dd` (S5
BLUE self_accept + AcceptedBlock broadcast gate), `58678af` (S6 RED
scheduler + GREEN tick-assembler + RED broadcast), `52b77c5` (S6
Cargo.lock follow-up for the new `ade_runtime -> ade_ledger` edge),
`694dd74` (S7 mechanical cross-impl adapter +
live_block_production_session binary).
