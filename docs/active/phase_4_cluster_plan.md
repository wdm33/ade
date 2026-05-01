# Phase 4 Cluster Plan — Runtime, Network, Consensus

> **Status**: draft skeleton for review. Establishes cluster structure,
> tier classification, and headline closure entries. Per-slice
> breakdowns and full obligation discharge are deferred to slice-entry
> work once clusters are sequenced.

## Purpose

Phase 4 lifts the project from a ledger-correctness oracle into a
running node. After Phase 3 closure, `apply_block_with_verdicts`
produces correct verdicts. Phase 4 makes those verdicts come from
real chain data over real network connections, persist correctly
across restarts, and (where opt-in) drive block production.

The shape of "Cardano-compatible node" Phase 4 targets is:
- **Conform** on every surface the protocol requires (hash-critical
  + wire mini-protocols + consensus rules).
- **Diverge deliberately** on every other operator-facing surface,
  per the Tier 5 doctrine (`docs/active/CE-79_tier5_addendum.md`),
  to make the implementation worth adopting.

## Tier discipline

Phase 4 surfaces split cleanly between Tier 1 (must-conform) and
Tier 5 (intentional-divergence). Every cluster below names its tier
and a one-line rationale.

## Clusters

### N-A — Network mini-protocols (Tier 1)

Implement the Ouroboros mini-protocol surface that cardano-node speaks
on both N2N (node-to-node) and N2C (node-to-client) families.

- **N2N**: handshake, chain-sync, block-fetch, tx-submission2,
  keep-alive, peer-sharing.
- **N2C**: handshake, local-chain-sync, local-tx-submission,
  local-state-query, local-tx-monitor.

**Tier**: 1. Wire bytes are protocol; non-conformance forks the
network or makes Ade un-peerable.

**Headline CEs**:
- **CE-N-A-1**: handshake version negotiation byte-identical to oracle
  across cardano-node 10.6.2 supported versions.
- **CE-N-A-2**: chain-sync state machine produces the same
  fork-choice signals as oracle on a curated divergence corpus.
- **CE-N-A-3**: block-fetch protocol delivers blocks byte-identically
  and in the same order as oracle for a curated batch.
- **CE-N-A-4**: tx-submission2 round-trips a curated mempool trace
  with byte-identical wire frames.
- **CE-N-A-5**: full N2N + N2C suite passes against a real
  cardano-node peer (live interop test, not just synthetic corpus).

### N-B — Consensus runtime (Tier 1 semantic)

Fork choice (Ouroboros Praos), leader schedule (VRF-based), rollback
handling, HFC schedule (era-to-slot mapping), slot-to-time mapping.

**Tier**: 1. Decisions affect which blocks Ade accepts, which is
observable on the network and gates consensus participation.

**Headline CEs**:
- **CE-N-B-1**: fork choice produces identical best-chain selection
  on every multi-tip case in a curated divergence corpus.
- **CE-N-B-2**: rollback to k-deep produces identical post-rollback
  state as oracle for a curated rollback corpus.
- **CE-N-B-3**: HFC schedule (era-to-slot mapping) matches oracle's
  known hard-fork slots exactly.
- **CE-N-B-4**: leader schedule (VRF-based) produces identical
  slot-leader set as oracle for a curated epoch-replay corpus.

### N-C — Block production (Tier 1 wire + 1 semantic)

VRF leader proof, KES key rotation, opcert lifecycle, block forging.
Optional cluster — Ade can run as a non-producing relay without N-C.

**Tier**: 1. Forged block bytes ARE the chain; non-conformance
produces invalid blocks that peers reject.

**Headline CEs**:
- **CE-N-C-1**: forged block bytes hash-identical to oracle when given
  the same inputs (mempool, state, slot, KES key, VRF key).
- **CE-N-C-2**: KES rotation occurs at the correct slot per protocol
  rules.
- **CE-N-C-3**: opcert renewal flow round-trips through cardano-cli's
  opcert format (interop with the existing operator workflow).

### N-D — Chain DB & persistence (Tier 5)

Block storage, ledger state persistence, snapshot management, recovery
on unclean shutdown.

**Tier**: 5. No protocol requirement on storage layout.

**Tier 5 rationale (what's better)**:
- Single backing store (rocksdb or sled) with logical separation via
  key prefixes, replacing cardano-node's three-DB pattern (ImmutableDB
  + VolatileDB + LedgerDB).
- Snapshots as compact CBOR blobs at chosen intervals, using Ade's
  canonical fingerprint format — not Haskell-disk parity.
- Recovery: load latest snapshot + replay forward from immutable
  store. No full genesis replay path.
- Backup/restore is single-file copy + checksum, not a multi-directory
  ritual.
- Smaller on-disk footprint; faster warm restart.

**Headline CEs**:
- **CE-N-D-1**: chain DB survives 1,000 random kill-9 events on a
  synthetic workload with zero corruption (checksum-verified).
- **CE-N-D-2** (Tier 5 improvement target): warm restart latency
  ≤ 30s for state at chain tip. No comparable cardano-node SLA.
- **CE-N-D-3** (Tier 5 improvement target): on-disk state size
  ≤ 50% of cardano-node's equivalent at the same slot.

### N-E — Mempool

Tx ingest, validation against current ledger state, ordering, eviction
policy, propagation.

**Tier**: mixed.
- Consensus-relevant behavior (every tx must be valid before
  acceptance, no false acceptance of invalid txs): Tier 1.
- Policy (eviction order, prioritization, transparency): Tier 5.

**Headline CEs**:
- **CE-N-E-1** (Tier 1): every tx accepted into Ade's mempool would
  also be accepted by oracle's mempool given the same state.
- **CE-N-E-2** (Tier 1): every tx Ade rejects would also be rejected
  by oracle. No false acceptances of invalid txs (the same hard
  no-false-accept gate as Phase 3 CE-88).
- **CE-N-E-3** (Tier 5): mempool exposes per-tx queue position,
  enter/exit reasons, and eviction history via query API.
  cardano-node's mempool opacity is a known operational pain point.

### N-F — Operator surface (Tier 5)

Query / IPC layer, telemetry, configuration, packaging, observability.

**Tier**: 5. Entirely divergence-allowed; this is where adoption gets
won or lost.

**Tier 5 rationale (what's better)**:
- Native gRPC + HTTP/JSON query layer alongside the cardano-node N2C
  protocol. Operators can use either; tooling that talks to
  cardano-node still works.
- Single TOML config file with sane defaults, validated at startup
  with structured errors.
- Prometheus metrics exposed natively; OpenTelemetry tracing optional.
- Single static Rust binary. No GHC RTS dependency. Cross-compiles
  cleanly to ARM64 / x86_64 across Linux / macOS / Windows.

**Headline CEs**:
- **CE-N-F-1** (Tier 5 with semantic equivalence to Tier 1):
  query API returns semantically equivalent answers to cardano-node's
  LocalStateQuery for every documented query type. Wire format may
  differ; meaning may not.
- **CE-N-F-2** (Tier 5): every operational metric the cardano-node
  `ekg` interface exposes has a Prometheus equivalent in Ade with
  documented semantic mapping.
- **CE-N-F-3** (Tier 5): node parses, validates, and starts from a
  single bootstrap TOML with zero additional file edits required.

## Sequencing

Recommended order (each cluster gates the next where indicated):

1. **N-D Chain DB** (no upstream dependency, Tier 5 — fastest to
   start; establishes durability foundation that everything else
   layers on).
2. **N-A Network mini-protocols** (Tier 1 — gates everything that
   needs real chain data; parallel-startable with N-D once core types
   are stable).
3. **N-B Consensus runtime** (depends on N-A for chain data ingress
   and N-D for state).
4. **N-E Mempool** (depends on N-A tx-submission and N-B chain state).
5. **N-F Operator surface** (depends on N-B for queries; can begin in
   parallel with N-C).
6. **N-C Block production** (Tier 1, opt-in — last; requires N-A,
   N-B, N-D all working).

A non-producing relay node is feasible at end of N-E (N-A + N-B + N-D
+ N-E). Block production (N-C) is the optional capability that lifts
relay → full producer.

## Forbidden patterns

- **No "Phase 4 internal-mode mock network."** Real bytes against a
  real cardano-node peer must come early; otherwise wire conformance
  is never exercised under realistic conditions and Tier 1 surfaces
  rot silently.
- **No collapsing wire and canonical bytes.** The same dual-authority
  rule from Phases 1-3 carries forward. Wire-byte authority for
  hash-critical paths; canonical-byte authority for internal replay;
  never collide them.
- **No Tier 5 surface without a stated rationale.** A surface
  diverging from cardano-node without naming what's better is an
  unclassified surface, not a Tier 5. Reviewers reject.
- **No "we'll match it later" stubs on Tier 1 surfaces.** Tier 1
  conformance is the slice's exit gate, not a follow-up TODO.

## Out of scope

- **PV11+ protocol versions.** Version-scoped to cardano-node 10.6.2
  per CE-91 carry-over. PV11 is a future phase decision.
- **Multi-node consensus testing at scale.** Phase 5+.
- **Light-client / SPV mode.** Phase 6+.
- **Genesis replay path** as a recovery primary.  Recovery is
  snapshot-based; full genesis replay is supported only as an
  operator-explicit fallback.

## Exit criteria summary

Phase 4 closed when:

- Every Tier 1 headline CE in N-A, N-B, N-E (and N-C if pursued) is
  at least Tier 2 (derived) closed against a curated corpus AND a
  live cardano-node interop test.
- Every Tier 5 headline CE has shipped behavior with documented
  rationale, even if quantitative improvement targets (CE-N-D-2,
  CE-N-D-3, etc.) remain in active optimization.
- A non-producing relay node syncs mainnet from genesis (or a
  snapshot) to chain tip and stays at tip without divergence for a
  sustained window.

Tier 5 improvement targets (latency, footprint) may stay open as
ongoing optimization without blocking phase exit. They become
adoption-driver metrics for the operator pitch.

## Authority

This document is a planning skeleton. Authority for specific CEs
belongs to `constitution_registry.toml` once entries are added.
Slice-level work begins per cluster as obligation discharge documents
are written; this plan does not yet break clusters into slices.

## Adjacent doctrine

- `docs/active/CE-79_gate_statement.md` — original four-tier gate
  statement.
- `docs/active/CE-79_tier5_addendum.md` — Tier 5 (intentional
  divergence) addendum.
- `docs/active/comparison_surface_contract.md` — wire/canonical byte
  authority rule.
- `docs/active/CE-73_reclassification.md` — example of correctly
  applying Tier 4 to free up engineering capacity.
