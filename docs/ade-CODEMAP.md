# Module Authority Map — Ade

> **Status:** Living architectural document. Regenerated; not hand-edited.
> Per-project instance of `~/.claude/methodology/templates/codemap.md`.

> 9 crates, 182 canonical types, 841 tests, 19 CI checks at HEAD (`78da6c9`).

---

## Conventions

- A **module** in Ade is a Cargo workspace crate (smallest independently-buildable unit).
- Modules are listed by TCB color (BLUE → GREEN → RED), alphabetical within each color.
- TCB color sources, in order of authority:
  1. `.idd-config.json` `core_paths` — substring match (BLUE: `ade_codec`, `ade_types`, `ade_crypto`, `ade_core`, `ade_ledger`, `ade_plutus`).
  2. `.idd-config.json` `_core_paths_doc` — `ade_runtime` is RED; `ade_testkit` and `ade_node` are neither.
  3. `docs/active/phase_2c_progress_report.md` — historical placement of `ade_testkit` as GREEN and `ade_node` as RED.
- Counts:
  - Crates: 9, from `Cargo.toml` `[workspace] members`.
  - Canonical types: 182, from `grep -rE "^pub (struct|enum) "` across the 6 BLUE crates (registry `canonical_type_registry: null`, so a structural count is used). Unchanged since the previous run.
  - Tests: 841 — count of `#[test]` / `#[tokio::test]` attributes across `crates/`. Reported as approximate per the template's fallback rule (test runner not executed). Unchanged since the previous run; the 3 new CI scripts wire existing tests into CI rather than adding new ones.
  - CI checks: 19 — file count under `ci/ci_check_*.sh`. **+3 since the previous run** (`ci_check_chaindb_contract.sh`, `ci_check_recovery_contract.sh`, `ci_check_chaindb_crash_safety.sh`) added in commit `78da6c9` to close the Phase 4 N-D enforcement gap. No `.github/workflows/` yet.

---

## BLUE Modules — Pure Functional Core

> **Shared header (applies to every BLUE crate below).** Every `.rs` source file begins with the contract banner
> `// Core Contract:` and the following deny attributes are present in each `lib.rs`:
> `#![deny(unsafe_code)]`, `#![deny(clippy::unwrap_used)]`, `#![deny(clippy::expect_used)]`,
> `#![deny(clippy::panic)]`, `#![deny(clippy::float_arithmetic)]`.
> CI scripts that enforce the shared rules (all 6 scope the full BLUE set `{ade_codec, ade_types, ade_crypto, ade_core, ade_ledger, ade_plutus}` — drift closed in commit `5b70bee`):
> - `ci/ci_check_module_headers.sh` — banner first-line check.
> - `ci/ci_check_forbidden_patterns.sh` — `HashMap`, `HashSet`, `IndexMap`/`IndexSet`/`indexmap::`, `SystemTime`, `Instant`, `std::fs`, `std::net`, `tokio`, `async fn`, `f32`/`f64`, `anyhow`, `rand::thread_rng`, `thread::spawn`, plus `unsafe` outside a documented allowlist.
> - `ci/ci_check_dependency_boundary.sh` — no BLUE crate depends on a RED crate.
> - `ci/ci_check_no_signing_in_blue.sh` — `SigningKey`/`SecretKey`/`PrivateKey`/`private_key`/`sign_message`/`sign_block` forbidden in BLUE.
> - `ci/ci_check_no_semantic_cfg.sh` — `#[cfg(feature = ...)]` and `cfg!(feature = ...)` forbidden in BLUE.
> - `ci/ci_check_hash_uses_wire_bytes.sh` — no hashing of `canonical_bytes` / re-encoded bytes in BLUE.
> - `ci/ci_check_ingress_chokepoints.sh` — only named `decode_*` chokepoints construct `PreservedCbor`. The named-chokepoint list covers `decode_block_envelope`, the per-era block decoders (`decode_byron_ebb_block`, `decode_byron_regular_block`, `decode_shelley_block`, `decode_allegra_block`, `decode_mary_block`, `decode_alonzo_block`, `decode_babbage_block`, `decode_conway_block`), `decode_address`, and `PlutusScript::from_cbor` in `ade_plutus`. Check 3 explicitly allowlists `ade_plutus/src/evaluator.rs` because Plutus script CBOR is a distinct ingress surface from block CBOR (its decoder goes through aiken/pallas rather than the ade_codec primitives).
> - `ci/ci_check_pallas_quarantine.sh` — `pallas-*` references confined to `ade_plutus`.
>
> A BLUE crate that adds a feature flag, an async function, a `HashMap`, or a RED dep fails CI on push.
> The 3 CI scripts added in commit `78da6c9` (`ci_check_chaindb_contract.sh`, `ci_check_recovery_contract.sh`, `ci_check_chaindb_crash_safety.sh`) are **RED-scoped** — they run `cargo test -p ade_runtime ...` and do not appear in this shared header. They are documented in the cross-module CI matrix at the bottom.

---

### `ade_codec`

| Attribute | Value |
|-----------|-------|
| **Purpose** | Owns Cardano-canonical CBOR ingress: the only place in the workspace that turns raw bytes into typed semantic values, with wire-byte preservation for every hash-bearing structure. |
| **Creates** | `PreservedCbor<T>`, `RawCbor`, `BlockEnvelope`, `ByronDecodedBlock`, `CodecContext`, `CodecError`, `ContainerEncoding`, `IntWidth`, plus era-tagged block/tx wrappers under `byron/`, `shelley/`, `allegra/`, `mary/`, `alonzo/`, `babbage/`, `conway/`. |
| **Interprets** | All canonical Cardano CBOR — block envelopes, era-specific blocks, tx bodies, tx outs, certificates, addresses. Sole authority for `PreservedCbor::new` (constructor is `pub(crate)`). |
| **MUST NOT** | (1) Construct `PreservedCbor` outside `ade_codec` (enforced by `pub(crate)` + `ci_check_ingress_chokepoints.sh`). (2) Re-encode wire bytes when computing hashes (forbidden by `ci_check_hash_uses_wire_bytes.sh`). (3) Use any forbidden BLUE pattern (see shared header). (4) Depend on any other workspace crate except `ade_types`. |
| **Inbound deps** | `ade_ledger`, `ade_plutus`, `ade_testkit` (via Cargo); read-side imports from `ade_runtime` indirect via `ade_testkit`. |
| **Outbound deps** | `ade_types`. No external dependencies; std-only. Dev-deps: `serde_json`, `toml`. |
| **Entry points** | `ade_codec::cbor::envelope::decode_block_envelope` (31 import sites), `ade_codec::cbor` module, `ade_codec::traits::AdeEncode`, `ade_codec::CodecContext`, `ade_codec::byron::decode_byron_block`, `ade_codec::shelley::decode_shelley_block`, `ade_codec::allegra::decode_allegra_block`, `ade_codec::mary::decode_mary_block`, `ade_codec::alonzo::decode_alonzo_block`, `ade_codec::babbage::decode_babbage_block`, `ade_codec::conway::decode_conway_block`, `ade_codec::address::decode_address`. |
| **Key modules** | `cbor/` (envelope + primitive reader/writer), `byron/`, `shelley/`, `allegra/`, `mary/`, `alonzo/`, `babbage/`, `conway/` (era-tagged decoders), `address/`, `preserved.rs` (`PreservedCbor`), `traits.rs` (`AdeEncode`/`AdeDecode`/`CodecContext`), `primitives.rs`, `error.rs`. |

---

### `ade_core`

| Attribute | Value |
|-----------|-------|
| **Purpose** | Currently a placeholder. `docs/active/CE-79_gate_statement.md` explicitly classifies it as "reserved for shared abstractions that did not materialize." The actual functional core lives in `ade_ledger`. |
| **Creates** | None. `src/lib.rs` is the contract banner plus deny attributes; no `pub` items. |
| **Interprets** | None. |
| **MUST NOT** | If/when populated, inherits every shared-header BLUE rule. CE-79 lists `ade_core` non-emptiness as an explicit Tier 4 non-goal — adding semantic content needs a documented cluster gate. |
| **Inbound deps** | None — no crate's `Cargo.toml` lists `ade_core`. |
| **Outbound deps** | None. |
| **Entry points** | None. |

> **Gap surfaced.** `ade_core` is BLUE by configuration but empty. Three options need a human decision: (a) leave as documented placeholder; (b) remove from the workspace; (c) seed it with the cross-era shared abstractions originally intended.

---

### `ade_crypto`

| Attribute | Value |
|-----------|-------|
| **Purpose** | Owns pure cryptographic verification primitives — Blake2b hashing, Ed25519 / Byron-bootstrap signature verification, KES verification with operational certificates, VRF verification. Verification only — signing lives in `ade_runtime`. |
| **Creates** | `Blake2b224`, `Blake2b256`, `HashAlgorithm` trait, `Ed25519VerificationKey`, `Ed25519Signature`, `ByronExtendedVerificationKey`, `KesVerificationKey`, `KesPeriod`, `OperationalCertData`, `VrfVerificationKey`, `VrfProof`, `VrfOutput`, `CryptoError`. |
| **Interprets** | Verification key / signature / proof byte structures. Not a CBOR parser — accepts already-decoded byte slices from `ade_codec` consumers. |
| **MUST NOT** | (1) Implement signing (enforced by `ci_check_no_signing_in_blue.sh` — patterns `SigningKey`/`SecretKey`/`PrivateKey`/`private_key`/`sign_message`/`sign_block`). (2) Allocate global state — verification is `fn(&[u8], &[u8]) -> bool`. (3) Use any BLUE forbidden pattern (shared header). (4) Use `unsafe` outside the allowlisted FFI in `src/vrf.rs` (cardano-crypto VRF binding). |
| **Inbound deps** | `ade_ledger`, `ade_plutus`, `ade_testkit`. |
| **Outbound deps** | `ade_types`, `blake2`, `ed25519-dalek`, `cardano-crypto` (vrf-draft03 + kes-sum + dsign features, `default-features = false`). |
| **Entry points** | `ade_crypto::blake2b::blake2b_256` (top external), plus re-exports `blake2b_224`, `blake2b_256`, `block_header_hash`, `transaction_id`, `script_hash`, `credential_hash`, `verify_ed25519`, `verify_byron_bootstrap`, `verify_kes`, `verify_opcert`, `verify_vrf`. |

---

### `ade_ledger`

| Attribute | Value |
|-----------|-------|
| **Purpose** | The functional core: stateless ledger rules for every Cardano era — UTxO transitions, certificate effects, fee/value conservation, epoch boundary accounting, hard-fork translations, governance ratification/enactment, and canonical fingerprint of `LedgerState`. |
| **Creates** | `LedgerState`, `EpochState`, `UTxOState`, `TxOut`, `Value`, `MultiAsset`, `AssetName`, `DelegationState`, `CertState`, `PoolParams`, `PoolState`, `PoolRetirement`, `PoolRewards`, `SnapshotState`, `StakeSnapshot`, `MarkSnapshot`, `SetSnapshot`, `GoSnapshot`, `ConwayGovState`, `GovActionState`, `RatificationResult`, `EnactmentEffects`, `ProtocolParameters`, `ProtocolParameterUpdate`, `BlockApplyResult`, `BlockVerdict`, `TxVerdict`, `TxOutcome`, `ScriptPosture`, `ScriptVerdict`, `EpochBoundaryAccounting`, `EpochBoundaryResult`, `EpochBoundarySummary`, `LedgerFingerprint`, `Rational`, `WitnessInfo`, `ValidationPhase`, plus a layered family of error enums (`LedgerError`, `ValidityError`, `ConservationError`, `WitnessError`, `ScriptError`, `MintError`, `CertificateError`, `FeeError`, `EpochError`, `TranslationError`, `StructuralError`, `DecodingError`, `RuleNotYetEnforcedError`). |
| **Interprets** | Decoded canonical types from `ade_codec`. Hosts the per-era validation composers under `alonzo.rs`, `babbage.rs`, `conway.rs`, etc., and the late-era witness decoder via `decode_all_plutus_scripts_in_block` / `decode_witness_infos`. |
| **MUST NOT** | (1) Perform I/O (shared header). (2) Use `HashMap`, floats, or any other BLUE-forbidden pattern (`ci_check_forbidden_patterns.sh`). (3) Hash anything other than wire bytes (`ci_check_hash_uses_wire_bytes.sh`). (4) Construct `PreservedCbor` directly (`ci_check_ingress_chokepoints.sh`). (5) Depend on `ade_runtime` (enforced by `ci_check_dependency_boundary.sh`). (6) Use signing primitives or add `#[cfg(feature)]` gates (`ci_check_no_signing_in_blue.sh`, `ci_check_no_semantic_cfg.sh`). (7) Omit the `// Core Contract:` banner from any `.rs` (`ci_check_module_headers.sh`). |
| **Inbound deps** | `ade_testkit`. |
| **Outbound deps** | `ade_types`, `ade_crypto`, `ade_codec`, `ade_plutus`, `num-bigint`, `num-integer`, `num-traits`. No external runtime / I/O crates. |
| **Entry points** | `ade_ledger::rules::apply_block` (8 import sites), `ade_ledger::rules::apply_block_classified`, `ade_ledger::rules::apply_block_with_accounting`, `ade_ledger::rules::apply_block_with_verdicts`, `ade_ledger::rules::apply_epoch_boundary_full`, `ade_ledger::rules::apply_epoch_boundary_with_registrations`, `ade_ledger::state::LedgerState`, `ade_ledger::state::EpochState`, `ade_ledger::pparams::ProtocolParameters`, `ade_ledger::utxo::UTxOState`, `ade_ledger::hfc::translate_era`, `ade_ledger::fingerprint::fingerprint`, `ade_ledger::governance::{evaluate_ratification, enact_proposals, expire_proposals}`, `ade_ledger::delegation::{apply_cert, apply_certs}`, `ade_ledger::epoch::{rotate_snapshots, compute_rewards, retire_pools, apply_epoch_boundary}`, `ade_ledger::witness::{decode_all_plutus_scripts_in_block, decode_witness_infos}`. |
| **Key modules** | `byron.rs`, `shelley.rs`, `mary.rs`, `alonzo.rs`, `babbage.rs`, `conway.rs` (per-era composers), `rules.rs` (top-level `apply_block*`), `state.rs` (`LedgerState`, `EpochState`), `utxo.rs`, `delegation.rs`, `epoch.rs` (boundary + rewards), `governance.rs` (Conway ratification/enactment), `hfc.rs` (hard-fork translations), `late_era_validation.rs`, `phase.rs` (Phase 1/2 distinction), `plutus_eval.rs`, `pparams.rs`, `rational.rs`, `scripts.rs`, `value.rs`, `witness.rs`, `fingerprint.rs`, `error.rs`. |

---

### `ade_plutus`

| Attribute | Value |
|-----------|-------|
| **Purpose** | Quarantine boundary between the Ade-canonical ledger and the ported UPLC evaluator from `aiken-lang/aiken` (pinned to tag `v1.1.21`, commit `42babe5d`). Exposes Ade-canonical types only; aiken- and pallas-originated types are strictly internal. |
| **Creates** | `PlutusScript`, `PlutusLanguage`, `EvalOutput`, `PlutusError`, `CostModels`, `DecoderMode`, `PerScriptResult`, `TxEvalResult`. |
| **Interprets** | UPLC scripts (Plutus V1/V2/V3) and `CostModels` CBOR (with the project's PV-mode decoder). Phase-two transaction evaluation. `PlutusScript::from_cbor` (`crates/ade_plutus/src/evaluator.rs`) is a named ingress chokepoint under `ci_check_ingress_chokepoints.sh`, allowlisted in Check 3 because Plutus script CBOR uses the aiken/pallas decoder rather than `ade_codec` primitives. |
| **MUST NOT** | (1) Re-export any `pallas_*` or `aiken_uplc::` type from its public surface (enforced by `ci_check_pallas_quarantine.sh`; only `ade_plutus` may depend on `pallas-*`). (2) Allow another BLUE crate to import an evaluator entry point bypassing the canonical entry. (3) Activate PV11 builtins (`ExpModInteger`, `CaseList`, `CaseData`) — gated off to match mainnet's unactivated PV11; see `docs/active/S-29_obligation_discharge.md`. (4) Use any BLUE-forbidden pattern (shared header — now in scope under `ci_check_no_signing_in_blue.sh`, `ci_check_no_semantic_cfg.sh`, `ci_check_hash_uses_wire_bytes.sh`, `ci_check_module_headers.sh`); `aiken_uplc`'s transitive `indexmap` is allowed only because it lives inside the aiken tree, not in Ade sources. (5) Construct `PreservedCbor` outside `ade_codec` — `PlutusScript::from_cbor` is the chokepoint for Plutus script CBOR but produces `PlutusScript`, not `PreservedCbor`. |
| **Inbound deps** | `ade_ledger`, `ade_testkit`. |
| **Outbound deps** | `ade_types`, `ade_crypto`, `ade_codec`, `aiken_uplc` (git, tag `v1.1.21`, renamed from `uplc`), `pallas-primitives` (referenced internally for `Language` only; not re-exported). |
| **Entry points** | `ade_plutus::eval_tx_phase_two` (and `ade_plutus::tx_eval::SlotConfig` / `MAINNET_SLOT_CONFIG`), `ade_plutus::evaluator::{programs_alpha_equivalent, EvalOutput, PlutusLanguage, PlutusScript}`, `ade_plutus::cost_model::{CostModels, DecoderMode, decode_cost_models}`. |
| **Key modules** | `evaluator.rs` (aiken wrapper; hosts `PlutusScript::from_cbor` ingress chokepoint), `cost_model.rs` (CBOR decoder for Plutus cost models), `script_context.rs` (Ade-canonical V1/V2/V3 ScriptContext builder), `script_verdict.rs`, `tx_eval.rs` (phase-2 entry). |

---

### `ade_types`

| Attribute | Value |
|-----------|-------|
| **Purpose** | Owns the canonical Cardano domain vocabulary — primitives, era enum, address forms, era-tagged transaction bodies / outputs / certificates, governance types — used by every other workspace crate as the lingua franca. The schema, separated from the codec. |
| **Creates** | `CardanoEra`, `SlotNo`, `BlockNo`, `EpochNo`, `Hash28`, `Hash32`, `Coin`, `Lovelace`, `NetworkId`, `Nonce`, `TxIn`, `PoolId`, `Address`, `ByronAddress`, `Credential`, `Certificate`, `ConwayCert`, `MIRCert`, `MIRPot`, `DRep`, `GovAction`, `GovActionId`, `Anchor`, `OperationalCert`, `NativeScript`, `PlutusV1Script`, `Datum`, `DatumOption`, `MultiAsset`, `AssetName`, `CostModel`, `ExUnits`, plus per-era tx-body / tx-out / witness wrappers (`ByronTxBody`/`ByronTxIn`/`ByronTxOut`/`ByronTx`/`ByronWitness`, `AllegraTxBody`, `MaryTxBody`/`MaryTxOut`, `AlonzoTx`/`AlonzoTxBody`/`AlonzoTxOut`/`AlonzoWitnesses`, `BabbageTxBody`/`BabbageTxOut`, `ConwayTxBody`, plus era block headers `ByronEbbBlock`/`ByronEbbHeader`/`ByronRegularBlock`/`ByronRegularHeader`/`ByronConsensusData`). |
| **Interprets** | None — produce-only. Domain types are constructed by `ade_codec` decoders or by `ade_ledger` rules. |
| **MUST NOT** | (1) Construct or decode `PreservedCbor` (codec authority). (2) Use any BLUE-forbidden pattern (shared header). (3) Depend on any workspace crate — `ade_types` is the root of the dependency DAG and has no `[dependencies]` block. (4) Add open/extensible variants to closed era / certificate / governance enums without a versioned gate. |
| **Inbound deps** | `ade_codec`, `ade_crypto`, `ade_ledger`, `ade_plutus`, `ade_runtime`, `ade_testkit`. |
| **Outbound deps** | None (no external or internal deps in `Cargo.toml`). |
| **Entry points** | `ade_types::CardanoEra` (28 import sites — most-used name in the workspace), `ade_types::tx::{Coin, TxIn}` (13), `ade_types::tx::Coin` (10), `ade_types::primitives::{Hash32, SlotNo}` (8), `ade_types::tx::TxIn` (6), `ade_types::Hash32` (6), `ade_types::Hash28` (6), `ade_types::{CardanoEra, EpochNo, SlotNo}` (6). |
| **Key modules** | `primitives.rs`, `era.rs`, `tx.rs`, `address/`, `byron/`, `shelley/`, `allegra/`, `mary/`, `alonzo/`, `babbage/`, `conway/` (era-tagged types). |

---

## GREEN Modules — Deterministic Glue

> Deterministic, non-authoritative. May depend on BLUE; must not affect authoritative outputs.

### `ade_testkit`

| Attribute | Value |
|-----------|-------|
| **Purpose** | Test infrastructure: differential harness, oracle snapshot loader, regression corpus, genesis loader, era mapping, transcript replay, diff reporting. Not authoritative — produces comparisons of authoritative outputs against oracle data and serves the `replay_cmd` test suites. |
| **Creates** | `Era` (mirror of `CardanoEra` for oracle-IO use), `OracleManifest`, `OracleHashEntry`, `LoadedSnapshot`, `SnapshotHeader`, `Manifest`, `ManifestEntry`, `RegressionCorpus`, `RegressionEntry`, `ComparisonSurface`, `DifferentialReport`, `DiffReport`, `Divergence`, `DivergenceKind`, `FirstDivergenceReport`, `LedgerDiffReport`, `LedgerDivergence`, `LedgerHashSequence`, `ProtocolDiffReport`, `ProtocolDivergence`, `BlockFields`, `BlockMeta`, `CommitteeState`, `ConwayGovParams`, `AlonzoPlutusParams`, `DRepRegistration`, `DStateProbe`, `DStateFieldInfo`, `ArtifactDigest`, `ExpectedVerdict`, `HarnessError`, `InstantaneousRewardsSummary`, `MiniProtocolId`, `Transcript`, `TranscriptMessage`, `RewardParams`, `ShelleyOracleUtxo`, `ShelleyOracleUtxoEntry`, `ShelleyCompactTxIn`, `StateHash`, `StubBlockDecoder`, `StubLedgerApplicator`, `StubProtocolStateMachine`, `VotingDelegationStats`, plus violation reports `ManifestViolation`/`CorpusViolation`/`ProvenanceViolation`. |
| **Interprets** | Oracle reference snapshots (compressed tarballs containing CBOR-encoded `ExtLedgerState` and associated metadata) and the regression-corpus manifest. Decode-side mirror of `ade_codec` for oracle-only formats. |
| **MUST NOT** | (1) Affect authoritative outputs — testkit code is excluded from BLUE forbidden-pattern scans and may use `HashMap`/`serde_json`/`flate2`/`tar`, but its results must never feed back into `ade_ledger`/`ade_codec`/`ade_crypto` state. (2) Be linked from any RED crate (it is dev infrastructure; current inbound is zero — testkit is consumed only by its own integration tests). (3) Import `ade_runtime` (preserves the GREEN-not-RED stance). (4) Introduce nondeterminism that leaks into stored fixtures — fixtures must be byte-reproducible. |
| **Inbound deps** | None at compile time (no other crate lists `ade_testkit` in `Cargo.toml`). All consumption is through its own integration tests in `crates/ade_testkit/tests/` (29 test files). |
| **Outbound deps** | `ade_types`, `ade_codec`, `ade_crypto`, `ade_ledger`, `ade_plutus`, `blake2`, `flate2`, `tar`, `serde`, `serde_json`, `toml`. |
| **Entry points** | `ade_testkit::harness::snapshot_loader::LoadedSnapshot` (9 import sites), `ade_testkit::harness::snapshot_loader::{extract_state_from_tarball, parse_*}` family, `ade_testkit::harness::genesis_loader::load_genesis_utxo` (3), plus the test-binary entry points under `crates/ade_testkit/tests/` (boundary_replay, differential_*_replay, ledger_determinism, contiguous_corpus_decode, plutus_conformance, etc.). |
| **Key modules** | `harness/snapshot_loader.rs` (oracle tarball extraction + per-era state parsers), `harness/genesis_loader.rs`, `harness/shelley_loader.rs`, `harness/era_mapping.rs`, `harness/oracle_manifest.rs`, `harness/regression_corpus.rs`, `harness/provenance.rs`, `harness/transcript.rs`, `harness/block_diff.rs`, `harness/ledger_diff.rs`, `harness/protocol_diff.rs`, `harness/diff_report.rs`, `harness/address_extractor.rs`, `harness/adapters/{byron,shelley,allegra,mary,alonzo,babbage,conway,shelley_common}.rs`. |

> **Classification note.** `ade_testkit` reads files from disk via `flate2`/`tar` in test helpers. That is an authoritative-vs-test distinction: I/O exists, but only to materialize oracle fixtures consumed by tests. By IDD doctrine that is GREEN (deterministic glue used to compare against authority) rather than RED. The crate is "demotable to RED" if/when it grows orchestration code that mutates oracle state.

---

## RED Modules — Imperative Shell

> I/O, network, storage, clocks, retries. May depend on BLUE/GREEN. Must not modify core state directly or construct semantic types unsafely.

### `ade_node`

| Attribute | Value |
|-----------|-------|
| **Purpose** | Binary entry point for the node process. Currently a hello-world stub: `main()` prints `"ade node"` and exits. Reserved as the assembly point for the runtime + ledger + chain DB. |
| **Creates** | None. |
| **Interprets** | None yet. |
| **MUST NOT** | (1) Bypass `ade_codec` to construct semantic types from bytes. (2) Modify `ade_ledger` state in place — all state transitions go through `ade_ledger::rules::*`. (3) Take an inbound dep from any other crate (it is the binary; nothing should import it). |
| **Inbound deps** | None. |
| **Outbound deps** | None at present. (`Cargo.toml` has no `[dependencies]` block; the stub `main()` uses only `std`.) |
| **Entry points** | `main()` in `crates/ade_node/src/main.rs`. |

> **Gap surfaced.** The Phase 4 cluster plan (`docs/active/phase_4_cluster_plan.md`) anticipates `ade_node` becoming the place where `ade_runtime::chaindb` + `ade_runtime::recovery` + `ade_ledger::rules::apply_block` are composed (cluster N-E onward). The MUST NOT list above is forward-looking; it has nothing to enforce today.

---

### `ade_runtime`

| Attribute | Value |
|-----------|-------|
| **Purpose** | The imperative shell. Hosts the `ChainDb` / `SnapshotStore` storage abstractions, their in-memory and redb-backed implementations, the crash-safety test harness, and the snapshot + forward-replay recovery primitive. Where I/O, fsync policy, and process lifecycle live. |
| **Creates** | `ChainDb` trait, `SnapshotStore` trait, `KillStrategy<D>` trait, `ChainTip`, `StoredBlock`, `ChainDbError`, `InMemoryChainDb`, `PersistentChainDb`, `PersistentChainDbOptions`, `SyncCadence`, `NoKill`, `BlockIter<'a>`, plus recovery types `Recoverable` trait, `StartingState`, `RecoveryReport<R>`, `RecoveryError<E>`. |
| **Interprets** | Block bytes for content-addressed storage (it stores them; semantic decoding remains in `ade_codec`). Snapshot bytes are byte-opaque to `ade_runtime` — encoding and decoding live in caller-provided `Recoverable` impls. |
| **MUST NOT** | (1) Depend on `ade_ledger` (S-36 forbidden patterns: `ade_runtime` stays decoupled; bytes-in/bytes-out only). (2) Leak `redb` or any other backing-store type through the `chaindb::*` public surface (S-34 forbidden patterns). (3) Have a second public chaindb path — trait is the only surface (S-34). (4) Couple snapshots to a specific state type — `SnapshotStore` is byte-opaque (S-35). (5) Apply automatic snapshot pruning — operator-driven only (S-35, S-36). (6) Allow partial-recovery success — mid-replay failure aborts (S-36). (7) Add async recovery surface — sync only; callers wrap if cancellation is needed (S-36). (8) Treat "not found" as an error path or silently retry on real errors (S-34). |
| **Inbound deps** | None at compile time; `crates/ade_testkit/tests/` and `crates/ade_runtime/tests/` exercise it directly via 2 `use ade_runtime::chaindb::{ ... }` sites. The binary `chaindb_kill_target` (added in S-37) lives at `src/bin/`. |
| **Outbound deps** | `ade_types`, `redb` (S-34 Tier-5 choice — single-file ACID, MIT/Apache). Dev-deps: `tempfile`. |
| **Entry points** | `ade_runtime::chaindb::ChainDb`, `ade_runtime::chaindb::SnapshotStore`, `ade_runtime::chaindb::PersistentChainDb` / `PersistentChainDbOptions` / `SyncCadence`, `ade_runtime::chaindb::InMemoryChainDb`, `ade_runtime::chaindb::run_contract_tests` / `run_snapshot_contract_tests` / `run_crash_safety_tests`, `ade_runtime::recovery::recover`, `ade_runtime::recovery::Recoverable`. |
| **Key modules** | `chaindb/mod.rs` (the `ChainDb` + `SnapshotStore` trait definitions), `chaindb/contract.rs` (block-store contract test battery), `chaindb/snapshot_contract.rs` (snapshot-store contract tests), `chaindb/crash_safety.rs` (kill-strategy framework), `chaindb/in_memory.rs` (S-33 reference impl), `chaindb/persistent.rs` (S-34 redb-backed real impl), `chaindb/types.rs`, `chaindb/error.rs`, `recovery.rs` (S-36 snapshot + forward replay), `bin/chaindb_kill_target.rs` (S-37 subprocess SIGKILL stress target). |
| **Mechanical enforcement** | The chaindb / recovery / crash-safety surface above is now backed by three dedicated CI scripts added in commit `78da6c9`: `ci/ci_check_chaindb_contract.sh` runs `cargo test -p ade_runtime --lib chaindb::` (covers `in_memory_passes_contract`, `persistent_passes_contract`, `in_memory_passes_snapshot_contract`, `persistent_passes_snapshot_contract`, `reopen_observes_committed_block`, `corrupted_magic_returns_corruption_error`, `snapshots_persist_across_reopen`, `persistent_passes_crash_safety_with_no_kill` — 8 tests) and enforces DC-STORE-02, DC-STORE-03, CN-STORE-04, CN-STORE-05. `ci/ci_check_recovery_contract.sh` runs `cargo test -p ade_runtime --lib recovery::` (covers `recover_from_snapshot_and_replay_forward`, `recover_from_genesis_when_no_snapshot`, `no_starting_point_error`, `snapshot_decode_failure_surfaces_as_error`, `apply_failure_surfaces_with_slot`, `snapshot_with_no_post_blocks_is_ok` — 6 tests) and enforces T-REC-01, T-REC-02, DC-STORE-05. `ci/ci_check_chaindb_crash_safety.sh` runs `stress_kill_smoke` (10 iterations) + `snapshot_table_intact_after_kill_loop` against the subprocess SIGKILL harness in `crates/ade_runtime/tests/stress_kill_harness.rs` and enforces T-REC-01 (crash variant), DC-STORE-01, CN-STORE-03 plus the CE-N-D-1 mechanical-acceptance gate. The 1000-iteration variant `stress_kill_1000` is `#[ignore]` and runs manually for closure-gate evidence. |

---

## Cross-Module Rules (project-wide)

### Dependency direction

`ade_runtime` → `{ade_ledger, ade_plutus, ade_crypto, ade_codec, ade_types}` is the legal direction (and at HEAD `ade_runtime` only depends on `ade_types`).
`ade_testkit` → `{ade_ledger, ade_plutus, ade_crypto, ade_codec, ade_types}` is legal (testkit is GREEN).
`ade_ledger` → `{ade_plutus, ade_crypto, ade_codec, ade_types}` is legal (BLUE among BLUEs).
`ade_plutus` → `{ade_crypto, ade_codec, ade_types}` is legal.
`ade_crypto` → `{ade_types}` is legal.
`ade_codec` → `{ade_types}` is legal.
`ade_types` → `{}`.
`ade_core` → `{}`.

**Forbidden directions.** Any BLUE crate depending on `ade_runtime` or `ade_node` is a CI failure (`ci_check_dependency_boundary.sh`, scoped to the full 6-crate BLUE list). Any non-`ade_plutus` crate referring to `pallas_*` is a CI failure (`ci_check_pallas_quarantine.sh`).

### Naming convention

All crates are prefixed `ade_`. TCB color is not encoded in the name. The authoritative classifier is `.idd-config.json` `core_paths`; CI scripts hard-code their BLUE list. As of commit `5b70bee` all six BLUE-scoped scripts (`ci_check_module_headers.sh`, `ci_check_no_semantic_cfg.sh`, `ci_check_no_signing_in_blue.sh`, `ci_check_hash_uses_wire_bytes.sh`, `ci_check_ingress_chokepoints.sh`, `ci_check_dependency_boundary.sh`) use the full BLUE set `{ade_codec, ade_types, ade_crypto, ade_core, ade_ledger, ade_plutus}`.

### CI enforcement (19 scripts under `ci/`)

| Script | Enforces | Scope |
|---|---|---|
| `ci_check_cbor_round_trip.sh` | T-ENC-03, DC-CBOR-01, DC-CBOR-02 | golden corpus |
| `ci_check_chaindb_contract.sh` *(new in `78da6c9`)* | DC-STORE-02, DC-STORE-03, CN-STORE-04, CN-STORE-05 | `ade_runtime --lib chaindb::` (RED) |
| `ci_check_chaindb_crash_safety.sh` *(new in `78da6c9`)* | T-REC-01 (crash variant), DC-STORE-01, CN-STORE-03; CE-N-D-1 gate | `ade_runtime --test stress_kill_harness` (RED) |
| `ci_check_constitution_coverage.sh` | invariant-registry ↔ code/test coverage | repo-wide |
| `ci_check_crypto_vectors.sh` | crypto KAT regression | `ade_crypto` |
| `ci_check_dependency_boundary.sh` | T-BOUND-02 — BLUE ⇎ RED separation | full BLUE (6-crate) |
| `ci_check_differential_divergence.sh` | DC-DIFF-* | replay outputs |
| `ci_check_forbidden_patterns.sh` | T-CORE-02 — no `HashMap`/floats/clocks/etc. + `unsafe` allowlist | BLUE crates |
| `ci_check_hash_uses_wire_bytes.sh` | DC-CBOR-02, T-ENC-01 | full BLUE (6-crate) |
| `ci_check_hfc_translation.sh` | DC-EPOCH-02 (CE-73-semantic) | `ade_ledger::hfc` |
| `ci_check_ingress_chokepoints.sh` | DC-INGRESS-01, T-INGRESS-01 — `PreservedCbor` via named decoders only; Check 3 allowlists `ade_plutus/src/evaluator.rs` for the `PlutusScript::from_cbor` chokepoint | full BLUE (6-crate) |
| `ci_check_ledger_determinism.sh` | DC-LEDGER-01 (CE-74) | `ade_ledger` |
| `ci_check_module_headers.sh` | CE-04 contract banner | full BLUE (6-crate) |
| `ci_check_no_secrets.sh` | no credentials/IPs/keys in tree | repo-wide |
| `ci_check_no_semantic_cfg.sh` | T-BUILD-01 — no `#[cfg(feature)]` | full BLUE (6-crate) |
| `ci_check_no_signing_in_blue.sh` | CE-05, T-KEY-01 — signing in RED only | full BLUE (6-crate) |
| `ci_check_pallas_quarantine.sh` | O-29.2 — `pallas-*` confined to `ade_plutus` | non-`ade_plutus` |
| `ci_check_recovery_contract.sh` *(new in `78da6c9`)* | T-REC-01, T-REC-02, DC-STORE-05 | `ade_runtime --lib recovery::` (RED) |
| `ci_check_ref_provenance.sh` | DC-REF-01 — manifest checksum integrity | reference corpus |

> **RED-scope note.** The three new scripts in commit `78da6c9` close a TRACEABILITY-surfaced gap where 9 storage / recovery rules had code and tests but empty `ci_script` fields (status `declared`). With this commit they flip to `enforced`: T-REC-01, T-REC-02, DC-STORE-01, DC-STORE-02, DC-STORE-03, DC-STORE-05, CN-STORE-03, CN-STORE-04, CN-STORE-05. DC-STORE-04 remains `declared` by design — it names cardano-node's literal three-DB topology (ImmutableDB + VolatileDB + LedgerDB), and Ade has deliberately diverged per CE-79 Tier 5 to a single redb store with logical key-prefix separation; the semantic guarantees DC-STORE-04 stated survive under DC-STORE-02 / DC-STORE-03.
