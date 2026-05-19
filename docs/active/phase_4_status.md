# Phase 4 Status Snapshot

> **Purpose**: ground state of Phase 4 for continuity across sessions
> and contributors. Read this before picking up Phase 4 work.
>
> **Authority**: planning doc, not a claim doc. Authoritative closure
> statements live in `docs/ade-invariant-registry.toml`. This file
> summarizes their state and the slice-level progress against the
> cluster plan.

**Cluster plan**: `docs/active/phase_4_cluster_plan.md`.

**Tier doctrine**: `docs/active/CE-79_gate_statement.md` plus the
Tier 5 addendum at `docs/active/CE-79_tier5_addendum.md`.

---

## What Phase 4 Was Supposed To Deliver

Lift the project from a ledger-correctness oracle into a running
node. Conform on hash-critical wire surfaces (mini-protocols, block
production output, consensus rules); deliberately diverge on
operator-facing internal surfaces (chain DB, query layer,
configuration, telemetry, packaging) to make the implementation
worth adopting.

Six clusters: N-A through N-F. See cluster plan for shape.

---

## Cluster Status

| Cluster | Tier | State | Notes |
|---|---|---|---|
| **N-A** Ouroboros mini-protocols (N2N + N2C) | 1 | Not started | Gates real chain-sync. Largest cluster. |
| **N-B** Consensus runtime | 1 semantic | Not started | Depends on N-A for chain data. |
| **N-C** Block production | 1 | Not started | Opt-in; depends on N-A/B/D. |
| **N-D** Chain DB & persistence | 5 | **Slices 1-4 shipped; S-37 closure gate run logged** | First Phase 4 cluster pursued. |
| **N-E** Mempool | 1 + 5 | Not started | Depends on N-A tx-submission, N-B chain state. |
| **N-F** Operator surface | 5 | Not started | Query/IPC, telemetry, config, packaging. |

---

## Cluster N-D Detail

| Slice | Surface | Tier | State |
|---|---|---|---|
| **S-33** | `ChainDb` trait + `InMemoryChainDb` | 1 (trait) / 5 (layout) | **Closed** (commit `994203b`) |
| **S-34** | `PersistentChainDb` (redb) + `SyncCadence` + `ChainDbError` | 5 (backing store) | **Closed** (commit `fb4a5d4`) |
| **S-35** | `SnapshotStore` trait + impls | 1 (trait) / 5 (storage) | **Closed** (commit `e52fe9f`) |
| **S-36** | Snapshot+forward-replay recovery (`Recoverable` trait) | 1 (contract) | **Closed** (commit `5eecc8a`) |
| **S-37** | Subprocess SIGKILL stress harness; closes CE-N-D-1 | 5 (impl-specific) | **In-flight; closure gate logged below** |

### Engineering surface

`ade_runtime::chaindb`:
- `ChainDb` trait (block storage)
- `SnapshotStore` trait (state snapshots)
- `InMemoryChainDb` (Mutex<BTreeMap>-backed; test-only)
- `PersistentChainDb` (redb-backed; the real path)
- `PersistentChainDbOptions { path, sync_policy }`
- `SyncCadence::{PerWrite, Manual}`
- `ChainDbError::{Io, Corruption, SchemaMismatch, InvalidOperation}`
- `run_contract_tests`, `run_snapshot_contract_tests`
- `run_crash_safety_tests` + `KillStrategy` + `NoKill` (S-37 wires
  real fault injection via subprocess)

`ade_runtime::recovery`:
- `Recoverable` trait
- `recover<C, S, R>(chaindb, snapshots, genesis)` entry point
- `RecoveryReport<R>`, `RecoveryError<E>`, `StartingState`

### Tier discipline

- Backing-store crate (`redb`) imports appear only in
  `crates/ade_runtime/src/chaindb/persistent.rs`.
- No `ade_ledger` dependency in `ade_runtime` (recovery is generic
  over `Recoverable`).
- Schema versioning: chaindb file format is v2 (S-35 added the
  `snapshots_by_slot` table). v1 files open and auto-upgrade.
- Single-file backup: `cp file.chaindb backup.chaindb` is enough.
- File extension `.chaindb` (not `.redb`) so the format identity
  is project-controlled, not backing-store-controlled.

---

## CE-N-D-1 Closure Evidence

Gate test: `stress_kill_1000` in
`crates/ade_runtime/tests/stress_kill_harness.rs`.

Run command:
```
cargo test -p ade_runtime --test stress_kill_harness stress_kill_1000 \
    --release -- --ignored --nocapture
```

Most recent run log: `target/ce-evidence/CE-N-D-1_2026-05-02.log`.

Per-iteration invariants (from S-37 obligation discharge §O-37.3):
1. Reopen succeeds (no `Corruption` error).
2. Schema version is intact.
3. `tip()` is consistent (or `None` if pre-first-write).
4. Tip block readable via both slot and hash indices.
5. Hash index entries map to existing slot index entries.
6. Full slot-iter completes without error.

CE-N-D-1 status: **closed at this commit (`<S-37 commit>`)**.
Re-run is not required unless the persistent impl or harness
changes. The smoke variant (10 iterations) runs every `cargo test`
and catches regressions between manual gate runs.

---

## CE-N-D-2 / CE-N-D-3 (open, evidence-driven)

These are Tier 5 *improvement targets*, not gating CEs. They
accumulate evidence as cluster N-D matures and inform the operator
pitch.

- **CE-N-D-2**: warm-restart latency ≤ 30s for state at chain tip.
  No measurement yet — needs a benchmark harness (sibling slice,
  not yet planned).
- **CE-N-D-3**: on-disk state size ≤ 50% of cardano-node's
  equivalent at the same slot. No measurement yet — needs a
  reference comparison harness.

Both are deferred until cluster N-B (consensus runtime) provides
real chain data to feed the chaindb.

---

## What's Next

After cluster N-D closes (S-37 in-flight), the natural progression
is **cluster N-A: Ouroboros mini-protocols**. With persistence
working, the next layer up is networking — handshake, chain-sync,
block-fetch are the prerequisites for the chaindb to be fed real
chain data instead of synthetic test blocks.

Cluster N-B (consensus) sits between N-A and meaningful node
operation; N-C (block production) is opt-in and last; N-E and N-F
can run in parallel after N-A.

---

## Authority Reminder

This document is descriptive, not prescriptive. Authority for cluster
N-D's CEs (N-D-1, N-D-2, N-D-3) belongs to `docs/ade-invariant-registry.toml`
once the registry entries are added; for now, evidence pointers in
this doc are the closure record.
