# Cluster PHASE4-N-A — Network mini-protocols

> **Tier**: 1 wire + 1 semantic (no Tier 5 latitude — wire bytes are protocol)
> **Status**: opening — slice work begins with S-A1
> **Origin**: Phase 4 cluster N-A from `docs/active/phase_4_cluster_plan.md`
> **Planning trio**:
> - `docs/active/PHASE4-N-A_scope_decisions.md` (locked design decisions)
> - `docs/active/PHASE4-N-A_invariants.md` (invariants + closed §7 decisions)
> - `docs/active/PHASE4-N-A_cluster_plan.md` (ratified 10-slice plan)

## Primary invariant

`DC-PROTO-02` (transcript-equivalent mini-protocol behavior with
Haskell node) closed under `DC-PROTO-06` (BLUE transitions are pure
functions of canonical inputs with no ambient session influence) and
`DC-CORE-01` (BLUE is sync-only). See `docs/ade-invariant-registry.toml`.

## Normative anchors

- `docs/ade-invariant-registry.toml` — registry entries: CN-WIRE-07,
  DC-PROTO-01..06, DC-CORE-01, T-CORE-02, T-ENC-03, T-DET-01,
  T-INGRESS-01
- `docs/active/CE-79_gate_statement.md` — tier doctrine (this cluster
  is Tier 1, no divergence latitude)
- `docs/active/CE-79_tier5_addendum.md` — Tier 5 doctrine (for context;
  N-A explicitly excludes Tier 5 from its surface)
- `docs/active/comparison_surface_contract.md` — wire/canonical byte
  authority rule (applies to message preservation in tx-submission2)
- `docs/active/PHASE4-N-A_invariants.md` §7 — seven closed decisions
  binding this cluster
- `docs/active/PHASE4-N-A_cluster_plan.md` — 10-slice authority-aligned
  plan
- External: cardano-node 10.6.2 Ouroboros mini-protocol specs
  (IOG `ouroboros-network` Haskell package)

---

## Entry Conditions

What previous clusters guarantee that N-A builds on:

- **PHASE4-N-D closed** (`436b1d7`): persistence layer ships; N-A does
  not write to ChainDb directly (that's N-B's job), but the persistence
  trait surface is stable
- **`ade_types` is stable**: canonical IDs incl. `TxId`, `SlotNo`,
  `Hash32`, `CardanoEra` — N-A reuses, never redefines
- **`ade_codec` provides CBOR primitives**: `minicbor` integration,
  `PreservedCbor` chokepoint, era-aware block decoders — N-A's message
  codec is independent of block CBOR but reuses primitive encoders
- **`ade_crypto` is stable**: `Hash32` operations available if needed
  for protocol hashing (e.g., handshake magic)
- **BLUE/RED partition mechanically enforced** (`5b70bee`): six CI
  scripts scan the 6-crate BLUE list; adding `ade_network` BLUE
  submodules to `.idd-config.json` `core_paths` is the S-A1 entry
  obligation
- **Registry has 149 entries** with N-A-relevant rules declared
  (CN-WIRE-07, DC-PROTO-01..06, DC-CORE-01)

---

## Exit Criteria (CI-Verifiable)

Each CE names the concrete test or check that closes it. Tests are
forward-defined — they land with the slice that closes the CE. No
human review may substitute for these checks.

| CE | Check | Closed by |
|---|---|---|
| **CE-N-A-1** | `cargo test -p ade_network --test handshake_version_negotiation` PASS across cardano-node 10.6.2 supported version tables, exercised by corpus at `corpus/network/n2n/handshake/` | S-A3 |
| **CE-N-A-2** | `cargo test -p ade_network --test chain_sync_signal_trace` PASS — chain-sync state machine produces same fork-choice signal sequence as cardano-node oracle for curated divergence corpus at `corpus/network/n2n/chain_sync/` | S-A4 |
| **CE-N-A-3** | `cargo test -p ade_network --test block_fetch_frame_corpus` PASS — block-fetch delivers blocks byte-identically and in same order as cardano-node oracle for curated batch at `corpus/network/n2n/block_fetch/` | S-A5 |
| **CE-N-A-4** | `cargo test -p ade_network --test tx_submission2_mempool_trace` PASS — tx-submission2 round-trips curated mempool trace with byte-identical wire frames at `corpus/network/n2n/tx_submission2/` | S-A6 |
| **CE-N-A-5** | `cargo test -p ade_network_interop --test live_cardano_node_session --release -- --ignored` PASS against pinned-Docker cardano-node 10.6.2 — full N2N+N2C session establishes, exchanges blocks, exchanges txs, doesn't drop. Closure-gate evidence captured at `docs/clusters/PHASE4-N-A/CE-N-A-5_<date>.log` | S-A10 |

---

## Expected Slice Types

Concrete to this cluster, not generic:

- **S-A1**: RED substrate slice — mux/framing module + crate scaffold + sync-only CI gate (`ci_check_no_async_in_blue.sh`)
- **S-A2**: BLUE canonical-type-introduction slice — ~70 closed message enums + CBOR codecs for all 11 protocols
- **S-A3..S-A8**: BLUE state-machine-definition slices — pure transition functions per protocol with typed `*Version`, `*State`, `*Agency`, `*Output` enums
- **S-A9**: RED + GREEN composition + replay-corpus slice — session wiring, frame corpus, evidence schema for CE-N-A-5
- **S-A10**: RED release-evidence slice — pinned-Docker cardano-node interop transcript

---

## TCB Color Map (FC/IS Partition)

| Module | Color | Constraint |
|---|---|---|
| `ade_network::codec` | **BLUE** | Pure CBOR transformation. No I/O, no async, no time. |
| `ade_network::handshake` | **BLUE** | Pure version-negotiation state machine. |
| `ade_network::chain_sync` | **BLUE** | Pure transition; emits values, not effects. |
| `ade_network::block_fetch` | **BLUE** | Pure transition; outputs are frame values. |
| `ade_network::tx_submission` | **BLUE** | Pure transition + inventory state. Reuses `ade_types::TxId`. |
| `ade_network::keep_alive` | **BLUE** | Pure cookie protocol. |
| `ade_network::peer_sharing` | **BLUE** | Pure message exchange. Emits `PeerSharingOutput` events; does NOT own a peer book. |
| `ade_network::n2c` | **BLUE** | All 4 N2C state machines. LSQ codec owns closed wire grammar, NOT ledger meaning. |
| `ade_network::mux` | **RED** | Sockets, tokio, framing, flow control. The only place tokio first appears. |
| `ade_network::session` | **RED** | Composition glue: socket ↔ mux ↔ codec ↔ state machine. Holds selected version; threads it as explicit input to BLUE transitions. |
| `crates/ade_network/src/bin/ade_network_interop.rs` | **RED** | Live cardano-node interop driver binary. |
| `ade_testkit::network` | **GREEN** | Frame corpus harness, transcript replay. Non-authoritative test infrastructure. |

Color must be resolved before any slice in this cluster begins. Open
color questions from the invariants sketch are all resolved per §7
closed decisions.

Rules (inherit per global IDD doctrine):
- No RED behavior may appear in BLUE code.
- GREEN code must not affect authoritative outputs.

---

## Forbidden During This Cluster

Slice-level hard prohibitions inherit from this list:

- **Any of**: `async fn`, `.await`, `tokio::`, `async_std::`, `Future`, `futures::`, task spawn, async channels, timers in **any BLUE module** (DC-CORE-01; mechanically enforced by `ci_check_no_async_in_blue.sh` after S-A1)
- **Plugin-style runtime registration** of message types (DC-PROTO-03/04 require closed enums; no `dyn MessageHandler` patterns)
- **Direct dependency on `pallas-network` in production builds** — CI oracle only, same quarantine pattern as `ci_check_pallas_quarantine.sh` enforces for the ledger side
- **Redefinition of `TxId` (or any canonical ID) in `ade_network`** — must depend on `ade_types`
- **Hidden version state in RED affecting BLUE transition behavior** (DC-PROTO-06; selected version is threaded as explicit input, never read from session context)
- **Codec-as-opaque-bytes framing** (decision §7 #3) — codec owns closed wire grammar (closed discriminants, version-gated rejection, structured errors); semantic interpretation lives elsewhere
- **Generic `Agency<P>` wrapper unifying per-protocol agency types** (decision §7 #7) — each protocol has its own non-interchangeable agency enum
- **Deferred validation or TODO logic in any authoritative path** — typed errors must be structured at the wire, not strings or placeholder enums
- **Live interop dependency on operator-provided peer only** (decision §7 #6) — the closure gate must reproduce via pinned Docker

---

## Slices

| ID | Name | TCB | Closes |
|---|---|---|---|
| **S-A1** | Mux/framing + `ade_network` scaffold + sync-only CI gate | RED + CI | — (substrate) |
| **S-A2** | Protocol message codec authority (all 11 protocols) | BLUE | partial-CE-N-A-1..4 (codec layer) |
| **S-A3** | Version negotiation authority (N2N + N2C handshake) | BLUE | **CE-N-A-1** |
| **S-A4** | Chain-sync transition authority | BLUE | **CE-N-A-2** |
| **S-A5** | Block-fetch transition authority | BLUE | **CE-N-A-3** |
| **S-A6** | Tx-submission2 transition authority | BLUE | **CE-N-A-4** |
| **S-A7** | Keep-alive + peer-sharing transition authority | BLUE | — |
| **S-A8** | N2C transition authority (LocalChainSync + LocalTxSubmission + LocalStateQuery + LocalTxMonitor) | BLUE | — |
| **S-A9** | Session wiring + frame corpus + replay harness | RED + GREEN | partial-CE-N-A-5 (reproducibility) |
| **S-A10** | Live cardano-node interop release evidence | RED | **CE-N-A-5** |

Slice docs (`S-A1.md` ... `S-A10.md`) land in this directory as each
`/slice-doc PHASE4-N-A S-AN` runs.

---

## Engineering Surface (Forward-Looking)

Crate layout planned for S-A1:

```
crates/ade_network/
  Cargo.toml
  src/
    lib.rs
    codec/
    handshake/
    chain_sync/
    block_fetch/
    tx_submission/
    keep_alive/
    peer_sharing/
    n2c/
    mux/
    session/
    bin/
      ade_network_interop.rs
```

Test infrastructure additions:

```
crates/ade_testkit/src/network/        # frame corpus harness (GREEN)
corpus/network/
  n2n/{handshake,chain_sync,block_fetch,tx_submission2,keep_alive,peer_sharing}/
  n2c/{handshake,local_chain_sync,local_tx_submission,local_state_query,local_tx_monitor}/
```

New CI scripts:

```
ci/ci_check_no_async_in_blue.sh        # enforces DC-CORE-01 (lands S-A1)
```

The 5 existing BLUE-scope scripts (`ci_check_module_headers.sh`,
`ci_check_no_semantic_cfg.sh`, `ci_check_no_signing_in_blue.sh`,
`ci_check_hash_uses_wire_bytes.sh`, `ci_check_ingress_chokepoints.sh`,
`ci_check_dependency_boundary.sh`, already at 6-crate scope per
`5b70bee`) gain the 8 `ade_network` BLUE submodules in scope when
`ade_network` is added to `.idd-config.json` `core_paths` (S-A1 entry
obligation).

---

## Replay Obligations (Cluster-Level)

- **New canonical types**: ~70 enums across the 11 protocols. Each
  round-trip-tested in S-A2.
- **New replay corpus**: `corpus/network/{n2n,n2c}/<protocol>/` with
  per-frame metadata schema (cardano-node version, network magic,
  protocol, mini-protocol version, direction, agency, raw bytes,
  expected decode result, expected re-encode bytes).
- **Per-slice replay equivalence MAC**: every BLUE slice's per-protocol
  tests must replay byte-identically.
- **Cluster-level replay MAC**: replayed end-to-end, the corpus
  produces the same transcript hash as cardano-node↔cardano-node on
  the same canonical inputs (CE-N-A-5).

---

## Invariants Strengthened By This Cluster

| Slice | Strengthens (registry status flip pending until enforcement lands) |
|---|---|
| S-A1 | DC-CORE-01 (sync-only BLUE via `ci_check_no_async_in_blue.sh`); DC-PROTO-02 partial (mux framing portion) |
| S-A2 | CN-WIRE-07, DC-PROTO-03, DC-PROTO-04, T-ENC-03 |
| S-A3 | DC-PROTO-05 |
| S-A4 | DC-PROTO-01, DC-PROTO-02 (chain-sync portion), DC-PROTO-06 |
| S-A5 | DC-PROTO-01, DC-PROTO-02 (block-fetch portion), DC-PROTO-06 |
| S-A6 | DC-PROTO-01, DC-PROTO-02 (tx-submission portion), DC-PROTO-06 |
| S-A7 | DC-PROTO-01, DC-PROTO-06 |
| S-A8 | DC-PROTO-01, DC-PROTO-04, DC-PROTO-06 |
| S-A9 | DC-PROTO-02 (mux + replay-corpus portion) |
| S-A10 | DC-PROTO-02 (closes via live transcript equivalence) |

DC-PROTO-02 is incrementally strengthened across S-A1, S-A4, S-A5,
S-A6, S-A9 and closes at S-A10.

---

## Authority Reminder

This cluster doc is a planning aid. Authority for invariants belongs
to `docs/ade-invariant-registry.toml`. Authority for mechanical
acceptance belongs to the named tests/CI checks above. If guidance
here conflicts with normative documents, normative documents win.
