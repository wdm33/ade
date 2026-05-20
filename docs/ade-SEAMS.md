# Seams — Where New Work Can Attach (Ade)

> **Status:** Living architectural document. Regenerated; not hand-edited.
> Per-project instance of `~/.claude/methodology/templates/seams.md`.

> 11 crates, 25 CI checks at HEAD (`0d4457e`).
> Reads CODEMAP for the module list and TCB colors; reads the invariant
> registry (`docs/ade-invariant-registry.toml`) for rule IDs; reads the
> Phase 4 cluster plan (`docs/active/phase_4_cluster_plan.md`), the
> closed N-D / N-A / N-B cluster docs, and the just-closed PHASE4-B1
> cluster doc plus planning trio
> (`docs/clusters/PHASE4-B1/cluster.md`,
> `docs/planning/phase4-b1-invariants.md`,
> `docs/planning/phase4-b1-cluster-slice-plan.md`).

Ade is a Cardano block-producing node. Its closure surface is dominated
by two facts:

1. The Cardano protocol fixes wire bytes and hashes for hash-critical
   paths (Tier 1 — must-conform). New work that touches those bytes
   has essentially no degrees of freedom.
2. Everything operator-facing — storage layout, query API, telemetry,
   packaging — is Tier 5: deliberate divergence "in our own image"
   (per `docs/active/CE-79_tier5_addendum.md`).

This document names where the system opens and where it stays closed.

**PHASE4-B1 (Full Block Validity Agreement) just closed.** It added the
**composition root** of the node — `block_validity`, the single function
that composes the consensus header authority (`ade_core`) and the ledger
body authority (`ade_ledger`) into one total Valid/Invalid verdict — plus
the production `LedgerView` projection, a new closed verdict/error
taxonomy family, a CBOR-round-trippable comparison surface, and the new
acyclic dependency edge `ade_ledger → ade_core`.

---

## 1. Surface Reduction Rules

> External inputs reduce to canonical form before entering authoritative
> pipelines. At HEAD there are six fully-wired *external* ingress
> surfaces (block bytes, Plutus script bytes, snapshot bytes, Ouroboros
> mux frames, genesis JSON bundles, and chain-selector stream inputs),
> plus the new **internal composition root** (`block_validity`) and the
> **consensus-input extraction surface** (snapshot `state` CBOR tail-scan)
> introduced in PHASE4-B1, plus three further surfaces named in the
> Phase 4 plan (forge, mempool, query API).

### Surface: Full block validity (composition root — wired in B1)

```
Surface: A full block (era-tagged envelope CBOR) decided against
         (LedgerState, PraosChainDepState, EraSchedule, LedgerView)
Reduces to: BlockValidityVerdict { Valid { tip, block_no, body } |
                                    Invalid { class, error } }
            (defined in `ade_ledger::block_validity::verdict`)
Pipeline (fixed step ordering — no reorder, no shortcut):
  1. ade_ledger::block_validity::decode_block(block_cbor) -> DecodedBlock
     (header_input projection + block hash + recomputed body hash +
      inner-block byte range; era-dispatched; Babbage/Conway only today)
  2. ade_core::consensus::validate_and_apply_header(
         chain_dep, &header_input, ledger_view, era_schedule)
     (BLUE header authority — FAIL-FAST; the body authority is NOT
      reached if this fails — DC-VAL-03)
  3. body-hash binding: computed_body_hash == applied.summary.body_hash
     (cheap pre-flight before body application — CN-CONS-04; an altered
      body is rejected here as BodyHashMismatch)
  4. ade_ledger::rules::apply_block_with_verdicts(ledger, era, inner)
     (BLUE body authority — consumes the INNER block, env tag stripped)
  5. Valid -> evolved (LedgerState', PraosChainDepState'); Invalid ->
     input states returned UNCHANGED (no partial mutation — DC-VAL-05)
Cross-surface state sharing: none — `block_validity` is a pure total
  function fn(&LedgerState, &PraosChainDepState, &EraSchedule,
  &dyn LedgerView, &[u8]) -> BlockValidityOutcome. Both states are
  threaded by value through the outcome; nothing ambient.
```

**Rule.** `block_validity` is the **single composition root** that joins
the two authorities. A block is `Valid` **iff** both
`validate_and_apply_header` (consensus) **and**
`apply_block_with_verdicts` (ledger body) accept it (DC-VAL-02). The
ordering is normative: header is decided first, body never runs on a
header-invalid block (DC-VAL-03). The body-hash binding sits **between**
the two authorities (DC-VAL-02/CN-CONS-04) — it is wired, not a no-op.
**No path may produce a `Valid` verdict while skipping either
authority** — in particular the follow-bridge's RED peer-trusted
"trust the body / skip header" shortcut must not leak into this BLUE
verdict. `block_validity` introduces **no new validation rules**: it is
composition only (DC-VAL-02). The function does not move and does not
gain a second public entry; new clusters tighten the two authorities it
composes, not the composer.

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
the session / `ade_node` boundary. The B1 composition root reuses this
same chokepoint: `decode_block` calls `decode_block_envelope` plus the
per-era block decoder; it does not invent a parallel decode path.

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

### Surface: Consensus-input extraction (snapshot `state` CBOR tail-scan — wired in B1)

```
Surface: A UTxO-HD `utxohd-mem` ExtLedgerState snapshot `state` CBOR
         (external dump format — NOT an authoritative canonical type)
Reduces to: PraosNonces { evolving, candidate, epoch, lab,
                          last_epoch_block }   (5 Nonce([u8;32]) in
            record order — the third, `epoch`, is eta0)
            (defined in `ade_ledger::consensus_input_extract`)
Pipeline (fixed step ordering — no reorder, no shortcut):
  1. ade_ledger::consensus_input_extract::extract_praos_nonces(&[u8])
       -> Result<PraosNonces, NonceScanError>
     — a pure tail-scan for the 4-byte non-neutral nonce prefix
       (`82 01 5820`) followed by a 32-byte body. Fail-CLOSED: the
       captured snapshots always carry EXACTLY five contiguous nonce
       wrappers; anything other than five is a hard `NotFiveNonces`
       error, never a best-effort pick.
Cross-surface state sharing: none — the scan is pure over the input
  bytes; the extracted nonces seed a `PraosChainDepState` at the caller.
```

**Rule.** This is the provenance surface for the consensus nonces that
seed `PraosChainDepState`. It is **classified RED behavior** (it parses
an external dump format rather than an authoritative canonical type) but
the function is pure over its bytes, lives in `ade_ledger`, and respects
every BLUE forbidden pattern (no I/O, no clock, no HashMap, fail-closed).
It is the **only** sanctioned way to lift Praos nonces out of a captured
snapshot; it never re-derives them and never picks heuristically. The
exact-five requirement is a closure invariant: a future capture format
that carries a different nonce count is a version-gated change, not a
silent relaxation. **Candidate flag:** the module's own doc-comment
calls itself "RED" while it physically lives inside a BLUE crate
(`ade_ledger`); the cluster doc's TCB Color Map lists it as "RED (in
`ade_ledger` or testkit tool)." This dual placement is intentional
(pure-over-bytes, no ambient nondeterminism) but should be confirmed —
if a future capture introduces real I/O, the loader half must move to
`ade_runtime`/testkit and only the pure scan stays here.

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
     consumer (header validate, leader schedule, rollback,
     block_validity) that needs to assert "same genesis" compares
     anchor hashes.
Cross-surface state sharing: none. The schedule is constructed once at
  startup and threaded into every BLUE consensus surface as an
  argument. No global registry.
```

**Rule.** Genesis JSON is a **distinct ingress surface**. Like
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

**Rule.** Stream inputs are the **header-only** ingress surface that
drives Praos chain selection. The reduction shape is deliberately small
(3 variants) so the orchestrator's responsibility is sequencing, not
policy. **Every external trigger that can advance Ade's chain state must
reduce to one of these three variants** — there is no "fast path" into
BLUE consensus. The orchestrator never reads a chain store, never calls
into `ade_codec`, and never invents its own state-shape decisions; BLUE
owns each transition's success/reject shape. `OrchestratorError` is
closed (HeaderInvalid / NonceEvolution) and only fires when the BLUE
pipeline returns an `Err`; structured rejects (TiebreakerLossKeepCurrent,
ExceededRollback, ForkBeforeImmutableTip, HeaderInvalid) surface inside
`ChainEvent::Rejected` so a single shape carries both new state and
the rejection record. **Relationship to `block_validity`:** the
orchestrator validates *headers* (cheap, fork-choice-relevant); the
composition root validates *full blocks* (header ∧ body). At HEAD these
are two distinct surfaces; the future `ade_node` layer wires them so a
header that wins fork-choice triggers a full `block_validity` decision
on the fetched body. That bridge is a candidate seam, not yet wired.

### Candidates — surfaces not yet wired (Phase 4 B2/B3, N-C, N-E, N-F)

The following surfaces are named in the Phase 4 plan / B1 planning but
have no source today. They are listed so future slice docs can attach
without reinventing the reduction step. **Each is a candidate seam
pending confirmation at cluster entry.**

| Cluster | Surface | Expected reduction target | Expected chokepoint | Confidence |
|---------|---------|---------------------------|---------------------|------------|
| B2 (tx validity) | Single-tx validity (the body authority's per-tx half) | A per-tx verdict closely paralleling `BlockValidityVerdict` | A `tx_validity`-shaped chokepoint built atop `apply_block_with_verdicts`' per-tx machinery | candidate |
| B1+ (header→body bridge) | Forge/fetch bridge: a fork-choice-winning header triggers a full-block decision on the fetched body | `block_validity(...)` over the fetched body | `ade_node` composition layer joining `process_stream_input` and `block_validity` | candidate |
| B1+ (pre-Babbage) | TPraos full-block validity (Shelley..Alonzo) | `BlockValidityVerdict` via a TPraos `HeaderInput` projection | extend `block_validity::decode_block` to build `HeaderVrf::Tpraos` headers (today it returns a typed reject for non-Babbage/Conway) | candidate |
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

Ade has six authoritative domains. For each, a single BLUE chokepoint
holds enforcement authority; tooling layers (when they exist) live in
GREEN (`ade_testkit`) or RED (`ade_runtime`, `ade_network::mux::transport`,
`ade_network::session`, `ade_core_interop`).

### Full block validity — the composition root (B1)

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **Decode / projection** | `ade_ledger::block_validity::header_input::decode_block` | BLUE | Era-dispatched: reuses `decode_block_envelope` + the per-era block decoder, projects a `HeaderInput` (Praos for Babbage/Conway), recomputes the era-correct (segwit) body hash over preserved wire bytes, and records the inner-block byte range. Builds inputs; asserts nothing. |
| **Consensus header authority** | `ade_core::consensus::validate_and_apply_header` | BLUE | The header half. Decided first, fail-fast. |
| **Ledger body authority** | `ade_ledger::rules::apply_block_with_verdicts` | BLUE | The body half. Consumes the inner block, never reached on header failure. |
| **Composition transition** | `ade_ledger::block_validity::transition::block_validity` | BLUE | The single chokepoint joining the two authorities + the body-hash binding. `fn(&LedgerState, &PraosChainDepState, &EraSchedule, &dyn LedgerView, &[u8]) -> BlockValidityOutcome`. |
| **Comparison surface** | `ade_ledger::block_validity::encoding::{encode_verdict_surface, decode_verdict_surface}` | BLUE | Canonical CBOR for the **coarse** replay/oracle surface only (`VerdictSurface`). The full `LedgerError`/`HeaderValidationError` detail is debug-only and NOT encoded. |
| **Positive replay harness** | `ade_testkit::validity::replay` | GREEN | Drives `block_validity` over the Conway-576 positive corpus; asserts byte-identical verdict streams. |
| **Adversarial harness** | `ade_testkit::validity::adversarial` | GREEN | Deterministic block mutators (M1–M6) derive adversarial blocks from the real corpus; asserts each maps to its expected reject class. |

**Rule.** This domain has **two sub-authorities and one composer**. New
work that tightens the header half lands in `ade_core::consensus`; new
work that tightens the body half lands in `ade_ledger::rules` and the
per-era composers. **The composer `block_validity` introduces no rules
of its own and never moves** (DC-VAL-02). The verdict comparison surface
is deliberately *coarse* (`BlockRejectClass`: HeaderInvalid / BodyInvalid
/ BodyHashMismatch / MalformedField / MissingConsensusInput) so corpus
comparisons against the reference node are byte-stable; the rich
structured `BlockValidityError` rides alongside for debugging but is
**not** part of the canonical bytes (this split is the same "wire vs.
semantic" rib the project applies everywhere). **Known extension point
for B2:** the body authority's per-tx witness/script depth is whatever
`apply_block_with_verdicts` enforces today — refining single-tx validity
(and its phase-2 Plutus half) is B2's job, attaching by tightening the
body authority, not the composer. **Known extension point (pre-Babbage):**
`decode_block` only builds Praos `HeaderInput`s for Babbage/Conway; a
pre-Babbage block returns a typed `unsupported pre-Babbage era` reject
rather than guessing a TPraos projection — extending to TPraos full
blocks attaches at `decode_block`.

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

### Stake-snapshot projection for consensus (B1)

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **Trait boundary** | `ade_core::consensus::ledger_view::LedgerView` | BLUE | The closed 4-method surface BLUE consensus consults for stake snapshots. **Refined in B1:** `pool_vrf_keyhash(epoch, pool) -> Option<Hash32>` (was `pool_vrf_key` returning the vkey). The ledger holds the keyhash; the vkey arrives in the header; header validation binds the two by checking `blake2b_256(header.vrf_vkey) == pool_vrf_keyhash`. |
| **Production projection** | `ade_ledger::consensus_view::PoolDistrView` | BLUE | The leadership-relevant projection of a `LedgerState`'s pool-distribution (`nesPd` / `stakeDistrib.unPoolDistr`). Single-epoch: answers `None` for any epoch but the one it was built for, so a caller can never silently consume the wrong snapshot. `BTreeMap` only; no I/O; no rederivation. The first **production** `LedgerView` impl. |
| **Test stub** | `ade_testkit::consensus::ledger_view_stub::LedgerViewStub` | GREEN | The pre-B1 stub; still used by N-B integration tests. |

**Rule.** `LedgerView` remains a **closed trait, not a plugin point**.
B1 added the first production impl (`PoolDistrView`) alongside the
existing GREEN stub; the trait is still expected to have a small, fixed
set of impls (production + test), never an open registry. The B1
refinement (`pool_vrf_keyhash` returning the *hash*, not the vkey) is a
strengthening: the ledger surfaces the registered keyhash and header
validation does the binding, so BLUE consensus never has to trust a
vkey it was handed. **This is the surface where a future
LedgerState-backed `PoolDistrView` constructor attaches** — at HEAD
`PoolDistrView::new` is fed already-frozen B1 corpus data; a B4-style
sync slice will build it directly from a parsed `LedgerState` while
keeping the exact same trait shape. RED shells must not call BLUE
consensus with a hand-rolled `LedgerView` that bypasses ledger
semantics.

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
| **Stake-snapshot boundary** | `ade_core::consensus::ledger_view::LedgerView` (trait, BLUE) ↔ `ade_ledger::consensus_view::PoolDistrView` (production BLUE impl) / `ade_testkit::consensus::ledger_view_stub::LedgerViewStub` (test GREEN impl) | mixed | BLUE consumes ledger-owned stake snapshots **by-reference only**; never owns, mutates, or re-derives them. See §2 "Stake-snapshot projection" above for the B1 refinement. |
| **Header admission** | `ade_core::consensus::header_validate::validate_and_apply_header` | BLUE | Single chokepoint. Composes the 10-step pipeline (forecast → monotone slot → monotone block-no → op-cert pre-check → VRF nonce → VRF leader → leader threshold → op-cert observation → nonce contribution → advance state). **B1 extended this** with KES signature verification (`kes_check.rs`) and era-correct VRF domain. Pipeline is sequential and fail-fast; no partial state. |
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
3. **`LedgerView` is a closed trait, not a plugin point.** B1 added the
   first production impl (`PoolDistrView`); RED shells must not call
   BLUE consensus with a hand-rolled `LedgerView` that bypasses ledger
   semantics.
4. **The N-B authoritative chokepoints never move.**
   `validate_and_apply_header`, `select_best_chain`, `apply_rollback`,
   and (B1) `block_validity` are the only BLUE entry points the
   orchestrator / composition root uses; new clusters add new variants
   to closed inputs, never new chokepoints.
5. **Selector and chain-dep advance in lockstep through the
   orchestrator.** Header validation always precedes fork-choice; a
   tiebreaker loss does not undo the BLUE chain-dep advance (matches
   ouroboros-consensus). Rollback restores both via a snapshot from
   the bounded ring.

### Where the boundary is enforced

- `ci_check_dependency_boundary.sh` — no BLUE crate may depend on
  `ade_runtime` or `ade_node`; `ade_network` BLUE submodules may not
  depend on RED submodules within the same crate. **B1 added the
  acyclic edge `ade_ledger → ade_core`** (both BLUE), verified
  cycle-free at cluster entry: `ade_core` depends only on
  `ade_types`/`ade_crypto`/`minicbor`.
- `ci_check_no_async_in_blue.sh` — async / tokio / futures forbidden
  in BLUE (including `ade_core::consensus` and `ade_ledger::block_validity`).
- `ci_check_no_chaindb_in_consensus_blue.sh` *(N-B)* — forbids any
  `ChainDb` / `chain_db` token in `crates/ade_core/src/consensus`.
  Strengthens DC-CORE-01 + DC-CONS-07.
- `ci_check_no_float_in_consensus.sh` *(N-B)* — forbids `f32` / `f64`
  tokens anywhere in `crates/ade_core/src/consensus`. Strengthens
  T-CORE-02 + DC-CONS-07/08/09.
- `ci_check_no_density_in_fork_choice.sh` *(N-B)* — forbids any
  `density` reference in `fork_choice.rs` / `candidate.rs` except on
  lines beginning with the audit marker `// no-density:`. Strengthens
  DC-CONS-03.
- `ci_check_consensus_closed_enums.sh` *(N-B; B1-extended)* — four
  checks (no `#[non_exhaustive]`; no open-tail `Other` / `Unknown`; no
  owned `String` in the named error/event/encoding files; no
  `Box<dyn ...>`). **B1 extended its target set to
  `crates/ade_ledger/src/block_validity`** (with `verdict.rs` and
  `encoding.rs` added to the no-`String` file list). Strengthens
  DC-CONS-04 + DC-CONS-10 + T-DET-01 (consensus) and
  DC-VAL-02/04/05/06 (block validity).
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
| **`ChainEvent`** *(N-B)* | `ade_core::consensus::events` | 5 variants — ChainExtended, RolledBack, RolledForward, ChainSelected, Rejected | The complete output taxonomy of the fork-choice + rollback transitions. **No `#[non_exhaustive]`, no `Other`, no `String`** — enforced by `ci_check_consensus_closed_enums.sh`. A new event = new variant + arms in `encode_chain_event` / `decode_chain_event` + arms in every consumer. |
| **`ChainSelectionReject`** *(N-B)* | `ade_core::consensus::events` | 4 variants — ForkBeforeImmutableTip, ExceededRollback, HeaderInvalid, TiebreakerLossKeepCurrent | The complete reject taxonomy. Closed for the same reason as `ChainEvent`. Reject reasons are flat-data so corpus comparisons are byte-stable. |
| **Consensus error families** *(N-B)* | `ade_core::consensus::errors` | 8 closed error enums — HFCError, SlotTimeError, OutsideForecastRange, HeaderValidationError, VrfCertError, OpCertCounterError, NonceEvolutionError, LeaderScheduleError | Each is flat-data, no `String`, no `Box<dyn>`. Closed taxonomies enforced by `ci_check_consensus_closed_enums.sh`. |
| **`StreamInput`** *(N-B)* | `ade_runtime::consensus::chain_selector` | 3 variants — HeaderArrival, RollBack, EpochBoundary | The **single** ingress taxonomy for the chain-selector orchestrator. New trigger = new variant + arm in `process_stream_input`. **There is no plugin-style extension.** |
| **`OrchestratorError`** *(N-B)* | `ade_runtime::consensus::chain_selector` | 2 variants — HeaderInvalid, NonceEvolution | The orchestrator's fail-fast `Err`. Structured rejects ride inside `Ok(Some(ChainEvent::Rejected { ... }))`. |
| **`DecodeError`** *(N-B)* | `ade_core::consensus::encoding` | 4 variants — Cbor(&'static str), UnknownDiscriminant, FieldCountMismatch, InvalidLength | Closed CBOR-decode error taxonomy for `decode_chain_dep_state` / `decode_chain_event`. The `Cbor` payload is `&'static str` (audit-friendly), never `String`. |
| **`GenesisParseError`** *(N-B)* | `ade_runtime::consensus::genesis_parser` | 5 variants — MalformedJson, MissingField, InvalidValue, UnknownNetwork, Hfc(HFCError) | Closed RED-side parse-error taxonomy. **`field` is `&'static str`**, so equality on errors is replay-stable. |
| **`GenesisBlob`** *(N-B)* | `ade_runtime::consensus::genesis_parser` | 4 variants — Byron, Shelley, Alonzo, Conway | Identifies which genesis JSON file an error refers to. Closed because the genesis bundle is structurally a four-tuple at v1. |
| **`NetworkMagic`** *(N-B)* | `ade_runtime::consensus::genesis_parser` | 3 const-named values — MAINNET, PREPROD, PREVIEW (held in a `u32` newtype) | Operator-facing public networks. An unknown magic produces `GenesisParseError::UnknownNetwork { magic }` — never a silent default. |
| **`LedgerView` trait** *(N-B; B1-refined)* | `ade_core::consensus::ledger_view` | 4 methods — `total_active_stake`, `pool_active_stake`, **`pool_vrf_keyhash` (was `pool_vrf_key`)**, `active_slots_coeff` | Closed-shape boundary by which BLUE consensus consults ledger-owned stake snapshots. All methods return `Option<...>`. **B1 changed `pool_vrf_key -> VrfVerificationKey` to `pool_vrf_keyhash -> Hash32`**: the ledger holds the keyhash; header validation binds it to the header-supplied vkey. **Not a plugin point** — production adds exactly one impl (`PoolDistrView`), tests add `LedgerViewStub`. |
| **`HeaderVrf`** *(N-B; surfaced at B1)* | `ade_core::consensus::header_summary` | 2 variants — Tpraos { nonce_proof, leader_proof }, Praos { proof, output } | Era-dispatched VRF model: Shelley..Alonzo are two-proof TPraos; Babbage/Conway are single combined-VRF Praos. Closed; the era selects the arm. B1's `decode_block` builds only the `Praos` arm today (Babbage/Conway corpus); the `Tpraos` arm is the documented extension point for pre-Babbage full-block validity. |
| **`BlockValidityVerdict`** *(NEW in B1)* | `ade_ledger::block_validity::verdict` | 2 variants — Valid { tip, block_no, body }, Invalid { class, error } | The verdict of the composition root. Closed; no `String`, no `Box<dyn>`, no `#[non_exhaustive]` — enforced by `ci_check_consensus_closed_enums.sh` (target extended to `block_validity`). `Eq` is omitted only because `Body(LedgerError)` is `PartialEq`-only upstream — a structural fact, not an open surface. |
| **`BlockValidityError`** *(NEW in B1)* | `ade_ledger::block_validity::verdict` | 5 variants — Header(HeaderValidationError), Body(LedgerError), BodyHashMismatch{header, actual}, MalformedField(FieldError), MissingConsensusInput(MissingInput) | The full structured reject reason. Closed. A total `class()` mapping projects it onto the coarse `BlockRejectClass`. |
| **`BlockRejectClass`** *(NEW in B1)* | `ade_ledger::block_validity::verdict` | 5 variants — HeaderInvalid, BodyInvalid, BodyHashMismatch, MalformedField, MissingConsensusInput | The **canonical/replay comparison surface** — the coarse class the reference oracle exposes. CBOR-round-trippable (discriminants 0..4 fixed in `encoding.rs`). New class = new variant + arm in `class_discriminant`/`class_from_discriminant` + corpus regeneration. |
| **`FieldKind`** *(NEW in B1)* | `ade_ledger::block_validity::verdict` | 9 variants — VkeyWitness, BootstrapKey, Ed25519Signature, VrfVkey, VrfProof, KesVkey, KesSignature, OpCertSignature, BlockBodyHash | The closed set of fixed-size fields whose length is checked fail-closed (DC-VAL-06). New fixed-size field = new variant. |
| **`FieldError`** *(NEW in B1)* | `ade_ledger::block_validity::verdict` | struct { field: FieldKind, expected, actual } | The fail-closed field-size error shape. |
| **`MissingInput`** *(NEW in B1)* | `ade_ledger::block_validity::verdict` | 4 variants — EpochNonce, SetSnapshot, PoolVrfKeyhash(Hash28), ActiveSlotsCoeff | The closed set of consensus inputs whose absence is a hard reject (`MissingConsensusInput`), not a guess. |
| **`VerdictSurface`** *(NEW in B1)* | `ade_ledger::block_validity::encoding` | 2 variants — Valid { tip, block_no }, Invalid { class } | The CBOR-round-trippable comparison surface. **The full `LedgerError`/`HeaderValidationError` detail is deliberately NOT encoded** — only the coarse oracle-aligned surface is byte-stable (T-DET-01). Mirrors `ade_core::consensus::encoding` (definite-length arrays, `[discriminant, ...payload]`). |
| **`SurfaceDecodeError`** *(NEW in B1)* | `ade_ledger::block_validity::encoding` | 3 variants — Cbor(&'static str), UnknownDiscriminant{for_enum, found}, FieldCount{expected, actual} | Closed decode-error taxonomy for the verdict surface. No `String`, no `Box<dyn>`, no `#[non_exhaustive]`. |
| **`block_validity` chokepoint** *(NEW in B1)* | `ade_ledger::block_validity::transition` | 1 function | The single composition root joining header + body authorities. Does not move; gains no second public entry; introduces no validation rules (DC-VAL-02). |
| **Verdict-surface encode/decode chokepoints** *(NEW in B1)* | `ade_ledger::block_validity::encoding::{encode_verdict_surface, decode_verdict_surface}` | 2 functions | The frozen CBOR for the comparison surface. Round-trip required; field/discriminant additions are version-gated. |
| **`PraosNonces` / `NonceScanError`** *(NEW in B1)* | `ade_ledger::consensus_input_extract` | 1 struct (5 nonces) + 1 error (NotFiveNonces) | The consensus-input extraction shape. The exact-five-nonce requirement is a closure invariant; a different count is a version-gated capture-format change. |
| **`PraosChainDepState` canonical encoding** *(N-B)* | `ade_core::consensus::encoding::{encode_chain_dep_state, decode_chain_dep_state}` | 2 chokepoints | The frozen CBOR encoding for the consensus chain-dep state. Round-trip required (T-DET-01); any field addition is a version-gated bump plus corpus regeneration. |
| **`ChainEvent` canonical encoding** *(N-B)* | `ade_core::consensus::encoding::{encode_chain_event, decode_chain_event}` | 2 chokepoints | Same closure discipline as `PraosChainDepState`. |
| **CI check set** | `ci/ci_check_*.sh` | 25 scripts | Existing checks may be tightened, never relaxed. New CI check is **additive**. Deleting a CI script requires recording the deprecation in the invariant registry's `ci_scripts` arrays. |
| **Invariant registry families** | `docs/ade-invariant-registry.toml` | Families T / CN / DC / OP / RO; DC extended in N-A (`DC-PROTO-*`, `DC-CORE-01`), N-B (`DC-CONS-03..10`), and **B1 (`DC-VAL-01..06`)** | Append-only IDs; rules may be strengthened, never weakened; deprecation needs an explicit `deprecated_in`. |

### Extensible (open within constraints)

| Registry | Location | Extension Rule |
|----------|----------|---------------|
| `CostModels` map (Plutus V1/V2/V3 cost tables) | `ade_plutus::cost_model::CostModels` | New entries enter via the cost-model CBOR decoder when a protocol parameter update lands. Not runtime-pluggable; constrained by the closed `PlutusLanguage` set. |
| `ProtocolParameters` / `ProtocolParameterUpdate` field set | `ade_ledger::pparams` | Fields are appended per era. Not extensible at runtime — versioned-gated by era. |
| Pool / DRep / Stake registrations | `ade_ledger::state::{DelegationState, CertState}` | Mutated at runtime by `ade_ledger::delegation::apply_cert`. The **shape** of what can be registered is closed; the **set** of registrations is open and grows monotonically. |
| Governance proposal set | `ade_ledger::state::ConwayGovState::proposals` | Same pattern — shape closed, instance set open, lifecycle managed by `evaluate_ratification` / `enact_proposals` / `expire_proposals`. |
| `OpCertCounterMap` *(N-B)* | `ade_core::consensus::praos_state` | BTreeMap keyed by `(Hash28, u64)`. Inserts are strictly increasing per `(pool, kes_period)` (returns `OpCertCounterError::Regression` on attempt). Shape closed; instance set open and grows monotonically with each accepted header. |
| `PoolDistrView` pool table *(NEW in B1)* | `ade_ledger::consensus_view::PoolDistrView::pools` | `BTreeMap<Hash28, PoolEntry>`. The **shape** (one `PoolEntry { active_stake, vrf_keyhash }` per pool) is closed; the **set** of pools is open and is whatever the operating-epoch snapshot contains. Built once per epoch via `PoolDistrView::new` from already-frozen data; single-epoch (queries for any other epoch return `None`). Not runtime-pluggable. |
| `RollbackSnapshot` ring *(N-B)* | `ade_runtime::consensus::chain_selector::OrchestratorState::recent_snapshots` | Bounded `Vec<RollbackSnapshot>` capped at `DEFAULT_SNAPSHOT_LIMIT = 2160` (mainnet k); pushes appended, oldest dropped on overflow. Configurable via `with_snapshot_limit` for tests. No plugin extension. |
| Oracle reference snapshots / regression corpus | `ade_testkit::harness::{snapshot_loader, regression_corpus}` | Tooling-only. New oracle data added via `corpus/` directory plus a manifest update. `ci_check_ref_provenance.sh` enforces manifest checksum integrity. GREEN. |
| Network corpus (mini-protocol transcripts) | `corpus/network/{n2n,n2c}/*` | Tooling-only. Captured via the `ade_network::bin::capture_*` tools, then committed as deterministic fixtures. Append-only by convention; provenance recorded alongside the corpus. |
| Consensus corpus | `corpus/consensus/{hfc_schedule, nonce_evolution, op_cert, leader_schedule, fork_choice, rollback}/` | Tooling-only. Append-only by convention. Each replay test reads a fixed sub-tree; new test = new fixture file + new corpus entry. |
| Block-validity corpus *(NEW in B1)* | `corpus/validity/{conway_epoch576, adversarial}/` | Tooling-only. Positive (real on-chain blocks) + adversarial (mutator-derived). Both replay byte-identically (T-DET-01, DC-VAL-04). New positive entry = new captured block; new adversarial entry = new deterministic mutator (M1–M6 today). Append-only by convention. GREEN harness in `ade_testkit::validity`. |
| `KillStrategy<D>` trait impls | `ade_runtime::chaindb::crash_safety` | RED-only test infrastructure. New strategies plug in via the trait; `NoKill` is the production default. Object-safe by intent. |
| Recovery state types | callers of `Recoverable` | Open: any state type with a canonical encode + apply-block step can be recovered. The trait is the only way in; no central registry of state types. |
| Pinned external crates | `crates/*/Cargo.toml` | New external crate addition requires a Tier-5 rationale doc (per `docs/active/CE-79_tier5_addendum.md`). |

### Candidates — extensible surfaces not yet wired

| Cluster | Candidate registry | Rationale |
|---------|-------------------|-----------|
| B2 | Per-tx verdict / reject-class taxonomy | Tx validity. Likely a closed enum paralleling `BlockRejectClass`, attaching to the body authority. |
| N-A (deferred) | Peer address book | Operator-supplied; runtime mutable. Should live in `ade_runtime`. |
| N-C | Block-production policy (forge cadence, KES rotation, slot election) | Tier 1 semantics, Tier 5 operator triggers. Forge inputs must reduce to the existing `BlockEnvelope` chokepoint. |
| N-E | Mempool tx prioritization policy | Tier 5 — operator-tunable. Plugin trait candidate: `MempoolPolicy`. |
| N-F | Query API method set | Tier 5 wire / Tier 1 semantics. Should be a closed enum internally, mapped to gRPC / HTTP at the edge; shared with LSQ/LocalTxMonitor semantic dispatch. |
| N-F | Prometheus metric names | Tier 5; append-only registry expected. |

User confirmation needed for each at cluster entry: closed enum vs.
trait-based registry; runtime-extensible vs. compile-time-fixed; CI
enforcement shape.

### Closed-grammar audit (PHASE4-B1 specific)

This sweep was performed after PHASE4-B1 close. The author should
confirm each is intended-closed (no future plugin point) before any
extension is proposed:

1. `BlockValidityVerdict` / `BlockValidityError` / `BlockRejectClass` /
   `FieldKind` / `FieldError` / `MissingInput` — **closed by intent**.
   Mechanically enforced by `ci_check_consensus_closed_enums.sh`
   (target extended to `ade_ledger::block_validity`). New
   verdict/reject/field = new variant + arms everywhere it surfaces +
   corpus regeneration.
2. `VerdictSurface` / `SurfaceDecodeError` — **closed by intent**. The
   surface is deliberately *coarse*: it encodes only `(tip, block_no)`
   for Valid and the reject `class` for Invalid; the rich
   `LedgerError`/`HeaderValidationError` detail is debug-only and NOT
   part of the canonical bytes. This is the project's "wire vs.
   semantic" split applied to the verdict.
3. `HeaderVrf` {Tpraos | Praos} — **closed by intent**, era-dispatched.
   B1 wires only the `Praos` arm (Babbage/Conway). The `Tpraos` arm is
   a *documented extension point*, not an accidental gap — pre-Babbage
   full-block validity attaches at `block_validity::decode_block`.
4. `LedgerView` (refined `pool_vrf_keyhash`) — **closed by intent**.
   Production impls: `PoolDistrView` (B1, BLUE) + `LedgerViewStub`
   (GREEN, tests). The refinement strengthens the trust boundary (ledger
   surfaces the keyhash; header validation binds the vkey).
5. `PraosNonces` / `NonceScanError` — **closed by intent**, fail-closed
   at exactly five nonces.

**Candidate flag — body authority depth.** `block_validity`'s `Body`
verdict is exactly what `apply_block_with_verdicts` produces today.
Whether the body authority's witness/script depth is "complete enough"
for Conway is a B2 question — the **Conway body-witness depth is the
documented extension point for B2** (tx validity). The composition root
does not need to change for B2; B2 tightens the body authority it
composes. Confirm at B2 entry that the per-tx verdict shape parallels
`BlockValidityVerdict` rather than introducing a divergent taxonomy.

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
- **Era-correct block body hash** *(wired at B1)*: for Alonzo+ (segwit)
  the body hash is `blake2b_256( H(tx_bodies) ‖ H(witness_sets) ‖
  H(aux_data) ‖ H(invalid_txs) )` computed over the **preserved CBOR
  segment bytes** (never re-encoded — T-ENC-01). The body-hash binding
  in `block_validity` pivots on this.
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
- **`BootstrapAnchorHash` v1 preimage** *(N-B)*: Blake2b-256 over
  `b"ade_bootstrap_v1" || canonical_cbor([byron, shelley, alonzo,
  conway])`. The domain tag, the four-element ordering, the canonical
  CBOR encoding, and the hash algorithm are all frozen for v1.
- **`EraSchedule` invariants** *(N-B)*: monotonically increasing
  `start_slot`, non-empty era list, non-zero `slot_length_ms` and
  `epoch_length_slots`. Enforced inside `EraSchedule::new`.
- **`PraosChainDepState` CBOR encoding** *(N-B)*: frozen for the
  protocol version it serializes; round-trip via
  `decode_chain_dep_state` is byte-identical (T-DET-01).
- **`ChainEvent` / `ChainSelectionReject` closed taxonomies** *(N-B)*:
  variant set frozen for the protocol version; rejects compare
  byte-for-byte across replays.
- **Consensus error taxonomies** *(N-B)*: all flat-data, all
  `String`-free, `Box<dyn>`-free, replay-stable.
- **`StreamInput` 3-variant taxonomy** *(N-B)*: HeaderArrival, RollBack,
  EpochBoundary. New ingress = new variant (version-gated).
- **`HeaderVrf` era model** *(N-B)*: two arms (Tpraos / Praos), the era
  selects the arm; the proof-count and binding rule per arm are
  protocol-fixed.
- **`block_validity` composition contract** *(NEW in B1)*: `Valid` iff
  header authority ∧ body-hash binding ∧ body authority all accept
  (DC-VAL-02); header-before-body fail-fast ordering (DC-VAL-03); no
  partial mutation on the invalid path (DC-VAL-05). The composer is
  pure, total, deterministic (DC-VAL-01) and adds no rules of its own.
- **`VerdictSurface` CBOR encoding** *(NEW in B1)*: `Valid -> [0, tip,
  block_no]`, `Invalid -> [1, reject_class_discriminant]`; definite-
  length arrays; `BlockRejectClass` discriminants 0..4 fixed. Only the
  coarse class is encoded; the full error is never part of the bytes.
  Round-trip via `decode_verdict_surface` is byte-identical (T-DET-01).
- **`LedgerView` trait shape** *(N-B; B1-refined)*: 4 methods, all
  `Option`-returning. B1 froze `pool_vrf_keyhash -> Hash32` (the keyhash,
  not the vkey) as the registered-VRF surface; header validation binds
  the header vkey to it.
- **All canonical types**: shapes are frozen at the era / version they
  entered. Adding fields requires a versioned gate; renaming is
  forbidden.
- **TCB color assignments**: per `.idd-config.json` `core_paths`.
  `ade_core::consensus` and `ade_ledger::block_validity` /
  `ade_ledger::consensus_view` are BLUE; `ade_ledger::consensus_input_extract`
  is pure-over-bytes "RED behavior" living inside the BLUE
  `ade_ledger` crate (see §1 candidate flag); `ade_runtime::consensus`
  is RED; `ade_testkit::{consensus, validity}` is GREEN;
  `ade_core_interop` is RED.
- **`ChainDb` / `SnapshotStore` / `Recoverable` trait shapes** (N-D
  closed): the trait method sets are frozen.

### Version-gated (can evolve across major versions)

- **New `CardanoEra` variant**: requires new `decode_*_block`
  chokepoint, new per-era composer in `ade_ledger`, new hfc translation
  arm, new addition to `CardanoEra::ALL`, extension of the
  named-chokepoint header in `ci_check_ingress_chokepoints.sh`, and
  extension of the `later_eras` table in `parse_genesis`.
- **TPraos full-block validity** *(B1 extension point)*: extending
  `block_validity::decode_block` to build a `HeaderVrf::Tpraos`
  `HeaderInput` for pre-Babbage eras (today it returns a typed
  `unsupported pre-Babbage era` reject). Requires a TPraos VRF-domain
  projection + a pre-Babbage positive/adversarial corpus.
- **New `GovAction` / `ConwayCert` / Plutus version variant**: requires
  registry diff (§3) plus arms in every chokepoint.
- **New protocol parameter field**: append to `ProtocolParameters`;
  CBOR field-order discipline preserved by `ade_codec`.
- **New CI check**: additive. Removing an existing check requires
  invariant-registry deprecation note.
- **Pinned external crate bump**: Tier-5 rationale doc required.
- **New mini-protocol**: new module under `ade_network/` with a closed
  enum, new chokepoint pair, new transition function, and new `*Version`
  enum. Never an arm on an existing enum.
- **Mini-protocol version-table bump**: each `*Version` enum may grow
  by appending a higher variant when a new cardano-node release pins a
  new version.
- **New `ChainEvent` / `ChainSelectionReject` variant** *(N-B)*:
  requires bumping the events envelope version, adding encode/decode
  arms, adding the orchestrator dispatch arm, and regenerating the
  consensus corpus.
- **New `StreamInput` variant** *(N-B)*: requires extension of every
  call-site of `process_stream_input` plus a new corpus suite.
- **New `NetworkMagic`** *(N-B)*: requires the `parse_genesis` match arm
  + a new operator-supplied boundary table + a normative note.
- **New `LedgerView` impl / LedgerState-backed `PoolDistrView`
  constructor** *(N-B / B1)*: requires a slice that wires the impl into
  the call sites and a corpus showing equivalent observable behavior.
  The B4 sync path attaches here — building `PoolDistrView` directly
  from a parsed `LedgerState` while keeping the trait shape.
- **`BootstrapAnchorHash` preimage v2** *(N-B)*: hard version-gated.
- **New `BlockRejectClass` / `FieldKind` / `MissingInput` variant**
  *(B1)*: requires arms in `BlockValidityError::class`, arms in the
  verdict-surface encode/decode discriminant maps, and a regenerated
  block-validity corpus (positive + adversarial).
- **B2 per-tx validity** *(B1 extension point)*: tightens the body
  authority `apply_block_with_verdicts` (and its phase-2 Plutus half);
  the `block_validity` composer does not change. New per-tx verdict
  taxonomy should parallel `BlockValidityVerdict`.
- **Phase-4 cluster surface additions** (N-C, N-E, N-F): each cluster's
  wire surface gates additions via its own cluster doc.

---

## 5. Module Addition Rules

Ade's workspace is small and color-disciplined. New modules enter as
new crates under `crates/`, or as new BLUE submodules within an existing
BLUE crate. `ade_network` is the first BLUE crate with **per-submodule**
color assignment; `ade_runtime` is mixed (RED in `chaindb` + `recovery`,
GREEN in `consensus::candidate_fragment` / `consensus::chain_selector`,
RED in `consensus::genesis_parser`). **B1 added the first BLUE→BLUE
cross-crate dependency edge (`ade_ledger → ade_core`)**: the composition
root lives in `ade_ledger` and reaches up into `ade_core::consensus` for
the header authority. The edge was verified acyclic at cluster entry
(`ade_core` depends only on `ade_types`/`ade_crypto`/`minicbor`).

| Color | Naming convention | Build-config flags | May depend on | MUST NOT depend on |
|-------|-------------------|--------------------|----------------|--------------------|
| **BLUE** | `ade_*` (no color in name) | First line of every `.rs` is the contract banner `// Core Contract:`. `lib.rs` carries `#![deny(unsafe_code)]`, `#![deny(clippy::unwrap_used/expect_used/panic/float_arithmetic)]`. No `#[cfg(feature = ...)]`. No async (DC-CORE-01). No `ChainDb` reference inside `ade_core::consensus`. No `f32`/`f64` inside `ade_core::consensus`. No density-ordering term inside `ade_core::consensus::fork_choice`/`candidate`. No `#[non_exhaustive]`/open-tail/`String`/`Box<dyn>` in `ade_core::consensus` **and `ade_ledger::block_validity`**. | Other BLUE crates / submodules only (incl. the new `ade_ledger → ade_core` edge) | Any RED submodule or crate; GREEN in non-dev deps; `pallas_*` (except `ade_plutus`); async runtime; `HashMap`/`HashSet`/`IndexMap`; clock/rand/float/env/I/O. |
| **GREEN** | `ade_*` | Banner + deny attrs are project convention but not currently enforced for `ade_testkit` / `ade_network::mux::mod`. May use `HashMap`/`serde_json`/`flate2`/`tar` for fixture I/O. `ade_runtime::consensus::chain_selector` is GREEN but lives in `ade_runtime` for dep convenience. | BLUE crates + standard library + ecosystem crates | `ade_runtime` (for `ade_testkit`); RED submodules in non-test paths. Results must never feed back into BLUE state. |
| **RED** | `ade_*` | No special header. Free to use clocks, I/O, async, `HashMap`, signing keys (`ade_runtime` is the only crate that may sign). | Any BLUE / GREEN crate or submodule (one-way) | Cannot be depended on by BLUE (`ci_check_dependency_boundary.sh`, `ci_check_no_async_in_blue.sh`). |

### New module checklist

1. **Add to `Cargo.toml` workspace members.** `version = "0.1.0"`,
   `edition = "2021"`.
2. **Declare TCB color** by editing `.idd-config.json` `core_paths` if
   BLUE; if the crate is mixed-color, name each BLUE submodule path and
   ensure the BLUE CI scripts scan the submodule subset.
3. **CI script update obligations** — extend the relevant BLUE-scoped
   scripts (`ci_check_module_headers.sh`, `ci_check_no_semantic_cfg.sh`,
   `ci_check_no_signing_in_blue.sh`, `ci_check_hash_uses_wire_bytes.sh`,
   `ci_check_ingress_chokepoints.sh`, `ci_check_dependency_boundary.sh`,
   `ci_check_no_async_in_blue.sh`, `ci_check_forbidden_patterns.sh`).
   For consensus-shaped additions, also extend
   `ci_check_no_chaindb_in_consensus_blue.sh`,
   `ci_check_no_float_in_consensus.sh`,
   `ci_check_consensus_closed_enums.sh` (whose `TARGETS` array already
   covers both `ade_core::consensus` and `ade_ledger::block_validity` —
   add new closed-taxonomy paths here), and (if a fork-choice surface
   is touched) `ci_check_no_density_in_fork_choice.sh`.
4. **Add contract banner** (BLUE) to every `.rs` file.
5. **Add deny attributes** to `lib.rs` (BLUE).
6. **New canonical types:** at HEAD the canonical-type registry is
   inline in the invariant registry (`canonical_type_registry: null`) —
   add a `[[rules]]` block under family `T`, plus a round-trip test in
   the rule's `tests` array.
7. **Run `cargo test --workspace` and the full CI script suite.** Both
   must be green before the cluster can close.

### Phase 4 anticipated additions

- **B2 (tx validity)**: tightens the body authority
  (`apply_block_with_verdicts` per-tx machinery + phase-2 Plutus) inside
  `ade_ledger`; a `tx_validity`-shaped chokepoint may join it. The
  `block_validity` composer does not change. Per-tx verdict taxonomy
  should parallel `BlockValidityVerdict`.
- **B4 / sync — LedgerState-backed `PoolDistrView`**: a constructor that
  builds `PoolDistrView` from a parsed `LedgerState` (today it is fed
  already-frozen B1 corpus data via `PoolDistrView::new`). Lives in
  `ade_ledger` (BLUE); keeps the `LedgerView` trait shape unchanged.
- **header→body bridge**: the `ade_node` composition layer that joins
  `process_stream_input` (header fork-choice) and `block_validity`
  (full-block decision on the fetched body). Likely RED glue.
- **N-C (forge)**: forge-block path likely in `ade_runtime` (RED) for
  KES / VRF signing; must call into `ade_ledger` for canonical
  validation. Reduction target is the existing `BlockEnvelope`
  chokepoint; the forged header re-enters via `StreamInput::HeaderArrival`.
- **N-E (mempool)**: likely a new `ade_mempool` BLUE crate (canonical
  tx admission) with a RED operator shim in `ade_runtime`.
- **N-F (operator API)**: thin RED layer mapping a closed Query enum to
  gRPC/HTTP; shares semantic dispatch with N-A's LSQ / LocalTxMonitor
  opaque-bytes payloads.

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
- No `f32`, `f64`, floating-point arithmetic — `#![deny(clippy::float_arithmetic)]`
  plus the pattern script; `ci_check_no_float_in_consensus.sh` narrows
  this to `ade_core::consensus`.
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
  `ci_check_ingress_chokepoints.sh` Check 3. (Note: `block_validity`'s
  `decode_block` reads CBOR via the `ade_codec` primitive set
  (`read_array_header`, `read_bytes`, `skip_item`) and the per-era
  `decode_*_block` chokepoints — it never constructs `PreservedCbor`
  itself.)
- No `pallas_*` reference outside `ade_plutus` —
  `ci_check_pallas_quarantine.sh`.
- **(N-A specific)** No `Box<dyn Codec>` / `Box<dyn Protocol>` /
  `#[non_exhaustive]` on mini-protocol message enums; no generic
  `Codec<P>` trait.
- **(N-A specific)** No reading of "selected protocol version" from a
  session global inside a transition function (DC-PROTO-06).
- **(N-A specific)** No decoding of block CBOR, tx CBOR, or address
  CBOR inside `ade_network`.
- **(N-B specific)** No `ChainDb` / `chain_db` token inside
  `ade_core::consensus` — `ci_check_no_chaindb_in_consensus_blue.sh`.
- **(N-B specific)** No density-based ordering in caught-up Praos
  fork-choice — `ci_check_no_density_in_fork_choice.sh`.
- **(N-B specific)** No `#[non_exhaustive]`, no open-tail `Other` /
  `Unknown`, no owned `String`, no `Box<dyn>` anywhere in
  `ade_core::consensus` — `ci_check_consensus_closed_enums.sh`.
- **(N-B specific)** No body inspection for fork-choice tip comparison.
  Block-number + `TiebreakerView` only.
- **(N-B specific)** No stake-snapshot rederivation inside BLUE
  consensus — consume `&dyn LedgerView` only (DC-CONSENSUS-02).
- **(N-B specific)** No plugin-style runtime registration of consensus
  protocols. `StreamInput`, `ChainEvent`, `ChainSelectionReject` closed.
- **(B1 specific)** No `#[non_exhaustive]`, no open-tail `Other` /
  `Unknown`, no owned `String`, no `Box<dyn>` anywhere in
  `ade_ledger::block_validity` — `ci_check_consensus_closed_enums.sh`
  (target set extended; `verdict.rs` and `encoding.rs` in the
  no-`String` file list). Every reject is a structured
  `BlockValidityError`; the canonical surface is the coarse
  `BlockRejectClass` only.
- **(B1 specific)** No `Valid` verdict that skips either authority — a
  block is `Valid` iff header ∧ body both accept (DC-VAL-02). No
  "trust the body / skip header" shortcut may leak from the
  follow-bridge into the BLUE verdict.
- **(B1 specific)** No body validation on a header-invalid block —
  header is decided first, fail-fast (DC-VAL-03).
- **(B1 specific)** No partial / in-place mutation on the invalid path
  — `block_validity` returns the input states unchanged on any Invalid
  outcome (DC-VAL-05).
- **(B1 specific)** No fail-open length/size guard
  (`if X.len() == K { check } else { skip }`) on any authority path —
  size checks go through a helper that returns a `FieldError`
  (`expect_size` / `expect_array::<N>`); no defined-but-unwired or
  tautological guard (DC-VAL-06).
- **(B1 specific)** No re-encoding for the block body hash — it is
  computed over the preserved CBOR segment bytes (T-ENC-01).
- **(B1 specific)** No encoding of the full `LedgerError` /
  `HeaderValidationError` into the canonical comparison surface — only
  the coarse `BlockRejectClass` is byte-stable; the rich error is
  debug-only.
- **(B1 specific)** No silent fallback on a missing consensus input —
  an absent nonce / snapshot / pool-vrf-keyhash / asc is a hard
  `MissingConsensusInput` reject, and the nonce extractor fails closed
  at exactly five nonces.

### GREEN (`ade_testkit` incl. `validity`, `ade_network::lib` / `mux::mod`, `ade_runtime::consensus::{candidate_fragment, chain_selector}`)

- No nondeterminism that leaks into stored fixtures — fixtures must be
  byte-reproducible (the block-validity corpus replays identically).
- No participation in authoritative outputs. The B1 validity harness
  (`ade_testkit::validity`) only *drives* `block_validity` and asserts;
  the mutators (M1–M6) are deterministic transforms over real corpus
  blocks.
- No `HashMap` even in test helpers — `BTreeMap` only.
- No import of `ade_runtime` from `ade_testkit`.
- No inbound dep from any RED crate (for `ade_testkit` /
  `ade_network::lib` / `mux::mod`).
- (`ade_runtime::consensus::chain_selector` specifically) No comparison
  decision; defer to BLUE `select_best_chain` / `apply_rollback` /
  `validate_and_apply_header`.

### RED (`ade_runtime`, `ade_node`, `ade_network::mux::transport`, `ade_network::session`, `ade_network::bin::capture_*`, `ade_runtime::consensus::genesis_parser`, `ade_core_interop`, and the RED-behavior `ade_ledger::consensus_input_extract` scan)

- No direct mutation of `ade_ledger` state — all transitions go through
  `ade_ledger::rules::*` (and the `block_validity` composer).
- No bypassing `ade_codec` to construct semantic types from raw bytes.
- (`ade_runtime` specifically) No dep on `ade_ledger` — bytes-in /
  bytes-out only (S-36).
- (`ade_runtime` specifically) No leakage of `redb` types through the
  `chaindb::*` public surface (S-34).
- No second public `chaindb` path — the trait is the only surface.
- No automatic snapshot pruning — operator-driven only (S-35, S-36).
- No partial-recovery success — mid-replay failure aborts (S-36).
- No async recovery surface — sync only (S-36).
- (`ade_network::mux::transport` specifically) No protocol logic;
  bearer I/O only.
- (`ade_network::session` specifically) Composition glue only.
- (`ade_network::bin::capture_*` specifically) Live-interop tools only;
  never linked into the node binary.
- (`ade_runtime::consensus::genesis_parser` specifically) No
  re-derivation of the bootstrap anchor outside `compute_anchor_hash`,
  and no BLUE re-consumption of the JSON bytes.
- (`ade_ledger::consensus_input_extract` specifically) The nonce
  tail-scan parses an external dump format (RED behavior) but stays
  pure-over-bytes and fail-closed; it must never gain I/O, a clock, or a
  heuristic "best-effort" nonce pick. If a future capture introduces
  real I/O, the loading half moves to `ade_runtime`/testkit; only the
  pure scan stays in `ade_ledger`.
- (`ade_core_interop` specifically) Live-interop driver only; tests are
  `#[ignore]`-gated; outputs are operator evidence captures.

### Project-specific additions

- **No commits of credentials, hostnames, IPs, private keys** —
  public-repo discipline; enforced by `ci_check_no_secrets.sh`.
- **No `Phase 4 internal-mode mock network`** — Tier 1 surfaces must be
  exercised against real cardano-node 11.0.1 (N-A) / cardano-node
  10.6.2 (N-B) peers. B1's positive corpus is real on-chain Conway-576
  blocks; the adversarial corpus is mutator-derived from real blocks.
- **No collapsing wire and canonical bytes** — dual-authority rule.
- **No Tier 5 surface without a stated rationale** — divergence from
  cardano-node requires naming "what's better" per
  `docs/active/CE-79_tier5_addendum.md`.
- **No "we'll match it later" stubs on Tier 1 surfaces** — Tier 1
  closure is hard-gated. The B1 verdict is a Tier-1 surface.

---

## Cross-references

- CODEMAP: `docs/ade-CODEMAP.md` — module-by-module authority table,
  upstream of this document. **Cross-reference warning:** at the
  generation SHA the CODEMAP header reads `b9ff041` (pre-B1); its
  `ade_ledger` "Outbound deps" do not yet list `ade_core`, and its
  `ade_ledger` MUST-NOT item (8) still says the `LedgerView` impl is
  owed by a future cluster. Both are now stale — B1 added the
  `ade_ledger → ade_core` edge and shipped `PoolDistrView`. Regenerate
  CODEMAP (`/codemap`) to refresh.
- Invariant registry: `docs/ade-invariant-registry.toml` — rule
  families incl. `T`, `CN`, `DC` (with `DC-PROTO-*` + `DC-CORE-01`
  under N-A, `DC-CONS-03..10` under N-B, and `DC-VAL-01..06` under B1),
  `OP`, `RO`.
- Phase 4 cluster plan: `docs/active/phase_4_cluster_plan.md`.
- Tier doctrine: `docs/active/CE-79_gate_statement.md` and
  `docs/active/CE-79_tier5_addendum.md`.
- Cluster N-D slices (closed):
  `docs/clusters/completed/PHASE4-N-D/S-{33..37}.md`.
- Cluster N-A (closed): `docs/clusters/completed/PHASE4-N-A/cluster.md`
  + `S-A{1..10}.md`.
- Cluster N-B (closed): `docs/clusters/PHASE4-N-B/cluster.md` +
  `S-B{1..10}.md`.
- Cluster B1 (closed): `docs/clusters/PHASE4-B1/cluster.md` +
  `B1-S{1..7}.md`. Planning trio:
  `docs/planning/phase4-b1-invariants.md`,
  `docs/planning/phase4-b1-cluster-slice-plan.md`.
- N-A live-interop evidence: `docs/active/CE-N-A-5_evidence.toml`.
