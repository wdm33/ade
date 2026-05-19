# Seams — Where New Work Can Attach (Ade)

> **Status:** Living architectural document. Regenerated; not hand-edited.
> Per-project instance of `~/.claude/methodology/templates/seams.md`.

> 10 crates, 182 canonical types, 22 CI checks at HEAD (`56bfa7b`).
> Reads CODEMAP for the module list and TCB colors; reads the invariant
> registry (`docs/ade-invariant-registry.toml`) for rule IDs; reads the
> Phase 4 cluster plan (`docs/active/phase_4_cluster_plan.md`), the
> closed N-D slice docs, and the closed N-A cluster doc + planning trio
> (`docs/clusters/completed/PHASE4-N-A/cluster.md`,
> `docs/active/PHASE4-N-A_invariants.md`,
> `docs/active/PHASE4-N-A_cluster_plan.md`).

Ade is a Cardano block-producing node. Its closure surface is dominated
by two facts:

1. The Cardano protocol fixes wire bytes and hashes for hash-critical
   paths (Tier 1 — must-conform). New work that touches those bytes
   has essentially no degrees of freedom.
2. Everything operator-facing — storage layout, query API, telemetry,
   packaging — is Tier 5: deliberate divergence "in our own image"
   (per `docs/active/CE-79_tier5_addendum.md`).

This document names where the system opens and where it stays closed.

---

## 1. Surface Reduction Rules

> External inputs reduce to canonical form before entering authoritative
> pipelines. At HEAD there are four fully-wired ingress surfaces (block
> bytes, Plutus script bytes, snapshot bytes, and Ouroboros mux frames),
> plus three further surfaces named in the Phase 4 plan (forge, mempool,
> query API).

### Surface: Block bytes (wired today)

```
Surface: Block bytes (file/stream/network — caller-supplied)
Reduces to: BlockEnvelope { era: CardanoEra, era_block: PreservedCbor<EraBlock> }
            (BlockEnvelope is defined in `ade_codec::cbor::envelope`;
             EraBlock is one of the seven era-tagged decoded blocks)
Pipeline (fixed step ordering — no reorder, no shortcut):
  1. decode_block_envelope(&[u8]) -> BlockEnvelope
     (era tag dispatch; the only constructor of PreservedCbor for blocks)
  2. era-specific decode_{byron_ebb,byron_regular,shelley,allegra,
     mary,alonzo,babbage,conway}_block
     (closed set — 8 era-block decoders, named in
     `ci_check_ingress_chokepoints.sh`)
  3. ade_ledger::rules::apply_block_with_verdicts(state, &PreservedCbor<EraBlock>, ctx)
     (BLUE — single canonical chokepoint that produces BlockVerdict + new state)
Cross-surface state sharing: none today (Phase 3 was an offline oracle).
  Phase 4 introduces shared state between this surface and the network
  ingress surface (mux frames, below) via `ade_runtime::chaindb`
  (persistence) and a forthcoming `ade_node`-level composition layer.
```

**Rule.** New ingress that produces block bytes (e.g., the N-A `block-fetch`
mini-protocol delivering block bodies, N-D recovery replay, N-F
`local-tx-monitor`) **MUST** enter through `decode_block_envelope` and
flow through one of the era-specific block decoders before reaching any
ledger code. The pipeline cannot be reordered: hash-bearing bytes must
be preserved via `PreservedCbor` before they reach ledger rules
(enforced by `ci_check_hash_uses_wire_bytes.sh`,
`ci_check_ingress_chokepoints.sh`). **`ade_network` is forbidden from
decoding block CBOR** — its codec layer treats block / header / tx
bodies as opaque `Vec<u8>`, and dispatch into `ade_codec` happens at
the session / `ade_node` boundary (S-A9 / future N-B).

### Surface: Plutus script bytes (wired today)

```
Surface: Plutus script bytes (CBOR-wrapped Flat, extracted from witness sets)
Reduces to: PlutusScript { inner: aiken_uplc::ast::Program<DeBruijn> }
            (defined in `ade_plutus::evaluator`; aiken types do not
             leak past this boundary)
Pipeline:
  1. ade_plutus::evaluator::PlutusScript::from_cbor(&[u8]) -> Result<PlutusScript, PlutusError>
     (named ingress chokepoint — the only public path that turns Plutus
     script CBOR into a runnable program; uses the aiken/pallas decoder,
     not the ade_codec primitives)
  2. ade_plutus::tx_eval::eval_tx_phase_two(...) -> TxEvalResult
     (BLUE — single canonical phase-2 evaluation entry; aiken `uplc`
     machine is invoked internally and aiken types do not escape)
Cross-surface state sharing: none — phase-2 evaluation is pure
  fn(script, ScriptContext, CostModels, ExUnits) -> EvalOutput.
```

**Rule.** Plutus script CBOR is a **distinct ingress surface** from
block CBOR. It does not go through `decode_block_envelope` because its
wire format is CBOR-wrapped Flat decoded by `aiken_uplc`, not by the
project's own `ade_codec` primitives. The chokepoint is
`PlutusScript::from_cbor` in `ade_plutus/src/evaluator.rs`, named
explicitly in the header comment of `ci_check_ingress_chokepoints.sh`
and allowlisted from Check 3 of that script (Check 3 forbids
`from_cbor`/`minicbor::decode`/`cbor_decode` everywhere in BLUE except
in `ade_plutus/src/evaluator.rs`). All other BLUE crates remain
forbidden from decoding raw CBOR.

### Surface: Snapshot bytes (wired in N-D)

```
Surface: Snapshot bytes (disk — written and read by the node itself)
Reduces to: Recoverable::decode_snapshot(&[u8]) -> R  (caller-supplied)
Pipeline:
  1. SnapshotStore::latest_snapshot() -> Option<(SlotNo, Vec<u8>)>
  2. Recoverable::decode_snapshot(bytes) -> R       (caller's impl)
  3. for block in ChainDb::iter_from_slot(slot+1):
       R::apply_block(&block.bytes) -> R            (caller's impl)
Cross-surface state sharing: `ade_runtime` is intentionally bytes-in /
  bytes-out — it never touches the ledger state type directly. The
  shared state lives at the caller (eventually `ade_node`).
```

**Rule.** The recovery primitive (`ade_runtime::recovery::recover`) is
the **single** path from on-disk state to in-memory state. It does not
import `ade_ledger`. Any callsite that wants to recover a ledger state
must provide a `Recoverable` impl; there is no second public path
through `ade_runtime`.

### Surface: Ouroboros mux frames (wired in N-A)

```
Surface: Raw bytes off a TCP / Unix-socket bearer (cardano-node peer)
Reduces to: per-protocol message enums in `ade_network::codec::*`
            (BlockFetchMessage, ChainSyncMessage, HandshakeMessage,
             KeepAliveMessage, PeerSharingMessage, TxSubmission2Message,
             LocalChainSyncMessage, LocalStateQueryMessage,
             LocalTxMonitorMessage, LocalTxSubmissionMessage,
             N2cHandshakeMessage — 11 closed enums, one per protocol)
Pipeline (fixed step ordering — no reorder, no shortcut):
  1. ade_network::mux::transport::MuxTransport::read_raw (RED, async)
     — moves bytes off the bearer; no parsing.
  2. ade_network::mux::frame::decode_frame(&[u8])
       -> Result<(MuxFrame, &[u8]), MuxError>     (BLUE, sync, pure)
     — the **single** chokepoint that turns bytes into a typed
       (timestamp, mode, mini_protocol_id, payload) frame. Mirror
       symbol `encode_frame` is the **single** outbound chokepoint.
  3. ade_network::codec::<protocol>::decode_<protocol>_message(payload)
       -> Result<<Protocol>Message, CodecError>   (BLUE, sync, pure)
     — closed wire grammar per protocol; one decoder per closed enum
       above. Mirror symbol `encode_<protocol>_message` for outbound.
  4. ade_network::<protocol>::transition::<protocol>_transition(
         state, agency, version, msg)
       -> Result<(new_state, output), error>     (BLUE, sync, pure)
     — 8 closed transition functions (chain-sync, block-fetch,
       handshake [n2n + n2c arms share the module], keep-alive,
       peer-sharing, tx-submission2, plus 4 N2C state machines under
       `ade_network::n2c::local_*`). Selected protocol version is an
       explicit input (DC-PROTO-06); never read from a session global.
  5. Session composition (RED, S-A9 placeholder at HEAD;
     `ade_network::session::mod`) routes outputs into shell I/O and
     fans block / tx / query bytes to the appropriate authoritative
     pipeline above. **No N-A code calls `ade_ledger` or `ade_codec`
     block decoders directly** — that bridge lands in N-B.
Cross-surface state sharing: protocol version table
  (`ade_network::codec::version`) is shared across handshake +
  transition + codec call sites. No other shared state.
```

**Rule.** Mux frames are a **fourth distinct ingress surface**, layered
above the byte bearer and below all higher protocol decoding. The two
chokepoints `mux::frame::{encode_frame, decode_frame}` are the only
byte↔frame translation in the project; `ade_network::mux::transport`
(RED) calls them and nothing else does. **Each mini-protocol's codec
and transition function form a self-contained, structurally
independent closed semantic surface (IDD §6).** Adding a new
mini-protocol is *not* an extension of an existing one — it is a new
closed `*Message` enum + a new `encode_*_message` / `decode_*_message`
pair + a new `*_transition` function + a new `*Version` enum in
`ade_network::codec::version`. There is no `Codec<P>` trait, no
`Box<dyn Protocol>`, no `#[non_exhaustive]`, no runtime negotiation of
message meaning. Versioning happens through closed `*Version` enums
that gate which variants are legal at protocol-step time; mismatches
surface as `InvalidForVersion` at the protocol boundary rather than as
a silent fallback.

### Candidates — surfaces not yet wired (Phase 4 N-B, N-C, N-E, N-F)

The following surfaces are named in the Phase 4 cluster plan but have
no source today. They are listed so future slice docs can attach
without reinventing the reduction step. **Each is a candidate seam
pending confirmation at cluster entry.**

| Cluster | Surface | Expected reduction target | Expected chokepoint | Confidence |
|---------|---------|---------------------------|---------------------|------------|
| N-B | Bridge from `ade_network` block-fetch output to ledger apply | `BlockEnvelope` then ledger apply (block-bytes surface above) | Composition layer in `ade_node` (proposed) — calls `ade_codec::decode_block_envelope` on `BlockFetchMessage::Block` payload | candidate |
| N-C | Forge-block inputs (mempool + state + slot + KES + VRF) | `BlockEnvelope` bytes (forged, then re-decoded for validation) | `ade_runtime::forge::forge_block` (proposed) | candidate |
| N-E | Mempool tx ingest (from N-A tx-submission2 OR N-A local-tx-submission) | Per-era tx body (canonical bytes preserved) | `ade_runtime::mempool::ingest_tx` (proposed) | candidate |
| N-F | LSQ semantic dispatch (LocalStateQuery payloads) | Internal Query enum (closed, not yet defined) | Single dispatch fn that consumes `LocalStateQueryMessage::Acquire/Query/Result` opaque-bytes payloads — Tier 5 wire on operator-facing gRPC/HTTP, Tier 1 semantics shared with LSQ | candidate |
| N-F | LocalTxMonitor semantic dispatch | Mempool-snapshot Query/Reply enums | Single dispatch fn that consumes `LocalTxMonitorMessage` opaque-bytes payloads | candidate |

These candidates need user confirmation when each cluster is opened:
"Is the canonical reduction target named above the right one? Does the
chokepoint name fit the project's emerging naming convention?"

---

## 2. Data-Only vs. Authoritative Layers

Ade has four authoritative domains. For each, a single BLUE chokepoint
holds enforcement authority; tooling layers (when they exist) live in
GREEN (`ade_testkit`) or RED (`ade_runtime`, `ade_network::mux::transport`,
`ade_network::session`).

### Ledger application

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **Data-only tooling** | `ade_codec` | BLUE\* | Decodes block / tx / cert bytes into typed values, preserves wire bytes via `PreservedCbor`. **Never interprets ledger semantics.** |
| **Authoritative enforcement** | `ade_ledger` | BLUE | `rules::apply_block_with_verdicts` is the single chokepoint that produces `BlockVerdict` + new `LedgerState`. |
| **Loader** | `ade_runtime::chaindb` + `ade_runtime::recovery` | RED | Reads block / snapshot bytes from disk; feeds them through caller-supplied `Recoverable` impl into ledger. |

\* `ade_codec` is BLUE-data-only: it builds typed shapes but never
asserts a transition is valid. The semantic split between "this is
what the bytes say" (codec) and "this is whether the bytes are
allowed" (ledger) is the project's central design rib.

**Rule.** New work that touches ledger transitions adds enforcement
inside `ade_ledger` (typically a new composer step, or a tightening of
`apply_block_with_verdicts` / `apply_epoch_boundary_full`). New work
that touches block / tx CBOR adds parse / pack support inside
`ade_codec` only. **The compilation chokepoint
(`apply_block_with_verdicts`) never moves.**

### Plutus phase-2 evaluation

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **Data-only tooling** | `ade_plutus::cost_model`, `ade_plutus::script_context` | BLUE | Decodes cost-model CBOR; builds the V1/V2/V3 `ScriptContext`. Does not run programs. |
| **Script ingress** | `ade_plutus::evaluator::PlutusScript::from_cbor` | BLUE | Named ingress chokepoint for Plutus script CBOR. Allowlisted in `ci_check_ingress_chokepoints.sh` Check 3 because the decoder is `aiken_uplc`/`pallas`, not `ade_codec`. |
| **Authoritative enforcement** | `ade_plutus::tx_eval::eval_tx_phase_two` | BLUE | Single entry to phase-two evaluation. Internally wraps the aiken `uplc` machine; aiken types do not leak (enforced by `ci_check_pallas_quarantine.sh`). |
| **Quarantine** | (the `aiken_uplc` git dep, pinned tag `v1.1.21` commit `42babe5d`) | external | Frozen at tag — never re-exported. PV11 builtins gated off (S-29). |

**Rule.** Adding a new Plutus version, builtin, or cost-model entry
requires a registry diff (see §3) plus a pinned-version bump of
`aiken_uplc`; the chokepoint `eval_tx_phase_two` does not move. No
second public entry into the evaluator is allowed; tests use the same
entry as production callers. **No new BLUE callsite of
`PlutusScript::from_cbor` may be added outside `ade_plutus` itself** —
the chokepoint exists to keep aiken-decoded bytes inside the
quarantine.

### Governance ratification / enactment (Conway)

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **Data-only tooling** | `ade_types::conway` (governance types) | BLUE | Holds `GovAction`, `GovActionState`, `DRep`, `Anchor`, `VotingProcedures` shapes. |
| **Authoritative enforcement** | `ade_ledger::governance::{evaluate_ratification, enact_proposals, expire_proposals}` | BLUE | The three chokepoints that compute Conway ratification outcomes. |

**Rule.** A new governance action variant (CIP-1694 extension) adds a
variant to `GovAction` (§3 closed registry — version-gated) **and**
arms in all three chokepoints. The CI check
`ci_check_constitution_coverage.sh` enforces the invariant-registry ↔
code coverage for governance rules.

### Mini-protocol wire conformance (N-A)

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **Data-only tooling (frame)** | `ade_network::mux::frame` | BLUE | Pure encode/decode over the fixed 8-byte Ouroboros mux header + opaque payload. No I/O, no async, no time. `encode_frame` / `decode_frame` are the only byte↔frame chokepoints. |
| **Data-only tooling (messages)** | `ade_network::codec::{block_fetch, chain_sync, handshake, keep_alive, local_chain_sync, local_state_query, local_tx_monitor, local_tx_submission, n2c_handshake, peer_sharing, tx_submission}` | BLUE | 11 closed wire grammars, one per mini-protocol. Each exposes `encode_<protocol>_message` + `decode_<protocol>_message`. Payloads of higher-layer surfaces (block CBOR, tx CBOR, LSQ queries, mempool queries) remain `Vec<u8>` here — interpretation lives elsewhere. |
| **Authoritative enforcement (state)** | `ade_network::{block_fetch, chain_sync, handshake, keep_alive, peer_sharing, tx_submission}::transition` and `ade_network::n2c::local_*::transition` | BLUE | 8 closed pure transition functions. Shape: `fn (state, agency, version, msg) -> Result<(new_state, output), error>`. Closed state graphs; illegal tuples produce `IllegalTransition`. |
| **Bearer (I/O)** | `ade_network::mux::transport` | RED | Tokio-based TCP / Unix-socket scaffold. Async lives **here and only here** within `ade_network`; sync-only discipline in BLUE submodules is enforced by `ci_check_no_async_in_blue.sh` (DC-CORE-01). |
| **Session composition (placeholder)** | `ade_network::session::mod` | RED | S-A9 placeholder. Will drive the mux + state machines together; no protocol logic. |
| **Live-interop capture tools** | `ade_network::bin::capture_*` (7 RED binaries: handshake, chain_sync, block_fetch, tx_submission2, keep_alive, peer_sharing, n2c_handshake, n2c_protocols) | RED | Operator/dev tools for live cardano-node 11.0.1 capture. Never linked into the node binary. |

**Rule.** Three rules carry the cluster:

1. **The codec layer is opaque to higher semantics.** `ade_network`
   never decodes block CBOR or tx CBOR — those payloads are `Vec<u8>`
   carried through `*Message` variants. The bridge into `ade_codec` /
   `ade_ledger` lives at the session/`ade_node` composition layer
   (currently a placeholder, to be filled by S-A9 + N-B).
2. **The two chokepoints `mux::frame::{encode_frame, decode_frame}`
   never move.** Any future wire-framing change is a coordinated
   rewrite of both, not a duplicate path.
3. **The selected protocol version is an explicit transition input
   (DC-PROTO-06).** No state machine reads ambient session state.
   Mismatches surface as `InvalidForVersion`.

### Where the boundary is enforced

- `ci_check_dependency_boundary.sh` — no BLUE crate may depend on
  `ade_runtime` or `ade_node`; `ade_network` BLUE submodules may not
  depend on RED submodules within the same crate.
- `ci_check_no_async_in_blue.sh` — async / tokio / futures forbidden
  in `ade_network` BLUE submodules (DC-CORE-01).
- `ci_check_pallas_quarantine.sh` — only `ade_plutus` may name
  `pallas_*`.
- `ci_check_no_signing_in_blue.sh` — signing patterns
  (`SigningKey`/`sign_message`/etc.) forbidden in BLUE; only
  `ade_runtime` may sign.
- `ci_check_ingress_chokepoints.sh` — three checks:
  (1) `PreservedCbor::new` constructed only inside `ade_codec`;
  (2) `decode_block_envelope` exists as a named function in
      `ade_codec::cbor::envelope`;
  (3) raw CBOR decoding (`from_cbor`, `minicbor::decode`, `cbor_decode`)
      is forbidden in every BLUE crate except `ade_codec` itself
      **and** `ade_plutus/src/evaluator.rs` (allowlisted — see §1
      Plutus surface).
- `ci_check_ce_n_a_5_proof.sh` — N-A live-interop evidence: ensures
  `docs/active/CE-N-A-5_evidence.toml` is present, well-formed, and
  references a captured log from a real cardano-node 11.0.1 session.

---

## 3. Closed vs. Extensible Registries

Ade's authority surface is **almost entirely closed.** This is a
consequence of being a chain-compatibility implementation: the
protocol fixes most variants. The few extensible surfaces are
operator-config or testkit-only.

### Closed (frozen — version-gated changes only)

| Registry | Location | Count | Change Rule |
|----------|----------|-------|-------------|
| `CardanoEra` | `ade_types::era` | 8 variants (ByronEbb, ByronRegular, Shelley, Allegra, Mary, Alonzo, Babbage, Conway) | New variant = new hard fork. Requires a coordinated change across `ade_codec` (new era's `decode_*_block` chokepoint), `ade_ledger` (new era composer + hfc translation), and the canonical type list. Comment in source explicitly says "this enum is closed — unknown era tags produce a `CodecError`, never a fallback variant." |
| `Certificate` | `ade_types::shelley::cert` | 7 variants (StakeRegistration, StakeDeregistration, StakeDelegation, PoolRegistration, PoolRetirement, GenesisKeyDelegation, MoveInstantaneousRewards) | Frozen Shelley-era certificate set. New cert types live in `ConwayCert`. |
| `ConwayCert` | `ade_types::conway::cert` | N variants (Conway-era certificates) | Version-gated per protocol — extends but does not modify `Certificate`. |
| `GovAction` | `ade_types::conway::governance` | 7 variants (ParameterChange, HardForkInitiation, TreasuryWithdrawals, NoConfidence, UpdateCommittee, NewConstitution, InfoAction) | CIP-1694 fixed; new variant = CIP amendment + ratification chokepoint update. |
| `MIRPot` | `ade_types::shelley::cert` | 2 variants (Reserves, Treasury) | Frozen. |
| `DRep` | `ade_types::conway::cert` | 4 variants (KeyHash, ScriptHash, AlwaysAbstain, AlwaysNoConfidence) | CIP-1694 fixed. |
| `PlutusLanguage` | `ade_plutus::evaluator` | 3 variants (V1, V2, V3) | New variant = new Plutus version. Requires cost-model table extension + aiken bump. PV11 builtins gated off (S-29). |
| `Datum` / `DatumOption` | `ade_types::alonzo::plutus`, `ade_types::babbage::output` | Closed shapes — Datum hash vs. inline | Schema frozen at Babbage. |
| `NativeScript` | `ade_types::allegra::script` | Shelley/Allegra/Mary native script variants | Frozen. |
| **Named ingress chokepoints (block CBOR)** | `ade_codec::{cbor::envelope, byron, shelley, allegra, mary, alonzo, babbage, conway, address}` | 10 — `decode_block_envelope`, `decode_byron_ebb_block`, `decode_byron_regular_block`, `decode_shelley_block`, `decode_allegra_block`, `decode_mary_block`, `decode_alonzo_block`, `decode_babbage_block`, `decode_conway_block`, `decode_address` | Header comment of `ci_check_ingress_chokepoints.sh` enumerates this set. New era = new chokepoint added in lockstep with a `CardanoEra` variant. Removal forbidden. |
| **Named ingress chokepoint (Plutus script CBOR)** | `ade_plutus::evaluator::PlutusScript::from_cbor` | 1 — file `crates/ade_plutus/src/evaluator.rs` | Distinct from the block-CBOR chokepoints. Allowlisted by exact file path in Check 3 of `ci_check_ingress_chokepoints.sh`. |
| **`PreservedCbor::new` constructor** | `ade_codec::preserved` | 1 chokepoint, `pub(crate)` | Construction lives inside `ade_codec`. |
| **Mini-protocol message enums** | `ade_network::codec::{block_fetch, chain_sync, handshake, keep_alive, local_chain_sync, local_state_query, local_tx_monitor, local_tx_submission, n2c_handshake, peer_sharing, tx_submission}` | 11 closed enums | Closed wire grammar per protocol. **No `#[non_exhaustive]`, no `dyn` dispatch, no generic `Codec<P>` trait.** A new mini-protocol = new module + new closed enum + new `encode_*_message`/`decode_*_message` pair + new `*Version` enum + new transition function. Never an arm on an existing enum. |
| **Mini-protocol encode/decode chokepoints** | `ade_network::codec::*::{encode_<protocol>_message, decode_<protocol>_message}` | 22 functions (11 protocols × 2 directions) | Single chokepoint per direction per protocol. Removal or renaming forbidden; symbol shape is normative (DC-PROTO-01..05). |
| **Mux frame chokepoints** | `ade_network::mux::frame::{encode_frame, decode_frame}` | 2 free functions | The **single** byte↔frame translation in the project. `mux::transport` is the only caller in RED; `session::mod` (when populated) is the second. |
| **Mini-protocol transition functions** | `ade_network::{block_fetch, chain_sync, handshake, keep_alive, peer_sharing, tx_submission}::transition` + `ade_network::n2c::local_*::transition` | 8 state-machine modules (handshake module exposes both `n2n_transition` and `n2c_transition`) | Each transition is `fn (state, agency, version, msg) -> Result<(new_state, output), error>` — pure, sync, no ambient session influence (DC-PROTO-06). Closed state graphs; illegal tuples produce `IllegalTransition`. **Adding a mini-protocol = new transition function; never extend an existing graph for a new protocol.** |
| **Mini-protocol version enums** | `ade_network::codec::version::{N2NVersion, N2CVersion, ChainSyncVersion, BlockFetchVersion, KeepAliveVersion, PeerSharingVersion, TxSubmission2Version, LocalChainSyncVersion, LocalStateQueryVersion, LocalTxMonitorVersion, LocalTxSubmissionVersion}` | 11 closed enums | Each pins the upper version this codec/state-machine pair has been audited against. Bumping = registry diff + new corpus + cluster doc. Mismatches surface as `InvalidForVersion` at the boundary. |
| **`ChainDb` trait surface** | `ade_runtime::chaindb::mod` | 6 methods (`put_block`, `get_block_by_hash`, `get_block_by_slot`, `tip`, `iter_from_slot`, `rollback_to_slot`) | Object-safe; intended for multiple impls (in-memory + redb at HEAD; future: sharded / network-backed). |
| **`SnapshotStore` trait surface** | `ade_runtime::chaindb::mod` | 5 methods (`put_snapshot`, `get_snapshot`, `latest_snapshot`, `list_snapshot_slots`, `delete_snapshot`) | Same closure discipline as `ChainDb`. Bytes are opaque at this layer (S-35). |
| **`Recoverable` trait surface** | `ade_runtime::recovery` | 2 methods (`decode_snapshot`, `apply_block`) + 1 associated type (`Error`) | Caller-supplied. Trait deliberately commits to a single error type per impl; multi-error callers wrap. |
| **`recover` entry point** | `ade_runtime::recovery::recover` | 1 free function | The sole composition of `ChainDb` + `SnapshotStore` + `Recoverable` into a recovery sequence. |
| **Hash domain functions** | `ade_crypto::blake2b::{block_header_hash, transaction_id, script_hash, credential_hash}` | 4 named domains | Algorithm immutable per protocol version. |
| **CI check set** | `ci/ci_check_*.sh` | 22 scripts | Existing checks may be tightened, never relaxed. New CI check is **additive**. Deleting a CI script requires recording the deprecation in the invariant registry's `ci_scripts` arrays. |
| **Invariant registry families** | `docs/ade-invariant-registry.toml` | Multiple family letters (T, CN, DC, OP, RO — DC family expanded under N-A for `DC-PROTO-*` and `DC-CORE-01`) | Append-only IDs; rules may be strengthened, never weakened; deprecation needs an explicit `deprecated_in`. |

### Extensible (open within constraints)

| Registry | Location | Extension Rule |
|----------|----------|---------------|
| `CostModels` map (Plutus V1/V2/V3 cost tables) | `ade_plutus::cost_model::CostModels` | New entries enter via the cost-model CBOR decoder when a protocol parameter update lands. Not runtime-pluggable; constrained by the closed `PlutusLanguage` set. |
| `ProtocolParameters` / `ProtocolParameterUpdate` field set | `ade_ledger::pparams` | Fields are appended per era. Not extensible at runtime — versioned-gated by era. |
| Pool registrations / DRep registrations / Stake registrations | `ade_ledger::state::{DelegationState, CertState}` | Mutated at runtime by `ade_ledger::delegation::apply_cert`. The **shape** of what can be registered is closed; the **set** of registrations is open and grows monotonically. |
| Governance proposal set | `ade_ledger::state::ConwayGovState::proposals` | Same pattern — shape closed, instance set open, lifecycle managed by `evaluate_ratification` / `enact_proposals` / `expire_proposals`. |
| Oracle reference snapshots / regression corpus | `ade_testkit::harness::{snapshot_loader, regression_corpus}` | Tooling-only. New oracle data added via `corpus/` directory plus a manifest update. `ci_check_ref_provenance.sh` enforces manifest checksum integrity. GREEN. |
| Network corpus (mini-protocol transcripts, handshake fixtures, divergence sets) | `corpus/network/{n2n,n2c}/*` | Tooling-only. Captured via the `ade_network::bin::capture_*` tools, then committed as deterministic fixtures consumed by `cargo test -p ade_network`. Append-only by convention; provenance recorded alongside the corpus. |
| `KillStrategy<D>` trait impls | `ade_runtime::chaindb::crash_safety` | RED-only test infrastructure. New strategies plug in via the trait; `NoKill` is the production default. Object-safe by intent. |
| Recovery state types | callers of `Recoverable` | Open: any state type with a canonical encode + apply-block step can be recovered. The trait is the only way in; no central registry of state types. |
| Pinned external crates (`redb`, `aiken_uplc`, `pallas-primitives`, `blake2`, `ed25519-dalek`, `cardano-crypto`, `tokio` [`ade_network` RED only]) | `crates/*/Cargo.toml` | New external crate addition requires a Tier-5 rationale doc (per `docs/active/CE-79_tier5_addendum.md`). |

### Candidates — extensible surfaces not yet wired

| Cluster | Candidate registry | Rationale |
|---------|-------------------|-----------|
| N-A (deferred) | Peer address book | Operator-supplied; runtime mutable. Should live in `ade_runtime` (not `ade_network` BLUE). Deferred from cluster N-A scope. |
| N-E | Mempool tx prioritization policy | Tier 5 — operator-tunable. Plugin trait candidate: `MempoolPolicy`. |
| N-F | Query API method set | Tier 5 wire / Tier 1 semantics. Should be a closed enum internally, mapped to gRPC / HTTP at the edge; shared with LSQ/LocalTxMonitor semantic dispatch. |
| N-F | Prometheus metric names | Tier 5; append-only registry expected (parallel to invariant registry's append-only discipline). |

User confirmation needed for each at cluster entry: closed enum vs.
trait-based registry; runtime-extensible vs. compile-time-fixed; CI
enforcement shape.

---

## 4. Version-Gated vs. Frozen Contracts

### Frozen (immutable at current version — change = new major version)

- **Cardano-canonical CBOR wire format**: Each `decode_*_block` in
  `ade_codec` produces values whose wire bytes are preserved
  byte-identically. Hash inputs are wire bytes, not re-encoded bytes
  (enforced by `ci_check_hash_uses_wire_bytes.sh`).
- **Block envelope shape**: `[era_tag:u8, era_block:CBOR]` as a
  definite-length 2-element CBOR array; era tags 0..=7 (closed). Adding
  era 8 is a hard fork.
- **`PreservedCbor<T>` invariant**: `wire_bytes()` is exactly what the
  decoder consumed, byte-identical. Re-encoding is permitted only via
  `canonical_bytes(ctx)` and never used for hashing.
- **Hash algorithms**: Blake2b-224 for credential / script hashes,
  Blake2b-256 for block / transaction / Merkle hashes. Ed25519,
  Byron-bootstrap (extended Ed25519), KES-sum, VRF-draft-03 — all
  wired in `ade_crypto`, all protocol-frozen.
- **Plutus script ingress chokepoint**: `PlutusScript::from_cbor` in
  `crates/ade_plutus/src/evaluator.rs`. Moving the function elsewhere
  invalidates the path-exact allowlist in
  `ci_check_ingress_chokepoints.sh` Check 3.
- **Plutus language set**: V1, V2, V3. PV11 builtins (`ExpModInteger`,
  `CaseList`, `CaseData`) deliberately gated off — see S-29.
- **Aiken UPLC quarantine pin**: `aiken_uplc` (git dep) at tag
  `v1.1.21`, commit `42babe5d`.
- **Ouroboros mux frame layout** (N-A): 8-byte big-endian header —
  `[u32 timestamp][u16 mode_bit(1) + mini_protocol_id(15)][u16 length]`,
  payload `≤ 65535` bytes. Frozen for the protocol; new framing = new
  surface, not an extension of `mux::frame`.
- **11 closed mini-protocol message enums** (N-A): wire grammar per
  protocol is protocol-fixed. New variant = registry diff + new
  cardano-node 11.0.1 (or successor) conformance corpus.
- **8 closed mini-protocol state graphs** (N-A): each transition's
  legal `(state, agency, version, msg)` tuple set is normative. Illegal
  tuples produce `IllegalTransition` deterministically.
- **All 182 canonical types**: shapes are frozen at the era / version
  they entered. Adding fields requires a versioned gate; renaming is
  forbidden.
- **TCB color assignments**: Per `.idd-config.json` `core_paths`. BLUE
  ↔ RED separation is mechanical. `ade_network`'s split is
  per-submodule (BLUE submodules: `codec`, `handshake`, `chain_sync`,
  `block_fetch`, `tx_submission`, `keep_alive`, `peer_sharing`, `n2c`,
  `mux::frame`; RED submodules: `mux::transport`, `session`,
  `bin::capture_*`; GREEN: `lib`, `mux::mod`).
- **`ChainDb` / `SnapshotStore` / `Recoverable` trait shapes** (N-D
  closed): the trait method sets are frozen. Adding a method = new
  slice with a contract-test extension.

### Version-gated (can evolve across major versions)

- **New `CardanoEra` variant**: requires new `decode_*_block`
  chokepoint, new per-era composer in `ade_ledger`, new hfc
  translation arm, new addition to `CardanoEra::ALL`, and an extension
  of the named-chokepoint header in
  `ci_check_ingress_chokepoints.sh`.
- **New `GovAction` / `ConwayCert` / Plutus version variant**:
  requires registry diff (§3) plus arms in every chokepoint.
- **New protocol parameter field**: append to `ProtocolParameters`;
  CBOR field-order discipline preserved by `ade_codec`.
- **New CI check**: additive. Removing an existing check requires
  invariant-registry deprecation note.
- **Pinned external crate bump**: Tier-5 rationale doc required;
  cross-references invariant-registry quarantine families
  (`aiken_uplc` quarantine; `pallas_*` quarantine).
- **New mini-protocol** (e.g., a hypothetical extension beyond the 11
  closed today): adds a new module under `ade_network/` with a closed
  enum, new `encode_/decode_` chokepoint pair, new transition function,
  and new `*Version` enum. **Never an arm on an existing enum.**
- **Mini-protocol version-table bump**: each `*Version` enum may grow
  by appending a higher variant when a new cardano-node release pins a
  new version. Requires new conformance corpus and a corresponding
  registry strengthening; old variants must continue to round-trip
  (`InvalidForVersion` only on unrecognized future variants).
- **Phase-4 cluster surface additions** (N-B, N-C, N-E, N-F): each
  cluster's wire surface gates additions via its own cluster doc;
  pipeline steps added there cannot collide with existing chokepoints.

---

## 5. Module Addition Rules

Ade's workspace is small and color-disciplined. New modules enter as
new crates under `crates/`. `ade_network` is the first BLUE crate with
**per-submodule** color assignment rather than whole-crate; future
crates with the same shape should follow its pattern (top-of-crate
banner naming each submodule's color, plus a CI script that scans only
the BLUE submodule subset).

| Color | Naming convention | Build-config flags | May depend on | MUST NOT depend on |
|-------|-------------------|--------------------|----------------|--------------------|
| **BLUE** | `ade_*` (no color in name) | First line of every `.rs` is the contract banner `// Core Contract:`. `lib.rs` carries `#![deny(unsafe_code)]`, `#![deny(clippy::unwrap_used/expect_used/panic/float_arithmetic)]`. No `#[cfg(feature = ...)]` (semantic gates forbidden). No async (DC-CORE-01). | Other BLUE crates / submodules only | Any RED submodule or crate; GREEN in non-dev deps; `pallas_*` (except `ade_plutus`); async runtime; `HashMap`/`HashSet`/`IndexMap`; clock/rand/float/env/I/O. |
| **GREEN** | `ade_*` | Banner + deny attrs are project convention but not currently enforced for `ade_testkit` / `ade_network::mux::mod`. May use `HashMap`/`serde_json`/`flate2`/`tar` for fixture I/O. | BLUE crates + standard library + ecosystem crates | `ade_runtime`, `ade_node`, RED submodules. Results must never feed back into BLUE state. |
| **RED** | `ade_*` | No special header. Free to use clocks, I/O, async, `HashMap`, signing keys (`ade_runtime` is the only crate that may sign). | Any BLUE / GREEN crate or submodule (one-way) | Cannot be depended on by BLUE (`ci_check_dependency_boundary.sh`, `ci_check_no_async_in_blue.sh`). |

### New module checklist

1. **Add to `Cargo.toml` workspace members.** `version = "0.1.0"`,
   `edition = "2021"`.
2. **Declare TCB color** by editing `.idd-config.json` `core_paths` if
   BLUE; if the crate is mixed-color (like `ade_network`), name each
   BLUE submodule path and ensure the BLUE CI scripts scan the
   submodule subset (not the whole crate).
3. **CI script update obligations** — extend the relevant BLUE-scoped
   scripts (`ci_check_module_headers.sh`, `ci_check_no_semantic_cfg.sh`,
   `ci_check_no_signing_in_blue.sh`, `ci_check_hash_uses_wire_bytes.sh`,
   `ci_check_ingress_chokepoints.sh`, `ci_check_dependency_boundary.sh`,
   `ci_check_no_async_in_blue.sh`, `ci_check_forbidden_patterns.sh`).
4. **Add contract banner** (BLUE) to every `.rs` file.
5. **Add deny attributes** to `lib.rs` (BLUE).
6. **New canonical types:** at HEAD the canonical-type registry is
   inline in the invariant registry (`canonical_type_registry: null`
   in `.idd-config.json`) — add a `[[rules]]` block under family `T`
   with `tier = "true"` for each new type, plus a round-trip test
   referenced in the rule's `tests` array.
7. **Run `cargo test --workspace` and the full CI script suite.** Both
   must be green before the cluster can close.

### Phase 4 anticipated additions

- **N-B (chain selection)**: introduces the composition layer that
  bridges `ade_network` block-fetch output into `ade_codec` decoding
  and `ade_ledger` apply. Likely lives in `ade_node` (RED) or a new
  `ade_consensus` BLUE crate.
- **N-C (forge)**: forge-block path likely lives in `ade_runtime` (RED)
  for KES / VRF signing but must call into `ade_ledger` for canonical
  validation.
- **N-E (mempool)**: likely a new `ade_mempool` BLUE crate (canonical
  tx admission) with a RED operator shim in `ade_runtime`.
- **N-F (operator API)**: thin RED layer mapping a closed Query enum
  to gRPC/HTTP; shares semantic dispatch with N-A's LSQ /
  LocalTxMonitor opaque-bytes payloads.

**These placements are candidates** — user confirmation needed at
cluster entry.

---

## 6. Forbidden Patterns (per color)

### BLUE (universal IDD prohibitions; enforced by CI where marked)

- No `HashMap`, `HashSet`, `IndexMap`, `IndexSet`, `indexmap::*` —
  `ci_check_forbidden_patterns.sh`.
- No `SystemTime`, `Instant`, `std::time::*` clocks —
  `ci_check_forbidden_patterns.sh`.
- No `rand::thread_rng`, `thread::spawn` —
  `ci_check_forbidden_patterns.sh`.
- No `f32`, `f64`, floating-point arithmetic — enforced by
  `#![deny(clippy::float_arithmetic)]` plus the pattern script.
- No `std::fs`, `std::net`, `tokio`, `async fn` —
  `ci_check_forbidden_patterns.sh` + `ci_check_no_async_in_blue.sh`
  (the latter scans `ade_network`'s BLUE submodules specifically).
- No `anyhow`; `unwrap`/`expect`/`panic` denied at the lint level.
- No `unsafe` outside an explicit allowlist (currently only
  `ade_crypto::vrf`'s FFI binding).
- No `#[cfg(feature = ...)]` semantic gating —
  `ci_check_no_semantic_cfg.sh`.
- No signing patterns in BLUE — `ci_check_no_signing_in_blue.sh`.
- No re-hashing of `canonical_bytes` or re-encoded bytes — wire bytes
  only. `ci_check_hash_uses_wire_bytes.sh`.
- No construction of `PreservedCbor` outside `ade_codec` —
  `ci_check_ingress_chokepoints.sh` Checks 1 & 2.
- No raw CBOR decoding in any BLUE crate except `ade_codec` and the
  single allowlisted file `crates/ade_plutus/src/evaluator.rs` —
  `ci_check_ingress_chokepoints.sh` Check 3.
- No `pallas_*` reference outside `ade_plutus` —
  `ci_check_pallas_quarantine.sh`.
- **(N-A specific)** No `Box<dyn Codec>` / `Box<dyn Protocol>` /
  `#[non_exhaustive]` on mini-protocol message enums; no generic
  `Codec<P>` trait. **A new mini-protocol is a new closed module, not
  an arm on an existing enum.**
- **(N-A specific)** No reading of "selected protocol version" from a
  session global inside a transition function — version is an explicit
  input (DC-PROTO-06).
- **(N-A specific)** No decoding of block CBOR, tx CBOR, or address
  CBOR inside `ade_network` — those payloads remain `Vec<u8>` and are
  dispatched at the session / `ade_node` boundary.

### GREEN (`ade_testkit`, `ade_network::lib` / `mux::mod`)

- No nondeterminism that leaks into stored fixtures — fixtures must be
  byte-reproducible.
- No participation in authoritative outputs.
- No import of `ade_runtime` (preserves the GREEN-not-RED stance).
- No inbound dep from any RED crate.

### RED (`ade_runtime`, `ade_node`, `ade_network::mux::transport`, `ade_network::session`, `ade_network::bin::capture_*`)

- No direct mutation of `ade_ledger` state — all transitions go
  through `ade_ledger::rules::*`.
- No bypassing `ade_codec` to construct semantic types from raw bytes.
- (`ade_runtime` specifically) No dep on `ade_ledger` — bytes-in /
  bytes-out only (S-36 invariant).
- (`ade_runtime` specifically) No leakage of `redb` types through the
  `chaindb::*` public surface (S-34 invariant).
- No second public `chaindb` path — the trait is the only surface.
- No automatic snapshot pruning — operator-driven only (S-35, S-36).
- No partial-recovery success — mid-replay failure aborts (S-36).
- No async recovery surface — sync only; callers wrap if cancellation
  is needed (S-36).
- (`ade_network::mux::transport` specifically) No protocol logic;
  bearer I/O only. Frame parsing lives in BLUE `mux::frame`; codec /
  state-machine logic lives in BLUE codec / transition modules.
- (`ade_network::session` specifically) Composition glue only; never
  re-implements protocol logic that lives in BLUE transitions.
- (`ade_network::bin::capture_*` specifically) Live-interop tools only;
  never linked into the node binary; outputs are corpus fixtures, not
  authoritative state.

### Project-specific additions

- **No commits of credentials, hostnames, IPs, private keys** — public
  repo discipline; enforced by `ci_check_no_secrets.sh`.
- **No `Phase 4 internal-mode mock network`** — Tier 1 surfaces must
  be exercised against real cardano-node 11.0.1 peers (Phase 4 cluster
  plan §Forbidden patterns; CE-N-A-5 evidence).
- **No collapsing wire and canonical bytes** — dual-authority rule
  carries forward from Phases 1-3.
- **No Tier 5 surface without a stated rationale** — divergence from
  cardano-node requires naming "what's better" per
  `docs/active/CE-79_tier5_addendum.md`.
- **No "we'll match it later" stubs on Tier 1 surfaces** — Tier 1
  closure is hard-gated.

---

## Cross-references

- CODEMAP: `docs/ade-CODEMAP.md` — module-by-module authority table,
  upstream of this document.
- Invariant registry: `docs/ade-invariant-registry.toml` — rule
  families incl. `T`, `CN`, `DC` (with `DC-PROTO-*` + `DC-CORE-01`
  expanded under N-A), `OP`, `RO`. The registry's `tests` and
  `ci_scripts` arrays are the authoritative cross-reference for
  enforcement.
- Phase 4 cluster plan: `docs/active/phase_4_cluster_plan.md` — names
  the seven Phase 4 clusters and their tier classifications.
- Tier doctrine: `docs/active/CE-79_gate_statement.md` and
  `docs/active/CE-79_tier5_addendum.md`.
- Cluster N-D slices (closed): `docs/clusters/completed/PHASE4-N-D/S-{33..37}.md`
  — own the closed-surface invariants for `ChainDb`, `SnapshotStore`,
  `Recoverable`, and `recover`.
- Cluster N-A (closed): `docs/clusters/completed/PHASE4-N-A/cluster.md` +
  `docs/clusters/completed/PHASE4-N-A/S-A{1..10}.md` — own the closed-surface
  invariants for the 11 mini-protocol codecs, 8 mini-protocol
  transitions, the mux frame chokepoints, and the live-interop
  evidence script `ci_check_ce_n_a_5_proof.sh`.
- N-A planning trio: `docs/active/PHASE4-N-A_scope_decisions.md`,
  `docs/active/PHASE4-N-A_invariants.md`,
  `docs/active/PHASE4-N-A_cluster_plan.md`.
- N-A live-interop evidence: `docs/active/CE-N-A-5_evidence.toml`
  (manifest) + captured log under `docs/clusters/completed/PHASE4-N-A/`.
