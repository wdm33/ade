# Seams — Where New Work Can Attach (Ade)

> **Status:** Living architectural document. Regenerated; not hand-edited.
> Per-project instance of `~/.claude/methodology/templates/seams.md`.

> 11 crates, 25 CI checks at HEAD (`b9ff041`).
> Reads CODEMAP for the module list and TCB colors; reads the invariant
> registry (`docs/ade-invariant-registry.toml`) for rule IDs; reads the
> Phase 4 cluster plan (`docs/active/phase_4_cluster_plan.md`), the
> closed N-D / N-A cluster docs, and the closed N-B cluster doc plus
> planning trio (`docs/clusters/PHASE4-N-B/cluster.md`,
> `docs/planning/phase4-n-b-invariants.md`,
> `docs/planning/phase4-n-b-cluster-slice-plan.md`).

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
> pipelines. At HEAD there are six fully-wired ingress surfaces (block
> bytes, Plutus script bytes, snapshot bytes, Ouroboros mux frames,
> genesis JSON bundles, and chain-selector stream inputs), plus three
> further surfaces named in the Phase 4 plan (forge, mempool, query
> API).

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
the session / `ade_node` boundary.

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
     block decoders directly** — that bridge lands in the future
     `ade_node` composition layer.
Cross-surface state sharing: protocol version table
  (`ade_network::codec::version`) is shared across handshake +
  transition + codec call sites. No other shared state.
```

**Rule.** Mux frames are a distinct ingress surface, layered above the
byte bearer and below all higher protocol decoding. The two chokepoints
`mux::frame::{encode_frame, decode_frame}` are the only byte↔frame
translation in the project; `ade_network::mux::transport` (RED) calls
them and nothing else does. **Each mini-protocol's codec and transition
function form a self-contained, structurally independent closed
semantic surface (IDD §6).** Adding a new mini-protocol is *not* an
extension of an existing one — it is a new closed `*Message` enum + a
new `encode_*_message` / `decode_*_message` pair + a new `*_transition`
function + a new `*Version` enum in `ade_network::codec::version`.
There is no `Codec<P>` trait, no `Box<dyn Protocol>`, no
`#[non_exhaustive]`, no runtime negotiation of message meaning.
Versioning happens through closed `*Version` enums that gate which
variants are legal at protocol-step time; mismatches surface as
`InvalidForVersion` at the protocol boundary rather than as a silent
fallback.

### Surface: Genesis JSON bundles (wired in N-B)

```
Surface: Four genesis JSON blobs (byron + shelley + alonzo + conway)
Reduces to: EraSchedule { anchor: BootstrapAnchorHash, system_start_unix_ms, eras: [EraSummary; ≤7] }
            (defined in `ade_core::consensus::era_schedule`)
Pipeline (fixed step ordering — no reorder, no shortcut):
  1. caller assembles four byte slices into a `GenesisBundle`
     (`ade_runtime::consensus::genesis_parser::GenesisBundle` — closed
     struct with four named `&[u8]` fields; **not** an open bag of
     JSON files).
  2. ade_runtime::consensus::genesis_parser::compute_anchor_hash(&GenesisBundle)
       -> BootstrapAnchorHash                       (RED, pure)
     — Blake2b-256 over `b"ade_bootstrap_v1" || canonical_cbor([byron,
       shelley, alonzo, conway])`. **Domain-separation tag is frozen.**
  3. ade_runtime::consensus::genesis_parser::parse_genesis(&GenesisBundle, NetworkMagic)
       -> Result<EraSchedule, GenesisParseError>    (RED — uses serde_json)
     — the **single** RED → BLUE materialization chokepoint for the
       schedule. Returns a structured (no-`String`) error taxonomy:
       MalformedJson / MissingField / InvalidValue / UnknownNetwork /
       Hfc(HFCError). Internally validates `EraSchedule::new` (which
       in turn enforces monotonicity, non-empty era list, non-zero
       slot/epoch lengths).
  4. EraSchedule is then consumed BLUE **by-reference**; never
     mutated; never re-parsed. The `BootstrapAnchorHash` it carries
     binds the schedule to the parsed genesis bytes — any downstream
     consumer (header validate, leader schedule, rollback) that needs
     to assert "same genesis" compares anchor hashes.
Cross-surface state sharing: none. The schedule is constructed once at
  startup and threaded into every BLUE consensus surface as an
  argument. No global registry.
```

**Rule.** Genesis JSON is the **fifth distinct ingress surface**. Like
block CBOR, its decoder lives in a single named chokepoint and its
canonical reduction target (`EraSchedule`) is a BLUE type. Unlike block
CBOR, the decoder is RED (`genesis_parser` uses `serde_json` and
returns structured `GenesisParseError`) — but BLUE consensus never
re-parses, never reaches into JSON, and never re-derives the anchor
hash. The four-element domain-separated preimage layout is frozen at
v1; any future schema change to the anchor preimage is a hard
version-gated event because every downstream schedule check pivots on
`BootstrapAnchorHash`. `NetworkMagic` is a closed `enum`-shaped
newtype (MAINNET / PREPROD / PREVIEW); unknown magics produce a typed
`UnknownNetwork` reject, never a silent fallback.

### Surface: Chain-selector stream inputs (wired in N-B)

```
Surface: Ordered stream of N-A events (header arrival, rollback request, epoch boundary)
Reduces to: ade_runtime::consensus::chain_selector::StreamInput
            (closed 3-variant enum — `HeaderArrival(HeaderInput)`,
             `RollBack(RollBackRequest)`, `EpochBoundary { new_epoch,
             last_block_of_prev_epoch }`)
Pipeline (fixed step ordering — no reorder, no shortcut):
  1. caller wraps each external event in `StreamInput`.
  2. ade_runtime::consensus::chain_selector::process_stream_input(
         &mut OrchestratorState, &StreamInput,
         &dyn LedgerView, &EraSchedule)
       -> Result<Option<ChainEvent>, OrchestratorError>   (GREEN, sync, pure)
     — the **single** orchestrator chokepoint. Dispatches by variant:
       - HeaderArrival   -> validate_and_apply_header  (BLUE)
                         -> build_candidate_fragment   (GREEN materializer)
                         -> select_best_chain          (BLUE)
                         -> push_snapshot              (bounded ring; ≤ k)
       - RollBack        -> find snapshot by block_no
                         -> apply_rollback             (BLUE)
                         -> trim newer snapshots
       - EpochBoundary   -> apply_nonce_input          (BLUE)
  3. BLUE returns `ChainEvent` (closed 5-variant enum: ChainExtended,
     RolledBack, RolledForward, ChainSelected, Rejected) or a
     `ChainSelectionReject` carried inside `ChainEvent::Rejected`.
Cross-surface state sharing: `OrchestratorState` holds the
  authoritative `PraosChainDepState`, `ChainSelectorState`, and a
  bounded ring of `RollbackSnapshot { block_no, chain_dep, tiebreaker }`
  (default cap = DEFAULT_SNAPSHOT_LIMIT = 2160, the mainnet k). The
  ring is the only state shared across consecutive `StreamInput`s.
```

**Rule.** Stream inputs are the **sixth distinct ingress surface** and
the only one that drives Praos chain selection. The reduction shape is
deliberately small (3 variants) so the orchestrator's responsibility is
sequencing, not policy. **Every external trigger that can advance
Ade's chain state must reduce to one of these three variants** — there
is no "fast path" into BLUE consensus. The orchestrator never reads a
chain store, never calls into `ade_codec`, and never invents its own
state-shape decisions; BLUE owns each transition's success/reject
shape. `OrchestratorError` is closed (HeaderInvalid / NonceEvolution)
and only fires when the BLUE pipeline returns an `Err`; structured
rejects (TiebreakerLossKeepCurrent, ExceededRollback,
ForkBeforeImmutableTip, HeaderInvalid) surface inside
`ChainEvent::Rejected` so a single shape carries both new state and
the rejection record.

### Candidates — surfaces not yet wired (Phase 4 N-C, N-E, N-F)

The following surfaces are named in the Phase 4 cluster plan but have
no source today. They are listed so future slice docs can attach
without reinventing the reduction step. **Each is a candidate seam
pending confirmation at cluster entry.**

| Cluster | Surface | Expected reduction target | Expected chokepoint | Confidence |
|---------|---------|---------------------------|---------------------|------------|
| N-C | Forge-block inputs (mempool + state + slot + KES + VRF) | `BlockEnvelope` bytes (forged, then re-decoded for validation) | `ade_runtime::forge::forge_block` (proposed) | candidate |
| N-C | Operator block-production trigger | `StreamInput::HeaderArrival(HeaderInput)` (forged header is fed back into the same chain-selector entrypoint) | `process_stream_input` (existing) | candidate |
| N-E | Mempool tx ingest (from N-A tx-submission2 OR N-A local-tx-submission) | Per-era tx body (canonical bytes preserved) | `ade_runtime::mempool::ingest_tx` (proposed) | candidate |
| N-F | LSQ semantic dispatch (LocalStateQuery payloads) | Internal Query enum (closed, not yet defined) | Single dispatch fn that consumes `LocalStateQueryMessage::Acquire/Query/Result` opaque-bytes payloads — Tier 5 wire on operator-facing gRPC/HTTP, Tier 1 semantics shared with LSQ | candidate |
| N-F | LocalTxMonitor semantic dispatch | Mempool-snapshot Query/Reply enums | Single dispatch fn that consumes `LocalTxMonitorMessage` opaque-bytes payloads | candidate |
| N-B+ | Live cardano-node session driver (for `ade_core_interop::live_consensus_session`) | `StreamInput` translated from `ade_network::chain_sync::ChainSyncMessage` and `block_fetch::BlockFetchMessage` events | Composition layer in `ade_core_interop` (currently a `ready` stub binary; the full driver is operator-side work per S-B10) | candidate |

These candidates need user confirmation when each cluster is opened:
"Is the canonical reduction target named above the right one? Does the
chokepoint name fit the project's emerging naming convention?"

---

## 2. Data-Only vs. Authoritative Layers

Ade has five authoritative domains. For each, a single BLUE chokepoint
holds enforcement authority; tooling layers (when they exist) live in
GREEN (`ade_testkit`) or RED (`ade_runtime`, `ade_network::mux::transport`,
`ade_network::session`, `ade_core_interop`).

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
| **Live-interop capture tools** | `ade_network::bin::capture_*` (7 RED binaries) | RED | Operator/dev tools for live cardano-node 11.0.1 capture. Never linked into the node binary. |

**Rule.** Three rules carry the cluster:

1. **The codec layer is opaque to higher semantics.** `ade_network`
   never decodes block CBOR or tx CBOR — those payloads are `Vec<u8>`
   carried through `*Message` variants. The bridge into `ade_codec` /
   `ade_ledger` lives at the session/`ade_node` composition layer
   (currently a placeholder).
2. **The two chokepoints `mux::frame::{encode_frame, decode_frame}`
   never move.** Any future wire-framing change is a coordinated
   rewrite of both, not a duplicate path.
3. **The selected protocol version is an explicit transition input
   (DC-PROTO-06).** No state machine reads ambient session state.
   Mismatches surface as `InvalidForVersion`.

### Praos consensus runtime (N-B)

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **Data-only tooling (genesis)** | `ade_runtime::consensus::genesis_parser` | RED | `parse_genesis` + `compute_anchor_hash`. Reads JSON via `serde_json`, computes the v1 domain-separated anchor hash, produces a typed `EraSchedule`. Returns a closed `GenesisParseError` taxonomy (no `String`). |
| **Schedule authority** | `ade_core::consensus::era_schedule` | BLUE | `EraSchedule::new` validates monotonicity, non-empty era list, non-zero slot/epoch lengths; `locate`, `slot_to_time_ms`, `check_forecast_horizon` are pure integer arithmetic. `BootstrapAnchorHash` is carried verbatim and never recomputed in BLUE. |
| **Stake-snapshot boundary** | `ade_core::consensus::ledger_view::LedgerView` (trait, BLUE) ↔ `ade_testkit::consensus::ledger_view_stub::LedgerViewStub` (BTreeMap impl, GREEN) | mixed | BLUE consumes ledger-owned stake snapshots **by-reference only**; never owns, mutates, or re-derives them. The trait is intentionally small (4 methods, all returning `Option<...>`) so callers map `None` into their own typed reject (e.g. `LeaderScheduleError::UnknownPool`). A future production-grade impl will live in `ade_ledger`; the testkit stub is consumed only by integration tests. |
| **Header admission** | `ade_core::consensus::header_validate::validate_and_apply_header` | BLUE | Single chokepoint. Composes the 10-step pipeline (forecast → monotone slot → monotone block-no → op-cert pre-check → VRF nonce → VRF leader → leader threshold → op-cert observation → nonce contribution → advance state). Pipeline is sequential and fail-fast; no partial state. |
| **Best-chain authority** | `ade_core::consensus::fork_choice::select_best_chain` | BLUE | Single chokepoint. Total ordering is `(BlockNo, TiebreakerView{slot, issuer_hash, op_cert_counter, leader_vrf_output_first_8})`. **Chain-length-density ordering is explicitly forbidden** here (Genesis/catch-up reserved); enforced by `ci_check_no_density_in_fork_choice.sh`. |
| **Rollback authority** | `ade_core::consensus::rollback::apply_rollback` | BLUE | Single chokepoint. k-bound + immutable-tip refusal; rejects surface as `ChainEvent::Rejected { reason: ChainSelectionReject::* }` rather than `Err` so the caller keeps the prior state. |
| **Candidate materialization** | `ade_runtime::consensus::candidate_fragment::build_candidate_fragment` | GREEN | Builds the `CandidateFragment { anchor, anchor_block_no, headers, select_view, rollback_depth }` consumed by `select_best_chain`. Non-authoritative — only assembles the typed shape. |
| **Orchestration** | `ade_runtime::consensus::chain_selector::process_stream_input` | GREEN | Threads `StreamInput` through the BLUE pipeline; owns the bounded rollback-snapshot ring; never makes a comparison decision itself. |
| **Live-interop driver (scaffold)** | `ade_core_interop::bin::live_consensus_session` | RED | Operator-driven binary; current HEAD is a "ready" stub. The full driver (subscribe to N-A chain-sync, feed `HeaderArrival` into `process_stream_input`, validate tip agreement against a pinned cardano-node 10.6.2) is operator-side work owed by S-B10 closure-gate evidence. Never linked into the node binary. |
| **Replay harness** | `ade_testkit::consensus::stream_replay::replay_stream` | GREEN | Test-only driver for CE-N-B-5 — replays an ordered `StreamInput` slice through the orchestrator and returns `ReplayResult { final_state, events, steps, error }`. |

**Rule.** Five rules carry the cluster:

1. **The genesis parser is the sole RED → BLUE materialization point
   for `EraSchedule`.** No other crate may construct an `EraSchedule`
   from anything but a previously-validated one (`EraSchedule::new` is
   the only public constructor; it returns `Result<EraSchedule, HFCError>`
   and validates every invariant a typed schedule must satisfy).
2. **`BootstrapAnchorHash` binds the schedule.** Two consensus
   surfaces that both consume `&EraSchedule` are by construction
   consuming the same anchored schedule; anchor comparison is the
   canonical "same genesis" check. The v1 preimage layout
   (`b"ade_bootstrap_v1" || canonical_cbor([byron, shelley, alonzo, conway])`)
   is frozen; bumping it is a version-gated event.
3. **`LedgerView` is a closed trait, not a plugin point.** Production
   adds exactly one new impl (a future `ade_ledger`-backed one); RED
   shells must not call BLUE consensus with a hand-rolled
   `LedgerView` that bypasses ledger semantics.
4. **The three N-B authoritative chokepoints never move.**
   `validate_and_apply_header`, `select_best_chain`, and
   `apply_rollback` are the only BLUE entry points the orchestrator
   uses; new clusters add new variants to closed inputs, never new
   chokepoints.
5. **Selector and chain-dep advance in lockstep through the
   orchestrator.** Header validation always precedes fork-choice; a
   tiebreaker loss does not undo the BLUE chain-dep advance (matches
   ouroboros-consensus). Rollback restores both via a snapshot from
   the bounded ring.

### Where the boundary is enforced

- `ci_check_dependency_boundary.sh` — no BLUE crate may depend on
  `ade_runtime` or `ade_node`; `ade_network` BLUE submodules may not
  depend on RED submodules within the same crate.
- `ci_check_no_async_in_blue.sh` — async / tokio / futures forbidden
  in BLUE (including `ade_core::consensus`).
- `ci_check_no_chaindb_in_consensus_blue.sh` *(new in PHASE4-N-B)* —
  forbids any `ChainDb` / `chain_db` token in
  `crates/ade_core/src/consensus`. Strengthens DC-CORE-01 + DC-CONS-07.
- `ci_check_no_float_in_consensus.sh` *(new in PHASE4-N-B)* — forbids
  `f32` / `f64` tokens anywhere in `crates/ade_core/src/consensus`.
  Strengthens T-CORE-02 + DC-CONS-07/08/09.
- `ci_check_no_density_in_fork_choice.sh` *(new in PHASE4-N-B)* —
  forbids any `density` reference in `fork_choice.rs` / `candidate.rs`
  except on lines beginning with the audit marker
  `// no-density:`. Strengthens DC-CONS-03 by mechanically blocking the
  Genesis-style chain-length-density ordering term from creeping into
  caught-up Praos selection.
- `ci_check_consensus_closed_enums.sh` *(new in PHASE4-N-B)* — four
  checks on `ade_core::consensus`: (1) no `#[non_exhaustive]`; (2) no
  open-tail `Other` / `Unknown` variant declarations; (3) no owned
  `String` in `errors.rs` / `encoding.rs::DecodeError` / `events.rs`;
  (4) no `Box<dyn ...>`. Strengthens DC-CONS-04 + DC-CONS-10 +
  T-DET-01.
- `ci_check_pallas_quarantine.sh` — only `ade_plutus` may name
  `pallas_*`.
- `ci_check_no_signing_in_blue.sh` — signing patterns forbidden in
  BLUE; only `ade_runtime` may sign.
- `ci_check_ingress_chokepoints.sh` — three checks on `PreservedCbor`
  construction, named block-decoder presence, and raw-CBOR
  prohibition (with the `ade_plutus/src/evaluator.rs` allowlist).
- `ci_check_ce_n_a_5_proof.sh` — N-A live-interop evidence harness.

---

## 3. Closed vs. Extensible Registries

Ade's authority surface is **almost entirely closed.** This is a
consequence of being a chain-compatibility implementation: the
protocol fixes most variants. The few extensible surfaces are
operator-config or testkit-only.

### Closed (frozen — version-gated changes only)

| Registry | Location | Count | Change Rule |
|----------|----------|-------|-------------|
| `CardanoEra` | `ade_types::era` | 8 variants (ByronEbb, ByronRegular, Shelley, Allegra, Mary, Alonzo, Babbage, Conway) | New variant = new hard fork. Requires a coordinated change across `ade_codec` (new era's `decode_*_block` chokepoint), `ade_ledger` (new era composer + hfc translation), the canonical type list, and the genesis parser's `later_eras` table. Comment in source explicitly says "this enum is closed — unknown era tags produce a `CodecError`, never a fallback variant." |
| `Certificate` | `ade_types::shelley::cert` | 7 variants | Frozen Shelley-era certificate set. New cert types live in `ConwayCert`. |
| `ConwayCert` | `ade_types::conway::cert` | N variants (Conway-era certificates) | Version-gated per protocol — extends but does not modify `Certificate`. |
| `GovAction` | `ade_types::conway::governance` | 7 variants | CIP-1694 fixed; new variant = CIP amendment + ratification chokepoint update. |
| `MIRPot` | `ade_types::shelley::cert` | 2 variants (Reserves, Treasury) | Frozen. |
| `DRep` | `ade_types::conway::cert` | 4 variants | CIP-1694 fixed. |
| `PlutusLanguage` | `ade_plutus::evaluator` | 3 variants (V1, V2, V3) | New variant = new Plutus version. Requires cost-model table extension + aiken bump. PV11 builtins gated off (S-29). |
| **Named ingress chokepoints (block CBOR)** | `ade_codec::{cbor::envelope, byron, shelley, allegra, mary, alonzo, babbage, conway, address}` | 10 — `decode_block_envelope`, the per-era block decoders, `decode_address` | Header comment of `ci_check_ingress_chokepoints.sh` enumerates this set. New era = new chokepoint added in lockstep with a `CardanoEra` variant. Removal forbidden. |
| **Named ingress chokepoint (Plutus script CBOR)** | `ade_plutus::evaluator::PlutusScript::from_cbor` | 1 — file `crates/ade_plutus/src/evaluator.rs` | Distinct from the block-CBOR chokepoints. Allowlisted by exact file path in Check 3 of `ci_check_ingress_chokepoints.sh`. |
| **`PreservedCbor::new` constructor** | `ade_codec::preserved` | 1 chokepoint, `pub(crate)` | Construction lives inside `ade_codec`. |
| **Mini-protocol message enums** | `ade_network::codec::*` | 11 closed enums | Closed wire grammar per protocol. **No `#[non_exhaustive]`, no `dyn` dispatch, no generic `Codec<P>` trait.** New mini-protocol = new module + new closed enum + new `encode_*_message`/`decode_*_message` pair + new `*Version` enum + new transition function. Never an arm on an existing enum. |
| **Mini-protocol encode/decode chokepoints** | `ade_network::codec::*::{encode_<protocol>_message, decode_<protocol>_message}` | 22 functions (11 protocols × 2 directions) | Single chokepoint per direction per protocol. Removal or renaming forbidden; symbol shape is normative (DC-PROTO-01..05). |
| **Mux frame chokepoints** | `ade_network::mux::frame::{encode_frame, decode_frame}` | 2 free functions | The **single** byte↔frame translation in the project. |
| **Mini-protocol transition functions** | `ade_network::*::transition` + `n2c::local_*::transition` | 8 state-machine modules | Each: `fn (state, agency, version, msg) -> Result<(new_state, output), error>` — pure, sync, no ambient session influence (DC-PROTO-06). Closed state graphs; illegal tuples produce `IllegalTransition`. |
| **Mini-protocol version enums** | `ade_network::codec::version::*` | 11 closed enums | Each pins the upper version this codec/state-machine pair has been audited against. Bumping = registry diff + new corpus + cluster doc. Mismatches surface as `InvalidForVersion` at the boundary. |
| **`ChainDb` trait surface** | `ade_runtime::chaindb::mod` | 6 methods | Object-safe; intended for multiple impls (in-memory + redb at HEAD; future: sharded / network-backed). |
| **`SnapshotStore` trait surface** | `ade_runtime::chaindb::mod` | 5 methods | Same closure discipline as `ChainDb`. Bytes are opaque at this layer (S-35). |
| **`Recoverable` trait surface** | `ade_runtime::recovery` | 2 methods + 1 associated type | Caller-supplied. Trait commits to a single error type per impl. |
| **`recover` entry point** | `ade_runtime::recovery::recover` | 1 free function | The sole composition of `ChainDb` + `SnapshotStore` + `Recoverable` into a recovery sequence. |
| **Hash domain functions** | `ade_crypto::blake2b::{block_header_hash, transaction_id, script_hash, credential_hash}` | 4 named domains | Algorithm immutable per protocol version. |
| **`ChainEvent`** *(NEW in N-B)* | `ade_core::consensus::events` | 5 variants — ChainExtended, RolledBack, RolledForward, ChainSelected, Rejected | The complete output taxonomy of the fork-choice + rollback transitions. **No `#[non_exhaustive]`, no `Other`, no `String`** — enforced by `ci_check_consensus_closed_enums.sh`. A new event = new variant + arms in `encode_chain_event` / `decode_chain_event` + arms in every consumer. |
| **`ChainSelectionReject`** *(NEW in N-B)* | `ade_core::consensus::events` | 4 variants — ForkBeforeImmutableTip, ExceededRollback, HeaderInvalid, TiebreakerLossKeepCurrent | The complete reject taxonomy. Closed for the same reason as `ChainEvent`. Reject reasons are flat-data so corpus comparisons are byte-stable. |
| **Consensus error families** *(NEW in N-B)* | `ade_core::consensus::errors` | 8 closed error enums — HFCError, SlotTimeError, OutsideForecastRange, HeaderValidationError, VrfCertError, OpCertCounterError, NonceEvolutionError, LeaderScheduleError | Each is flat-data, no `String`, no `Box<dyn>`. Closed taxonomies enforced by `ci_check_consensus_closed_enums.sh`. |
| **`StreamInput`** *(NEW in N-B)* | `ade_runtime::consensus::chain_selector` | 3 variants — HeaderArrival, RollBack, EpochBoundary | The **single** ingress taxonomy for the chain-selector orchestrator. **Every external trigger that can advance Ade's chain state reduces to one of these three.** New trigger = new variant + arm in `process_stream_input`. **There is no plugin-style extension.** |
| **`OrchestratorError`** *(NEW in N-B)* | `ade_runtime::consensus::chain_selector` | 2 variants — HeaderInvalid, NonceEvolution | The orchestrator's fail-fast `Err`. Structured rejects (TiebreakerLoss, ExceededRollback, ForkBeforeImmutableTip) ride inside `Ok(Some(ChainEvent::Rejected { ... }))`; only header-validate / nonce-evolution surface here. |
| **`DecodeError`** *(NEW in N-B)* | `ade_core::consensus::encoding` | 4 variants — Cbor(&'static str), UnknownDiscriminant, FieldCountMismatch, InvalidLength | Closed CBOR-decode error taxonomy for `decode_chain_dep_state` / `decode_chain_event`. The `Cbor` payload is `&'static str` (audit-friendly), never `String`. |
| **`GenesisParseError`** *(NEW in N-B)* | `ade_runtime::consensus::genesis_parser` | 5 variants — MalformedJson, MissingField, InvalidValue, UnknownNetwork, Hfc(HFCError) | Closed RED-side parse-error taxonomy. **`field` is `&'static str`** (the JSON field name is a compile-time constant in the parser), so equality on errors is replay-stable. |
| **`GenesisBlob`** *(NEW in N-B)* | `ade_runtime::consensus::genesis_parser` | 4 variants — Byron, Shelley, Alonzo, Conway | Identifies which genesis JSON file an error refers to. Closed because the genesis bundle is structurally a four-tuple at v1. |
| **`NetworkMagic`** *(NEW in N-B)* | `ade_runtime::consensus::genesis_parser` | 3 const-named values — MAINNET, PREPROD, PREVIEW (held in a `u32` newtype) | Operator-facing public networks. An unknown magic produces `GenesisParseError::UnknownNetwork { magic }` — never a silent default. Adding a new operator-known network requires extending the `match` in `parse_genesis` plus a new boundary table under `shelley._ade_boundaries.<key>`. |
| **`LedgerView` trait** *(NEW in N-B)* | `ade_core::consensus::ledger_view` | 4 methods — `total_active_stake`, `pool_active_stake`, `pool_vrf_key`, `active_slots_coeff` | Closed-shape boundary by which BLUE consensus consults ledger-owned stake snapshots. All methods return `Option<...>` (caller maps `None` into a typed reject); methods are pure and deterministic. **Production adds exactly one new impl in a future `ade_ledger` slice — this trait is not a plugin point.** |
| **`PraosChainDepState` canonical encoding** *(NEW in N-B)* | `ade_core::consensus::encoding::{encode_chain_dep_state, decode_chain_dep_state}` | 2 chokepoints | The frozen CBOR encoding for the consensus chain-dep state. Round-trip is required (T-DET-01); any field addition is a version-gated bump of the encoding plus a corpus regeneration. |
| **`ChainEvent` canonical encoding** *(NEW in N-B)* | `ade_core::consensus::encoding::{encode_chain_event, decode_chain_event}` | 2 chokepoints | Same closure discipline as `PraosChainDepState`. Replay corpora compare events byte-for-byte. |
| **CI check set** | `ci/ci_check_*.sh` | 25 scripts | Existing checks may be tightened, never relaxed. New CI check is **additive**. Deleting a CI script requires recording the deprecation in the invariant registry's `ci_scripts` arrays. |
| **Invariant registry families** | `docs/ade-invariant-registry.toml` | Families T / CN / DC / OP / RO; DC family extended in N-A (`DC-PROTO-*`, `DC-CORE-01`) and N-B (`DC-CONS-03..10` sub-family) | Append-only IDs; rules may be strengthened, never weakened; deprecation needs an explicit `deprecated_in`. |

### Extensible (open within constraints)

| Registry | Location | Extension Rule |
|----------|----------|---------------|
| `CostModels` map (Plutus V1/V2/V3 cost tables) | `ade_plutus::cost_model::CostModels` | New entries enter via the cost-model CBOR decoder when a protocol parameter update lands. Not runtime-pluggable; constrained by the closed `PlutusLanguage` set. |
| `ProtocolParameters` / `ProtocolParameterUpdate` field set | `ade_ledger::pparams` | Fields are appended per era. Not extensible at runtime — versioned-gated by era. |
| Pool / DRep / Stake registrations | `ade_ledger::state::{DelegationState, CertState}` | Mutated at runtime by `ade_ledger::delegation::apply_cert`. The **shape** of what can be registered is closed; the **set** of registrations is open and grows monotonically. |
| Governance proposal set | `ade_ledger::state::ConwayGovState::proposals` | Same pattern — shape closed, instance set open, lifecycle managed by `evaluate_ratification` / `enact_proposals` / `expire_proposals`. |
| `OpCertCounterMap` *(NEW in N-B)* | `ade_core::consensus::praos_state` | BTreeMap keyed by `(Hash28, u64)`. Inserts are strictly increasing per `(pool, kes_period)` (returns `OpCertCounterError::Regression` on attempt). Shape closed; instance set open and grows monotonically with each accepted header. |
| `RollbackSnapshot` ring *(NEW in N-B)* | `ade_runtime::consensus::chain_selector::OrchestratorState::recent_snapshots` | Bounded `Vec<RollbackSnapshot>` capped at `DEFAULT_SNAPSHOT_LIMIT = 2160` (mainnet k); pushes are appended, oldest dropped on overflow. Configurable via `OrchestratorState::with_snapshot_limit` for tests. Authoritative consumers of rollback (the orchestrator itself) own the ring; no plugin extension. |
| Oracle reference snapshots / regression corpus | `ade_testkit::harness::{snapshot_loader, regression_corpus}` | Tooling-only. New oracle data added via `corpus/` directory plus a manifest update. `ci_check_ref_provenance.sh` enforces manifest checksum integrity. GREEN. |
| Network corpus (mini-protocol transcripts) | `corpus/network/{n2n,n2c}/*` | Tooling-only. Captured via the `ade_network::bin::capture_*` tools, then committed as deterministic fixtures. Append-only by convention; provenance recorded alongside the corpus. |
| Consensus corpus | `corpus/consensus/{hfc_schedule, nonce_evolution, op_cert, leader_schedule, fork_choice, rollback}/` | Tooling-only. Append-only by convention. Each replay test under `crates/ade_core/tests/` and `crates/ade_runtime/tests/` reads a fixed sub-tree; new test = new fixture file + new corpus entry. |
| `KillStrategy<D>` trait impls | `ade_runtime::chaindb::crash_safety` | RED-only test infrastructure. New strategies plug in via the trait; `NoKill` is the production default. Object-safe by intent. |
| Recovery state types | callers of `Recoverable` | Open: any state type with a canonical encode + apply-block step can be recovered. The trait is the only way in; no central registry of state types. |
| Pinned external crates | `crates/*/Cargo.toml` | New external crate addition requires a Tier-5 rationale doc (per `docs/active/CE-79_tier5_addendum.md`). |

### Candidates — extensible surfaces not yet wired

| Cluster | Candidate registry | Rationale |
|---------|-------------------|-----------|
| N-A (deferred) | Peer address book | Operator-supplied; runtime mutable. Should live in `ade_runtime`. |
| N-C | Block-production policy (forge cadence, KES rotation, slot election) | Tier 1 semantics, Tier 5 operator triggers. Forge inputs must reduce to the existing `BlockEnvelope` chokepoint. |
| N-E | Mempool tx prioritization policy | Tier 5 — operator-tunable. Plugin trait candidate: `MempoolPolicy`. |
| N-F | Query API method set | Tier 5 wire / Tier 1 semantics. Should be a closed enum internally, mapped to gRPC / HTTP at the edge; shared with LSQ/LocalTxMonitor semantic dispatch. |
| N-F | Prometheus metric names | Tier 5; append-only registry expected (parallel to invariant registry's append-only discipline). |

User confirmation needed for each at cluster entry: closed enum vs.
trait-based registry; runtime-extensible vs. compile-time-fixed; CI
enforcement shape.

### Closed-grammar audit (PHASE4-N-B specific)

This sweep was performed after PHASE4-N-B close. The author should
confirm each is intended-closed (no future plugin point) before any
extension is proposed:

1. `LedgerView` trait — **closed by intent**. The trait is a typed
   boundary, not an extension point. Exactly two impls expected:
   `LedgerViewStub` (GREEN, tests) and a future `ade_ledger`-backed
   impl. Any RED shell construction of a hand-rolled `LedgerView` for
   production use is a discipline failure.
2. `StreamInput` — **closed by intent**. Three variants enumerate
   every external trigger that can advance Ade's chain state. New
   ingress = new variant + new arm in `process_stream_input`; never a
   trait-object slot.
3. `ChainEvent` / `ChainSelectionReject` — **closed by intent**.
   Mechanically enforced by `ci_check_consensus_closed_enums.sh`
   (no `#[non_exhaustive]`, no `Other`/`Unknown`, no `String`, no
   `Box<dyn>`). New event/reject = new variant + arms everywhere it
   surfaces.
4. `PraosChainDepState` CBOR encoding — **closed by intent**.
   `encode_chain_dep_state` / `decode_chain_dep_state` are a single
   chokepoint pair; the encoding is round-trip stable; field
   additions are a version-gated bump.
5. `GenesisBundle` / `NetworkMagic` — **closed by intent**. The
   bundle is structurally a four-tuple of byron/shelley/alonzo/conway
   JSON at v1; the anchor preimage layout depends on this exact shape
   and order. Adding a fifth genesis blob bumps the v1 anchor domain
   tag.

No surfaces in this cluster look closed by accident.

---

## 4. Version-Gated vs. Frozen Contracts

### Frozen (immutable at current version — change = new major version)

- **Cardano-canonical CBOR wire format**: Each `decode_*_block` in
  `ade_codec` produces values whose wire bytes are preserved
  byte-identically. Hash inputs are wire bytes, not re-encoded bytes
  (enforced by `ci_check_hash_uses_wire_bytes.sh`).
- **Block envelope shape**: `[era_tag:u8, era_block:CBOR]` as a
  definite-length 2-element CBOR array; era tags 0..=7 (closed).
- **`PreservedCbor<T>` invariant**: `wire_bytes()` is exactly what the
  decoder consumed, byte-identical. Re-encoding never used for hashing.
- **Hash algorithms**: Blake2b-224 for credential / script hashes,
  Blake2b-256 for block / transaction / Merkle hashes. Ed25519,
  Byron-bootstrap (extended Ed25519), KES-sum, VRF-draft-03 — all
  wired in `ade_crypto`, all protocol-frozen.
- **Plutus script ingress chokepoint**: `PlutusScript::from_cbor` in
  `crates/ade_plutus/src/evaluator.rs`. Moving the function elsewhere
  invalidates the path-exact allowlist in
  `ci_check_ingress_chokepoints.sh` Check 3.
- **Plutus language set**: V1, V2, V3. PV11 builtins deliberately
  gated off — see S-29.
- **Aiken UPLC quarantine pin**: `aiken_uplc` (git dep) at tag
  `v1.1.21`, commit `42babe5d`.
- **Ouroboros mux frame layout**: 8-byte big-endian header,
  payload `≤ 65535` bytes.
- **11 closed mini-protocol message enums** (N-A): wire grammar per
  protocol is protocol-fixed.
- **8 closed mini-protocol state graphs** (N-A): each transition's
  legal `(state, agency, version, msg)` tuple set is normative.
- **`BootstrapAnchorHash` v1 preimage** *(NEW in N-B)*: Blake2b-256
  over `b"ade_bootstrap_v1" || canonical_cbor([byron, shelley, alonzo,
  conway])`. The domain tag, the four-element ordering, the canonical
  CBOR encoding (definite-length array with canonical width, byte
  strings with canonical minimal length), and the hash algorithm are
  all frozen for v1. Changing any of them is a hard version bump:
  every downstream schedule check pivots on anchor equality.
- **`EraSchedule` invariants** *(NEW in N-B)*: monotonically
  increasing `start_slot`, non-empty era list, non-zero
  `slot_length_ms` and `epoch_length_slots`. Enforced inside
  `EraSchedule::new`; the constructor is the only public path and
  `eras` is private behind `eras()`.
- **`PraosChainDepState` CBOR encoding** *(NEW in N-B)*: the wire
  format of `encode_chain_dep_state` is frozen for the protocol
  version it serializes. Round-trip via `decode_chain_dep_state` is
  byte-identical (T-DET-01); replay corpora pivot on this.
- **`ChainEvent` / `ChainSelectionReject` closed taxonomies** *(NEW
  in N-B)*: the variant set of both enums is frozen for the protocol
  version; rejects compare byte-for-byte across replays. Adding a new
  variant requires a coordinated rewrite of every consumer and a
  bumped envelope version.
- **Consensus error taxonomies** *(NEW in N-B)*: HFCError,
  HeaderValidationError, VrfCertError, OpCertCounterError,
  NonceEvolutionError, LeaderScheduleError, SlotTimeError,
  OutsideForecastRange — all flat-data, all `String`-free,
  `Box<dyn>`-free, replay-stable.
- **`StreamInput` 3-variant taxonomy** *(NEW in N-B)*: HeaderArrival,
  RollBack, EpochBoundary. Frozen at this shape; new ingress = new
  variant (version-gated), not a plugin slot.
- **All canonical types**: shapes are frozen at the era / version
  they entered. Adding fields requires a versioned gate; renaming is
  forbidden.
- **TCB color assignments**: per `.idd-config.json` `core_paths`.
  `ade_core::consensus` is BLUE under the `crates/ade_core/` prefix;
  `ade_runtime::consensus` is RED (genesis_parser + chain_selector
  orchestrator with `serde_json` and BTreeMap state);
  `ade_testkit::consensus` is GREEN (corpus + replay + stub);
  `ade_core_interop` is RED (live-interop driver).
- **`ChainDb` / `SnapshotStore` / `Recoverable` trait shapes** (N-D
  closed): the trait method sets are frozen.

### Version-gated (can evolve across major versions)

- **New `CardanoEra` variant**: requires new `decode_*_block`
  chokepoint, new per-era composer in `ade_ledger`, new hfc
  translation arm, new addition to `CardanoEra::ALL`, an extension of
  the named-chokepoint header in `ci_check_ingress_chokepoints.sh`,
  and an extension of the `later_eras` table in
  `ade_runtime::consensus::genesis_parser::parse_genesis`.
- **New `GovAction` / `ConwayCert` / Plutus version variant**:
  requires registry diff (§3) plus arms in every chokepoint.
- **New protocol parameter field**: append to `ProtocolParameters`;
  CBOR field-order discipline preserved by `ade_codec`.
- **New CI check**: additive. Removing an existing check requires
  invariant-registry deprecation note.
- **Pinned external crate bump**: Tier-5 rationale doc required.
- **New mini-protocol**: adds a new module under `ade_network/` with
  a closed enum, new chokepoint pair, new transition function, and
  new `*Version` enum. Never an arm on an existing enum.
- **Mini-protocol version-table bump**: each `*Version` enum may
  grow by appending a higher variant when a new cardano-node release
  pins a new version.
- **New `ChainEvent` / `ChainSelectionReject` variant** *(N-B)*:
  requires bumping the `PraosChainDepState`+events envelope version,
  adding arms in `encode_chain_event` / `decode_chain_event`, adding
  the orchestrator dispatch arm, and regenerating
  `corpus/consensus/{fork_choice, rollback}/` byte fixtures.
- **New `StreamInput` variant** *(N-B)*: requires extension of every
  call-site of `process_stream_input` plus a new corpus suite.
- **New `NetworkMagic`** *(N-B)*: requires the `parse_genesis` match
  arm + a new operator-supplied boundary table under
  `shelley._ade_boundaries.<key>` + a normative note that the
  bootstrap anchor is now committed to a wider set of operator
  networks.
- **New `LedgerView` impl** *(N-B)*: requires a slice that wires the
  impl into the orchestrator's call sites and a corresponding corpus
  showing equivalent observable behavior with the stub.
- **`BootstrapAnchorHash` preimage v2** *(N-B)*: hard version-gated.
  Requires a new domain tag (`b"ade_bootstrap_v2"`), a new preimage
  layout, a coordinated regeneration of every snapshot anchored on v1,
  and an explicit invariant-registry deprecation note for the v1
  layout.
- **Phase-4 cluster surface additions** (N-C, N-E, N-F): each
  cluster's wire surface gates additions via its own cluster doc.

---

## 5. Module Addition Rules

Ade's workspace is small and color-disciplined. New modules enter as
new crates under `crates/`. `ade_network` is the first BLUE crate with
**per-submodule** color assignment rather than whole-crate;
`ade_runtime` is mixed (RED in `chaindb` + `recovery`, GREEN-ish in
`consensus::candidate_fragment`, RED in `consensus::genesis_parser` and
`consensus::chain_selector`). Future crates with the same shape should
follow these patterns (top-of-crate banner naming each submodule's
color, plus a CI script that scans only the BLUE submodule subset).

| Color | Naming convention | Build-config flags | May depend on | MUST NOT depend on |
|-------|-------------------|--------------------|----------------|--------------------|
| **BLUE** | `ade_*` (no color in name) | First line of every `.rs` is the contract banner `// Core Contract:`. `lib.rs` carries `#![deny(unsafe_code)]`, `#![deny(clippy::unwrap_used/expect_used/panic/float_arithmetic)]`. No `#[cfg(feature = ...)]` (semantic gates forbidden). No async (DC-CORE-01). No `ChainDb` reference inside `ade_core::consensus` (DC-CONS-07). No `f32`/`f64` inside `ade_core::consensus`. No density-ordering term inside `ade_core::consensus::fork_choice`/`candidate`. | Other BLUE crates / submodules only | Any RED submodule or crate; GREEN in non-dev deps; `pallas_*` (except `ade_plutus`); async runtime; `HashMap`/`HashSet`/`IndexMap`; clock/rand/float/env/I/O. |
| **GREEN** | `ade_*` | Banner + deny attrs are project convention but not currently enforced for `ade_testkit` / `ade_network::mux::mod`. May use `HashMap`/`serde_json`/`flate2`/`tar` for fixture I/O. `ade_runtime::consensus::chain_selector` is GREEN but lives in `ade_runtime` for dep convenience; it uses only `Vec`/`BTreeMap`, no HashMap, no async. | BLUE crates + standard library + ecosystem crates | `ade_runtime` (for `ade_testkit`); RED submodules in non-test paths. Results must never feed back into BLUE state. |
| **RED** | `ade_*` | No special header. Free to use clocks, I/O, async, `HashMap`, signing keys (`ade_runtime` is the only crate that may sign). | Any BLUE / GREEN crate or submodule (one-way) | Cannot be depended on by BLUE (`ci_check_dependency_boundary.sh`, `ci_check_no_async_in_blue.sh`). |

### New module checklist

1. **Add to `Cargo.toml` workspace members.** `version = "0.1.0"`,
   `edition = "2021"`.
2. **Declare TCB color** by editing `.idd-config.json` `core_paths` if
   BLUE; if the crate is mixed-color, name each BLUE submodule path
   and ensure the BLUE CI scripts scan the submodule subset (not the
   whole crate).
3. **CI script update obligations** — extend the relevant BLUE-scoped
   scripts (`ci_check_module_headers.sh`, `ci_check_no_semantic_cfg.sh`,
   `ci_check_no_signing_in_blue.sh`, `ci_check_hash_uses_wire_bytes.sh`,
   `ci_check_ingress_chokepoints.sh`, `ci_check_dependency_boundary.sh`,
   `ci_check_no_async_in_blue.sh`, `ci_check_forbidden_patterns.sh`).
   For consensus-shaped additions, also extend
   `ci_check_no_chaindb_in_consensus_blue.sh`,
   `ci_check_no_float_in_consensus.sh`,
   `ci_check_consensus_closed_enums.sh`, and (if a fork-choice
   surface is touched) `ci_check_no_density_in_fork_choice.sh`.
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

- **N-C (forge)**: forge-block path likely lives in `ade_runtime`
  (RED) for KES / VRF signing but must call into `ade_ledger` for
  canonical validation. Reduction target is the existing
  `BlockEnvelope` chokepoint; the chain-selector orchestrator
  consumes the forged header through `StreamInput::HeaderArrival`.
- **N-E (mempool)**: likely a new `ade_mempool` BLUE crate
  (canonical tx admission) with a RED operator shim in `ade_runtime`.
- **N-F (operator API)**: thin RED layer mapping a closed Query enum
  to gRPC/HTTP; shares semantic dispatch with N-A's LSQ /
  LocalTxMonitor opaque-bytes payloads.
- **`ade_ledger`-backed `LedgerView` impl** *(N-B follow-on)*: lives
  in `ade_ledger` (BLUE) and consumes existing stake-snapshot state;
  wires into the orchestrator's call sites via a slice that swaps
  `LedgerViewStub` for the production impl in non-test paths.

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
  Additionally `ci_check_no_float_in_consensus.sh` narrows this to
  `ade_core::consensus` as a defense-in-depth.
- No `std::fs`, `std::net`, `tokio`, `async fn` —
  `ci_check_forbidden_patterns.sh` + `ci_check_no_async_in_blue.sh`.
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
  `Codec<P>` trait.
- **(N-A specific)** No reading of "selected protocol version" from a
  session global inside a transition function — version is an explicit
  input (DC-PROTO-06).
- **(N-A specific)** No decoding of block CBOR, tx CBOR, or address
  CBOR inside `ade_network`.
- **(N-B specific)** No `ChainDb` / `chain_db` token inside
  `ade_core::consensus` — `ci_check_no_chaindb_in_consensus_blue.sh`.
  BLUE consensus consumes typed inputs (`PraosChainDepState`,
  `EraSchedule`, `LedgerView`, `HeaderInput`, `RollBackRequest`,
  `&[CandidateFragment]`), never a storage handle.
- **(N-B specific)** No density-based ordering in caught-up Praos
  fork-choice — `ci_check_no_density_in_fork_choice.sh`. Density is
  reserved for Genesis / catch-up logic and is forbidden in
  `fork_choice.rs` / `candidate.rs`. Audit markers begin with
  `// no-density:`.
- **(N-B specific)** No `#[non_exhaustive]`, no open-tail `Other` /
  `Unknown` variant, no owned `String`, no `Box<dyn>` anywhere in
  `ade_core::consensus` — `ci_check_consensus_closed_enums.sh`.
  Every event and reject reason is structured flat data.
- **(N-B specific)** No body inspection for fork-choice tip
  comparison (sketch decision (i) from the N-B invariant doc).
  Block-number + `TiebreakerView` only.
- **(N-B specific)** No stake-snapshot rederivation inside BLUE
  consensus — consume `&dyn LedgerView` only. Snapshots are ledger-
  owned (DC-CONSENSUS-02).
- **(N-B specific)** No plugin-style runtime registration of
  consensus protocols. `StreamInput`, `ChainEvent`,
  `ChainSelectionReject` are closed.

### GREEN (`ade_testkit`, `ade_network::lib` / `mux::mod`, `ade_runtime::consensus::{candidate_fragment, chain_selector}`)

- No nondeterminism that leaks into stored fixtures — fixtures must be
  byte-reproducible.
- No participation in authoritative outputs.
- No `HashMap` even in test helpers — `BTreeMap` only (the
  `LedgerViewStub` uses `BTreeMap` by rule, not by accident).
- No import of `ade_runtime` from `ade_testkit` (preserves the
  GREEN-not-RED stance).
- No inbound dep from any RED crate (for `ade_testkit` /
  `ade_network::lib` / `mux::mod`).
- (`ade_runtime::consensus::chain_selector` specifically) No
  comparison decision; defer to BLUE `select_best_chain` /
  `apply_rollback` / `validate_and_apply_header`. The orchestrator
  threads inputs and stores BLUE-returned new state; it never invents
  its own reject reason.

### RED (`ade_runtime`, `ade_node`, `ade_network::mux::transport`, `ade_network::session`, `ade_network::bin::capture_*`, `ade_runtime::consensus::genesis_parser`, `ade_core_interop`)

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
  bearer I/O only.
- (`ade_network::session` specifically) Composition glue only; never
  re-implements protocol logic that lives in BLUE transitions.
- (`ade_network::bin::capture_*` specifically) Live-interop tools
  only; never linked into the node binary.
- (`ade_runtime::consensus::genesis_parser` specifically) **No
  re-derivation of the bootstrap anchor outside `compute_anchor_hash`,
  and no BLUE re-consumption of the JSON bytes** — once `EraSchedule`
  is materialized, the bytes are not held. The parser is the sole
  RED → BLUE materialization point for the schedule.
- (`ade_core_interop` specifically) Live-interop driver only; the
  CI does not run its tests by default (`#[ignore]` gates them); the
  binary outputs are operator evidence captures, not authoritative
  state.

### Project-specific additions

- **No commits of credentials, hostnames, IPs, private keys** —
  public-repo discipline; enforced by `ci_check_no_secrets.sh`.
- **No `Phase 4 internal-mode mock network`** — Tier 1 surfaces must
  be exercised against real cardano-node 11.0.1 (N-A) /
  cardano-node 10.6.2 (N-B) peers.
- **No collapsing wire and canonical bytes** — dual-authority rule.
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
  expanded under N-A and `DC-CONS-03..10` sub-family expanded under
  N-B), `OP`, `RO`.
- Phase 4 cluster plan: `docs/active/phase_4_cluster_plan.md`.
- Tier doctrine: `docs/active/CE-79_gate_statement.md` and
  `docs/active/CE-79_tier5_addendum.md`.
- Cluster N-D slices (closed):
  `docs/clusters/completed/PHASE4-N-D/S-{33..37}.md`.
- Cluster N-A (closed): `docs/clusters/completed/PHASE4-N-A/cluster.md`
  + `S-A{1..10}.md`.
- Cluster N-B (closed): `docs/clusters/PHASE4-N-B/cluster.md` +
  `S-B{1..10}.md`. Planning trio:
  `docs/planning/phase4-n-b-invariants.md`,
  `docs/planning/phase4-n-b-cluster-slice-plan.md`.
- N-A live-interop evidence: `docs/active/CE-N-A-5_evidence.toml`.
