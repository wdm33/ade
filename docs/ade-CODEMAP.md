# Module Authority Map — Ade

> **Status:** Living architectural document. Regenerated; not hand-edited.
> Per-project instance of `~/.claude/methodology/templates/codemap.md`.

> 11 crates, 446 canonical types, 2033 tests, 99 CI checks at HEAD (`273c887`, PHASE4-N-X closing).

---

## Conventions

- A **module** in Ade is a Cargo workspace crate (smallest independently-buildable unit). Source: `Cargo.toml` `[workspace] members` (11 entries). One exception to crate-granularity: `ade_network` is split by *submodule color* — `.idd-config.json` `core_paths` resolves BLUE at the submodule path level (9 paths), so its BLUE / GREEN / RED submodules are documented as separate entries.
- Several BLUE crates host sub-trees that carry a *different* TCB color by their own module banner and cluster TCB Color Map; and the RED `ade_runtime` crate hosts numerous **GREEN-by-content** sub-trees. These are surfaced as sub-classification notes inside the owning crate's entry (or, where load-bearing, as their own GREEN/RED entries below).
- Modules are listed by TCB color (BLUE → GREEN → RED), alphabetical within each color.

### TCB color sources (in order of authority)

1. `.idd-config.json` `core_paths` — substring match against absolute path. BLUE crate prefixes: `ade_codec`, `ade_types`, `ade_crypto`, `ade_core`, `ade_ledger`, `ade_plutus`. Plus the 9 BLUE `ade_network` submodule paths: `mux/frame.rs`, `codec/`, `handshake/`, `chain_sync/`, `block_fetch/`, `tx_submission/`, `keep_alive/`, `peer_sharing/`, `n2c/`.
2. `.idd-config.json` `_core_paths_doc` — `ade_runtime` is RED; `ade_testkit` is GREEN; `ade_node` is RED; `ade_network::mux::transport` + `ade_network::session` are nominally RED (but `session/` is GREEN-by-content per PHASE4-N-L). `ade_core_interop` is RED.
3. **Module doc-comment banners** — every BLUE/GREEN source file opens with the `// Core Contract:` banner followed by `//! BLUE …` / `//! GREEN …` / `//! RED …`. This is the per-file authority that distinguishes GREEN-by-content from RED inside `ade_runtime`. Verified directly against the tree at HEAD for every new/changed module in this regeneration.
4. Cluster TCB Color Maps under `docs/clusters/` and `docs/clusters/completed/` — §5 "TCB color map (FC/IS partition)" of each `cluster.md`.
5. The invariant registry `docs/ade-invariant-registry.toml` (292 rules) — each rule's `code_locus` + `statement` is the authoritative source for the **MUST NOT** rows below.

### Active cluster at HEAD

**PHASE4-N-X — closing now (N2N Tag-24 Wire Envelope Authority).** This cluster establishes the single workspace authority for the N2N tag-24 CBOR-in-CBOR wire envelope. It introduces the NEW BLUE module `ade_codec::cbor::tag24` — `wrap_tag24(inner) -> Vec<u8>`, `unwrap_tag24(wire) -> Result<&[u8], TagEnvelopeError>`, and the closed `TagEnvelopeError` enum (`NotTag24`, `NotByteString`, `Truncated`, `TrailingBytes`) — re-exported at the `ade_codec` crate root. Protocol-specific composition lives in the `ade_network` BLUE codecs: a served **BlockFetch** `MsgBlock` payload is `tag24(bytes([era, block]))` (era **inside** the wrap; `compose_blockfetch_block` / `decompose_blockfetch_block`, EBB-aware era index, Conway = storage index 7); a served **ChainSync** `RollForward` header is `[era_tag, tag24(bytes(header_cbor))]` (era_tag **outside** the wrap; `compose_rollforward_header` / `decompose_rollforward_header` + `chain_sync_wire_era_index`, the CONSENSUS era index Conway = 6 = storage − 1, deliberately different from block-fetch). The serve reducers (`block_fetch::server`, `chain_sync::server`) now emit composed (tag-24-wrapped) bytes, and `HeaderProjection` gained an `era` field. The RED hand-rolled tag-24 parses were deleted and migrated onto the shared authority (`ade_node::admission::runner` and `ade_core_interop::follow` now call `ade_codec::unwrap_tag24`). `ade_codec::cbor::mod` (`read_bytes`/`read_text`/`skip_item`) gained a fail-closed `checked_add` overflow guard on the declared length argument (behavior change for malformed input only — no panic). It flips/adds registry rule **CN-WIRE-08** (introduced enforced) and ships one new CI gate (`ci_check_tag24_wire_authority.sh`); **CN-FORGE-03 / DC-CONS-17 / DC-CONS-18** carry an N-X strengthening. A real Conway `RollForward` golden fixture is committed under `corpus/network/n2n/chain_sync/preprod_conway_rollforward_*`.

**Cluster-doc location.** Every closed cluster doc is archived under `docs/clusters/completed/` — including the entire **PHASE4-N-Q / N-R-* / N-S-*** set, the **N-M-\*** (admission/seed/WAL/anchor) sub-trees, **N-O**, **N-P**, **N-T**, **N-V**, and now **N-W**. The **only** cluster directory living outside `completed/` is the now-closing **PHASE4-N-X** (`cluster.md` + `CLOSURE.md`, not yet archived; it is archived after the grounding docs regenerate).

This regeneration is a **full inventory at HEAD `273c887`**. The N-M-* admission/seed/WAL/anchor sub-trees, the N-Q/N-R/N-S producer/network/node sub-trees, the N-W Praos-VRF authority surface, and the new N-X tag-24 wire-envelope authority are all inventoried below.

### Counts (mechanical, with sources)

| Count | Value | Source / command |
|---|---|---|
| Crates | **11** | `Cargo.toml` `[workspace] members`. |
| Canonical types | **446** | `grep -rhE "^(pub )?(struct\|enum) " --include='*.rs'` over the 6 BLUE crate `src/` trees + the 9 BLUE `ade_network` submodule paths. Breakdown: `ade_codec` 11, `ade_types` 81, `ade_crypto` 21, `ade_core` 49, `ade_ledger` 167, `ade_plutus` 8 (= 337) + `ade_network` BLUE submodules 109 (`mux/frame.rs` 5, `codec/` 38, `handshake/` 9, `chain_sync/` 11, `block_fetch/` 10, `tx_submission/` 5, `keep_alive/` 5, `peer_sharing/` 5, `n2c/` 21). `canonical_type_registry: null`, so the structural grep is authoritative. **Δ vs prior CODEMAP (445): +1 in `ade_codec` (10→11) from the new closed `TagEnvelopeError` enum (N-X).** |
| Tests | **2033** | `grep -rEc "#\[test\]\|#\[tokio::test\]" --include='*.rs' crates/`, summed. Approximate per the template fallback rule (count of attributes, not a runner enumeration). Per-crate: `ade_codec` 175, `ade_types` 24, `ade_crypto` 86, `ade_core` 135, `ade_ledger` 600, `ade_plutus` 28, `ade_testkit` 312, `ade_runtime` 330, `ade_network` 243, `ade_node` 73, `ade_core_interop` 27. **Δ vs prior (2016): +17 (ade_codec 164→175 = +11 tag-24 wrap/unwrap + overflow-guard tests; ade_network 237→243 = +6 compose/decompose + served-shape oracle tests).** |
| CI checks | **99** | `ls ci/ci_check_*.sh \| wc -l`. No `.github/workflows/` in this repo; `ci_dirs = ["ci"]`. **Δ vs prior (98): +1 — `ci_check_tag24_wire_authority.sh` (N-X, CN-WIRE-08).** |
| Registry rules | **292** | `grep -cE "^id = " docs/ade-invariant-registry.toml`. Reference only — not a header count. **Δ vs prior (291): +1 — the new **CN-WIRE-08** (introduced enforced in N-X). N-X also strengthened **CN-FORGE-03**, **DC-CONS-17**, **DC-CONS-18** (`strengthened_in += "PHASE4-N-X"`).** |

---

## BLUE Modules — Pure Functional Core

> **Shared header (applies to every BLUE entry below).** Every `.rs` source file begins with the `// Core Contract:` banner and the following deny attributes are present in each crate's `lib.rs` (or, for `ade_network`, at the crate root — BLUE submodules inherit them):
> `#![deny(unsafe_code)]`, `#![deny(clippy::unwrap_used)]`, `#![deny(clippy::expect_used)]`, `#![deny(clippy::panic)]`, `#![deny(clippy::float_arithmetic)]`.
>
> Cross-cutting BLUE CI gates (all scope the full BLUE set — 6 BLUE crates + 9 BLUE `ade_network` submodule paths):
> - `ci_check_module_headers.sh` — banner first-line check.
> - `ci_check_forbidden_patterns.sh` — `HashMap`/`HashSet`/`IndexMap`/`indexmap::`, `SystemTime`, `Instant`, `std::fs`, `std::net`, `tokio`, `async fn`, `f32`/`f64`, `anyhow`, `rand::thread_rng`, `thread::spawn`, `unsafe` outside an allowlist.
> - `ci_check_dependency_boundary.sh` — no BLUE crate depends on a RED crate.
> - `ci_check_no_signing_in_blue.sh` — signing operations forbidden in BLUE (signing-key *types* in `ade_crypto::kes_sum` are permitted; signing *operations* are RED-confined in `ade_runtime::producer::signing`).
> - `ci_check_no_semantic_cfg.sh` — `#[cfg(feature = …)]` / `cfg!(feature = …)` forbidden in BLUE.
> - `ci_check_hash_uses_wire_bytes.sh` — no hashing of re-encoded bytes in BLUE.
> - `ci_check_ingress_chokepoints.sh` — only named `decode_*` chokepoints construct `PreservedCbor`.
> - `ci_check_pallas_quarantine.sh` — `pallas-*` confined to `ade_plutus`.
> - `ci_check_no_async_in_blue.sh` — async constructs forbidden in BLUE (DC-CORE-01).
>
> Narrow BLUE gates: `ci_check_no_chaindb_in_consensus_blue.sh`, `ci_check_no_float_in_consensus.sh`, `ci_check_consensus_closed_enums.sh`, `ci_check_no_density_in_fork_choice.sh` (DC-CONS-03), `ci_check_deposit_param_authority.sh` (DC-TXV-07), `ci_check_gov_cert_accumulation_closed.sh` (DC-LEDGER-09), `ci_check_credential_discriminant_closed.sh` (DC-LEDGER-10), `ci_check_proposal_procedures_closed.sh` (DC-LEDGER-11), `ci_check_conway_cert_classification_closed.sh`, the N-C producer-authority gate set (`ci_check_forge_purity.sh`, `ci_check_no_producer_body_encoder.sh`, `ci_check_opcert_closed.sh`, `ci_check_scheduler_closure.sh`, `ci_check_self_accept_gate.sh`), the N-G server-role gate set (`ci_check_served_chain_closure.sh`, `ci_check_chain_sync_server_closure.sh`, `ci_check_block_fetch_server_closure.sh`, `ci_check_no_parallel_header_splitter.sh`), the N-H receive gate set (`ci_check_admitted_block_closure.sh`, `ci_check_receive_reducer_closure.sh`, `ci_check_receive_replay_purity.sh`), the N-I/N-J rollback+snapshot gate set (`ci_check_rollback_materialize_closure.sh`, `ci_check_snapshot_cadence_purity.sh`, `ci_check_snapshot_encoder_closure.sh`), the mempool-ingress gate set (`ci_check_mempool_ingress_closure.sh`, `ci_check_mempool_ingress_replay.sh`), the N-P KES gate (`ci_check_kes_sum_compatibility.sh`), the N-R/N-S/N-V BLUE gates (`ci_check_leader_check_authority.sh`, `ci_check_unsigned_header_preimage_single_source.sh`, `ci_check_no_independent_forge_codepath.sh`, `ci_check_forge_decode_round_trip.sh`), the N-W gate `ci_check_producer_praos_vrf.sh` (CN-FORGE-04 — single era→leader-VRF-input authority), and the **NEW N-X gate** `ci_check_tag24_wire_authority.sh` (CN-WIRE-08 — single tag-24 wrap/unwrap authority + per-protocol composition + no hand-rolled tag-24 parse in RED).

---

### `ade_codec`

| Attribute | Value |
|-----------|-------|
| **Purpose** | Owns Cardano-canonical CBOR ingress — the only place in the workspace that turns raw bytes into typed semantic values, with wire-byte preservation for every hash-bearing structure. Owns the standalone opcert byte authority and the canonical Conway-tx preserved-byte splitter. Owns the canonical block-envelope **encoder** `cbor::envelope::encode_block_envelope` (N-V), symmetric to the long-standing `decode_block_envelope`. **NEW (N-X):** owns the single workspace **tag-24 CBOR-in-CBOR wire-envelope authority** `cbor::tag24::{wrap_tag24, unwrap_tag24}` + closed `TagEnvelopeError`, re-exported at the crate root. |
| **Creates** | `PreservedCbor<T>`, `RawCbor`, `BlockEnvelope`, `ByronDecodedBlock`, `CodecContext`, `CodecError` (incl. `UnknownCertTag`, `DuplicateMapKey`, `TrailingBytes`, `InvalidCborStructure`), `ContainerEncoding`, `IntWidth`, era-tagged block/tx wrappers under `byron/…conway/`, `OpCertCodecError`, `TxComponents<'a>`. **NEW (N-X):** `TagEnvelopeError` — the closed tag-24 unwrap error enum (`NotTag24 { first_byte }`, `NotByteString { offset }`, `Truncated { offset, needed }`, `TrailingBytes { consumed, total }`). Functions: `conway::cert::{decode_conway_certs, decode_drep}`, `conway::withdrawals::{decode_withdrawals, withdrawals_sum}`, `conway::governance::{decode_proposal_procedures, encode_proposal_procedures}`, `shelley::opcert::{encode_opcert, decode_opcert}`, `shelley::tx_components::split_conway_tx_components`, `cbor::envelope::{decode_block_envelope, encode_block_envelope}`, **NEW** `cbor::tag24::{wrap_tag24, unwrap_tag24}`. |
| **Interprets** | All canonical Cardano CBOR — block envelopes (`[era, block]`), era-specific blocks, tx bodies/outs, certificates, addresses. Conway cert array (closed over CDDL tags 0..18) and withdrawals map (dedup). Sole authority for `PreservedCbor::new` (`pub(crate)`). CIP-1694 `proposal_procedure`. cardano-cli `OperationalCertificate` 4-tuple. **NEW (N-X):** `unwrap_tag24` is the single workspace authority that strips a tag-24 (`0xd8 0x18`) CBOR byte-string envelope, returning a **zero-copy borrow** of the verbatim inner bytes (no re-encode) and failing closed with a typed `TagEnvelopeError` on a wrong marker, non-byte-string payload, truncated inner, or trailing bytes. `cbor::mod::{read_bytes, read_text, skip_item}` now fail-close (no panic) on an overflowing declared length via `checked_add`. |
| **MUST NOT** | (1) Construct `PreservedCbor` outside `ade_codec` (`pub(crate)` + `ci_check_ingress_chokepoints.sh`). (2) Re-encode wire bytes when computing hashes (`ci_check_hash_uses_wire_bytes.sh`). (3) Use any forbidden BLUE pattern. (4) Depend on any workspace crate except `ade_types`. (5) `conway::cert` (DC-LEDGER-08) — no unknown-tag swallow; owner-complete; no catch-all. (6) `conway::withdrawals` — no last-wins on duplicate `RewardAccount`. (7) `decode_stake_credential` (DC-LEDGER-10) — must not erase the credential tag. (8) `conway::governance` (DC-LEDGER-11) — no silent skip on unknown `GovAction`. (9) `shelley::opcert` (DC-CONS-11/12) — cardano-byte-identical 4-tuple. (10) `shelley::tx_components` (DC-CONS-13/16) — preserved-byte slices that alias the input. (11) `encode_block_envelope` (CN-FORGE-03, strengthened N-X) — the workspace's **single** block-envelope encoder; emits the era-tagged `[era, block]` form (Conway = discriminant 7); must re-encode a corpus block byte-identically and must round-trip through `decode_block_envelope`; no second/parallel block serializer is permitted (`ci_check_forge_decode_round_trip.sh`, `ci_check_no_independent_forge_codepath.sh`). **(12) NEW — `cbor::tag24` (CN-WIRE-08):** `wrap_tag24` / `unwrap_tag24` MUST each be defined **exactly once** (the single workspace tag-24 authority); `unwrap_tag24` MUST fail closed (typed `TagEnvelopeError`, never panic) on a non-`(0xd8 0x18)` marker, a non-byte-string inner, a wrong inner length, or trailing bytes; the inner bytes MUST be copied verbatim / returned as a zero-copy borrow (never re-encoded); `read_bytes`/`read_text`/`skip_item` MUST reject an overflowing declared length without panic (`ci_check_tag24_wire_authority.sh`). |
| **Inbound deps** | `ade_ledger` (heavy), `ade_plutus`, `ade_testkit`, `ade_network`, `ade_runtime`, `ade_core_interop`, `ade_node`. |
| **Outbound deps** | `ade_types`. std-only; dev-deps `serde_json`, `toml`. |
| **Entry points** | `ade_codec::cbor::envelope::{decode_block_envelope, encode_block_envelope}`, `ade_codec::{wrap_tag24, unwrap_tag24, TagEnvelopeError}` (crate-root re-exports), `ade_codec::traits::AdeEncode`, `ade_codec::CodecContext`, per-era `decode_*_block`, `ade_codec::address::decode_address`, `ade_codec::conway::tx::decode_conway_tx_body`, `ade_codec::conway::cert::decode_conway_certs`, `ade_codec::conway::governance::{decode,encode}_proposal_procedures`, `ade_codec::shelley::opcert::{encode_opcert, decode_opcert}`, `ade_codec::shelley::tx_components::split_conway_tx_components`. |
| **Key modules** | `cbor/` (incl. `envelope.rs`, `tag24.rs`, `mod.rs`), `byron/`, `shelley/` (incl. `cert.rs`, `opcert.rs`, `tx_components.rs`), `allegra/`, `mary/`, `alonzo/`, `babbage/`, `conway/`, `address/`, `preserved.rs`, `traits.rs`, `primitives.rs`, `error.rs`. |

---

### `ade_core`

| Attribute | Value |
|-----------|-------|
| **Purpose** | BLUE authoritative Praos consensus core. Owns canonical types and pure state-transitions deciding which header/chain Ade accepts: HFC era schedule + slot↔era↔time, Praos chain-dep state (nonce / op-cert counters), VRF cert verification + leader-eligibility predicate, KES signature + op-cert period verification, header validation, fork-choice, header-level rollback, leader-schedule query, canonical encodings, producer-side opcert acceptance, and the closed BLUE leader-check evaluator (`consensus::leader_check`, N-R-A). Owns the single era→leader-VRF-input authority `consensus::vrf_cert::{ExpectedVrfInput, leader_vrf_input, leader_value_for}` (N-W) — the one place that selects a Praos vs TPraos leader-eligibility VRF construction. |
| **Creates** | **Schedule:** `BootstrapAnchorHash`, `EraSchedule`, `EraSummary`, `EraLocation`. **State:** `PraosChainDepState`, `OpCertCounterMap`, `Nonce`. **Events/points:** `Point`, `ChainHash`, `BlockDistance`, `SecurityParam`, `ChainEvent`, `ChainSelectionReject`. **Errors:** `HFCError`, `SlotTimeError`, `HeaderValidationError`, `VrfCertError`, `OpCertCounterError`, `NonceEvolutionError`, `LeaderScheduleError`, `OpCertError`. **Header surface:** `HeaderInput`, `HeaderVrf`, `HeaderKes`, `ValidatedHeaderSummary`, `HeaderApplied`. **Fork-choice:** `TiebreakerView`, `CandidateFragment`, `ChainSelectorState`, `ForkChoiceError`. **VRF/leader:** `VrfRole`, `VerifiedVrf`, `StakeFraction`, `ActiveSlotsCoeff`, `LeaderScheduleQuery`, `LeaderScheduleAnswer`, `ExpectedVrfInput` (closed 2-variant era-discriminated leader-VRF-input enum: `Praos([u8;32])` / `Tpraos([u8;VRF_INPUT_LEN=41])`, built only via `leader_vrf_input`). **Boundary:** `LedgerView` trait. **Rollback:** `RollBackRequest`, `RollBackApplied`. **`leader_check` (N-R-A):** `LeaderCheckVerdict` (closed 2-variant enum), `LeaderCheckError`, `VrfOutputFingerprint`, `LeaderProofFingerprint`. 49 public types. |
| **Interprets** | Canonical inputs from the `ade_runtime` shell. KES check verifies the hot-KES signature over the header body bytes via `ade_crypto::kes::verify_kes` (Ade-owned `kes_sum::Sum6Kes` internally). `leader_vrf_input(era, slot, eta0)` constructs the era-correct alpha; `leader_value_for(&ExpectedVrfInput, &VrfOutput)` applies the era-correct range-extension (Praos `praos_leader_value` vs TPraos). `leader_check::verify_and_evaluate_leader` consumes only public-key material + canonical inputs (era, slot, eta0, stake distribution, leader threshold, vrf vk, vrf proof/output, `LeaderScheduleAnswer`) and fail-closes on full-enum `ExpectedVrfInput` mismatch (wrong era *or* wrong alpha). `opcert_validate` consumes an `OperationalCert`, cold key, expected period, prev counter. |
| **MUST NOT** | (1)–(14) consensus carry-forward (no I/O, no clock, no `ChainDb` in BLUE consensus, no density in fork-choice, closed consensus enums, etc.). (15) `consensus::leader_check` (CN-FORGE-02) — MUST NOT depend on `LedgerView`, `EraSchedule`, `ChainDepState`, wall-clock, storage, or any RED crate; MUST NOT observe `KesSecret` / `VrfSigningKey` / `ColdSigningKey` (BLUE never sees private keys); `LeaderCheckVerdict::NotEligible` MUST carry only a bounded `vrf_output_fingerprint`, never forge-capable material (the closed 2-variant enum makes illegal observation structurally impossible). (16) `consensus::vrf_cert` leader-VRF authority (CN-FORGE-04 / N4): `leader_vrf_input` MUST be the **single** era→construction authority (defined exactly once); no verification or construction fallback may accept **both** TPraos and Praos VRF inputs for one era; for Praos (Babbage/Conway) the producer alpha MUST equal the validator alpha (`praos_vrf_input(slot, eta0) = blake2b256(slot‖eta0)` + `praos_leader_value` range-extension), NOT the TPraos role-tagged alpha (`slot‖eta0‖0x4C`); the `ExpectedVrfInput` variant **is** the protocol-family tag, so a Praos and a TPraos alpha can never be confused (`ci_check_producer_praos_vrf.sh`). |
| **Inbound deps** | `ade_ledger`, `ade_runtime` (heavy), `ade_testkit`, `ade_core_interop`, `ade_node`. |
| **Outbound deps** | `ade_types`, `ade_crypto`, `minicbor`. Dev-deps: `ade_testkit`, `serde_json`, `cardano-crypto`. |
| **Entry points** | `use ade_core::consensus::{…}` aggregator; top-level transitions `validate_and_apply_header`, `select_best_chain`, `apply_rollback`, `apply_nonce_input`, `apply_op_cert`, `query_leader_schedule`, `verify_vrf_cert`, `tiebreaker_prefer`; `ade_core::consensus::opcert_validate::opcert_validate`; `ade_core::consensus::leader_check::{verify_and_evaluate_leader, LeaderCheckVerdict, is_leader_for_vrf_output}`; `ade_core::consensus::vrf_cert::{ExpectedVrfInput, leader_vrf_input, leader_value_for, praos_vrf_input, praos_leader_value}`. |
| **Key modules** | `consensus/{era_schedule, praos_state, events, errors, vrf_cert, kes_check, nonce, op_cert, leader_schedule, header_summary, header_validate, candidate, fork_choice, rollback, ledger_view, encoding, opcert_validate, leader_check}.rs`. |

---

### `ade_crypto`

| Attribute | Value |
|-----------|-------|
| **Purpose** | Pure cryptographic verification primitives — Blake2b hashing, Ed25519 / Byron-bootstrap signature verification, KES verification with operational certificates, VRF verification, plus closed signature-artifact types. Verification only — signing lives in RED `ade_runtime::producer::signing`. Also owns the Ade-native `Sum6KES Ed25519DSIGN` algorithm (`kes_sum/`), byte-identical to Haskell `cardano-base`. |
| **Creates** | `Blake2b224`, `Blake2b256`, `HashAlgorithm`, `Ed25519VerificationKey`, `Ed25519Signature`, `ByronExtendedVerificationKey`, `KesVerificationKey`, `KesPeriod`, `OperationalCertData`, `VrfVerificationKey`, `VrfProof`, `VrfOutput`, `CryptoError`, `KesSignature`, `SUM6_KES_SIG_LEN = 448`. **`kes_sum::`** (BLUE): `KesAlgorithm` trait, `Sum0Kes`, `Sum0SigningKey`, `Sum0Signature`, `SumKes<D>`, `SumSigningKey<D>`, `SumSignature<D>`, type aliases `Sum1Kes..Sum6Kes`, closed `KesError` (5 variants), closed `KesParseError` (6 variants). |
| **Interprets** | Already-decoded byte slices (not a CBOR parser). `Sum6Kes::raw_deserialize_signing_key_kes` interprets the canonical 608-byte expanded cardano-cli skey (byte layout is the structural validator); `raw_deserialize_signature_kes` the 448-byte signature; `period_from_zeroed_sum6_tree_shape` infers the current period from tree shape. |
| **MUST NOT** | (1) Implement signing as a top-level `pub fn sign_*` (`ci_check_no_signing_in_blue.sh`). (2) Allocate global state. (3) Use any BLUE-forbidden pattern. (4) `unsafe` outside the allowlisted FFI in `vrf.rs`. (5) `build_opcert_signable` must produce the spec-correct concat. (6) `KesSignature` (DC-CRYPTO-04) — closed length-pinned wrapper, `from_bytes` only, redacting `Debug`. (7) `kes_sum` (DC-CRYPTO-08) — MUST NOT import `cardano_crypto::kes::*` in production paths under `crates/ade_crypto/src/**` (`#[cfg(test)]` oracle only; `ci_check_kes_sum_compatibility.sh` Guard 3). (8) `kes_sum::hash::expand_seed` — MUST use Haskell prefix bytes `0x01`/`0x02`, not `0x00`/`0x01` (Guard 4). (9) every `*SigningKey` MUST hand-roll a zeroizing `Drop` + redacting `Debug` + no public byte accessors. (10) `kes_sum` corpus — every `SKEY{N}` const preceded by the throwaway-fixture comment; no `.skey` files under `crates/ade_crypto/` (Guards 1, 2). (11) error variants carry only non-secret primitives. |
| **Inbound deps** | `ade_core`, `ade_ledger`, `ade_plutus`, `ade_testkit`, `ade_core_interop`, `ade_runtime` (consumes `kes_sum::{KesAlgorithm, Sum6Kes, KesParseError}` directly). |
| **Outbound deps** | `ade_types`, `blake2`, `ed25519-dalek`, `cardano-crypto` (`["vrf-draft03", "kes-sum", "dsign"]`, `default-features = false`; `kes-sum` retained for `#[cfg(test)]` oracle only). |
| **Entry points** | `ade_crypto::blake2b::*`, `verify_ed25519`, `verify_byron_bootstrap`, `verify_kes`, `verify_opcert`, `verify_vrf`, `ade_crypto::kes::{KesSignature, SUM6_KES_SIG_LEN, KesVerificationKey}`, `ade_crypto::kes_sum::{KesAlgorithm, Sum6Kes, KesError, KesParseError, period_from_zeroed_sum6_tree_shape}`. |
| **Key modules** | `blake2b.rs`, `ed25519.rs`, `error.rs`, `kes.rs`, `kes_sum/` (`mod`, `single`, `sum`, `hash`, `errors`, `period`, `cardano_cli_corpus` `#[cfg(test)]`, `tests` `#[cfg(test)]`), `traits.rs`, `vrf.rs`. |

---

### `ade_ledger`

| Attribute | Value |
|-----------|-------|
| **Purpose** | The functional core (ledger half): stateless ledger rules for every era; B1 block-validity verdict; B2 transaction-validity verdict + mempool admission; B3 Conway value-conservation; B4/B5 closed Conway cert-state + governance-cert accumulation; live committee enactment; the single BLUE wire-ingress chokepoint `mempool::ingress::mempool_ingress`; the BLUE producer authority (`producer::{forge, self_accept, served_chain, state}`); the single canonical body-hash authority (`block_body_hash`); the BLUE receive-side header→body bridge (`receive::*`); the BLUE rollback authority (`rollback::*`); the BLUE canonical snapshot encoder/decoder (`snapshot::*`); the BLUE Ade-native WAL replay+event authority (`wal::*`); the BLUE bootstrap-anchor types (`bootstrap_anchor::*`); the canonical KES-signing pre-image recipe `block_validity::unsigned_header_pre_image` (N-S-A). |
| **Creates** | ~167 public types across `block_validity/`, `tx_validity/`, `mempool/`, `cert_classify/`, `consensus_view/`, `producer/`, `receive/`, `rollback/`, `snapshot/`, `wal/`, `bootstrap_anchor/`, `block_body_hash`. Load-bearing: `BlockValidityError`, `BlockVerdict`, `TxValidity*`, `MempoolState`, `ProducerTick`, `ForgedBlock`, `ForgeError`, `ForgeEffects`, `ServedChainSnapshot`, `ServedChainAdmitError`, `PoolDistrView`, `LedgerState`, `WalEvent`, `BootstrapAnchor`, `UnsignedHeaderPreImage` (branded `Vec<u8>` newtype with a single canonical constructor), `UnsignedHeaderPreImageError`. |
| **Interprets** | Canonical decoded blocks/txs/certs/snapshots; `mempool_ingress` is the sole BLUE chokepoint from wire ingress to `admit`; `self_accept` re-validates a forged artifact against a pre-forge base; `served_chain_admit` derives the served index from accepted blocks; `unsigned_header_pre_image` produces the canonical CBOR encoding of `ShelleyHeaderBody`. |
| **MUST NOT** | All carry-forward ledger prohibitions (no I/O, no clock, closed cert/governance enums, single body-hash recipe, etc.). **`producer::forge` (CN-FORGE-01/03, CN-FORGE-03 strengthened N-X):** MUST emit exactly one of `ForgeSucceeded`/`ForgeNotLeader`/`ForgeFailed`; MUST NOT emit `ForgeSucceeded` unless `self_accept` accepts; MUST wrap output via `ade_codec::encode_block_envelope` so `decode_block(forge_block(tick).bytes)` is `Ok` (CN-FORGE-03) — no bare-block forge output, no parallel forge codepath (`ci_check_forge_decode_round_trip.sh`, `ci_check_no_independent_forge_codepath.sh`). **`producer::served_chain` (CN-PROD-04):** only self-accepted blocks may be admitted; the sole entry path is `served_chain_admit`. **`block_validity::unsigned_header_pre_image` (CN-KES-HEADER-01 / DC-KES-HEADER-01):** the branded `UnsignedHeaderPreImage`'s only constructor is the canonical recipe (arbitrary-byte signing structurally unrepresentable); output MUST be byte-identical to the validator-side extractor `header_input::decode_block(...).header_input.kes.header_body_bytes` for every corpus block (CN-PREIMAGE-FIXTURE-01); MUST be a pure function — same inputs → byte-identical output (`ci_check_unsigned_header_preimage_single_source.sh`). |
| **Inbound deps** | `ade_testkit`, `ade_core_interop`, `ade_runtime`, `ade_node`. |
| **Outbound deps** | `ade_types`, `ade_crypto`, `ade_codec`, `ade_plutus`, `ade_core`, `minicbor`, `num-bigint`, `num-integer`, `num-traits`. Dev-dep: `ade_testkit`. |
| **Entry points** | `ade_ledger::block_validity::{decode_block, validate_block}`, `ade_ledger::tx_validity::*`, `ade_ledger::mempool::{ingress::mempool_ingress, admit::*}`, `ade_ledger::producer::{forge::forge_block, self_accept::self_accept, served_chain::{ServedChainSnapshot, served_chain_admit}}`, `ade_ledger::block_body_hash::*`, `ade_ledger::receive::reducer::*`, `ade_ledger::rollback::*`, `ade_ledger::snapshot::*`, `ade_ledger::wal::*`, `ade_ledger::bootstrap_anchor::*`, `ade_ledger::block_validity::unsigned_header_pre_image::unsigned_header_pre_image`. |
| **Key modules** | `block_validity/` (incl. `unsigned_header_pre_image.rs`, `header_input.rs`, `transition.rs`, `verdict.rs`), `tx_validity/`, `mempool/`, `cert_classify/`, `consensus_view/`, `producer/` (`forge`, `self_accept`, `served_chain`, `state`), `block_body_hash.rs`, `receive/`, `rollback/`, `snapshot/`, `wal/`, `bootstrap_anchor/`. |

> **GREEN sub-classification:** `ade_ledger::mempool::{policy, canonicalize}` are GREEN-by-content (Tier-5 policy + canonicalization). **RED sub-classification:** `ade_ledger::consensus_input_extract` is RED-by-content. Both carry their own module banner.

---

### `ade_network` *(BLUE submodules)*

| Attribute | Value |
|-----------|-------|
| **Purpose** | Owns the Cardano Ouroboros mini-protocol authority — the closed wire grammar (CBOR codecs) and pure state machines for all N2N + N2C mini-protocols, plus the BLUE mux frame primitive. Includes the producer-side server-role reducer surface (`chain_sync::server`, `block_fetch::server`). **NEW (N-X):** owns the per-protocol tag-24 **composition** authorities layered over `ade_codec::{wrap_tag24, unwrap_tag24}` — `codec::block_fetch::{compose_blockfetch_block, decompose_blockfetch_block}` (era **inside** the wrap; EBB-aware era index, Conway = storage index 7) and `codec::chain_sync::{compose_rollforward_header, decompose_rollforward_header, chain_sync_wire_era_index}` (era_tag **outside** the wrap; CONSENSUS era index, Conway = 6 = storage − 1). |
| **Creates** | 109 BLUE canonical types: `mux/frame.rs` 5, `codec/` 38, `handshake/` 9, `chain_sync/` 11, `block_fetch/` 10, `tx_submission/` 5, `keep_alive/` 5, `peer_sharing/` 5, `n2c/` 21. **N-X (no new type — extends existing):** `chain_sync::server::HeaderProjection` gained an `era: CardanoEra` field (drives the ChainSync consensus era index in the RollForward wrap; `header_bytes` stays the BARE era-specific header). |
| **Interprets** | Mini-protocol wire frames; `mux::frame::decode_frame` is the single frame decode authority; per-protocol `*_transition` reducers consume decoded messages. **N-X:** `decompose_blockfetch_block` / `decompose_rollforward_header` strip the per-protocol tag-24 composition (delegating the inner unwrap to `ade_codec::unwrap_tag24`), returning the verbatim inner block / `(era_tag, header_cbor)`. |
| **MUST NOT** | (1) `mux::frame` — single `encode_frame`/`decode_frame` pair workspace-wide. (2) handshake — single `n2n_transition` + single `n2c_transition`. (3) closed `AcceptedMiniProtocol` registry — `match` with no wildcard accept. (4) no socket I/O in BLUE submodules. (5) no async/tokio in BLUE submodules. (6) depend on no workspace crate beyond `ade_codec` + `ade_types`. (7) server reducers MUST NOT split headers in parallel (`ci_check_no_parallel_header_splitter.sh`) or depend on signing. **(8) NEW — tag-24 composition (CN-WIRE-08):** the per-protocol composers MUST delegate the byte-level wrap/strip to the single `ade_codec` authority (no hand-rolled tag-24 parse in `ade_network`); a served BlockFetch `MsgBlock` payload MUST be `tag24(bytes([era, block]))` and a served ChainSync `RollForward` header MUST be `[era_tag, tag24(bytes(header_cbor))]` — no bare `[era, block]` served over BlockFetch, no bare header served over ChainSync RollForward; both compositions are pinned byte-identically against captured cardano-node 11.0.1 wire fixtures (`ci_check_tag24_wire_authority.sh`). |
| **Inbound deps** | `ade_runtime`, `ade_node`, `ade_core_interop`, `ade_testkit` (and the `ade_network` GREEN/RED submodules). |
| **Outbound deps** | `ade_types`, `ade_codec`. No external deps in BLUE submodules. |
| **Entry points** | `ade_network::mux::frame::{encode_frame, decode_frame}`, `ade_network::handshake::{n2n_transition, n2c_transition}`, `ade_network::chain_sync::{server::*, client::*}`, `ade_network::block_fetch::{server::*, client::*}`, `ade_network::codec::block_fetch::{compose_blockfetch_block, decompose_blockfetch_block}`, `ade_network::codec::chain_sync::{compose_rollforward_header, decompose_rollforward_header, chain_sync_wire_era_index}`, `ade_network::{tx_submission, keep_alive, peer_sharing, n2c}::*`. |
| **Key modules** | `mux/frame.rs`, `codec/` (incl. `block_fetch.rs`, `chain_sync.rs`), `handshake/`, `chain_sync/` (incl. `server.rs`), `block_fetch/` (incl. `server.rs`), `tx_submission/`, `keep_alive/`, `peer_sharing/`, `n2c/`. |

---

### `ade_plutus`

| Attribute | Value |
|-----------|-------|
| **Purpose** | Quarantine boundary between the Ade-canonical ledger and the ported UPLC evaluator from `aiken-lang/aiken` (pinned tag `v1.1.21`). |
| **Creates** | `PlutusScript`, `PlutusLanguage`, `EvalOutput`, `PlutusError`, `CostModels`, `DecoderMode`, `PerScriptResult`, `TxEvalResult`. |
| **Interprets** | UPLC scripts (Plutus V1/V2/V3) and `CostModels` CBOR; phase-two tx evaluation. `PlutusScript::from_cbor` is a named ingress chokepoint. |
| **MUST NOT** | (1) Re-export any `pallas_*` or `aiken_uplc::` type. (2) Let another BLUE crate bypass the canonical entry. (3) Activate PV11 builtins. (4) Use any BLUE-forbidden pattern. (5) Construct `PreservedCbor` outside `ade_codec`. |
| **Inbound deps** | `ade_ledger`, `ade_testkit`. |
| **Outbound deps** | `ade_types`, `ade_crypto`, `ade_codec`, `aiken_uplc` (git, tag `v1.1.21`), `pallas-primitives` (internal-only). |
| **Entry points** | `ade_plutus::eval_tx_phase_two`, `ade_plutus::tx_eval::*`, `ade_plutus::evaluator::*`, `ade_plutus::cost_model::*`. |
| **Key modules** | `evaluator.rs`, `cost_model.rs`, `script_context.rs`, `script_verdict.rs`, `tx_eval.rs`. |

---

### `ade_types`

| Attribute | Value |
|-----------|-------|
| **Purpose** | Owns the canonical Cardano domain vocabulary — primitives, era enum, address forms, era-tagged tx bodies/outputs/certificates, governance types — the lingua franca for every other crate. The `CardanoEra` era enum carries the `is_praos()` classifier (true for Babbage and Conway), the canonical "is this a Praos-family era?" predicate used by the producer forge era-guard and the leader-VRF authority. |
| **Creates** | `CardanoEra` (with `is_praos()`), `SlotNo`, `BlockNo`, `EpochNo`, `Hash28`, `Hash32`, `Coin`, `Lovelace`, `NetworkId`, `Nonce`, `TxIn`, `RewardAccount`, `PoolId`, `Address`, `ByronAddress`, `Credential`, `StakeCredential`, `Certificate`, `PoolRegistrationCert`, `ConwayCert`, `MIRCert`, `DRep`, `GovAction`, `GovActionState`, `GovActionId`, `Anchor`, `ProposalProcedure`, `OperationalCert`, `NativeScript`, `Datum`, `MultiAsset`, `AssetName`, `CostModel`, `ExUnits`, plus per-era tx-body/tx-out/witness wrappers and `ShelleyBlock`/`ShelleyHeader`/`VrfData`/`ProtocolVersion`. |
| **Interprets** | None — produce-only. |
| **MUST NOT** | (1) Construct or decode `PreservedCbor`. (2) Use any BLUE-forbidden pattern. (3) Depend on any workspace crate. (4) Add open/extensible variants to closed enums without a versioned gate (the `CardanoEra` discriminants are a closed set: Byron=0 … Conway=7; `is_praos()` MUST classify exactly {Babbage, Conway}, never widen silently). |
| **Inbound deps** | Every other workspace crate. |
| **Outbound deps** | None. |
| **Entry points** | `ade_types::CardanoEra` (incl. `is_praos`, `is_byron`, `as_u8`), `ade_types::tx::{Coin, TxIn, RewardAccount}`, `ade_types::{Hash32, SlotNo, Hash28, BlockNo, EpochNo}`, `ade_types::conway::{tx::ConwayTxBody, cert::*, governance::{ProposalProcedure, GovAction, GovActionId}}`, `ade_types::shelley::block::{OperationalCert, ProtocolVersion, ShelleyHeader, ShelleyBlock, VrfData}`. |
| **Key modules** | `primitives.rs`, `era.rs`, `tx.rs`, `address/`, `byron/`, `shelley/`, `allegra/`, `mary/`, `alonzo/`, `babbage/`, `conway/`. |

---

## GREEN Modules — Deterministic Glue

> Deterministic, non-authoritative. May depend on BLUE; must not affect authoritative outputs. GREEN sub-trees inside the RED `ade_runtime` and `ade_node` crates carry a `//! GREEN …` banner + the same deny attributes and are CI-gated for purity.

### `ade_testkit`

| Attribute | Value |
|-----------|-------|
| **Purpose** | Test infrastructure: differential harness, oracle snapshot loader, regression corpus, genesis loader, era mapping, transcript replay, diff reporting; consensus / block-validity / tx-validity / mempool-ingress / proposal-procedures / producer test harnesses. |
| **Creates** | Harness + corpus types (`ConwayValidityCorpus`, differential replay drivers, snapshot loaders). Not canonical-counted (GREEN). |
| **MUST NOT** | (1) Affect authoritative outputs. (2) Introduce nondeterminism into the BLUE-under-test path. (3) Construct semantic types bypassing the canonical decoders. |
| **Inbound deps** | None at compile time; consumed via integration tests + dev-dep links. |
| **Outbound deps** | `ade_types`, `ade_codec`, `ade_crypto`, `ade_core`, `ade_ledger`, `ade_plutus`, `ade_runtime`, `blake2`, `flate2`, `tar`, `serde`, `serde_json`, `toml`, `cardano-crypto` (dev). |
| **Entry points** | `ade_testkit::validity::corpus::ConwayValidityCorpus`, the differential `*_replay` drivers, the snapshot loaders, the genesis loader. |
| **Key modules** | `consensus/`, `validity/`, `tx_validity/`, `mempool/`, `governance/`, `producer/`, differential + corpus infrastructure. |

---

### `ade_network::session` *(GREEN by content — PHASE4-N-L)*

| Attribute | Value |
|-----------|-------|
| **Purpose** | The pure session driver. Composes the BLUE authorities `mux::frame::{encode,decode}_frame`, `handshake::n2n_transition`, and per-mini-protocol state machines through `session::core::step`; owns the partial-frame buffer + `SessionState` type-state + closed `AcceptedMiniProtocol` registry + per-mini-protocol payload reassembly. |
| **Creates** | ~12 closed types across `session::{event, state, demux, core, handshake_driver}` (`SessionState`, `AcceptedMiniProtocol`, session events/effects). |
| **MUST NOT** | (1) `session::core::step` is the only pub reducer in `session/`. (2) no `tokio::*` imports in session core. (3) `AcceptedMiniProtocol` closed — `match` with no wildcard accept. (4) no `mpsc::unbounded_channel` / unbounded queue (`ci_check_session_no_unbounded.sh`). (5) affect authoritative outputs. |
| **Inbound deps** | `ade_runtime::network::{mux_pump, n2n_dialer}`, `ade_runtime::orchestrator::keep_alive_session`. |
| **Outbound deps** | `ade_network` BLUE submodules (`mux::frame`, `handshake`, the per-protocol reducers), `ade_codec`, `ade_types`. |
| **Entry points** | `ade_network::session::core::step`, `ade_network::session::{state::SessionState, event::*, demux::*, handshake_driver::*}`. |
| **Key modules** | `session/{mod, core, event, state, demux, handshake_driver}.rs`. |

---

### `ade_runtime` GREEN-by-content sub-trees

> All carry a `//! GREEN …` banner inside the RED `ade_runtime` crate, the BLUE deny attributes, and a purity CI gate. Grouped here; each is "promotable to BLUE on demand, never demotable to RED" unless noted.

| Sub-tree | Purpose | MUST NOT | CI gate |
|---|---|---|---|
| `producer::coordinator` *(N-Q)* | Pure-state-machine coordinator for live producer-mode — orchestrates the slot loop + forge requests + peer lifecycle. Emits closed `CoordinatorEffect`s. | Own/store private signing material; `KesSecret`/`VrfSigningKey`/`ColdSigningKey` never enter `CoordinatorState` (CN-PROD-02); affect authoritative outputs. | `ci_check_producer_coordinator_no_secrets.sh` |
| `producer::producer_log` *(N-Q, extended N-W)* | Closed-vocabulary producer evidence log — `ProducerLogEvent` is the closed enum emitted on every observable producer-mode transition; the closed `ForgeFailureReason` enum carries the `UnsupportedProducerEra` variant (N-W) for a non-Praos forge attempt. | Add an open/wildcard event variant; emit `BlockServed` for a block not in the served snapshot (CN-PROD-04). | `ci_check_operator_evidence_manifest_schema.sh` (evidence schema) |
| `producer::chain_evolution` *(N-T)* | Linear `ChainEvolution` typestate threading the producer's chain state forward across forges (a pure value — no held trait object). | **Never mints `AcceptedBlock`** — obtains the token solely from `self_accept` (CE-T-7); advance on authority disagreement (fail-closes `ChainEvolutionError::AuthorityMismatch`, CE-T-6b); introduce nondeterminism. | gated by CE-T-7 (no token minting); purity inherited |
| `producer::{broadcast_to_served, served_chain_lookups}` *(N-G)* | GREEN glue computing served-chain broadcast targets + read-side lookups over the BLUE `ServedChainSnapshot`. | Affect authoritative outputs; serve a block that failed self-accept. | `ci_check_broadcast_to_served_purity.sh` |
| `bootstrap` *(N-K)* | Sole `bootstrap_initial_state` authority (CN-NODE-01). | Be a parallel bootstrap path; produce_mode obtains initial state only here. | `ci_check_bootstrap_closure.sh`, `ci_check_node_binary_uses_single_bootstrap.sh` |
| `clock` (`DeterministicClock` + trait) *(N-K)* | GREEN clock trait + deterministic impl (DC-NODE-03). The `SystemClock` in the same file is RED-sub-classified. | DeterministicClock must read no wall-clock. | `ci_check_clock_seam.sh` |
| `orchestrator::{mod, event, state, core}` *(N-K)* | GREEN core reducer + closed-vocabulary event + state for the node orchestrator. | tokio imports in core; open event vocabulary. | `ci_check_orchestrator_core_purity.sh` |
| `rollback::{cadence, in_memory_cache, chaindb_block_source, persistent_cache, persistent_writer}` *(N-I/N-J/N-K)* | GREEN rollback adapter glue + snapshot cadence + persistent cache/writer (DC-NODE-02, DC-CONS-21). | Parallel snapshot cadence; affect authoritative rollback state. | `ci_check_persistent_writer_no_parallel_cadence.sh`, `ci_check_snapshot_cadence_purity.sh` |
| `receive::{events_to_state, in_memory_chain_write}` *(N-H)* | GREEN receive-side glue mapping BLUE receive events to in-memory chain writes. | Affect authoritative receive verdict. | `ci_check_receive_replay_purity.sh` |
| `seed_import` *(N-M-A)* | Single authority converting a cardano-cli JSON UTxO dump into canonical seed entries. | Construct semantic types bypassing canonical decoders. | `ci_check_seed_import_closure.sh`, `ci_check_seed_import_full_preprod_support.sh` |
| `bootstrap_anchor` *(N-M-A)* | Sole authority minting `BootstrapAnchor` from import inputs (network, genesis, …). | Mint an anchor outside this composer. | `ci_check_bootstrap_anchor_closure.sh` |
| `wal` *(N-M-A)* | File-backed Ade-native WAL (append-only). | Mutate/rewrite committed WAL entries. | `ci_check_wal_append_only.sh` |
| `consensus_inputs` *(N-M-C)* | Operator-extracted `LiveConsensusInputs` importer. | Treat the peer as runtime authority; overstate semantic truth. | `ci_check_live_consensus_inputs_closure.sh`, `ci_check_live_consensus_inputs_fingerprint.sh` |
| `admission::*` (the GREEN reducer half) *(N-M-C)* | GREEN admission verdict/agreement reducer comparing already-authoritative outputs. | Emit RED verdicts; skip reference scripts; treat `lagging` as success (DC-EVIDENCE-01). | `ci_check_admission_runner_closure.sh`, `ci_check_admission_no_red_verdicts.sh`, `ci_check_lagging_is_evidence_only.sh`, `ci_check_admit_replay_equivalence.sh` |

---

### `ade_node` GREEN-by-content sub-trees

| Sub-tree | Purpose | MUST NOT | CI gate |
|---|---|---|---|
| `admission_log` *(N-M-B)* | GREEN admission-mode JSONL event vocabulary + writer (closed enum). | Add open/wildcard event variant. | `ci_check_admission_log_vocabulary_closed.sh` |
| `live_log` *(N-L-LIVE)* | GREEN closed JSONL vocabulary for the wire-only live pass. | Add open event variant; overstate semantic truth (wire success ≠ admission). | `ci_check_wire_only_event_vocabulary_closed.sh` |
| `admission` (the GREEN half of the orchestrator) *(N-M-B)* | GREEN admission orchestrator reducer + verdict mapping (RED runner lives alongside). | Affect authoritative verdict; skip reference-script validation. | `ci_check_admission_no_refscript_skip.sh` |

---

## RED Modules — Imperative Shell

> I/O, network, storage, clocks, retries. May depend on BLUE/GREEN. Must not modify core state directly or construct semantic types unsafely.

### `ade_runtime`

| Attribute | Value |
|-----------|-------|
| **Purpose** | The imperative shell — moves bytes, owns sockets/files/clocks/keys, drives tokio tasks. Hosts: producer-mode key custody + shell composition (`producer/`), the N2N network drivers (`network/`), the node orchestrator runners (`orchestrator/`), the receive/rollback/admission shells, the seed-import/WAL/consensus-inputs importers (GREEN-by-content, see above), ChainDB + recovery shells. |
| **Creates (RED-only)** | `KesSecret`, `VrfSigningKey`, `ColdSigningKey` custody wrappers; `KeyLoadError` (incl. `KesParse`, `UnsupportedExpandedKesKeyFormat`); `MuxTransportHandle` consumers; `OutboundCommand` (closed enum, N-S-B); `PerPeerOutbound` map (`Arc<RwLock<BTreeMap<PeerId, mpsc::Sender<OutboundCommand>>>>`); `DispatchError`; `ServedChainHandle` (watch-channel wrapper); `GenesisAnchor` (parsed); the RED producer shell handlers. Never semantic/canonical types. |
| **Interprets** | Canonicalizes peer bytes/files for the BLUE core: `producer::keys::load_kes_signing_key_skey` interprets the 608-byte cardano-cli skey via the BLUE `Sum6Kes::raw_deserialize_signing_key_kes`; `producer::genesis_parser` parses `shelley-genesis.json` → `GenesisAnchor`; `producer::opcert_envelope` parses the cardano-cli opcert text envelope; `seed_import` parses the cardano-cli UTxO dump. |
| **MUST NOT** | (1) Modify BLUE state directly or construct semantic types from raw bytes (must go through the canonical decoders). (2) Bypass canonical validation. (3) `producer::signing` (DC-CRYPTO-03/05) — RED-confined key custody: no public byte accessors, redacted `Debug`, hand-rolled zeroizing `Drop`; MUST consume `ade_crypto::kes_sum::Sum6Kes` (not `cardano_crypto::kes`). (4) `producer::keys` — MUST NOT call `KesSecret::from_bytes_zeroizing`/`from_seed_at_period` inside `load_kes_signing_key_skey` (only the BLUE deserializer path; `ci_check_kes_envelope_closed.sh` Guard 2). (5) `producer::coordinator` is GREEN and MUST NOT hold secrets — secret custody is confined to `producer::producer_shell`. (6) **`network::outbound_command` (CN-OUTBOUND-RELAY-01):** `OutboundCommand` is the **sole** channel between `produce_mode` and `MuxPump`'s outbound encoder — typed `ChainSyncServerMsg`/`BlockFetchServerMsg` variants, **no `Vec<u8>` byte tunnel**, no direct `MuxTransportHandle::outbound` write from `produce_mode` (`ci_check_no_produce_mode_direct_transport_writes.sh`). (7) **per-peer outbound map (CN-PEER-OUTBOUND-MAP-01 / DC-OUTBOUND-FIFO-01):** `BTreeMap` (not `HashMap`) for deterministic iteration; lookup failure is structured (`DispatchError::{UnknownPeer, PeerOutboundMissing}`); no cross-peer byte leakage; FIFO preserved per peer. (8) `network::n2n_server` — MUST NOT depend on signing (`ci_check_n2n_server_no_signing_dep.sh`). |
| **Inbound deps** | `ade_node`, `ade_core_interop`, `ade_testkit` (dev/integration). |
| **Outbound deps** | `ade_types`, `ade_core`, `ade_crypto`, `ade_codec`, `ade_ledger`, `ade_network`, `redb`, `serde`, `serde_json`, `bech32`, `base58`, `cardano-crypto` (`["vrf-draft03", "dsign"]` — `kes-sum` dropped), `ed25519-dalek`, `tokio`. |
| **Entry points** | `ade_runtime::producer::{producer_shell::*, coordinator::*, served_chain_handle::{ServedChainHandle, push_atomic}, genesis_parser::*, opcert_envelope::*}`, `ade_runtime::network::{n2n_listener::*, n2n_dialer::*, mux_pump::*, n2n_server::*, outbound_command::OutboundCommand}`, `ade_runtime::orchestrator::*`, `ade_runtime::bootstrap::bootstrap_initial_state`, `ade_runtime::{seed_import, consensus_inputs, wal, bootstrap_anchor}::*`. |
| **Key modules** | `producer/` (RED: `producer_shell`, `signing`, `keys`, `scheduler`, `broadcast`, `tick_assembler`, `ade_kes_envelope`, `genesis_parser`, `opcert_envelope`, `served_chain_handle`; GREEN-by-content: `coordinator`, `producer_log`, `chain_evolution`, `broadcast_to_served`, `served_chain_lookups`), `network/` (`n2n_listener`, `n2n_dialer`, `mux_pump`, `n2n_server`, `outbound_command`), `orchestrator/` (RED runners + GREEN core), `receive/`, `rollback/`, `admission/`, `seed_import/`, `consensus_inputs/`, `wal/`, `chaindb/`, `bootstrap.rs`, `bootstrap_anchor.rs`, `clock.rs`, `recovery.rs`. |

---

### `ade_node`

| Attribute | Value |
|-----------|-------|
| **Purpose** | The node binary + library entry. Owns argv parsing, the node lifecycle (`run_node_until_shutdown`), and the mode drivers: `--mode produce` (`produce_mode`), the admission mode (`admission/`), the wire-only live smoke pass (`wire_only`), and `key-gen-KES` (`key_gen`). Composes the `ade_runtime` shell surfaces into a runnable process. `produce_mode::run_real_forge` carries an era guard — a non-Praos era fail-closes to `ForgeFailureReason::UnsupportedProducerEra` before any forge, and the prove-step reads the leader-schedule answer's `expected_vrf_input.alpha_bytes()` (no RED-side era dispatch — the era→VRF construction is the BLUE authority's). **NEW (N-X):** `admission::runner` deleted its hand-rolled `unwrap_block_fetch_envelope` and now strips the served block's tag-24 envelope via `ade_codec::unwrap_tag24` (single shared authority). |
| **Creates (RED-only)** | `Cli`, `CliError`, `ProduceCli`, `NodeStartupInputs`, `NodeShutdownEvidence`, `NodeRunError`, exit-code constants; the `produce_mode` slot loop + absolute-slot ticker + evidence-I/O types; the admission runner types. Never semantic/canonical types. |
| **Interprets** | argv (closed mode set); operator-supplied key/genesis/opcert file paths (delegated to `ade_runtime` parsers); evidence-manifest TOML schema. **N-X:** the admission runner interprets a peer's tag-24-wrapped BlockFetch `MsgBlock` payload via the shared `ade_codec::unwrap_tag24` (no local tag-24 parse). |
| **MUST NOT** | (1) Construct semantic types bypassing the canonical decoders / `ade_runtime` parsers. (2) **`produce_mode` (CN-PROD-04, CN-OUTBOUND-RELAY-01):** obtain initial state only via `bootstrap_initial_state` (`ci_check_produce_mode_uses_bootstrap_initial_state.sh`); reconstruct each broadcast block through BLUE `self_accept` before `push_atomic`; emit outbound bytes **only** via `OutboundCommand` → `MuxPump` (no direct transport write — `ci_check_no_produce_mode_direct_transport_writes.sh`). (3) No synthetic forge state — no `SyntheticForgeInputs`/zero-stake `LeaderScheduleAnswer`/inline `LedgerState::new(...)` forge base (N-T hard prohibition). (4) No durability in the produce_mode path (no WAL/snapshot writes — N-U scope). (5) **`run_real_forge` (CN-FORGE-04, N-W):** MUST NOT perform RED-side era dispatch for the leader-VRF alpha — it proves over the BLUE answer's `expected_vrf_input.alpha_bytes()`; a non-Praos era MUST fail-close to `UnsupportedProducerEra` (never silently forge with a TPraos alpha) (`ci_check_producer_praos_vrf.sh`). (6) **`admission::runner` (CN-WIRE-08, N-X):** MUST NOT hand-roll a tag-24 parse — the served block's CBOR-in-CBOR envelope MUST be stripped through the single `ade_codec::unwrap_tag24` authority (`ci_check_tag24_wire_authority.sh`). (7) `wire_only` — overstate semantic truth (wire success ≠ admission ≠ agreement); closed JSONL vocabulary only (`ci_check_wire_only_no_bootstrap.sh`). (8) operator-evidence manifest must carry the closed schema (CN-OPERATOR-EVIDENCE-01; `ci_check_operator_evidence_manifest_schema.sh`). (9) closed mode set (`ci_check_node_mode_closure.sh`). |
| **Inbound deps** | None (binary + integration tests). |
| **Outbound deps** | `ade_types`, `ade_core`, `ade_crypto`, `ade_ledger`, `ade_runtime`, `ade_network`, `ade_codec`, `tokio`. Dev-deps: `ade_testkit`, `tempfile`. |
| **Entry points** | `main()`; `ade_node::run_node_until_shutdown` (library entry driven in-process by integration tests); `ade_node::produce_mode::{run_real_forge, *}`; `ade_node::cli::{Cli, ProduceCli}`; `ade_node::admission::*`; `ade_node::wire_only::*`; `ade_node::key_gen::*`. |
| **Key modules** | `lib.rs`, `cli.rs`, `node.rs`, `main.rs`, `produce_mode.rs`, `wire_only.rs`, `key_gen.rs`, `admission/` (`bootstrap`, `runner`, `seed_to_snapshot`, `verdict`), `admission_log/`, `live_log/`. |

---

### `ade_core_interop`

| Attribute | Value |
|-----------|-------|
| **Purpose** | Live cardano-node interop driver. Hosts the `live_*_session` RED binaries (operator-action evidence harness) plus the N-E S4/S5 GREEN tx-submission bridges, and the chain-follow driver (`follow`). **NEW (N-X):** `follow` deleted its hand-rolled tag-24 header parse and now strips the peer's ChainSync `RollForward` `[era_tag, tag24(header_cbor)]` envelope via `ade_codec::unwrap_tag24` (single shared authority). |
| **Creates (RED-only)** | Live-session drivers and transcript types; never semantic types. |
| **MUST NOT** | (1) Construct semantic types from raw bytes. (2) Be depended on by any BLUE/GREEN crate (RED leaf). (3) Overstate semantic truth in evidence (wire success ≠ admission). (4) **`follow` (CN-WIRE-08, N-X):** MUST NOT hand-roll a tag-24 parse — the peer header's CBOR-in-CBOR envelope MUST be stripped through the single `ade_codec::unwrap_tag24` authority (`ci_check_tag24_wire_authority.sh`). |
| **Inbound deps** | None (RED leaf — binaries). |
| **Outbound deps** | `ade_core`, `ade_codec`, `ade_crypto`, `ade_ledger`, `ade_runtime`, `ade_network`, `ade_testkit`, `ade_types`, `tokio`. |
| **Entry points** | `ade_core_interop::bin::{live_tx_submission_session, live_block_production_session, live_block_fetch_session, live_block_follow_session}`; `ade_core_interop::follow::*`; `ade_core_interop::tx_submission` (GREEN sub-class), `ade_core_interop::local_tx_submission` (GREEN sub-class). |
| **Key modules** | `bin/`, `follow.rs`, `tx_submission.rs`, `local_tx_submission.rs`. |

---

### `ade_network::mux::transport` *(RED)*

| Attribute | Value |
|-----------|-------|
| **Purpose** | The only place inside `ade_network` where socket I/O happens. RED tokio shell over the BLUE mux frame primitive and the GREEN session reducer. Provides `MuxTransportHandle` + closed `TransportError` + `DuplexCapacity::DEFAULT` + `spawn_duplex` (preserving the older `MuxTransport`/`open_tcp` API). |
| **Creates (RED-only)** | `MuxTransportHandle`, `TransportError`, `DuplexCapacity`, `MuxTransport`. |
| **MUST NOT** | (1) Construct semantic types. (2) Bypass `mux::frame` for framing. (3) Live in BLUE scope (nominally RED per `_core_paths_doc`). |
| **Inbound deps** | `ade_runtime::network::{mux_pump, n2n_dialer}`. |
| **Outbound deps** | `ade_network::mux::frame` (BLUE), `ade_network::session` (GREEN), `tokio`. |
| **Entry points** | `ade_network::mux::transport::{MuxTransportHandle, spawn_duplex, open_tcp}`. |
| **Key modules** | `mux/transport.rs`. |

---

### `ade_network` *(RED capture binaries — non-session)*

| Attribute | Value |
|-----------|-------|
| **Purpose** | Operator-action capture binaries (live evidence harness) inside `ade_network`. The `ade_chain_sync_capture` binary (`bin/capture_chain_sync.rs`) gained `--intersect-slot` / `--intersect-hash` flags (N-X) to capture a real Conway `RollForward` golden fixture from an explicit intersection point. |
| **MUST NOT** | (1) Construct semantic types from raw bytes. (2) Overstate semantic truth in captured evidence. |
| **Outbound deps** | `ade_network` BLUE/GREEN submodules, `ade_codec`, `ade_types`, `tokio`. |

---

## Cross-Module Rules (project-wide)

### Dependency direction (RED → GREEN → BLUE; never outward)

```
ade_types     → {}
ade_codec     → {ade_types}
ade_crypto    → {ade_types, blake2, ed25519-dalek, cardano-crypto[vrf-draft03,kes-sum(test-only),dsign]}
ade_core      → {ade_types, ade_crypto, minicbor}
ade_ledger    → {ade_core, ade_plutus, ade_crypto, ade_codec, ade_types, minicbor, num-*}
ade_plutus    → {ade_crypto, ade_codec, ade_types, aiken_uplc, pallas-primitives(internal)}
ade_network   → BLUE submodules → {ade_codec, ade_types}; session(GREEN) → +BLUE submodules; transport(RED) → +tokio
ade_testkit   → {ade_core, ade_ledger, ade_plutus, ade_runtime, ade_crypto, ade_codec, ade_types, cardano-crypto}   [GREEN]
ade_runtime   → {ade_core, ade_crypto, ade_codec, ade_types, ade_ledger, ade_network, redb, serde*, bech32, base58, cardano-crypto[vrf-draft03,dsign], ed25519-dalek, tokio}   [RED]
ade_node      → {ade_types, ade_core, ade_crypto, ade_ledger, ade_runtime, ade_network, ade_codec, tokio}   [RED]
ade_core_interop → {ade_core, ade_codec, ade_crypto, ade_ledger, ade_runtime, ade_network, ade_testkit, ade_types, tokio}   [RED leaf]
```

**Forbidden directions.** Any BLUE crate (or BLUE `ade_network` submodule) depending on `ade_runtime`, `ade_node`, `ade_core_interop`, or the RED half of `ade_network` is a CI failure (`ci_check_dependency_boundary.sh`). Any non-`ade_plutus` crate referring to `pallas_*` is a CI failure (`ci_check_pallas_quarantine.sh`). Any `cardano_crypto::kes` import outside `#[cfg(test)]` under `crates/ade_crypto/src/**` is a CI failure (`ci_check_kes_sum_compatibility.sh` Guard 3). A second block-envelope encoder or a `produce_mode` direct-transport write is a CI failure (`ci_check_no_independent_forge_codepath.sh`, `ci_check_no_produce_mode_direct_transport_writes.sh`). A second `leader_vrf_input` authority, or a verification/construction path that accepts both TPraos and Praos VRF inputs for one era, is a CI failure (`ci_check_producer_praos_vrf.sh`, N-W). **A second `wrap_tag24`/`unwrap_tag24` definition, a hand-rolled tag-24 parse in RED, or a serve path emitting a bare `[era, block]` / bare header (not the composed tag-24 form) is a CI failure (`ci_check_tag24_wire_authority.sh`, N-X).**

### Naming convention

All crates are prefixed `ade_`. TCB color is not encoded in the crate name. The authoritative classifier is `.idd-config.json` `core_paths` + the per-file `// Core Contract:` / `//! BLUE|GREEN|RED` banner + the cluster TCB Color Maps; CI scripts hard-code their BLUE list.

### Closed enums / registries (for SEAMS cross-reference)

Closed semantic surfaces detected at HEAD: `AcceptedMiniProtocol` (mini-protocol registry, `ade_network::session`), `TagEnvelopeError` (closed tag-24 unwrap error enum, `ade_codec::cbor::tag24`, N-X), `ExpectedVrfInput` (2-variant era-discriminated leader-VRF-input authority, `ade_core::consensus::vrf_cert`, N-W), `LeaderCheckVerdict` (2-variant, `ade_core::consensus::leader_check`), `OutboundCommand` (typed relay, `ade_runtime::network::outbound_command`), `DispatchError`, `ProducerLogEvent` + `ForgeFailureReason` (closed evidence vocabulary — `ForgeFailureReason` carries `UnsupportedProducerEra` from N-W), `ChainEvolutionError`, `ServedChainAdmitError`, `KesError` / `KesParseError`, the admission/wire-only/live-log JSONL event vocabularies, the operator-evidence manifest TOML schema, the closed `CardanoEra` (with `is_praos()` classifier) / Conway cert + governance enums.

### CI enforcement (99 scripts under `ci/`)

The full list is mechanically obtainable via `ls ci/ci_check_*.sh` (99 at HEAD). New / load-bearing since the prior CODEMAP HEAD `01e7e08` (PHASE4-N-W → N-X) — for the upcoming SEAMS / HEAD_DELTAS / TRACEABILITY regens:

| Script | Enforces | Cluster |
|---|---|---|
| `ci_check_tag24_wire_authority.sh` | **NEW** CN-WIRE-08 — single tag-24 wrap/unwrap authority (`wrap_tag24`/`unwrap_tag24` each defined exactly once); no hand-rolled tag-24 parse in RED (`admission::runner` / `follow` call the shared authority); serve paths compose via `compose_blockfetch_block` / `compose_rollforward_header` (no bare `[era, block]` / bare header); both compositions pinned byte-identically against captured cardano-node 11.0.1 fixtures. | N-X |
| `ci_check_producer_praos_vrf.sh` | CN-FORGE-04 — single era→leader-VRF-input authority (`leader_vrf_input` defined exactly once); no TPraos/Praos fallback for one era; producer alpha = validator alpha for Praos; closed `ExpectedVrfInput`. | N-W |
| `ci_check_leader_check_authority.sh` | CN-FORGE-02 — BLUE leader-check has no LedgerView/EraSchedule/RED dep; never sees private keys; closed `LeaderCheckVerdict`. | N-R-A |
| `ci_check_no_independent_forge_codepath.sh` | CN-FORGE-03 (strengthened N-X) — single forge codepath; no parallel block serializer. | N-V |
| `ci_check_forge_decode_round_trip.sh` | CN-FORGE-03 (strengthened N-X) — `decode_block(forge_block(tick).bytes)` is `Ok`; forge output is the enveloped `[era, block]` form. | N-V |
| `ci_check_unsigned_header_preimage_single_source.sh` | CN-KES-HEADER-01 / DC-KES-HEADER-01 — single canonical pre-image recipe; branded `UnsignedHeaderPreImage`; pure. | N-S-A |
| `ci_check_no_produce_mode_direct_transport_writes.sh` | CN-OUTBOUND-RELAY-01 — `produce_mode` emits bytes only via `OutboundCommand` → `MuxPump`; no direct transport write. | N-S-B |
| `ci_check_operator_evidence_manifest_schema.sh` | CN-OPERATOR-EVIDENCE-01 — closed evidence-manifest TOML schema + peer-log SHA256 cross-check. | N-S-C |
| `ci_check_produce_mode_uses_bootstrap_initial_state.sh` | CN-NODE-01 / N-T — produce_mode obtains initial state only via `bootstrap_initial_state`. | N-T |
| `ci_check_producer_coordinator_no_secrets.sh` | CN-PROD-02 — GREEN coordinator never owns/stores private signing material. | N-Q |
| `ci_check_node_mode_closure.sh` | closed `ade_node` mode set. | N-Q |
| (the N-R-B served/snapshot gates) | `ci_check_served_chain_closure.sh`, `ci_check_snapshot_encoder_closure.sh` (carry-forward, exercised by per-peer dispatch). | N-R-B |

> Earlier-cluster scripts (N-A through N-P, the N-M-* admission/seed/WAL/anchor set, the N-L wire-session set) are present and counted in the 99 total. The per-script enforce/scope detail for those is in the registry's `ci_script` fields per rule.

---

## Generation notes

- Regenerated full at HEAD `273c887` (`git rev-parse --short HEAD`). PHASE4-N-X (N2N tag-24 wire envelope authority) is the cluster closing now.
- **Cluster-doc location:** all closed cluster docs (N-Q/N-R/N-S, N-M-*, N-O, N-P, N-T, N-V, N-W) are under `docs/clusters/completed/`; the only cluster directory outside `completed/` is the closing `PHASE4-N-X` (`cluster.md` + `CLOSURE.md`, archived after the grounding docs regenerate).
- All mechanical counts recomputed fresh from the tree (not copied): 11 crates, 446 canonical types (+1 from the closed `TagEnvelopeError` enum in `ade_codec`), 2033 tests (+17: `ade_codec` +11 tag-24/overflow-guard, `ade_network` +6 compose/decompose+served-shape oracle), 99 CI checks (+`ci_check_tag24_wire_authority.sh`), 292 registry rules (+1: new **CN-WIRE-08** introduced enforced; N-X also strengthened CN-FORGE-03, DC-CONS-17, DC-CONS-18).
- Counts are mechanical (commands in the Counts table). `canonical_type_registry: null`, so the canonical-type count is a structural grep over BLUE scopes.
- TCB color for every new/changed module was verified against the on-disk `// Core Contract:` / `//!` banner, not inferred from the cluster doc alone. The new `tag24` wrap/unwrap surface resolves BLUE under the `crates/ade_codec/` `core_paths` prefix; the per-protocol composers resolve BLUE under the `crates/ade_network/src/codec/` prefix; the RED unwraps (`ade_node::admission::runner`, `ade_core_interop::follow`) call into the BLUE authority.
- The doc is regenerated, not edited. If a value drifts, fix the source, not the doc.
