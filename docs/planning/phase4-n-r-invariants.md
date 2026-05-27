# PHASE4-N-R — Invariants sketch (live forge + bounty artifact)

> **Concept.** Close the open obligations carried out of
> PHASE4-N-Q by composing the real forge handler in
> `produce_mode::apply_effects_with_forge_handler` (VRF
> leader-check + KES sign + `scheduler_step` + `self_accept` +
> `BroadcastBlock`), wiring `ServedChainSnapshot::push_atomic`
> on broadcast, dispatching per-peer chain-sync / block-fetch
> frames through `n2n_server::dispatch_*`, parsing real
> cardano-cli opcert envelopes + Conway genesis files, and
> capturing the bounty-facing paired evidence (Ade
> `BlockForged H` + Haskell cardano-node `BlockAccepted H` /
> `AddedToCurrentChain H`) against the local docker
> `cardano-node-preprod` peer.
>
> **Cluster split** — three invariant-driven sub-clusters:
>
> - **PHASE4-N-R-A** — real forge composition. Closes the
>   `RequestForge → {ForgeNotLeader | ForgeFailed | ForgeSucceeded}`
>   contract via VRF leader-check + KES-period/opcert window
>   check + `scheduler_step` + `self_accept`. No served-snapshot
>   push outside the test harness.
> - **PHASE4-N-R-B** — served snapshot + per-peer dispatch.
>   `BroadcastBlock → push_atomic` + replace `_ => {}` in
>   `produce_mode::handle_listener_event` + route ChainSync /
>   BlockFetch frames through `n2n_server::dispatch_*` + prove
>   no block is served unless present in `ServedChainSnapshot`.
> - **PHASE4-N-R-C** — bounty artifact run path. opcert envelope
>   parser + Conway genesis parser + legacy binary shim +
>   docker-preprod evidence capture + paired `BlockForged H` /
>   peer `BlockAccepted H` capture.
>
> Each sub-cluster is independently mergeable, replay-verifiable,
> and reviewable. None of them depend on the next sub-cluster's
> exit criteria; N-R-A and N-R-B compose end-to-end via the
> existing N-Q surfaces.

## §1 What must always be true (I-rules)

- **I1.** When the coordinator emits `RequestForge { slot,
  kes_period, ledger_snapshot_ref, chain_tip }`, the RED shell
  handler returns exactly one of:
  - `ForgeSucceeded { slot, artifact }` where `artifact.bytes`
    decodes via Ade's BLUE block decoder AND
    `self_accept(artifact, chain_tip, ledger_snapshot)` returns
    `Accepted`;
  - `ForgeNotLeader { slot, vrf_output_fingerprint }`;
  - `ForgeFailed { slot, structured_error }`.
  No other outcome is permitted.
- **I2 (corrected).** Leader-check splits across the color
  boundary:
  - **RED side:** the producer shell computes a VRF
    proof/output for `slot` using the operator's VRF signing
    key. RED owns the key; RED produces the proof. The
    canonical surface is `(slot, eta0) → (vrf_output, vrf_proof)`.
  - **BLUE side:** given `(slot, eta0, vrf_output, vrf_proof,
    vrf_verification_key, stake_distribution, leader_threshold)`,
    the leader-eligibility verdict is a pure function of those
    canonical inputs. BLUE verifies the proof and evaluates
    eligibility; BLUE never sees the VRF signing key.
- **I3.** The forged block's body-hash matches the header's
  `body_hash` field byte-for-byte (DC-CONS-18 strengthening
  under real load).
- **I4 (corrected).** A forged block becomes visible to peers
  only AFTER `push_atomic` succeeds, not merely because the
  coordinator emitted `BroadcastBlock`. Ordering:
  1. RED forge handler produces `ForgeSucceeded { artifact }`.
  2. Coordinator emits `BroadcastBlock { artifact }`.
  3. RED effect handler calls
     `served_chain_snapshot.push_atomic(artifact)`.
  4. *Only if `push_atomic` returns `Ok(ServedTip)`* may per-peer
     reducers serve the block.
  5. If `push_atomic` fails, emit a structured shutdown event
     (`ShutdownReason::PushAtomicFailed` or similar) and
     fail-closed. Do not continue as though broadcast succeeded.
- **I5.** The per-peer dispatch path is unbroken:
  `n2n_listener` → `MuxPump` →
  `PeerN2nServerChainSyncFrame` / `PeerN2nServerBlockFetchFrame` →
  `n2n_server::dispatch_chain_sync` /
  `n2n_server::dispatch_block_fetch` → `ServerReply` → mux →
  peer. No frame skips a layer; no event is absorbed by a
  catch-all arm.
- **I6.** The opcert envelope parser accepts a cardano-cli
  `node.opcert` text envelope whose `type` field equals
  `NodeOperationalCertificate` and whose `cborHex` decodes,
  per the **golden fixtures captured in N-R-A's proof
  obligation**, to a canonical `OperationalCert`. The exact CBOR
  shape (flat 4-element array vs. tagged variant vs. nested) is
  derived from the fixtures, not assumed.
- **I7.** The Conway genesis parser is a **closed-contract**
  parser: it accepts a documented set of JSON shapes (numeric
  encoding, key ordering, default behavior), extracts a fixed
  set of fields (`network_magic`, `slot_zero_time_unix_ms`,
  `slot_length_ms`, `slots_per_kes_period`,
  `kes_anchor_slot`, `kes_max_period`), and fail-closes on
  anything else. Library permissiveness MUST NOT leak into BLUE
  acceptance behavior; the parser-output → `GenesisAnchor` step
  is GREEN over canonical bytes the RED parser has already
  extracted.
- **I8.** "cardano-node accepts Ade-forged block H" is captured
  as the paired evidence: an Ade `evidence.jsonl` line
  `{kind: "BlockForged", block_hash: H, ...}` AND a Haskell
  peer log line containing `H` with `BlockAccepted` or
  `AddedToCurrentChain`. Both artifacts are committed under
  `docs/clusters/PHASE4-N-R/CE-N-R-C-LIVE_<date>.{jsonl,log}`.
- **I9 (new).** Empty-block forging is the **explicit scope** of
  N-R. Forge handler builds a block whose body is the empty
  transaction set; this proves the production + serving path
  end-to-end without depending on mempool integration. The
  runbook MUST state this explicitly so empty-block evidence is
  not misread as closing the broader TxSubmission obligation.

## §2 What must never be possible (N-rules)

- **N1.** No `ForgeSucceeded` whose `artifact` fails
  `self_accept`. The bridge is enforced inside the RED forge
  handler: if `self_accept` fails, emit `ForgeFailed` (with the
  self-accept verdict as `structured_error`), never
  `ForgeSucceeded`.
- **N2.** No `BroadcastBlock` effect from a non-leader slot.
  The BLUE leader-check verdict is the gate inside the forge
  handler.
- **N3.** No KES sign with a `kes_period` outside the opcert's
  `[start, start + SUM6_MAX_PERIOD]` window (CN-PROD-02
  strengthening; same rule exercised under real Conway load).
- **N4 (strengthened).** A `RequestRange` covering a slot range
  that is not entirely present in `ServedChainSnapshot` MUST
  follow the cardano-node block-fetch protocol's failure
  semantics exactly — `NoBlocks` (or whichever the Cardano
  block-fetch state machine specifies for partial / unknown
  ranges). No partial ad-hoc response; no silent truncation; no
  serving of a strict prefix. This is a Cardano compatibility
  boundary, not just an internal safety rule.
- **N5.** No silent dispatch failure. Every
  `PeerN2nServerChainSyncFrame` /
  `PeerN2nServerBlockFetchFrame` event MUST route to
  `n2n_server::dispatch_*`; the `_ => {}` arm in
  `produce_mode::handle_listener_event` MUST be replaced with
  explicit dispatch + structured failure on unhandled variants.
- **N6.** No opcert acceptance when the cardano-cli envelope's
  `type` field is not exactly `NodeOperationalCertificate`
  (closed envelope-type check; mirrors the KES/VRF loader
  pattern).
- **N7.** No genesis acceptance when required fields are
  missing, malformed, or when the JSON parser library exhibits
  permissive behavior on a malformed input. Fail-closed with a
  structured error naming the field.
- **N8.** No retroactive forge. If wall-clock has advanced past
  the slot's deadline before forge completes, emit
  `SlotMissed { reason: DeadlineExceeded }`, not
  `BroadcastBlock` (carry-forward from CN-PROD-02).
- **N9.** No secret material in `BroadcastBlock { artifact }` —
  the artifact carries the *forged bytes* (header + body),
  never the signing keys (carry-forward from N-Q's N9).
- **N10.** No independent legacy production path. After N-R-C
  closes, `live_block_production_session.rs` is a **thin
  non-authoritative shim** that prints a deprecation note and
  delegates to `produce_mode::run_produce_mode`. There is no
  second codepath running real forge logic outside of
  `produce_mode`.
- **N11 (new).** No torn snapshot. A peer reading
  `ServedChainSnapshot` mid-`push_atomic` MUST NOT see a
  partially-applied update. Locking semantics cover the full
  insertion.
- **N12 (new).** No BLUE component holds VRF / KES / cold
  signing keys. RED owns key material; RED produces signed
  outputs; BLUE consumes the *outputs* (proofs, signatures) as
  canonical inputs. This is the T-tier key-custody boundary
  carried forward and tightened.

## §3 Determinism surface (D-rules)

- **D1.** `verify_and_evaluate_leader(slot, eta0, vrf_output,
  vrf_proof, vrf_vk, stake_distribution, leader_threshold) →
  Result<is_leader, LeaderError>` is a pure BLUE function.
  Same inputs → byte-identical verdict.
- **D2.** `scheduler_step(chain_tip, ledger_snapshot, slot,
  vrf_output, kes_sigma, hot_vkey, opcert) → Block` is a pure
  BLUE function. Same canonical inputs → byte-identical block
  bytes.
- **D3.** `self_accept(block, chain_tip, ledger_snapshot) →
  AcceptVerdict` is a pure BLUE function.
- **D4.** Opcert text-envelope decode: same cborHex bytes →
  byte-identical `OperationalCert` (CBOR canonical decode +
  closed type check).
- **D5.** Conway genesis parse: same canonical-byte input →
  byte-identical `GenesisAnchor`. The parser's library-side
  behavior is constrained by the closed parser contract (I7)
  and a golden-fixture suite that pins permissive-input
  behavior to fail-closed.
- **D6 (new).** `ServedChainSnapshot::push_atomic` is
  deterministic in its argument order: the same sequence of
  `push_atomic(a₀), push_atomic(a₁), ..., push_atomic(aₙ)`
  produces a byte-identical `ServedChainView`.

## §4 Replay equivalence (R-rules)

- **R1.** Given the same canonical event corpus into
  `produce_mode` (slot ticks, peer-connection events, ledger
  snapshot reference, opcert public metadata, VRF/KES seed
  fingerprints, leader-check inputs), the `ProducerLogEvent`
  stream is byte-identical (carry-forward DC-PROD-02
  strengthened under real forge).
- **R2.** Given the same `ServedChainSnapshot` content + the
  same `RequestRange` from a peer, the bytes served by
  `n2n_server::dispatch_block_fetch` are byte-identical
  (carry-forward DC-CONS-17 strengthened end-to-end).
- **R3.** Given the same `(stake_distribution, eta0, vrf_proof
  sequence)`, the sequence of `is_leader` verdicts across
  slots 0..N is byte-identical (BLUE-side determinism;
  RED-side VRF proof sequence is itself deterministic in
  `(slot, eta0, vrf_sk)`).
- **R4 (new).** Replay of an N-R-C operator-pass evidence
  bundle (the canonical event corpus + the `ServedChainSnapshot`
  state at the point of broadcast) reproduces the same
  `BlockForged H` event with the same `H`. The peer's
  `BlockAccepted H` is operator-witnessed and non-replayable
  in-process; it is paired with the Ade evidence via hash
  equality, not via in-process replay.

## §5 State transitions in scope

```rust
// RED forge handler (N-R-A). Owns VRF + KES + cold keys.
forge_handler(
    state: ProducerShell,
    event: CoordinatorEvent::RequestForge { slot, kes_period,
        ledger_snapshot_ref, chain_tip },
) -> Result<(ProducerShell', ForgeResult), ForgeHandlerError>;

enum ForgeResult {
    Succeeded { slot, artifact: ForgedBlockArtifact },
    NotLeader { slot, vrf_output_fingerprint: [u8; 8] },
    Failed { slot, structured_error: ForgeFailureReason },
}

// BLUE leader-check (N-R-A). No signing keys.
verify_and_evaluate_leader(
    slot: SlotNo,
    eta0: Eta0,
    vrf_output: VrfOutput,
    vrf_proof: VrfProof,
    vrf_vk: VrfVerificationKey,
    stake_distribution: &StakeDistribution,
    leader_threshold: Rational,
) -> Result<bool, LeaderCheckError>;

// RED served-snapshot mutation (N-R-B).
served_chain_snapshot.push_atomic(
    artifact: ForgedBlockArtifact,
) -> Result<ServedTip, PushError>;

// GREEN snapshot read view (N-R-B). No lock owned.
served_chain_snapshot.read_snapshot() -> ServedChainView;

// RED per-peer dispatch (N-R-B).
n2n_server::dispatch_chain_sync(
    per_peer_state: ChainSyncServerState,
    frame: ChainSyncClientMessage,
    snapshot: &ServedChainView,
) -> (ChainSyncServerState', Option<ServerReply>);

n2n_server::dispatch_block_fetch(
    per_peer_state: BlockFetchServerState,
    frame: BlockFetchClientMessage,
    snapshot: &ServedChainView,
) -> (BlockFetchServerState', Option<ServerReply>);

// RED opcert parser (N-R-C). Gated on N-R-A proof obligation.
parse_cardano_cli_opcert(
    envelope_bytes: &[u8],
) -> Result<OperationalCert, OpCertParseError>;

// RED genesis parser (N-R-C). Closed parser contract.
parse_conway_genesis(
    json_bytes: &[u8],
) -> Result<GenesisAnchor, GenesisParseError>;
```

## §6 TCB color hypothesis (corrected)

- **BLUE — authoritative core.**
  - VRF proof verification + leader-eligibility evaluation
    from canonical inputs (`verify_and_evaluate_leader`).
  - Block construction rules (`scheduler_step`).
  - Self-accept verdict (`self_accept`).
  - Opcert CBOR typed decode from already-extracted canonical
    bytes (the canonical bytes are extracted by the RED parser;
    the decode itself is BLUE).
- **GREEN — deterministic non-authoritative glue.**
  - Forge orchestration inside
    `produce_mode::apply_effects_with_forge_handler`: composes
    BLUE helpers + RED signing calls + coordinator effect
    emission. No I/O, no key custody.
  - `GenesisAnchor` projection from already-extracted canonical
    fields (a pure value transform; the RED parser extracts the
    fields).
  - `ServedChainView` read API — a value type derived from
    `ServedChainSnapshot`'s state at a point in time; consumed by
    BLUE/GREEN reducers without owning the lock or doing I/O.
- **RED — shell.**
  - VRF / KES / cold key custody (producer_shell carry-forward
    from N-Q).
  - VRF proof production (RED owns the VRF signing key; RED
    emits the proof).
  - KES signature production.
  - JSON text parsing for cardano-cli envelopes + Conway
    genesis (library-dependent permissiveness lives here; the
    closed parser contract bounds it).
  - File I/O for envelopes / genesis / evidence log.
  - tokio listener + TCP socket I/O.
  - `ServedChainSnapshot`'s `Arc<RwLock<…>>` handle +
    `push_atomic` mutation across listener tasks.
  - Per-peer dispatch wiring in `produce_mode::handle_listener_event`.

## §7 Open questions / proof obligations

- **OQ4 / Proof obligation (N-R-A entry).**
  Capture cardano-cli (cardano-node 10.6.2 + 11.0.1)
  `node.opcert` envelope bytes from a real
  `cardano-cli node issue-op-cert` run, decode with reference
  tooling (cbor.me / `cardano-cli text-view`), and record the
  exact CBOR shape. Commit golden fixtures for:
  - an accepted, well-formed envelope (the one we'll parse);
  - a malformed cborHex (rejected by length check);
  - a malformed `type` field (rejected by envelope-type check);
  - a CBOR shape that decodes but has the wrong arity / element
    types (rejected by typed decoder).
  The parser slice does NOT begin until these fixtures exist
  in the repo at `crates/ade_runtime/tests/fixtures/opcert/`.
- **OQ7 (new) / Proof obligation (N-R-A entry).**
  Capture cardano-cli Conway genesis JSON bytes (from
  `cardano-node-preprod`'s mounted config) and record the
  closed parser contract: which JSON shapes are accepted,
  which are rejected, and what permissive-input behavior the
  parser exhibits on:
  - numeric values in unexpected JSON forms (string vs.
    number);
  - missing required fields;
  - extra unknown fields (accept-and-ignore vs. fail-closed);
  - key ordering;
  - duplicate keys;
  - null vs. missing distinction.
  The parser slice does NOT begin until the closed contract is
  written + golden-fixture corpus committed at
  `crates/ade_runtime/tests/fixtures/conway_genesis/`.
- **OQ8 (new).** N4's protocol-defined block-fetch failure
  semantics — confirm against `ouroboros-network`'s
  block-fetch state machine (Haskell reference). Specifically:
  what reply does cardano-node send when a `RequestRange` covers
  a range that contains an unknown slot? `NoBlocks`?
  `RequestRangeFail`? This is a Cardano compatibility check; the
  answer goes into N-R-B's slice doc as the canonical reply
  string.
- **OQ9 (new).** Does the existing `n2n_server::dispatch_*`
  signature accept a `&ServedChainView` reference, or does it
  require an owned `ServedChainSnapshot`? If the latter, N-R-B
  needs a small refactor in the dispatch API. Check
  `crates/ade_runtime/src/network/n2n_server.rs` before slicing.

## §8 Cluster scope (subject to refinement in /cluster-plan)

### PHASE4-N-R-A — Real forge composition

**In scope:**
- BLUE leader-check (`verify_and_evaluate_leader`): pure
  verification + eligibility evaluation from canonical inputs;
  unit tests on the math.
- RED VRF proof production (uses existing
  `producer_shell.vrf_prove`).
- RED KES sign at slot's period (uses existing
  `producer_shell.kes_sign_at`).
- BLUE block construction (uses existing `scheduler_step`).
- BLUE `self_accept` integration.
- Replace the stub `ForgeNotLeader`-only handler in
  `produce_mode::apply_effects_with_forge_handler` with the
  real composition.
- Proof-obligation fixtures (OQ4 opcert + OQ7 genesis) committed
  but not yet wired to a parser.
- Unit + integration tests proving:
  - non-leader slot → `ForgeNotLeader`;
  - leader slot → `ForgeSucceeded` whose artifact survives
    `self_accept`;
  - opcert-period-out-of-range → `ForgeFailed { KesPeriodOutOfRange }`;
  - self-accept failure → `ForgeFailed { SelfAcceptRejected }`.

**Out of scope (deferred to N-R-B):**
- `ServedChainSnapshot::push_atomic` integration.
- Per-peer dispatch.

### PHASE4-N-R-B — Served snapshot + per-peer dispatch

**In scope:**
- `ServedChainSnapshot::push_atomic` API (RED shared handle;
  GREEN value model).
- `BroadcastBlock` effect handler that calls `push_atomic` and
  fail-closes on `PushError`.
- Replace `_ => {}` in `produce_mode::handle_listener_event`
  with explicit dispatch into `n2n_server::dispatch_chain_sync`
  / `dispatch_block_fetch`.
- Integration test: a synthetic dialer peer issues
  `RequestRange` covering a slot in `ServedChainSnapshot` →
  served bytes byte-identical to `scheduler_step` output.
- Integration test: a synthetic dialer peer issues
  `RequestRange` covering an unknown slot → the protocol-
  defined failure reply (resolved per OQ8).
- Integration test: no-block-served-before-push race —
  `RequestRange` for slot S during `push_atomic(S)` must serve
  either the pre-push view (`unknown`) or the post-push view
  (`block bytes`), never a torn snapshot.

**Out of scope (deferred to N-R-C):**
- Real opcert / genesis parsing.
- Bounty evidence capture.

### PHASE4-N-R-C — Bounty artifact run path

**In scope:**
- cardano-cli opcert envelope parser (driven by the OQ4
  fixtures from N-R-A).
- Conway genesis closed-contract parser (driven by the OQ7
  fixtures from N-R-A).
- `live_block_production_session.rs` rewritten as a thin shim
  (prints deprecation, invokes `produce_mode::run_produce_mode`).
- Operator-pass evidence capture against docker
  `cardano-node-preprod`: Ade
  `evidence.jsonl` + peer log → committed paired files at
  `docs/clusters/PHASE4-N-R/CE-N-R-C-LIVE_<date>.{jsonl,log}`.
- Registry flips:
  - `CN-CONS-06` → `enforced` (live half closed by paired
    evidence). Live-half open_obligation cleared OR narrowed.
  - `RO-LIVE-01` → `enforced`. Open_obligation cleared OR
    narrowed.
  - `CN-PROD-01.open_obligation` (per-peer dispatch) cleared by
    N-R-B.
  - `CN-PROD-02.open_obligation` (real forge + opcert parser +
    genesis parser) cleared by N-R-A + N-R-C.
  - `DC-PROD-02.open_obligation` (operator-pass replay
    anchor) cleared by an operator-pass paired transcript.

**Out of scope (separate clusters / smoke):**
- Private-testnet smoke evidence. Tracked as separate
  follow-on smoke — does NOT replace the docker-preprod
  artifact for N-R closure, but is captured as additional
  evidence per the bounty requirements (private testnet with
  two Haskell nodes).
- TxSubmission → mempool → block inclusion. N-R proves
  *empty-block* production + serving end-to-end. Mempool
  integration is its own cluster; N-R explicitly does NOT
  close the broader TxSubmission obligation.

## §9 Explicit out of scope

- Multi-peer concurrent forge load.
- Multi-listener / multi-port.
- TLS over N2N (¬P-8 continues to defer).
- Mlocked secret memory (future operational cluster).
- Mempool / TxSubmission2 integration. **N-R ships
  empty-block forging only.** Runbook MUST state this
  explicitly so empty-block evidence is not misread as
  closing the broader TxSubmission bounty obligation.
- Hot-key KES rotation across periods (single-period bounty
  artifact is the scope; rotation is OP-OPS-04 follow-on).
- Multi-relay topology.
- Private-testnet two-Haskell-node bounty leg (separate
  follow-on; carried as `blocked_until_two_node_private_testnet_pass`
  on the relevant bounty rule).

## §10 References

- Predecessor: PHASE4-N-Q (HEAD `c1c4b06`).
- Predecessor invariants: `docs/planning/phase4-n-q-invariants.md`.
- N-Q closure: `docs/clusters/PHASE4-N-Q/cluster.md` §4 +
  `docs/active/cn-cons-06-operator-runbook.md` §5 (deferral
  table).
- Bounty: [[project-bounty-requirements]].
- Doctrine:
  - [[feedback-hard-closure-gates]];
  - [[feedback-proof-discipline]] (OQ4 + OQ7 are proof
    obligations, not assumptions);
  - [[feedback-shell-must-not-overstate-semantic-truth]] (paired
    evidence — Ade emit + peer-witnessed line);
  - [[feedback-fail-closed-validation]] (N4, N6, N7, N11);
  - [[feedback-bounded-smoke-slices]] (private-testnet smoke is
    additional evidence, never a substitute);
  - [[reference-local-preprod-docker-cardano-node]] (docker
    target for N-R-C evidence).
