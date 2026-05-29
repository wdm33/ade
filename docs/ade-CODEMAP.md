# Module Authority Map — Ade

> **Status:** Living architectural document. Regenerated; not hand-edited.
> Per-project instance of `~/.claude/methodology/templates/codemap.md`.

> 11 crates, 452 canonical types, 2067 tests, 104 CI checks at HEAD (`5db9aae`, post-PHASE4-N-Y close + grounding-doc/atlas wiring).

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
5. The invariant registry `docs/ade-invariant-registry.toml` (298 rules) — each rule's `code_locus` + `statement` is the authoritative source for the **MUST NOT** rows below.

### Active cluster at HEAD

**PHASE4-N-Y — CLOSED (Mithril-anchored bootstrap, network forward-sync & WAL recovery).** Commit `3b78008` is the close commit; the cluster doc + closure record are archived under `docs/clusters/completed/PHASE4-N-Y/`. HEAD `5db9aae` is two post-close commits beyond the close: `f0d0bf9` ("ci: notify ade-atlas to rebuild on grounding-doc changes" — adds `.github/workflows/notify-atlas.yml`, a repo-orchestration workflow, **not** an Ade invariant gate) and `5db9aae` ("fix(registry): repair recovery.rs code_locus drift + add code-locus existence gate" — adds CI script #104 and repoints three rules' `code_locus` at the post-N-Y recovery paths). Neither post-close commit changes a module, a canonical type, or a test. The cluster's **primary invariant** is durability-before-tip: the network forward-sync path may never advance the persisted chain tip for a block before that block's preserved wire bytes and its Ade-canonical WAL entry have been written and acknowledged durable (DC-SYNC-01). It ships the Ade-side Mithril import provenance binding and the genesis/Conway cold-start source, both routed through the single closed bootstrap authority, and end-to-end crash-recovery wiring that reconciles the ChainDB to the WAL tail.

The cluster introduces / extends:
- **BLUE** `ade_ledger::bootstrap_anchor` — extends `BootstrapAnchor` with the closed `SeedProvenance` enum (`CardanoCliJson` / `Mithril { certificate_hash, certified_point, immutable_range }`); the canonical CBOR schema constant was **renamed `SCHEMA_VERSION → ANCHOR_SCHEMA_VERSION`** to disambiguate from the snapshot-framing `SCHEMA_VERSION` (DC-STORE-09) and bumped 1→2 additively/version-gated. New BLUE submodule `bootstrap_anchor::binding` — the pure `verify_mithril_binding` predicate + closed `MithrilImportError` + `MithrilManifestReport`.
- **BLUE** `ade_ledger::genesis_source` (NEW) — the pure `genesis_initial_state` Conway-genesis→initial-state transform + closed `GenesisSourceError::NonConwayEra`.
- **GREEN** `ade_runtime::forward_sync::reducer` (NEW, GREEN-by-content) — the closed `SyncEffect` plan + `AdmitPlan::durable` (sole `AdvanceTip` emitter) + `forward_sync_step`. **RED** `ade_runtime::forward_sync::pump` — the durability-ordered driver with the `TipBeforeDurable` fail-closed guard.
- **RED** `ade_runtime::mithril_import` (NEW), `ade_runtime::genesis_bootstrap` (NEW), and the `ade_runtime::recovery` module promoted to a dir (`recovery/restart.rs` — `recover_node_state`; the post-close `5db9aae` registry fix repointed DC-STORE-05 / T-REC-01 / T-REC-02 `code_locus` at `recovery/mod.rs` + `recovery/restart.rs`).
- **GREEN** `ade_testkit::harness::sync_diff` (NEW) — the observable-surface differential harness.

**Governance note (cluster §7).** Three structural decisions were ratified and are load-bearing for SEAMS: (1) the **single `bootstrap_initial_state` authority** is preserved — `genesis_bootstrap` and the Mithril path both route through it, never a parallel storage-init path (CN-NODE-01); (2) the **two-driver split** (GREEN reducer / RED pump) mirrors `session`/`mux_pump` — the reducer emits a closed effect plan, the RED pump applies it in durable order and is the only place that touches sockets/files; (3) **`WalEntry` stays a CE-not-law** — the WAL entry vocabulary is exercised as a cluster acceptance criterion (CE), not promoted into a frozen registry-law surface, so it remains additively evolvable behind the WAL schema version.

**Cluster-doc location.** Every closed cluster doc is archived under `docs/clusters/completed/` — including the entire **PHASE4-N-Q / N-R-\* / N-S-\*** set, the **N-M-\*** (admission/seed/WAL/anchor) sub-trees, **N-O**, **N-P**, **N-T**, **N-V**, **N-W**, **N-X**, and now **N-Y**. There is no cluster directory living outside `completed/` at this HEAD.

This regeneration is a **full inventory at HEAD `5db9aae`**. The N-M-\* admission/seed/WAL/anchor sub-trees, the N-Q/N-R/N-S producer/network/node sub-trees, the N-W Praos-VRF authority, the N-X tag-24 wire-envelope authority, and the N-Y Mithril-bootstrap / forward-sync / recovery surfaces are all inventoried below.

### Counts (mechanical, with sources)

| Count | Value | Source / command |
|---|---|---|
| Crates | **11** | `grep -cE '"crates/' Cargo.toml` (`[workspace] members`). **Δ vs prior (11): 0.** |
| Canonical types | **452** | `grep -rhE "^(pub )?(struct\|enum) " --include='*.rs'` over the 6 BLUE crate `src/` trees + the 9 BLUE `ade_network` submodule paths, summed. Breakdown: `ade_codec` 11, `ade_types` 81, `ade_crypto` 21, `ade_core` 49, `ade_ledger` **173**, `ade_plutus` 8 (= 343) + `ade_network` BLUE submodules 109 (`mux/frame.rs` 5, `codec/` 38, `handshake/` 9, `chain_sync/` 11, `block_fetch/` 10, `tx_submission/` 5, `keep_alive/` 5, `peer_sharing/` 5, `n2c/` 21). `canonical_type_registry: null`, so the structural grep is authoritative. **Δ vs prior CODEMAP (452): 0** — the two post-close commits added no canonical types. |
| Tests | **2067** | `grep -rEc "#\[test\]\|#\[tokio::test\]" --include='*.rs' crates/`, summed (`awk` over the `path:count` lines). Approximate per the template fallback rule (count of attributes, not a runner enumeration). **Δ vs prior (2067): 0** — the `5db9aae` registry fix touched only `code_locus` strings + added a shell gate; no new `#[test]`. |
| CI checks | **104** | `ls ci/ci_check_*.sh \| wc -l`. CI scripts live under `ci/` (`ci_dirs = ["ci"]`). A `.github/workflows/` dir now exists (`notify-atlas.yml`, added by `f0d0bf9`) but it is a grounding-doc→atlas-rebuild dispatch workflow, **not** an Ade invariant gate, so it is not counted as a CI check. **Δ vs prior (103): +1** — `ci_check_registry_code_locus_exists.sh` (added by `5db9aae`): a traceability drift guard — every `crates/**.rs` + `ci/**.sh` path cited in any registry rule's `code_locus` must exist on disk (560 code paths across 298 rules at HEAD; globs and `docs/` paths skipped; fails closed on a moved/renamed/deleted path). |
| Registry rules | **298** | `grep -cE "^id = " docs/ade-invariant-registry.toml`. Reference only — not a header count. **Δ vs prior (298): 0** — `5db9aae` repointed `code_locus` on DC-STORE-05, T-REC-01, T-REC-02 (the N-Y `recovery.rs → recovery/mod.rs` rename drift) but added/removed no rule. |

---

## BLUE Modules — Pure Functional Core

> **Shared header (applies to every BLUE entry below).** Every `.rs` source file begins with the `// Core Contract:` banner and the following deny attributes are present in each crate's `lib.rs` (or, for `ade_network`, at the crate root — BLUE submodules inherit them):
> `#![deny(unsafe_code)]`, `#![deny(clippy::unwrap_used)]`, `#![deny(clippy::expect_used)]`, `#![deny(clippy::panic)]`, `#![deny(clippy::float_arithmetic)]`.
>
> Cross-cutting BLUE CI gates (all scope the full BLUE set — 6 BLUE crates + 9 BLUE `ade_network` submodule paths):
> - `ci_check_module_headers.sh` — banner first-line check.
> - `ci_check_forbidden_patterns.sh` — `HashMap`/`HashSet`/`IndexMap`/`indexmap::`, `SystemTime`, `Instant`, `std::fs`, `std::net`, `tokio`, `async fn`, `f32`/`f64`, `anyhow`, `rand::thread_rng`, `thread::spawn`, `unsafe` outside an allowlist.
> - `ci_check_dependency_boundary.sh` — no BLUE crate depends on a RED crate.
> - `ci_check_no_signing_in_blue.sh` — signing operations forbidden in BLUE.
> - `ci_check_no_semantic_cfg.sh` — `#[cfg(feature = …)]` / `cfg!(feature = …)` forbidden in BLUE.
> - `ci_check_hash_uses_wire_bytes.sh` — no hashing of re-encoded bytes in BLUE.
> - `ci_check_ingress_chokepoints.sh` — only named `decode_*` chokepoints construct `PreservedCbor`.
> - `ci_check_pallas_quarantine.sh` — `pallas-*` confined to `ade_plutus`.
> - `ci_check_no_async_in_blue.sh` — async constructs forbidden in BLUE (DC-CORE-01).
>
> Narrow BLUE gates: `ci_check_no_chaindb_in_consensus_blue.sh`, `ci_check_no_float_in_consensus.sh`, `ci_check_consensus_closed_enums.sh`, `ci_check_no_density_in_fork_choice.sh` (DC-CONS-03), `ci_check_deposit_param_authority.sh` (DC-TXV-07), `ci_check_gov_cert_accumulation_closed.sh` (DC-LEDGER-09), `ci_check_credential_discriminant_closed.sh` (DC-LEDGER-10), `ci_check_proposal_procedures_closed.sh` (DC-LEDGER-11), `ci_check_conway_cert_classification_closed.sh`, the N-C producer-authority gate set, the N-G server-role gate set, the N-H receive gate set, the N-I/N-J rollback+snapshot gate set, the mempool-ingress gate set, the N-P KES gate, the N-R/N-S/N-V BLUE gates, the N-W gate `ci_check_producer_praos_vrf.sh` (CN-FORGE-04), the N-X gate `ci_check_tag24_wire_authority.sh` (CN-WIRE-08), the N-Y gates `ci_check_forward_sync_chokepoint_only.sh` (DC-SYNC-01 — durable-before-tip; the GREEN reducer's single `AdvanceTip` emitter), `ci_check_mithril_uses_bootstrap_initial_state.sh` (CN-MITHRIL-01 — Mithril path routes through the single bootstrap authority and the BLUE binding predicate), and `ci_check_no_haskell_fingerprint_equality.sh` (DC-COMPAT-01 — observable-surface-only compatibility, never internal-state-hash equality), plus the **NEW post-close** workspace-wide traceability gate `ci_check_registry_code_locus_exists.sh` (every registry `code_locus` path under `crates/**.rs` or `ci/**.sh` must exist on disk).

---

### `ade_codec`

| Attribute | Value |
|-----------|-------|
| **Purpose** | Owns Cardano-canonical CBOR ingress — the only place in the workspace that turns raw bytes into typed semantic values, with wire-byte preservation for every hash-bearing structure. Owns the standalone opcert byte authority and the canonical Conway-tx preserved-byte splitter, the canonical block-envelope encoder `cbor::envelope::encode_block_envelope`, and (N-X) the single workspace **tag-24 CBOR-in-CBOR wire-envelope authority** `cbor::tag24::{wrap_tag24, unwrap_tag24}` + closed `TagEnvelopeError`. |
| **Creates** | `PreservedCbor<T>`, `RawCbor`, `BlockEnvelope`, `ByronDecodedBlock`, `CodecContext`, `CodecError` (incl. `UnknownCertTag`, `DuplicateMapKey`, `TrailingBytes`, `InvalidCborStructure`), `ContainerEncoding`, `IntWidth`, era-tagged block/tx wrappers, `OpCertCodecError`, `TxComponents<'a>`, `TagEnvelopeError` (closed: `NotTag24`, `NotByteString`, `Truncated`, `TrailingBytes`). Functions: `conway::cert::{decode_conway_certs, decode_drep}`, `conway::withdrawals::{decode_withdrawals, withdrawals_sum}`, `conway::governance::{decode,encode}_proposal_procedures`, `shelley::opcert::{encode_opcert, decode_opcert}`, `shelley::tx_components::split_conway_tx_components`, `cbor::envelope::{decode_block_envelope, encode_block_envelope}`, `cbor::tag24::{wrap_tag24, unwrap_tag24}`. |
| **Interprets** | All canonical Cardano CBOR — block envelopes (`[era, block]`), era-specific blocks, tx bodies/outs, certificates, addresses. `unwrap_tag24` is the single workspace authority that strips a tag-24 (`0xd8 0x18`) envelope, returning a zero-copy borrow of the verbatim inner bytes (no re-encode) and failing closed with a typed `TagEnvelopeError`. `cbor::mod::{read_bytes, read_text, skip_item}` fail-close (no panic) on an overflowing declared length via `checked_add`. |
| **MUST NOT** | (1) Construct `PreservedCbor` outside `ade_codec` (`pub(crate)` + `ci_check_ingress_chokepoints.sh`). (2) Re-encode wire bytes when computing hashes. (3) Use any forbidden BLUE pattern. (4) Depend on any workspace crate except `ade_types`. (5) `conway::cert` (DC-LEDGER-08) — no unknown-tag swallow. (6) `conway::withdrawals` — no last-wins on duplicate `RewardAccount`. (7) `decode_stake_credential` (DC-LEDGER-10) — must not erase the credential tag. (8) `conway::governance` (DC-LEDGER-11) — no silent skip on unknown `GovAction`. (9) `shelley::opcert` (DC-CONS-11/12). (10) `shelley::tx_components` (DC-CONS-13/16). (11) `encode_block_envelope` (CN-FORGE-03) — the workspace's single block-envelope encoder; no parallel serializer. (12) `cbor::tag24` (CN-WIRE-08) — `wrap_tag24`/`unwrap_tag24` defined exactly once; `unwrap_tag24` fails closed (typed `TagEnvelopeError`, never panic); inner bytes copied verbatim / zero-copy borrowed; `read_bytes`/`read_text`/`skip_item` reject an overflowing declared length without panic (`ci_check_tag24_wire_authority.sh`). |
| **Inbound deps** | `ade_ledger` (heavy), `ade_plutus`, `ade_testkit`, `ade_network`, `ade_runtime`, `ade_core_interop`, `ade_node`. |
| **Outbound deps** | `ade_types`. std-only; dev-deps `serde_json`, `toml`. |
| **Entry points** | `ade_codec::cbor::envelope::{decode_block_envelope, encode_block_envelope}`, `ade_codec::{wrap_tag24, unwrap_tag24, TagEnvelopeError}` (crate-root re-exports), `ade_codec::traits::AdeEncode`, `ade_codec::CodecContext`, per-era `decode_*_block`, `ade_codec::address::decode_address`, `ade_codec::conway::tx::decode_conway_tx_body`, `ade_codec::conway::cert::decode_conway_certs`, `ade_codec::conway::governance::{decode,encode}_proposal_procedures`, `ade_codec::shelley::opcert::{encode_opcert, decode_opcert}`, `ade_codec::shelley::tx_components::split_conway_tx_components`. |
| **Key modules** | `cbor/` (incl. `envelope.rs`, `tag24.rs`, `mod.rs`), `byron/`, `shelley/` (incl. `cert.rs`, `opcert.rs`, `tx_components.rs`), `allegra/`, `mary/`, `alonzo/`, `babbage/`, `conway/`, `address/`, `preserved.rs`, `traits.rs`, `primitives.rs`, `error.rs`. |

---

### `ade_core`

| Attribute | Value |
|-----------|-------|
| **Purpose** | BLUE authoritative Praos consensus core. Owns canonical types and pure state-transitions deciding which header/chain Ade accepts: HFC era schedule + slot↔era↔time, Praos chain-dep state, VRF cert verification + leader-eligibility, KES + op-cert period verification, header validation, fork-choice, header-level rollback, leader-schedule query, canonical encodings, producer-side opcert acceptance, the closed BLUE leader-check evaluator (`consensus::leader_check`, N-R-A), and the single era→leader-VRF-input authority `consensus::vrf_cert::{ExpectedVrfInput, leader_vrf_input, leader_value_for}` (N-W). |
| **Creates** | **Schedule:** `BootstrapAnchorHash`, `EraSchedule`, `EraSummary`, `EraLocation`. **State:** `PraosChainDepState`, `OpCertCounterMap`, `Nonce`. **Events/points:** `Point`, `ChainHash`, `BlockDistance`, `SecurityParam`, `ChainEvent`, `ChainSelectionReject`. **Errors:** `HFCError`, `SlotTimeError`, `HeaderValidationError`, `VrfCertError`, `OpCertCounterError`, `NonceEvolutionError`, `LeaderScheduleError`, `OpCertError`. **Header surface:** `HeaderInput`, `HeaderVrf`, `HeaderKes`, `ValidatedHeaderSummary`, `HeaderApplied`. **Fork-choice:** `TiebreakerView`, `CandidateFragment`, `ChainSelectorState`, `ForkChoiceError`. **VRF/leader:** `VrfRole`, `VerifiedVrf`, `StakeFraction`, `ActiveSlotsCoeff`, `LeaderScheduleQuery`, `LeaderScheduleAnswer`, `ExpectedVrfInput` (closed 2-variant era-discriminated leader-VRF-input enum). **Boundary:** `LedgerView` trait. **Rollback:** `RollBackRequest`, `RollBackApplied`. **`leader_check`:** `LeaderCheckVerdict`, `LeaderCheckError`, `VrfOutputFingerprint`, `LeaderProofFingerprint`. 49 public types. |
| **Interprets** | Canonical inputs from the `ade_runtime` shell. KES check verifies the hot-KES signature via `ade_crypto::kes::verify_kes`. `leader_vrf_input(era, slot, eta0)` constructs the era-correct alpha; `leader_value_for` applies the era-correct range-extension. `leader_check::verify_and_evaluate_leader` consumes only public-key material + canonical inputs and fail-closes on full-enum `ExpectedVrfInput` mismatch. `opcert_validate` consumes an `OperationalCert`, cold key, expected period, prev counter. |
| **MUST NOT** | (1)–(14) consensus carry-forward (no I/O, no clock, no `ChainDb` in BLUE consensus, no density in fork-choice, closed consensus enums, etc.). (15) `consensus::leader_check` (CN-FORGE-02) — MUST NOT depend on `LedgerView`/`EraSchedule`/`ChainDepState`/wall-clock/storage/any RED crate; MUST NOT observe `KesSecret`/`VrfSigningKey`/`ColdSigningKey`; `NotEligible` carries only a bounded fingerprint. (16) `consensus::vrf_cert` leader-VRF authority (CN-FORGE-04) — `leader_vrf_input` is the single era→construction authority; no path accepts both TPraos and Praos inputs for one era; Praos producer alpha = validator alpha (`ci_check_producer_praos_vrf.sh`). |
| **Inbound deps** | `ade_ledger`, `ade_runtime` (heavy), `ade_testkit`, `ade_core_interop`, `ade_node`. |
| **Outbound deps** | `ade_types`, `ade_crypto`, `minicbor`. Dev-deps: `ade_testkit`, `serde_json`, `cardano-crypto`. |
| **Entry points** | `use ade_core::consensus::{…}` aggregator; `validate_and_apply_header`, `select_best_chain`, `apply_rollback`, `apply_nonce_input`, `apply_op_cert`, `query_leader_schedule`, `verify_vrf_cert`, `tiebreaker_prefer`; `ade_core::consensus::opcert_validate::opcert_validate`; `ade_core::consensus::leader_check::{verify_and_evaluate_leader, LeaderCheckVerdict, is_leader_for_vrf_output}`; `ade_core::consensus::vrf_cert::{ExpectedVrfInput, leader_vrf_input, leader_value_for, praos_vrf_input, praos_leader_value}`. |
| **Key modules** | `consensus/{era_schedule, praos_state, events, errors, vrf_cert, kes_check, nonce, op_cert, leader_schedule, header_summary, header_validate, candidate, fork_choice, rollback, ledger_view, encoding, opcert_validate, leader_check}.rs`. |

---

### `ade_crypto`

| Attribute | Value |
|-----------|-------|
| **Purpose** | Pure cryptographic verification primitives — Blake2b hashing, Ed25519 / Byron-bootstrap signature verification, KES verification with operational certificates, VRF verification, plus closed signature-artifact types. Verification only — signing lives in RED `ade_runtime::producer::signing`. Also owns the Ade-native `Sum6KES Ed25519DSIGN` algorithm (`kes_sum/`), byte-identical to Haskell `cardano-base`. |
| **Creates** | `Blake2b224`, `Blake2b256`, `HashAlgorithm`, `Ed25519VerificationKey`, `Ed25519Signature`, `ByronExtendedVerificationKey`, `KesVerificationKey`, `KesPeriod`, `OperationalCertData`, `VrfVerificationKey`, `VrfProof`, `VrfOutput`, `CryptoError`, `KesSignature`, `SUM6_KES_SIG_LEN = 448`. **`kes_sum::`** (BLUE): `KesAlgorithm` trait, `Sum0Kes`, `Sum0SigningKey`, `Sum0Signature`, `SumKes<D>`, `SumSigningKey<D>`, `SumSignature<D>`, aliases `Sum1Kes..Sum6Kes`, closed `KesError`, closed `KesParseError`. |
| **Interprets** | Already-decoded byte slices. `Sum6Kes::raw_deserialize_signing_key_kes` interprets the canonical 608-byte expanded cardano-cli skey; `raw_deserialize_signature_kes` the 448-byte signature; `period_from_zeroed_sum6_tree_shape` infers the current period from tree shape. |
| **MUST NOT** | (1) Implement signing as a top-level `pub fn sign_*` (`ci_check_no_signing_in_blue.sh`). (2) Allocate global state. (3) Use any BLUE-forbidden pattern. (4) `unsafe` outside the allowlisted FFI in `vrf.rs`. (5) `build_opcert_signable` must produce the spec-correct concat. (6) `KesSignature` (DC-CRYPTO-04) — closed length-pinned wrapper, `from_bytes` only, redacting `Debug`. (7) `kes_sum` (DC-CRYPTO-08) — MUST NOT import `cardano_crypto::kes::*` in production paths (`#[cfg(test)]` oracle only; `ci_check_kes_sum_compatibility.sh` Guard 3). (8) `kes_sum::hash::expand_seed` — Haskell prefix bytes `0x01`/`0x02`. (9) every `*SigningKey` hand-rolls zeroizing `Drop` + redacting `Debug` + no public byte accessors. (10) `kes_sum` corpus — throwaway-fixture comment; no `.skey` files (Guards 1, 2). (11) error variants carry only non-secret primitives. |
| **Inbound deps** | `ade_core`, `ade_ledger`, `ade_plutus`, `ade_testkit`, `ade_core_interop`, `ade_runtime`. |
| **Outbound deps** | `ade_types`, `blake2`, `ed25519-dalek`, `cardano-crypto` (`["vrf-draft03", "kes-sum", "dsign"]`, `default-features = false`; `kes-sum` retained for `#[cfg(test)]` oracle only). |
| **Entry points** | `ade_crypto::blake2b::*`, `verify_ed25519`, `verify_byron_bootstrap`, `verify_kes`, `verify_opcert`, `verify_vrf`, `ade_crypto::kes::{KesSignature, SUM6_KES_SIG_LEN, KesVerificationKey}`, `ade_crypto::kes_sum::{KesAlgorithm, Sum6Kes, KesError, KesParseError, period_from_zeroed_sum6_tree_shape}`. |
| **Key modules** | `blake2b.rs`, `ed25519.rs`, `error.rs`, `kes.rs`, `kes_sum/` (`mod`, `single`, `sum`, `hash`, `errors`, `period`, `cardano_cli_corpus` `#[cfg(test)]`, `tests` `#[cfg(test)]`), `traits.rs`, `vrf.rs`. |

---

### `ade_ledger`

| Attribute | Value |
|-----------|-------|
| **Purpose** | The functional core (ledger half): stateless ledger rules for every era; B1 block-validity verdict; B2 transaction-validity verdict + mempool admission; B3 Conway value-conservation; B4/B5 closed Conway cert-state + governance-cert accumulation; live committee enactment; the single BLUE wire-ingress chokepoint `mempool::ingress::mempool_ingress`; the BLUE producer authority (`producer::{forge, self_accept, served_chain, state}`); the single canonical body-hash authority (`block_body_hash`); the BLUE receive-side bridge (`receive::*`); the BLUE rollback authority (`rollback::*`); the BLUE canonical snapshot encoder/decoder (`snapshot::*`); the BLUE Ade-native WAL replay+event authority (`wal::*`); the BLUE bootstrap-anchor + Mithril-binding types (`bootstrap_anchor::*`); the BLUE Conway-genesis→initial-state transform (`genesis_source`, N-Y); the canonical KES-signing pre-image recipe `block_validity::unsigned_header_pre_image`. |
| **Creates** | ~173 public types. Load-bearing: `BlockValidityError`, `BlockVerdict`, `TxValidity*`, `MempoolState`, `ProducerTick`, `ForgedBlock`, `ForgeError`, `ForgeEffects`, `ServedChainSnapshot`, `ServedChainAdmitError`, `PoolDistrView`, `LedgerState`, `WalEvent`, `WalEntry`, `BootstrapAnchor`, `SeedProvenance` (closed: `CardanoCliJson` / `Mithril { certificate_hash, certified_point, immutable_range }`), `SeedPoint`, `ANCHOR_SCHEMA_VERSION = 2`, `MithrilManifestReport`, `MithrilImportError` (closed: `NetworkMagicMismatch`, `GenesisHashMismatch`, `CertifiedPointMismatch`, `CertificateHashMismatch`, `UnsupportedArtifactType`), `BootstrapAnchorError`, `GenesisInitialFund`, `ConwayGenesisConfig`, `GenesisSourceError` (closed: `NonConwayEra`), `UnsignedHeaderPreImage`, `UnsignedHeaderPreImageError`. |
| **Interprets** | Canonical decoded blocks/txs/certs/snapshots; `mempool_ingress` is the sole BLUE chokepoint from wire ingress to `admit`; `self_accept` re-validates a forged artifact; `served_chain_admit` derives the served index; `decode_bootstrap_anchor` reads the version-gated `ANCHOR_SCHEMA_VERSION = 2` CBOR framing (rejects an unknown version); `verify_mithril_binding(report, anchor)` cross-checks the Mithril manifest's attested `{network_magic, genesis_hash, certified_point, certificate_hash}` against the independently-minted anchor; `genesis_initial_state(&ConwayGenesisConfig)` produces the `(LedgerState, PraosChainDepState)` cold-start pair; `unsigned_header_pre_image` produces the canonical CBOR of `ShelleyHeaderBody`. |
| **MUST NOT** | All carry-forward ledger prohibitions (no I/O, no clock, closed cert/governance enums, single body-hash recipe, etc.). **`producer::forge` (CN-FORGE-01/03):** emit exactly one of `ForgeSucceeded`/`ForgeNotLeader`/`ForgeFailed`; no `ForgeSucceeded` unless `self_accept` accepts; wrap output via `ade_codec::encode_block_envelope`; no parallel forge codepath. **`producer::served_chain` (CN-PROD-04):** only self-accepted blocks admitted; sole entry `served_chain_admit`. **`block_validity::unsigned_header_pre_image` (CN-KES-HEADER-01 / DC-KES-HEADER-01):** branded `UnsignedHeaderPreImage`'s only constructor is the canonical recipe; byte-identical to the validator-side extractor; pure. **`bootstrap_anchor` (CN-ANCHOR-01 / DC-ANCHOR-01, strengthened N-Y):** `ANCHOR_SCHEMA_VERSION` is version-gated — `decode_bootstrap_anchor` MUST reject an unknown schema version and MUST be byte-canonical (round-trips); the `SeedProvenance` enum is closed (no open/wildcard variant). **`bootstrap_anchor::binding` (CN-MITHRIL-01 / DC-MITHRIL-01):** `verify_mithril_binding` is a pure predicate that MUST fail closed (typed `MithrilImportError`) on any of the four field mismatches or an unsupported artifact type; MUST NOT re-verify the STM multisig (that is the mithril-client's job); MUST NOT be a tautological self-check — the report side and the anchor side originate independently (the manifest vs the `--json-seed`-minted anchor). **`genesis_source` (DC-GENESIS-SRC-01):** `genesis_initial_state` is Conway-only — fail-closed `GenesisSourceError::NonConwayEra` for any other era; MUST NOT be a second bootstrap/storage-init authority and introduces no `*Anchor` trait/plugin seam. |
| **Inbound deps** | `ade_testkit`, `ade_core_interop`, `ade_runtime`, `ade_node`. |
| **Outbound deps** | `ade_types`, `ade_crypto`, `ade_codec`, `ade_plutus`, `ade_core`, `minicbor`, `num-bigint`, `num-integer`, `num-traits`. Dev-dep: `ade_testkit`. |
| **Entry points** | `ade_ledger::block_validity::{decode_block, validate_block}`, `ade_ledger::tx_validity::*`, `ade_ledger::mempool::{ingress::mempool_ingress, admit::*}`, `ade_ledger::producer::{forge::forge_block, self_accept::self_accept, served_chain::{ServedChainSnapshot, served_chain_admit}}`, `ade_ledger::block_body_hash::*`, `ade_ledger::receive::reducer::*`, `ade_ledger::rollback::*`, `ade_ledger::snapshot::*`, `ade_ledger::wal::*`, `ade_ledger::bootstrap_anchor::{encode_bootstrap_anchor, decode_bootstrap_anchor, BootstrapAnchor, SeedProvenance, verify_mithril_binding, MithrilManifestReport, MithrilImportError}`, `ade_ledger::genesis_source::{genesis_initial_state, ConwayGenesisConfig, GenesisSourceError}`, `ade_ledger::block_validity::unsigned_header_pre_image::unsigned_header_pre_image`. |
| **Key modules** | `block_validity/` (incl. `unsigned_header_pre_image.rs`, `header_input.rs`, `transition.rs`, `verdict.rs`), `tx_validity/`, `mempool/`, `cert_classify/`, `consensus_view/`, `producer/` (`forge`, `self_accept`, `served_chain`, `state`), `block_body_hash.rs`, `receive/`, `rollback/`, `snapshot/`, `wal/`, `bootstrap_anchor/` (`mod`, `anchor`, `binding`, `error`), `genesis_source.rs`. |

> **GREEN sub-classification:** `ade_ledger::mempool::{policy, canonicalize}` are GREEN-by-content. **RED sub-classification:** `ade_ledger::consensus_input_extract` is RED-by-content. Both carry their own module banner.

---

### `ade_network` *(BLUE submodules)*

| Attribute | Value |
|-----------|-------|
| **Purpose** | Owns the Cardano Ouroboros mini-protocol authority — the closed wire grammar (CBOR codecs) and pure state machines for all N2N + N2C mini-protocols, plus the BLUE mux frame primitive and the producer-side server-role reducers. Owns the per-protocol tag-24 **composition** authorities layered over `ade_codec::{wrap_tag24, unwrap_tag24}` — `codec::block_fetch::{compose,decompose}_blockfetch_block` (era **inside** the wrap; Conway = storage index 7) and `codec::chain_sync::{compose,decompose}_rollforward_header` + `chain_sync_wire_era_index` (era_tag **outside** the wrap; CONSENSUS era index Conway = 6). |
| **Creates** | 109 BLUE canonical types: `mux/frame.rs` 5, `codec/` 38, `handshake/` 9, `chain_sync/` 11, `block_fetch/` 10, `tx_submission/` 5, `keep_alive/` 5, `peer_sharing/` 5, `n2c/` 21. `chain_sync::server::HeaderProjection` carries an `era: CardanoEra` field. |
| **Interprets** | Mini-protocol wire frames; `mux::frame::decode_frame` is the single frame decode authority; per-protocol `*_transition` reducers consume decoded messages. `decompose_blockfetch_block` / `decompose_rollforward_header` strip the per-protocol tag-24 composition (delegating the inner unwrap to `ade_codec::unwrap_tag24`). |
| **MUST NOT** | (1) `mux::frame` — single `encode_frame`/`decode_frame` pair. (2) handshake — single `n2n_transition` + single `n2c_transition`. (3) closed `AcceptedMiniProtocol` registry. (4) no socket I/O in BLUE submodules. (5) no async/tokio in BLUE submodules. (6) depend on no workspace crate beyond `ade_codec` + `ade_types`. (7) server reducers MUST NOT split headers in parallel or depend on signing. (8) tag-24 composition (CN-WIRE-08) — composers delegate byte-level wrap/strip to the single `ade_codec` authority; a served BlockFetch `MsgBlock` payload is `tag24(bytes([era, block]))` and a served ChainSync `RollForward` header is `[era_tag, tag24(bytes(header_cbor))]`; both pinned byte-identically against captured cardano-node 11.0.1 fixtures (`ci_check_tag24_wire_authority.sh`). |
| **Inbound deps** | `ade_runtime`, `ade_node`, `ade_core_interop`, `ade_testkit` (and the `ade_network` GREEN/RED submodules). |
| **Outbound deps** | `ade_types`, `ade_codec`. No external deps in BLUE submodules. |
| **Entry points** | `ade_network::mux::frame::{encode_frame, decode_frame}`, `ade_network::handshake::{n2n_transition, n2c_transition}`, `ade_network::chain_sync::{server::*, client::*}`, `ade_network::block_fetch::{server::*, client::*}`, `ade_network::codec::block_fetch::{compose,decompose}_blockfetch_block`, `ade_network::codec::chain_sync::{compose,decompose}_rollforward_header, chain_sync_wire_era_index}`, `ade_network::{tx_submission, keep_alive, peer_sharing, n2c}::*`. |
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
| **Purpose** | Owns the canonical Cardano domain vocabulary — primitives, era enum, address forms, era-tagged tx bodies/outputs/certificates, governance types — the lingua franca for every other crate. `CardanoEra` carries the `is_praos()` classifier (Babbage + Conway). |
| **Creates** | `CardanoEra` (with `is_praos()`), `SlotNo`, `BlockNo`, `EpochNo`, `Hash28`, `Hash32`, `Coin`, `Lovelace`, `NetworkId`, `Nonce`, `TxIn`, `RewardAccount`, `PoolId`, `Address`, `ByronAddress`, `Credential`, `StakeCredential`, `Certificate`, `PoolRegistrationCert`, `ConwayCert`, `MIRCert`, `DRep`, `GovAction`, `GovActionState`, `GovActionId`, `Anchor`, `ProposalProcedure`, `OperationalCert`, `NativeScript`, `Datum`, `MultiAsset`, `AssetName`, `CostModel`, `ExUnits`, plus per-era tx-body/tx-out/witness wrappers and `ShelleyBlock`/`ShelleyHeader`/`VrfData`/`ProtocolVersion`. |
| **Interprets** | None — produce-only. |
| **MUST NOT** | (1) Construct or decode `PreservedCbor`. (2) Use any BLUE-forbidden pattern. (3) Depend on any workspace crate. (4) Add open/extensible variants to closed enums without a versioned gate (`CardanoEra` discriminants are closed: Byron=0 … Conway=7; `is_praos()` classifies exactly {Babbage, Conway}). |
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
| **Purpose** | Test infrastructure: differential harness, oracle snapshot loader, regression corpus, genesis loader, era mapping, transcript replay, diff reporting; consensus / block-validity / tx-validity / mempool-ingress / proposal-procedures / producer test harnesses; the N-Y observable-surface snapshot→tip differential harness (`harness::sync_diff`). |
| **Creates** | Harness + corpus types (`ConwayValidityCorpus`, differential replay drivers, snapshot loaders). **N-Y `harness::sync_diff`:** `BlockVerdict` (closed observable-surface verdict string), `ObservableBlockSurface`, `UtxoEntry`, `SyncOracleFixture`, `AdeObservedSurfaces`, `SyncRegressionFixture`, `RegressionFixtureViolation`. Not canonical-counted (GREEN). |
| **MUST NOT** | (1) Affect authoritative outputs. (2) Introduce nondeterminism into the BLUE-under-test path. (3) Construct semantic types bypassing the canonical decoders. **`harness::sync_diff` (DC-COMPAT-01):** MUST NOT compare Ade's internal ledger `fingerprint` to a Haskell / serialized-state hash — compatibility is proven only on observable surfaces (per-block verdict, selected tip hash, block hash, `query utxo`-style UTxO set); it decides nothing about authority — it compares already-authoritative outputs (`ci_check_no_haskell_fingerprint_equality.sh`). |
| **Inbound deps** | None at compile time; consumed via integration tests + dev-dep links. |
| **Outbound deps** | `ade_types`, `ade_codec`, `ade_crypto`, `ade_core`, `ade_ledger`, `ade_plutus`, `ade_runtime`, `blake2`, `flate2`, `tar`, `serde`, `serde_json`, `toml`, `cardano-crypto` (dev). |
| **Entry points** | `ade_testkit::validity::corpus::ConwayValidityCorpus`, the differential `*_replay` drivers, the snapshot loaders, the genesis loader, `ade_testkit::harness::sync_diff::{sync_differential_snapshot_to_tip, diff_observable_surfaces, parse_sync_oracle_fixture, load_committed_regression_fixtures, validate_regression_fixture}`. |
| **Key modules** | `consensus/`, `validity/`, `tx_validity/`, `mempool/`, `governance/`, `producer/`, `harness/` (incl. `sync_diff.rs`, `diff_report.rs`), differential + corpus infrastructure. |

---

### `ade_network::session` *(GREEN by content — PHASE4-N-L)*

| Attribute | Value |
|-----------|-------|
| **Purpose** | The pure session driver. Composes the BLUE authorities `mux::frame::{encode,decode}_frame`, `handshake::n2n_transition`, and per-mini-protocol state machines through `session::core::step`; owns the partial-frame buffer + `SessionState` type-state + closed `AcceptedMiniProtocol` registry + per-mini-protocol payload reassembly. |
| **Creates** | ~12 closed types across `session::{event, state, demux, core, handshake_driver}`. |
| **MUST NOT** | (1) `session::core::step` is the only pub reducer in `session/`. (2) no `tokio::*` imports in session core. (3) `AcceptedMiniProtocol` closed. (4) no unbounded queue (`ci_check_session_no_unbounded.sh`). (5) affect authoritative outputs. |
| **Inbound deps** | `ade_runtime::network::{mux_pump, n2n_dialer}`, `ade_runtime::orchestrator::keep_alive_session`. |
| **Outbound deps** | `ade_network` BLUE submodules, `ade_codec`, `ade_types`. |
| **Entry points** | `ade_network::session::core::step`, `ade_network::session::{state::SessionState, event::*, demux::*, handshake_driver::*}`. |
| **Key modules** | `session/{mod, core, event, state, demux, handshake_driver}.rs`. |

---

### `ade_runtime` GREEN-by-content sub-trees

> All carry a `//! GREEN …` banner inside the RED `ade_runtime` crate, the BLUE deny attributes, and a purity CI gate. Each is "promotable to BLUE on demand, never demotable to RED" unless noted.

| Sub-tree | Purpose | MUST NOT | CI gate |
|---|---|---|---|
| `forward_sync::reducer` *(N-Y, NEW)* | GREEN forward-sync lifecycle reducer. Composes the BLUE admit chokepoint (`ade_ledger::receive::receive_apply` / `admit_via_block_validity`) and emits a closed `SyncEffect` plan (`StoreBlockBytes`, `AppendWal`, `CommitCheckpoint`, `AdvanceTip`); the private `AdmitPlan::durable` is the sole `AdvanceTip` emitter, fixing the durable-before-tip order. | Emit `AdvanceTip` for a block before that block's `StoreBlockBytes` + `AppendWal` (DC-SYNC-01) — structurally unrepresentable since `AdmitPlan` has no public out-of-order constructor; affect authoritative outputs; introduce nondeterminism. | `ci_check_forward_sync_chokepoint_only.sh` |
| `producer::coordinator` *(N-Q)* | Pure-state-machine coordinator for live producer-mode. Emits closed `CoordinatorEffect`s. | Own/store private signing material (CN-PROD-02); affect authoritative outputs. | `ci_check_producer_coordinator_no_secrets.sh` |
| `producer::producer_log` *(N-Q, extended N-W)* | Closed-vocabulary producer evidence log — closed `ProducerLogEvent` + closed `ForgeFailureReason` (incl. `UnsupportedProducerEra`). | Add an open/wildcard event variant; emit `BlockServed` for a block not in the served snapshot (CN-PROD-04). | `ci_check_operator_evidence_manifest_schema.sh` |
| `producer::chain_evolution` *(N-T)* | Linear `ChainEvolution` typestate threading the producer's chain state forward. | Never mints `AcceptedBlock` (CE-T-7); advance on authority disagreement (CE-T-6b); introduce nondeterminism. | gated by CE-T-7; purity inherited |
| `producer::{broadcast_to_served, served_chain_lookups}` *(N-G)* | GREEN glue computing served-chain broadcast targets + read-side lookups. | Affect authoritative outputs; serve a block that failed self-accept. | `ci_check_broadcast_to_served_purity.sh` |
| `bootstrap` *(N-K)* | Sole `bootstrap_initial_state` authority (CN-NODE-01, strengthened N-Y). | Be a parallel bootstrap path; produce_mode / genesis_bootstrap / Mithril path obtain initial state only here. | `ci_check_bootstrap_closure.sh`, `ci_check_node_binary_uses_single_bootstrap.sh`, `ci_check_mithril_uses_bootstrap_initial_state.sh` |
| `clock` (`DeterministicClock` + trait) *(N-K)* | GREEN clock trait + deterministic impl (DC-NODE-03). | DeterministicClock must read no wall-clock. | `ci_check_clock_seam.sh` |
| `orchestrator::{mod, event, state, core}` *(N-K)* | GREEN core reducer + closed-vocabulary event + state for the node orchestrator. | tokio imports in core; open event vocabulary. | `ci_check_orchestrator_core_purity.sh` |
| `rollback::{cadence, in_memory_cache, chaindb_block_source, persistent_cache, persistent_writer}` *(N-I/N-J/N-K)* | GREEN rollback adapter glue + snapshot cadence + persistent cache/writer (DC-NODE-02, DC-CONS-21). | Parallel snapshot cadence; affect authoritative rollback state. | `ci_check_persistent_writer_no_parallel_cadence.sh`, `ci_check_snapshot_cadence_purity.sh` |
| `receive::{events_to_state, in_memory_chain_write}` *(N-H)* | GREEN receive-side glue mapping BLUE receive events to in-memory chain writes. | Affect authoritative receive verdict. | `ci_check_receive_replay_purity.sh` |
| `seed_import` *(N-M-A)* | Single authority converting a cardano-cli JSON UTxO dump into canonical seed entries. | Construct semantic types bypassing canonical decoders. | `ci_check_seed_import_closure.sh`, `ci_check_seed_import_full_preprod_support.sh` |
| `bootstrap_anchor` *(N-M-A)* | RED-shell composer minting `BootstrapAnchor` from import inputs (the *types* + binding predicate are BLUE in `ade_ledger`). | Mint an anchor outside this composer; bind a Mithril anchor without the BLUE `verify_mithril_binding` cross-check. | `ci_check_bootstrap_anchor_closure.sh` |
| `wal` *(N-M-A)* | File-backed Ade-native WAL (append-only). | Mutate/rewrite committed WAL entries. | `ci_check_wal_append_only.sh` |
| `consensus_inputs` *(N-M-C)* | Operator-extracted `LiveConsensusInputs` importer. | Treat the peer as runtime authority; overstate semantic truth. | `ci_check_live_consensus_inputs_closure.sh`, `ci_check_live_consensus_inputs_fingerprint.sh` |
| `admission::*` (the GREEN reducer half) *(N-M-C)* | GREEN admission verdict/agreement reducer comparing already-authoritative outputs. | Emit RED verdicts; skip reference scripts; treat `lagging` as success (DC-EVIDENCE-01). | `ci_check_admission_runner_closure.sh`, `ci_check_admission_no_red_verdicts.sh`, `ci_check_lagging_is_evidence_only.sh`, `ci_check_admit_replay_equivalence.sh` |

---

### `ade_node` GREEN-by-content sub-trees

| Sub-tree | Purpose | MUST NOT | CI gate |
|---|---|---|---|
| `admission_log` *(N-M-B)* | GREEN admission-mode JSONL event vocabulary + writer (closed enum). | Add open/wildcard event variant. | `ci_check_admission_log_vocabulary_closed.sh` |
| `live_log` *(N-L-LIVE)* | GREEN closed JSONL vocabulary for the wire-only live pass. | Add open event variant; overstate semantic truth. | `ci_check_wire_only_event_vocabulary_closed.sh` |
| `admission` (the GREEN half of the orchestrator) *(N-M-B)* | GREEN admission orchestrator reducer + verdict mapping. | Affect authoritative verdict; skip reference-script validation. | `ci_check_admission_no_refscript_skip.sh` |

---

## RED Modules — Imperative Shell

> I/O, network, storage, clocks, retries. May depend on BLUE/GREEN. Must not modify core state directly or construct semantic types unsafely.

### `ade_runtime`

| Attribute | Value |
|-----------|-------|
| **Purpose** | The imperative shell — moves bytes, owns sockets/files/clocks/keys, drives tokio tasks. Hosts: producer-mode key custody + shell composition (`producer/`), the N2N network drivers (`network/`), the node orchestrator runners (`orchestrator/`), the receive/rollback/admission shells, the seed-import/WAL/consensus-inputs importers, ChainDB + recovery shells, and the N-Y bootstrap/forward-sync surfaces: `forward_sync::pump` (RED driver), `mithril_import` (RED manifest shell), `genesis_bootstrap` (RED Conway-genesis entry), `recovery/restart` (RED crash-recovery wiring). |
| **Creates (RED-only)** | `KesSecret`, `VrfSigningKey`, `ColdSigningKey` custody wrappers; `KeyLoadError`; `MuxTransportHandle` consumers; `OutboundCommand` (closed enum, N-S-B); `PerPeerOutbound` map; `DispatchError`; `ServedChainHandle`; `GenesisAnchor`. **N-Y:** `forward_sync::pump::{PumpError (incl. TipBeforeDurable), PumpTip, SnapshotSink trait, NoCheckpointSink}`; `mithril_import::{RawMithrilManifest, RawCertifiedPoint, RawImmutableRange, MithrilManifestError, MithrilProvenanceImport}`; `recovery::restart::{NodeRecoveryError (incl. WalTailFingerprintMismatch), RecoveredNode}`. Never semantic/canonical types. |
| **Interprets** | Canonicalizes peer bytes/files for the BLUE core: `producer::keys::load_kes_signing_key_skey` (608-byte skey via BLUE `Sum6Kes::raw_deserialize_signing_key_kes`); `producer::genesis_parser` (`shelley-genesis.json` → `GenesisAnchor`); `producer::opcert_envelope`; `seed_import` (cardano-cli UTxO dump). **N-Y:** `mithril_import::json::parse_mithril_manifest_json` is the SOLE manifest-JSON parser → `RawMithrilManifest`, then `import_mithril_manifest` maps it into the closed `SeedProvenance::Mithril` + `MithrilManifestReport` (no semantic decision — the BLUE `verify_mithril_binding` decides; never re-verifies the STM multisig). |
| **MUST NOT** | (1) Modify BLUE state directly or construct semantic types from raw bytes. (2) Bypass canonical validation. (3) `producer::signing` (DC-CRYPTO-03/05) — RED-confined key custody. (4) `producer::keys` — no `KesSecret::from_*` inside `load_kes_signing_key_skey`. (5) `producer::coordinator` is GREEN and holds no secrets. (6) `network::outbound_command` (CN-OUTBOUND-RELAY-01) — `OutboundCommand` is the sole channel to MuxPump's encoder; no `Vec<u8>` byte tunnel; no direct transport write from `produce_mode`. (7) per-peer outbound map (CN-PEER-OUTBOUND-MAP-01 / DC-OUTBOUND-FIFO-01) — `BTreeMap`; structured lookup failure; FIFO per peer. (8) `network::n2n_server` — no signing dep. **(9) `forward_sync::pump` (DC-SYNC-01):** MUST apply the GREEN reducer's `SyncEffect` plan in order against the persistent `ChainDb` + `FileWalStore` + snapshot writer, and MUST refuse to advance the tip before the `StoreBlockBytes` + `AppendWal` durability writes return Ok — a tip-before-durable condition fails closed (`PumpError::TipBeforeDurable`); MUST NOT advance the tip from any path other than applying the reducer plan. **(10) `mithril_import` (CN-MITHRIL-01):** MUST perform no semantic decision and MUST NOT re-verify the STM multisig; the only authority deciding whether a Mithril anchor binds is the BLUE `verify_mithril_binding`; MUST route initial state through the single `bootstrap_initial_state` authority (`ci_check_mithril_uses_bootstrap_initial_state.sh`). **(11) `genesis_bootstrap` (CN-NODE-01 / DC-GENESIS-SRC-01):** MUST route a controlled Conway genesis through the same single `bootstrap_initial_state` authority — never a parallel storage-init path; the file read/parse is the RED `genesis_parser`, the genesis→state transform is the BLUE `genesis_initial_state`; records `SeedProvenance::CardanoCliJson` on the anchor. **(12) `recovery::restart` (recovery-contract / DC-WAL-* / DC-STORE-05 / T-REC-01/02):** MUST compose the existing authorities (no second recovery engine) — `WalStore::read_all` + BLUE `wal::replay_from_anchor` + `rollback_to_slot` to reconcile the ChainDb to the WAL tail before warm-start; MUST fail fast on `NodeRecoveryError::WalTailFingerprintMismatch`. (Post-N-Y `5db9aae` repointed the DC-STORE-05/T-REC-01/T-REC-02 `code_locus` at `recovery/mod.rs` + `recovery/restart.rs` after the `recovery.rs → recovery/` dir promotion.) |
| **Inbound deps** | `ade_node`, `ade_core_interop`, `ade_testkit` (dev/integration). |
| **Outbound deps** | `ade_types`, `ade_core`, `ade_crypto`, `ade_codec`, `ade_ledger`, `ade_network`, `redb`, `serde`, `serde_json`, `bech32`, `base58`, `cardano-crypto` (`["vrf-draft03", "dsign"]`), `ed25519-dalek`, `tokio`. |
| **Entry points** | `ade_runtime::producer::{producer_shell::*, coordinator::*, served_chain_handle::{ServedChainHandle, push_atomic}, genesis_parser::*, opcert_envelope::*}`, `ade_runtime::network::{n2n_listener::*, n2n_dialer::*, mux_pump::*, n2n_server::*, outbound_command::OutboundCommand}`, `ade_runtime::orchestrator::*`, `ade_runtime::bootstrap::bootstrap_initial_state`, `ade_runtime::forward_sync::{forward_sync_step, pump_block, SyncEffect, PumpError, SnapshotSink}`, `ade_runtime::mithril_import::{import_mithril_manifest, parse_mithril_manifest_json, MithrilProvenanceImport, MithrilManifestError}`, `ade_runtime::genesis_bootstrap::bootstrap_from_conway_genesis`, `ade_runtime::recovery::restart::{recover_node_state, NodeRecoveryError}`, `ade_runtime::{seed_import, consensus_inputs, wal, bootstrap_anchor}::*`. |
| **Key modules** | `producer/` (RED + GREEN-by-content), `network/` (`n2n_listener`, `n2n_dialer`, `mux_pump`, `n2n_server`, `outbound_command`), `orchestrator/` (RED runners + GREEN core), `receive/`, `rollback/`, `admission/`, `seed_import/`, `consensus_inputs/`, `wal/`, `chaindb/`, `forward_sync/` (`mod`, `reducer` GREEN, `pump` RED), `mithril_import/` (`mod`, `importer`, `json`), `genesis_bootstrap.rs`, `recovery/` (`mod`, `restart`), `bootstrap.rs`, `bootstrap_anchor.rs`, `clock.rs`. |

---

### `ade_node`

| Attribute | Value |
|-----------|-------|
| **Purpose** | The node binary + library entry. Owns argv parsing, the node lifecycle (`run_node_until_shutdown`), and the mode drivers: `--mode produce` (`produce_mode`), the admission mode (`admission/`), the wire-only live smoke pass (`wire_only`), and `key-gen-KES` (`key_gen`). `produce_mode::run_real_forge` carries an era guard (non-Praos → `ForgeFailureReason::UnsupportedProducerEra`; prove-step reads `expected_vrf_input.alpha_bytes()` — no RED-side era dispatch). `admission::runner` strips the served block's tag-24 envelope via `ade_codec::unwrap_tag24` (single shared authority). |
| **Creates (RED-only)** | `Cli`, `CliError`, `ProduceCli`, `NodeStartupInputs`, `NodeShutdownEvidence`, `NodeRunError`, exit-code constants; the `produce_mode` slot loop + ticker + evidence-I/O types; the admission runner types. Never semantic/canonical types. |
| **Interprets** | argv (closed mode set); operator-supplied key/genesis/opcert file paths (delegated to `ade_runtime` parsers); evidence-manifest TOML schema; the admission runner interprets a peer's tag-24-wrapped BlockFetch `MsgBlock` payload via `ade_codec::unwrap_tag24`. |
| **MUST NOT** | (1) Construct semantic types bypassing the canonical decoders / `ade_runtime` parsers. (2) `produce_mode` (CN-PROD-04, CN-OUTBOUND-RELAY-01) — obtain initial state only via `bootstrap_initial_state`; re-validate each broadcast block through BLUE `self_accept`; emit outbound bytes only via `OutboundCommand` → `MuxPump`. (3) No synthetic forge state (N-T). (4) No durability in the produce_mode path (N-U scope). (5) `run_real_forge` (CN-FORGE-04, N-W) — no RED-side era dispatch for the leader-VRF alpha; non-Praos era fail-closes to `UnsupportedProducerEra`. (6) `admission::runner` (CN-WIRE-08, N-X) — no hand-rolled tag-24 parse; strip via `ade_codec::unwrap_tag24`. (7) `wire_only` — overstate semantic truth; closed JSONL vocabulary only. (8) operator-evidence manifest carries the closed schema (CN-OPERATOR-EVIDENCE-01). (9) closed mode set (`ci_check_node_mode_closure.sh`). |
| **Inbound deps** | None (binary + integration tests). |
| **Outbound deps** | `ade_types`, `ade_core`, `ade_crypto`, `ade_ledger`, `ade_runtime`, `ade_network`, `ade_codec`, `tokio`. Dev-deps: `ade_testkit`, `tempfile`. |
| **Entry points** | `main()`; `ade_node::run_node_until_shutdown`; `ade_node::produce_mode::{run_real_forge, *}`; `ade_node::cli::{Cli, ProduceCli}`; `ade_node::admission::*`; `ade_node::wire_only::*`; `ade_node::key_gen::*`. |
| **Key modules** | `lib.rs`, `cli.rs`, `node.rs`, `main.rs`, `produce_mode.rs`, `wire_only.rs`, `key_gen.rs`, `admission/` (`bootstrap`, `runner`, `seed_to_snapshot`, `verdict`), `admission_log/`, `live_log/`. |

---

### `ade_core_interop`

| Attribute | Value |
|-----------|-------|
| **Purpose** | Live cardano-node interop driver. Hosts the `live_*_session` RED binaries (operator-action evidence harness) plus the N-E S4/S5 GREEN tx-submission bridges, and the chain-follow driver (`follow`). `follow` strips the peer's ChainSync `RollForward` `[era_tag, tag24(header_cbor)]` envelope via `ade_codec::unwrap_tag24`. |
| **Creates (RED-only)** | Live-session drivers and transcript types; never semantic types. |
| **MUST NOT** | (1) Construct semantic types from raw bytes. (2) Be depended on by any BLUE/GREEN crate (RED leaf). (3) Overstate semantic truth in evidence. (4) `follow` (CN-WIRE-08) — no hand-rolled tag-24 parse; strip via `ade_codec::unwrap_tag24`. |
| **Inbound deps** | None (RED leaf — binaries). |
| **Outbound deps** | `ade_core`, `ade_codec`, `ade_crypto`, `ade_ledger`, `ade_runtime`, `ade_network`, `ade_testkit`, `ade_types`, `tokio`. |
| **Entry points** | `ade_core_interop::bin::{live_tx_submission_session, live_block_production_session, live_block_fetch_session, live_block_follow_session}`; `ade_core_interop::follow::*`; `ade_core_interop::tx_submission`, `ade_core_interop::local_tx_submission` (GREEN sub-class). |
| **Key modules** | `bin/`, `follow.rs`, `tx_submission.rs`, `local_tx_submission.rs`. |

---

### `ade_network::mux::transport` *(RED)*

| Attribute | Value |
|-----------|-------|
| **Purpose** | The only place inside `ade_network` where socket I/O happens. RED tokio shell over the BLUE mux frame primitive and the GREEN session reducer. Provides `MuxTransportHandle` + closed `TransportError` + `DuplexCapacity::DEFAULT` + `spawn_duplex`. |
| **Creates (RED-only)** | `MuxTransportHandle`, `TransportError`, `DuplexCapacity`, `MuxTransport`. |
| **MUST NOT** | (1) Construct semantic types. (2) Bypass `mux::frame` for framing. (3) Live in BLUE scope. |
| **Inbound deps** | `ade_runtime::network::{mux_pump, n2n_dialer}`. |
| **Outbound deps** | `ade_network::mux::frame` (BLUE), `ade_network::session` (GREEN), `tokio`. |
| **Entry points** | `ade_network::mux::transport::{MuxTransportHandle, spawn_duplex, open_tcp}`. |
| **Key modules** | `mux/transport.rs`. |

---

### `ade_network` *(RED capture binaries — non-session)*

| Attribute | Value |
|-----------|-------|
| **Purpose** | Operator-action capture binaries (live evidence harness) inside `ade_network`. The `ade_chain_sync_capture` binary supports `--intersect-slot` / `--intersect-hash` flags (N-X) to capture a real Conway `RollForward` golden fixture. |
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

**Forbidden directions.** Any BLUE crate (or BLUE `ade_network` submodule) depending on `ade_runtime`, `ade_node`, `ade_core_interop`, or the RED half of `ade_network` is a CI failure (`ci_check_dependency_boundary.sh`). Any non-`ade_plutus` crate referring to `pallas_*` is a CI failure (`ci_check_pallas_quarantine.sh`). Any `cardano_crypto::kes` import outside `#[cfg(test)]` under `crates/ade_crypto/src/**` is a CI failure (`ci_check_kes_sum_compatibility.sh` Guard 3). A second block-envelope encoder or a `produce_mode` direct-transport write is a CI failure. A second `leader_vrf_input` authority, or a path accepting both TPraos and Praos VRF inputs for one era, is a CI failure (`ci_check_producer_praos_vrf.sh`). A second `wrap_tag24`/`unwrap_tag24` definition or a hand-rolled tag-24 parse in RED is a CI failure (`ci_check_tag24_wire_authority.sh`). A forward-sync `AdvanceTip` reachable before the block's durability writes (DC-SYNC-01 — `ci_check_forward_sync_chokepoint_only.sh`), a Mithril/genesis bootstrap path bypassing the single `bootstrap_initial_state` authority (`ci_check_mithril_uses_bootstrap_initial_state.sh`), and an internal-ledger-fingerprint-vs-Haskell-hash equality assertion in the compatibility harness (DC-COMPAT-01 — `ci_check_no_haskell_fingerprint_equality.sh`) are CI failures. **NEW (post-N-Y): a registry rule citing a `crates/**.rs` or `ci/**.sh` `code_locus` path that does not exist on disk is a CI failure (`ci_check_registry_code_locus_exists.sh`) — a traceability drift guard catching moved/renamed/deleted source paths at close time.**

### Naming convention

All crates are prefixed `ade_`. TCB color is not encoded in the crate name. The authoritative classifier is `.idd-config.json` `core_paths` + the per-file `// Core Contract:` / `//! BLUE|GREEN|RED` banner + the cluster TCB Color Maps; CI scripts hard-code their BLUE list.

### Closed enums / registries (for SEAMS cross-reference)

Closed semantic surfaces detected at HEAD: `AcceptedMiniProtocol` (`ade_network::session`), `TagEnvelopeError` (`ade_codec::cbor::tag24`, N-X), `ExpectedVrfInput` (2-variant, `ade_core::consensus::vrf_cert`, N-W), `LeaderCheckVerdict` (2-variant, `ade_core::consensus::leader_check`), `OutboundCommand` + `DispatchError` (`ade_runtime::network`), `ProducerLogEvent` + `ForgeFailureReason` (incl. `UnsupportedProducerEra`), `ChainEvolutionError`, `ServedChainAdmitError`, `KesError` / `KesParseError`, the admission/wire-only/live-log JSONL vocabularies, the operator-evidence manifest TOML schema, the closed `CardanoEra` / Conway cert + governance enums. **N-Y:** `SeedProvenance` (`CardanoCliJson` / `Mithril{…}` — closed, version-gated behind `ANCHOR_SCHEMA_VERSION = 2`, `ade_ledger::bootstrap_anchor`), `MithrilImportError` (5-variant, `ade_ledger::bootstrap_anchor::binding`), `GenesisSourceError` (`NonConwayEra`, `ade_ledger::genesis_source`), `SyncEffect` (4-variant, `ade_runtime::forward_sync::reducer`), `MithrilManifestError` (`ade_runtime::mithril_import`), `PumpError` (incl. `TipBeforeDurable`, `ade_runtime::forward_sync::pump`), `NodeRecoveryError` (incl. `WalTailFingerprintMismatch`, `ade_runtime::recovery::restart`), the `sync_diff` observable `BlockVerdict` + `RegressionFixtureViolation` (`ade_testkit::harness::sync_diff`). **Governance: `WalEntry` deliberately stays a CE-not-law surface — additively evolvable behind the WAL schema version, not a frozen registry law.**

### CI enforcement (104 scripts under `ci/`)

The full list is mechanically obtainable via `ls ci/ci_check_*.sh` (104 at HEAD). CI lives entirely under `ci/` (`ci_dirs = ["ci"]`). A `.github/workflows/` dir now exists in the repo (`notify-atlas.yml`, added by `f0d0bf9`) but it is a **grounding-doc→ade-atlas-rebuild dispatch workflow, not an Ade invariant gate** — it does not validate the codebase, so it is not part of the 104 CI-check count. New / load-bearing since the prior CODEMAP HEAD `3b78008` (PHASE4-N-Y close):

| Script | Enforces | Cluster / commit |
|---|---|---|
| `ci_check_registry_code_locus_exists.sh` | **NEW** traceability drift guard — every `crates/**.rs` + `ci/**.sh` path cited in any registry rule's `code_locus` must exist on disk (560 code paths across 298 rules; globs and `docs/` paths skipped; fails closed on a moved/renamed/deleted path). | post-N-Y `5db9aae` |
| `ci_check_forward_sync_chokepoint_only.sh` | DC-SYNC-01 — durable-before-tip; the GREEN reducer's `AdvanceTip` is reachable only after `StoreBlockBytes` + `AppendWal`; `AdmitPlan` is the sole `AdvanceTip` emitter. | N-Y |
| `ci_check_mithril_uses_bootstrap_initial_state.sh` | CN-MITHRIL-01 — the Mithril path routes initial state through the single `bootstrap_initial_state` authority and decides binding only via the BLUE `verify_mithril_binding`; never re-verifies the STM multisig. | N-Y |
| `ci_check_no_haskell_fingerprint_equality.sh` | DC-COMPAT-01 — the compatibility harness compares only observable surfaces; no internal-ledger-fingerprint-vs-Haskell-hash equality. | N-Y |
| `ci_check_sync_evidence_manifest_schema.sh` | RO-SYNC-EVIDENCE-01 — closed sync-evidence manifest schema (references the CN-OPERATOR-EVIDENCE-01 pattern). | N-Y |
| `ci_check_tag24_wire_authority.sh` | CN-WIRE-08 — single tag-24 wrap/unwrap authority; no hand-rolled tag-24 parse in RED; serve paths compose via `compose_blockfetch_block` / `compose_rollforward_header`. | N-X |
| `ci_check_producer_praos_vrf.sh` | CN-FORGE-04 — single era→leader-VRF-input authority; closed `ExpectedVrfInput`. | N-W |
| `ci_check_leader_check_authority.sh` | CN-FORGE-02 — BLUE leader-check has no LedgerView/EraSchedule/RED dep; closed `LeaderCheckVerdict`. | N-R-A |
| `ci_check_unsigned_header_preimage_single_source.sh` | CN-KES-HEADER-01 / DC-KES-HEADER-01 — single canonical pre-image recipe; branded `UnsignedHeaderPreImage`. | N-S-A |
| `ci_check_no_produce_mode_direct_transport_writes.sh` | CN-OUTBOUND-RELAY-01 — `produce_mode` emits bytes only via `OutboundCommand` → `MuxPump`. | N-S-B |
| `ci_check_operator_evidence_manifest_schema.sh` | CN-OPERATOR-EVIDENCE-01 — closed evidence-manifest TOML schema. | N-S-C |
| `ci_check_recovery_contract.sh` | recovery-contract / DC-WAL-* / DC-STORE-05 / T-REC-01/02 (strengthened N-Y; `code_locus` repointed at `recovery/mod.rs` + `recovery/restart.rs` by post-N-Y `5db9aae`) — recovery composes existing authorities; reconciles ChainDb to the WAL tail; fail-fast on `WalTailFingerprintMismatch`. | N-Y (carry-forward strengthened) |
| `ci_check_snapshot_encoder_closure.sh` | DC-STORE-09 — kept green by the `SCHEMA_VERSION → ANCHOR_SCHEMA_VERSION` disambiguation (the snapshot-framing `SCHEMA_VERSION` is now unambiguous). | N-Y (rename, no new gate) |

> Earlier-cluster scripts (N-A through N-X) are present and counted in the 104 total. The per-script enforce/scope detail is in the registry's `ci_script` fields per rule.

---

## Generation notes

- Regenerated full at HEAD `5db9aae` (`git rev-parse --short HEAD`). PHASE4-N-Y (Mithril-anchored bootstrap, network forward-sync & WAL recovery) is CLOSED at commit `3b78008`; HEAD is two post-close commits beyond: `f0d0bf9` (`.github/workflows/notify-atlas.yml` — atlas-rebuild dispatch, not an Ade gate) and `5db9aae` (CI script #104 `ci_check_registry_code_locus_exists.sh` + DC-STORE-05/T-REC-01/T-REC-02 `code_locus` repoint). The cluster doc is archived under `docs/clusters/completed/PHASE4-N-Y/`.
- **Cluster-doc location:** all closed cluster docs are under `docs/clusters/completed/`. No cluster directory lives outside `completed/` at this HEAD.
- All mechanical counts recomputed fresh from the tree (not copied from the prior CODEMAP): 11 crates (Δ 0), 452 canonical types (Δ 0 — the two post-close commits added no types), 2067 tests (Δ 0 — `5db9aae` touched only `code_locus` strings + added a shell gate), 104 CI checks (**Δ +1**: `ci_check_registry_code_locus_exists.sh`), 298 registry rules (Δ 0 — `5db9aae` repointed three `code_locus` fields, added/removed no rule).
- Counts are mechanical (commands in the Counts table). `canonical_type_registry: null`, so the canonical-type count is a structural grep over BLUE scopes.
- `.idd-config.json` `_ci_dirs_doc` still reads "No .github/workflows in this repo yet" — that note is now **stale** as of `f0d0bf9`; the workflow that exists is a non-gate atlas-rebuild dispatch. The config note should be updated (or `.github/workflows` added to `ci_dirs`) only if/when an actual invariant gate is added there; the 104 CI-check count is unaffected.
- TCB color for every module was verified against the on-disk `// Core Contract:` / `//!` banner: `bootstrap_anchor::binding` + `genesis_source` resolve BLUE under `crates/ade_ledger/`; `forward_sync::reducer` is GREEN-by-content and `forward_sync::pump` is RED (both under the RED `crates/ade_runtime/`); `mithril_import`, `genesis_bootstrap`, `recovery::restart` are RED; `harness::sync_diff` is GREEN (testkit).
- The doc is regenerated, not edited. If a value drifts, fix the source, not the doc.
