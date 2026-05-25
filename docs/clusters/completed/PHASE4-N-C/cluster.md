# Cluster PHASE4-N-C — Block Production Closure (Tier 1 producer half)

> **Status:** Planning artifact (non-normative). Strengthens `T-DET-01`
> and `T-ENC-01` (no new constitutional rule). Introduces
> `DC-CRYPTO-03/04/05`, `DC-CONS-11/12/13/14/15/16`, `DC-LEDGER-12`,
> `CN-CONS-06/07`, `OP-OPS-04/05` (all `status="declared"` at cluster
> entry; flip to `enforced` as the named slices land). Produced from
> `docs/planning/phase4-n-c-invariants.md` and
> `docs/planning/phase4-n-c-cluster-slice-plan.md`. If this doc
> conflicts with the registry / specs, those win.

---

## Primary invariant

> A forged block is byte-deterministic over a canonical `ProducerTick`
> (BLUE), passes Ade's own N-B header validator and B1 body validator
> before broadcast (self-acceptance), and is accepted by cardano-node
> when delivered via N2N (cross-impl). Private-key custody stays in
> RED; BLUE consumes signed artifacts (`VrfProof`, `KesSignature`,
> `OpCert`) only — replay never invokes RED signing and replay corpora
> never carry private keys.

## Normative anchors

- `docs/ade-invariant-registry.toml` — `T-DET-01` (strengthened in
  N-C), `T-ENC-01` (strengthened in N-C), `DC-CRYPTO-03/04/05`,
  `DC-CONS-11/12/13/14/15/16`, `DC-LEDGER-12`, `CN-CONS-06/07`,
  `OP-OPS-04/05`.
- Project constitution §2 (`T-DET-01`, `T-ENC-01`, Byte Authority
  Model; Functional Core / Imperative Shell).
- IDD `~/.claude/methodology/idd.md` Part I §§4 (determinism),
  §5 (replay), §9 (FC/IS partition), §§6–8 (closed surfaces +
  versioning + fail-fast).
- IETF `draft-irtf-cfrg-vrf-03` (Praos VRF wire format).
- Cardano `Sum6KES` specification (depth-6 sum composition over
  ed25519).
- `docs/planning/phase4-n-c-invariants.md` §§1–10 (sketch with OQ
  resolutions).

## OQ resolutions (locked — see invariants sketch §7)

- **OQ-1** Ade consumes operator-supplied cardano-cli-format keys /
  opcerts. No key generation.
- **OQ-2** KES algorithm: `Sum6KES` only.
- **OQ-3** VRF algorithm: Praos VRF (compatible with the validator's
  verify path).
- **OQ-4** Conway / Praos producer only. TPraos producer is explicit
  non-goal.
- **OQ-5** OpCert anchor is RED/operator-config supplied as a pure
  input on `ProducerTick`; BLUE never infers from clock / FS.
- **OQ-6** Ade does not own opcert renewal; cardano-cli remains the
  minting tool. Ade round-trips and validates.
- **OQ-7** Empty mempool → empty block when leader. Forbidden
  transition is non-leader forge, not empty-body forge.
- **OQ-8** Nonce contribution is an explicit proof obligation
  (fixture under S3/S4).
- **OQ-9** Slot-deadline performance is operational (`OP-OPS-05`),
  not a hash-critical invariant.
- **OQ-10** N-C includes forge → N2N delivery → cardano-node
  acceptance evidence.
- **OQ-11** Mechanical CI adapter + operator-action live evidence;
  live half marked `blocked_until_operator_stake_available` if
  testnet SPO stake is not provisioned at cluster close.
- **OQ-12** Sign-side primitives are greenfield; zero validator
  regressions allowed.

## Grounding (verified at HEAD `96d043c`)

- **`cardano-crypto = "1.0.8"`** already in
  `crates/ade_crypto/Cargo.toml` with features
  `["vrf-draft03", "kes-sum", "dsign"]`. The crate exposes:
  - `vrf::VrfAlgorithm::prove(sk, message)`
    (`~/.cargo/registry/.../cardano-crypto-1.0.8/src/vrf/mod.rs:122`)
  - `kes::KesAlgorithm::sign_kes(...)` (`src/kes/mod.rs:161`)
  - `kes::KesAlgorithm::update_kes(...)` (`src/kes/mod.rs:178`)
  - `key::text_envelope` parser for cardano-cli `*.skey` files.
- **Validator entry points** N-C's `self_accept` will wrap:
  - `ade_core::consensus::header_validate::validate_and_apply_header`
    (`crates/ade_core/src/consensus/header_validate.rs:71`)
  - `ade_ledger::block_validity::transition::block_validity`
    (`crates/ade_ledger/src/block_validity/transition.rs:43`)
- **Mempool snapshot surface** N-C's `forge_block` will consume:
  - `ade_ledger::mempool::admit::admit`
    (`crates/ade_ledger/src/mempool/admit.rs:78`)
  - `MempoolState` exposes `accepted()` (ordered `&[Hash32]`) and
    `accumulating()` (`&LedgerState`).
- **Header/opcert codec** N-C's body-hash parity slice (S4) and
  opcert-validate slice (S2) build on:
  - `ade_codec::shelley::block` (header encode/decode with
    `body_hash` + `operational_cert` + `kes_period`)
  - `ade_crypto::kes::OperationalCertData` + `verify_opcert`
    (`crates/ade_crypto/src/kes.rs:124,148`)
- **Live-evidence binary pattern** N-C's S7 will follow:
  - `crates/ade_core_interop/src/bin/live_consensus_session.rs`
    (CE-N-B-6 precedent)
  - `crates/ade_core_interop/src/bin/live_tx_submission_session.rs`
    (CE-N-E-6 precedent)
- **No `producer` module exists anywhere** in the workspace at HEAD —
  N-C is greenfield for the producer scaffolding.
- **Existing trailer ratio**: 236/291 = 81.10% at HEAD `96d043c`.
  Project hook `ci/git-hooks/commit-msg` enforces the
  `Co-Authored-By: Claude Opus 4.7 (1M context)` trailer; activated
  in this clone via `git config core.hooksPath ci/git-hooks` (already
  active).

## Entry Conditions

- **PHASE4-N-A closed** — N2N protocol codecs (chain-sync,
  block-fetch) + handshake exist and are version-pinned.
- **PHASE4-N-B closed** — Praos consensus runtime (fork choice,
  rollback, leader schedule, nonce evolution, op-cert counter
  monotonicity at the validator side, header validation).
  `validate_and_apply_header` is the validator chokepoint S5 reuses.
- **PHASE4-N-D closed** — Chain DB exists; `LedgerState` is
  reachable.
- **PHASE4-N-E closed** — Mempool wire-level ingress closed.
  `MempoolState::accumulating()` + `accepted()` are the canonical
  snapshot surfaces S3 consumes.
- **PHASE4-B1..B5 closed** — `tx_validity` no-false-accept proven;
  `block_validity` exists. S5 reuses these as the body-validator
  chokepoint.
- **PHASE4-B3F closed** — Conway-576 corpus + adversarial corpus
  exist; S3/S4 mechanical adapter (S7) reuses both as oracles.
- **`PROPOSAL-PROCEDURES-DECODE` closed** — Closed Conway tx-body
  shape is final; no opacity in the bodies S3 assembles.
- **Constitution-coverage gate PASSES** at HEAD:
  `bash ci/ci_check_constitution_coverage.sh`.

## Exit Criteria (CI-Verifiable)

Each CE names the test or check that proves it. Every named test /
script is introduced by the slice that owns the CE.

- **CE-N-C-1 (signing transcript)** — RED signing primitives match
  cardano-node reference vectors and round-trip with existing verify
  paths. Named tests:
  - `vrf_prove_matches_reference_vectors` (S1)
  - `kes_sign_matches_reference_vectors` (S1)
  - `kes_update_chain_matches_reference` (S1)
  - `vrf_prove_then_verify_round_trip` (S1)
  - `kes_sign_then_verify_round_trip` (S1)
  - `kes_sign_rejects_period_past_evolutions_remaining` (S1)
  - `kes_update_rejects_backwards_evolution` (S1)
  - `cardano_cli_skey_envelope_round_trips_through_keys_loader` (S1)
  - CI: `ci/ci_check_private_key_custody.sh` (S1) — forbids any
    `*SigningKey` / `KesSecret` / cold-key type from appearing in
    `ade_core` / `ade_codec` / `ade_types` / `ade_ledger` / `ade_crypto`
    public APIs.
  - Registry flip on close: `DC-CRYPTO-03/04/05`, `OP-OPS-04` →
    `enforced`.

- **CE-N-C-2 (opcert validate)** — BLUE `opcert_validate` rejects
  every forbidden state; encoder produces cardano-cli-byte-identical
  bytes. Named tests:
  - `opcert_validate_accepts_canonical_fixture` (S2)
  - `opcert_validate_rejects_counter_regression` (S2)
  - `opcert_validate_rejects_counter_repeat` (S2)
  - `opcert_validate_rejects_period_mismatch` (S2)
  - `opcert_validate_rejects_bad_signature_over_cold_key` (S2)
  - `opcert_encoder_matches_cardano_cli_byte_identical` (S2)
  - CI: `ci/ci_check_opcert_closed.sh` (S2) — forbids parallel opcert
    encoders / decoders outside `ade_codec::shelley::block` +
    `ade_core::consensus::opcert_validate`.
  - Registry flip on close: `DC-CONS-11/12` → `enforced`.

- **CE-N-C-3 (forge core)** — BLUE `forge_block` is pure,
  replay-equivalent, leader-gated, and tx-admissibility-gated.
  Named tests:
  - `forge_block_pure_no_io` (S3, type-level + grep gate)
  - `forge_block_replay_byte_identical` (S3, replay corpus over
    `ProducerTick` stream)
  - `forge_block_rejects_non_leader_tick` (S3)
  - `forge_block_rejects_tx_not_in_mempool_accepted_prefix` (S3)
  - `forge_block_rejects_tx_permuted_from_accumulating_order` (S3)
  - `forge_block_empty_mempool_produces_empty_body` (S3, OQ-7)
  - `forge_block_uses_validator_leader_check_function` (S3, asserts
    the producer calls `consensus::leader_schedule::is_leader_for_vrf_output`
    or equivalent — no producer-side fork)
  - CI: `ci/ci_check_forge_purity.sh` (S3) — forbids
    `std::time::SystemTime`, `rand`, `HashMap` iteration,
    `std::env`, `std::fs` in `ade_core::consensus::forge`.
  - Registry flip on close: `DC-CONS-13/14/15`, `DC-LEDGER-12` →
    `enforced`.

- **CE-N-C-4 (body-hash parity)** — Producer and validator hash the
  same bytes through the same encoder. Named tests:
  - `forged_body_hash_matches_validator_recomputation` (S4) — for
    every fixture in the replay corpus, the validator's body_hash
    recomputation over the forged block's body bytes equals the
    producer-emitted `header.body_hash`.
  - `body_encoder_is_single_authority` (S4) — `encode_block_body` has
    exactly one definition reachable from both `forge` and
    `header_validate` recomputation paths.
  - CI: `ci/ci_check_no_producer_body_encoder.sh` (S4) — grep gate
    forbidding any new `pub fn .*encode_block_body` outside the
    canonical authority.
  - Registry flip on close: `DC-CONS-16` → `enforced`.

- **CE-N-C-5 (self-acceptance)** — Forged bytes pass Ade's own
  validator stack before RED can consume them. Named tests:
  - `self_accept_accepts_freshly_forged_block` (S5) — golden fixtures.
  - `self_accept_rejects_corrupted_body_hash` (S5) — adversarial.
  - `self_accept_rejects_invalid_kes_signature` (S5) — adversarial.
  - `self_accept_rejects_unbalanced_tx_in_body` (S5) — adversarial.
  - `broadcast_callable_only_with_accept_verdict` (S5, type-level —
    `Broadcast::send(&self, AcceptedBlock)` consumes a token type
    that only `self_accept` produces).
  - CI: `ci/ci_check_self_accept_gate.sh` (S5) — forbids any
    construction of the accepted-block token outside `self_accept`.
  - Registry flip on close: `CN-CONS-07` → `enforced`.

- **CE-N-C-6 (scheduler + broadcast)** — Full RED→GREEN→BLUE→BLUE→RED
  path completes within the slot deadline. Named tests:
  - `producer_scheduler_silent_on_non_leader_slots` (S6)
  - `producer_scheduler_self_accept_failure_halts_clean` (S6)
  - `producer_full_path_under_slot_deadline_on_reference_fixture` (S6)
    — measured on a recorded `ProducerTick` corpus replayed through
    the full pipeline; a wall-clock guard test, not a hash-critical
    invariant.
  - `tick_assembler_deterministic_over_captured_red_outputs` (S6)
  - Registry flip on close: `OP-OPS-05` → `enforced`.

- **CE-N-C-7 (mechanical cross-impl adapter)** — CI test exercises
  the full Ade producer path over a captured corpus and verifies
  forged bytes are accepted by Ade's own decoder + header validator.
  Named tests / artefacts:
  - `cross_impl_adapter_forged_block_decodes_through_ade_codec` (S7)
  - `cross_impl_adapter_forged_block_passes_header_validate` (S7)
  - `cross_impl_adapter_corpus_round_trips_byte_identical` (S7)
  - CI: `ci/ci_check_producer_corpus_present.sh` (S7) — guards corpus
    presence + non-empty `expected_forged.cbor` outputs.
  - Registry flip on close: `CN-CONS-06` mechanical half →
    `enforced`.

- **CE-N-C-8 (operator-action live evidence)** — Conditional. Either:
  - (a) `docs/clusters/PHASE4-N-C/CE-N-C-LIVE_<date>.log` captures
    cardano-node accepting at least one Ade-forged block via N2N,
    AND `docs/clusters/PHASE4-N-C/CE-N-C-8_PROCEDURE.md` documents
    how the session was run; OR
  - (b) `CE-N-C-8_PROCEDURE.md` documents the
    `blocked_until_operator_stake_available` status, names the
    specific blocker (testnet SPO registration unavailable at HEAD
    `<hash>`), and records the re-open obligation as a registry
    `open_obligation` on `CN-CONS-06`.
  - Registry flip on close: `CN-CONS-06` live half →
    `enforced` (case a) or `partial` with `open_obligation` (case b).
  - This is the explicit conditional closure pattern, not deferral.

## Slice index

Slices land in this order; each slice independently leaves the
system in a fully correct state. See per-slice docs once `/slice-doc`
runs.

| Slice | One-line scope | TCB |
|----|----|----|
| **N-C-S1** | RED signing primitives (`vrf_prove`, `kes_sign`, `kes_update`) + cardano-cli `*.skey` loader; private-key custody RED-confined. | RED + GREEN test harness |
| **N-C-S2** | BLUE `opcert_validate` + counter monotonicity + opcert encoder closed-grammar parity with cardano-cli. | BLUE |
| **N-C-S3** | BLUE `forge_block` + `ProducerTick` canonical type + leader-check gate + tx-admissibility prefix + purity. | BLUE |
| **N-C-S4** | BLUE body-hash parity via single Cardano-compatible canonical body encoder shared with validator hash path. | BLUE |
| **N-C-S5** | BLUE `self_accept` bridge wrapping N-B header validator + B1 body validator; type-level gate on RED `broadcast`. | BLUE |
| **N-C-S6** | RED `scheduler` + RED `broadcast` + GREEN `tick_assembler`; deterministic slot loop with self-accept halt and slot-deadline measurement. | RED + GREEN |
| **N-C-S7** | Mechanical cross-impl CI adapter + operator-action `live_block_production_session` binary; CE-N-C-8 conditional. | RED + test-harness |

## TCB Color Map (FC/IS Partition)

For every module touched / created by this cluster:

**BLUE (deterministic, authoritative):**
- `ade_ledger::producer::forge` (new — S3, S4)
  *(originally planned at `ade_core::consensus::forge`; relocated to
  `ade_ledger::producer` because `ade_ledger` already depends on
  `ade_core` and the forge body needs `ade_ledger::{state::LedgerState,
  mempool::admit::*}`. BLUE classification unchanged. Correction
  recorded in `docs/clusters/PHASE4-N-C/N-C-S3.md` §Host-crate decision.)*
- `ade_core::consensus::opcert_validate` (S2 — landed)
- `ade_ledger::producer::self_accept` (new — S5; same host-crate
  correction as forge)
- `ade_ledger::producer::state` (new — S3; carries `ProducerTick`)
- `ade_codec::shelley::block::encode_block_body` (existing, lifted
  to single-authority status — S4)
- `ade_crypto::vrf::verify_*` / `ade_crypto::kes::verify_*` /
  `ade_crypto::kes::verify_opcert` (existing, **unchanged** — OQ-12)
- `ade_core::consensus::leader_schedule::is_leader_for_vrf_output`
  (existing, reused unchanged by S3 — NC-VRF-3 single source of
  leader truth)
- `ade_core::consensus::header_validate::validate_and_apply_header`
  (existing, reused unchanged by S5)
- `ade_ledger::block_validity::transition::block_validity`
  (existing, reused unchanged by S5)
- `ade_ledger::mempool::admit::admit` (existing, reused unchanged by
  S3 — N-C never re-admits, only consumes the snapshot)

**GREEN (deterministic glue, non-authoritative):**
- `ade_runtime::producer::tick_assembler` (new — S6). Composes
  canonical `ProducerTick` from RED scheduler outputs. Must be
  observably deterministic — captured RED outputs → identical
  `ProducerTick` across two replays.

**RED (nondeterministic shell):**
- `ade_runtime::producer::signing` (new — S1). Holds in-memory
  `KesSecret` / `VrfSigningKey` / cold-key bytes; zeroize-on-drop;
  `vrf_prove` / `kes_sign` / `kes_update` only. No reads of
  wall-clock or env.
- `ade_runtime::producer::keys` (new — S1). Disk reads of
  cardano-cli `*.skey` text envelopes; decoding into RED in-memory
  secrets.
- `ade_runtime::producer::scheduler` (new — S6). Slot wakeup loop,
  RED→GREEN→BLUE call sequence, post-self-accept network handoff.
- `ade_runtime::producer::broadcast` (new — S6). Outbound queue
  handing self-accepted bytes to `ade_network`'s N2N block-fetch /
  chain-sync server path. Scope: enough delivery for cardano-node to
  fetch the block. Full relay-mesh behaviour is N-A successor scope.
- `ade_core_interop::bin::live_block_production_session` (new — S7).
  Operator-action evidence binary producing
  `CE-N-C-LIVE_<date>.log`. Conditional on
  `blocked_until_operator_stake_available`.

**Test harness (`ade_testkit`):**
- `ade_testkit::producer::reference_vectors` (new — S1)
- `ade_testkit::producer::replay` (new — S3, S4)
- `ade_testkit::producer::cross_impl_adapter` (new — S7)
- `crates/ade_testkit/fixtures/producer/` (new — corpus root)

**Color rules:**
- No RED behaviour may appear in BLUE code (enforced by
  `ci/ci_check_forge_purity.sh` + `ci/ci_check_private_key_custody.sh`).
- GREEN code must not affect authoritative outputs (enforced by S3's
  `forge_block_replay_byte_identical` over captured ticks — two
  identical replays imply tick assembler isn't injecting
  nondeterminism).
- Color must be resolved before any slice begins (resolved above; no
  open colors).

## Forbidden during this cluster

- Importing `std::time::SystemTime`, `rand`, `std::env`, `std::fs`,
  `tokio::time`, or `std::collections::HashMap` iteration in
  `ade_core::consensus::forge` / `opcert_validate` / `self_accept` /
  `producer_state`.
- Any new `pub fn .*encode_block_body` outside the canonical
  authority in `ade_codec::shelley::block`.
- Any public type carrying a KES / VRF / cold private-key in
  `ade_core::*`, `ade_codec::*`, `ade_types::*`, `ade_ledger::*`, or
  `ade_crypto::*` public APIs.
- Constructing the accepted-block / broadcast-eligible token outside
  `ade_core::consensus::self_accept`.
- `git commit --no-verify` (silently bypasses the project trailer
  hook).
- Slice docs claiming a CE that the slice does not mechanically
  enforce. Every CE flipped to `enforced` must point to a named test
  + green CI script.
- "TODO: support TPraos" or any pre-Conway producer scaffolding
  (OQ-4 lock).
- Producer-side fork of `is_leader` / `check_leader_claim`
  (NC-VRF-3, OQ-12).
- Replay corpus carrying private-key bytes — enforced by
  `ci/ci_check_no_private_keys_in_corpus.sh` (introduced by S3).

## Replay obligations introduced by this cluster

- **New canonical replay corpus**: `crates/ade_testkit/fixtures/producer/`
  contains ordered `Vec<ProducerTick>` plus expected
  `Vec<ForgedBlockBytes>`. Drives `forge_block_replay_byte_identical`
  and `cross_impl_adapter_corpus_round_trips_byte_identical`.
- **T-DET-01 strengthening**: the producer authority surface joins
  the list of byte-deterministic transformations (canonical
  `ProducerTick` → forged block bytes). Registry entry T-DET-01's
  `strengthened_in` array gains `PHASE4-N-C` on cluster close.
- **T-ENC-01 strengthening**: the producer's `header.body_hash`
  computation joins the hash-critical byte paths (validator-shared
  encoder). Registry entry T-ENC-01's `strengthened_in` array gains
  `PHASE4-N-C` on cluster close.
- **No private-key material in corpora.** Replay corpora carry
  signed artifacts (`VrfProof`, `KesSignature`, `OpCert`) captured
  from a one-time RED signing pass; replay drives BLUE only.

## Authority reminder

This document is a planning aid only. All correctness rules live in
the project's normative specifications and the invariant registry.
If there is ever a disagreement:

> **Normative documents + registry + CI enforcement win.**
