# Module Authority Map — Ade

> **Status:** Living architectural document. Regenerated; not hand-edited.
> Per-project instance of `~/.claude/methodology/templates/codemap.md`.

> 11 crates, 438 canonical types, 1927 tests, 89 CI checks at HEAD (`d6f3399`, PHASE4-N-P close).

---

## Conventions

- A **module** in Ade is a Cargo workspace crate (smallest independently-buildable unit). One exception: `ade_network` is split by *submodule color* — its BLUE submodules, its GREEN submodules, and its RED submodules are documented as separate entries below, because `.idd-config.json` `core_paths` resolves BLUE at the submodule path level rather than crate-wide. The `ade_core::consensus` submodule sits *inside* the BLUE `ade_core` crate and is covered by that entry. The `ade_ledger::block_validity` / `ade_ledger::consensus_view` / `ade_ledger::tx_validity` / `ade_ledger::mempool::admit` / `ade_ledger::mempool::ingress` / `ade_ledger::cert_classify` modules sit inside the BLUE `ade_ledger` crate and are covered by that entry; the `ade_ledger::producer` sub-tree (PHASE4-N-C — `forge`, `self_accept`, `state`; PHASE4-N-G — `served_chain`), the `ade_ledger::block_body_hash` top-level module (PHASE4-N-C S4 single body-hash authority), the `ade_ledger::receive` sub-tree (PHASE4-N-H), the `ade_ledger::rollback` sub-tree (PHASE4-N-I — `traits`, `error`, `materialize`, `commit`), and the `ade_ledger::snapshot` sub-tree (PHASE4-N-J — `error`, `chain_dep`, `utxo_state`, `cert_state`, `epoch_state`, `gov_state`, `ledger`, `framing`) are likewise BLUE under the already-BLUE `ade_ledger` crate prefix. The RED `ade_ledger::consensus_input_extract`, the GREEN `ade_ledger::mempool::policy`, and the GREEN `ade_ledger::mempool::canonicalize` sit inside the BLUE `ade_ledger` crate but carry a different color by their own module doc-comment and the cluster TCB Color Maps — they are surfaced as sub-classification notes inside the `ade_ledger` entry. The `ade_testkit::mempool` sub-tree (PHASE4-N-E S2), the `ade_testkit::governance` sub-tree (PROPOSAL-PROCEDURES-DECODE PP-S2), and the `ade_testkit::producer` sub-tree (PHASE4-N-C) sit inside the GREEN `ade_testkit` crate, classified GREEN by their own module doc-comments and the cluster TCB Color Maps. **PHASE4-N-G — `ade_runtime::network` sub-tree** sits inside the RED `ade_runtime` crate and hosts the per-peer N2N **server** session driver (RED). **PHASE4-N-H — `ade_runtime::receive` sub-tree** sits inside the RED `ade_runtime` crate and hosts the GREEN+RED receive-side glue: `events_to_state` (GREEN), `in_memory_chain_write` (GREEN), `orchestrator` (RED). **PHASE4-N-I — `ade_runtime::rollback` sub-tree** sits inside the RED `ade_runtime` crate and hosts the GREEN+RED rollback adapter glue: `cadence` (GREEN), `in_memory_cache` (GREEN), `chaindb_block_source` (GREEN), `snapshot_writer` (RED). **PHASE4-N-J extends `ade_runtime::rollback` with `persistent_cache` (GREEN; closes DC-CONS-21).** **PHASE4-N-K extends `ade_runtime::rollback` with `persistent_writer` (GREEN; cadence-fidelity glue calling `PersistentSnapshotCache::capture` — DC-NODE-02), introduces the new top-level files `ade_runtime::bootstrap` (GREEN; sole `pub fn bootstrap_initial_state` — CN-NODE-01) and `ade_runtime::clock` (GREEN trait + GREEN `DeterministicClock` + the RED `SystemClock` sub-classified inside the same file — DC-NODE-03), and a new sub-tree `ade_runtime::orchestrator` (mixed) hosting the GREEN core reducer `core` + GREEN closed-vocabulary `event` + GREEN `state` + barrel `mod`, alongside the RED tokio runners `peer_session` + `leadership_session` + `n2n_server_pump`. PHASE4-N-K reshapes `ade_node` from a hello-world stub into a lib+bin**: `src/lib.rs` (RED library entry; re-exports `Cli`/`CliError`/`run_node_until_shutdown`/`NodeStartupInputs`/`NodeShutdownEvidence`/`NodeRunError` + exit-code constants), `src/cli.rs` (RED argv parser), `src/node.rs` (RED `run_node_until_shutdown` lifecycle), and the refactored `src/main.rs` (RED bin shim). **PHASE4-N-L promotes `ade_network::session` from an empty RED placeholder to a populated GREEN sub-tree by content** (6 files: `mod`, `event`, `state`, `demux`, `core`, `handshake_driver` — pure reducer + closed `AcceptedMiniProtocol` registry + `SessionState` type-state + partial-frame buffer + handshake driver over an opaque `Transport` trait). The `.idd-config.json` `_core_paths_doc` still classifies `session/` as RED, but every PHASE4-N-L file carries a GREEN module banner and is gated by `ci/ci_check_session_core_closure.sh` + `ci/ci_check_clock_seam.sh` (extended); the GREEN-by-content sub-classification is documented in `session/mod.rs` and surfaced here as a new GREEN module entry. **PHASE4-N-L extends `ade_network::mux::transport` (still RED) with `MuxTransportHandle` + closed `TransportError` sum + `DuplexCapacity::DEFAULT` + `spawn_duplex` while preserving the old `MuxTransport` / `open_tcp` API.** **PHASE4-N-L adds two RED files inside `ade_runtime::network/` (`mux_pump.rs` + `n2n_dialer.rs`) and one RED file inside `ade_runtime::orchestrator/` (`keep_alive_session.rs`).** **PHASE4-N-L adds one `OrchestratorEvent` variant `OutboundKeepAlive { peer_id }` (additive; closed enum re-closed).** **NEW — PHASE4-N-P adds the BLUE sub-tree `crates/ade_crypto/src/kes_sum/`** (8 files: `mod`, `single`, `sum`, `hash`, `errors`, `period`, `cardano_cli_corpus` (`#[cfg(test)]`), `tests` (`#[cfg(test)]`)) — the Ade-owned `Sum6KES Ed25519DSIGN` algorithm, byte-identical to Haskell `cardano-base`. After PHASE4-N-P S5, `KesSecret.inner` in `ade_runtime::producer::signing` consumes the BLUE signing key directly; `cardano-crypto` Rust 1.0.8 is demoted to a `#[cfg(test)]` oracle in `ade_crypto`'s KES path and is dropped from `ade_runtime/Cargo.toml`'s feature list. VRF + DSIGN paths continue to use `cardano-crypto` upstream.
- Modules are listed by TCB color (BLUE → GREEN → RED), alphabetical within each color.
- TCB color sources, in order of authority:
  1. `.idd-config.json` `core_paths` — substring match against absolute path. BLUE matches: `ade_codec`, `ade_types`, `ade_crypto` (covers `ade_crypto::{blake2b, ed25519, error, kes, kes_sum, traits, vrf}` — the new `kes_sum/` sub-tree added by PHASE4-N-P inherits BLUE from the `ade_crypto/` prefix match), `ade_core`, `ade_ledger` (covers `ade_ledger::{snapshot, rollback, receive, producer, block_validity, tx_validity, mempool, ...}`), `ade_plutus`, and the 9 `ade_network` submodule paths (`mux/frame.rs`, `codec/`, `handshake/`, `chain_sync/`, `block_fetch/`, `tx_submission/`, `keep_alive/`, `peer_sharing/`, `n2c/`).
  2. `.idd-config.json` `_core_paths_doc` — `ade_runtime` is RED; `ade_testkit` is GREEN; `ade_node` is RED; `ade_network::mux::transport` and `ade_network::session` are nominally RED (PHASE4-N-L: `session/` is now GREEN-by-content — see the GREEN entry below); `ade_network::lib` and `ade_network::mux::mod` are GREEN barrels. `ade_core_interop` is **RED** per its `Cargo.toml` header comment and the PHASE4-N-B TCB Color Map; the `ade_core_interop::tx_submission` (S4) and `ade_core_interop::local_tx_submission` (S5) modules carry a GREEN sub-classification by their own module doc-comments. The RED binaries `ade_core_interop::bin::{live_tx_submission_session, live_block_production_session, live_block_fetch_session, live_block_follow_session}` are the operator-action live evidence pattern.
  3. `docs/clusters/completed/PHASE4-N-B/cluster.md` § "TCB Color Map" — `era_schedule`, `praos_state`, `vrf_cert`, `nonce`, `op_cert`, `leader_schedule`, `header_validate`, `fork_choice`, `rollback` (consensus-side header rollback applier) are BLUE; `chain_selector`, `candidate_fragment` are GREEN; `genesis_parser` is RED; `ade_testkit::consensus` is GREEN; `ade_core_interop` is RED.
  4. PHASE4-B1 — `ade_ledger::consensus_view`, `ade_ledger::block_validity` BLUE; `ade_core::consensus::{header_validate, kes_check}` extensions BLUE; `ade_testkit::validity` GREEN; `ade_ledger::consensus_input_extract` RED.
  5. PHASE4-B2 — `ade_ledger::tx_validity::*`, `ade_ledger::mempool::admit` BLUE; `ade_ledger::mempool::policy` GREEN (Tier-5); `ade_testkit::tx_validity` GREEN.
  6. PHASE4-B3/B5 — all new/changed modules BLUE under already-BLUE crate prefixes.
  7. PHASE4-N-E — `ade_ledger::mempool::ingress` BLUE (S1); `ade_ledger::mempool::canonicalize` GREEN (S3); `ade_testkit::mempool::ingress_replay` GREEN (S2); `ade_core_interop::tx_submission` GREEN (S4); `ade_core_interop::local_tx_submission` GREEN (S5); `ade_core_interop::bin::live_tx_submission_session` RED (S6).
  8. PROPOSAL-PROCEDURES-DECODE — `ade_codec::conway::governance` BLUE; new `ProposalProcedure` in `ade_types::conway::governance` BLUE; `ade_testkit::governance::proposal_procedures_replay` GREEN.
  9. PHASE4-N-C — `ade_ledger::producer::{forge, self_accept, state}` BLUE; `ade_ledger::block_body_hash` BLUE; `ade_codec::shelley::{opcert, tx_components}` BLUE; `ade_core::consensus::opcert_validate` BLUE; `ade_crypto::kes::KesSignature` BLUE; `ade_runtime::producer::{signing, keys, scheduler, broadcast}` RED; `ade_runtime::producer::tick_assembler` GREEN; `ade_testkit::producer::*` GREEN; `ade_core_interop::bin::live_block_production_session` RED.
  10. PHASE4-N-G — `ade_network::chain_sync::server`, `ade_network::block_fetch::server` BLUE; `ade_ledger::producer::served_chain` BLUE; `accepted_block_header_bytes` accessor BLUE; `ade_runtime::producer::{broadcast_to_served, served_chain_lookups}` GREEN; `ade_runtime::network::n2n_server` RED; `ade_core_interop::bin::live_block_fetch_session` RED.
  11. PHASE4-N-H — `ade_ledger::receive::{admitted, chain_write, events, pending_header_cache, reducer}` BLUE; `ade_runtime::receive::{events_to_state, in_memory_chain_write}` GREEN; `ade_runtime::receive::orchestrator` RED; `ade_core_interop::bin::live_block_follow_session` RED.
  12. PHASE4-N-I — `ade_ledger::rollback::{traits, error, materialize, commit}` BLUE; `ChainDbWrite::rollback_to_slot` extension BLUE; the `RollbackContext<'a>` + reducer extensions BLUE; `ade_runtime::rollback::{cadence, in_memory_cache, chaindb_block_source}` GREEN; `ade_runtime::rollback::snapshot_writer` RED.
  13. PHASE4-N-J — `ade_ledger::snapshot::{error, chain_dep, utxo_state, cert_state, epoch_state, gov_state, ledger, framing}` BLUE; `ade_runtime::rollback::persistent_cache` GREEN.
  14. `docs/clusters/completed/PHASE4-N-K/cluster.md` § "TCB color map (FC/IS partition)" — `ade_runtime::bootstrap` GREEN (CN-NODE-01). `ade_runtime::clock` trait + `DeterministicClock` GREEN; `ade_runtime::clock::SystemClock` (same file) RED (DC-NODE-03). `ade_runtime::orchestrator::{mod, event, state, core}` GREEN (`ci/ci_check_orchestrator_core_purity.sh`). `ade_runtime::rollback::persistent_writer` GREEN (DC-NODE-02). `ade_runtime::orchestrator::{peer_session, leadership_session, n2n_server_pump}` RED (DC-NODE-01). `ade_node::{cli, node, lib, main}` RED. No new BLUE in this cluster.
  15. `docs/clusters/completed/PHASE4-N-L/cluster.md` § "TCB color map (FC/IS partition)" — `ade_network::session::{mod, core, event, state, demux, handshake_driver}` are **GREEN**. `ade_network::mux::transport` (extended) remains **RED**. `ade_runtime::network::{mux_pump, n2n_dialer}` are **RED**. `ade_runtime::orchestrator::keep_alive_session` is **RED**.
  16. **NEW — `docs/clusters/completed/PHASE4-N-P/cluster.md` § "TCB Color Map"** — `ade_crypto::kes_sum::{mod, single, sum, hash, errors, period}` are **BLUE** (inherited from the `crates/ade_crypto/` BLUE prefix in `.idd-config.json` `core_paths`; the new sub-tree gains BLUE classification automatically by path inclusion). `ade_crypto::kes_sum::{cardano_cli_corpus, tests}` are `#[cfg(test)]`-only and not classified for production. `ade_runtime::producer::{signing, keys}` carry forward as **RED** (the PHASE4-N-P S5 migration changes `KesSecret.inner`'s type and `load_kes_signing_key_skey`'s body, but the RED-confined custody discipline and module classification are unchanged). **No new GREEN modules; no new RED modules.**

- **Active cluster at HEAD (none).** **PHASE4-N-P (Sum6KES expanded compatibility) is closed at HEAD `d6f3399` and archived under `docs/clusters/completed/PHASE4-N-P/`** (closure record at `docs/clusters/completed/PHASE4-N-P/CLOSURE.md`; S5 close commit `6973318` + close-archive commit `d6f3399`). The cluster ships the Ade-owned `Sum6Kes` BLUE algorithm (`KesAlgorithm` trait, `Sum0Kes` leaf, generic recursive `SumKes<D>`, domain-separated `expand_seed`, `period_from_zeroed_sum6_tree_shape`, closed `KesError` + `KesParseError`), the 608-byte expanded skey serde + 448-byte signature serde, a cardano-cli ground-truth corpus (3 throwaway-comment-prefixed hex-literal fixtures), the `KesSecret.inner` migration from `cardano_crypto::kes::Sum6Kes::SigningKey` to `ade_crypto::kes_sum::Sum6Kes::SigningKey` (RED `ade_runtime::producer::signing.rs`), the `load_kes_signing_key_skey` flip from fail-closed to accept-608-valid via the BLUE deserializer (RED `ade_runtime::producer::keys.rs`), and the drop of the `kes-sum` cardano-crypto feature in `ade_runtime/Cargo.toml`. New registry rules at `status = "enforced"`: **`DC-CRYPTO-08`** (Ade-owned Sum6KES algorithm is Haskell-equivalent — `derive_verification_key` / `gen_key_kes_from_seed_bytes` / `update_kes` / `sign_kes` byte-identical to Haskell `cardano-base`'s `Sum6KES Ed25519DSIGN`; cross-impl validation against cardano-cli ground-truth corpus under `#[cfg(test)]`); **`DC-CRYPTO-09`** (608-byte expanded signing-key serde + 448-byte signature serde + `current_period_of_signing_key` period inference are byte-canonical; closed `KesParseError` for every malformed-payload shape). `OP-OPS-04.open_obligation` cleared (was "PHASE4-N-P deferral"). `DC-CRYPTO-07.open_obligation` cleared (was "fail-closed always until PHASE4-N-P"); statement narrowed to "608-byte valid → `Ok(KesSecret)` via BLUE deserializer; any other shape fail-closes via closed `KesParseError` / `KeyLoadError`". Existing rules strengthened (`strengthened_in += "PHASE4-N-P"`): **`DC-CRYPTO-03`** (KES key custody now backed by Ade-owned signing-key types with hand-rolled `Drop` zeroize), **`DC-CRYPTO-04`** (KES signing transcript equivalence now validated against Haskell ground truth rather than only `cardano-crypto` Rust), **`DC-CRYPTO-05`** (one-way evolution discipline now backed by Ade-owned `SumKes::update_kes` + `ZeroizingSeed` Drop). One discovery recorded in CLOSURE.md §Key discovery: `cardano-crypto` Rust 1.0.8 uses different `expand_seed` prefix bytes (`0x00`/`0x01`) than Haskell `cardano-base` (`0x01`/`0x02`); Ade matches Haskell. `docs/clusters/completed/` now contains 30 directories (the 29 prior closures plus PHASE4-N-P).

- **Honest-scope gap (NEW — load-bearing).** This CODEMAP refresh narrows its delta narrative to **the PHASE4-N-P cluster only**. The prior CODEMAP HEAD was `d62c2bc` + PHASE4-N-L worktree; nine clusters closed between that baseline and HEAD `d6f3399` (**PHASE4-N-L-LIVE, PHASE4-N-M-A, PHASE4-N-M-A1.1, PHASE4-N-M-B, PHASE4-N-M-C, PHASE4-N-M-FRAG, PHASE4-N-M-SCHED, PHASE4-N-M-FOLLOW, PHASE4-N-O, and PHASE4-N-P itself**), and those clusters added BLUE seed-import / WAL / bootstrap-anchor authorities, GREEN admission-runner / evidence-reducer / ade-kes-envelope code, RED admission-orchestrator code, and ~21 CI scripts that this CODEMAP does **not** yet inventory module-by-module. Per-module Purpose / Creates / Interprets / MUST NOT / Inbound / Outbound / Entry-point rows below for `ade_ledger`, `ade_runtime`, `ade_node`, `ade_core_interop` are **carry-forward-from-N-L-worktree** for the non-KES surface; the test count (1927) and CI inventory (89) and BLUE canonical-type total (438) at the top of this file are mechanical recounts at HEAD, but their delta breakdowns below pre-date the intervening clusters. This is a scoped refresh — a full per-cluster CODEMAP regeneration covering the N-L-LIVE → N-O intermediate clusters is a separate work item (carried as gap (yy) in the residual list at the bottom of this file).

- **Delta since prior CODEMAP HEAD `d62c2bc` + PHASE4-N-L worktree (this regeneration — PHASE4-N-P cluster only, 5 slices S1–S5).**

  **Structural deltas summarized:**
  - **NEW BLUE sub-tree `ade_crypto/src/kes_sum/`** (8 files, ~1933 LOC including tests; was nonexistent at the prior HEAD).
    - `kes_sum/mod.rs` (240 LOC) — barrel + `KesAlgorithm` trait (closed BLUE surface: `total_periods`, `gen_key_kes_from_seed_bytes`, `derive_verification_key`, `update_kes`, `sign_kes`, `verify_kes`, `raw_serialize_signing_key_kes`, `raw_deserialize_signing_key_kes`, `raw_serialize_signature_kes`, `raw_deserialize_signature_kes`, `current_period_of_signing_key`) + closed `KesError` (5 variants: `InvalidSeedLength`, `PeriodOutOfRange`, `VerificationFailed`, `KeyExpired`, `Ed25519(&'static str)`) + `Sum1Kes..Sum6Kes` type aliases + compile-time size assertions (Sum6 SK = 608, sig = 448, VK = 32). Re-exports `period_from_zeroed_sum6_tree_shape`, `KesParseError`, `Sum0Kes`/`Sum0Signature`/`Sum0SigningKey`, `SumKes`/`SumSignature`/`SumSigningKey`.
    - `kes_sum/single.rs` (222 LOC, S2) — `Sum0Kes` (leaf — `SingleKes<Ed25519>`), `Sum0SigningKey { seed: [u8; 32] }` with hand-rolled `Drop` zeroize + redacted `Debug`, `Sum0Signature { sig: [u8; 64] }`. Signs / verifies via `ed25519-dalek` directly; no `cardano-crypto` dependency.
    - `kes_sum/sum.rs` (466 LOC, S2 + S3) — generic recursive `SumKes<D: KesAlgorithm>`; `SumSigningKey<D>` holds `(child_sk, r1_seed: Option<ZeroizingSeed>, vk_left: [u8;32], vk_right: [u8;32])`; `SumSignature<D> { sigma: D::Signature, vk_left, vk_right }`; recurrence: `SK_size = D::SK_size + 96`, `SIG_size = D::SIG_size + 64`, `VK_size = 32`, `total_periods = 2 * D::total_periods`. `ZeroizingSeed` newtype with `Drop` overwrite — held inside `Option<ZeroizingSeed>` so `Option::take` consumes it during `update_kes` (avoids the partial-move-from-Drop coherence problem). S3 adds `raw_serialize_signing_key_kes` / `raw_deserialize_signing_key_kes` (recursive vk-consistency check per the S1 proof-obligation doc), `raw_serialize_signature_kes` / `raw_deserialize_signature_kes`, `current_period_of_signing_key`.
    - `kes_sum/hash.rs` (61 LOC, S2 + S4 correction) — `expand_seed(&[u8; 32]) -> ([u8; 32], [u8; 32])` via domain-separated `Blake2b256(0x01 || seed)` / `Blake2b256(0x02 || seed)` (Haskell `cardano-base` convention); `hash_concat_vk(vk_left, vk_right) = Blake2b256(vk_left || vk_right)`. **S4 byte-load-bearing correction**: prefix bytes corrected from `0x00`/`0x01` (which matched upstream `cardano-crypto` Rust 1.0.8) to `0x01`/`0x02` (which matches Haskell, which is what `cardano-cli` emits) after the cardano-cli ground-truth corpus rejected the S2 implementation with `InconsistentSubtreeVkRight { level: 2 }`. Gated by `ci/ci_check_kes_sum_compatibility.sh` Guard 4 (literal-byte check).
    - `kes_sum/errors.rs` (55 LOC, S3) — closed `KesParseError` (6 variants: `WrongPayloadSize { actual: usize }`, `LeafSignKeyAllZero`, `InconsistentSubtreeVkLeft { level: u32 }`, `InconsistentSubtreeVkRight { level: u32 }`, `LevelOutOfRange { level: u32 }`, `InvalidEd25519SignatureLength { actual: usize }`). Every variant carries only non-secret metadata (`u32` / `usize`); no key bytes, no hex, no decimal seed runs.
    - `kes_sum/period.rs` (75 LOC, S3) — `period_from_zeroed_sum6_tree_shape(bytes: &[u8; 608]) -> Result<u32, KesParseError>` implementing the S1 proof-obligation pseudocode: walks levels 6 → 1, accumulates `2^(level-1)` at each level where the level's seed is zero; leaf-all-zero is fail-closed via `KesParseError::LeafSignKeyAllZero`. Returns `Ok(period)` with `period ∈ 0..=63`.
    - `kes_sum/cardano_cli_corpus.rs` (200 LOC, S4, `#[cfg(test)]`) — 3 throwaway-fixture-comment-prefixed `pub(super) const SKEY{N}: &[u8; 608]` + matching VKs captured from `cardano-cli 11.0.0.0 node key-gen-KES`. Each fixture is preceded by the load-bearing comment `// TEST ONLY: throwaway deterministic fixture generated for Sum6KES …`. Enforced by `ci/ci_check_kes_sum_compatibility.sh` Guard 1.
    - `kes_sum/tests.rs` (614 LOC, S2/S3/S4, `#[cfg(test)]`) — 35 unit tests covering Sum0/Sum1/Sum6 sign+verify, 64-period chain, period-inference, serde round-trip across every period 0..=63, malformed-payload negatives (8 sizes), inconsistent-vk-left / inconsistent-vk-right at level 6, leaf-zero, signature serde size, redacted-`Debug` assertions, zeroizing-seed-drop-overwrites-bytes, cardano-cli ground-truth corpus VK matches + cross-impl sign-verify + flip-one-byte negative, and two divergence-documenting tests against `cardano-crypto` Rust 1.0.8 (`sum6_kes_seed_expansion_diverges_from_cardano_crypto_rust_1_0_8` + `sum6_kes_vk_diverges_from_cardano_crypto_rust_for_same_seed`).
  - **MIGRATED `ade_crypto/src/kes.rs`** — `verify_kes` body now consumes the BLUE algorithm: `use crate::kes_sum::{KesAlgorithm, Sum6Kes}` + `Sum6Kes::raw_deserialize_signature_kes` + `Sum6Kes::verify_kes`. The existing `KesSignature`, `KesPeriod`, `KesVerificationKey`, `SUM6_KES_SIG_LEN`, `OperationalCertData`, `verify_kes_signature`, `verify_opcert`, `build_opcert_signable` surfaces are unchanged. Test-only `cardano_crypto::kes::*` imports remain in inline `#[cfg(test)]` blocks for cross-impl oracle assertions only.
  - **EXTENDED `ade_crypto/src/lib.rs`** — adds `pub mod kes_sum;` to the module list. No new top-level re-export (`kes_sum`'s public types are consumed via `ade_crypto::kes_sum::*` qualified paths from the RED consumer).
  - **MIGRATED RED `ade_runtime/src/producer/signing.rs`** — `KesSecret.inner` field type changed from `<cardano_crypto::kes::Sum6Kes as KesAlgorithm>::SigningKey` to `<ade_crypto::kes_sum::Sum6Kes as KesAlgorithm>::SigningKey`. All `kes_sign` / `kes_update` / `verification_key_fingerprint` / `from_bytes_zeroizing` call sites migrated to the BLUE API. New `pub(super) fn KesSecret::from_blue_signing_key(inner, current_period) -> Self` constructor (consumed only by `keys.rs::load_kes_signing_key_skey`). Inline `Drop` deferral comment retired (per-field `ZeroizingSeed::Drop` + Sum0 hand-rolled `Drop` now do the work). `cardano_crypto::vrf::VrfDraft03` import preserved (VRF path unchanged). Redacted `Debug`, no public byte accessors, RED-only custody — all carry forward unchanged.
  - **MIGRATED RED `ade_runtime/src/producer/keys.rs`** — `load_kes_signing_key_skey` body flipped from "fail-closed unconditional via `KeyLoadError::UnsupportedExpandedKesKeyFormat`" to "608-byte valid → `Ok(KesSecret)` via `Sum6Kes::raw_deserialize_signing_key_kes` + `Sum6Kes::current_period_of_signing_key` + `KesSecret::from_blue_signing_key`; wrong-size payloads stay fail-closed via `UnsupportedExpandedKesKeyFormat`; structurally-invalid 608-byte payloads (truncated sub-tree, inconsistent vk hash, leaf-all-zero) fail-close via new variant `KeyLoadError::KesParse(ade_crypto::kes_sum::KesParseError)`". Closed `KeyLoadError` enum gains one variant (`KesParse(KesParseError)`); the other variants and the path-leak-prevention discipline carry forward unchanged.
  - **CARGO TOML CHANGES.** `crates/ade_crypto/Cargo.toml` still lists `cardano-crypto` with features `["vrf-draft03", "kes-sum", "dsign"]` — the `kes-sum` feature is **retained** because `#[cfg(test)]` cross-impl oracle assertions in `kes_sum/tests.rs` + `kes.rs::tests` still consume `cardano_crypto::kes::Sum6Kes` as a comparison source. This is gated by `ci/ci_check_kes_sum_compatibility.sh` Guard 3 (no `cardano_crypto::kes` import outside `#[cfg(test)]` in `ade_crypto/src/**`). `crates/ade_runtime/Cargo.toml` `cardano-crypto` features narrow from `["vrf-draft03", "kes-sum", "dsign"]` to `["vrf-draft03", "dsign"]` — `kes-sum` is dropped because the runtime no longer signs through the upstream KES algorithm; VRF (`vrf-draft03`) and cold-key (`dsign`) continue unchanged.
  - **NEW closed canonical types — counted toward the BLUE total.** In `ade_crypto::kes_sum` (BLUE): `Sum0Kes` (struct), `Sum0SigningKey` (struct), `Sum0Signature` (struct), `SumKes<D>` (struct), `SumSigningKey<D>` (struct), `SumSignature<D>` (struct), `KesParseError` (enum), `KesError` (enum). **+8 BLUE canonical types.** `ZeroizingSeed` is `pub(super)` not `pub` and does NOT count. **BLUE canonical-type total: 424 → 438** at HEAD (delta is exactly the 8 new `kes_sum` types; ade_codec / ade_types / ade_core / ade_ledger / ade_plutus / ade_network counts unchanged for the N-P scope).
  - **NEW CI gate (+1).** `ci/ci_check_kes_sum_compatibility.sh` (S4) — 4 guards: Guard 1 (cardano-cli corpus exists + every `SKEY{N}` const is preceded by the throwaway-fixture comment + ≥ 3 fixtures); Guard 2 (no `.skey` envelope files committed under `crates/ade_crypto/`); Guard 3 (`cardano_crypto::kes` only imported under `#[cfg(test)]` in `crates/ade_crypto/src/**` — KES-scoped; VRF + DSIGN imports remain permitted); Guard 4 (`expand_seed` prefix bytes literal-match Haskell convention `0x01`/`0x02`, defense-in-depth forbids `0x00`/`0x01`). **EXTENDED:** `ci/ci_check_kes_envelope_closed.sh` Guard 2 narrows — the cardano-cli loader body must contain a `raw_deserialize_signing_key_kes` call (the new accept path) and must NOT contain `KesSecret::from_bytes_zeroizing` / `KesSecret::from_seed_at_period` calls (which would bypass the structural validator); `UnsupportedExpandedKesKeyFormat` retained for the size-mismatch branch. **CI script count: 66 → 89** at HEAD. The +1 PHASE4-N-P delta is `ci_check_kes_sum_compatibility.sh`; the remaining +22 are from the intervening clusters that this scoped refresh does not inventory by name (gap (yy)).
  - **NEW registry rules / strengthenings (PHASE4-N-P scope only).** 2 new entries at `status = "enforced"`: `DC-CRYPTO-08` (Ade-owned Sum6KES algorithm Haskell-equivalent; `introduced_in = "PHASE4-N-P"`); `DC-CRYPTO-09` (Sum6KES expanded skey + signature serde + period inference byte-canonical; `introduced_in = "PHASE4-N-P"`). 5 existing rules strengthened (`strengthened_in += "PHASE4-N-P"`): `DC-CRYPTO-03`, `DC-CRYPTO-04`, `DC-CRYPTO-05`, `DC-CRYPTO-07` (statement narrowed; `open_obligation` cleared), `OP-OPS-04` (`open_obligation` cleared). Registry total at HEAD: **265 entries** (up from 223 at the N-L baseline; the 42-entry delta covers PHASE4-N-P (+2) plus the intervening clusters' rules added in N-L-LIVE / N-M-* / N-O — not inventoried per gap (yy)).
  - **NEW test inventory (+35 in `ade_crypto`).** Per-crate at HEAD: `ade_codec` 162 (unchanged in N-P), `ade_types` 23 (unchanged in N-P), `ade_crypto` 51 → **86 (+35)** (35 unit tests in `kes_sum/tests.rs`; 16 inline tests in `kes.rs` already counted), `ade_core` 126 (unchanged in N-P), `ade_ledger` 594 (delta beyond N-P scope), `ade_plutus` 28 (unchanged in N-P), `ade_testkit` 312 (delta beyond N-P scope), `ade_runtime` 274 (delta beyond N-P scope), `ade_network` 234 (delta beyond N-P scope), `ade_node` 61 (delta beyond N-P scope), `ade_core_interop` 27 (unchanged in N-P). **Test inventory total at HEAD: 1927** (counts via `grep -cE "#\[test\]|#\[tokio::test\]"` workspace-wide; reported approximate per the template's fallback rule). The +35 PHASE4-N-P delta is entirely in `kes_sum/tests.rs`; the remaining +189 are from intervening clusters (gap (yy)).
  - **NEW crate-level dependency changes.** `ade_runtime/Cargo.toml` cardano-crypto features narrow — `kes-sum` dropped. `ade_crypto/Cargo.toml` carries `cardano-crypto` features unchanged at `["vrf-draft03", "kes-sum", "dsign"]` (test-only consumer; gated by `ci_check_kes_sum_compatibility.sh` Guard 3). No new external workspace edges. No new internal workspace edges — `ade_runtime::producer::signing.rs` already depended on `ade_crypto`; the migration is a within-crate-import flip.

- Counts:
  - Crates: 11, from `Cargo.toml` `[workspace] members`. Unchanged.
  - Canonical types: **438** at HEAD, from `grep -rE "^(pub )?(struct|enum) " --include='*.rs' crates/{ade_codec,ade_types,ade_crypto,ade_core,ade_ledger,ade_plutus}/src/ + the 9 BLUE ade_network submodule paths`. Breakdown at HEAD: `ade_codec` 10 (unchanged), `ade_types` 81 (unchanged), `ade_crypto` 21 (was 13; +8 from `kes_sum`'s 8 public `struct`/`enum` declarations — `Sum0Kes`, `Sum0SigningKey`, `Sum0Signature`, `SumKes<D>`, `SumSigningKey<D>`, `SumSignature<D>`, `KesParseError`, `KesError`), `ade_core` 44 (unchanged), `ade_ledger` 165 (delta beyond N-P scope — likely +6 from N-M-* WAL / bootstrap-anchor types), `ade_plutus` 8 (unchanged), plus 9 BLUE `ade_network` submodule paths 109 (unchanged). Registry `canonical_type_registry: null`, so a structural grep count is used.
  - Tests: **1927** at HEAD — count of `#[test]` / `#[tokio::test]` attributes across `crates/`. Approximate per template fallback. **+35 in `ade_crypto/src/kes_sum/tests.rs` (PHASE4-N-P scope)**; +189 elsewhere across intervening clusters (gap (yy)).
  - CI checks: **89** at HEAD — file count under `ci/ci_check_*.sh`. **+1 in PHASE4-N-P scope (`ci_check_kes_sum_compatibility.sh`)**; +22 across intervening clusters (gap (yy)).

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
> - `ci/ci_check_no_signing_in_blue.sh` — `SigningKey`/`SecretKey`/`PrivateKey`/`private_key`/`sign_message`/`sign_block` forbidden in BLUE. **Note for PHASE4-N-P:** the new BLUE `Sum0SigningKey` / `SumSigningKey<D>` types in `ade_crypto::kes_sum` are signing-key *types* (not signing *operations*); the upstream rule covers code that performs signing, which lives in RED `ade_runtime::producer::signing`. The BLUE types own only the algorithm + custody discipline (zeroize on Drop, no public byte accessors); the actual `kes_sign` is RED-confined. Confirmed mechanically by `ci/ci_check_private_key_custody.sh` passing post-N-P with the new `kes_sum/` whitelist entry.
> - `ci/ci_check_no_semantic_cfg.sh` — `#[cfg(feature = ...)]` and `cfg!(feature = ...)` forbidden in BLUE.
> - `ci/ci_check_hash_uses_wire_bytes.sh` — no hashing of `canonical_bytes` / re-encoded bytes in BLUE.
> - `ci/ci_check_ingress_chokepoints.sh` — only named `decode_*` chokepoints construct `PreservedCbor`.
> - `ci/ci_check_pallas_quarantine.sh` — `pallas-*` references confined to `ade_plutus`.
> - `ci/ci_check_no_async_in_blue.sh` *(PHASE4-N-A, S-A1)* — async constructs forbidden anywhere in the BLUE scope. Enforces DC-CORE-01.
>
> Three additional CI scripts narrow the shared header to the `ade_core::consensus` tree (PHASE4-N-B): `ci/ci_check_no_chaindb_in_consensus_blue.sh`, `ci/ci_check_no_float_in_consensus.sh`, `ci/ci_check_consensus_closed_enums.sh` (TARGETS scope: `crates/ade_core/src/consensus/`, `crates/ade_ledger/src/block_validity/`, `crates/ade_ledger/src/tx_validity/`, `crates/ade_ledger/src/mempool/`).
> A fourth narrow check enforces a single fork-choice rule: `ci/ci_check_no_density_in_fork_choice.sh` (DC-CONS-03).
> A fifth narrow check (PHASE4-B3) enforces deposit-parameter authority: `ci/ci_check_deposit_param_authority.sh` (DC-TXV-07).
> A sixth narrow check (PHASE4-B5) enforces gov-cert accumulation closure: `ci/ci_check_gov_cert_accumulation_closed.sh` (DC-LEDGER-09).
> A seventh narrow check (OQ5/COMMITTEE/DREP/ENACTMENT-COMMITTEE-FIDELITY + ENACTMENT-COMMITTEE-WRITEBACK) enforces credential-discriminant closure: `ci/ci_check_credential_discriminant_closed.sh` (DC-LEDGER-10).
> An eighth narrow check (PROPOSAL-PROCEDURES-DECODE, PP-S1) enforces proposal_procedures closure: `ci/ci_check_proposal_procedures_closed.sh` (DC-LEDGER-11).
>
> **Eight narrow checks added by PHASE4-N-C (the producer-authority gate set):** `ci_check_forge_purity.sh` (DC-CONS-13/14/15); `ci_check_no_producer_body_encoder.sh` (DC-CONS-16); `ci_check_no_private_keys_in_corpus.sh` (DC-CRYPTO-03/04/05 corpus discipline); `ci_check_opcert_closed.sh` (DC-CONS-11/12); `ci_check_private_key_custody.sh` (DC-CRYPTO-03/04/05 + OP-OPS-04); `ci_check_producer_corpus_present.sh` (CN-CONS-06 mechanical half); `ci_check_scheduler_closure.sh` (DC-CONS-13 + DC-MEM-03 + OP-OPS-05); `ci_check_self_accept_gate.sh` (CN-CONS-07).
>
> **Seven narrow checks added by PHASE4-N-G (the producer-side server-role gate set):** `ci_check_no_parallel_header_splitter.sh`; `ci_check_served_chain_closure.sh`; `ci_check_chain_sync_server_closure.sh`; `ci_check_block_fetch_server_closure.sh`; `ci_check_broadcast_to_served_purity.sh`; `ci_check_n2n_server_no_signing_dep.sh`; `ci_check_server_paths_corpus_present.sh`.
>
> **Four narrow checks added by PHASE4-N-H (the receive-side authority gate set):** `ci_check_admitted_block_closure.sh`; `ci_check_receive_reducer_closure.sh`; `ci_check_receive_replay_purity.sh`; `ci_check_receive_orchestrator_no_producer_dep.sh`; `ci_check_receive_paths_corpus_present.sh`.
>
> **Two narrow checks added by PHASE4-N-I (the rollback authority gate set):** `ci_check_rollback_materialize_closure.sh` (CN-STORE-07 + DC-CONS-22); `ci_check_snapshot_cadence_purity.sh` (DC-STORE-07).
>
> **One narrow check added by PHASE4-N-J (the snapshot encoder single-authority gate):** `ci_check_snapshot_encoder_closure.sh` (CN-STORE-08 + DC-STORE-08 + DC-STORE-09).
>
> **Six narrow checks added by PHASE4-N-K (the node-orchestrator gate set; all scope `ade_runtime` GREEN files + `ade_node` source — none target BLUE crates):** `ci_check_bootstrap_closure.sh` (CN-NODE-01); `ci_check_clock_seam.sh` (DC-NODE-03 — extended in N-L to also cover `ade_network::session/`); `ci_check_orchestrator_core_purity.sh` (DC-NODE-03 + general purity); `ci_check_persistent_writer_no_parallel_cadence.sh` (DC-NODE-02); `ci_check_peer_session_isolation.sh` (DC-NODE-01); `ci_check_node_binary_uses_single_bootstrap.sh` (CN-NODE-01 + DC-NODE-04).
>
> **Five narrow checks added by PHASE4-N-L (the wire-session gate set; all scope `ade_network::session/` + `ade_network::mux/` + `ade_runtime::{network, orchestrator}` paths — none target BLUE crates):**
> - `ci/ci_check_mux_frame_closure.sh` (S1; CN-SESS-01) — repo-wide grep gate asserting a single pub `encode_frame` / `decode_frame` pair in the workspace.
> - `ci/ci_check_handshake_closure.sh` (S1; CN-SESS-02) — repo-wide grep gate asserting a single pub `n2n_transition` and a single pub `n2c_transition`.
> - `ci/ci_check_session_core_closure.sh` (S2; CN-SESS-03 + DC-SESS-01 + DC-SESS-05 session-side) — `session::core::step` is the only pub reducer in `session/`; `Handshaking`/`Connected` type-state structurally present; session core files contain no `tokio::*` imports.
> - `ci/ci_check_mini_protocol_id_registry_closed.sh` (S1; DC-SESS-02) — `AcceptedMiniProtocol` enum is closed; the dispatch table is a `match` over it with no wildcard accept.
> - `ci/ci_check_session_no_unbounded.sh` (S5; DC-SESS-04) — no `mpsc::unbounded_channel` / `unbounded`-named queue constructor in `session/` / `mux_pump.rs` / `n2n_dialer.rs` / `keep_alive_session.rs`.
>
> **NEW — One narrow check added by PHASE4-N-P (the Sum6KES compatibility gate; scopes `ade_crypto/src/kes_sum/` + `ade_crypto/src/` + `ade_runtime/Cargo.toml`):**
> - `ci/ci_check_kes_sum_compatibility.sh` (S4; DC-CRYPTO-08 + DC-CRYPTO-09) — 4 guards: (1) cardano-cli corpus exists + every `SKEY{N}` const is preceded by the throwaway-fixture comment + ≥ 3 fixtures (gates DC-CRYPTO-08's cross-impl evidence); (2) no `.skey` envelope files committed under `crates/ade_crypto/` (gates N6 hard prohibition); (3) `cardano_crypto::kes` only imported under `#[cfg(test)]` in `crates/ade_crypto/src/**` — KES-scoped (gates N9 hard prohibition); (4) `expand_seed` prefix bytes literal-match Haskell convention `0x01`/`0x02` and forbid `0x00` (gates the byte-load-bearing S4 correction).
>
> Two checks narrow the shared header to the wire-ingress path of `ade_ledger::mempool` (PHASE4-N-E): `ci_check_mempool_ingress_closure.sh` (DC-MEM-03, S1); `ci_check_mempool_ingress_replay.sh` (DC-MEM-04, S2 + S3).
>
> A BLUE crate or BLUE `ade_network` submodule that adds a feature flag, an async function, a `HashMap`, or a RED dep fails CI on push.
> The 3 RED-scope CI scripts (`ci_check_chaindb_contract.sh`, `ci_check_recovery_contract.sh`, `ci_check_chaindb_crash_safety.sh`) and the 1 evidence script `ci_check_ce_n_a_5_proof.sh` are not part of this shared header — they are documented in the cross-module CI matrix at the bottom.

---

### `ade_codec`

> **Status carried from prior CODEMAP — unchanged at HEAD `d6f3399` (PHASE4-N-P close).** PHASE4-N-P did not touch `ade_codec`. The existing snapshot-encoder consumption sites carry forward unchanged.

| Attribute | Value |
|-----------|-------|
| **Purpose** | Owns Cardano-canonical CBOR ingress: the only place in the workspace that turns raw bytes into typed semantic values, with wire-byte preservation for every hash-bearing structure. Also owns the standalone opcert byte authority (`shelley::opcert::{encode_opcert, decode_opcert}`) and the canonical Conway-tx preserved-byte splitter (`shelley::tx_components::split_conway_tx_components`). |
| **Creates** | `PreservedCbor<T>`, `RawCbor`, `BlockEnvelope`, `ByronDecodedBlock`, `CodecContext`, `CodecError` (incl. `UnknownCertTag`, `DuplicateMapKey`, `TrailingBytes`, `InvalidCborStructure`), `ContainerEncoding`, `IntWidth`, plus era-tagged block/tx wrappers under `byron/`, `shelley/`, `allegra/`, `mary/`, `alonzo/`, `babbage/`, `conway/`. Functions: `conway::cert::{decode_conway_certs, decode_drep}`, `conway::withdrawals::{decode_withdrawals, withdrawals_sum}`, `conway::governance::{decode_proposal_procedures, encode_proposal_procedures}`, `shelley::cert::read_pool_registration_cert`, both era `decode_stake_credential`, `shelley::opcert::{encode_opcert, decode_opcert, write_opcert_fields_into}`, `shelley::tx_components::split_conway_tx_components`. Types: `OpCertCodecError`, `TxComponents<'a>`. |
| **Interprets** | All canonical Cardano CBOR — block envelopes, era-specific blocks, tx bodies, tx outs, certificates, addresses. Conway certificate array (closed over CDDL tags 0..18, owner-complete) and Conway withdrawals map (deduplicated). Both era credential decoders preserve the key/script tag. Sole authority for `PreservedCbor::new` (constructor is `pub(crate)`). CIP-1694 `proposal_procedure` array. Standalone cardano-cli `OperationalCertificate` 4-tuple. Conway transaction 4-tuple (preserved-byte slicing only). |
| **MUST NOT** | (1) Construct `PreservedCbor` outside `ade_codec` (`pub(crate)` + `ci_check_ingress_chokepoints.sh`). (2) Re-encode wire bytes when computing hashes (`ci_check_hash_uses_wire_bytes.sh`). (3) Use any forbidden BLUE pattern. (4) Depend on any other workspace crate except `ade_types`. (5) `conway::cert` (DC-LEDGER-08) — no unknown-tag swallow; owner-complete; no catch-all. (6) `conway::withdrawals` — no last-wins on duplicate `RewardAccount`. (7) `decode_stake_credential` (DC-LEDGER-10) — must not erase the credential tag. (8) `conway::governance` (DC-LEDGER-11) — no silent skip on unknown `GovAction`; no opaque pass-through at body codec key 20. (9) `shelley::opcert` (DC-CONS-11/12) — cardano-byte-identical 4-tuple; dedicated `OpCertCodecError` variant per shape failure; not a second header-embedded opcert encoder (enforced by `ci_check_opcert_closed.sh`). (10) `shelley::tx_components` (DC-CONS-13/16) — preserved-byte slices that alias the input buffer (no `to_vec` / clones); reject non-4-tuple shapes; not re-encode the boolean validity flag or the auxiliary-data null. |
| **Inbound deps** | `ade_ledger` (heavy), `ade_plutus`, `ade_testkit`, `ade_network`, `ade_runtime`, `ade_core_interop`, `ade_node`. No new inbound crate edge in PHASE4-N-P. |
| **Outbound deps** | `ade_types`. No external dependencies; std-only. Dev-deps: `serde_json`, `toml`. |
| **Entry points** | `ade_codec::cbor::envelope::decode_block_envelope`, `ade_codec::cbor`, `ade_codec::traits::AdeEncode`, `ade_codec::CodecContext`, per-era `decode_*_block`, `ade_codec::address::decode_address`. B2: `ade_codec::conway::tx::decode_conway_tx_body`. B3: `ade_codec::conway::cert::decode_conway_certs` and `ade_codec::conway::withdrawals::{decode_withdrawals, withdrawals_sum}`. PP-S1: `ade_codec::conway::governance::{decode_proposal_procedures, encode_proposal_procedures}`. N-C: `ade_codec::shelley::opcert::{encode_opcert, decode_opcert, OpCertCodecError}`, `ade_codec::shelley::tx_components::{split_conway_tx_components, TxComponents}`. |
| **Key modules** | `cbor/`, `byron/`, `shelley/` (incl. `cert.rs`, `opcert.rs`, `tx_components.rs`), `allegra/`, `mary/`, `alonzo/`, `babbage/`, `conway/`, `address/`, `preserved.rs`, `traits.rs`, `primitives.rs`, `error.rs`. |

---

### `ade_core`

> **Status carried from prior CODEMAP — unchanged at HEAD `d6f3399` (PHASE4-N-P close).** PHASE4-N-P did not touch `ade_core`.

| Attribute | Value |
|-----------|-------|
| **Purpose** | BLUE authoritative Praos consensus core. Owns canonical types and pure state-transitions that decide which header / chain Ade accepts: HFC era schedule + slot↔era↔time translation, Praos chain-dep state (nonce / op-cert counters), VRF cert verification + leader-eligibility predicate, KES signature + op-cert period verification wired into header admission, header validation pipeline, fork-choice, header-level rollback authority, leader-schedule query, canonical encodings of all chain-dep state and chain events. Owns the producer-side opcert acceptance authority (`opcert_validate`). |
| **Creates** | **Schedule:** `BootstrapAnchorHash`, `EraSchedule`, `EraSummary`, `EraLocation`. **State:** `PraosChainDepState`, `OpCertCounterMap`, `Nonce`. **Events/points:** `Point`, `ChainHash`, `BlockDistance`, `SecurityParam`, `ChainEvent`, `ChainSelectionReject`. **Errors:** `HFCError`, `SlotTimeError`, `OutsideForecastRange`, `HeaderValidationError`, `FieldError`, `FieldKind`, `VrfCertError`, `OpCertCounterError`, `NonceEvolutionError`, `LeaderScheduleError`, `OpCertError`. **Header surface:** `HeaderInput`, `HeaderVrf`, `HeaderKes`, `ValidatedHeaderSummary`, `HeaderApplied`. **Fork-choice:** `TiebreakerView`, `CandidateFragment`, `ChainSelectorState`, `ForkChoiceError`. **Op-cert/nonce:** `OpCertObservation`, `NonceInput`. **VRF:** `VrfRole`, `VerifiedVrf`, `StakeFraction`, `ActiveSlotsCoeff`. **Leader schedule:** `LeaderScheduleQuery`, `LeaderScheduleAnswer`. **Ledger view boundary:** `LedgerView` trait. **Rollback (header-level):** `RollBackRequest`, `RollBackApplied`. **Encoding:** `DecodeError`. 44 public types. |
| **Interprets** | Canonical inputs from the `ade_runtime` shell. KES check verifies hot KES key signature over the header body CBOR bytes via the BLUE consumer `ade_crypto::kes::verify_kes` (PHASE4-N-P S3 redirected internally to the Ade-owned `ade_crypto::kes_sum::Sum6Kes::verify_kes` — no observable change at this consumer). `opcert_validate` consumes an `OperationalCert`, cold key, expected period, and prev counter. |
| **MUST NOT** | (1)–(14) carry-forward from prior CODEMAP. |
| **Inbound deps** | `ade_ledger`, `ade_runtime` (heavy), `ade_testkit`, `ade_core_interop`, `ade_node`. No new inbound crate edge in PHASE4-N-P. |
| **Outbound deps** | `ade_types`, `ade_crypto`. Dev-deps: `ade_testkit`, `serde_json`, `cardano-crypto`. |
| **Entry points** | `use ade_core::consensus::{...}` aggregator, `ade_core::consensus::ledger_view::LedgerView`, `ade_core::consensus::vrf_cert::*`, `ade_core::consensus::kes_check::*` (now backed by Ade-owned KES verification via `ade_crypto::kes::verify_kes`), `ade_core::consensus::praos_state::*`, `ade_core::consensus::header_summary::*`. Top-level transitions: `validate_and_apply_header`, `select_best_chain`, `apply_rollback` (header-level), `apply_nonce_input`, `apply_op_cert`, `query_leader_schedule`, `verify_vrf_cert`, `tiebreaker_prefer`, `encode/decode_chain_dep_state`, `encode/decode_chain_event`. `ade_core::consensus::opcert_validate::{opcert_validate, OpCertError}`. |
| **Key modules** | `consensus/era_schedule.rs`, `consensus/praos_state.rs`, `consensus/events.rs`, `consensus/errors.rs`, `consensus/vrf_cert.rs`, `consensus/kes_check.rs`, `consensus/nonce.rs`, `consensus/op_cert.rs`, `consensus/leader_schedule.rs`, `consensus/header_summary.rs` + `consensus/header_validate.rs`, `consensus/candidate.rs`, `consensus/fork_choice.rs`, `consensus/rollback.rs`, `consensus/ledger_view.rs`, `consensus/encoding.rs`, `consensus/opcert_validate.rs`. |

---

### `ade_crypto`

> **Status change at HEAD `d6f3399` (PHASE4-N-P close).** PHASE4-N-P adds the new BLUE sub-tree `kes_sum/` (8 files, 6 production + 2 `#[cfg(test)]`; ~1933 LOC) — the Ade-owned `Sum6KES Ed25519DSIGN` algorithm. `kes.rs` migrates its `verify_kes` internals from `cardano_crypto::kes::Sum6Kes` to `ade_crypto::kes_sum::Sum6Kes` (the existing public surface — `KesSignature`, `KesPeriod`, `KesVerificationKey`, `SUM6_KES_SIG_LEN`, `OperationalCertData`, `verify_kes`, `verify_kes_signature`, `verify_opcert`, `build_opcert_signable` — is byte-identically unchanged). `lib.rs` gains `pub mod kes_sum;`. The `Cargo.toml` `cardano-crypto` features `["vrf-draft03", "kes-sum", "dsign"]` are retained because `#[cfg(test)]` cross-impl oracle assertions still consume `cardano_crypto::kes::Sum6Kes`; production-code import is mechanically forbidden by `ci/ci_check_kes_sum_compatibility.sh` Guard 3 (`cardano_crypto::kes` only inside `#[cfg(test)]` under `crates/ade_crypto/src/**`). N9 hard prohibition: no compatibility shim via `unsafe`, `transmute`, vendored `pub(crate)` access, or fork-only constructors — `kes_sum` is a from-first-principles reimplementation.

| Attribute | Value |
|-----------|-------|
| **Purpose** | Owns pure cryptographic verification primitives — Blake2b hashing, Ed25519 / Byron-bootstrap signature verification, KES verification with operational certificates, VRF verification, plus the closed signature-artifact types (`KesSignature`, `VrfProof`, `VrfOutput`, `Ed25519Signature`) that BLUE consumes across the RED→BLUE boundary. Verification only — signing lives in `ade_runtime::producer::signing`. **NEW — PHASE4-N-P:** also owns the Ade-native `Sum6KES Ed25519DSIGN` algorithm — `derive_verification_key`, `gen_key_kes_from_seed_bytes`, `update_kes`, `sign_kes`, `verify_kes`, `raw_serialize_signing_key_kes`, `raw_deserialize_signing_key_kes`, `raw_serialize_signature_kes`, `raw_deserialize_signature_kes`, `current_period_of_signing_key`, `period_from_zeroed_sum6_tree_shape` — byte-identical to Haskell `cardano-base`. The algorithm authority is BLUE; private-key custody for signing remains RED-confined in `ade_runtime::producer::signing.rs`. |
| **Creates** | **Existing (carry-forward):** `Blake2b224`, `Blake2b256`, `HashAlgorithm` trait, `Ed25519VerificationKey`, `Ed25519Signature`, `ByronExtendedVerificationKey`, `KesVerificationKey`, `KesPeriod`, `OperationalCertData`, `VrfVerificationKey`, `VrfProof`, `VrfOutput`, `CryptoError`, `KesSignature(pub [u8; SUM6_KES_SIG_LEN])`, `SUM6_KES_SIG_LEN: usize = 448`. **NEW — PHASE4-N-P (`kes_sum::`):** `KesAlgorithm` trait (closed BLUE surface with associated `SigningKey` / `Signature` types and consts `LEVEL`, `ALGORITHM_NAME`, `SEED_SIZE`, `SIGNING_KEY_SIZE`, `SIGNATURE_SIZE`, `VERIFICATION_KEY_SIZE = 32`); `Sum0Kes` (leaf — `SingleKes<Ed25519>`); `Sum0SigningKey { seed: [u8; 32] }`; `Sum0Signature { sig: [u8; 64] }`; generic `SumKes<D>`; `SumSigningKey<D: KesAlgorithm>` (holds `child_sk`, `r1_seed: Option<ZeroizingSeed>`, `vk_left`, `vk_right`); `SumSignature<D: KesAlgorithm>` (holds `sigma`, `vk_left`, `vk_right`); type aliases `Sum1Kes..Sum6Kes`; closed `KesError` (5 variants); closed `KesParseError` (6 variants). **+8 BLUE canonical types.** |
| **Interprets** | Verification key / signature / proof byte structures. Not a CBOR parser — accepts already-decoded byte slices. **NEW — PHASE4-N-P:** `Sum6Kes::raw_deserialize_signing_key_kes` interprets the canonical 608-byte expanded `Sum6KES Ed25519DSIGN` payload emitted by Haskell `cardano-base` / `cardano-cli node key-gen-KES` — the byte layout is the structural validator (`InconsistentSubtreeVkLeft` / `InconsistentSubtreeVkRight` / `LeafSignKeyAllZero` are the closed parse-time failures). `Sum6Kes::raw_deserialize_signature_kes` interprets the canonical 448-byte signature. `period_from_zeroed_sum6_tree_shape` interprets the shape (which sub-seeds are zero) of a 608-byte payload to infer its current period. |
| **MUST NOT** | (1) Implement signing as a `pub fn sign_*` at the crate top level (`ci_check_no_signing_in_blue.sh`). **Note:** the BLUE `KesAlgorithm::sign_kes` is a trait *method* on a closed BLUE surface; the actual RED-confined `kes_sign` wrapper that calls it (with period bounds + range checks + wrapper-discipline) lives in `ade_runtime::producer::signing`. The signing-key *type* (`SumSigningKey<D>` / `Sum0SigningKey`) is BLUE; signing *operations* are RED-driven. The `ci_check_private_key_custody.sh` script carries a PHASE4-N-P whitelist for `kes_sum/` because that sub-tree contains `pub struct .*SigningKey` declarations that are part of the closed algorithm surface, not a custody escape. (2) Allocate global state. (3) Use any BLUE forbidden pattern. (4) Use `unsafe` outside the allowlisted FFI in `src/vrf.rs`. (5) `build_opcert_signable` must produce the spec-correct raw concatenation. (6) `KesSignature` (DC-CRYPTO-04) — closed length-pinned wrapper; only `from_bytes` construction; custom redacting `Debug`; no `PartialOrd`/`Ord`/`Hash` derives. (7) `KesSignature` carries no `Drop` impl that zeroizes — `KesSignature` is BLUE and not secret. **(8) NEW — PHASE4-N-P (`kes_sum`, DC-CRYPTO-08):** MUST NOT import `cardano_crypto::kes::*` in any production-code path under `crates/ade_crypto/src/**` (only `#[cfg(test)]` cross-impl oracle imports allowed; gated by `ci/ci_check_kes_sum_compatibility.sh` Guard 3). **(9) NEW — `kes_sum::hash::expand_seed` (DC-CRYPTO-08):** MUST use the Haskell `cardano-base` domain-separation prefix bytes `0x01` / `0x02`; MUST NOT use the `cardano-crypto` Rust 1.0.8 prefixes `0x00` / `0x01` (gated by `ci/ci_check_kes_sum_compatibility.sh` Guard 4 — literal-byte check). **(10) NEW — `kes_sum` signing-key types (DC-CRYPTO-08 + DC-CRYPTO-05):** every `*SigningKey` MUST have a hand-rolled `Drop` that best-effort zeroizes its sub-seed buffers (Sum0 hand-rolled; SumKes via per-field `ZeroizingSeed`); MUST NOT expose public byte accessors; MUST have a custom redacting `Debug` impl. **(11) NEW — `kes_sum::cardano_cli_corpus` (N6 hard prohibition):** every `pub(super) const SKEY{N}: &[u8; 608]` MUST be preceded by the throwaway-fixture comment; MUST NOT commit any `.skey` envelope file anywhere under `crates/ade_crypto/` (gated by `ci/ci_check_kes_sum_compatibility.sh` Guards 1 and 2). **(12) NEW — `kes_sum::errors::KesParseError` + `KesError` (DC-CRYPTO-08 + DC-CRYPTO-09):** every variant payload is a non-secret primitive (`u32` / `usize` / `&'static str`); MUST NOT carry raw key bytes, hex representations of secret material, or path strings. **(13) NEW — `kes_sum::period::period_from_zeroed_sum6_tree_shape` (DC-CRYPTO-09):** MUST be heuristic-free — returns exactly one `u32 ∈ 0..=63` or `KesParseError::LeafSignKeyAllZero`; MUST NOT silently accept period > 63 tree shapes. |
| **Inbound deps** | `ade_core`, `ade_ledger`, `ade_plutus`, `ade_testkit`, `ade_core_interop`, `ade_runtime`. **NEW from PHASE4-N-P:** `ade_runtime::producer::{signing, keys}` consume `ade_crypto::kes_sum::{KesAlgorithm, Sum6Kes, KesParseError, KesError}` directly (was: consumed `cardano_crypto::kes::*`). No new workspace-crate-level inbound edge — the migration is a within-crate-import flip at the consumer. |
| **Outbound deps** | `ade_types`, `blake2`, `ed25519-dalek`, `cardano-crypto` (vrf-draft03 + kes-sum + dsign features, `default-features = false`) — `kes-sum` is retained for `#[cfg(test)]` cross-impl oracle assertions only; production code path under `crates/ade_crypto/src/**` does NOT import `cardano_crypto::kes` (gated by `ci/ci_check_kes_sum_compatibility.sh` Guard 3). |
| **Entry points** | `ade_crypto::blake2b::*`, `verify_ed25519`, `verify_byron_bootstrap`, `verify_kes`, `verify_opcert`, `verify_vrf`, `ade_crypto::kes::{KesSignature, SUM6_KES_SIG_LEN, KesVerificationKey, KesPeriod, OperationalCertData}`. **NEW — PHASE4-N-P:** `ade_crypto::kes_sum::{KesAlgorithm, Sum0Kes, Sum1Kes, …, Sum6Kes, KesError, KesParseError, period_from_zeroed_sum6_tree_shape, Sum0SigningKey, Sum0Signature, SumSigningKey, SumSignature}`. |
| **Key modules** | `blake2b.rs`, `ed25519.rs`, `error.rs`, `kes.rs` (migrated internals to `kes_sum`; public surface unchanged), `kes_sum/` (NEW — `mod.rs`, `single.rs`, `sum.rs`, `hash.rs`, `errors.rs`, `period.rs`, `cardano_cli_corpus.rs` `#[cfg(test)]`, `tests.rs` `#[cfg(test)]`), `traits.rs`, `vrf.rs`. |
| **Feature flags** | None at `ade_crypto` level. The cardano-crypto feature `kes-sum` is retained for test-only consumption — see Outbound deps. |

---

### `ade_ledger`

> **Status carried-forward (scoped-refresh note).** PHASE4-N-P did not touch `ade_ledger`; row contents below are carry-forward from the prior CODEMAP HEAD `d62c2bc` + PHASE4-N-L worktree. The intervening clusters PHASE4-N-M-A / N-M-A1.1 / N-M-B / N-M-C / N-M-FRAG / N-M-SCHED / N-M-FOLLOW added BLUE seed-import + bootstrap-anchor + WAL + admission-orchestrator types under this crate that are not inventoried here; the canonical-type count delta (159 → 165 = +6) reflects these intervening additions but the per-type names are not enumerated in this refresh (gap (yy)).

| Attribute | Value |
|-----------|-------|
| **Purpose** | The functional core (ledger half): stateless ledger rules for every Cardano era; the B1 top-level block-validity verdict; the B2 top-level transaction-validity verdict + mempool admission; the B3 full Conway value-conservation accounting; the B4 closed Conway cert-state accumulation; the B5 closed Conway governance-cert accumulation; the live committee-enactment write-back; the single BLUE chokepoint `mempool::ingress::mempool_ingress` from wire ingress into `admit`; the BLUE producer authority; the BLUE producer-side served-chain index and single canonical header-projection authority; the BLUE receive-side header→body bridge authority; the BLUE rollback authority; the BLUE canonical snapshot encoder/decoder authority. Plus intervening-cluster BLUE additions (seed-import / bootstrap-anchor / WAL / admission-orchestrator) not inventoried in this refresh. |
| **Creates** | Carry-forward 159 + intervening-cluster additions (165 at HEAD; net +6 since N-L). |
| **Interprets** | Carry-forward. |
| **MUST NOT** | All carry-forward. |
| **Inbound deps** | `ade_testkit`, `ade_core_interop`, `ade_runtime`, `ade_node`. |
| **Outbound deps** | `ade_types`, `ade_crypto`, `ade_codec`, `ade_plutus`, `ade_core`, `minicbor`, `num-bigint`, `num-integer`, `num-traits`. Dev-dep: `ade_testkit`. |
| **Entry points** | Carry-forward. |
| **Key modules** | All carry-forward. |

> **GREEN sub-classification (`ade_ledger::mempool::policy`, `ade_ledger::mempool::canonicalize`).** Carry-forward.
> **RED sub-classification (`ade_ledger::consensus_input_extract`).** Carry-forward.

---

### `ade_network` *(BLUE submodules)*

> **Status carried from prior CODEMAP — unchanged at HEAD `d6f3399` (PHASE4-N-P close).** PHASE4-N-P did not touch any BLUE submodule. The BLUE submodule set in `.idd-config.json` `core_paths` (9 paths) remains the canonical source of truth.

| Attribute | Value |
|-----------|-------|
| **Purpose** | Owns the Cardano Ouroboros mini-protocol authority — the closed wire grammar (CBOR codecs) and pure state machines for all 11 N2N + N2C mini-protocols, plus the BLUE Ouroboros mux frame primitive. Also owns the producer-side server-role reducer surface (PHASE4-N-G `chain_sync::server` + `block_fetch::server`). |
| **Creates** | Carry-forward (109 BLUE canonical types: 5 `mux/frame.rs` + 38 `codec/` + 9 `handshake/` + 11 `chain_sync/` + 10 `block_fetch/` + 5 `tx_submission/` + 5 `keep_alive/` + 5 `peer_sharing/` + 21 `n2c/`). |
| **Interprets** | Carry-forward. |
| **MUST NOT** | Carry-forward (1)–(14). |
| **Inbound deps** | Carry-forward. |
| **Outbound deps** | `ade_types`, `ade_codec`. No external deps in the BLUE submodules. |
| **Entry points** | Carry-forward. |
| **Key modules** | Carry-forward. |

---

### `ade_plutus`

> **Status carried from prior CODEMAP — unchanged at HEAD `d6f3399` (PHASE4-N-P close).** PHASE4-N-P did not touch `ade_plutus`.

| Attribute | Value |
|-----------|-------|
| **Purpose** | Quarantine boundary between the Ade-canonical ledger and the ported UPLC evaluator from `aiken-lang/aiken` (pinned to tag `v1.1.21`). |
| **Creates** | `PlutusScript`, `PlutusLanguage`, `EvalOutput`, `PlutusError`, `CostModels`, `DecoderMode`, `PerScriptResult`, `TxEvalResult`. |
| **Interprets** | UPLC scripts (Plutus V1/V2/V3) and `CostModels` CBOR. Phase-two transaction evaluation. `PlutusScript::from_cbor` is a named ingress chokepoint. |
| **MUST NOT** | (1) Re-export any `pallas_*` or `aiken_uplc::` type. (2) Allow another BLUE crate to bypass the canonical entry. (3) Activate PV11 builtins. (4) Use any BLUE-forbidden pattern. (5) Construct `PreservedCbor` outside `ade_codec`. |
| **Inbound deps** | `ade_ledger`, `ade_testkit`. |
| **Outbound deps** | `ade_types`, `ade_crypto`, `ade_codec`, `aiken_uplc` (git, tag `v1.1.21`), `pallas-primitives` (internal-only). |
| **Entry points** | `ade_plutus::eval_tx_phase_two`, `ade_plutus::tx_eval::*`, `ade_plutus::evaluator::*`, `ade_plutus::cost_model::*`. |
| **Key modules** | `evaluator.rs`, `cost_model.rs`, `script_context.rs`, `script_verdict.rs`, `tx_eval.rs`. |

---

### `ade_types`

> **Status carried from prior CODEMAP — unchanged at HEAD `d6f3399` (PHASE4-N-P close).** PHASE4-N-P did not touch `ade_types`.

| Attribute | Value |
|-----------|-------|
| **Purpose** | Owns the canonical Cardano domain vocabulary — primitives, era enum, address forms, era-tagged transaction bodies / outputs / certificates, governance types — used by every other workspace crate as the lingua franca. |
| **Creates** | `CardanoEra`, `SlotNo`, `BlockNo`, `EpochNo`, `Hash28`, `Hash32`, `Coin`, `Lovelace`, `NetworkId`, `Nonce`, `TxIn`, `RewardAccount`, `PoolId`, `Address`, `ByronAddress`, `Credential`, `StakeCredential`, `Certificate`, `PoolRegistrationCert`, `ConwayCert`, `CertDisposition` / `DepositEffect` / `CoinSource`, `MIRCert`, `MIRPot`, `DRep`, `GovAction`, `GovActionState`, `GovActionId`, `Anchor`, `ProposalProcedure`, `OperationalCert`, `NativeScript`, `PlutusV1Script`, `Datum`, `DatumOption`, `MultiAsset`, `AssetName`, `CostModel`, `ExUnits`, plus per-era tx-body / tx-out / witness wrappers. |
| **Interprets** | None — produce-only. |
| **MUST NOT** | (1) Construct or decode `PreservedCbor`. (2) Use any BLUE-forbidden pattern. (3) Depend on any workspace crate. (4) Add open/extensible variants to closed enums without a versioned gate. |
| **Inbound deps** | `ade_codec`, `ade_crypto`, `ade_ledger`, `ade_plutus`, `ade_runtime`, `ade_testkit`, `ade_network`, `ade_core`, `ade_core_interop`, `ade_node`. |
| **Outbound deps** | None. |
| **Entry points** | `ade_types::CardanoEra`, `ade_types::tx::{Coin, TxIn, RewardAccount}`, `ade_types::{Hash32, SlotNo, Hash28, BlockNo, EpochNo}`, `ade_types::conway::tx::ConwayTxBody`, `ade_types::conway::cert::*`, `ade_types::conway::governance::{ProposalProcedure, Anchor, GovAction, GovActionId}`, `ade_types::shelley::block::{OperationalCert, ProtocolVersion, ShelleyHeader, ShelleyBlock, VrfData}`. |
| **Key modules** | `primitives.rs`, `era.rs`, `tx.rs`, `address/`, `byron/`, `shelley/`, `allegra/`, `mary/`, `alonzo/`, `babbage/`, `conway/`. |

---

## GREEN Modules — Deterministic Glue

> Deterministic, non-authoritative. May depend on BLUE; must not affect authoritative outputs.

### `ade_testkit`

> **Status carried-forward (scoped-refresh note).** PHASE4-N-P did not add a sub-module to `ade_testkit`; the cluster's evidence surface is the inline tests in `ade_crypto::kes_sum::tests.rs` + the cardano-cli ground-truth corpus. Intervening clusters added GREEN test infrastructure under this crate not inventoried here (gap (yy)).

| Attribute | Value |
|-----------|-------|
| **Purpose** | Test infrastructure: differential harness, oracle snapshot loader, regression corpus, genesis loader, era mapping, transcript replay, diff reporting; N-B consensus harness; B1 block-validity harness; B2 transaction-validity harness; N-E S2 mempool-ingress replay harness; PP-S2 proposal_procedures replay harness; PHASE4-N-C producer test harness. |
| **Creates** | Carry-forward + intervening-cluster additions (gap (yy)). |
| **MUST NOT** | Carry-forward. |
| **Inbound deps** | None at compile time; consumption via integration tests and dev-dep links. |
| **Outbound deps** | `ade_types`, `ade_codec`, `ade_crypto`, `ade_core`, `ade_ledger`, `ade_plutus`, `ade_runtime`, `blake2`, `flate2`, `tar`, `serde`, `serde_json`, `toml`, `cardano-crypto` (dev-dep). |
| **Entry points** | Carry-forward. |
| **Key modules** | Carry-forward. |

---

### `ade_network::session` *(GREEN by content — PHASE4-N-L)*

> **Status carried from prior CODEMAP — unchanged at HEAD `d6f3399` (PHASE4-N-P close).** PHASE4-N-P did not touch this sub-tree.

| Attribute | Value |
|-----------|-------|
| **Purpose** | The pure session driver. Composes the BLUE authorities `mux::frame::{encode_frame, decode_frame}`, `handshake::n2n_transition`, and the per-mini-protocol state machines through `session::core::step`. |
| **Creates** | Carry-forward (12 closed types in `session::{event, state, demux, core, handshake_driver}`). |
| **MUST NOT** | Carry-forward (1)–(11). |
| **Inbound deps** | Carry-forward. |
| **Outbound deps** | Carry-forward. |
| **Entry points** | Carry-forward. |
| **Key modules** | Carry-forward. |

---

### `ade_runtime::bootstrap`, `ade_runtime::clock`, `ade_runtime::orchestrator::{mod, event, state, core}`, `ade_runtime::rollback::{persistent_writer, cadence, in_memory_cache, chaindb_block_source, persistent_cache}`, `ade_runtime::producer::{broadcast_to_served, served_chain_lookups}`, `ade_runtime::receive::{events_to_state, in_memory_chain_write}`

> **Status carried from prior CODEMAP — unchanged at HEAD `d6f3399` (PHASE4-N-P close).** PHASE4-N-P did not touch any of these GREEN sub-trees. Per-entry rows carry forward verbatim from the prior CODEMAP.

---

## RED Modules — Imperative Shell

> I/O, network, storage, clocks, retries. May depend on BLUE/GREEN. Must not modify core state directly or construct semantic types unsafely.

### `ade_core_interop`

> **Status carried-forward (scoped-refresh note).** PHASE4-N-P did not add a new live binary. Intervening clusters added evidence-replay binaries not inventoried here (gap (yy)).

| Attribute | Value |
|-----------|-------|
| **Purpose** | Live cardano-node interop driver. Hosts the `live_*_session` RED binaries plus N-E S4/S5 GREEN bridges. |
| **Creates** | Carry-forward. |
| **MUST NOT** | Carry-forward. |
| **Inbound deps** | None. |
| **Outbound deps** | `ade_core`, `ade_codec`, `ade_crypto`, `ade_ledger`, `ade_runtime`, `ade_network`, `ade_testkit`, `ade_types`, `tokio`. |
| **Entry points** | Carry-forward. |
| **Key modules** | Carry-forward. |

---

### `ade_network::mux::transport` *(RED)*

> **Status carried from prior CODEMAP — unchanged at HEAD `d6f3399` (PHASE4-N-P close).** PHASE4-N-P did not touch this file.

| Attribute | Value |
|-----------|-------|
| **Purpose** | The only place inside `ade_network` where socket I/O happens. RED tokio shell over the BLUE mux frame primitive and the GREEN session reducer. |
| **Creates** | Carry-forward. |
| **MUST NOT** | Carry-forward. |
| **Inbound deps** | Carry-forward. |
| **Outbound deps** | Carry-forward. |
| **Entry points** | Carry-forward. |
| **Key modules** | `mux/transport.rs`. |

---

### `ade_network` *(RED capture binaries — non-session)*

> **Status carried from prior CODEMAP — unchanged at HEAD `d6f3399` (PHASE4-N-P close).**

| Attribute | Value |
|-----------|-------|
| **Purpose** | The capture binaries (operator-action evidence harness). |
| **MUST NOT** | Carry-forward. |
| **Outbound deps** | Carry-forward. |

---

### `ade_node`

> **Status carried-forward (scoped-refresh note).** PHASE4-N-P did not modify `ade_node`. Intervening clusters added subcommand routing (e.g., `key-gen-KES` from N-O) and admission-orchestrator entry-point wiring not inventoried here (gap (yy)).

| Attribute | Value |
|-----------|-------|
| **Purpose** | Binary entry point for the node process. |
| **Creates** | Carry-forward + intervening-cluster additions (gap (yy)). |
| **MUST NOT** | Carry-forward. |
| **Inbound deps** | None (binary). |
| **Outbound deps** | `ade_types`, `ade_core`, `ade_ledger`, `ade_runtime`, `ade_network`, `ade_codec`, `tokio`. Dev-deps: `ade_testkit`, `tempfile`. |
| **Entry points** | `main()`. `ade_node::run_node_until_shutdown` is the library entry the integration tests drive in-process. |

---

### `ade_runtime`

> **Status change at HEAD `d6f3399` (PHASE4-N-P close).** PHASE4-N-P modifies two RED files in `ade_runtime::producer/`: `signing.rs` (migrates `KesSecret.inner` type + adds `pub(super) fn KesSecret::from_blue_signing_key`) and `keys.rs` (flips `load_kes_signing_key_skey` from fail-closed to accept-608-valid via the BLUE deserializer + adds `KeyLoadError::KesParse(KesParseError)` variant). Drops the `kes-sum` cardano-crypto feature in `Cargo.toml`. No new file. No new outbound workspace edge. **Intervening clusters added other RED files** (e.g., admission-orchestrator, evidence reducers under `runtime::evidence` / `runtime::admission`, `runtime::seed`, `runtime::wal`, Ade-native KES envelope writer) **not inventoried in this scoped refresh** (gap (yy)).

| Attribute | Value |
|-----------|-------|
| **Purpose** | Carry-forward (all earlier purposes) + intervening-cluster additions (gap (yy)). **NEW from PHASE4-N-P (RED migration, no new file):** `producer::signing.rs` now consumes the BLUE `ade_crypto::kes_sum::Sum6Kes` algorithm directly via `KesSecret.inner: <Sum6Kes as KesAlgorithm>::SigningKey`; `producer::keys.rs::load_kes_signing_key_skey` accepts 608-byte valid payloads via `Sum6Kes::raw_deserialize_signing_key_kes` + `Sum6Kes::current_period_of_signing_key` + `KesSecret::from_blue_signing_key`. RED-confined private-key custody discipline (no public byte accessors, redacted `Debug`, hand-rolled `Drop` zeroize delegated to the BLUE `ZeroizingSeed` per-field guarantee) carries forward unchanged. |
| **Creates** | All carry-forward + intervening additions. **NEW — PHASE4-N-P (RED):** `KeyLoadError::KesParse(ade_crypto::kes_sum::KesParseError)` enum variant (additive; closed enum re-closed); `KesSecret::from_blue_signing_key` `pub(super)` constructor (no new public type). No new BLUE canonical type counted here. |
| **Interprets** | Carry-forward + intervening additions. **NEW from PHASE4-N-P:** `producer::keys.rs::load_kes_signing_key_skey` interprets the canonical 608-byte cardano-cli `KesSigningKey_ed25519_kes_2^6` payload via the BLUE deserializer — every byte position is structurally validated by `Sum6Kes::raw_deserialize_signing_key_kes` (vk-consistency recursive check, leaf-non-zero, payload-size pinned to 608); any structural defect maps to a closed `KeyLoadError::KesParse(KesParseError::*)` variant. The current period is inferred from the tree shape (which sub-seeds are zero) by `Sum6Kes::current_period_of_signing_key`. |
| **MUST NOT** | (1)–(carry-forward) carry-forward + intervening rules. **NEW — PHASE4-N-P (`producer::signing.rs` + `keys.rs`, DC-CRYPTO-08 + DC-CRYPTO-07 strengthening):** MUST consume `ade_crypto::kes_sum::Sum6Kes` (not `cardano_crypto::kes::Sum6Kes`) for every production `kes_sign` / `kes_update` / `derive_verification_key` / `gen_key_kes_from_seed_bytes` / `raw_deserialize_signing_key_kes` call. MUST NOT include `kes-sum` in the `cardano-crypto` feature list in `ade_runtime/Cargo.toml` (gated by `ci/ci_check_kes_sum_compatibility.sh` Guard inspecting `Cargo.toml`). MUST NOT call `KesSecret::from_bytes_zeroizing` or `KesSecret::from_seed_at_period` inside `load_kes_signing_key_skey`'s body — only `KesSecret::from_blue_signing_key` (via the BLUE deserializer) is permitted, because the fresh-from-seed constructors would silently re-derive the tree from a 32-byte prefix of the 608-byte payload (gated by `ci/ci_check_kes_envelope_closed.sh` Guard 2 narrowed for PHASE4-N-P S5). |
| **Inbound deps** | Carry-forward. |
| **Outbound deps** | Carry-forward (`tokio` features unchanged). **NEW from PHASE4-N-P:** `cardano-crypto` features narrow from `["vrf-draft03", "kes-sum", "dsign"]` to `["vrf-draft03", "dsign"]` — `kes-sum` is dropped. No new workspace edge — the migration is a within-crate consumer flip from `cardano_crypto::kes::Sum6Kes` to `ade_crypto::kes_sum::Sum6Kes` (`ade_runtime → ade_crypto` was already an edge). |
| **Entry points** | All carry-forward. **NEW — PHASE4-N-P:** `KesSecret::from_blue_signing_key` (`pub(super)`, consumed by `keys.rs` only). |
| **Key modules** | All prior keys + intervening-cluster modules (gap (yy)). |
| **Mechanical enforcement** | Carry-forward. **NEW — one PHASE4-N-P CI script backs the new BLUE algorithm + RED migration surface:** `ci_check_kes_sum_compatibility.sh` (S4; DC-CRYPTO-08 + DC-CRYPTO-09 — 4 guards as described in the BLUE shared-header section). **One existing script extended:** `ci_check_kes_envelope_closed.sh` Guard 2 narrowed to require `raw_deserialize_signing_key_kes` in the loader body and forbid `KesSecret::from_bytes_zeroizing` / `from_seed_at_period` calls there. |

> **Gap surfaced (carried + narrowed).** PHASE4-N-P closes the cardano-cli expanded-skey-import open obligation on `OP-OPS-04` and the `open_obligation` on `DC-CRYPTO-07`. The honest-scope gap that carries forward from prior clusters is `RO-LIVE-*` (operator-action live evidence against a peer with stake) and gap (yy) — the intermediate-cluster CODEMAP per-module inventory.

---

## Cross-Module Rules (project-wide)

### Dependency direction

`ade_core_interop` → `{ade_core, ade_codec, ade_crypto, ade_ledger, ade_runtime, ade_network, ade_testkit, ade_types, tokio}` is legal (RED leaf binary). **No new crate-level outbound edge in PHASE4-N-P.**
`ade_runtime` → `{ade_core, ade_crypto, ade_codec, ade_types, ade_ledger, ade_network, redb, serde_json, cardano-crypto, ed25519-dalek, tokio}` is legal (RED → BLUE). **PHASE4-N-P unchanged outbound — only the `cardano-crypto` feature set narrows from `["vrf-draft03", "kes-sum", "dsign"]` to `["vrf-draft03", "dsign"]`.**
`ade_node` → `{ade_types, ade_core, ade_ledger, ade_runtime, ade_network, ade_codec, tokio}` is legal (RED → BLUE/GREEN). **No change in N-P.**
`ade_testkit` → `{ade_core, ade_ledger, ade_plutus, ade_runtime, ade_crypto, ade_codec, ade_types, cardano-crypto}` is legal (GREEN). **No change in N-P.**
`ade_network` (BLUE submodules) → `{ade_codec, ade_types}` is legal.
`ade_network` (GREEN-by-content `session/` submodule) → carry-forward.
`ade_network` (RED submodules + capture bins) → carry-forward.
`ade_ledger` → `{ade_core, ade_plutus, ade_crypto, ade_codec, ade_types, minicbor}` is legal (BLUE among BLUEs).
`ade_core` → `{ade_types, ade_crypto, minicbor}` is legal (BLUE among BLUEs).
`ade_plutus` → `{ade_crypto, ade_codec, ade_types}` is legal.
`ade_crypto` → `{ade_types, blake2, ed25519-dalek, cardano-crypto (features = ["vrf-draft03", "kes-sum", "dsign"], default-features = false)}` is legal. **NEW — PHASE4-N-P:** `cardano-crypto` consumption is `#[cfg(test)]`-only for the KES surface within `crates/ade_crypto/src/**`; production-path imports of `cardano_crypto::kes::*` are a CI failure (`ci_check_kes_sum_compatibility.sh` Guard 3). VRF + DSIGN paths under `crates/ade_crypto/src/vrf.rs` continue to import `cardano_crypto::vrf::VrfDraft03` in production unchanged.
`ade_codec` → `{ade_types}` is legal.
`ade_types` → `{}`.

**Forbidden directions.** Any BLUE crate (or BLUE `ade_network` submodule) depending on `ade_runtime`, `ade_node`, `ade_core_interop`, or the RED half of `ade_network` is a CI failure. Any non-`ade_plutus` crate referring to `pallas_*` is a CI failure. All prior dependency notes carry forward. **NEW — PHASE4-N-P dependency notes:** any `cardano_crypto::kes` import outside `#[cfg(test)]` under `crates/ade_crypto/src/**` is a CI failure (`ci_check_kes_sum_compatibility.sh` Guard 3, KES-scoped — VRF + DSIGN imports remain permitted). Any `expand_seed` prefix-byte literal other than `0x01` / `0x02` in `crates/ade_crypto/src/kes_sum/hash.rs` is a CI failure (`ci_check_kes_sum_compatibility.sh` Guard 4). Any `.skey` envelope file committed under `crates/ade_crypto/` is a CI failure (`ci_check_kes_sum_compatibility.sh` Guard 2). Any `KesSecret::from_bytes_zeroizing` or `KesSecret::from_seed_at_period` call inside `load_kes_signing_key_skey`'s body is a CI failure (`ci_check_kes_envelope_closed.sh` Guard 2 narrowed).

### Naming convention

All crates are prefixed `ade_`. TCB color is not encoded in the crate name. The authoritative classifier is `.idd-config.json` `core_paths` plus the cluster doc TCB Color Maps for sub-crate scopes; CI scripts hard-code their BLUE list. **NEW — the PHASE4-N-P script scopes `ade_crypto/src/kes_sum/` and `ade_runtime/{Cargo.toml, src/producer/keys.rs, src/producer/signing.rs}` — no broader BLUE-scope re-targeting.**

### CI enforcement (89 scripts under `ci/`)

| Script | Enforces | Scope |
|---|---|---|
| (carried-forward list of 65 scripts as inventoried in the prior CODEMAP at HEAD `d62c2bc` + PHASE4-N-L worktree) | (carry-forward) | (carry-forward) |
| **`ci_check_kes_sum_compatibility.sh`** *(NEW — PHASE4-N-P S4)* | **DC-CRYPTO-08 + DC-CRYPTO-09 — 4 guards: (1) cardano-cli corpus exists + throwaway-comment-prefixed + ≥ 3 fixtures; (2) no `.skey` files under `crates/ade_crypto/`; (3) `cardano_crypto::kes` only under `#[cfg(test)]` in `crates/ade_crypto/src/**` (KES-scoped — VRF/DSIGN permitted); (4) `expand_seed` prefix bytes = `0x01` / `0x02` (Haskell), not `0x00` / `0x01` (cardano-crypto Rust 1.0.8)** | `crates/ade_crypto/src/kes_sum/` + `crates/ade_crypto/src/**` + `crates/ade_crypto/` (find for `.skey` files) |
| **`ci_check_kes_envelope_closed.sh`** *(EXTENDED — PHASE4-N-P S5)* | **DC-CRYPTO-07 strengthened: Guard 2 narrows — loader body must contain `raw_deserialize_signing_key_kes` call and must NOT contain `KesSecret::from_bytes_zeroizing` / `from_seed_at_period` calls; `UnsupportedExpandedKesKeyFormat` retained for size-mismatch branch** | `crates/ade_runtime/src/producer/keys.rs` + `crates/ade_runtime/src/producer/ade_kes_envelope.rs` + `crates/ade_node/src/key_gen.rs` |
| *(plus +22 intervening-cluster scripts — names enumerated by `ls ci/ci_check_*.sh` at HEAD; per-script Enforces/Scope rows not in this scoped refresh, gap (yy))* | | |

**Total: 89 scripts.** Net delta vs prior CODEMAP HEAD: +23 (+1 from PHASE4-N-P; +22 from intervening clusters not inventoried here). The full inventory is mechanically obtainable via `ls ci/ci_check_*.sh | wc -l`.

> **Post-`d62c2bc` CI delta (PHASE4-N-P scope).** +1 new (`ci_check_kes_sum_compatibility.sh`); +1 extended (`ci_check_kes_envelope_closed.sh` Guard 2). **CI inventory 66 → 89 at HEAD** (the +22 are intervening clusters per gap (yy)).

> **Carried residual gaps (unchanged at HEAD, plus the N-P narrowings).** **(rr)–(xx)** carry-forward (see prior CODEMAP). **(yy) NEW — intermediate-cluster CODEMAP inventory pending.** This refresh is scoped to PHASE4-N-P only; nine intermediate clusters (PHASE4-N-L-LIVE, PHASE4-N-M-A, PHASE4-N-M-A1.1, PHASE4-N-M-B, PHASE4-N-M-C, PHASE4-N-M-FRAG, PHASE4-N-M-SCHED, PHASE4-N-M-FOLLOW, PHASE4-N-O) closed between the prior CODEMAP HEAD `d62c2bc` and HEAD `d6f3399` without a per-cluster CODEMAP regeneration. Their structural deltas are partially evident in the canonical-type / test / CI-script count deltas reported above, but per-module Purpose / Creates / Interprets / MUST NOT / deps / entry-point rows below for `ade_ledger`, `ade_runtime`, `ade_node`, `ade_core_interop` are carry-forward-from-N-L for the non-KES surface. The closure records at `docs/clusters/completed/{PHASE4-N-L-LIVE, PHASE4-N-M-A, PHASE4-N-M-A1.1, PHASE4-N-M-B, PHASE4-N-M-C, PHASE4-N-M-FRAG, PHASE4-N-M-SCHED, PHASE4-N-M-FOLLOW, PHASE4-N-O, PHASE4-N-P}/CLOSURE.md` are the canonical sources to consult when regenerating the intervening per-module rows. **(zz) NEW — PHASE4-N-P narrows `OP-OPS-04`** — both Ade-native and cardano-cli expanded KES key flows are now supported in `load_kes_signing_key_skey` / `load_ade_kes_signing_key`; the `open_obligation` on `OP-OPS-04` and `DC-CRYPTO-07` is cleared. The remaining honest-scope gap is the operator-action live producer pass against a peer with testnet stake (still `CN-CONS-06.open_obligation = "blocked_until_operator_stake_available"`).
