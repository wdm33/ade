# Module Authority Map — Ade

> **Status:** Living architectural document. Regenerated; not hand-edited.
> Per-project instance of `~/.claude/methodology/templates/codemap.md`, in IDSE v3.0 form.

> 12 modules, 637 canonical types, 3658 tests, 286 CI checks at HEAD (`08c7c3f5`).

**Regenerated for IDSE.** This is the first refresh since Ade migrated from legacy IDD to IDSE v3.0 (`strict`, namespace `ADE`, 2026-11-08/09; `docs/planning/idse-migration-plan.md`, step M2). Every invariant reference below is an IDSE registry ID from `docs/ade-invariant-registry.toml` (260 invariants, assumptions A1–A7). Legacy IDs are not cited. To follow a legacy ID quoted in a source comment, use `docs/legacy/idd-to-idse-map.toml`. The previous CODEMAP refresh was at `1e4896eb`, 221 commits back. Since then:

- the LV-series ledger and recovery fixes landed: the donation pot, signed native-script thresholds, heap-bounded native scripts and CBOR skip, the script-witness closure, the UTxO-free admission checks, frozen ratification, durable intersection, the store-semantics version gate, and the bootstrap-bound forge timing authority
- the twelve Define batches ran, followed by the M2 cutover

Delta against `1e4896eb` (564 types / 3319 tests / 255 CI): **+0 modules, +73 canonical types, +31 CI gates.** The test count isn't directly comparable, because the counting method changed (see Counts). Counted the old way (test attributes), it went from 3319 to 3662.

---

## Conventions

- A **module** in Ade is a Cargo workspace crate, the smallest independently buildable unit. Source: the root `Cargo.toml` `[workspace] members`, which has **12** entries.
- **One exception to crate granularity.** `.idse-config.json` `core_paths` resolves `ade_network` color per submodule (9 BLUE paths), so its BLUE, GREEN and RED parts get separate entries.
- **`CLAUDE.md` crate list is incomplete.** At HEAD it names 7 crates. It omits `ade_ledger`, `ade_plutus`, `ade_network`, `ade_core_interop` and `ade_mem_diag`. An uncommitted working-tree edit adds four of these but still omits `ade_ledger`. It also describes `ade_core` as "Ledger, consensus, and protocol logic", although the ledger lives in `ade_ledger` and `ade_core` holds only `consensus/`. See the open questions.
- **Invariant citations.** "(VAL-024)" means the registry entry whose `enforcement_runtime` loci name this module's file, or whose statement names its construct. Those loci were carried from the legacy registry at M1, and none is yet backed by an IDSE cluster obligation. Every entry's `claim_status` is `partial` or `unresolved`, except PRC-001, which is `falsified` (Doctrine §10; re-evidence at `e563ad94`).

### TCB color sources (in order of authority)

1. **`.idse-config.json` `core_paths`** (substring match ⇒ BLUE). Six whole crates are BLUE: `ade_codec`, `ade_types`, `ade_crypto`, `ade_core`, `ade_plutus`, `ade_ledger`. So are nine `ade_network` submodule paths: `mux/frame.rs`, `codec/`, `handshake/`, `chain_sync/`, `block_fetch/`, `tx_submission/`, `keep_alive/`, `peer_sharing/`, `n2c/`.
2. **`CLAUDE.md` role notes.** `ade_runtime` is the "imperative shell" (RED) and `ade_node` the "binary entry point". `CLAUDE.md` states no explicit color naming rule.
3. **Source banners and manifest signals.** These are the `// Core Contract:` header, the `//! BLUE|GREEN|RED …` / `// TCB: …` banners, and the BLUE `lib.rs` deny attributes. Examples: the `ade_network/src/lib.rs` color partition, the `ade_core_interop/src/lib.rs` "RED" banner, the `ade_mem_diag` "RED DIAGNOSTIC crate" contract, and the `ade_node/src/lib.rs` "RED … binary library" banner.
4. **Import heuristic.** Code that reaches for tokio, sockets, the filesystem, a clock or randomness is RED.

### Counts (mechanical, with sources)

| Count | Value | Source (at `08c7c3f5`) |
|---|---|---|
| **Modules** | **12** | `Cargo.toml` `[workspace] members`. |
| **Canonical types** | **637** | `git grep -hE '^(pub )?(struct\|enum) '` over the 6 BLUE crate `src/` trees (526) plus the 9 BLUE `ade_network` paths (111); `canonical_type_registry` is `null`. Per crate: `ade_codec` 14, `ade_types` 86, `ade_crypto` 22, `ade_core` 59, `ade_plutus` 9, `ade_ledger` 336. `ade_network` BLUE: `mux/frame.rs` 5, `codec/` 40, `handshake/` 9, `chain_sync/` 11, `block_fetch/` 10, `tx_submission/` 5, `keep_alive/` 5, `peer_sharing/` 5, `n2c/` 21. |
| **Tests** | **3658** | `cargo test --workspace --no-run --message-format=json` gave 225 test executables, each run with `--list --format terse` (list mode only; no test executed). The 3658 includes 94 `#[ignore]` tests and excludes doc-tests. **One test target is excluded:** `crates/ade_plutus/tests/conway_validity_range_reproducer.rs` (1 `#[ignore]` test) does not compile, because it `include_str!`s fixtures from the git-ignored `target/aiken_divergent_fixture/`. Because of this, `cargo test --workspace --no-run` fails at HEAD in this checkout. Cross-check: `git grep` for `#[test]` / `#[tokio::test]` attributes finds 3662. |
| **CI checks** | **286** | `ci/ci_check_*.sh` gate scripts. `ci/` also holds 4 non-gate helper `.sh` scripts, `ci/git-hooks/commit-msg`, `ci/idse_legacy_carried.py` and `ci/store-semantics-surface.lock`. `.github/workflows/notify-atlas.yml` is a notifier (it triggers an ade-atlas rebuild), not a check. No `.gitlab-ci.yml`. |

> **Registry snapshot** (`docs/ade-invariant-registry.toml`, schema 3): **260** invariants, all `status = "active"`.
> - Families: VAL 67, SEC 38, DET 26, SAF 25, PRC 23, STR 22, DUR 14, LIV 11, OBS 9, REC 8, EVO 6, ORD 6, CON 5.
> - Criticality: 215 critical, 45 standard.
> - Claim status: 185 `partial`, 74 `unresolved`, 1 `falsified` (PRC-001).
> - 25 entries name no enforcement locus.
> - 259 distinct CI scripts are named in `ci_scripts`. The other 27 gate scripts under `ci/` are named by no entry (see the gaps).

---

## BLUE Modules — Pure Functional Core

> **Shared header (applies to every BLUE entry below).**
> - **Banner.** Every BLUE `.rs` file starts with the `// Core Contract:` banner: "Deterministic: same inputs ⇒ byte-identical outputs; no wall-clock time, true randomness, HashMap/HashSet, or floats; encode invariants in types; explicit state transitions only; canonical serialization for all persisted/hashed data". `ci_check_module_headers.sh` gates it.
> - **Deny attributes.** Each BLUE crate root carries `#![deny(unsafe_code)]`, `#![deny(clippy::unwrap_used)]`, `#![deny(clippy::expect_used)]`, `#![deny(clippy::panic)]` and `#![deny(clippy::float_arithmetic)]`. For `ade_network`, the crate root carries them and the BLUE submodules inherit them.
> - **Workspace `clippy.toml`.** Its `disallowed-types` covers `HashMap`, `HashSet`, `SystemTime`, `Instant` and `anyhow::Error`.
> - **Invariants every BLUE path carries:**
>   - STR-001: no I/O, clock, environment, randomness, tasks or async.
>   - DET-004: no float, non-canonical iteration, or locale, pointer-width or endianness dependence.
>   - DET-006: no cargo feature or `cfg` changes an authoritative result.
>   - STR-003: no dependency on RED code or on I/O, async, clock, randomness or general-serialization libraries.
>   - SEC-001: no BLUE transition takes a private key or produces a signature.
>   - STR-004: external bytes enter only through named decode chokepoints.
> - **Cross-cutting BLUE gates:**
>   - `ci_check_forbidden_patterns.sh` (STR-001, DET-004)
>   - `ci_check_dependency_boundary.sh` (STR-001, STR-003)
>   - `ci_check_no_async_in_blue.sh` (STR-001; scope read from `core_paths`)
>   - `ci_check_no_float_in_consensus.sh` (STR-001, DET-004)
>   - `ci_check_no_semantic_cfg.sh` (DET-006)
>   - `ci_check_no_signing_in_blue.sh` (SEC-001)
>   - `ci_check_ingress_chokepoints.sh` (STR-004)
>   - `ci_check_hash_uses_wire_bytes.sh` (DET-002)
>   - `ci_check_module_headers.sh` and `ci_check_pallas_quarantine.sh` (both named by no registry entry)

---

### `ade_codec`

| Attribute | Value |
|-----------|-------|
| **Purpose** | Owns Cardano CBOR ingress: the single place raw bytes become typed values, with wire bytes kept for every hash-bearing structure (DET-002, DET-003). Holds the single tag-24 envelope authority (VAL-002), the block-envelope codec (VAL-057), the closed `PrevHash` header codec (VAL-003) and the shared canonical CBOR read/write primitives the other BLUE codecs build on. *(No doc comment, README or manifest description; Purpose is taken from the module surface and registry loci.)* |
| **Creates** | `PreservedCbor<T>`, `RawCbor`, `BlockEnvelope`, `CodecError`, `CodecContext`, `TagEnvelopeError` (closed), `address::pointer::{Ptr, PointerDecodeError}`, `ContainerEncoding`, `IntWidth`, `OpCertCodecError`, `TxComponents<'a>`, and the era block/tx wrappers. 14 type definitions. |
| **Interprets** | All Conway-reachable Cardano CBOR: `cbor::envelope::decode_block_envelope`, the per-era `decode_*_block` functions (incl. `conway::decode_conway_block`), `conway::cert::decode_conway_certs`, `conway::tx::decode_conway_tx_body`, `unwrap_tag24`, `address::pointer::decode_pointer_address`, `shelley::opcert::*`. |
| **MUST NOT** | (1) Have more than one decoder per type and protocol version, or any fallback or lenient parser that accepts bytes the decoder rejects (VAL-001). (2) Compute a protocol hash over a re-encoding instead of the received wire bytes (DET-002; `ci_check_hash_uses_wire_bytes.sh`). It must not break decode→encode byte identity (DET-003). (3) Build or remove a tag-24 envelope anywhere except `cbor::tag24` (VAL-002). (4) Use a placeholder hash for the genesis predecessor (VAL-003). (5) Use call-stack depth that grows with input nesting when skipping CBOR or decoding native scripts (SEC-009). (6) Decode a native-script threshold as anything but signed 64-bit (VAL-017). (7) Drop a credential's key/script tag, or convert a bare 28-byte hash into a credential (STR-005). (8) Hold proposal procedures as opaque bytes (STR-006). (9) Hand raw or partly decoded bytes to a core transition around a named chokepoint (STR-004). (10) Depend on any workspace crate other than `ade_types` (manifest; STR-003). Also the BLUE shared header. |
| **Inbound deps** | `ade_ledger`, `ade_plutus`, `ade_network`, `ade_runtime`, `ade_node`, `ade_core_interop`, `ade_testkit`; `ade_core` (dev only). |
| **Outbound deps** | `ade_types`. No external runtime deps (dev: `serde_json`, `toml`). |
| **Entry points** | `ade_codec::cbor::envelope::decode_block_envelope`, `ade_codec::cbor::*` (the canonical read/write primitives), `ade_codec::{wrap_tag24, unwrap_tag24, TagEnvelopeError}`, `ade_codec::CodecError`, `ade_codec::traits::{AdeDecode, AdeEncode}`, `ade_codec::conway::{decode_conway_block, cert::decode_conway_certs, tx::decode_conway_tx_body}`. |
| **Key modules** | `cbor/` (incl. `envelope.rs`, `tag24.rs`, the heap-bounded skip), `byron/`, `shelley/` (`block.rs`, `cert.rs`, `opcert.rs`, `tx_components.rs`), `allegra/` (native script), `mary/`, `alonzo/`, `babbage/`, `conway/`, `address/` (incl. `pointer.rs`), `preserved.rs`, `traits.rs`, `error.rs`. |

---

### `ade_core`

| Attribute | Value |
|-----------|-------|
| **Purpose** | The consensus half of the functional core. It owns Praos header validation, chain selection, the chain-dependent state and nonce evolution, the leader check and leader-schedule query, and the era schedule and slot/timing authority that every epoch and slot decision derives from. *(No doc comment, README or manifest description; Purpose is taken from `consensus/` and the registry loci.)* |
| **Creates** | `PraosChainDepState`, `Nonce`, `EraSchedule`, `CandidateFragment`, `ValidatedHeaderSummary`, `HeaderInput`, `LeaderCheckVerdict` (closed), `LeaderEligibility` (closed), `LeaderScheduleAnswer`, `LedgerView`, `SecurityParam`, `ActiveSlotsCoeff`, `Point`. New since the last refresh, in `era_schedule.rs`: `BootstrapBoundTimingAuthority`, `DerivedTimingAnchor`, `VenueTimingHistory`, `VenueTimingSegment`, `BootstrapTimingBinding`, `ScheduleCommitment`, and the closed `SlotDerivationError`, `TimingAnchorError` and `TimingAuthorityError`. 59 type definitions. |
| **Interprets** | Validated header inputs (`validate_and_apply_header`), candidate summaries (`select_best_chain`), VRF/KES/opcert evidence (`vrf_cert`, `kes_check`, `opcert_validate`), and captured millisecond timestamps converted to slots (`BootstrapBoundTimingAuthority::slot_at`, caller-supplied; it reads no clock). |
| **MUST NOT** | (1) Contain a second chain selector. Selection depends only on canonical observables, never on arrival order, scheduling or clock, and never uses density (VAL-031, DET-012, SEC-013; `ci_check_chain_selection_arrival_order_independent.sh`, `ci_check_no_density_in_fork_choice.sh`). (2) Accept a candidate not built from headers Ade validated itself (SEC-021). (3) Split the per-header nonce transition, fabricate a nonce, or add an extra-entropy operand (VAL-022). (4) Have more than one Praos VRF construction or a producer-side leader-check variant, or fall back to the TPraos input (VAL-036, VAL-056). (5) Give the core leader evaluator a signing key, a ledger view, an era schedule, chain-dependent state, a clock or storage. An ineligible verdict carries no more than a bounded VRF fingerprint (SEC-026). (6) Lower a pool's recorded opcert issue number on a forward transition (SEC-011). (7) Apply a rollback deeper than k (SAF-007). (8) Return a guessed value beyond the safe forecast range, or take the range bound as a caller constant (VAL-033, ORD-005). (9) Read files, JSON or operator configuration; the era schedule enters only as a typed value bound to the bootstrap anchor (STR-009). (10) Derive the slot by a second authority or read a clock (DET-019). (11) Let the ledger own the chain-dependent state (STR-008). (12) Depend on anything beyond `ade_types`, `ade_crypto` and `minicbor` (manifest; STR-003). Also the BLUE shared header. |
| **Inbound deps** | `ade_ledger`, `ade_runtime`, `ade_node`, `ade_core_interop`, `ade_testkit`; `ade_network` (dev only). |
| **Outbound deps** | `ade_types`, `ade_crypto`, `minicbor` (`alloc`). Dev: `ade_testkit`, `ade_codec`, `cardano-crypto`, `ed25519-dalek`, `serde_json`. |
| **Entry points** | `ade_core::consensus::era_schedule::{EraSchedule, praos_rsw_slots, BootstrapBoundTimingAuthority}`, `ade_core::consensus::vrf_cert::ActiveSlotsCoeff`, `ade_core::consensus::praos_state::{PraosChainDepState, Nonce}`, `ade_core::consensus::ledger_view::LedgerView`, `ade_core::consensus::{select_best_chain, validate_and_apply_header, query_leader_schedule, verify_header_kes}`, `ade_core::consensus::events::Point`. |
| **Key modules** | `consensus/`: `fork_choice.rs`, `header_validate.rs`, `candidate.rs`, `header_summary.rs`, `praos_state.rs`, `nonce.rs`, `era_schedule.rs`, `leader_check.rs`, `leader_schedule.rs`, `ledger_view.rs`, `vrf_cert.rs`, `kes_check.rs`, `op_cert.rs`, `opcert_validate.rs`, `rollback.rs`, `events.rs`, `encoding.rs`, `errors.rs`. |

---

### `ade_crypto`

| Attribute | Value |
|-----------|-------|
| **Purpose** | Pure cryptographic verification, matching the reference implementation (SEC-003): Praos VRF (draft-03), Ed25519, the Ade-owned Sum6 KES algorithm, blake2b and (new) SHA3-256 for bootstrap-witness key hashes, and the versioned UTxO set commitment (DET-026). *(No doc comment, README or manifest description; Purpose is taken from module docs and registry loci.)* |
| **Creates** | `VrfOutput`, `VrfProof`, `VrfVerificationKey`, `Ed25519VerificationKey`, the `kes_sum` Sum6 key and signature types with `KesAlgorithm`, `KesError`, `KesParseError`, `CryptoError`, `UtxoSetCommitment`. 22 type definitions. |
| **Interprets** | VRF proofs, KES signatures and the 608-byte expanded Sum6 key envelope (`kes_sum`), Ed25519 signatures, and hash inputs (`blake2b_256`, `blake2b_224`, `sha3_256`). |
| **MUST NOT** | (1) Be called for signing, proving or key evolution by anything but RED code. Signing algorithms may live here only as library functions (SEC-001; `ci_check_no_signing_in_blue.sh`). (2) Retry a failed verification with another parser or serialization (SEC-003). (3) Build an upstream library's key through unsafe layout assumptions, transmute, vendored private access or fork-only constructors (VAL-006). (4) Evolve a KES key backwards, sign for a period evolved past, or keep evolved-away seed material (SEC-005). (5) Mix commitment versions in one fold, or let an incremental or cached commitment differ from full recomputation (DET-026). (6) Depend on a Mithril or multisignature crate (SEC-034). (7) Depend on any workspace crate other than `ade_types` (manifest; STR-003). Also the BLUE shared header. |
| **Inbound deps** | `ade_core`, `ade_ledger`, `ade_plutus`, `ade_runtime`, `ade_node`, `ade_core_interop`, `ade_testkit`; `ade_network` (dev only). |
| **Outbound deps** | `ade_types`, `blake2`, `cardano-crypto` (`vrf-draft03`, `kes-sum`, `dsign`), `ed25519-dalek`, `sha3` (new), `curve25519-dalek` v4. |
| **Entry points** | `ade_crypto::vrf::{verify_vrf, VrfOutput, VrfVerificationKey}`, `ade_crypto::blake2b::blake2b_256` / `ade_crypto::blake2b_256`, `ade_crypto::kes::*`, `ade_crypto::kes_sum::{KesAlgorithm, …}`, `ade_crypto::ed25519::Ed25519VerificationKey`, `ade_crypto::sha3_256`, `ade_crypto::utxo_set_commitment::UtxoSetCommitment`. |
| **Key modules** | `vrf/`, `kes.rs`, `kes_sum/`, `ed25519`, `blake2b.rs`, `sha3.rs` (new), `utxo_set_commitment.rs`, `traits.rs`, `error.rs`. |

---

### `ade_ledger`

| Attribute | Value |
|-----------|-------|
| **Purpose** | The ledger half of the functional core: Conway block and transaction validity, UTxO and value accounting, certificates and governance, epoch boundaries, the non-UTxO epoch accumulator and sealed leadership, receive and rollback reducers, the producer forge and self-acceptance, the WAL entry grammar, snapshots, and the native Mithril ledger-state decoders. *(The `lib.rs` banner is the Core Contract only; Purpose is taken from module banners and registry loci.)* |
| **Creates** | 336 type definitions. The load-bearing ones: `LedgerState`, `UTxOState`, `TxOut`, `BlockVerdict`, `BlockValidityError`, `LedgerFingerprint`, `WalEntry` (closed, 4 variants: `AdmitBlock`, `SeedEpochConsensusInputsImported`, `RollBack`, `EpochConsensusViewActivated`), `MempoolState`, `ForgedBlock`, `ProducerTick`, `ReceiveState`, `SeedEpochConsensusInputs` (schema v6), `BootstrapAnchor`, `RecoveredAnchorPoint`, `ProtocolParameters` / `MinUtxoRule`, `EpochAccumulator`, `FrozenLeadershipPoolDistr`, `EpochConsensusView`, `ReducedBoundaryProjection`. New since the last refresh: `governance::frozen::{RatificationRecord, FrozenRatificationInput, …}`, `rollback::intersection::{IntersectionCandidates, IntersectReply, IntersectionOutcome, …}`, `store_semantics::StoreSemanticsVersionMismatch` (`STORE_SEMANTICS_VERSION = 13`), `replay_divergence::ReplayDivergenceReport`, `collateral::{CollateralValueResolver, CollateralBalanceError}`, `tx_validity::script_witnesses::{ScriptsNeeded, ReducedScriptVerdict, ExactScriptVerdict, …}`, `tx_validity::utxo_free::UtxoFreeRejection`. |
| **Interprets** | Decoded Conway blocks and transactions (`block_validity::decode_block`, `tx_validity`), the mempool ingress event (`mempool::ingress::mempool_ingress`), snapshots (`snapshot::framing`), WAL entries (`wal::replay`), the reference node's ledger-state and UTxO-tables files (`ledgerdb_state::decode_native_nonutxo_state`, `ledgerdb_tables`, `mithril_utxo_materialize`), the bootstrap manifest (`bootstrap_manifest`), script references (`tx_validity::script_witnesses::decode_script_ref`), and peer intersection replies (`rollback::intersection::classify_intersect_reply`). |
| **MUST NOT** | **Validity.** (1) Produce a valid verdict while skipping header, body, phase-1 or phase-2 checks, validate a body before its header, or leave a check defined but unwired (VAL-009, ORD-002, VAL-010). (2) Have a mempool-only validity rule or a second phase-1 / script-evaluation path (VAL-067). Every source reduces to one closed ingress event (STR-022). (3) Decide script-witness sufficiency anywhere but the one closure authority, or return "valid" from the reduced level (SEC-010). (4) Derive a transaction's UTxO and accounting effect in more than one place, or let consumers see the phase-2 status (VAL-016). (5) Take collateral value from the declared total-collateral field, or treat an unresolved collateral input as zero (CON-003). (6) Let an output quantity be negative, wrap or saturate (VAL-013), or remap era protocol parameters (VAL-014). **Accounting and epoch.** (7) Resolve a certificate's deposit from a default or fallback constant (CON-002), or apply a certificate effect twice or drop one (VAL-011). (8) Reimplement the epoch boundary outside the one NEWEPOCH-order transition (VAL-020). Within it, it must not read pool parameters for rewards from anything but the go snapshot (VAL-028, VAL-027), and must not include a zero-stake credential in a snapshot (VAL-026). (9) Read votes, stake, committee, thresholds or expiry when applying a ratification record, or enact part of a ratified action (VAL-021). (10) Default or saturate the donation pot (CON-005). (11) Answer a leader check from anything but the sealed frozen leadership object for exactly that epoch, or compute its sigma denominator by summing pool entries (VAL-024). The accumulator must not hold a full UTxO map (DUR-003). **Rollback and recovery.** (12) Have a second rolled-back-state materializer (STR-011; `ci_check_rollback_materialize_closure.sh`), or roll back the database, ledger and chain-dependent state other than in lockstep (REC-003). (13) Treat a peer intersection reply as rollback authority, or offer points outside the validated durable ancestry (VAL-055, REC-005). (14) Let the WAL offer truncate, rewrite or delete (DUR-006), or break its fingerprint chain (DUR-009). (15) Open a durable artifact whose semantics marker is absent, older or newer than the binary's (EVO-005), or change the locked semantics surface without a version bump or a neutral record (PRC-005; `ci/store-semantics-surface.lock`). **Producer.** (16) Forge without the shared leader check (VAL-056), build a body with any encoder but the validator's inverse (VAL-057), reorder or skip snapshot transactions (VAL-058), or hand on bytes self-acceptance did not accept (SAF-020). **Snapshots and bootstrap.** (17) Decode a snapshot payload before its version tag is checked, or accept a fingerprint mismatch (DET-015, DET-011). (18) Mint a bootstrap anchor by more than one function or with a defaulted field (STR-021). (19) Depend on RED/GREEN crates (manifest; STR-003). Also the BLUE shared header. |
| **Inbound deps** | `ade_runtime`, `ade_node`, `ade_core_interop`, `ade_testkit`; `ade_network` (dev only). |
| **Outbound deps** | `ade_types`, `ade_crypto`, `ade_codec`, `ade_core`, `ade_plutus`, `minicbor` (`alloc`), `num-bigint`, `num-integer`, `num-traits`. Dev: `ade_testkit`, `ed25519-dalek`. |
| **Entry points** | `ade_ledger::state::LedgerState`, `ade_ledger::consensus_view::*`, `ade_ledger::block_validity::decode_block`, `ade_ledger::wal::*`, `ade_ledger::bootstrap_anchor::SeedPoint`, `ade_ledger::receive::ReceiveState`, `ade_ledger::pparams::ProtocolParameters`, `ade_ledger::ledgerdb_state::decode_native_nonutxo_state`, `ade_ledger::fingerprint::fingerprint`, `ade_ledger::mempool::ingress::mempool_ingress`, `ade_ledger::epoch_accumulator::apply_selected_block{,_with_resolver}`, `ade_ledger::rollback::{materialize::materialize_rolled_back_state, admission::admit_rollback, intersection::select_intersection_candidates}`, `ade_ledger::governance::frozen::{ratify_frozen, apply_ratification_record}`, `ade_ledger::store_semantics::check_store_semantics_version`. |
| **Key modules** | `block_validity/` (verdict, header input/position, unsigned-header pre-image), `tx_validity/` (`phase1.rs`, `witness.rs`, `required_signers.rs`, `script_witnesses.rs` new, `utxo_free.rs` new), `mempool/` (`ingress.rs`, `admit.rs`, `canonicalize.rs`, `policy.rs`), `rules.rs`, `epoch.rs`, `epoch_accumulator.rs`, `frozen_leadership.rs`, `governance.rs` + `governance/frozen.rs` (new), `delegation.rs`, `gov_cert.rs`, `collateral.rs` (new), `utxo.rs`, `utxo_overlay.rs`, `fingerprint.rs`, `value.rs`, `pparams.rs`, `producer/`, `receive/`, `rollback/` (`materialize.rs`, `commit.rs`, `admission.rs`, `intersection.rs` new), `snapshot/`, `wal/`, `store_semantics.rs` (new), `replay_divergence.rs` (new), the reduced-view family (`stake_ref`, `pointer_resolve`, `reduced_*`, `reduced_epoch_view`, `bootstrap_manifest`), the native Mithril decoders (`ledgerdb_state`, `ledgerdb_tables`, `mithril_utxo_materialize`), `bootstrap_anchor/`, `bootstrap_bridge.rs`, `bootstrap_reward_update.rs`, `seed_consensus_inputs.rs`, `cred.rs`. |

> **Banner sub-classification (inside a BLUE `core_paths` crate; config governs).** These banners disagree with the crate's BLUE classification:
> - GREEN: `mempool/policy.rs` ("GREEN (Tier-5) mempool eviction/ordering"), `pre_resolve.rs` ("pre-resolve dependency extractor (GREEN)").
> - RED: `consensus_input_extract.rs` ("RED consensus-input extractor … parses an external dump format").
>
> They are scanned by the BLUE gates as BLUE. See the open questions.
>
> **Files with no registry locus:** `reduced_boundary.rs`, `rollback/admission.rs`, `cred.rs` and `pre_resolve.rs`. The crate-wide rows above cover them. Their own banners state:
> - `reduced_boundary.rs`: a `ReducedBoundaryProjection` "is not a `LedgerState`, cannot be widened into one, cannot be serialized as an accumulator snapshot".
> - `rollback/admission.rs`: `admit_rollback` "is NOT fork-choice", with "never a silent rematerialization".
> - `pre_resolve.rs`: "never a `HashSet`"; it is "the ONLY UTxO view BLUE validation sees".
>
> No IDSE invariant names these files.

---

### `ade_network` *(BLUE submodules)*

| Attribute | Value |
|-----------|-------|
| **Purpose** | Owns the Ouroboros mini-protocol wire grammar and pure state machines for every node-to-node and node-to-client mini-protocol (VAL-044, DET-016), plus the single mux frame codec (STR-012) and the server-role reducers that serve the durable chain (DET-017, VAL-050). *(Purpose is taken from the `lib.rs` color-partition banner and the registry loci.)* |
| **Creates** | 111 BLUE type definitions: `mux::frame::MuxFrame` + `MuxError`, `codec::primitives::ArrayHead` (closed), `codec::tx_submission::TxSubmissionTxId`, `codec::chain_sync::Point`, `codec::version::N2NVersion`, the per-protocol state/agency/event enums, and `chain_sync::server::ServedHeaderLookup`. |
| **Interprets** | Mux frames (`mux::frame::decode_frame`), handshake messages (`handshake::{n2n_transition, n2c_transition}`), and every mini-protocol message through `codec::*`. This includes FindIntersect point lists in both array forms (VAL-005) and the tx-submission wire form (VAL-043). |
| **MUST NOT** | (1) Have a second mux frame codec or handshake reducer (STR-012; `ci_check_mux_frame_closure.sh`, `ci_check_handshake_closure.sh`). (2) Choose a message variant by guessing or trying alternatives; non-matching bytes end the session (EVO-002). (3) Offer or accept a protocol version outside the enumerated set (EVO-006). (4) Accept a message from the side without agency, or let the server API build a client-agency message (SEC-016). (5) Re-encode served block bytes, or serve a header from a different block than its body (VAL-045). (6) Answer a block-fetch range with a partial batch (VAL-046). (7) Leave the chain-sync read position anywhere but the intersection point (VAL-047). (8) Build or remove a tag-24 envelope except through `ade_codec::cbor::tag24` (VAL-002). (9) Strip or guess a tx-submission id (VAL-043). (10) Emit keep-alive traffic as a pump or evidence event (VAL-061). (11) Contain `async`, `.await`, `tokio` or `async_std` (STR-001; `ci_check_no_async_in_blue.sh`, scoped by `core_paths`). (12) Depend on any workspace crate other than `ade_codec` and `ade_types` (manifest). Also the BLUE shared header. |
| **Inbound deps** | `ade_runtime`, `ade_node`, `ade_core_interop` (whole crate). |
| **Outbound deps** | `ade_types`, `ade_codec`. The crate manifest also lists `tokio`, used only by the RED `mux::transport` and the capture binaries. |
| **Entry points** | `ade_network::codec::{chain_sync, block_fetch, handshake, version, tx_submission}::*`, `ade_network::mux::frame::{encode_frame, decode_frame}`, `ade_network::handshake::{n2n_transition, n2c_transition, n2n_supported_for_magic, encode_n2n_version_params}`, `ade_network::chain_sync::server::ServedHeaderLookup`, `ade_network::block_fetch::server::*`. |
| **Key modules** | `mux/frame.rs`, `codec/` (incl. `local_chain_sync`, `local_state_query`, `local_tx_monitor`, `local_tx_submission`, `n2c_handshake`, `peer_sharing`, `keep_alive`), `handshake/`, `chain_sync/`, `block_fetch/`, `tx_submission/`, `keep_alive/`, `peer_sharing/`, `n2c/`. |

---

### `ade_plutus`

| Attribute | Value |
|-----------|-------|
| **Purpose** | The quarantine boundary between Ade's canonical ledger and the ported UPLC evaluator (`aiken-lang/aiken` `uplc`, pinned at tag `v1.1.21`). Plutus evaluation is deterministic in result and budget (DET-008) and host-independent (DET-004). *(Source: the `lib.rs` banner.)* |
| **Creates** | `PlutusScript`, `PlutusLanguage`, `EvalOutput`, `PlutusError`, `CostModels`, `DecoderMode`, `PerScriptResult`, `TxEvalResult`, `RedeemerFields`. 9 type definitions. |
| **Interprets** | UPLC scripts (V1/V2/V3), `CostModels` CBOR, and phase-2 transaction evaluation (`eval_tx_phase_two`). |
| **MUST NOT** | (1) Expose a pallas- or aiken-originated type at its public surface. Pallas references must not appear outside this crate (`lib.rs` banner; `ci_check_pallas_quarantine.sh`). (2) Be bypassed: "No other BLUE crate may import an evaluator entry point directly" (`lib.rs` banner). (3) Report a failure as anything but a closed structured value (DET-005). (4) Give a result or budget that varies by run or host (DET-008, DET-004; `ci_check_plutus_eval_purity.sh`). (5) Accept a script context or witness not bound to the preserved transaction body (SEC-008). Also the BLUE shared header (STR-001). |
| **Inbound deps** | `ade_ledger`, `ade_testkit`. |
| **Outbound deps** | `ade_types`, `ade_crypto`, `ade_codec`, `aiken_uplc` (git tag `v1.1.21`), `pallas-primitives` 0.33 (internal only). |
| **Entry points** | `ade_plutus::eval_tx_phase_two`, `ade_plutus::evaluator::{PlutusScript, EvalOutput, PlutusError}`, `ade_plutus::cost_model::CostModels`. |
| **Key modules** | `evaluator.rs`, `tx_eval.rs`, `cost_model.rs`, `script_context.rs`, `script_verdict.rs`. |

---

### `ade_types`

| Attribute | Value |
|-----------|-------|
| **Purpose** | Owns the canonical Cardano domain vocabulary every other crate shares: primitives, the era enum, addresses, era-tagged transaction bodies and outputs, certificates, governance types and native scripts. Closed sums encode invariants: `PrevHash` (VAL-003), credentials (STR-005), output quantities (VAL-013). *(Purpose is taken from the module surface and registry loci.)* |
| **Creates** | `CardanoEra`, `SlotNo`, `BlockNo`, `EpochNo`, `Hash28`, `Hash32`, `Coin`, `TxId`, `TxIn`, `PoolId`, `RewardAccount`, `shelley::cert::StakeCredential`, `shelley::block::{PrevHash, ShelleyHeader, OperationalCert}`, `conway::cert::*`, `conway::governance::*`, `mary::value::{OutputAssetQuantity, MintBurnQuantity}`. New: `allegra::script::{NativeScriptNode, MalformedPostOrder}` (heap-bounded native scripts). 86 type definitions. |
| **Interprets** | None — production-only. |
| **MUST NOT** | (1) Represent the genesis predecessor except as the `PrevHash::Genesis` variant (VAL-003). (2) Let a key-hash and a script-hash credential with the same bytes compare equal, or offer a bare-hash→credential conversion (STR-005). (3) Represent a negative output quantity, or let output arithmetic wrap or saturate (VAL-013). (4) Hold a certificate outside the closed Conway certificate grammar (VAL-011), or resolve deposits other than through the closed function (CON-002). (5) Hold proposal procedures as opaque bytes (STR-006). (6) Hold a native-script threshold as anything but signed 64-bit (VAL-017). (7) Clone, compare or drop native scripts with recursion depth that grows with nesting (SEC-009). (8) Depend on any workspace crate (manifest: no dependencies). Also the BLUE shared header. |
| **Inbound deps** | Every workspace crate except `ade_mem_diag`. |
| **Outbound deps** | None. |
| **Entry points** | `ade_types::tx::{Coin, TxIn, …}`, `ade_types::shelley::cert::StakeCredential`, `ade_types::{Hash32, Hash28, CardanoEra, SlotNo}`, `ade_types::conway::governance::*`, `ade_types::shelley::block::*`. |
| **Key modules** | `primitives.rs`, `era.rs`, `tx.rs`, `address/`, `byron/`, `shelley/`, `allegra/` (`script.rs`), `mary/` (`value.rs`), `alonzo/`, `babbage/`, `conway/`. |

---

## GREEN Modules — Deterministic Glue

> Deterministic and non-authoritative. GREEN code may depend on BLUE and must not affect authoritative outputs. Besides `ade_testkit`, GREEN code sits in banner-classified sub-trees inside `ade_network` and the RED crates. Every row below is classified by its file banner, and its MUST NOT is sourced from the registry loci that name the file.

### `ade_network::session` *(GREEN by banner)*

| Attribute | Value |
|-----------|-------|
| **Purpose** | A pure session reducer. It turns inbound byte chunks into whole per-mini-protocol messages and outbound frames, composing the mux frame codec, the handshake transition and per-protocol fan-out (DET-018). *(Source: the `session/mod.rs` banner: "GREEN session driver … every file under this module is GREEN by content".)* |
| **Creates** | `session::core::step`, `session::demux::FrameBuffer`, `session::event::AcceptedMiniProtocol` (closed), and the session state and event types. |
| **MUST NOT** | (1) Deliver a frame before the handshake is accepted; a handshaking session cannot express a delivery (SAF-011). (2) Deliver anything but one complete CBOR item per mini-protocol, or vary by chunking (DET-018). (3) Ignore a frame for a mini-protocol outside the closed registry (VAL-048). (4) Exceed its fixed reassembly bound, which is not configurable; overflow drops the peer with a typed error (SEC-038). (5) Segment outbound payloads other than through the one frame encoder (VAL-049). (6) Have a second session step reducer (STR-012; `ci_check_session_core_closure.sh`). (7) Read a clock (DET-018). |
| **Inbound deps** | `ade_runtime::network`, `ade_node`. |
| **Outbound deps** | `ade_network` BLUE submodules, `ade_codec`, `ade_types`. |
| **Entry points** | `ade_network::session::{step, FrameBuffer}`, `ade_network::session::event::*`. |

> **Promote or demote:** the banner states the module is pure, so it is a candidate to promote to BLUE (add it to `core_paths`). However, `ade_network/src/lib.rs` still lists `session` as RED. See the open questions.

### `ade_testkit` *(GREEN by banner — ambiguous)*

| Attribute | Value |
|-----------|-------|
| **Purpose** | Deterministic test and corpus infrastructure: validity and tx-validity corpora and adversarial mutations, consensus and governance stream replay, producer reference vectors, the snapshot / ledger / sync differential harnesses, `LedgerViewStub` and mempool ingress replay. It is `replay_cmd` (`cargo test -p ade_testkit`). *(Source: file banners such as "GREEN tx-validity test harness" and "GREEN harness"; no crate-level doc.)* |
| **Creates** | `validity::ConwayValidityCorpus`, `harness::snapshot_loader::LoadedSnapshot`, `consensus::ledger_view_stub::*`, the tx-validity builders and corpora, the producer fixtures and reference vectors. Not counted as canonical. |
| **MUST NOT** | (1) Compare Ade's internal ledger fingerprint with a hash of the reference node's serialized state (PRC-006). (2) Hold private key material in a replay corpus, or call a signing function during replay (DET-020; `ci_check_no_private_keys_in_corpus.sh`). (3) Report a differential without localizing the first divergence (PRC-007). (4) Use an evidence artifact that does not name its reference source and extraction method (PRC-011). (5) Lose a credential's key/script tag in its loaders (STR-005). |
| **Inbound deps** | `ade_core_interop` (plain dep); dev dep of `ade_core`, `ade_ledger`, `ade_network`, `ade_runtime` and `ade_node`. |
| **Outbound deps** | `ade_codec`, `ade_core`, `ade_crypto`, `ade_ledger`, `ade_plutus`, `ade_runtime` (**a RED crate, plain dependency**), `blake2`, `ed25519-dalek`, `flate2`, `tar`, `serde`, `serde_json`, `toml`, `cardano-crypto`. |
| **Entry points** | `ade_testkit::validity::ConwayValidityCorpus`, `ade_testkit::harness::snapshot_loader::{LoadedSnapshot, …}`, `ade_testkit::consensus::ledger_view_stub::*`, `ade_testkit::mempool::*`, `ade_testkit::tx_validity::{extract_corpus_txs, build_valid}`. |

> **Promote or demote:** this is ambiguous GREEN. It reads corpus files from disk (6 `src/` files use `std::fs` or similar) and has a plain dependency on the RED `ade_runtime`. By the import heuristic that makes it demotable to RED. No source states that it must stay off production paths. See the open questions.

### GREEN sub-trees inside `ade_runtime` *(by banner)*

| Sub-tree | Purpose (banner) | MUST NOT (registry) |
|---|---|---|
| `bootstrap` | "GREEN single bootstrap authority". It composes BLUE authorities. | Have a parallel first-run or warm-start path (STR-013). Restore the seed record without the single verified WAL fact (DUR-004), or with geometry or profile re-derived from restart input (DUR-005). Use a placeholder or genesis nonce at warm start (VAL-041). |
| `chaindb::reduced_utxo_checkpoint` | "A GREEN durable CACHE of a BLUE-derivable projection". | Differ from the ledger's own UTxO application, or produce a stake snapshot from a partial window (DET-013). Hold collateral retention inside the commitment, or keep it across a reset (DUR-011). Open with a stale semantics marker (EVO-005). |
| `chaindb::transient_epoch_view` | "Transient (GREEN, non-authoritative)" replay-window storage. | Survive as a source for follow, forge, recovery or activation. Delete anything outside its own window-key entries. Continue past an unrecognized entry (SAF-008). |
| `consensus::{chain_selector, candidate_fragment}` | GREEN chain-selector driver over BLUE `select_best_chain`. | Diverge from reference Praos selection (VAL-031), or break replay byte-identity (DET-001). |
| `consensus_inputs::*` | GREEN cardano-cli JSON → canonical consensus-input record and view. | Default a missing field, or let the raw bundle reach the core (VAL-034). Answer for any epoch but its own (VAL-035). Count rehearsal evidence as public (PRC-019). |
| `seed_consensus_merge` | GREEN mapping into `SeedEpochConsensusInputs`. | Write the seed record outside the verified-bootstrap paths (SEC-014). Re-derive geometry or profile from config (DUR-005). |
| `seed_import::*` | GREEN streaming cardano-cli UTxO importer. | Skip an entry, return a partial seed, or have a second importer (DET-022). Match an import error permissively (VAL-062). |
| `forward_sync::reducer` | GREEN forward-sync lifecycle reducer. | Recompute instead of reusing the cached UTxO commitment, or diverge from it (DET-026). Turn the persisted-anchor RollBackward into anything but a no-op (SAF-016). |
| `orchestrator::{core, event, state}` | GREEN orchestrator core. | Read a clock; only the slot crosses in (DET-019). Share state across peer sessions (SEC-019). Override the one snapshot cadence policy (STR-017). |
| `producer::{coordinator, chain_evolution, broadcast_to_served, served_chain_lookups, producer_log, tick_assembler, self_accepted_handoff}` | GREEN producer reducers. | Hold a signing key in coordinator state, or vary effects for the same event stream (DET-021). Forge against a stale base, or mint an accepted-block token (STR-018). Build a synthetic forge base (VAL-059). Drop a forge tick unaccounted (OBS-001). |
| `rollback::{cadence, in_memory_cache, persistent_cache, persistent_writer, chaindb_block_source}` | GREEN snapshot-cadence policy and caches. | Make cadence an operator setting or anything but a pure function (DET-014). Capture a snapshot outside the one policy (STR-017). |
| `receive::{events_to_state, in_memory_chain_write}`, `wal::file_wal_store` | GREEN adapters. | Break replay byte-identity (DET-001). Offer a WAL truncate, rewrite or delete (DUR-006). |

### GREEN sub-trees inside `ade_node` *(by banner)*

| Sub-tree | Purpose (banner) | MUST NOT (registry) |
|---|---|---|
| `candidate_aggregator`, `selector_state` | Build fork-choice candidates from Ade-validated headers. | Build a candidate from a peer summary or a raw followed tip, or feed a second selector (SEC-021; `ci_check_candidate_construction_validated.sh`). |
| `fork_switch` | Prove a branch before any rollback commit. | Commit a rollback before the full replacement branch validates (SEC-022). Admit a descendant before its parent (LIV-006). |
| `lca_walk` *(the banner states no GREEN classification; listed here as a pure walk)* | Fork-anchor walk within k. | Accept an anchor beyond k, or one not confirmed by the chain database (SEC-023). Let a retained rolled-back block become authority (SEC-024). |
| `post_switch_continuity` | Post-switch lineage verdict reducer. | Read the peer's tip, or be read by selection, admission or forge (DET-023). |
| `epoch_rebind` *(no GREEN banner; pure reducer by signature)* | Boundary rebind decision. | Skip a boundary (ORD-004), or fall back to another view (SAF-005). |
| `convergence_evidence`, `admission_log/*`, `admission::verdict` | Closed convergence and admission evidence. | Gate admission, rollback or fork choice (OBS-005, OBS-009). Treat "lagging" as success (OBS-006). Emit another mode's events (OBS-008). |
| `forge_intent`, `run_loop_planner` | Closed forge-activation intent and lifecycle planner. | Accept a partial operator key set (SEC-025). Forge twice per slot or at or before the last forged slot (SAF-012). Derive the slot by a second authority (DET-019). |
| `live_log::*` | Emit-only scheduling diagnostics. | Be read by the planner, or carry an open reason variant (OBS-002). |
| `mem_measure::{bounded_admission, evidence, runner}` | Bounded admission and memory evidence. | Shed work by timing or randomness (SEC-035). Let memory measurement alter semantics (PRC-013). |
| `ba02_evidence`, `rehearsal_evidence` | Peer-acceptance evidence correlator. | Count a non-allow-listed peer line (PRC-018). Promote rehearsal evidence (PRC-019). |

---

## RED Modules — Imperative Shell

> I/O, network, storage, clocks, keys, retries. RED code may depend on BLUE and GREEN. It must not modify core state except through the BLUE transitions, and must not construct semantic types from raw bytes around the codec chokepoints (STR-004).

### `ade_core_interop`

| Attribute | Value |
|-----------|-------|
| **Purpose** | Live cardano-node interop drivers: the `live_*_session` binaries, tx-submission bridges, and the chain-follow driver. *(Source: the `lib.rs` banner: "RED — Live cardano-node interop driver. This crate carries no authoritative decisions".)* |
| **Creates** | Live-session driver and transcript types. No semantic types. |
| **Interprets** | Live N2N wire data from a reference peer. Tag-24 is stripped through `ade_codec::unwrap_tag24` (VAL-002). |
| **MUST NOT** | (1) Carry authoritative decisions (`lib.rs` banner). (2) Act as a minting path for chain-selection candidates: `follow.rs`'s peer-trusted summary must not be used for candidate construction (SEC-021; `ci_check_candidate_construction_validated.sh`). (3) Strip tag-24 other than through the shared authority (VAL-002). (4) Count a live pass whose transcript misses the required window or fields (PRC-021, PRC-023). |
| **Inbound deps** | None — leaf module (binaries). |
| **Outbound deps** | `ade_core`, `ade_codec`, `ade_crypto`, `ade_ledger`, `ade_runtime`, `ade_network`, `ade_testkit`, `ade_types`, `tokio`. |
| **Entry points** | `ade_core_interop::follow::*`, `ade_core_interop::tx_submission::*`, `ade_core_interop::local_tx_submission::*`; binaries `live_consensus_session`, `live_tx_submission_session`, `live_block_production_session`, `live_block_fetch_session`, `live_block_follow_session`. |

### `ade_mem_diag`

| Attribute | Value |
|-----------|-------|
| **Purpose** | Quarantines the workspace's one `unsafe` FFI call, mimalloc `mi_collect(force=true)`, so that `ade_node` keeps `#![deny(unsafe_code)]` with zero exceptions. It is used only behind the `ADE_MEM_PHASE_DIAGNOSTIC` toggle. *(Source: the `lib.rs` CONTRACT banner.)* |
| **Creates** | No types. One function. |
| **Interprets** | Nothing semantic. |
| **MUST NOT** | (`lib.rs` CONTRACT, enforced by `ci_check_mem_diag_quarantine.sh`.) (1) Carry authoritative semantics. (2) Let any return value influence the ledger, consensus, storage, chain selection, replay fingerprints or protocol behavior. (3) Depend on BLUE crates, or be depended on by any BLUE crate. (4) Be a production memory-management or replay-agreement dependency: "admissible only as a MEASUREMENT INTERVENTION". *No IDSE invariant names this crate.* |
| **Inbound deps** | `ade_node` only. |
| **Outbound deps** | `libmimalloc-sys` (`extended`). |
| **Entry points** | `ade_mem_diag::force_allocator_collect_for_diagnostic_only`. |

### `ade_network` *(RED parts: `mux::transport`, capture binaries)*

| Attribute | Value |
|-----------|-------|
| **Purpose** | `mux::transport` is the crate's single socket and async surface. The `src/bin/capture_*` binaries are live wire-capture tools. *(Source: the `lib.rs` color partition: "RED: mux::transport"; capture banners: "RED".)* |
| **Creates** | Transport handle and runtime types. No semantic types. |
| **Interprets** | Socket bytes. Framing is delegated to `mux::frame`. |
| **MUST NOT** | (1) Use an unbounded channel, or drop silently on overflow (SEC-018). (2) Frame bytes except through the BLUE `mux::frame` codec (STR-012). (3) Leak `async` or `tokio` into a BLUE submodule (STR-001; `ci_check_no_async_in_blue.sh`). |
| **Inbound deps** | `ade_runtime`, `ade_node`, `ade_core_interop`. |
| **Outbound deps** | `ade_codec`, `ade_types`, `tokio`. |
| **Entry points** | `ade_network::mux::transport::*`; binaries `ade_*_capture`. |

### `ade_node`

| Attribute | Value |
|-----------|-------|
| **Purpose** | The `ade` binary and its library. Owns argv, the closed mode set and the live run: `--mode node` through `node_lifecycle`, the single lifecycle owner (STR-014), plus `wire_only`, `admission`, `produce` and `key_gen_kes`. It also owns automatic epoch-view activation, native Mithril first run, the forge-path rollback follow, and (new) the bootstrap-bound forge timing authority `forge_timing` (DET-019). *(Source: the `lib.rs` banner: "RED `ade_node` binary library".)* |
| **Creates** | `Cli`, `CliError`, `Mode` (closed), `ProduceCli`, `KeyGenKesCli`, `NodeSyncItem`, the exit-code constants, the `epoch_*` activation types (`epoch_activation::ActiveEpochAuthority`, …), `native_firstrun::*` errors, `forge_timing::*`, `mithril_fetch::MithrilProfile`. None is canonical-counted. |
| **Interprets** | Argv; operator key, genesis and opcert files; peer block-fetch blocks (tag-24 stripped by the shared authority, VAL-002); peer RollBackward points against the durable store (SEC-020, SAF-016, SAF-017); the native Mithril inputs (SEC-030). |
| **MUST NOT** | **Lifecycle.** (1) Let a second binary mode drive the live node, or emit a planner vocabulary that can express an authority decision (STR-014). (2) Advance the tip other than through the one admission path after bytes and WAL are durable (DUR-012, DUR-010). Self-forged blocks get no admission path of their own (STR-015). (3) Let any flag, build option or alternative path change semantics (EVO-001). **Epoch.** (4) Activate an epoch view before its record is durable (DUR-001), more than once per target epoch (SAF-004), skipping a boundary (ORD-004), or by falling back to another view (SAF-005). (5) Forge in epoch E with any authority but E's (SAF-021), or without the shared leader check (VAL-056). **Rollback and fork choice.** (6) Build a rollback target from a peer slot plus a local hash, or accept an unknown hash, a slot mismatch or Origin (SEC-020). (7) Follow a forge-path rollback into an epoch before the promoted authority (SAF-017). (8) Apply fork choice except through the existing authorities (STR-016), commit a rollback before the replacement branch validates (SEC-022), or forge while a selection is pending (SAF-014). (9) Leave the selector ahead of, behind or forked from the durable chain (REC-006). **Forge.** (10) Forge without the full operator key set, or copy key material into loop, planner or persisted state (SEC-025). (11) Sign outside the bound-genesis KES window (SEC-027). (12) Forge from a base other than the latest admission's (VAL-051, VAL-059). (13) Forge on a base a peer cannot intersect (VAL-052). **Native bootstrap.** (14) Reach a cardano-cli importer, bundle or operator seed on the native route, or fall back between routes (SEC-030). **Diagnostics.** (15) Let diagnostics or evidence influence scheduling or authority (OBS-002, OBS-005). Per their banners, `ops_log` "NEVER influences any authoritative" state and must never panic or block, and `mithril_fetch` "is the only step that talks to Mithril". (16) Allow `unsafe`: `#![deny(unsafe_code)]` in `lib.rs` and `main.rs`, with zero local allows (`ci_check_mem_diag_quarantine.sh`). |
| **Inbound deps** | None — leaf module (binary plus integration tests). |
| **Outbound deps** | `ade_types`, `ade_core`, `ade_crypto`, `ade_ledger`, `ade_runtime`, `ade_network`, `ade_codec`, `ade_mem_diag`, `tokio` (incl. `signal`), `serde`, `serde_json`, `mimalloc`. Dev: `ade_testkit`, `tempfile`, `cardano-crypto`, `ed25519-dalek`. |
| **Entry points** | `main()`; `ade_node::run_node_lifecycle`, `run_produce_mode`, `run_wire_only` and `run_key_gen_kes`; `ade_node::cli::{Cli, Mode}`; `ade_node::node_sync::*`; `ade_node::fair_merge::fair_merge`; `ade_node::native_firstrun::native_first_run_bootstrap`; `ade_node::epoch_rebind::decide_epoch_rebind`; `ade_node::forge_timing::{establish_forge_timing_authority, venue_timing_history_for_genesis}`. |
| **Key modules** | `node_lifecycle.rs` (the lifecycle owner; holds `maybe_activate_epoch_boundary`, `resolve_and_apply_peer_rollback`, `sidecar_freeze_rsw` and the fork-switch drivers), `node_sync.rs`, `fair_merge.rs`, `forge_timing.rs` (new), the `epoch_*` activation set, `native_firstrun.rs`, `mithril_fetch.rs`, `ops_log.rs`, `admission/`, `admission_log/`, `live_log/`, `mem_measure/`, `operator_forge.rs`, `produce_mode.rs`, `wire_only.rs`, `key_gen.rs`, `ba02_pass.rs`, `rehearsal_pass.rs`, and the GREEN sub-trees above. |

### `ade_runtime`

| Attribute | Value |
|-----------|-------|
| **Purpose** | The imperative shell (`CLAUDE.md`). It moves bytes and owns sockets, files, clocks and keys. Its parts: the N2N dialer, listener and mux pump; the wire pump; ChainDB and the WAL file store; the forward-sync `pump_block` admission gate; recovery; the epoch-accumulator store and its advance over ChainDB; the reduced-window driver; Mithril import and native assembly; genesis parsing; and the producer key custody and signing shell. The GREEN sub-trees are listed above. |
| **Creates** | `producer::{keys::*, signing::*, producer_shell::ProducerShell}` (key custody), `network::{outbound_command::OutboundCommand, ChainDbServedSource}`, `chaindb::{ChainDb, EpochAccumulatorStore, …}`, `wal::FileWalStore`, `mithril_native_assembly::*`, `seed_consensus_provenance::*`. No semantic types. |
| **Interprets** | Socket bytes (to the BLUE codecs), genesis files (`consensus::genesis_parser`, `producer::genesis_parser`, VAL-039, STR-009), key envelopes (SEC-006), Mithril manifests and snapshot files (SEC-029, SEC-030), and cardano-cli JSON. |
| **MUST NOT** | **Keys.** (1) Let private keys leave the RED producer shell, or let a key appear in an error, log or telemetry (SEC-001, SEC-006). (2) Sign at a KES period other than the one the header claims (SEC-005, SEC-027). **Admission and durability.** (3) Advance the tip before bytes and WAL are durable (DUR-012). It must not skip a different block by slot alone; skips go by hash (SAF-018). (4) Write a checkpoint that a crash could leave partial (DUR-007), or open a store whose semantics marker mismatches (EVO-005). (5) Recover onto a prefix that differs between ChainDB, chain-dependent state, accumulator and checkpoint (REC-002, REC-004). (6) Rewind the accumulator to an unsettled or non-canonical point, or restore without the settled-triple fingerprint (REC-001, DUR-002, SAF-006). (7) Commit a boundary advance without sealing leadership for E+2, or read leadership from anything but the sealed object (VAL-023, VAL-024). (8) Re-implement the boundary or the non-UTxO transition in parallel (VAL-020, VAL-016). **Network.** (9) Tunnel raw bytes to a peer, or let bytes for one peer reach another (SEC-017). It must not run unbounded channels (SEC-018, SEC-038), or let more than one wire pump drive a peer (STR-019). (10) Stop keep-alive under backpressure, or reorder events while parked (LIV-008, ORD-006). (11) Serve a block range without the fixed bound (SEC-036), or serve anything but the durable projection (VAL-050). **Bootstrap.** (12) Initialize a store before its anchor is verified (SEC-015). It must not take the seed record from operator bundles (SEC-014). The native route must not draw on anything but the verified snapshot and genesis (SEC-030), and Mithril binding must not compare a manifest against itself (SEC-029). |
| **Inbound deps** | `ade_node`, `ade_core_interop`, `ade_testkit` (plain dependency). |
| **Outbound deps** | `ade_types`, `ade_core`, `ade_crypto`, `ade_codec`, `ade_ledger`, `ade_network`, `redb` 2, `tokio` (incl. `signal`), `serde`, `serde_json` (`raw_value`), `bech32`, `base58`, `cardano-crypto` (`vrf-draft03`, `dsign`), `ed25519-dalek`. |
| **Entry points** | `ade_runtime::chaindb::{ChainDb, EpochAccumulatorStore, …}`, `ade_runtime::wal::FileWalStore`, `ade_runtime::seed_consensus_provenance::append_seed_epoch_provenance`, `ade_runtime::rollback::cadence::SnapshotCadence`, `ade_runtime::receive::ChainDbWriter`, `ade_runtime::network::ChainDbServedSource`, `ade_runtime::producer::producer_shell::ProducerShell`, `ade_runtime::forward_sync::pump::pump_block`, `ade_runtime::bootstrap::bootstrap_initial_state`, `ade_runtime::mithril_native_assembly::assemble_native_mithril_seed`. |
| **Key modules** | `chaindb/` (`persistent.rs`, `epoch_accumulator_store.rs`, `epoch_accumulator_advance.rs`, `reduced_window_driver.rs`, `reduced_utxo_checkpoint.rs`, `transient_epoch_view.rs`, `utxo_anchor.rs`, `crash_safety.rs`), `forward_sync/` (`pump.rs`, `reducer.rs`), `network/`, `admission/` (`wire_pump.rs`), `orchestrator/`, `producer/`, `recovery/`, `rollback/`, `receive/`, `wal/`, `mithril_import/`, `mithril_bootstrap.rs`, `mithril_native_assembly.rs`, `genesis_bootstrap.rs`, `bootstrap.rs`, `consensus/`, `consensus_inputs/`, `seed_import/`, `seed_consensus_*`, `seed_epoch_lineage.rs`, `recovered_anchor.rs` (banner: "BLUE recovered anchor-point load … RED I/O of a BLUE-authoritative" record), `clock.rs`. |

---

## Cross-Module Rules (project-wide)

### Dependency direction (RED → GREEN → BLUE; never outward)

Taken from each crate's `[dependencies]` at HEAD:

- **BLUE depends only inward (STR-003).**
  - `ade_types` has no workspace dependencies.
  - `ade_codec` and `ade_crypto` depend on `ade_types`.
  - `ade_core` depends on `ade_types` and `ade_crypto`.
  - `ade_plutus` depends on `ade_types`, `ade_crypto` and `ade_codec`.
  - `ade_ledger` depends on `ade_types`, `ade_crypto`, `ade_codec`, `ade_core` and `ade_plutus`.
  - `ade_network` depends on `ade_types` and `ade_codec`, plus `tokio` for its RED transport.

  No BLUE crate has a plain dependency on a RED or GREEN crate.
- **Dev-only transitive edge (open question).** `ade_core`, `ade_ledger` and `ade_network` dev-depend on `ade_testkit`, which has a plain dependency on the RED `ade_runtime`. STR-003 says "directly or transitively". `ci_check_dependency_boundary.sh` checks only direct edges, and only against `ade_runtime` and `ade_node`. It does not treat `ade_core_interop` or `ade_mem_diag` as RED.
- **GREEN** (`ade_testkit`, `ade_network::session`, the banner-GREEN sub-trees in RED crates) depends on BLUE.
- **RED** (`ade_runtime`, `ade_node`, `ade_core_interop`, `ade_network::mux::transport`, `ade_mem_diag`) depends on BLUE and GREEN. `ade_node` depends on `ade_runtime` and `ade_mem_diag`; `ade_core_interop` is a leaf.

### Naming convention

Every crate is named `ade_*`. Color is not encoded in names, and `CLAUDE.md` states no naming rule. Color comes from `.idse-config.json` `core_paths` and the file banners. `ade_network` is the one crate split by submodule.

### Closed enums / registries (for SEAMS §3 cross-reference)

- **Carried:**
  - `ade_types`: `CardanoEra`, `shelley::block::PrevHash`, `mary::value::{OutputAssetQuantity, MintBurnQuantity}`
  - `ade_core::consensus`: `LeaderCheckVerdict`, `LeaderEligibility`
  - `ade_ledger`: `pparams::MinUtxoRule`, `wal::WalEntry` (4 variants), `epoch_accumulator::{EpochBoundaryEffect, LedgerTransitionError}`, `frozen_leadership::FrozenLeadershipError`, `reduced_boundary::{LedgerBoundaryVerdict, LedgerValidityCapability}`, `rollback::admission::{RollbackAdmissionError, RecoveryAction}`, `bootstrap_bridge::BridgeSourceKind`
  - `ade_network`: `codec::primitives::ArrayHead`, `codec::tx_submission::TxSubmissionTxId`, `session::event::AcceptedMiniProtocol` (it lives in `session`, not `codec`)
  - `ade_node`: `cli::Mode`, `node_sync::NodeSyncItem`
- **New since `1e4896eb`:**
  - `ade_core::consensus::era_schedule`: `SlotDerivationError`, `TimingAnchorError`, `TimingAuthorityError`
  - `ade_ledger`:
    - `bootstrap_bridge::BridgeEta0Finality`
    - `bootstrap_reward_update::{SeedRewardAuthority, SnapshotRewardUpdate}`
    - `governance::frozen::{RatificationSource, RatificationOutcome, RatificationState, UnenactableReason, RatificationImportRefusal}`
    - `governance::{DanglingVoteDelegationKind, EnactableUpdateError}`
    - `rollback::intersection::{AncestryEnd, IntersectionBound, CandidateSource, IntersectReply, IntersectionOutcome, IntersectionReplyFault, IntersectionAncestryError}`
    - `store_semantics::{AuthorityArtifact, FoundSemanticsVersion, RemediationAction}`
    - `tx_validity::script_witnesses::{ScriptPurpose, ReducedScriptVerdict, ExactScriptVerdict, NativeScriptVerdict, ScriptWitnessError, ReferenceScript}`
    - `tx_validity::utxo_free::{OutputPosition, UtxoFreeRejection, AddressNetworkError}`
    - `collateral::CollateralBalanceError`
    - `replay_divergence::ResetReason`
  - `ade_types::allegra::script::NativeScriptNode`
- **Removed:** `governance::{ExecUnitsParamUpdate, ExecUnitsUpdateError}`, `SlotAlignmentError`, and the old `enum NativeScript` (replaced by the heap-bounded `NativeScriptNode` form).
- **Versioned surfaces:**
  - `ade_ledger::store_semantics::STORE_SEMANTICS_VERSION = 13`, locked by `ci/store-semantics-surface.lock` (EVO-005, PRC-005)
  - `SEED_CINPUT_SCHEMA_VERSION = 6` (STR-010)

### CI enforcement (286 gates under `ci/`)

- **Color discipline:** `ci_check_dependency_boundary.sh`, `ci_check_no_async_in_blue.sh`, `ci_check_forbidden_patterns.sh`, `ci_check_no_float_in_consensus.sh`, `ci_check_no_semantic_cfg.sh`, `ci_check_no_signing_in_blue.sh`, `ci_check_ingress_chokepoints.sh`, `ci_check_hash_uses_wire_bytes.sh`, `ci_check_module_headers.sh`, `ci_check_pallas_quarantine.sh`, `ci_check_mem_diag_quarantine.sh`.
- **Hard-coded BLUE lists.** Except `ci_check_no_async_in_blue.sh`, which reads `core_paths`, these gates carry their own BLUE crate and path lists. Their header comments still describe `session` as RED and the `ade_network` BLUE submodules as "placeholders".

**31 gates added since `1e4896eb` (for the HEAD_DELTAS §5 cross-reference):** `ci_check_accumulator_refold_bound.sh`, `ci_check_bnd_typed_stall_cause.sh`, `ci_check_bridge_nonce_freeze_differential.sh`, `ci_check_coadvance_retry.sh`, `ci_check_collateral_balance_resolver.sh`, `ci_check_collateral_retention_positioning.sh`, `ci_check_donation_pot.sh`, `ci_check_durable_intersection.sh`, `ci_check_epoch_agreement.sh`, `ci_check_every_supplied_witness_verifies.sh`, `ci_check_eview_refold_reseal.sh`, `ci_check_followed_peer_tip_served_evidence.sh`, `ci_check_forge_slot_authority.sh`, `ci_check_frozen_ratification_authority.sh`, `ci_check_native_script_depth_independent.sh`, `ci_check_native_script_threshold_signed.sh`, `ci_check_native_script_witness_verdict.sh`, `ci_check_owner_stake_authority.sh`, `ci_check_phase2_invalid_utxo_effect.sh`, `ci_check_recovery_fault_self_describing.sh`, `ci_check_seed_reward_authority.sh`, `ci_check_settled_rewind_survives_recovery.sh`, `ci_check_settled_triple_integrity.sh`, `ci_check_sigma_denominator_authority.sh`, `ci_check_snapshot_fidelity.sh`, `ci_check_store_semantics_gate.sh`, `ci_check_store_semantics_lock.sh`, `ci_check_utxo_free_admission_obligations.sh`, `ci_check_venue_constant_containment.sh`, `ci_check_venue_differential.sh`, `ci_check_wire_liveness.sh`.

**27 gates no registry entry names (for TRACEABILITY):** `ci_check_alloc_determinism_neutral.sh`, `ci_check_eview_activation.sh`, `ci_check_eview_stability_gate.sh`, `ci_check_genesis_replay_open_obligation.sh`, `ci_check_live_blockfetch_byte_only.sh`, `ci_check_live_transcript_forge_base.sh`, `ci_check_mem_compare_evidence.sh`, `ci_check_mem_diag_quarantine.sh`, `ci_check_mem_opt_s1_reduction.sh`, `ci_check_mem_opt_s2_import_peak.sh`, `ci_check_mem_opt_s3_owned.sh`, `ci_check_mem_opt_utxo_disk_s0.sh`, `ci_check_module_headers.sh`, `ci_check_no_secrets.sh`, `ci_check_pallas_quarantine.sh`, `ci_check_recovered_anchor_intersectable.sh`, `ci_check_reduced_boundary_plane.sh`, `ci_check_registry_unique_ids.sh`, `ci_check_single_producer_extend_own_spine.sh`, `ci_check_single_producer_loop_continuation.sh`, `ci_check_trusted_replay_boundary.sh`, `ci_check_utxo_admission_seam.sh`, `ci_check_utxo_disk_anchor.sh`, `ci_check_utxo_disk_key.sh`, `ci_check_utxo_fp_cache.sh`, `ci_check_utxo_pre_resolve.sh`, `ci_check_wire_rollback_signal_preserved.sh`.

---

## Generation notes

Regenerated at HEAD `08c7c3f5` by introspecting the following:

- the Cargo workspace and each crate's manifest
- the crate roots and file banners
- `clippy.toml`
- the IDSE registry's `enforcement_runtime` loci and invariant statements
- `ci/`

Every MUST NOT row is sourced from one of these: a registry invariant whose locus names the module or whose statement names its construct; a lint attribute or `clippy.toml`; a source banner; or a CI gate. **No MUST NOT row is guessed.** No cluster documents were used: no IDSE cluster exists yet, and `.idse-config.json` `clusters_dir` (`docs/clusters`) does not exist. Legacy cluster documents under `docs/legacy/` are frozen design input and are not cited.

**Gaps (need human judgment or registry follow-up):**

1. **Purpose has no doc-comment, README or manifest-description source** for `ade_codec`, `ade_core`, `ade_crypto`, `ade_ledger` and `ade_types`. None of the 12 manifests has a `description`, and no crate has a README or a crate-level `//!` purpose. Those Purpose rows were composed from module surfaces and registry loci.
2. **Unlocated files.** No registry locus names `ade_ledger::{reduced_boundary, rollback::admission, cred, pre_resolve}`, `ade_node::{mithril_fetch, ops_log}` or the whole `ade_mem_diag` crate. Their file-specific prohibitions come from banners only.
3. **Color ambiguities:**
   - `ade_network::session`: `lib.rs` says RED, `session/mod.rs` says GREEN by content, and it is not in `core_paths`.
   - `ade_testkit`: GREEN banners, but it does file I/O and has a plain dependency on RED.
   - `ade_ledger::{mempool::policy, pre_resolve}` (GREEN banners) and `consensus_input_extract` (RED banner) sit inside a BLUE `core_paths` crate.
4. **27 CI gates no registry entry names.** This includes three color-discipline gates: `ci_check_module_headers.sh`, `ci_check_pallas_quarantine.sh`, `ci_check_mem_diag_quarantine.sh`. **25 registry entries name no enforcement locus:** SAF-002, STR-003, PRC-001, PRC-002, ORD-001, SEC-002, STR-004, PRC-003, VAL-015, EVO-003, CON-004, DUR-003, SEC-015, VAL-040, SEC-031, DUR-014, SEC-034, PRC-012, SEC-037, PRC-014, PRC-015, PRC-016, PRC-017, PRC-020, REC-008.
5. **Test build.** `crates/ade_plutus/tests/conway_validity_range_reproducer.rs` `include_str!`s fixtures from the git-ignored `target/aiken_divergent_fixture/`, so `cargo test --workspace` (`test_cmd`) fails to compile in a checkout without them.
6. **Legacy IDs in sources.** 310 of the 721 tracked `.rs` files and 236 `ci/*.sh` scripts still cite legacy IDs in comments. Follow them through `docs/legacy/idd-to-idse-map.toml`.

The doc is regenerated, not hand-edited. If a value drifts, fix the source, not the doc.
