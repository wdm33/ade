# Cluster/Slice Plan — PHASE4-N-A (Network mini-protocols)

> **Status**: planning artifact, ratified after `/cluster-plan` run.
> **Inputs**: `docs/active/PHASE4-N-A_invariants.md` (sketch + closed
> decisions), `docs/active/PHASE4-N-A_scope_decisions.md` (locked
> design decisions), `docs/active/phase_4_cluster_plan.md` (Phase 4
> cluster index).
> **Next step**: `/cluster-doc PHASE4-N-A` to expand into the
> canonical `docs/clusters/PHASE4-N-A/cluster.md`.

## Cluster Index (Phase 4 — dependency order)

| # | Cluster | State | Primary invariant |
|---|---|---|---|
| 1 | **PHASE4-N-D** — Chain DB & persistence | **Closed** (`436b1d7`) | Chain DB recovery is replay-equivalent under unclean shutdown |
| 2 | **PHASE4-N-A** — Network mini-protocols | **opening** | Ouroboros mini-protocols are byte-identical to cardano-node 10.6.2 on wire AND deterministic on pure-transition semantics |
| 3 | **PHASE4-N-B** — Consensus runtime | pending | Fork choice, leader schedule, and HFC mapping match cardano-node on a curated divergence corpus |
| 4 | **PHASE4-N-E** — Mempool | pending | Every tx Ade accepts/rejects would be accepted/rejected identically by cardano-node given the same state |
| 5 | **PHASE4-N-F** — Operator surface | pending | LocalStateQuery semantic equivalence + cardano-node-EKG metric coverage |
| 6 | **PHASE4-N-C** — Block production (opt-in) | pending | Forged block bytes hash-identical to cardano-node for the same inputs |

Dependency rationale: N-A produces frames; N-B reads chain-sync output
to drive fork choice; N-E uses N-A tx-submission2 + N-B chain state;
N-F queries N-B state; N-C consumes mempool + state + KES/VRF to
forge.

This plan expands PHASE4-N-A only. N-B/N-E/N-F/N-C get their own
`/cluster-plan` runs as they open.

---

## Cluster PHASE4-N-A — Network mini-protocols

### Primary invariant

Every byte spoken or accepted on the Ouroboros wire by Ade is identical
to what cardano-node 10.6.2 would speak or accept for the same canonical
inputs; every BLUE mini-protocol transition is a pure function of
(canonical prior state, canonical input, selected protocol version,
deterministic configuration) with no ambient session influence
(DC-PROTO-06).

### Tier

1 wire + 1 semantic. No Tier 5 latitude — wire bytes are protocol.

### TCB partition

| Color | Modules |
|---|---|
| **BLUE** (8) | `ade_network::codec`, `::handshake`, `::chain_sync`, `::block_fetch`, `::tx_submission`, `::keep_alive`, `::peer_sharing`, `::n2c` |
| **RED** (2 + 1 bin) | `ade_network::mux`, `::session`, `bin/ade_network_interop` |
| **GREEN** | `ade_testkit::network` (frame corpus harness) |

### External dependencies

- **Inbound** (N-A depends on): `ade_types` (canonical IDs incl. `TxId`), `ade_codec` (CBOR primitives only — protocol message codecs are independent of block CBOR).
- **Outbound** (depends on N-A): none into BLUE crates; N-A is the producer of mini-protocol bytes, not a consumer of higher-layer authority.

### Cluster Exit Criteria

| CE | Statement | Closed by | Evidence shape |
|---|---|---|---|
| **CE-N-A-1** | Handshake version negotiation byte-identical to oracle across cardano-node 10.6.2 supported versions | S-A3 | Handshake corpus + state-machine trace; byte-identical output for every supported version tuple |
| **CE-N-A-2** | Chain-sync state machine produces same fork-choice signals as oracle on a curated divergence corpus | S-A4 | Chain-sync transcript corpus; signal sequence equivalence |
| **CE-N-A-3** | Block-fetch protocol delivers blocks byte-identically and in same order as oracle for a curated batch | S-A5 | Block-fetch frame corpus; byte-identical request→response sequences |
| **CE-N-A-4** | Tx-submission2 round-trips a curated mempool trace with byte-identical wire frames | S-A6 | Mempool transcript corpus; flow-control + inventory state trace equivalence |
| **CE-N-A-5** | Full N2N + N2C suite passes against a real cardano-node peer (live interop, reproducible via pinned Docker) | S-A10 | Live transcript hash, 5-condition proof obligation per invariants §7 #6 |

### Slices (10, authority-surface aligned)

| ID | Name | Slice invariant | Addresses CE | TCB |
|---|---|---|---|---|
| **S-A1** | Mux/framing + `ade_network` scaffold + sync-only CI gate | Mux framing bytes for a given chunk sequence are byte-identical to cardano-node mux output; `ci_check_no_async_in_blue.sh` enforces DC-CORE-01 | — (substrate) | RED + CI |
| **S-A2** | Protocol message codec authority | Every protocol-visible message in the 11-protocol surface decodes into exactly one closed, versioned message type and round-trips byte-identically (CN-WIRE-07, DC-PROTO-03, DC-PROTO-04, T-ENC-03) | partial-CE-N-A-1..4 (codec layer) | BLUE |
| **S-A3** | Version negotiation authority (N2N + N2C) | Handshake state machine selects exactly one version per session or rejects, byte-identical to cardano-node 10.6.2 (DC-PROTO-05) | **CE-N-A-1** | BLUE |
| **S-A4** | Chain-sync transition authority | Chain-sync transitions are deterministic pure functions of (state, version, agency, msg); fork-choice signals byte-identical to cardano-node oracle (DC-PROTO-01, DC-PROTO-06) | **CE-N-A-2** | BLUE |
| **S-A5** | Block-fetch transition authority | Block-fetch transitions are deterministic; frame delivery byte-identical and in cardano-node-equivalent order (DC-PROTO-01, DC-PROTO-06) | **CE-N-A-3** | BLUE |
| **S-A6** | Tx-submission2 transition authority | Tx-submission2 inventory + flow-control transitions are deterministic; round-trip byte-identical with curated mempool trace; reuses `ade_types::TxId` with preserved-byte-authority handling | **CE-N-A-4** | BLUE |
| **S-A7** | Keep-alive + peer-sharing transition authority | Keep-alive cookie protocol and peer-sharing message exchange are deterministic; `PeerSharingOutput` event interface, no peer-book authority | — | BLUE |
| **S-A8** | N2C transition authority (4 protocols) | LocalChainSync, LocalTxSubmission, LocalStateQuery, LocalTxMonitor state machines are deterministic; LSQ codec owns closed wire grammar, not ledger meaning (DC-PROTO-04) | — | BLUE |
| **S-A9** | Session wiring + frame corpus + replay harness | Composition of S-A1..S-A8 produces actual TCP/Unix sessions; `corpus/network/{n2n,n2c}/<protocol>/` captured frames replay byte-identically through codec; evidence schema (protocol_id, selected_version, canonical bytes, pre/post state hashes, output_or_error) matches CE-N-A-5 shape | partial-CE-N-A-5 (corpus reproducibility) | RED + GREEN |
| **S-A10** | Live cardano-node interop release evidence | Pinned-Docker cardano-node 10.6.2 accepts Ade's session; full 5-condition proof obligation logged | **CE-N-A-5** | RED |

### Slice ordering rationale

- **S-A1 must be first**: mux framing + sync-only CI gate must land before any BLUE codec slice ships, so the BLUE/RED partition is mechanically enforced from S-A2 onward.
- **S-A2 must precede S-A3..S-A8**: every state-machine slice needs the codec to encode/decode its messages.
- **S-A3..S-A8 internally**: S-A3 ships before S-A4..S-A8 to unblock the version-threading discipline (handshake produces the typed `*Version` value the others consume). After S-A3, the remaining transition slices (S-A4..S-A8) have no inter-protocol dependencies and may ship in any order.
- **S-A9 must precede S-A10**: live interop needs the session-wiring infrastructure.
- **S-A10 last**: closure gate.

### Replay obligations (heavy)

Cluster N-A is replay-heavy — every BLUE slice contributes new canonical
types and new replay corpus.

- **New canonical types**: ~70 enums across the 11 protocols (message types, state types, agency types, version markers, error types, output events). Each gets round-trip tests in S-A2.
- **New replay corpus**: `corpus/network/{n2n,n2c}/<protocol>/` with captured frames per CE. Per-frame metadata schema:
  - cardano-node version
  - network magic
  - protocol
  - mini-protocol version
  - direction
  - agency
  - raw bytes
  - expected decode result
  - expected re-encode bytes
- **New replay tests**:
  - per-protocol round-trip (S-A2)
  - per-state-machine transition traces (S-A3..S-A8)
  - frame-corpus replay (S-A9)
  - full-session transcript hash (S-A10)
- **Per-slice replay equivalence MAC**: every BLUE slice's per-protocol tests must replay byte-identically.
- **Cluster-level replay MAC**: the cluster-N-A replay corpus, replayed end-to-end, produces the same transcript hash as cardano-node↔cardano-node on the same canonical inputs (CE-N-A-5 evidence).

### FC/IS partition discipline

- `ci_check_no_async_in_blue.sh` (lands in S-A1) is the mechanical gate enforcing DC-CORE-01: any `async fn`, `.await`, `tokio::`, `async_std::`, `Future`, `futures::`, task spawn, async channel, or timer in any BLUE module fails CI. Grep-first; graduate to syn-based if false positives accumulate.
- `ci_check_module_headers.sh`, `ci_check_no_semantic_cfg.sh`, `ci_check_no_signing_in_blue.sh`, `ci_check_hash_uses_wire_bytes.sh`, `ci_check_ingress_chokepoints.sh`, `ci_check_dependency_boundary.sh` (already at 6-crate scope per commit `5b70bee`) gain the 8 `ade_network` BLUE submodules when `ade_network` is added to `.idd-config.json` `core_paths` with S-A1.

### Cross-cluster non-dependencies (explicit)

- N-A does NOT depend on N-B (consensus): chain-sync emits fork-choice *signals*; interpreting them is N-B.
- N-A does NOT depend on N-C (block production): block-fetch transports block bytes; forging is N-C.
- N-A does NOT depend on N-E (mempool): tx-submission2 transports tx ids and bytes; mempool semantics are N-E.
- N-A does NOT depend on N-F (operator surface): peer-book lives in N-F; peer-sharing produces `PeerSharingOutput` events for N-F to consume.

Cluster N-A is the first cluster after N-D where the entire BLUE surface is invariant-shaped from day one — no Tier 5 latitude, no per-slice carry-forward.

### Invariants strengthened by this cluster's slices

| Slice | Strengthens (registry status flip pending) |
|---|---|
| S-A1 | DC-CORE-01 (via `ci_check_no_async_in_blue.sh`); DC-PROTO-02 partial (mux framing portion) |
| S-A2 | CN-WIRE-07, DC-PROTO-03, DC-PROTO-04, T-ENC-03 |
| S-A3 | DC-PROTO-05 |
| S-A4 | DC-PROTO-01, DC-PROTO-02 (partial — chain-sync portion), DC-PROTO-06 |
| S-A5 | DC-PROTO-01, DC-PROTO-02 (partial — block-fetch portion), DC-PROTO-06 |
| S-A6 | DC-PROTO-01, DC-PROTO-02 (partial — tx-submission portion), DC-PROTO-06 |
| S-A7 | DC-PROTO-01, DC-PROTO-06 |
| S-A8 | DC-PROTO-01, DC-PROTO-04, DC-PROTO-06 |
| S-A9 | DC-PROTO-02 (closes mux + replay-corpus portion) |
| S-A10 | DC-PROTO-02 (closes via live transcript equivalence) |

DC-PROTO-02 ("transcript-equivalent miniprotocol behavior with Haskell
node") is the broadest invariant and gets strengthened incrementally
across S-A1, S-A4, S-A5, S-A6, S-A9, S-A10 until S-A10 closes it.

---

## Out of scope for this plan

- Cluster-level docs for PHASE4-N-B, N-E, N-F, N-C — those clusters need their own `/cluster-plan` runs when they open.
- `/cluster-doc PHASE4-N-A` expansion — that's the next step after this plan is ratified.
- Per-slice implementation details — those land in `/slice-doc PHASE4-N-A S-AN` per slice.

## Authority

This document is a planning artifact, not normative. Authority for
PHASE4-N-A's invariants belongs to `docs/ade-invariant-registry.toml`
(CN-WIRE-07, DC-PROTO-01..06, DC-CORE-01, T-ENC-03, T-CORE-02, etc.).
Authority for the cluster's mechanical acceptance criteria belongs to
the cluster doc generated by `/cluster-doc PHASE4-N-A`.
