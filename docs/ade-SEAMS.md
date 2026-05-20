# Seams — Where New Work Can Attach (Ade)

> **Status:** Living architectural document. Regenerated; not hand-edited.
> Per-project instance of `~/.claude/methodology/templates/seams.md`.

> 11 crates, 25 CI checks at HEAD (`85a50dc`).
> Reads CODEMAP for the module list and TCB colors; reads the invariant
> registry (`docs/ade-invariant-registry.toml`) for rule IDs; reads the
> Phase 4 cluster plan (`docs/active/phase_4_cluster_plan.md`), the
> closed N-D / N-A / N-B / B1 cluster docs, and the just-closed
> PHASE4-B2 cluster doc plus its slices
> (`docs/clusters/PHASE4-B2/cluster.md`,
> `docs/clusters/PHASE4-B2/B2-S{1..5}.md`).

Ade is a Cardano block-producing node. Its closure surface is dominated
by two facts:

1. The Cardano protocol fixes wire bytes and hashes for hash-critical
   paths (Tier 1 — must-conform). New work that touches those bytes
   has essentially no degrees of freedom.
2. Everything operator-facing — storage layout, query API, telemetry,
   packaging — is Tier 5: deliberate divergence "in our own image"
   (per `docs/active/CE-79_tier5_addendum.md`).

This document names where the system opens and where it stays closed.

**PHASE4-B2 (Transaction Validity Agreement) just closed.** It added the
single-tx composition root — `tx_validity`, the per-tx parallel of B1's
`block_validity` — plus the closed era-versioned `required_signers` /
`SignerSource` enumeration (the DC-TXV-05 surface), a fail-closed witness
closure, a new closed tx verdict / reject-class / error family, a
CBOR-round-trippable tx comparison surface, and the **Tier-1 / Tier-5
mempool boundary** (`mempool::admit` over `tx_validity`, with a
must-not-affect-validity `mempool::policy` below it). Registry rules
`DC-TXV-01..05` and `DC-MEM-01/02` flipped to `enforced`. The B1
composition root (`block_validity`) and the two authorities it composes
remain the upstream context for everything B2 added.

---

## 1. Surface Reduction Rules

> External inputs reduce to canonical form before entering authoritative
> pipelines. At HEAD there are six fully-wired *external* ingress
> surfaces (block bytes, Plutus script bytes, snapshot bytes, Ouroboros
> mux frames, genesis JSON bundles, and chain-selector stream inputs),
> plus the two **internal composition roots** (`block_validity` from B1,
> `tx_validity` from B2), the **mempool admission gate** (`mempool::admit`,
> a Tier-1 surface over `tx_validity`), and the **consensus-input
> extraction surface** (snapshot `state` CBOR tail-scan from B1), plus the
> remaining surfaces named in the Phase 4 plan (forge, query API, and the
> not-yet-wired N2N/N2C tx-submission ingress that will eventually feed
> `mempool::admit`).

### Surface: Single-tx validity (composition root — wired in B2)

```
Surface: A single Conway transaction (full tx CBOR
         [body, witness_set, is_valid, aux_data]) decided against a
         LedgerState (its track_utxo flag selects partial vs. full)
Reduces to: TxValidityVerdict { Valid { tx_id, applied } |
                                Invalid { class, error } }
            (defined in `ade_ledger::tx_validity::verdict`)
Pipeline (fixed step ordering — no reorder, no shortcut):
  1. ade_ledger::tx_validity::phase1::decode_tx(tx_cbor) -> DecodedTx
     (lifts the PRESERVED body slice → tx_id = blake2b_256(body_slice),
      the witness-set slice, the typed body, the raw vkey witnesses, and
      the script-presence WitnessInfo; Conway-only today — T-ENC-01)
  2. ade_ledger::tx_validity::phase1::tx_phase_one(ledger, &decoded)
     (the SHARED per-tx phase-1 authority — see §2; runs the witness
      closure UNCONDITIONALLY, then the UTxO-dependent state-backed
      checks ONLY at track_utxo=true; FAIL-FAST — DC-TXV-02)
  3. phase-2 (Plutus) via plutus_eval::try_evaluate_tx — ONLY when the
     tx carries Plutus scripts (decoded.witness_info.has_plutus()); a
     phase-2 failure maps into the closed TxValidityError::Phase2
     (DC-TXV-02; phase-2 never runs on a phase-1-failed tx)
  4. Valid -> evolve the UTxO via rules::apply_conway_tx_to_utxo;
     Invalid -> the input state is returned UNCHANGED (no partial
     mutation — DC-TXV-04)
Cross-surface state sharing: none — `tx_validity` is a pure total
  function fn(&LedgerState, &[u8]) -> TxValidityOutcome. The applied
  state is threaded by value through the outcome; nothing ambient
  (no arrival order, no clock, no HashMap iteration — DC-TXV-01).
```

**Rule.** `tx_validity` is the **single per-tx composition root**, the
exact parallel of B1's `block_validity`: a transaction is `Valid` **iff**
phase-1 accepts it **and** (when it carries Plutus scripts) phase-2
accepts it (DC-TXV-02). The ordering is normative — phase-1 is decided
first, phase-2 never runs on a phase-1-failed tx (DC-TXV-02). On any
Invalid outcome the input state is returned unchanged (DC-TXV-04).
`tx_validity` introduces **no new validation rules**: it is composition
only, joining the B2-S1 witness closure, the shared `tx_phase_one`
state-backed authority, and the existing Plutus phase-2 dispatch. The
function does not move and does not gain a second public entry; new work
tightens the authorities it composes (and the body authority `block_validity`
shares), not the composer.

### Surface: Mempool admission (Tier-1 gate — wired in B2)

```
Surface: A candidate transaction offered to the mempool, against the
         mempool's accumulating LedgerState
Reduces to: AdmitOutcome { Admitted { tx_id } |
                           Rejected { class, error } }
            + a new MempoolState
            (defined in `ade_ledger::mempool::admit`)
Pipeline (fixed step ordering — no reorder, no shortcut):
  1. ade_ledger::mempool::admit(mempool, tx_cbor)
       -> (MempoolState, AdmitOutcome)
     - calls tx_validity(&mempool.accumulating, tx_cbor) — the Tier-1
       verdict. Re-validation is ALWAYS against the CURRENT accumulating
       state, never a stale snapshot, so a dependent tx (B spending A's
       output) validates once A is admitted.
     - Valid -> append tx_id to `accepted`, replace `accumulating` with
       the applied state; Admitted.
     - Invalid -> mempool returned UNCHANGED; Rejected with the same
       coarse class + structured reason tx_validity produced. NO FALSE
       ACCEPT (DC-MEM-01).
  2. ade_ledger::mempool::policy::order(mempool, OrderPolicy) -> Vec<Hash32>
     (Tier-5, GREEN behavior — a deterministic PERMUTATION over the
      already-admitted tx ids. Reads ONLY the accepted-id list; never
      calls tx_validity, never touches accumulating state, cannot change
      any admit verdict — DC-MEM-02.)
Cross-surface state sharing: the mempool's `accumulating` LedgerState is
  the only state carried across consecutive `admit` calls; it is the
  same shape `tx_validity` consumes, threaded by value.
```

**Rule.** Admission is a **thin Tier-1 gate over `tx_validity`** — its
verdict equals `tx_validity`'s verdict exactly (DC-MEM-01). The
Tier-1 / Tier-5 split is the key seam: `admit` (Tier-1, BLUE) owns the
validity decision; `policy` (Tier-5, GREEN behavior) may only reorder or
trim what Tier-1 already admitted, and is provably below it because
`order` consumes only the admitted-id list (DC-MEM-02). **No mempool
policy — eviction, prioritization, fee sorting, congestion shedding —
may move into the validity decision.** Every future mempool feature
attaches as Tier-5 below `admit`; anything that would change which txs
are valid is a Tier-1 change to `tx_validity` (and therefore to the
ledger authority it composes), not a policy knob.

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

**Rule.** `block_validity` is the **single block-level composition root**
that joins the consensus header authority and the ledger body authority.
A block is `Valid` **iff** both `validate_and_apply_header` **and**
`apply_block_with_verdicts` accept it (DC-VAL-02). The ordering is
normative: header is decided first, body never runs on a header-invalid
block (DC-VAL-03). The body-hash binding sits **between** the two
authorities (DC-VAL-02/CN-CONS-04). **No path may produce a `Valid`
verdict while skipping either authority** — the follow-bridge's RED
peer-trusted "trust the body / skip header" shortcut must not leak into
this BLUE verdict. `block_validity` introduces **no new validation
rules** (DC-VAL-02). **Relationship to `tx_validity` (B2):** the block
body authority `apply_block_with_verdicts` validates *all* of a block's
txs in their per-block context; `tx_validity` validates a *single* tx
against a standalone `LedgerState`. They **converge on the same per-tx
authorities** (the witness closure and `validate_conway_state_backed`) —
see §2 — but neither composer subsumes the other: `block_validity`
composes header ∧ body, `tx_validity` composes phase-1 ∧ phase-2.

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
**Note (B2):** `tx_validity::decode_tx` decodes a *standalone* Conway tx
CBOR via the `ade_codec` primitive set + `decode_conway_tx_body` — it
does **not** go through `decode_block_envelope` (a bare tx is not a block
envelope), and it never constructs `PreservedCbor` itself.

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
forbidden from decoding raw CBOR. **B2 note:** `tx_validity`'s phase-2
step reaches phase-2 via `plutus_eval::try_evaluate_tx`, which feeds
`eval_tx_phase_two` — it does not bypass the chokepoint.

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
fallback. **B2 note:** the `tx-submission2` (N2N) and
`local-tx-submission` (N2C) protocols carry tx bytes as opaque
`Vec<u8>`; their delivered payloads are the **future ingress to
`mempool::admit`**. That bridge is a candidate seam (see the candidate
table) — at HEAD it is unwired: B2 explicitly scoped out tx-submission
wiring (cluster doc §15), so the mempool gate is reachable only by direct
caller / test invocation, not yet from the network.

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

### Candidates — surfaces not yet wired (Phase 4 B3+, N-C, N-E, N-F)

The following surfaces are named in the Phase 4 plan / B2 planning but
have no source today. They are listed so future slice docs can attach
without reinventing the reduction step. **Each is a candidate seam
pending confirmation at cluster entry.**

| Cluster | Surface | Expected reduction target | Expected chokepoint | Confidence |
|---------|---------|---------------------------|---------------------|------------|
| B2+ / N-E | **N2N/N2C tx-submission ingest → mempool** — the RED ingress that delivers a candidate tx from the `tx-submission2` (N2N) or `local-tx-submission` (N2C) opaque-bytes payload into the Tier-1 gate | `mempool::admit(mempool, tx_cbor)` | A RED bridge (likely `ade_node` / `ade_runtime`) translating `TxSubmission2Message` / `LocalTxSubmissionMessage` delivered tx bytes into an `admit` call | candidate (B2 explicitly scoped this OUT — cluster doc §15) |
| B2+ (full tx UTxO scope) | Full-scope single-tx validity over real resolved UTxO (today the positive corpus runs at `track_utxo=false`; value/fee/input-resolution are deferred) | `TxValidityVerdict` at `track_utxo=true` over a real or synthetic UTxO | `tx_validity` (existing) — the gating already exists in `tx_phase_one`; this is corpus + state wiring, not a new chokepoint | candidate |
| B2+ (deposit/refund) | **Deposit/refund preservation-of-value** — the deferred deposit-accounting value-conservation rule (cluster doc §15 "deferred follow-up") | tightening of the body / phase-1 value-conservation authority | `validate_conway_state_backed` (existing phase-1 authority) — no new composer | candidate (named deferral in B2-S5) |
| B2+ (pre-Conway tx) | Pre-Conway single-tx validity (`tx_validity` is Conway-only today; `decode_tx` and `required_signers` return `UnsupportedEra` otherwise) | `TxValidityVerdict` via per-era body decode + per-era `SignerSource` enumeration | extend `decode_tx` + add the era arm to `required_signers` | candidate |
| B1+ (header→body bridge) | Forge/fetch bridge: a fork-choice-winning header triggers a full-block decision on the fetched body | `block_validity(...)` over the fetched body | `ade_node` composition layer joining `process_stream_input` and `block_validity` | candidate |
| B1+ (pre-Babbage block) | TPraos full-block validity (Shelley..Alonzo) | `BlockValidityVerdict` via a TPraos `HeaderInput` projection | extend `block_validity::decode_block` to build `HeaderVrf::Tpraos` headers (today it returns a typed reject for non-Babbage/Conway) | candidate |
| N-C | Forge-block inputs (mempool + state + slot + KES + VRF) | `BlockEnvelope` bytes (forged, then re-decoded for validation) | `ade_runtime::forge::forge_block` (proposed) | candidate |
| N-C | Operator block-production trigger | `StreamInput::HeaderArrival(HeaderInput)` (forged header is fed back into the same chain-selector entrypoint) | `process_stream_input` (existing) | candidate |
| N-F | LSQ semantic dispatch (LocalStateQuery payloads) | Internal Query enum (closed, not yet defined) | Single dispatch fn that consumes `LocalStateQueryMessage::Acquire/Query/Result` opaque-bytes payloads — Tier 5 wire on operator-facing gRPC/HTTP, Tier 1 semantics shared with LSQ | candidate |
| N-F | LocalTxMonitor semantic dispatch | Mempool-snapshot Query/Reply enums (over the `mempool::admit` accepted set) | Single dispatch fn that consumes `LocalTxMonitorMessage` opaque-bytes payloads | candidate |
| N-B+ | Live cardano-node session driver (for `ade_core_interop::live_consensus_session`) | `StreamInput` translated from `ade_network::chain_sync::ChainSyncMessage` and `block_fetch::BlockFetchMessage` events | Composition layer in `ade_core_interop` (currently a `ready` stub binary; the full driver is operator-side work per S-B10) | candidate |

These candidates need user confirmation when each cluster is opened:
"Is the canonical reduction target named above the right one? Does the
chokepoint name fit the project's emerging naming convention?" In
particular, the **N2N/N2C tx-submission → `mempool::admit` ingress** and
the **deposit/refund value-conservation extension** are the two seams
B2 deliberately left open and should be confirmed first at the next
mempool/tx cluster entry.

---

## 2. Data-Only vs. Authoritative Layers

Ade has eight authoritative domains. For each, a single BLUE chokepoint
holds enforcement authority; tooling layers (when they exist) live in
GREEN (`ade_testkit`) or RED (`ade_runtime`, `ade_network::mux::transport`,
`ade_network::session`, `ade_core_interop`).

### Single-tx validity — the per-tx composition root (B2)

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **Decode / projection** | `ade_ledger::tx_validity::phase1::decode_tx` | BLUE | Lifts the PRESERVED body slice (`tx_id = blake2b_256(body_slice)`), the witness-set slice, the typed `ConwayTxBody`, the raw vkey witnesses, and the script-presence `WitnessInfo`. Conway-only. Builds inputs; asserts nothing. |
| **Required-signer enumeration** | `ade_ledger::tx_validity::required_signers::{required_signers, tx_derived_required_signers}` | BLUE | The closed, era-versioned `SignerSource` enumeration (DC-TXV-05). Derives every `Hash28` a tx must have a vkey witness for, partitioned by source. `tx_derived_*` is the UTxO-free strict subset (explicit/withdrawal/cert/voter); the full function adds input/collateral payment-key sources when the UTxO is available. |
| **Witness closure** | `ade_ledger::tx_validity::witness::verify_required_witnesses` | BLUE | Fail-closed coverage: every required key hash must be covered by a witness whose Ed25519 signature over the PRESERVED body hash verifies (DC-VAL-06 / CN-LEDGER-09). Wrong-size key/sig → `MalformedWitnessField`; an extra irrelevant witness never substitutes. |
| **Shared per-tx phase-1** | `ade_ledger::tx_validity::phase1::tx_phase_one` | BLUE | The single per-tx phase-1 authority. Composes the witness closure (run UNCONDITIONALLY) + `crate::conway::validate_conway_state_backed` (the SAME state-backed authority the block loop runs, gated on `track_utxo`). Introduces no new rule. |
| **Phase-2 dispatch** | `crate::plutus_eval::try_evaluate_tx` → `ade_plutus::tx_eval::eval_tx_phase_two` | BLUE | Plutus phase-2, reached only when the tx carries Plutus scripts. The aiken `String`-bearing failure is mapped into the closed `TxValidityError::Phase2`. |
| **Composition transition** | `ade_ledger::tx_validity::transition::tx_validity` | BLUE | The single chokepoint joining phase-1 ∧ phase-2 + the UTxO evolution. `fn(&LedgerState, &[u8]) -> TxValidityOutcome`. |
| **Comparison surface** | `ade_ledger::tx_validity::encoding::{encode_tx_verdict_surface, decode_tx_verdict_surface}` | BLUE | Canonical CBOR for the **coarse** replay/oracle surface only (`TxVerdictSurface`: `Valid -> [0, tx_id]`, `Invalid -> [1, class]`). The full `TxValidityError` detail is debug-only and NOT encoded. |
| **Positive replay harness** | `ade_testkit::tx_validity::{extract, …}` | GREEN | Extracts every on-wire Conway tx from the committed Conway-576 corpus blocks and drives BLUE `tx_validity` over each (at `track_utxo=false` — partial scope); asserts byte-identical verdict streams. |
| **Adversarial harness** | `ade_testkit::tx_validity::{adversarial, valid_synthetic}` | GREEN | Family A: witness mutations on real corpus txs at `track_utxo=false`. Family B: synthetic value/input/witness mutations at `track_utxo=true`. Each mutation must map to its expected reject class — no false accept. |

**Rule.** This domain has **two phase authorities and one composer**.
New work that tightens phase-1 lands in `tx_phase_one` (and the
authorities it composes — the witness closure and
`validate_conway_state_backed`); new work that tightens phase-2 lands in
the Plutus evaluator. **The composer `tx_validity` introduces no rules of
its own and never moves** (DC-TXV-02). The verdict comparison surface is
deliberately *coarse* (`TxRejectClass`: Phase1Invalid / WitnessInvalid /
MissingRequiredSigner / Phase2Invalid / MalformedField) so corpus
comparisons against the reference node are byte-stable; the rich
structured `TxValidityError` rides alongside for debugging but is **not**
part of the canonical bytes (the same "wire vs. semantic" rib B1 applied
to `block_validity`). **The `track_utxo` boundary is a first-class seam:**
the witness closure runs unconditionally; the UTxO-dependent state-backed
checks run only at `track_utxo=true`. `track_utxo=false` is the strict
PARTIAL mode (structural + witness closure; value/fee/input-resolution
deferred) — it must NOT be read as "full validity." This mirrors the B1
block path exactly (`verify_conway_witness_closure` unconditional +
`run_phase_one_composers` gated). **Known extension points (candidates):**
deposit/refund value conservation (deferred follow-up, attaches to
`validate_conway_state_backed`), full-scope `track_utxo=true` corpus, and
pre-Conway eras (attach at `decode_tx` + `required_signers`).

### Mempool admission — the Tier-1 / Tier-5 boundary (B2)

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **Tier-1 admission gate** | `ade_ledger::mempool::admit::admit` | BLUE | A tx is admitted iff `tx_validity(accumulating, tx)` is `Valid`. Threads the accumulating `LedgerState` (base + every admitted tx); re-validates against the CURRENT state. No false accept (DC-MEM-01). |
| **Mempool state** | `ade_ledger::mempool::admit::MempoolState` | BLUE | Closed: `accepted: Vec<Hash32>` (admission order) + `accumulating: LedgerState`. The only state carried across `admit` calls. |
| **Tier-5 ordering policy** | `ade_ledger::mempool::policy::order` | GREEN behavior | A deterministic PERMUTATION over the admitted-id list (`ArrivalOrder` / `TxIdAscending`). Reads only `accepted()`; never `tx_validity`, never `accumulating`. Cannot change a verdict (DC-MEM-02). |

**Rule.** The Tier-1 / Tier-5 split is the load-bearing seam. **`admit`
owns the validity decision and is provably equal to `tx_validity`'s
verdict** (DC-MEM-01). **`policy` is provably below it** — `order` reads
only the admitted-id list, so no choice of policy can alter which txs
`admit` accepts (DC-MEM-02). Every future mempool feature (eviction,
fee prioritization, congestion shedding, size caps) attaches as Tier-5
*below* `admit`; anything that would change validity is a Tier-1 change
to `tx_validity`, not a policy knob. **No mempool policy may call
`tx_validity` or touch the accumulating state.** Both rules are
mechanically enforced by `ci_check_consensus_closed_enums.sh` (target set
extended to `ade_ledger::mempool`), which keeps `AdmitOutcome`,
`OrderPolicy`, and the verdict family closed (no `String`, no `Box<dyn>`,
no `#[non_exhaustive]`).

### Full block validity — the block-level composition root (B1)

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **Decode / projection** | `ade_ledger::block_validity::header_input::decode_block` | BLUE | Era-dispatched: reuses `decode_block_envelope` + the per-era block decoder, projects a `HeaderInput` (Praos for Babbage/Conway), recomputes the era-correct (segwit) body hash over preserved wire bytes, and records the inner-block byte range. Builds inputs; asserts nothing. |
| **Consensus header authority** | `ade_core::consensus::validate_and_apply_header` | BLUE | The header half. Decided first, fail-fast. |
| **Ledger body authority** | `ade_ledger::rules::apply_block_with_verdicts` | BLUE | The body half. Consumes the inner block, never reached on header failure. Runs `verify_conway_witness_closure` (unconditional) + `run_phase_one_composers` (track_utxo-gated) — the SAME per-tx authorities `tx_validity` converges on. |
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
comparisons against the reference node are byte-stable. **B2 sharpened
the body authority:** `apply_block_with_verdicts` and `tx_validity` now
converge on the same per-tx witness/required-signer closure
(`verify_conway_witness_closure` ↔ `tx_phase_one`'s closure) and the same
state-backed authority (`validate_conway_state_backed`); the B1
documented "Conway body-witness depth" extension point is the surface B2
filled. **Known extension point (pre-Babbage):** `decode_block` only
builds Praos `HeaderInput`s for Babbage/Conway; a pre-Babbage block
returns a typed `unsupported pre-Babbage era` reject rather than guessing
a TPraos projection — extending to TPraos full blocks attaches at
`decode_block`.

### Ledger application

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **Data-only tooling** | `ade_codec` | BLUE\* | Decodes block / tx / cert bytes into typed values, preserves wire bytes via `PreservedCbor`. **Never interprets ledger semantics.** |
| **Authoritative enforcement** | `ade_ledger` | BLUE | `rules::apply_block_with_verdicts` is the single chokepoint that produces `BlockVerdict` + new `LedgerState`; `tx_validity` is the single-tx chokepoint (B2). |
| **Loader** | `ade_runtime::chaindb` + `ade_runtime::recovery` | RED | Reads block / snapshot bytes from disk; feeds them through caller-supplied `Recoverable` impl into ledger. |

\* `ade_codec` is BLUE-data-only: it builds typed shapes but never
asserts a transition is valid. The semantic split between "this is
what the bytes say" (codec) and "this is whether the bytes are
allowed" (ledger) is the project's central design rib.

**Rule.** New work that touches ledger transitions adds enforcement
inside `ade_ledger` (typically a new composer step, or a tightening of
`apply_block_with_verdicts` / `apply_epoch_boundary_full` / the per-tx
`tx_phase_one`). New work that touches block / tx CBOR adds parse / pack
support inside `ade_codec` only. **The compilation chokepoints
(`apply_block_with_verdicts` for blocks, `tx_validity` for single txs)
never move.**

### Stake-snapshot projection for consensus (B1)

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **Trait boundary** | `ade_core::consensus::ledger_view::LedgerView` | BLUE | The closed 4-method surface BLUE consensus consults for stake snapshots. `pool_vrf_keyhash(epoch, pool) -> Option<Hash32>` (the ledger holds the keyhash; the vkey arrives in the header; header validation binds the two). |
| **Production projection** | `ade_ledger::consensus_view::PoolDistrView` | BLUE | The leadership-relevant projection of a `LedgerState`'s pool-distribution. Single-epoch; `BTreeMap` only; no I/O; no rederivation. The first **production** `LedgerView` impl. |
| **Test stub** | `ade_testkit::consensus::ledger_view_stub::LedgerViewStub` | GREEN | The pre-B1 stub; still used by N-B integration tests. |

**Rule.** `LedgerView` remains a **closed trait, not a plugin point**.
The trait is expected to have a small, fixed set of impls (production +
test), never an open registry. **This is the surface where a future
LedgerState-backed `PoolDistrView` constructor attaches** — at HEAD
`PoolDistrView::new` is fed already-frozen B1 corpus data; a B4-style
sync slice will build it directly from a parsed `LedgerState` while
keeping the exact same trait shape. RED shells must not call BLUE
consensus with a hand-rolled `LedgerView` that bypasses ledger semantics.

### Plutus phase-2 evaluation

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **Data-only tooling** | `ade_plutus::cost_model`, `ade_plutus::script_context` | BLUE | Decodes cost-model CBOR; builds the V1/V2/V3 `ScriptContext`. Does not run programs. |
| **Script ingress** | `ade_plutus::evaluator::PlutusScript::from_cbor` | BLUE | Named ingress chokepoint for Plutus script CBOR. Allowlisted in `ci_check_ingress_chokepoints.sh` Check 3 because the decoder is `aiken_uplc`/`pallas`, not `ade_codec`. |
| **Authoritative enforcement** | `ade_plutus::tx_eval::eval_tx_phase_two` | BLUE | Single entry to phase-two evaluation. Internally wraps the aiken `uplc` machine; aiken types do not leak (enforced by `ci_check_pallas_quarantine.sh`). Reached from `tx_validity` via `plutus_eval::try_evaluate_tx` (B2). |
| **Quarantine** | (the `aiken_uplc` git dep, pinned tag `v1.1.21` commit `42babe5d`) | external | Frozen at tag — never re-exported. PV11 builtins gated off (S-29). |

**Rule.** Adding a new Plutus version, builtin, or cost-model entry
requires a registry diff (see §3) plus a pinned-version bump of
`aiken_uplc`; the chokepoint `eval_tx_phase_two` does not move. No
second public entry into the evaluator is allowed; tests and the new
`tx_validity` phase-2 step use the same entry as production callers.
**No new BLUE callsite of `PlutusScript::from_cbor` may be added outside
`ade_plutus` itself** — the chokepoint exists to keep aiken-decoded bytes
inside the quarantine.

### Governance ratification / enactment (Conway)

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **Data-only tooling** | `ade_types::conway` (governance types) | BLUE | Holds `GovAction`, `GovActionState`, `DRep`, `Anchor`, `VotingProcedures` shapes. |
| **Authoritative enforcement** | `ade_ledger::governance::{evaluate_ratification, enact_proposals, expire_proposals}` | BLUE | The three chokepoints that compute Conway ratification outcomes. |

**Rule.** A new governance action variant (CIP-1694 extension) adds a
variant to `GovAction` (§3 closed registry — version-gated) **and**
arms in all three chokepoints. The CI check
`ci_check_constitution_coverage.sh` enforces the invariant-registry ↔
code coverage for governance rules. **B2 note:** governance voters are
also a `SignerSource` (`GovernanceVoter`) in the required-signer
enumeration — adding a voter credential kind touches both this domain and
`required_signers`.

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
   (currently a placeholder). The `tx-submission2` / `local-tx-submission`
   tx-bytes → `mempool::admit` bridge is a candidate seam (§1).
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
| **Stake-snapshot boundary** | `ade_core::consensus::ledger_view::LedgerView` (trait, BLUE) ↔ `ade_ledger::consensus_view::PoolDistrView` (production BLUE impl) / `ade_testkit::consensus::ledger_view_stub::LedgerViewStub` (test GREEN impl) | mixed | BLUE consumes ledger-owned stake snapshots **by-reference only**; never owns, mutates, or re-derives them. See §2 "Stake-snapshot projection" above. |
| **Header admission** | `ade_core::consensus::header_validate::validate_and_apply_header` | BLUE | Single chokepoint. 10-step pipeline + B1 KES verification + era-correct VRF domain. Sequential and fail-fast; no partial state. |
| **Best-chain authority** | `ade_core::consensus::fork_choice::select_best_chain` | BLUE | Single chokepoint. Total ordering is `(BlockNo, TiebreakerView{slot, issuer_hash, op_cert_counter, leader_vrf_output_first_8})`. Chain-length-density ordering forbidden (enforced by `ci_check_no_density_in_fork_choice.sh`). |
| **Rollback authority** | `ade_core::consensus::rollback::apply_rollback` | BLUE | Single chokepoint. k-bound + immutable-tip refusal; rejects surface as `ChainEvent::Rejected`. |
| **Candidate materialization** | `ade_runtime::consensus::candidate_fragment::build_candidate_fragment` | GREEN | Builds the `CandidateFragment` consumed by `select_best_chain`. Non-authoritative. |
| **Orchestration** | `ade_runtime::consensus::chain_selector::process_stream_input` | GREEN | Threads `StreamInput` through the BLUE pipeline; owns the bounded rollback-snapshot ring; never makes a comparison decision itself. |
| **Live-interop driver (scaffold)** | `ade_core_interop::bin::live_consensus_session` | RED | Operator-driven binary; current HEAD is a "ready" stub. Never linked into the node binary. |
| **Replay harness** | `ade_testkit::consensus::stream_replay::replay_stream` | GREEN | Test-only driver for CE-N-B-5. |

**Rule.** Five rules carry the cluster:

1. **The genesis parser is the sole RED → BLUE materialization point
   for `EraSchedule`.** No other crate may construct an `EraSchedule`
   from anything but a previously-validated one.
2. **`BootstrapAnchorHash` binds the schedule.** The v1 preimage layout
   (`b"ade_bootstrap_v1" || canonical_cbor([byron, shelley, alonzo, conway])`)
   is frozen; bumping it is a version-gated event.
3. **`LedgerView` is a closed trait, not a plugin point.**
4. **The N-B/B1 authoritative chokepoints never move.**
   `validate_and_apply_header`, `select_best_chain`, `apply_rollback`,
   `block_validity`, and (B2) `tx_validity` are the only BLUE entry
   points the orchestrator / composition roots use; new clusters add new
   variants to closed inputs, never new chokepoints.
5. **Selector and chain-dep advance in lockstep through the
   orchestrator.** Header validation always precedes fork-choice.

### Where the boundary is enforced

- `ci_check_dependency_boundary.sh` — no BLUE crate may depend on
  `ade_runtime` or `ade_node`; `ade_network` BLUE submodules may not
  depend on RED submodules within the same crate. The acyclic edge
  `ade_ledger → ade_core` (both BLUE, B1) is verified cycle-free.
- `ci_check_no_async_in_blue.sh` — async / tokio / futures forbidden in
  BLUE (incl. `ade_core::consensus`, `ade_ledger::block_validity`, and
  the new `ade_ledger::{tx_validity, mempool}`).
- `ci_check_no_chaindb_in_consensus_blue.sh` *(N-B)* — forbids any
  `ChainDb` / `chain_db` token in `crates/ade_core/src/consensus`.
- `ci_check_no_float_in_consensus.sh` *(N-B)* — forbids `f32` / `f64`
  in `crates/ade_core/src/consensus`.
- `ci_check_no_density_in_fork_choice.sh` *(N-B)* — forbids any
  `density` reference in `fork_choice.rs` / `candidate.rs`.
- `ci_check_consensus_closed_enums.sh` *(N-B; B1- and B2-extended)* —
  four checks (no `#[non_exhaustive]`; no open-tail `Other` / `Unknown`;
  no owned `String` in the named error/event/encoding/verdict files; no
  `Box<dyn>`). **B2 extended its `TARGETS` set to
  `crates/ade_ledger/src/tx_validity` and `crates/ade_ledger/src/mempool`**
  (with `tx_validity/{required_signers, witness, verdict, phase1,
  transition}.rs` and `mempool/{admit, policy}.rs` added to the
  no-`String` file list). It is the **sole CI script** carrying
  `DC-TXV-01..05` and `DC-MEM-01/02` — there is no dedicated
  `ci_check_no_fail_open_in_validation.sh` (the B1 forward-looking gate
  was never shipped; see the gap note in §3).
- `ci_check_pallas_quarantine.sh` — only `ade_plutus` may name
  `pallas_*`.
- `ci_check_no_signing_in_blue.sh` — signing patterns forbidden in BLUE;
  only `ade_runtime` may sign.
- `ci_check_ingress_chokepoints.sh` — three checks on `PreservedCbor`
  construction, named block-decoder presence, and raw-CBOR prohibition
  (with the `ade_plutus/src/evaluator.rs` allowlist).
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
| `CardanoEra` | `ade_types::era` | 8 variants (ByronEbb, ByronRegular, Shelley, Allegra, Mary, Alonzo, Babbage, Conway) | New variant = new hard fork. Coordinated change across `ade_codec`, `ade_ledger`, the canonical type list, and the genesis parser's `later_eras` table. Unknown era tags produce a `CodecError`, never a fallback. |
| `Certificate` | `ade_types::shelley::cert` | 7 variants | Frozen Shelley-era certificate set. New cert types live in `ConwayCert`. |
| `ConwayCert` | `ade_types::conway::cert` | N variants (Conway-era certificates) | Version-gated per protocol — extends but does not modify `Certificate`. |
| `GovAction` | `ade_types::conway::governance` | 7 variants | CIP-1694 fixed; new variant = CIP amendment + ratification chokepoint update. |
| `MIRPot` | `ade_types::shelley::cert` | 2 variants (Reserves, Treasury) | Frozen. |
| `DRep` | `ade_types::conway::cert` | 4 variants | CIP-1694 fixed. |
| `PlutusLanguage` | `ade_plutus::evaluator` | 3 variants (V1, V2, V3) | New variant = new Plutus version. Requires cost-model table extension + aiken bump. PV11 builtins gated off (S-29). |
| **Named ingress chokepoints (block CBOR)** | `ade_codec::{cbor::envelope, byron, shelley, allegra, mary, alonzo, babbage, conway, address}` | 10 — `decode_block_envelope`, the per-era block decoders, `decode_address` | Header comment of `ci_check_ingress_chokepoints.sh` enumerates this set. New era = new chokepoint in lockstep with a `CardanoEra` variant. Removal forbidden. |
| **Named ingress chokepoint (Plutus script CBOR)** | `ade_plutus::evaluator::PlutusScript::from_cbor` | 1 — file `crates/ade_plutus/src/evaluator.rs` | Distinct from the block-CBOR chokepoints. Allowlisted by exact file path in Check 3 of `ci_check_ingress_chokepoints.sh`. |
| **`PreservedCbor::new` constructor** | `ade_codec::preserved` | 1 chokepoint, `pub(crate)` | Construction lives inside `ade_codec`. |
| **Mini-protocol message enums** | `ade_network::codec::*` | 11 closed enums | Closed wire grammar per protocol. No `#[non_exhaustive]`, no `dyn` dispatch, no generic `Codec<P>` trait. New mini-protocol = new module + new closed enum + new chokepoint pair + new `*Version` enum + new transition. |
| **Mini-protocol encode/decode chokepoints** | `ade_network::codec::*::{encode_<protocol>_message, decode_<protocol>_message}` | 22 functions | Single chokepoint per direction per protocol. Removal/renaming forbidden (DC-PROTO-01..05). |
| **Mux frame chokepoints** | `ade_network::mux::frame::{encode_frame, decode_frame}` | 2 free functions | The **single** byte↔frame translation in the project. |
| **Mini-protocol transition functions** | `ade_network::*::transition` + `n2c::local_*::transition` | 8 state-machine modules | Each `fn (state, agency, version, msg) -> Result<...>` — pure, sync, no ambient session influence (DC-PROTO-06). |
| **Mini-protocol version enums** | `ade_network::codec::version::*` | 11 closed enums | Each pins the upper version this codec/state-machine pair has been audited against. Bumping = registry diff + new corpus + cluster doc. |
| **`ChainDb` trait surface** | `ade_runtime::chaindb::mod` | 6 methods | Object-safe; intended for multiple impls. |
| **`SnapshotStore` trait surface** | `ade_runtime::chaindb::mod` | 5 methods | Bytes opaque at this layer (S-35). |
| **`Recoverable` trait surface** | `ade_runtime::recovery` | 2 methods + 1 associated type | Caller-supplied; single error type per impl. |
| **`recover` entry point** | `ade_runtime::recovery::recover` | 1 free function | The sole composition of `ChainDb` + `SnapshotStore` + `Recoverable`. |
| **Hash domain functions** | `ade_crypto::blake2b::{block_header_hash, transaction_id, script_hash, credential_hash}` | 4 named domains | Algorithm immutable per protocol version. |
| **`ChainEvent`** *(N-B)* | `ade_core::consensus::events` | 5 variants | Complete output taxonomy of the fork-choice + rollback transitions. No `#[non_exhaustive]`, no `Other`, no `String`. |
| **`ChainSelectionReject`** *(N-B)* | `ade_core::consensus::events` | 4 variants | Complete reject taxonomy. Flat-data so corpus comparisons are byte-stable. |
| **Consensus error families** *(N-B)* | `ade_core::consensus::errors` | 8 closed error enums | Each flat-data, no `String`, no `Box<dyn>`. |
| **`StreamInput`** *(N-B)* | `ade_runtime::consensus::chain_selector` | 3 variants | The single ingress taxonomy for the chain-selector orchestrator. No plugin-style extension. |
| **`OrchestratorError`** *(N-B)* | `ade_runtime::consensus::chain_selector` | 2 variants | Fail-fast `Err`. Structured rejects ride inside `Ok(Some(ChainEvent::Rejected))`. |
| **`DecodeError`** *(N-B)* | `ade_core::consensus::encoding` | 4 variants | Closed CBOR-decode error taxonomy. `Cbor` payload is `&'static str`. |
| **`GenesisParseError`** *(N-B)* | `ade_runtime::consensus::genesis_parser` | 5 variants | Closed RED-side parse-error taxonomy. `field` is `&'static str`. |
| **`GenesisBlob`** *(N-B)* | `ade_runtime::consensus::genesis_parser` | 4 variants | Closed because the genesis bundle is structurally a four-tuple at v1. |
| **`NetworkMagic`** *(N-B)* | `ade_runtime::consensus::genesis_parser` | 3 const-named values | Unknown magic → `UnknownNetwork`, never a default. |
| **`LedgerView` trait** *(N-B; B1-refined)* | `ade_core::consensus::ledger_view` | 4 methods (`pool_vrf_keyhash -> Hash32`) | Closed-shape boundary. Not a plugin point — production adds `PoolDistrView`, tests add `LedgerViewStub`. |
| **`HeaderVrf`** *(N-B; surfaced at B1)* | `ade_core::consensus::header_summary` | 2 variants — Tpraos / Praos | Era-dispatched. B1's `decode_block` builds only `Praos` (Babbage/Conway); `Tpraos` is the documented pre-Babbage extension point. |
| **`BlockValidityVerdict`** *(B1)* | `ade_ledger::block_validity::verdict` | 2 variants | The block-validity composition verdict. Closed; enforced by `ci_check_consensus_closed_enums.sh`. |
| **`BlockValidityError` / `BlockRejectClass` / `FieldKind` / `FieldError` / `MissingInput`** *(B1)* | `ade_ledger::block_validity::verdict` | 5 / 5 / 9 / struct / 4 | Full structured reject + coarse class + closed fixed-size-field set. New class = new variant + arm in `class()` + corpus regeneration. |
| **`VerdictSurface` / `SurfaceDecodeError`** *(B1)* | `ade_ledger::block_validity::encoding` | 2 / 3 variants | CBOR-round-trippable coarse comparison surface; full error NOT encoded (T-DET-01). |
| **`block_validity` chokepoint** *(B1)* | `ade_ledger::block_validity::transition` | 1 function | The single block-level composition root. Does not move; introduces no rules (DC-VAL-02). |
| **`TxValidityVerdict`** *(NEW in B2)* | `ade_ledger::tx_validity::verdict` | 2 variants — Valid { tx_id, applied }, Invalid { class, error } | The single-tx composition verdict, paralleling `BlockValidityVerdict`. Closed; no `String`, no `Box<dyn>`, no `#[non_exhaustive]` — enforced by `ci_check_consensus_closed_enums.sh` (target extended to `tx_validity`). `Eq` omitted because `LedgerError` is `PartialEq`-only upstream (a structural fact, not an open surface). |
| **`TxRejectClass`** *(NEW in B2)* | `ade_ledger::tx_validity::verdict` | 5 variants — Phase1Invalid, WitnessInvalid, MissingRequiredSigner, Phase2Invalid, MalformedField | The **canonical/replay comparison surface** — the coarse class the reference oracle exposes. CBOR-round-trippable (discriminants 0..4 fixed in `encoding.rs`). New class = new variant + arm in `class_discriminant`/`class_from_discriminant` + corpus regeneration. |
| **`TxValidityError`** *(NEW in B2)* | `ade_ledger::tx_validity::verdict` | 5 variants — Decode(LedgerError), Witness(WitnessClosureError), Phase1(LedgerError), Phase2(LedgerError), MalformedField(FieldError) | The full structured reject reason. Closed. A total `class()` projects it onto `TxRejectClass`. |
| **`SignerSource`** *(NEW in B2 — the DC-TXV-05 surface)* | `ade_ledger::tx_validity::required_signers` | 6 variants — InputPaymentKey, ExplicitRequiredSigner, WithdrawalKey, CertificateKey, GovernanceVoter, CollateralPaymentKey | The **closed, era-versioned required-signer enumeration**. A signer source not in the enum is impossible to silently omit (incomplete enumeration is a forbidden false-accept path). New source = explicit, versioned addition + arm everywhere it is derived. |
| **`RequiredSignerError` / `RequiredSignerField`** *(NEW in B2)* | `ade_ledger::tx_validity::required_signers` | 3 / 4 variants | Closed fail-closed derivation-error taxonomy (UnresolvableInput / MalformedField / UnsupportedEra). No `String`. |
| **`WitnessClosureError` / `WitnessField`** *(NEW in B2)* | `ade_ledger::tx_validity::witness` | 3 / 2 variants — MissingRequiredSigner{key_hash, source}, InvalidWitnessSignature, MalformedWitnessField; VerificationKey/Signature | The fail-closed witness-coverage error shape. Reports WHICH `SignerSource` obligation went uncovered. No `String`. |
| **`TxVerdictSurface` / `TxSurfaceDecodeError`** *(NEW in B2)* | `ade_ledger::tx_validity::encoding` | 2 / 3 variants | The CBOR-round-trippable per-tx comparison surface (`Valid -> [0, tx_id]`, `Invalid -> [1, class]`); the full `TxValidityError` detail is NOT encoded (T-DET-01). Mirrors `block_validity::encoding`. |
| **`tx_validity` chokepoint** *(NEW in B2)* | `ade_ledger::tx_validity::transition` | 1 function | The single per-tx composition root. Does not move; gains no second public entry; introduces no validation rules (DC-TXV-02). |
| **Tx-verdict-surface encode/decode chokepoints** *(NEW in B2)* | `ade_ledger::tx_validity::encoding::{encode_tx_verdict_surface, decode_tx_verdict_surface}` | 2 functions | Frozen CBOR for the per-tx comparison surface. Round-trip required; field/discriminant additions are version-gated. |
| **`AdmitOutcome`** *(NEW in B2)* | `ade_ledger::mempool::admit` | 2 variants — Admitted { tx_id }, Rejected { class, error } | The closed Tier-1 admission outcome. Mirrors `TxValidityVerdict`. `Eq` omitted for the same structural reason. Closed — enforced by `ci_check_consensus_closed_enums.sh` (target extended to `mempool`). |
| **`MempoolState`** *(NEW in B2)* | `ade_ledger::mempool::admit` | struct { accepted: Vec<Hash32>, accumulating: LedgerState } | The closed mempool state. The only state carried across `admit` calls. |
| **`OrderPolicy`** *(NEW in B2)* | `ade_ledger::mempool::policy` | 2 variants — ArrivalOrder, TxIdAscending | The closed Tier-5 ordering-policy set. A policy is a pure projection over the admitted-id list (DC-MEM-02). New policy = new variant; may never read validity. |
| **`PraosNonces` / `NonceScanError`** *(B1)* | `ade_ledger::consensus_input_extract` | 1 struct (5 nonces) + 1 error | The consensus-input extraction shape. Exact-five-nonce requirement is a closure invariant. |
| **`PraosChainDepState` / `ChainEvent` canonical encodings** *(N-B)* | `ade_core::consensus::encoding` | 4 chokepoints | Frozen CBOR; round-trip required (T-DET-01); field additions are version-gated. |
| **CI check set** | `ci/ci_check_*.sh` | 25 scripts | Existing checks may be tightened, never relaxed. New CI check is additive. Deleting a script requires recording the deprecation in the registry's `ci_scripts` arrays. |
| **Invariant registry families** | `docs/ade-invariant-registry.toml` | Families T / CN / DC / OP / RO; DC extended in N-A (`DC-PROTO-*`, `DC-CORE-01`), N-B (`DC-CONS-03..10`), B1 (`DC-VAL-01..06`), and **B2 (`DC-TXV-01..05`, `DC-MEM-01/02`)** | Append-only IDs; rules may be strengthened, never weakened; deprecation needs an explicit `deprecated_in`. |

### Extensible (open within constraints)

| Registry | Location | Extension Rule |
|----------|----------|---------------|
| `CostModels` map (Plutus V1/V2/V3 cost tables) | `ade_plutus::cost_model::CostModels` | New entries enter via the cost-model CBOR decoder when a protocol parameter update lands. Not runtime-pluggable; constrained by the closed `PlutusLanguage` set. |
| `ProtocolParameters` / `ProtocolParameterUpdate` field set | `ade_ledger::pparams` | Fields are appended per era. Versioned-gated by era. |
| Pool / DRep / Stake registrations | `ade_ledger::state::{DelegationState, CertState}` | Mutated at runtime by `ade_ledger::delegation::apply_cert`. The **shape** of what can be registered is closed; the **set** of registrations is open and grows monotonically. |
| Governance proposal set | `ade_ledger::state::ConwayGovState::proposals` | Shape closed, instance set open, lifecycle managed by `evaluate_ratification` / `enact_proposals` / `expire_proposals`. |
| `OpCertCounterMap` *(N-B)* | `ade_core::consensus::praos_state` | BTreeMap keyed by `(Hash28, u64)`. Inserts strictly increasing per `(pool, kes_period)`. Shape closed; set open. |
| `PoolDistrView` pool table *(B1)* | `ade_ledger::consensus_view::PoolDistrView::pools` | `BTreeMap<Hash28, PoolEntry>`. Shape closed; set of pools open (whatever the operating-epoch snapshot contains). Built once per epoch; not runtime-pluggable. |
| **Mempool admitted set** *(NEW in B2)* | `ade_ledger::mempool::admit::MempoolState::accepted` | `Vec<Hash32>` of admitted tx ids in admission order. The **shape** is closed; the **set** is open and grows monotonically per accepted tx. Mutated only by `admit` (Tier-1, after a Valid `tx_validity` verdict); the accumulating state evolves in lockstep. NOT runtime-pluggable; no policy may add/remove ids (DC-MEM-02). |
| **`SignerSource` provenance set** *(NEW in B2)* | `ade_ledger::tx_validity::required_signers::RequiredSigners::{keys, provenance}` | `BTreeSet<Hash28>` + `BTreeSet<(SignerSource, Hash28)>`. The `SignerSource` *enum* is closed; the per-tx **set** of required keys is open and is whatever the tx body demands. Built deterministically per tx; not a registry. |
| `RollbackSnapshot` ring *(N-B)* | `ade_runtime::consensus::chain_selector::OrchestratorState::recent_snapshots` | Bounded `Vec<RollbackSnapshot>` capped at `DEFAULT_SNAPSHOT_LIMIT = 2160`. No plugin extension. |
| Oracle reference snapshots / regression corpus | `ade_testkit::harness::{snapshot_loader, regression_corpus}` | Tooling-only. New oracle data via `corpus/` + manifest update. `ci_check_ref_provenance.sh` enforces checksum integrity. GREEN. |
| Network corpus (mini-protocol transcripts) | `corpus/network/{n2n,n2c}/*` | Tooling-only. Captured via `ade_network::bin::capture_*`. Append-only by convention. |
| Consensus corpus | `corpus/consensus/*` | Tooling-only. Append-only by convention. |
| Block-validity corpus *(B1)* | `corpus/validity/{conway_epoch576, adversarial}/` | Tooling-only. Positive + adversarial; both replay byte-identically (T-DET-01, DC-VAL-04). GREEN harness in `ade_testkit::validity`. |
| **Tx-validity corpus** *(NEW in B2)* | the Conway-576 corpus txs extracted by `ade_testkit::tx_validity::extract` + the family A/B adversarial mutators | Tooling-only. Positive = every on-wire Conway tx extracted from the committed corpus blocks (at `track_utxo=false`); adversarial = witness mutations (family A, real txs) + synthetic value/input/witness mutations (family B, `track_utxo=true`). New positive entry = new captured block tx; new adversarial entry = new deterministic mutator. Append-only; GREEN harness in `ade_testkit::tx_validity`. |
| `KillStrategy<D>` trait impls | `ade_runtime::chaindb::crash_safety` | RED-only test infrastructure. Object-safe by intent. |
| Recovery state types | callers of `Recoverable` | Open: any state type with a canonical encode + apply-block step. The trait is the only way in. |
| Pinned external crates | `crates/*/Cargo.toml` | New external crate requires a Tier-5 rationale doc (per `docs/active/CE-79_tier5_addendum.md`). |

### Candidates — extensible surfaces not yet wired

| Cluster | Candidate registry | Rationale |
|---------|-------------------|-----------|
| B2+ / N-E | Mempool eviction / prioritization policy (beyond the `OrderPolicy` stub) | Tier-5 — operator-tunable. Plugin trait candidate: `MempoolPolicy`. MUST stay below the Tier-1 `admit` gate (DC-MEM-02) — never reads `tx_validity`. |
| N-A (deferred) | Peer address book | Operator-supplied; runtime mutable. Should live in `ade_runtime`. |
| N-C | Block-production policy (forge cadence, KES rotation, slot election) | Tier 1 semantics, Tier 5 operator triggers. Forge inputs must reduce to the existing `BlockEnvelope` chokepoint. |
| N-F | Query API method set | Tier 5 wire / Tier 1 semantics. Closed enum internally, mapped to gRPC/HTTP at the edge; shared with LSQ/LocalTxMonitor semantic dispatch. The LocalTxMonitor query set reads the `mempool::admit` accepted set. |
| N-F | Prometheus metric names | Tier 5; append-only registry expected. |

User confirmation needed for each at cluster entry: closed enum vs.
trait-based registry; runtime-extensible vs. compile-time-fixed; CI
enforcement shape.

### Closed-grammar audit (PHASE4-B2 specific)

This sweep was performed after PHASE4-B2 close. The author should
confirm each is intended-closed (no future plugin point) before any
extension is proposed:

1. `TxValidityVerdict` / `TxValidityError` / `TxRejectClass` —
   **closed by intent**, parallel to the B1 block-validity family.
   Mechanically enforced by `ci_check_consensus_closed_enums.sh`
   (target extended to `ade_ledger::tx_validity`). New
   verdict/reject = new variant + arms everywhere + corpus regeneration.
2. `SignerSource` (6 variants) + `RequiredSignerError` /
   `RequiredSignerField` — **closed by intent**, the DC-TXV-05 surface.
   An omitted source is impossible because the enum is exhaustive and the
   match is total; adding a source is an explicit versioned change. This
   is the load-bearing false-accept guard for the witness path.
3. `WitnessClosureError` / `WitnessField` — **closed by intent**.
   Fail-closed: wrong-size key/sig → `MalformedWitnessField`; right-key-
   wrong-body → `InvalidWitnessSignature`; uncovered → `MissingRequiredSigner`
   naming the source. An extra irrelevant witness never substitutes.
4. `TxVerdictSurface` / `TxSurfaceDecodeError` — **closed by intent**.
   The surface is deliberately *coarse*: `Valid -> [0, tx_id]`,
   `Invalid -> [1, class]`; the rich `TxValidityError` detail is debug-only
   and NOT part of the canonical bytes.
5. `AdmitOutcome` / `MempoolState` / `OrderPolicy` — **closed by intent**.
   The Tier-1 / Tier-5 split is structural: `admit` (BLUE) decides
   validity; `policy` (GREEN behavior) may only permute the admitted set.
   No `OrderPolicy` variant may ever read validity (DC-MEM-02).

**Candidate flag — deferred deposit/refund value conservation.** B2
explicitly deferred deposit-accounting preservation-of-value (cluster doc
§15). This is a **future validity-completeness extension point** — it
attaches to the phase-1 state-backed authority
(`validate_conway_state_backed`), not to the `tx_validity` composer.
Confirm at the next tx cluster that the deposit/refund rule lands as a
tightening of that authority (and is therefore shared with the block body
path) rather than as a new composer or a mempool policy.

**Candidate flag — `track_utxo=false` partial scope.** The positive tx
corpus runs at `track_utxo=false` (witness closure + structural only;
value/fee/input-resolution deferred). This is an honest partial scope, not
a relaxation — the gating already exists in `tx_phase_one`. The full
`track_utxo=true` corpus over real/synthetic resolved UTxO is the
completion seam; confirm it does not change the chokepoint.

**Candidate flag — N2N/N2C tx-submission → `mempool::admit` ingress.**
The Tier-1 gate is reachable only by direct caller/test invocation at
HEAD. The future RED ingress that delivers a tx from `tx-submission2`
(N2N) / `local-tx-submission` (N2C) opaque-bytes payloads into `admit` is
the named not-yet-wired surface (B2 cluster doc §15). Confirm the bridge
lands as RED glue producing the existing `admit(mempool, tx_cbor)` call —
not a parallel admission path.

No surfaces in this cluster look closed by accident.

---

## 4. Version-Gated vs. Frozen Contracts

### Frozen (immutable at current version — change = new major version)

- **Cardano-canonical CBOR wire format**: Each `decode_*_block` in
  `ade_codec` produces values whose wire bytes are preserved
  byte-identically. Hash inputs are wire bytes, not re-encoded bytes
  (enforced by `ci_check_hash_uses_wire_bytes.sh`).
- **Block envelope shape**: `[era_tag:u8, era_block:CBOR]`; era tags
  0..=7 (closed).
- **`PreservedCbor<T>` invariant**: `wire_bytes()` is exactly what the
  decoder consumed, byte-identical.
- **Hash algorithms**: Blake2b-224 for credential / script hashes,
  Blake2b-256 for block / transaction / Merkle hashes. Ed25519,
  Byron-bootstrap, KES-sum, VRF-draft-03 — all protocol-frozen.
- **Era-correct block body hash** *(wired at B1)*: for Alonzo+ the body
  hash is computed over the **preserved CBOR segment bytes** (never
  re-encoded — T-ENC-01). The body-hash binding in `block_validity`
  pivots on this.
- **Tx id over preserved body bytes** *(wired at B2)*: `tx_id =
  blake2b_256(preserved_body_slice)` — the body slice is lifted
  byte-for-byte out of the full tx CBOR; never a re-encode (T-ENC-01).
  Both `tx_validity` and the witness closure pivot on this hash.
- **Plutus script ingress chokepoint**: `PlutusScript::from_cbor` in
  `crates/ade_plutus/src/evaluator.rs`. Moving it invalidates the
  path-exact allowlist in `ci_check_ingress_chokepoints.sh` Check 3.
- **Plutus language set**: V1, V2, V3. PV11 builtins gated off (S-29).
- **Aiken UPLC quarantine pin**: `aiken_uplc` at tag `v1.1.21`, commit
  `42babe5d`.
- **Ouroboros mux frame layout**: 8-byte big-endian header, payload
  `≤ 65535` bytes.
- **11 closed mini-protocol message enums** + **8 closed state graphs**
  (N-A): wire grammar and legal `(state, agency, version, msg)` tuple
  set per protocol are protocol-fixed.
- **`BootstrapAnchorHash` v1 preimage** *(N-B)*: Blake2b-256 over
  `b"ade_bootstrap_v1" || canonical_cbor([byron, shelley, alonzo,
  conway])`. Domain tag, ordering, encoding, and algorithm frozen for v1.
- **`EraSchedule` invariants** *(N-B)*: monotonic `start_slot`, non-empty
  era list, non-zero `slot_length_ms` and `epoch_length_slots`.
- **`PraosChainDepState` / `ChainEvent` CBOR encodings** *(N-B)*: frozen
  for the protocol version; round-trip byte-identical (T-DET-01).
- **Consensus error taxonomies** *(N-B)*: flat-data, `String`-free,
  `Box<dyn>`-free, replay-stable.
- **`StreamInput` 3-variant taxonomy** *(N-B)*. **`HeaderVrf` era model**
  *(N-B)*: two arms (Tpraos / Praos), era selects the arm.
- **`block_validity` composition contract** *(B1)*: `Valid` iff header
  authority ∧ body-hash binding ∧ body authority all accept (DC-VAL-02);
  header-before-body fail-fast (DC-VAL-03); no partial mutation on the
  invalid path (DC-VAL-05). Pure, total, deterministic (DC-VAL-01).
- **`VerdictSurface` CBOR encoding** *(B1)*: only the coarse class is
  encoded; round-trip byte-identical (T-DET-01).
- **`LedgerView` trait shape** *(N-B; B1-refined)*: 4 `Option`-returning
  methods; `pool_vrf_keyhash -> Hash32` is the registered-VRF surface.
- **`tx_validity` composition contract** *(NEW in B2)*: `Valid` iff
  phase-1 ∧ (phase-2 when Plutus scripts present) accept (DC-TXV-02);
  phase-1-before-phase-2 fail-fast; no partial mutation on the invalid
  path (DC-TXV-04). Pure, total, deterministic over `(LedgerState,
  tx_cbor)` — no arrival order, clock, HashMap iteration, or float
  (DC-TXV-01). The composer adds no rules of its own.
- **`SignerSource` enumeration** *(NEW in B2)*: the 6-variant closed,
  era-versioned required-signer surface (DC-TXV-05). A signer source not
  in the enum cannot be silently omitted; the per-cert-kind / per-voter /
  per-credential derivation rules are grounded in Conway
  `getConwayWitsVKeyNeeded` + `getVKeyWitnessConwayTxCert` and are frozen
  for the Conway protocol version.
- **Witness-closure contract** *(NEW in B2)*: coverage is by key hash =
  `Blake2b-224(vkey)`, signature verified over the preserved body hash,
  fail-closed; wrong-size fields and uncovered keys are hard rejects, and
  an extra irrelevant witness never substitutes (DC-VAL-06 /
  CN-LEDGER-09).
- **`TxVerdictSurface` CBOR encoding** *(NEW in B2)*: `Valid -> [0,
  tx_id]`, `Invalid -> [1, reject_class_discriminant]`; `TxRejectClass`
  discriminants 0..4 fixed; only the coarse class is encoded; round-trip
  byte-identical (T-DET-01).
- **Mempool admission contract** *(NEW in B2)*: `admit`'s verdict equals
  `tx_validity`'s verdict; no false accept; on Invalid the mempool is
  returned unchanged (DC-MEM-01). The Tier-5 `OrderPolicy` projection is a
  deterministic permutation of the admitted set that cannot change a
  verdict (DC-MEM-02).
- **All canonical types**: shapes frozen at the era / version they
  entered. Adding fields requires a versioned gate; renaming forbidden.
- **TCB color assignments**: per `.idd-config.json` `core_paths`.
  `ade_core::consensus`, `ade_ledger::{block_validity, tx_validity,
  mempool::admit, consensus_view}` are BLUE; `ade_ledger::mempool::policy`
  is GREEN behavior inside the BLUE crate; `ade_ledger::consensus_input_extract`
  is pure-over-bytes "RED behavior" inside the BLUE crate;
  `ade_runtime::consensus` is RED; `ade_testkit::{consensus, validity,
  tx_validity}` is GREEN; `ade_core_interop` is RED.
- **`ChainDb` / `SnapshotStore` / `Recoverable` trait shapes** (N-D
  closed): trait method sets frozen.

### Version-gated (can evolve across major versions)

- **New `CardanoEra` variant**: requires new `decode_*_block` chokepoint,
  new per-era composer, new hfc translation arm, addition to
  `CardanoEra::ALL`, extension of the named-chokepoint header in
  `ci_check_ingress_chokepoints.sh`, and the `later_eras` table.
- **Pre-Conway single-tx validity** *(B2 extension point)*: extending
  `decode_tx` to per-era body decode + adding the era arm to
  `required_signers` / `tx_derived_required_signers` (both return
  `UnsupportedEra` for non-Conway today). Requires a per-era
  `SignerSource` grounding + a per-era positive/adversarial corpus.
- **Deposit/refund preservation-of-value** *(B2 deferred follow-up)*:
  tightens `validate_conway_state_backed` (the phase-1 state-backed
  authority shared by `tx_validity` and the block body path); the
  `tx_validity` composer does not change.
- **Full-scope `track_utxo=true` tx corpus** *(B2 extension point)*: the
  gating already exists in `tx_phase_one`; completion is corpus + state
  wiring over real/synthetic resolved UTxO, not a new chokepoint.
- **TPraos full-block validity** *(B1 extension point)*: extending
  `block_validity::decode_block` to build `HeaderVrf::Tpraos` for
  pre-Babbage eras.
- **New `GovAction` / `ConwayCert` / Plutus version variant**: registry
  diff (§3) + arms in every chokepoint.
- **New `SignerSource` variant** *(B2)*: an explicit versioned addition
  (e.g., a future credential kind) — requires arms in `required_signers`
  (+ `tx_derived_*` if UTxO-free), the witness-closure source reporting,
  and a corpus showing coverage.
- **New `TxRejectClass` / `BlockRejectClass` / `FieldKind` /
  `MissingInput` variant**: arms in the relevant `class()` mapping, arms
  in the verdict-surface discriminant maps, and a regenerated
  positive + adversarial corpus.
- **New `OrderPolicy` variant** *(B2)*: a new deterministic permutation
  over the admitted set; must read only the admitted-id list (DC-MEM-02).
- **New protocol parameter field**: append to `ProtocolParameters`; CBOR
  field-order discipline preserved by `ade_codec`.
- **New CI check**: additive. Removing a check requires a registry
  deprecation note.
- **Pinned external crate bump**: Tier-5 rationale doc required.
- **New mini-protocol**: new module with a closed enum, new chokepoint
  pair, new transition, new `*Version` enum. Never an arm on an existing
  enum.
- **Mini-protocol version-table bump**: each `*Version` enum may grow by
  appending a higher variant.
- **New `ChainEvent` / `ChainSelectionReject` / `StreamInput` variant**
  *(N-B)*: bump the envelope version, add encode/decode + dispatch arms,
  regenerate the corpus.
- **New `NetworkMagic`** *(N-B)*: the `parse_genesis` match arm + a new
  boundary table + a normative note.
- **New `LedgerView` impl / LedgerState-backed `PoolDistrView`
  constructor** *(N-B / B1; B4 sync path)*: a slice wiring the impl while
  keeping the trait shape, plus a corpus showing equivalent behavior.
- **`BootstrapAnchorHash` preimage v2** *(N-B)*: hard version-gated.
- **N2N/N2C tx-submission → `mempool::admit` ingress** *(B2 deferred)*:
  the RED bridge from tx-submission opaque-bytes payloads into the
  existing `admit` call; gated by its own cluster doc.
- **Phase-4 cluster surface additions** (N-C, N-E, N-F): each cluster's
  wire surface gates additions via its own cluster doc.

---

## 5. Module Addition Rules

Ade's workspace is small and color-disciplined. New modules enter as new
crates under `crates/`, or as new BLUE submodules within an existing BLUE
crate. `ade_network` is the first BLUE crate with **per-submodule** color
assignment; `ade_runtime` is mixed. **B2 added two new submodule trees
inside the BLUE `ade_ledger` crate** — `tx_validity::*` (all BLUE) and
`mempool::{admit (BLUE), policy (GREEN behavior)}` — alongside B1's
`ade_ledger → ade_core` cross-crate edge. The mempool tree shows the
project's first **intra-module Tier-1/Tier-5 split**: `admit` is the BLUE
authority, `policy` is GREEN-behavior glue, both inside the BLUE crate and
both scanned by `ci_check_consensus_closed_enums.sh`.

| Color | Naming convention | Build-config flags | May depend on | MUST NOT depend on |
|-------|-------------------|--------------------|----------------|--------------------|
| **BLUE** | `ade_*` (no color in name) | First line of every `.rs` is the contract banner `// Core Contract:`. `lib.rs` carries `#![deny(unsafe_code)]`, `#![deny(clippy::unwrap_used/expect_used/panic/float_arithmetic)]`. No `#[cfg(feature = ...)]`. No async (DC-CORE-01). No `ChainDb`/`f32`/`f64`/density inside `ade_core::consensus`. No `#[non_exhaustive]`/open-tail/`String`/`Box<dyn>` in `ade_core::consensus`, `ade_ledger::block_validity`, **`ade_ledger::tx_validity`, and `ade_ledger::mempool`**. | Other BLUE crates / submodules only (incl. the `ade_ledger → ade_core` edge) | Any RED submodule or crate; GREEN in non-dev deps; `pallas_*` (except `ade_plutus`); async runtime; `HashMap`/`HashSet`/`IndexMap`; clock/rand/float/env/I/O. |
| **GREEN** | `ade_*` | Banner + deny attrs are project convention but not currently enforced for `ade_testkit` / `ade_network::mux::mod` / `ade_ledger::mempool::policy`. May use `HashMap`/`serde_json`/`flate2`/`tar` for fixture I/O (testkit). `ade_runtime::consensus::chain_selector` and `ade_ledger::mempool::policy` are GREEN-behavior but live in BLUE crates for dep convenience. | BLUE crates + standard library + ecosystem crates | `ade_runtime` (for `ade_testkit`); RED submodules in non-test paths. Results must never feed back into a BLUE authoritative decision (policy must never affect `admit`). |
| **RED** | `ade_*` | No special header. Free to use clocks, I/O, async, `HashMap`, signing keys (`ade_runtime` is the only crate that may sign). | Any BLUE / GREEN crate or submodule (one-way) | Cannot be depended on by BLUE (`ci_check_dependency_boundary.sh`, `ci_check_no_async_in_blue.sh`). |

### New module checklist

1. **Add to `Cargo.toml` workspace members.** `version = "0.1.0"`,
   `edition = "2021"`.
2. **Declare TCB color** by editing `.idd-config.json` `core_paths` if
   BLUE; if the crate is mixed-color, name each BLUE submodule path and
   ensure the BLUE CI scripts scan the submodule subset.
3. **CI script update obligations** — extend the relevant BLUE-scoped
   scripts. For closed-taxonomy additions (a new verdict / reject /
   error / outcome family), add the new module path to the `TARGETS` /
   no-`String` file list in `ci_check_consensus_closed_enums.sh` (whose
   set now covers `ade_core::consensus`, `ade_ledger::block_validity`,
   `ade_ledger::tx_validity`, and `ade_ledger::mempool`). For
   consensus-shaped additions also extend
   `ci_check_no_chaindb_in_consensus_blue.sh`,
   `ci_check_no_float_in_consensus.sh`, and (if a fork-choice surface is
   touched) `ci_check_no_density_in_fork_choice.sh`.
4. **Add contract banner** (BLUE) to every `.rs` file.
5. **Add deny attributes** to `lib.rs` (BLUE).
6. **New canonical types:** at HEAD the canonical-type registry is inline
   in the invariant registry (`canonical_type_registry: null`) — add a
   `[[rules]]` block under family `T`, plus a round-trip test.
7. **Run `cargo test --workspace` and the full CI script suite.** Both
   must be green before the cluster can close.

### Phase 4 anticipated additions

- **B2 follow-ups (tx validity completeness)**: deposit/refund value
  conservation (tightens `validate_conway_state_backed`); full
  `track_utxo=true` corpus; pre-Conway eras (extend `decode_tx` +
  `required_signers`). The `tx_validity` composer does not change.
- **N-E (mempool propagation / eviction)**: a Tier-5 `MempoolPolicy`
  trait below the existing `admit` gate, plus the RED N2N/N2C
  tx-submission ingress that calls `admit`. Likely a RED operator shim in
  `ade_runtime` joined to the BLUE `ade_ledger::mempool`.
- **B4 / sync — LedgerState-backed `PoolDistrView`**: a constructor that
  builds `PoolDistrView` from a parsed `LedgerState`. Lives in
  `ade_ledger` (BLUE); keeps the `LedgerView` trait shape.
- **header→body bridge**: the `ade_node` composition layer joining
  `process_stream_input` (header fork-choice) and `block_validity`
  (full-block decision on the fetched body). Likely RED glue.
- **N-C (forge)**: forge-block path likely in `ade_runtime` (RED) for
  KES / VRF signing; must call into `ade_ledger` for canonical
  validation. Reduction target is the existing `BlockEnvelope` chokepoint.
- **N-F (operator API)**: thin RED layer mapping a closed Query enum to
  gRPC/HTTP; shares semantic dispatch with N-A's LSQ / LocalTxMonitor
  opaque-bytes payloads. LocalTxMonitor reads the mempool admitted set.

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
  only. `ci_check_hash_uses_wire_bytes.sh`. (B2: `tx_id` is over the
  preserved body slice, never a re-encode.)
- No construction of `PreservedCbor` outside `ade_codec` —
  `ci_check_ingress_chokepoints.sh` Checks 1 & 2.
- No raw CBOR decoding in any BLUE crate except `ade_codec` and the
  single allowlisted file `crates/ade_plutus/src/evaluator.rs` —
  `ci_check_ingress_chokepoints.sh` Check 3. (`tx_validity::decode_tx` and
  `required_signers` read CBOR via the `ade_codec` primitive set and the
  `decode_conway_tx_body` chokepoint — they never construct `PreservedCbor`.)
- No `pallas_*` reference outside `ade_plutus` —
  `ci_check_pallas_quarantine.sh`.
- **(N-A specific)** No `Box<dyn Codec>` / `Box<dyn Protocol>` /
  `#[non_exhaustive]` on mini-protocol message enums; no generic
  `Codec<P>` trait. No reading "selected protocol version" from a session
  global inside a transition (DC-PROTO-06). No decoding block/tx/address
  CBOR inside `ade_network`.
- **(N-B specific)** No `ChainDb` / `chain_db` token inside
  `ade_core::consensus`. No density-based ordering in caught-up Praos
  fork-choice. No `#[non_exhaustive]` / open-tail / `String` / `Box<dyn>`
  in `ade_core::consensus`. No body inspection for fork-choice. No
  stake-snapshot rederivation in BLUE consensus. No plugin-style runtime
  registration of consensus protocols.
- **(B1 specific)** No `#[non_exhaustive]` / open-tail / `String` /
  `Box<dyn>` in `ade_ledger::block_validity`. No `Valid` block verdict
  that skips either authority (DC-VAL-02). No body validation on a
  header-invalid block (DC-VAL-03). No partial mutation on the invalid
  path (DC-VAL-05). No fail-open length/size guard (DC-VAL-06). No
  re-encoding for the block body hash (T-ENC-01). No encoding of the full
  error into the comparison surface — coarse class only. No silent
  fallback on a missing consensus input.
- **(B2 specific)** No `#[non_exhaustive]`, no open-tail `Other` /
  `Unknown`, no owned `String`, no `Box<dyn>` anywhere in
  `ade_ledger::tx_validity` **or `ade_ledger::mempool`** —
  `ci_check_consensus_closed_enums.sh` (TARGETS extended; the
  `tx_validity` + `mempool` `.rs` files are in the no-`String` file
  list). Every reject is a structured `TxValidityError`; the canonical
  surface is the coarse `TxRejectClass` only.
- **(B2 specific)** No `Valid` tx verdict that skips either phase — a tx
  is `Valid` iff phase-1 ∧ (phase-2 when Plutus present) accept
  (DC-TXV-02). No phase-2 evaluation on a phase-1-failed tx.
- **(B2 specific)** No partial / in-place mutation on the invalid path —
  `tx_validity` returns the input state unchanged on any Invalid
  (DC-TXV-04).
- **(B2 specific)** No nondeterminism in the per-tx verdict — no arrival
  order, clock, HashMap iteration, or float may influence Valid/Invalid
  (DC-TXV-01). The required-signer set uses `BTreeSet`; the first-failing
  requirement is reported in stable `Hash28` order.
- **(B2 specific)** No incomplete or silently-omitted required-signer
  source — `SignerSource` is closed and exhaustive (DC-TXV-05); an
  unresolvable input is a fail-fast `UnresolvableInput`, never an "assume
  covered"; an unknown cert/voter discriminant is a fail-closed
  `MalformedField`, never a skip.
- **(B2 specific)** No fail-open witness check — wrong-size key/sig is a
  `MalformedWitnessField` (via `from_bytes`), a covered-but-non-verifying
  signature is `InvalidWitnessSignature`, and an extra irrelevant witness
  can never substitute for a missing required one (DC-VAL-06 /
  CN-LEDGER-09).
- **(B2 specific)** No re-encoding for the tx id — it is
  `blake2b_256(preserved_body_slice)` (T-ENC-01); the witness closure
  verifies over this same hash.
- **(B2 specific)** No reading `track_utxo=false` as "full validity" — it
  is a strict partial subset (witness closure + structural; UTxO-dependent
  checks deferred). The witness closure runs unconditionally; the
  state-backed checks are gated on `track_utxo`.
- **(B2 specific — `mempool::admit`)** No false accept — a tx is admitted
  iff `tx_validity(accumulating, tx)` is `Valid` (DC-MEM-01); on Invalid
  the mempool is returned unchanged.

### GREEN (`ade_testkit` incl. `validity` / `tx_validity`, `ade_network::lib` / `mux::mod`, `ade_runtime::consensus::{candidate_fragment, chain_selector}`, `ade_ledger::mempool::policy`)

- No nondeterminism that leaks into stored fixtures — fixtures must be
  byte-reproducible (the block-validity and tx-validity corpora replay
  identically).
- No participation in authoritative outputs. The B1/B2 validity harnesses
  only *drive* `block_validity` / `tx_validity` and assert; the mutators
  are deterministic transforms over real corpus blocks/txs.
- No `HashMap` even in test helpers — `BTreeMap` only.
- No import of `ade_runtime` from `ade_testkit`.
- No inbound dep from any RED crate (for `ade_testkit` /
  `ade_network::lib` / `mux::mod`).
- (`ade_runtime::consensus::chain_selector` specifically) No comparison
  decision; defer to BLUE.
- **(`ade_ledger::mempool::policy` specifically — B2)** No call to
  `tx_validity`; no read of the accumulating state; no add/remove of a tx
  id. `order` is a pure deterministic PERMUTATION over the admitted-id
  list and cannot change which txs `admit` accepted (DC-MEM-02). Tier-5
  is provably below Tier-1.

### RED (`ade_runtime`, `ade_node`, `ade_network::mux::transport`, `ade_network::session`, `ade_network::bin::capture_*`, `ade_runtime::consensus::genesis_parser`, `ade_core_interop`, and the RED-behavior `ade_ledger::consensus_input_extract` scan)

- No direct mutation of `ade_ledger` state — all transitions go through
  `ade_ledger::rules::*`, the `block_validity` / `tx_validity` composers,
  or `mempool::admit`.
- No bypassing `ade_codec` to construct semantic types from raw bytes.
- (`ade_runtime` specifically) No dep on `ade_ledger` — bytes-in /
  bytes-out only (S-36). No leakage of `redb` types through `chaindb::*`
  (S-34). No second public `chaindb` path. No automatic snapshot pruning.
  No partial-recovery success. No async recovery surface.
- (`ade_network::mux::transport`) No protocol logic; bearer I/O only.
- (`ade_network::session`) Composition glue only.
- (`ade_network::bin::capture_*`) Live-interop tools only; never linked
  into the node binary.
- (`ade_runtime::consensus::genesis_parser`) No re-derivation of the
  bootstrap anchor outside `compute_anchor_hash`; no BLUE re-consumption
  of the JSON bytes.
- (`ade_ledger::consensus_input_extract`) The nonce tail-scan parses an
  external dump format (RED behavior) but stays pure-over-bytes and
  fail-closed; never gains I/O, a clock, or a heuristic "best-effort"
  nonce pick.
- (future N2N/N2C tx-submission ingress — candidate) When wired, the RED
  bridge must call `mempool::admit(mempool, tx_cbor)` — it must NOT carry
  a parallel admission path or any validity decision of its own.
- (`ade_core_interop`) Live-interop driver only; tests `#[ignore]`-gated.

### Project-specific additions

- **No commits of credentials, hostnames, IPs, private keys** —
  public-repo discipline; enforced by `ci_check_no_secrets.sh`.
- **No `Phase 4 internal-mode mock network`** — Tier 1 surfaces must be
  exercised against real cardano-node peers. B1's positive corpus is real
  on-chain Conway-576 blocks; **B2's positive tx corpus is the real
  on-wire Conway txs extracted from those same blocks**, and the
  adversarial corpus is mutator-derived from real txs.
- **No collapsing wire and canonical bytes** — dual-authority rule.
- **No Tier 5 surface without a stated rationale** — divergence from
  cardano-node requires naming "what's better" per
  `docs/active/CE-79_tier5_addendum.md`. The mempool `policy` layer is the
  newest Tier-5 surface and must stay below the Tier-1 `admit` gate.
- **No "we'll match it later" stubs on Tier 1 surfaces** — Tier 1
  closure is hard-gated. The B1 block verdict, the B2 tx verdict, and the
  B2 mempool admission gate are all Tier-1 surfaces.

---

## Cross-references

- CODEMAP: `docs/ade-CODEMAP.md` — module-by-module authority table,
  upstream of this document. **Cross-reference warning:** at the SEAMS
  generation SHA (`85a50dc`) the CODEMAP header reads `0d4457e` (pre-B2)
  and its narrative stops at PHASE4-B1 — it has **no entries for the new
  `ade_ledger::tx_validity::*` or `ade_ledger::mempool::*` submodules**,
  its `ade_ledger` "Creates" / "Entry points" lists omit the B2 types
  (`TxValidityVerdict`, `SignerSource`, `AdmitOutcome`, `OrderPolicy`,
  etc.), and its CI note does not record that
  `ci_check_consensus_closed_enums.sh` was extended to cover `tx_validity`
  + `mempool`. Regenerate CODEMAP (`/codemap`) to refresh before relying
  on it for the B2 surface.
- Invariant registry: `docs/ade-invariant-registry.toml` — rule families
  incl. `T`, `CN`, `DC` (with `DC-PROTO-*` + `DC-CORE-01` under N-A,
  `DC-CONS-03..10` under N-B, `DC-VAL-01..06` under B1, and
  **`DC-TXV-01..05` + `DC-MEM-01/02` under B2**), `OP`, `RO`.
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
  `B1-S{1..7}.md`.
- Cluster B2 (closed): `docs/clusters/PHASE4-B2/cluster.md` +
  `B2-S{1..5}.md`.
- N-A live-interop evidence: `docs/active/CE-N-A-5_evidence.toml`.
