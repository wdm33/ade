# Module Authority Map — Ade

> **Status:** Living architectural document. Regenerated; not hand-edited.
> Per-project instance of `~/.claude/methodology/templates/codemap.md`.

> 11 crates, 338 canonical types, 1189 tests, 25 CI checks at HEAD (`0d4457e`).

---

## Conventions

- A **module** in Ade is a Cargo workspace crate (smallest independently-buildable unit). One exception: `ade_network` is split by *submodule color* — its BLUE submodules and its RED submodules are documented as two separate entries below, because `.idd-config.json` `core_paths` resolves BLUE at the submodule path level rather than crate-wide. The `ade_core::consensus` submodule sits *inside* the BLUE `ade_core` crate and is covered by that entry. The `ade_ledger::block_validity` / `ade_ledger::consensus_view` modules sit inside the BLUE `ade_ledger` crate and are covered by that entry; the RED `ade_ledger::consensus_input_extract` sits inside the BLUE `ade_ledger` crate but is RED by its own module doc-comment and the PHASE4-B1 TCB Color Map — it is surfaced as a sub-classification note inside the `ade_ledger` entry.
- Modules are listed by TCB color (BLUE → GREEN → RED), alphabetical within each color.
- TCB color sources, in order of authority:
  1. `.idd-config.json` `core_paths` — substring match against absolute path. BLUE matches: `ade_codec`, `ade_types`, `ade_crypto`, `ade_core`, `ade_ledger`, `ade_plutus`, and the 9 `ade_network` submodule paths (`mux/frame.rs`, `codec/`, `handshake/`, `chain_sync/`, `block_fetch/`, `tx_submission/`, `keep_alive/`, `peer_sharing/`, `n2c/`).
  2. `.idd-config.json` `_core_paths_doc` — `ade_runtime` is RED; `ade_testkit` is GREEN; `ade_node` is RED; `ade_network::mux::transport` and `ade_network::session` are RED; `ade_network::lib` and `ade_network::mux::mod` are GREEN barrels. `ade_core_interop` is **RED** per its `Cargo.toml` header comment and the PHASE4-N-B TCB Color Map.
  3. `docs/clusters/PHASE4-N-B/cluster.md` § "TCB Color Map" — `era_schedule`, `praos_state`, `vrf_cert`, `nonce`, `op_cert`, `leader_schedule`, `header_validate`, `fork_choice`, `rollback` are BLUE; `chain_selector`, `candidate_fragment` are GREEN; `genesis_parser` is RED; `ade_testkit::consensus` is GREEN; `ade_core_interop` is RED.
  4. `docs/clusters/PHASE4-B1/cluster.md` § "TCB Color Map" — `ade_ledger::consensus_view`, `ade_ledger::block_validity` are BLUE; `ade_core::consensus::{header_validate, kes_check}` extensions are BLUE; `ade_testkit::validity` is GREEN; `ade_ledger::consensus_input_extract` is RED (parses an external dump format rather than a canonical type).
- Counts:
  - Crates: 11, from `Cargo.toml` `[workspace] members`.
  - Canonical types: 338, from `grep -rE "^pub (struct|enum) "` across the full BLUE scope. Breakdown: `ade_codec` 8, `ade_types` 76, `ade_crypto` 12, `ade_core` 42, `ade_ledger` 94, `ade_plutus` 8, plus the 9 BLUE `ade_network` submodule paths 98. Registry `canonical_type_registry: null`, so a structural count is used. **+19 since the previous run** (`ade_core` +4: `HeaderVrf`, `HeaderKes`, `FieldError`, `FieldKind` for the KES/VRF header surface; `ade_ledger` +15: `BlockValidityOutcome`, `DecodedBlock`, `VerdictSurface`, `SurfaceDecodeError`, `BlockValidityVerdict`, `BlockRejectClass`, `BlockValidityError`, `FieldError`, `FieldKind`, `MissingInput`, `PoolEntry`, `PoolDistrView`, plus the RED extractor's `Nonce`, `PraosNonces`, `NonceScanError`).
  - Tests: 1189 — count of `#[test]` / `#[tokio::test]` attributes across `crates/`. Reported as approximate per the template's fallback rule (test runner not executed). **+38 since the previous run** (PHASE4-B1: `ade_ledger::{consensus_view, consensus_input_extract, block_validity::*}` unit tests plus the `block_validity_*_corpus` / `ledger_view_corpus` / `block_validity_types` / `block_validity_compose` integration tests, and `ade_testkit::validity` harness tests).
  - CI checks: 25 — file count under `ci/ci_check_*.sh`. **No change since the previous run.** PHASE4-B1 added **no new CI script** — DC-VAL-01..06 are enforced by named `cargo test` targets (the closing-slice tests recorded in the registry), not by grep gates. The `ci_check_no_fail_open_in_validation.sh` script anticipated in the cluster doc's forward-looking engineering surface was **not** shipped; DC-VAL-06 is currently test-enforced only. See the gap note in the `ade_ledger` entry. No `.github/workflows/` yet.

---

## BLUE Modules — Pure Functional Core

> **Shared header (applies to every BLUE entry below).** Every `.rs` source file begins with the contract banner
> `// Core Contract:` and the following deny attributes are present in each crate's `lib.rs` (or, for `ade_network`,
> at the crate root — the BLUE submodules inherit them):
> `#![deny(unsafe_code)]`, `#![deny(clippy::unwrap_used)]`, `#![deny(clippy::expect_used)]`,
> `#![deny(clippy::panic)]`, `#![deny(clippy::float_arithmetic)]`.
> CI scripts that enforce the shared rules (all 7 scope the full BLUE set — the 6 BLUE crates plus the 9 BLUE `ade_network` submodule paths declared in `.idd-config.json` `core_paths`):
> - `ci/ci_check_module_headers.sh` — banner first-line check.
> - `ci/ci_check_forbidden_patterns.sh` — `HashMap`, `HashSet`, `IndexMap`/`IndexSet`/`indexmap::`, `SystemTime`, `Instant`, `std::fs`, `std::net`, `tokio`, `async fn`, `f32`/`f64`, `anyhow`, `rand::thread_rng`, `thread::spawn`, plus `unsafe` outside a documented allowlist.
> - `ci/ci_check_dependency_boundary.sh` — no BLUE crate depends on a RED crate.
> - `ci/ci_check_no_signing_in_blue.sh` — `SigningKey`/`SecretKey`/`PrivateKey`/`private_key`/`sign_message`/`sign_block` forbidden in BLUE.
> - `ci/ci_check_no_semantic_cfg.sh` — `#[cfg(feature = ...)]` and `cfg!(feature = ...)` forbidden in BLUE.
> - `ci/ci_check_hash_uses_wire_bytes.sh` — no hashing of `canonical_bytes` / re-encoded bytes in BLUE.
> - `ci/ci_check_ingress_chokepoints.sh` — only named `decode_*` chokepoints construct `PreservedCbor`. The named-chokepoint list covers `decode_block_envelope`, the per-era block decoders (`decode_byron_ebb_block`, `decode_byron_regular_block`, `decode_shelley_block`, `decode_allegra_block`, `decode_mary_block`, `decode_alonzo_block`, `decode_babbage_block`, `decode_conway_block`), `decode_address`, and `PlutusScript::from_cbor` in `ade_plutus`. Check 3 explicitly allowlists `ade_plutus/src/evaluator.rs` because Plutus script CBOR is a distinct ingress surface from block CBOR (its decoder goes through aiken/pallas rather than the ade_codec primitives).
> - `ci/ci_check_pallas_quarantine.sh` — `pallas-*` references confined to `ade_plutus`.
> - `ci/ci_check_no_async_in_blue.sh` *(PHASE4-N-A, S-A1)* — `async fn`, `.await`, `tokio`, `async_std`, `futures::`, `tokio::spawn` / `async_std::task::spawn` / `smol::spawn`, and `tokio::time::*` / `async_std::task::sleep` / `futures_timer::*` forbidden anywhere in the BLUE scope. Self-test mode (`--self-test`) plants a synthetic violation and verifies the scanner trips on it. Enforces DC-CORE-01.
>
> Three additional CI scripts narrow the shared header to the `ade_core::consensus` tree specifically (PHASE4-N-B):
> - `ci/ci_check_no_chaindb_in_consensus_blue.sh` — `ChainDb` / `chain_db` token forbidden anywhere under `crates/ade_core/src/consensus/`. Strengthens DC-CORE-01 + DC-CONS-07.
> - `ci/ci_check_no_float_in_consensus.sh` — `f32`/`f64` token forbidden anywhere under `crates/ade_core/src/consensus/`. Narrows T-CORE-02 + DC-CONS-07/08/09 to the consensus surface.
> - `ci/ci_check_consensus_closed_enums.sh` — `#[non_exhaustive]`, open-tail `Other`/`Unknown` enum variants, owned `String` fields in `errors.rs` / `encoding.rs` / `events.rs`, and `Box<dyn ...>` are all forbidden under `crates/ade_core/src/consensus/`. Strengthens DC-CONS-04 + DC-CONS-10 + T-DET-01.
>
> A fourth narrow check enforces a single fork-choice rule:
> - `ci/ci_check_no_density_in_fork_choice.sh` — the term `density` (case-insensitive) is forbidden in `crates/ade_core/src/consensus/fork_choice.rs` and `crates/ade_core/src/consensus/candidate.rs`, with a documented `// no-density:` annotation as the only allowed exception. Strengthens DC-CONS-03 by mechanically forbidding the Genesis/catch-up scoring shape inside caught-up Praos.
>
> A BLUE crate or BLUE `ade_network` submodule that adds a feature flag, an async function, a `HashMap`, or a RED dep fails CI on push.
> The 3 RED-scope CI scripts added in commit `78da6c9` (`ci_check_chaindb_contract.sh`, `ci_check_recovery_contract.sh`, `ci_check_chaindb_crash_safety.sh`) and the 1 evidence script added at S-A10 (`ci_check_ce_n_a_5_proof.sh`) are not part of this shared header. They are documented in the cross-module CI matrix at the bottom.
>
> **PHASE4-B1 note.** The new BLUE surface (`ade_ledger::consensus_view`, `ade_ledger::block_validity::*`, `ade_core::consensus::kes_check`, the Praos-VRF + KES + `HeaderVrf` extensions to `ade_core::consensus::{vrf_cert, header_summary, header_validate, errors}`) inherits the shared-header scope above — it lives under the already-BLUE `ade_ledger` and `ade_core` crate prefixes, so `ci_check_forbidden_patterns.sh`, `ci_check_no_async_in_blue.sh`, `ci_check_no_chaindb_in_consensus_blue.sh`, `ci_check_no_float_in_consensus.sh`, and `ci_check_consensus_closed_enums.sh` all cover it without modification. No CI script was added for B1; DC-VAL-01..06 are test-enforced (see the per-module gap note).

---

### `ade_codec`

| Attribute | Value |
|-----------|-------|
| **Purpose** | Owns Cardano-canonical CBOR ingress: the only place in the workspace that turns raw bytes into typed semantic values, with wire-byte preservation for every hash-bearing structure. |
| **Creates** | `PreservedCbor<T>`, `RawCbor`, `BlockEnvelope`, `ByronDecodedBlock`, `CodecContext`, `CodecError`, `ContainerEncoding`, `IntWidth`, plus era-tagged block/tx wrappers under `byron/`, `shelley/`, `allegra/`, `mary/`, `alonzo/`, `babbage/`, `conway/`. |
| **Interprets** | All canonical Cardano CBOR — block envelopes, era-specific blocks, tx bodies, tx outs, certificates, addresses. Sole authority for `PreservedCbor::new` (constructor is `pub(crate)`). |
| **MUST NOT** | (1) Construct `PreservedCbor` outside `ade_codec` (enforced by `pub(crate)` + `ci_check_ingress_chokepoints.sh`). (2) Re-encode wire bytes when computing hashes (forbidden by `ci_check_hash_uses_wire_bytes.sh`). (3) Use any forbidden BLUE pattern (see shared header). (4) Depend on any other workspace crate except `ade_types`. |
| **Inbound deps** | `ade_ledger`, `ade_plutus`, `ade_testkit`, `ade_network`, `ade_runtime` (consensus genesis parser uses canonical CBOR primitives for the bootstrap anchor); read-side imports from RED test code go through `ade_testkit`. |
| **Outbound deps** | `ade_types`. No external dependencies; std-only. Dev-deps: `serde_json`, `toml`. |
| **Entry points** | `ade_codec::cbor::envelope::decode_block_envelope` (31 import sites), `ade_codec::cbor` module, `ade_codec::traits::AdeEncode`, `ade_codec::CodecContext`, `ade_codec::byron::decode_byron_block`, `ade_codec::shelley::decode_shelley_block`, `ade_codec::allegra::decode_allegra_block`, `ade_codec::mary::decode_mary_block`, `ade_codec::alonzo::decode_alonzo_block`, `ade_codec::babbage::decode_babbage_block`, `ade_codec::conway::decode_conway_block`, `ade_codec::address::decode_address`. Also `ade_codec::cbor::{canonical_width, write_array_header, write_bytes_canonical, ContainerEncoding}` — the canonical-CBOR primitive set consumed by `ade_runtime::consensus::genesis_parser::compute_anchor_hash`. |
| **Key modules** | `cbor/` (envelope + primitive reader/writer), `byron/`, `shelley/`, `allegra/`, `mary/`, `alonzo/`, `babbage/`, `conway/` (era-tagged decoders), `address/`, `preserved.rs` (`PreservedCbor`), `traits.rs` (`AdeEncode`/`AdeDecode`/`CodecContext`), `primitives.rs`, `error.rs`. |

---

### `ade_core`

> **Status change in PHASE4-N-B.** `ade_core` hosts the entire Praos consensus authority surface under `consensus/`. **PHASE4-B1 extended it**: a real single-VRF Praos verify path (`verify_praos_vrf` / `praos_vrf_input` / `praos_leader_value` / `praos_nonce_value`) alongside the existing two-proof TPraos path; a new `kes_check` module that wires `ade_crypto::kes` into header validation with the fail-closed `expect_size` guard; a `HeaderVrf` enum (TPraos two-proof vs Praos single-proof) and `HeaderKes` carrier in `header_summary`; and `FieldKind` / `FieldError` plus new `HeaderValidationError` variants in `errors.rs` for the Praos branch of `header_validate`.

| Attribute | Value |
|-----------|-------|
| **Purpose** | BLUE authoritative Praos consensus core. Owns the canonical types and pure state-transitions that decide which header / chain Ade accepts: HFC era schedule + slot↔era↔time translation, Praos chain-dep state (nonce / op-cert counters), VRF cert verification (TPraos two-proof **and** Praos single-proof) + leader-eligibility predicate, KES signature + op-cert period verification wired into header admission, header validation pipeline (TPraos and Praos branches), fork-choice (`(BlockNo, TiebreakerView)`), rollback authority, leader-schedule query, and canonical encodings of all chain-dep state and chain events. |
| **Creates** | **Schedule (`era_schedule.rs`):** `BootstrapAnchorHash`, `EraSchedule`, `EraSummary`, `EraLocation`. **State (`praos_state.rs`):** `PraosChainDepState`, `OpCertCounterMap`, `Nonce`. **Events / points (`events.rs`):** `Point`, `ChainHash`, `BlockDistance`, `SecurityParam`, `ChainEvent`, `ChainSelectionReject`. **Errors (`errors.rs`):** `HFCError`, `SlotTimeError`, `OutsideForecastRange`, `HeaderValidationError`, `FieldError`, `FieldKind` *(B1)*, `VrfCertError`, `OpCertCounterError`, `NonceEvolutionError`, `LeaderScheduleError`. **Header surface (`header_summary.rs`, `header_validate.rs`):** `HeaderInput`, `HeaderVrf` *(B1; closed TPraos/Praos VRF discriminant)*, `HeaderKes` *(B1)*, `ValidatedHeaderSummary`, `HeaderApplied`. **KES (`kes_check.rs` — NEW B1):** no public types; `expect_size` + `verify_header_kes`. **Fork-choice (`candidate.rs`, `fork_choice.rs`):** `TiebreakerView`, `CandidateFragment`, `ChainSelectorState`, `ForkChoiceError`. **Op-cert / nonce (`op_cert.rs`, `nonce.rs`):** `OpCertObservation`, `NonceInput`. **VRF (`vrf_cert.rs`):** `VrfRole`, `VerifiedVrf`, `StakeFraction`, `ActiveSlotsCoeff`. **Leader schedule (`leader_schedule.rs`):** `LeaderScheduleQuery`, `LeaderScheduleAnswer`. **Ledger view boundary (`ledger_view.rs`):** `LedgerView` trait. **Rollback (`rollback.rs`):** `RollBackRequest`, `RollBackApplied`. **Encoding (`encoding.rs`):** `DecodeError`. 42 public types across the consensus tree; no public items elsewhere in `ade_core`. |
| **Interprets** | Canonical inputs from the `ade_runtime` shell — `HeaderInput` projections (already-parsed header fields, now carrying a `HeaderVrf` discriminant and a `HeaderKes` carrier), `EraSchedule` materialized once from genesis JSON, `LedgerView` snapshots from `ade_ledger`'s production `PoolDistrView`, and an ordered stream of `StreamInput` events. The KES check verifies the hot KES key signature over the header **body** CBOR bytes (`currentKESPeriod = slot / SLOTS_PER_KES_PERIOD`, mainnet `129_600`). All inputs are typed values; BLUE never reads raw bytes, files, sockets, or a chain store. Also decodes its own canonically-encoded `PraosChainDepState` and `ChainEvent` for replay. |
| **MUST NOT** | (1) Take or accept a `&ChainDb` / `&chain_db` / any storage reference — enforced by `ci_check_no_chaindb_in_consensus_blue.sh` (DC-CORE-01 + DC-CONS-07). (2) Use any `f32`/`f64` token — enforced by `ci_check_no_float_in_consensus.sh`; the leader-eligibility predicate uses Q.123 fixed-point in `u128` against the 18-term Cardano `taylorExpCmp` polynomial, never floats. (3) Add `#[non_exhaustive]` to any consensus enum, define an open-tail `Other`/`Unknown` variant, hold owned `String` fields in `errors.rs` / `encoding.rs` / `events.rs`, or use `Box<dyn ...>` — enforced by `ci_check_consensus_closed_enums.sh` (DC-CONS-04 + DC-CONS-10 + T-DET-01); every reject reason and error variant is flat, value-typed, replay-stable. The B1 `FieldError`/`FieldKind` taxonomy obeys this — flat enums, no `String`. (4) Reference the word `density` in `fork_choice.rs` or `candidate.rs` outside a `// no-density:` annotation — enforced by `ci_check_no_density_in_fork_choice.sh` (DC-CONS-03). (5) Read wall-clock time — `slot_to_time_ms` is pure integer arithmetic over `EraSchedule.system_start_unix_ms`; DC-CONS-08. (6) Use `HashMap` / `HashSet` anywhere; `OpCertCounterMap` and per-epoch fixture maps use `BTreeMap`. (7) Contain `async fn`, `.await`, `tokio`, etc. — `ci_check_no_async_in_blue.sh`. (8) Construct a `PreservedCbor`. (9) Re-derive stake snapshots — consume `LedgerView` only (DC-CONSENSUS-02). (10) Bypass the canonical `validate_and_apply_header` pipeline. (11) **B1 fail-closed rule (DC-VAL-06):** never apply a fixed-size crypto-field check via `if len == K { check } else { skip }` — every size guard goes through `kes_check::expect_size`, which returns a typed `FieldError` on any mismatch and cannot silently skip. (12) **B1 KES rule (DC-CRYPTO-01):** never accept a header whose KES signature or op-cert period fails; `verify_header_kes` is fail-closed. (13) All shared-header BLUE rules. |
| **Inbound deps** | `ade_ledger` *(NEW B1 — `block_validity` calls `validate_and_apply_header` and the Praos VRF/KES surface; acyclic, see dependency-direction section)*, `ade_runtime` (consensus orchestrator + genesis parser), `ade_testkit` (consensus + validity harnesses, ledger-view stubs, stream-replay driver), `ade_core_interop` (live interop binary). |
| **Outbound deps** | `ade_types`, `ade_crypto` (`blake2b`, `verify_vrf`, and B1's `kes::{verify_kes, verify_opcert, build_opcert_signable}`), `minicbor` (canonical encoding of `PraosChainDepState` and `ChainEvent`). Dev-deps: `ade_testkit`, `serde_json`, `cardano-crypto` (vrf-draft03; tests only). |
| **Entry points** | **Most-reached BLUE consensus imports:** `use ade_core::consensus::{...}` aggregator, `ade_core::consensus::ledger_view::LedgerView`, `ade_core::consensus::vrf_cert::{vrf_input, verify_praos_vrf, praos_vrf_input, praos_leader_value, praos_nonce_value}` *(B1 Praos path)*, `ade_core::consensus::kes_check::{verify_header_kes, expect_size}` *(NEW B1)*, `ade_core::consensus::praos_state::{Nonce, PraosChainDepState}`, `ade_core::consensus::header_summary::{HeaderInput, HeaderVrf, HeaderKes, ValidatedHeaderSummary}` *(B1)*. **Top-level transitions:** `validate_and_apply_header` (header pipeline, now with a Praos branch), `select_best_chain`, `apply_rollback`, `apply_nonce_input`, `apply_op_cert`, `query_leader_schedule` + `is_leader_for_vrf_output`, `verify_vrf_cert` + `verify_praos_vrf` + `check_leader_claim` + `is_leader`, `tiebreaker_prefer`, `encode_chain_dep_state` / `decode_chain_dep_state` / `encode_chain_event` / `decode_chain_event`. |
| **Key modules** | `consensus/era_schedule.rs`, `consensus/praos_state.rs`, `consensus/events.rs`, `consensus/errors.rs` (flat error taxonomy, no `String`; B1 added `FieldKind`/`FieldError` + Praos `HeaderValidationError` variants), `consensus/vrf_cert.rs` (TPraos two-proof + B1 single-proof Praos verify + Q.123 `taylorExpCmp` leader predicate), `consensus/kes_check.rs` *(NEW B1 — `expect_size` fail-closed guard + `verify_header_kes` over header body CBOR)*, `consensus/nonce.rs`, `consensus/op_cert.rs`, `consensus/leader_schedule.rs`, `consensus/header_summary.rs` (now carries `HeaderVrf` + `HeaderKes`) + `consensus/header_validate.rs` (TPraos and Praos admission branches), `consensus/candidate.rs`, `consensus/fork_choice.rs`, `consensus/rollback.rs`, `consensus/ledger_view.rs`, `consensus/encoding.rs`. |

---

### `ade_crypto`

| Attribute | Value |
|-----------|-------|
| **Purpose** | Owns pure cryptographic verification primitives — Blake2b hashing, Ed25519 / Byron-bootstrap signature verification, KES verification with operational certificates, VRF verification. Verification only — signing lives in `ade_runtime`. |
| **Creates** | `Blake2b224`, `Blake2b256`, `HashAlgorithm` trait, `Ed25519VerificationKey`, `Ed25519Signature`, `ByronExtendedVerificationKey`, `KesVerificationKey`, `KesPeriod`, `OperationalCertData`, `VrfVerificationKey`, `VrfProof`, `VrfOutput`, `CryptoError`. |
| **Interprets** | Verification key / signature / proof byte structures. Not a CBOR parser — accepts already-decoded byte slices from `ade_codec` consumers. |
| **MUST NOT** | (1) Implement signing (enforced by `ci_check_no_signing_in_blue.sh` — patterns `SigningKey`/`SecretKey`/`PrivateKey`/`private_key`/`sign_message`/`sign_block`). (2) Allocate global state — verification is `fn(&[u8], &[u8]) -> bool`. (3) Use any BLUE forbidden pattern (shared header). (4) Use `unsafe` outside the allowlisted FFI in `src/vrf.rs` (cardano-crypto VRF binding). (5) **B1:** `build_opcert_signable` must produce the spec-correct **raw concatenation** of the op-cert fields (KES vkey ‖ counter ‖ KES period) — not a CBOR-wrapped or otherwise re-encoded form; a wrong signable shape silently breaks KES verification for every Praos header. |
| **Inbound deps** | `ade_core` (consensus VRF wrapping, KES via `kes_check`, blake2b for nonce evolution + bootstrap anchor), `ade_ledger`, `ade_plutus`, `ade_testkit`. |
| **Outbound deps** | `ade_types`, `blake2`, `ed25519-dalek`, `cardano-crypto` (vrf-draft03 + kes-sum + dsign features, `default-features = false`). |
| **Entry points** | `ade_crypto::blake2b::blake2b_256` (top external), plus re-exports `blake2b_224`, `blake2b_256`, `block_header_hash`, `transaction_id`, `script_hash`, `credential_hash`, `verify_ed25519`, `verify_byron_bootstrap`, `verify_kes`, `verify_opcert`, `verify_vrf`. `ade_crypto::vrf::{VrfOutput, VrfProof, VrfVerificationKey, verify_vrf}` consumed by `ade_core::consensus::vrf_cert`. **B1:** `ade_crypto::kes::{verify_kes, verify_opcert, build_opcert_signable}` consumed by `ade_core::consensus::kes_check::verify_header_kes`; `build_opcert_signable` fixed in B1 to the spec-correct raw concat. |

---

### `ade_ledger`

> **Status change in PHASE4-B1.** `ade_ledger` gained the full block-validity authority and now **depends on `ade_core`** (acyclic: `ade_core` does not depend on `ade_ledger`). New BLUE submodules: `consensus_view` (the production `LedgerView` projection — `PoolDistrView`) and `block_validity::{verdict, encoding, transition, header_input}` (the closed `BlockValidityVerdict`/`BlockValidityError` taxonomies, canonical CBOR replay surface, `header ∧ body` composition, and body-hash binding). One **RED** submodule was added inside the crate: `consensus_input_extract` (PraosState nonce tail-scan over an external snapshot dump format) — see the sub-classification note below.

| Attribute | Value |
|-----------|-------|
| **Purpose** | The functional core (ledger half): stateless ledger rules for every Cardano era — UTxO transitions, certificate effects, fee/value conservation, epoch boundary accounting, hard-fork translations, governance ratification/enactment, canonical fingerprint of `LedgerState` — **and, as of B1, the top-level block-validity verdict** that composes the consensus-header authority (`ade_core::consensus::validate_and_apply_header`) with the ledger-body authority (`apply_block_with_verdicts`) into a single pure, total `block_validity` function whose Valid/Invalid verdict is meant to equal the reference cardano-node verdict (DC-VAL-02/03/04/05). |
| **Creates** | Pre-B1: `LedgerState`, `EpochState`, `UTxOState`, `TxOut`, `Value`, `MultiAsset`, `AssetName`, `DelegationState`, `CertState`, `PoolParams`, `PoolState`, `PoolRetirement`, `PoolRewards`, `SnapshotState`, `StakeSnapshot`, `MarkSnapshot`, `SetSnapshot`, `GoSnapshot`, `ConwayGovState`, `GovActionState`, `RatificationResult`, `EnactmentEffects`, `ProtocolParameters`, `ProtocolParameterUpdate`, `BlockApplyResult`, `BlockVerdict`, `TxVerdict`, `TxOutcome`, `ScriptPosture`, `ScriptVerdict`, `EpochBoundaryAccounting`, `EpochBoundaryResult`, `EpochBoundarySummary`, `LedgerFingerprint`, `Rational`, `WitnessInfo`, `ValidationPhase`, plus the layered error-enum family (`LedgerError`, `ValidityError`, `ConservationError`, `WitnessError`, `ScriptError`, `MintError`, `CertificateError`, `FeeError`, `EpochError`, `TranslationError`, `StructuralError`, `DecodingError`, `RuleNotYetEnforcedError`). **NEW B1 — `consensus_view`:** `PoolDistrView`, `PoolEntry` (the production `LedgerView` projection: total active stake, per-pool active stake, per-pool registered VRF keyhash, active-slots coefficient — and nothing else). **NEW B1 — `block_validity`:** `BlockValidityOutcome`, `DecodedBlock`, `BlockValidityVerdict`, `BlockRejectClass`, `BlockValidityError`, `FieldError`, `FieldKind`, `MissingInput`, `VerdictSurface`, `SurfaceDecodeError`. |
| **Interprets** | Decoded canonical types from `ade_codec`. Hosts the per-era validation composers and the late-era witness decoder. **B1:** `block_validity::header_input::decode_block` decodes a block envelope into the header projection consumed by `validate_and_apply_header`; `block_validity::encoding::{encode_verdict_surface, decode_verdict_surface}` is the canonical CBOR replay/comparison surface for the verdict. |
| **MUST NOT** | (1) Perform I/O (shared header). (2) Use `HashMap`, floats, or any other BLUE-forbidden pattern (`ci_check_forbidden_patterns.sh`). (3) Hash anything other than wire bytes (`ci_check_hash_uses_wire_bytes.sh`). (4) Construct `PreservedCbor` directly outside the named decoders (`ci_check_ingress_chokepoints.sh`). (5) Depend on `ade_runtime` (`ci_check_dependency_boundary.sh`). (6) Use signing primitives or add `#[cfg(feature)]` gates. (7) Omit the `// Core Contract:` banner. (8) **B1 — `consensus_view` (DC-CONSENSUS-02, CN-EPOCH-01):** must not rederive a stake snapshot; the projection reads the **set** snapshot (not mark/go) from a loaded `LedgerState` and must use `BTreeMap` only — any `HashMap`/iteration-order leak would surface in BLUE consumer behavior. (9) **B1 — `block_validity` (DC-VAL-02/03):** must not produce a `Valid` verdict while skipping either the header authority or the body authority; the header is validated **before** the body and the body authority is unreachable on a header-invalid block (fail-fast ordering); the body-hash binding (header `body_hash` ↔ block body) is mandatory. (10) **B1 — (DC-VAL-05):** the invalid path must return the unchanged input states plus a structured `BlockValidityError`; no partial/in-place mutation on the invalid path. (11) **B1 — (DC-VAL-06):** every crypto-input / field-size / structural check rejects on wrong size or shape and never silently skips; `if X.len() == K { check } else { skip }` is forbidden; reason variants are structured `BlockValidityError`, never TODO/placeholder. (12) **B1 — no "trust the body / skip header" shortcut** may leak into the authoritative verdict (DC-VAL-02). |
| **Inbound deps** | `ade_testkit`. |
| **Outbound deps** | `ade_types`, `ade_crypto`, `ade_codec`, `ade_plutus`, **`ade_core` *(NEW B1)***, `minicbor`, `num-bigint`, `num-integer`, `num-traits`. No external runtime / I/O crates. Dev-dep: `ade_testkit`. |
| **Entry points** | `ade_ledger::rules::apply_block` (8 import sites), `apply_block_classified`, `apply_block_with_accounting`, `apply_block_with_verdicts`, `apply_epoch_boundary_full`, `apply_epoch_boundary_with_registrations`, `ade_ledger::state::{LedgerState, EpochState}`, `ade_ledger::pparams::ProtocolParameters`, `ade_ledger::utxo::UTxOState`, `ade_ledger::hfc::translate_era`, `ade_ledger::fingerprint::fingerprint`, `ade_ledger::governance::{evaluate_ratification, enact_proposals, expire_proposals}`, `ade_ledger::delegation::{apply_cert, apply_certs}`, `ade_ledger::epoch::{rotate_snapshots, compute_rewards, retire_pools, apply_epoch_boundary}`, `ade_ledger::witness::{decode_all_plutus_scripts_in_block, decode_witness_infos}`. **NEW B1:** `ade_ledger::block_validity::{block_validity, BlockValidityVerdict, BlockRejectClass, BlockValidityError, encode_verdict_surface, decode_verdict_surface, decode_block}`, `ade_ledger::consensus_view::{PoolDistrView, PoolEntry}`. |
| **Key modules** | `byron.rs`, `shelley.rs`, `mary.rs`, `alonzo.rs`, `babbage.rs`, `conway.rs` (per-era composers), `rules.rs` (top-level `apply_block*`), `state.rs`, `utxo.rs`, `delegation.rs`, `epoch.rs`, `governance.rs`, `hfc.rs`, `late_era_validation.rs`, `phase.rs`, `plutus_eval.rs`, `pparams.rs`, `rational.rs`, `scripts.rs`, `value.rs`, `witness.rs`, `fingerprint.rs`, `error.rs`. **NEW B1:** `consensus_view.rs` (BLUE `PoolDistrView` projection), `block_validity/{mod.rs, verdict.rs, encoding.rs, transition.rs, header_input.rs}` (BLUE), `consensus_input_extract.rs` (**RED** — see note). |

> **RED sub-classification (`ade_ledger::consensus_input_extract`).** This module is physically inside the BLUE `ade_ledger` crate but is **RED** per its own module doc-comment and the PHASE4-B1 TCB Color Map: it tail-scans an external UTxO-HD `utxohd-mem` `ExtLedgerState` CBOR dump for the five `PraosState` nonces. It is classified RED because it parses an *external dump format* rather than an authoritative canonical Cardano type. The scan itself is pure and fail-closed (exactly five non-neutral nonces or a hard `NonceScanError`). Its public types are `Nonce`, `PraosNonces`, `NonceScanError`. **Enforcement note:** the BLUE/RED partition inside `ade_ledger` is *by module*, not by crate; no dedicated CI script enforces that `consensus_input_extract` stays out of the BLUE authority path. It is reached only by the RED snapshot loader and the GREEN corpus harness, never by `block_validity` — but this is currently a review-and-doc invariant, not a mechanical one. Surfaced as a gap.

> **Gap surfaced — DC-VAL-06 is test-enforced, not CI-enforced.** The cluster doc's forward-looking engineering surface named `ci/ci_check_no_fail_open_in_validation.sh` (a grep gate against the `if len == K { check } else { skip }` shape). That script was **not** shipped; the CI directory still holds 25 scripts. DC-VAL-01..06 are enforced by the named `cargo test` targets recorded in the registry (`expect_size_rejects_wrong_length`, `header_before_body_fail_fast`, `invalid_block_leaves_state_unchanged`, `no_mutation_is_ever_valid`, `each_mutation_maps_to_expected_class`, `all_corpus_blocks_valid`, `verdict_stream_replays_identically`, etc.), not by a grep gate. The `expect_size` helper structurally prevents the fail-open shape *within* `kes_check`, but nothing mechanically forbids a future author from reintroducing the `if len == K {…} else {…}` pattern elsewhere in the validation path. Promoting DC-VAL-06 from "test-enforced" to "grep-gated" is the open enforcement item from B1.

> **Documented Conway body-witness gap (B2 concern).** `ade_ledger::rules::apply_block_with_verdicts` does **not** verify Ed25519 vkey witnesses on the **Conway** body path. Shelley/Allegra/Mary/Alonzo/Babbage bodies run the fail-closed `verify_ed25519` path (`shelley.rs:204-221`) and Byron bootstrap witnesses are verified (`byron.rs:231-238`); the Conway dispatch in `rules.rs` reuses the Shelley-era block applicator for structural/required-signer checks but does not re-run full vkey-witness signature verification for Conway transactions. This gap is **unreachable via `block_validity`** at HEAD — the B1 positive corpus is real on-chain Conway-576 blocks (whose witnesses are valid by construction), and the adversarial corpus's witness-forgery mutator (M6) patches the header `body_hash` to reach body validation, but the verdict still rejects via the header/body-hash binding and the structural checks before a Conway-specific vkey check would be needed. It becomes a live correctness concern for **PHASE4-B2 (full transaction validity)**, where adversarial Conway transactions with fabricated vkey witnesses must be rejected on the body path itself. Recorded here so B2 opens with this as a known proof obligation, not a surprise. |

---

### `ade_network` *(BLUE submodules)*

> Per-submodule BLUE scoping per `.idd-config.json` `_core_paths_doc`. The crate is split below into its BLUE half (this entry) and its RED half (further down). Two submodules — `lib.rs` (the barrel) and `mux/mod.rs` (a re-export shim) — are GREEN and carry no authority. Slice provenance: cluster PHASE4-N-A, slices S-A1 through S-A10.

| Attribute | Value |
|-----------|-------|
| **Purpose** | Owns the Cardano Ouroboros mini-protocol authority — the closed wire grammar (CBOR codecs) and pure state machines for all 11 N2N + N2C mini-protocols, plus the BLUE Ouroboros mux frame primitive. Encoders/decoders and transitions only; bytes-on-the-wire and effect emission belong to the RED half of the crate. |
| **Creates** | **Wire grammar (`codec/`):** `HandshakeMessage`, `N2cHandshakeMessage`, `ChainSyncMessage`, `BlockFetchMessage`, `TxSubmission2Message`, `KeepAliveMessage`, `PeerSharingMessage`, `LocalChainSyncMessage`, `LocalTxSubmissionMessage`, `LocalStateQueryMessage`, `LocalTxMonitorMessage`; closed payload types `VersionTable`/`VersionParams`/`N2cVersionTable`/`N2cVersionParams`, `RefuseReason`/`N2cRefuseReason`, `Point`, `Tip`, `Range`, `TxIdAndSize`, `TxAcceptance`, `TxRejection`, `KeepAliveCookie`, `PeerAddress`, `AcquireFailure`, `QueryPayload`, `ResultPayload`, `MempoolMeasures`/`MempoolSizeAndCapacity`/`MeasureSizeAndCapacity`/`MeasureName`; structured error taxonomy `CodecError` + `ProtocolKind`; typed per-protocol version newtypes `N2NVersion`, `N2CVersion`, `ChainSyncVersion`, `BlockFetchVersion`, `TxSubmission2Version`, `KeepAliveVersion`, `PeerSharingVersion`, `LocalChainSyncVersion`, `LocalTxSubmissionVersion`, `LocalStateQueryVersion`, `LocalTxMonitorVersion`. **State machines:** one closed `*State` / `*Agency` / `*Output` / `*Error` quad per protocol plus event/value carriers `ForkChoiceSignal`, `BatchDeliveryEvent`, `InventoryEvent`, `KeepAliveEvent`, `PeerSharingEvent`, `LocalChainSyncEvent`, `LocalTxSubmissionEvent`, `LocalStateQueryEvent`, `LocalTxMonitorEvent`, `BusyKind`, and handshake-specific `VersionData`, `N2cVersionData`, `PeerSharingFlag`, `SelectionOutcome<V, D>`, `N2nHandshakeOutput`, `N2cHandshakeOutput`. **Mux primitives (`mux/frame.rs`):** `MuxFrame`, `MuxHeader`, `MuxMode`, `MiniProtocolId`, `MuxError`. |
| **Interprets** | The on-wire CBOR of every Ouroboros mini-protocol message (closed grammar) and the 8-byte Ouroboros mux frame header. Block, header, and tx-body bytes inside RollForward / Block / Tx are *not* interpreted at this layer (opaque pass-through per DC-PROTO-06); LSQ Query/Result payloads are also opaque. The codec verifies CBOR well-formedness around opaque blobs but never decodes them. |
| **MUST NOT** | (1) Contain `async fn`, `.await`, `tokio`, `async_std`, `futures::`, async-runtime spawn, or async timers in any file under the 9 BLUE paths (`ci_check_no_async_in_blue.sh`; DC-CORE-01). (2) Decode block / header / tx-body bytes — opaque `Vec<u8>` (DC-PROTO-06). (3) Construct a `PreservedCbor` outside `ade_codec`'s named chokepoints. (4) Add `#[non_exhaustive]` to any wire-message enum, introduce a generic `Codec<P>` trait, or use `dyn MessageHandler` (DC-PROTO-03/04). (5) Define a generic `Agency<P>` wrapper. (6) Read a selected version from ambient session state — explicit input to every transition (DC-PROTO-06). (7) Redefine `TxId` / `SlotNo` / `Hash32` / `CardanoEra` — use `ade_types`. (8) Depend on `pallas-network`. (9) Hold `String` in any error variant. (10) Use any other BLUE forbidden pattern. |
| **Inbound deps** | `ade_core_interop` (live consensus session binary, RED → RED). At the library level no BLUE crate imports `ade_network`. The 7 capture binaries import the BLUE codecs internally; the 19 integration tests exercise the BLUE surface end-to-end. |
| **Outbound deps** | `ade_types`, `ade_codec` (for `CodecError::MalformedCbor`). No external deps in the BLUE submodules; the `tokio` line in `Cargo.toml` is confined to `mux/transport.rs` and the capture binaries (RED). |
| **Entry points** | **Codec** (11 protocols, each `encode_<protocol>_message` / `decode_<protocol>_message`): `ade_network::codec::{handshake,n2c_handshake,chain_sync,block_fetch,tx_submission,keep_alive,peer_sharing,local_chain_sync,local_tx_submission,local_state_query,local_tx_monitor}::{encode_*_message, decode_*_message}`; `ade_network::codec::primitives::*` (8 import sites); `ade_network::codec::version::*`; `ade_network::codec::{CodecError, ProtocolKind}`. **State machines:** `ade_network::handshake::{n2n_transition, n2c_transition, select_n2n_version, select_n2c_version, N2N_SUPPORTED, N2C_SUPPORTED}`; per-protocol `*_transition` functions. **Mux:** `ade_network::mux::frame::{encode_frame, decode_frame, MuxFrame, MuxHeader, MuxMode, MiniProtocolId, MuxError, HEADER_LEN, MAX_PAYLOAD}` (8 import sites — the most-reached BLUE surface in the crate). |
| **Key modules** | `codec/` (11 message-codec modules + `primitives.rs` + `version.rs` + `error.rs`); `handshake/`, `chain_sync/`, `block_fetch/`, `tx_submission/`, `keep_alive/`, `peer_sharing/`; `n2c/` (4 local-* submodules); `mux/frame.rs`. |

---

### `ade_plutus`

| Attribute | Value |
|-----------|-------|
| **Purpose** | Quarantine boundary between the Ade-canonical ledger and the ported UPLC evaluator from `aiken-lang/aiken` (pinned to tag `v1.1.21`, commit `42babe5d`). Exposes Ade-canonical types only; aiken- and pallas-originated types are strictly internal. |
| **Creates** | `PlutusScript`, `PlutusLanguage`, `EvalOutput`, `PlutusError`, `CostModels`, `DecoderMode`, `PerScriptResult`, `TxEvalResult`. |
| **Interprets** | UPLC scripts (Plutus V1/V2/V3) and `CostModels` CBOR. Phase-two transaction evaluation. `PlutusScript::from_cbor` is a named ingress chokepoint, allowlisted in `ci_check_ingress_chokepoints.sh` Check 3. |
| **MUST NOT** | (1) Re-export any `pallas_*` or `aiken_uplc::` type (`ci_check_pallas_quarantine.sh`). (2) Allow another BLUE crate to bypass the canonical entry. (3) Activate PV11 builtins (`ExpModInteger`, `CaseList`, `CaseData`). (4) Use any BLUE-forbidden pattern (shared header). (5) Construct `PreservedCbor` outside `ade_codec` — `PlutusScript::from_cbor` produces `PlutusScript`, not `PreservedCbor`. |
| **Inbound deps** | `ade_ledger`, `ade_testkit`. |
| **Outbound deps** | `ade_types`, `ade_crypto`, `ade_codec`, `aiken_uplc` (git, tag `v1.1.21`), `pallas-primitives` (internal-only). |
| **Entry points** | `ade_plutus::eval_tx_phase_two`, `ade_plutus::tx_eval::{SlotConfig, MAINNET_SLOT_CONFIG}`, `ade_plutus::evaluator::{programs_alpha_equivalent, EvalOutput, PlutusLanguage, PlutusScript}`, `ade_plutus::cost_model::{CostModels, DecoderMode, decode_cost_models}`. |
| **Key modules** | `evaluator.rs`, `cost_model.rs`, `script_context.rs`, `script_verdict.rs`, `tx_eval.rs`. |

---

### `ade_types`

| Attribute | Value |
|-----------|-------|
| **Purpose** | Owns the canonical Cardano domain vocabulary — primitives, era enum, address forms, era-tagged transaction bodies / outputs / certificates, governance types — used by every other workspace crate as the lingua franca. The schema, separated from the codec. |
| **Creates** | `CardanoEra`, `SlotNo`, `BlockNo`, `EpochNo`, `Hash28`, `Hash32`, `Coin`, `Lovelace`, `NetworkId`, `Nonce`, `TxIn`, `PoolId`, `Address`, `ByronAddress`, `Credential`, `Certificate`, `ConwayCert`, `MIRCert`, `MIRPot`, `DRep`, `GovAction`, `GovActionId`, `Anchor`, `OperationalCert`, `NativeScript`, `PlutusV1Script`, `Datum`, `DatumOption`, `MultiAsset`, `AssetName`, `CostModel`, `ExUnits`, plus per-era tx-body / tx-out / witness wrappers and Byron era block headers. |
| **Interprets** | None — produce-only. Domain types are constructed by `ade_codec` decoders or `ade_ledger` rules. |
| **MUST NOT** | (1) Construct or decode `PreservedCbor`. (2) Use any BLUE-forbidden pattern. (3) Depend on any workspace crate — root of the dependency DAG, no `[dependencies]` block. (4) Add open/extensible variants to closed era / certificate / governance enums without a versioned gate. |
| **Inbound deps** | `ade_codec`, `ade_crypto`, `ade_ledger`, `ade_plutus`, `ade_runtime`, `ade_testkit`, `ade_network`, `ade_core`, `ade_core_interop`. |
| **Outbound deps** | None. |
| **Entry points** | `ade_types::CardanoEra` (28 import sites — most-used name in the workspace), `ade_types::tx::{Coin, TxIn}` (13), `ade_types::{Hash32, SlotNo}` (13), `ade_types::tx::Coin` (10), `ade_types::primitives::{Hash32, SlotNo}` (8), `ade_types::Hash28` (8), `ade_types::{CardanoEra, EpochNo, SlotNo}` (7), `ade_types::tx::TxIn` (6). |
| **Key modules** | `primitives.rs`, `era.rs`, `tx.rs`, `address/`, `byron/`, `shelley/`, `allegra/`, `mary/`, `alonzo/`, `babbage/`, `conway/`. |

---

## GREEN Modules — Deterministic Glue

> Deterministic, non-authoritative. May depend on BLUE; must not affect authoritative outputs.

### `ade_testkit`

> **Status change in PHASE4-N-B / PHASE4-B1.** `ade_testkit` depends on `ade_runtime` in `[dependencies]` (used by `consensus::stream_replay`). **B1 added** a new GREEN harness sub-tree `validity/` (positive + adversarial block-validity corpus loaders, a production-`PoolDistrView` builder for the acceptance tests, a replay harness driving the BLUE `block_validity` over the Conway-576 corpus, and deterministic block mutators). The RED snapshot loader was extended in B1 to un-skip the pool VRF field. The crate remains GREEN.

| Attribute | Value |
|-----------|-------|
| **Purpose** | Test infrastructure: differential harness, oracle snapshot loader, regression corpus, genesis loader, era mapping, transcript replay, diff reporting; PHASE4-N-B consensus harness; **PHASE4-B1 block-validity harness** (`validity/`). Not authoritative — produces comparisons of authoritative outputs against oracle data and serves the `replay_cmd` test suites. |
| **Creates** | **Harness (pre-N-B):** `Era`, `OracleManifest`, `OracleHashEntry`, `LoadedSnapshot`, `SnapshotHeader`, `Manifest`, `ManifestEntry`, `RegressionCorpus`, `RegressionEntry`, `ComparisonSurface`, `DifferentialReport`, `DiffReport`, `Divergence`, `DivergenceKind`, `FirstDivergenceReport`, `LedgerDiffReport`, `LedgerDivergence`, `LedgerHashSequence`, `ProtocolDiffReport`, `ProtocolDivergence`, `BlockFields`, `BlockMeta`, `CommitteeState`, `ConwayGovParams`, `AlonzoPlutusParams`, `DRepRegistration`, `DStateProbe`, `DStateFieldInfo`, `ArtifactDigest`, `ExpectedVerdict`, `HarnessError`, `InstantaneousRewardsSummary`, `MiniProtocolId`, `Transcript`, `TranscriptMessage`, `RewardParams`, `ShelleyOracleUtxo`, `ShelleyOracleUtxoEntry`, `ShelleyCompactTxIn`, `StateHash`, `StubBlockDecoder`, `StubLedgerApplicator`, `StubProtocolStateMachine`, `VotingDelegationStats`, plus violation reports `ManifestViolation`/`CorpusViolation`/`ProvenanceViolation`. **Consensus (N-B):** `LedgerViewStub`, `EpochStakeFixture`, `PoolFixture`, `ReplayStep`, `ReplayResult`. **Validity (NEW B1, `validity/`):** `ConwayValidityCorpus` (corpus loader) + `CorpusLoadError`, the production-`PoolDistrView` builder helper, the GREEN replay harness, and the deterministic adversarial mutators (M1–M6). |
| **Interprets** | Oracle reference snapshots, the regression-corpus manifest, the PHASE4-N-B consensus corpus, **and the PHASE4-B1 validity corpus** (`corpus/validity/conway_epoch576/` — 14 real Conway-576 `[era, block]` envelopes across 13 files plus `consensus_inputs.json`; `corpus/validity/adversarial/` — mutator-derived negatives). |
| **MUST NOT** | (1) Affect authoritative outputs — testkit code is excluded from BLUE forbidden-pattern scans and may use `HashMap`/`serde_json`/`flate2`/`tar`, but its results must never feed back into `ade_ledger`/`ade_codec`/`ade_crypto`/`ade_core`/`ade_runtime` state. (2) Be linked from any BLUE crate. (3) Introduce nondeterminism that leaks into stored fixtures — fixtures must be byte-reproducible. (4) `ade_testkit::consensus::ledger_view_stub` must use `BTreeMap` only. (5) The stream-replay driver must report identical event sequences across runs. (6) **B1 — `validity::replay`** must report identical verdict streams across runs (the `verdict_stream_replays_identically` / `adversarial_replays_identically` failure mode); the GREEN `validity` harness must not alter the BLUE `block_validity` verdict — it only loads corpus, builds the production `LedgerView`, drives the BLUE transition, and compares. (7) **B1 — the adversarial mutators** must each apply a single targeted corruption that the BLUE authority is required to reject; `no_mutation_is_ever_valid` is the CE-B1-4 false-accept guard. |
| **Inbound deps** | None at compile time (no non-dev `ade_testkit` dep). All consumption is through its own integration tests and dev-dep links from `ade_core`, `ade_runtime`, `ade_ledger`, `ade_core_interop`. |
| **Outbound deps** | `ade_types`, `ade_codec`, `ade_crypto`, `ade_core`, `ade_ledger`, `ade_plutus`, `ade_runtime`, `blake2`, `flate2`, `tar`, `serde`, `serde_json`, `toml`. |
| **Entry points** | `ade_testkit::harness::snapshot_loader::LoadedSnapshot` (9 import sites), `extract_state_from_tarball` / `parse_*` family, `ade_testkit::harness::genesis_loader::load_genesis_utxo` (3); N-B: `ade_testkit::consensus::ledger_view_stub::{LedgerViewStub, EpochStakeFixture, PoolFixture}`, `ade_testkit::consensus::stream_replay::{replay_stream, ReplayResult}`, `ade_testkit::consensus::corpus::*`. **NEW B1:** `ade_testkit::validity::corpus::ConwayValidityCorpus`, `ade_testkit::validity::ledger_view::*` (production-`PoolDistrView` builder), `ade_testkit::validity::replay::*`, `ade_testkit::validity::adversarial::*` (M1–M6 mutators). |
| **Key modules** | **`harness/`** (pre-N-B): `snapshot_loader.rs` (B1: un-skipped pool VRF field), `genesis_loader.rs`, `shelley_loader.rs`, `era_mapping.rs`, `oracle_manifest.rs`, `regression_corpus.rs`, `provenance.rs`, `transcript.rs`, `block_diff.rs`, `ledger_diff.rs`, `protocol_diff.rs`, `diff_report.rs`, `address_extractor.rs`, `adapters/*`. **`consensus/`** (N-B): `corpus.rs`, `ledger_view_stub.rs`, `stream_replay.rs`. **`validity/`** (NEW B1): `mod.rs`, `corpus.rs` (Conway-576 loader), `ledger_view.rs` (production `PoolDistrView` builder for the acceptance tests), `replay.rs` (GREEN positive-corpus replay over `block_validity`), `adversarial.rs` (deterministic mutators M1–M6). |

> **Classification note.** `ade_testkit` reads files from disk via `flate2`/`tar` and `std::fs` in test helpers and drives BLUE/GREEN authorities from corpus inputs. That is an authoritative-vs-test distinction: I/O exists, but only to materialize oracle/corpus fixtures consumed by tests and to compose deterministic GREEN harnesses for replay equivalence. By IDD doctrine that is GREEN. The crate is "demotable to RED" if/when it grows orchestration code that mutates authoritative state.

---

## RED Modules — Imperative Shell

> I/O, network, storage, clocks, retries. May depend on BLUE/GREEN. Must not modify core state directly or construct semantic types unsafely.

### `ade_core_interop` *(PHASE4-N-B, S-B10)*

| Attribute | Value |
|-----------|-------|
| **Purpose** | Live cardano-node interop driver for the consensus orchestrator. Builds the orchestrator at genesis (`fresh_orchestrator(k, initial_nonce)`), connects to a pinned-Docker cardano-node 10.6.2 peer over N2N via `ade_network`, feeds arriving headers and rollbacks into `ade_runtime::consensus::chain_selector::process_stream_input`, and asserts best-chain agreement for a sustained window. The release-evidence test is `#[ignore]`-gated. |
| **Creates** | One library helper: `ade_core_interop::fresh_orchestrator(k, initial_nonce) -> OrchestratorState`. No new public types — every state shape is BLUE or GREEN. |
| **Interprets** | Live N2N byte streams from a real cardano-node 10.6.2 peer; bytes flow in via `ade_network::mux::transport::MuxTransport` into BLUE codecs / state machines / consensus transitions without re-interpretation. Genesis JSON is not read here at HEAD; an all-zeros placeholder `initial_nonce` is used. |
| **MUST NOT** | (1) Contain authoritative state-transition logic — every transition is BLUE in `ade_core::consensus::*` or GREEN in `ade_runtime::consensus::chain_selector`. (2) Be depended on by any other workspace crate. (3) Run by default in CI — the closure-gate test is `#[ignore]`-gated. (4) Bypass `ade_codec`'s `PreservedCbor` chokepoints if/when blocks are decoded. (5) Read versions out of ambient context (DC-PROTO-06). (6) Vendor cardano-node mainnet genesis blobs into the crate. |
| **Inbound deps** | None. |
| **Outbound deps** | `ade_core`, `ade_runtime`, `ade_network`, `ade_testkit`, `ade_types`, `tokio` (`net`, `rt`, `io-util`, `macros`, `time`, `rt-multi-thread`). |
| **Entry points** | `ade_core_interop::fresh_orchestrator(k, initial_nonce)`, binary `live_consensus_session` (`src/bin/live_consensus_session.rs`). Closure-gate test at `crates/ade_core_interop/tests/live_consensus_session.rs` (`#[ignore]`). |
| **Key modules** | `src/lib.rs` (one helper, `fresh_orchestrator`), `src/bin/live_consensus_session.rs` (operator-run readiness probe). |

> **Gap surfaced.** `ade_core_interop` at HEAD is a *readiness probe* — it constructs the orchestrator and prints a ready line; the full live tip-agreement loop is the operator's manual task. CE-N-B-6 closes via the manual evidence log, not via an automated CI check. Largest unenforced surface introduced by N-B; B1 did not change it.

---

### `ade_network` *(RED submodules + capture binaries)*

> The RED half of the `ade_network` crate: `mux::transport`, `session`, and the 7 `src/bin/capture_*` binaries. Provenance: cluster PHASE4-N-A.

| Attribute | Value |
|-----------|-------|
| **Purpose** | The only place inside `ade_network` where I/O happens — tokio-based TCP / Unix-socket scaffolding for the Ouroboros mux layer (`mux::transport`), the session-composition placeholder (`session`), and 7 capture binaries that perform real handshakes against pinned mainnet / preprod cardano-node 11.0.1 peers and write the recorded byte stream as canonical replay corpus under `corpus/network/`. |
| **Creates** | `MuxTransport` (tokio TcpStream wrapper). `session/mod.rs` is an empty placeholder. The 7 capture binaries produce no public Rust types; their authoritative output is the on-disk byte corpus. |
| **Interprets** | Live TCP / Unix-socket byte streams from cardano-node 11.0.1 peers; bytes flow in via `MuxTransport::read_raw` into the BLUE mux frame decoder and protocol codecs without re-interpretation. |
| **MUST NOT** | (1) Contain protocol logic. (2) Define or modify any closed wire-message enum, agency type, or state. (3) Read a selected protocol version from ambient state and pass it into a BLUE transition implicitly (DC-PROTO-06). (4) Replace the BLUE codec / state machine with a "fast path" (CN-WIRE-07). (5) Bypass `ade_codec`'s `PreservedCbor` chokepoints if/when blocks are decoded. (6) Be depended on by any BLUE submodule of `ade_network`. |
| **Inbound deps** | `ade_core_interop`. At the library level, no BLUE crate imports the RED submodules. |
| **Outbound deps** | `tokio` (features `net`, `rt`, `io-util`, `macros`, `time`), `ade_types`, `ade_codec`, plus the BLUE `ade_network::codec::*` and `ade_network::mux::frame`. |
| **Entry points** | `ade_network::mux::transport::{MuxTransport, open_tcp}`. The 7 capture binaries: `ade_handshake_capture`, `ade_chain_sync_capture`, `ade_block_fetch_capture`, `ade_keep_alive_capture`, `ade_peer_sharing_capture`, `ade_n2c_handshake_capture`, `ade_n2c_protocols_capture`, plus the older `ade_tx_submission2_capture`. `session::*` exposes no entry points at HEAD. |
| **Key modules** | `mux/transport.rs`, `session/mod.rs` (empty placeholder), `bin/capture_*`. |

> **Gap surfaced.** `ade_network::session::mod.rs` is still a placeholder. The session driver remains owed by a future orchestration cluster.

---

### `ade_node`

| Attribute | Value |
|-----------|-------|
| **Purpose** | Binary entry point for the node process. Currently a hello-world stub. Reserved as the assembly point for runtime + ledger + chain DB + network + consensus orchestrator. |
| **Creates** | None. |
| **Interprets** | None yet. |
| **MUST NOT** | (1) Bypass `ade_codec` to construct semantic types from bytes. (2) Modify `ade_ledger` or `ade_core::consensus` state in place — transitions go through `ade_ledger::rules::*`, `ade_ledger::block_validity::*`, or `ade_core::consensus::*` BLUE transitions. (3) Take an inbound dep from any other crate. (4) Read versions out of ambient context and pass them into `ade_network` BLUE transitions implicitly (DC-PROTO-06). (5) Construct an `OrchestratorState` outside `ade_runtime::consensus::chain_selector` or `ade_core_interop::fresh_orchestrator`. |
| **Inbound deps** | None. |
| **Outbound deps** | None at present. |
| **Entry points** | `main()` in `crates/ade_node/src/main.rs`. |

> **Gap surfaced.** `ade_node` is still a hello-world stub; its MUST NOT list is forward-looking with nothing to enforce today.

---

### `ade_runtime`

> **Status change in PHASE4-N-B.** `ade_runtime` hosts storage + recovery (cluster N-D) and the consensus orchestrator (`consensus/`, cluster N-B). The crate remains RED; `chain_selector.rs` / `candidate_fragment.rs` are GREEN by file-level classification, `genesis_parser.rs` is RED. PHASE4-B1 did not change `ade_runtime`.

| Attribute | Value |
|-----------|-------|
| **Purpose** | The imperative shell. Hosts (a) the `ChainDb` / `SnapshotStore` storage abstractions and their in-memory / redb-backed impls, the crash-safety harness, and the snapshot + forward-replay recovery primitive (N-D); (b) the GREEN chain-selector orchestrator + GREEN `CandidateFragment` materializer (N-B); (c) the RED genesis-text parser that materializes the BLUE `EraSchedule` and computes the `BootstrapAnchorHash` (N-B). |
| **Creates** | **ChainDb (`chaindb/`):** `ChainDb` trait, `SnapshotStore` trait, `KillStrategy<D>` trait, `ChainTip`, `StoredBlock`, `ChainDbError`, `InMemoryChainDb`, `PersistentChainDb`, `PersistentChainDbOptions`, `SyncCadence`, `NoKill`, `BlockIter<'a>`. **Recovery (`recovery.rs`):** `Recoverable` trait, `StartingState`, `RecoveryReport<R>`, `RecoveryError<E>`. **Genesis (`consensus/genesis_parser.rs` — RED):** `NetworkMagic`, `GenesisBlob`, `GenesisBundle<'a>`, `GenesisParseError`. **Orchestrator (`consensus/chain_selector.rs` — GREEN):** `StreamInput`, `OrchestratorError`, `OrchestratorState`, `RollbackSnapshot`, `DEFAULT_SNAPSHOT_LIMIT`. **Materializer (`consensus/candidate_fragment.rs` — GREEN):** `build_candidate_fragment`. |
| **Interprets** | Block bytes for content-addressed storage. Snapshot bytes are byte-opaque to chaindb. Genesis JSON for the four blobs → typed `EraSummary` + `system_start_unix_ms` + immutable `EraSchedule`. N-A `StreamInput` events → canonical BLUE input shape threaded through BLUE transitions. |
| **MUST NOT** | (1) Depend on `ade_ledger`. (2) Leak `redb` through the `chaindb::*` public surface. (3) Have a second public chaindb path. (4) Couple snapshots to a specific state type. (5) Apply automatic snapshot pruning. (6) Allow partial-recovery success. (7) Add async recovery surface. (8) Treat "not found" as an error path or silently retry on real errors. (9) **Consensus sub-tree:** the GREEN `chain_selector` must not affect authoritative outputs; no `HashMap`/`HashSet` (`BTreeMap` + `Vec`), no async, no wall-clock, no global mutable state, no redefining a closed BLUE enum. (10) **Genesis parser:** must not be reached by BLUE; structured errors only (no `String` in `GenesisParseError`). (11) **`candidate_fragment` materializer:** must not own state — pure constructor. |
| **Inbound deps** | `ade_testkit` (non-dev dep, used by `stream_replay`), `ade_core_interop`. The binary `chaindb_kill_target` lives at `src/bin/`. |
| **Outbound deps** | `ade_types`, `ade_core` (consensus + genesis materialization), `ade_crypto` (`blake2b_256`), `ade_codec` (canonical CBOR primitives), `redb`, `serde_json`. Dev-deps: `tempfile`, `ade_testkit`, `cardano-crypto` (tests only). |
| **Entry points** | **Chaindb / recovery:** `ade_runtime::chaindb::{ChainDb, SnapshotStore, PersistentChainDb, PersistentChainDbOptions, SyncCadence, InMemoryChainDb, run_contract_tests, run_snapshot_contract_tests, run_crash_safety_tests}`, `ade_runtime::recovery::{recover, Recoverable}`. **Consensus:** `ade_runtime::consensus::chain_selector::{process_stream_input, OrchestratorError, OrchestratorState, RollbackSnapshot, StreamInput, DEFAULT_SNAPSHOT_LIMIT}`, `ade_runtime::consensus::candidate_fragment::build_candidate_fragment`, `ade_runtime::consensus::genesis_parser::{compute_anchor_hash, parse_genesis, GenesisBlob, GenesisBundle, GenesisParseError, NetworkMagic}`. |
| **Key modules** | **`chaindb/`**: `mod.rs`, `contract.rs`, `snapshot_contract.rs`, `crash_safety.rs`, `in_memory.rs`, `persistent.rs`, `types.rs`, `error.rs`. **`recovery.rs`**. **`consensus/`**: `mod.rs`, `genesis_parser.rs` (RED), `candidate_fragment.rs` (GREEN), `chain_selector.rs` (GREEN). **`bin/chaindb_kill_target.rs`**. |
| **Mechanical enforcement** | Three dedicated CI scripts back the chaindb / recovery / crash-safety surface: `ci/ci_check_chaindb_contract.sh`, `ci/ci_check_recovery_contract.sh`, `ci/ci_check_chaindb_crash_safety.sh`. The consensus sub-tree has no dedicated CI script — the four consensus narrow checks all scope `crates/ade_core/src/consensus/` (BLUE), not `crates/ade_runtime/src/consensus/`. |

> **Gap surfaced.** No dedicated CI script enforces the GREEN/RED partition *inside* `ade_runtime::consensus`. At HEAD the partition is "by file" and enforced only by review + header comments. Candidate name (forward): `ci/ci_check_consensus_runtime_partition.sh`.

---

## Cross-Module Rules (project-wide)

### Dependency direction

`ade_core_interop` → `{ade_core, ade_runtime, ade_network, ade_testkit, ade_types, tokio}` is legal (RED leaf binary).
`ade_runtime` → `{ade_core, ade_crypto, ade_codec, ade_types, redb, serde_json}` is legal (RED + GREEN glue).
`ade_testkit` → `{ade_core, ade_ledger, ade_plutus, ade_runtime, ade_crypto, ade_codec, ade_types}` is legal (GREEN).
`ade_network` (BLUE submodules) → `{ade_codec, ade_types}` is legal (BLUE among BLUEs).
`ade_network` (RED submodules + capture bins) → `{tokio, ade_codec, ade_types, ade_network::codec::*, ade_network::mux::frame}` is legal.
**`ade_ledger` → `{ade_core, ade_plutus, ade_crypto, ade_codec, ade_types, minicbor}` is legal (BLUE among BLUEs).** The `ade_ledger → ade_core` edge is **NEW in PHASE4-B1** and is **acyclic**: `ade_core` depends only on `{ade_types, ade_crypto, minicbor}` and does **not** depend on `ade_ledger`. The edge exists so `block_validity` can call `validate_and_apply_header` and the Praos VRF/KES surface, and so `consensus_view::PoolDistrView` can implement the `ade_core::consensus::LedgerView` trait.
`ade_core` → `{ade_types, ade_crypto, minicbor}` is legal (BLUE among BLUEs).
`ade_plutus` → `{ade_crypto, ade_codec, ade_types}` is legal.
`ade_crypto` → `{ade_types}` is legal.
`ade_codec` → `{ade_types}` is legal.
`ade_types` → `{}`.

**Forbidden directions.** Any BLUE crate (or BLUE `ade_network` submodule) depending on `ade_runtime`, `ade_node`, `ade_core_interop`, or the RED half of `ade_network` is a CI failure (`ci_check_dependency_boundary.sh` + `ci_check_no_async_in_blue.sh`). Any non-`ade_plutus` crate referring to `pallas_*` is a CI failure (`ci_check_pallas_quarantine.sh`). Any reference to `ChainDb` / `chain_db` inside `crates/ade_core/src/consensus/` is a CI failure (`ci_check_no_chaindb_in_consensus_blue.sh`). **B1 note:** the `ade_ledger → ade_core` edge passes `ci_check_dependency_boundary.sh` because `ade_core` is BLUE; the acyclicity (no `ade_core → ade_ledger` back-edge) is enforced by the crate graph (Cargo would reject a cycle), not by a dedicated script.

### Naming convention

All crates are prefixed `ade_`. TCB color is not encoded in the crate name. The authoritative classifier is `.idd-config.json` `core_paths` plus the cluster doc TCB Color Maps for sub-crate scopes; CI scripts hard-code their BLUE list. The seven full-BLUE-scoped scripts (`ci_check_module_headers.sh`, `ci_check_no_semantic_cfg.sh`, `ci_check_no_signing_in_blue.sh`, `ci_check_hash_uses_wire_bytes.sh`, `ci_check_ingress_chokepoints.sh`, `ci_check_dependency_boundary.sh`, `ci_check_no_async_in_blue.sh`) use the full BLUE set: the 6 BLUE crates `{ade_codec, ade_types, ade_crypto, ade_core, ade_ledger, ade_plutus}` plus the 9 BLUE `ade_network` submodule paths. Four narrow scripts target `crates/ade_core/src/consensus/`.

### CI enforcement (25 scripts under `ci/`)

| Script | Enforces | Scope |
|---|---|---|
| `ci_check_cbor_round_trip.sh` | T-ENC-03, DC-CBOR-01, DC-CBOR-02 | golden corpus |
| `ci_check_ce_n_a_5_proof.sh` *(PHASE4-N-A, S-A10)* | CE-N-A-5 5-condition evidence | `ade_network` (RED + real-capture corpus) |
| `ci_check_chaindb_contract.sh` | DC-STORE-02, DC-STORE-03, CN-STORE-04, CN-STORE-05 | `ade_runtime --lib chaindb::` (RED) |
| `ci_check_chaindb_crash_safety.sh` | T-REC-01 (crash variant), DC-STORE-01, CN-STORE-03; CE-N-D-1 gate | `ade_runtime --test stress_kill_harness` (RED) |
| `ci_check_consensus_closed_enums.sh` *(PHASE4-N-B, S-B2)* | DC-CONS-04, DC-CONS-10, T-DET-01 | `crates/ade_core/src/consensus/` |
| `ci_check_constitution_coverage.sh` | invariant-registry ↔ code/test coverage | repo-wide |
| `ci_check_crypto_vectors.sh` | crypto KAT regression | `ade_crypto` |
| `ci_check_dependency_boundary.sh` | T-BOUND-02 — BLUE ⇎ RED separation | full BLUE (6 crates + 9 `ade_network` paths) |
| `ci_check_differential_divergence.sh` | DC-DIFF-* | replay outputs |
| `ci_check_forbidden_patterns.sh` | T-CORE-02 — no `HashMap`/floats/clocks/etc. + `unsafe` allowlist | full BLUE |
| `ci_check_hash_uses_wire_bytes.sh` | DC-CBOR-02, T-ENC-01 | full BLUE |
| `ci_check_hfc_translation.sh` | DC-EPOCH-02 (CE-73-semantic) | `ade_ledger::hfc` |
| `ci_check_ingress_chokepoints.sh` | DC-INGRESS-01, T-INGRESS-01 | full BLUE |
| `ci_check_ledger_determinism.sh` | DC-LEDGER-01 (CE-74) | `ade_ledger` |
| `ci_check_module_headers.sh` | CE-04 contract banner | full BLUE |
| `ci_check_no_async_in_blue.sh` *(PHASE4-N-A, S-A1)* | DC-CORE-01 | full BLUE |
| `ci_check_no_chaindb_in_consensus_blue.sh` *(PHASE4-N-B, S-B1)* | DC-CORE-01 + DC-CONS-07 | `crates/ade_core/src/consensus/` |
| `ci_check_no_density_in_fork_choice.sh` *(PHASE4-N-B, S-B8)* | DC-CONS-03 | `fork_choice.rs` + `candidate.rs` |
| `ci_check_no_float_in_consensus.sh` *(PHASE4-N-B, S-B1)* | T-CORE-02 + DC-CONS-07/08/09 | `crates/ade_core/src/consensus/` |
| `ci_check_no_secrets.sh` | no credentials/IPs/keys in tree | repo-wide |
| `ci_check_no_semantic_cfg.sh` | T-BUILD-01 — no `#[cfg(feature)]` | full BLUE |
| `ci_check_no_signing_in_blue.sh` | CE-05, T-KEY-01 — signing in RED only | full BLUE |
| `ci_check_pallas_quarantine.sh` | O-29.2 — `pallas-*` confined to `ade_plutus` | non-`ade_plutus` |
| `ci_check_recovery_contract.sh` | T-REC-01, T-REC-02, DC-STORE-05 | `ade_runtime --lib recovery::` (RED) |
| `ci_check_ref_provenance.sh` | DC-REF-01 — manifest checksum integrity | reference corpus |

> **PHASE4-B1 enforcement note.** B1 added **no new CI script**. `DC-VAL-01..06` flip `declared → enforced` (registry `status = "enforced"`, `strengthened_in = ["PHASE4-B1"]`) but their `ci_script` field is empty — they are enforced by the named `cargo test` targets recorded in each registry entry (run under the existing `cargo test --workspace`), not by a grep gate. The cluster doc's forward-looking `ci_check_no_fail_open_in_validation.sh` (DC-VAL-06 grep gate) was **not** shipped. Three rules carry residual gaps that are documented in the per-module entries: (a) DC-VAL-06's fail-open prohibition is structurally guaranteed only *inside* `kes_check::expect_size`, not grep-gated across the whole validation path; (b) the BLUE/RED partition inside `ade_ledger` (keeping the RED `consensus_input_extract` off the `block_validity` authority path) is review-enforced, not mechanical; (c) the documented Conway body-witness verification gap in `apply_block_with_verdicts` (unreachable via `block_validity` at HEAD; a live PHASE4-B2 proof obligation). These join the pre-existing N-B residual gaps (runtime-consensus partition, CE-N-B-6 manual evidence, unbuilt production `LedgerView` — now partially closed by `consensus_view::PoolDistrView`).
