# Cluster PHASE4-N-D — Chain DB & Persistence

> **Tier**: 5 (intentional divergence on storage layout).
> **Origin**: Phase 4 cluster N-D from `docs/active/phase_4_cluster_plan.md`.
> **Status snapshot**: `docs/active/phase_4_status.md` (load-bearing).

## Purpose

Establish the durability foundation: block storage, ledger state
persistence, snapshot management, and recovery from unclean shutdown.
Phase 4's first cluster pursued.

## Tier rationale (what's better than cardano-node)

- Single backing store (redb) with logical separation via key prefixes,
  replacing cardano-node's three-DB pattern (ImmutableDB + VolatileDB +
  LedgerDB).
- Snapshots as compact CBOR blobs using Ade's canonical fingerprint
  format — not Haskell-disk parity.
- Recovery: load latest snapshot + replay forward from immutable store.
  No full genesis replay path as the primary.
- Backup/restore is single-file copy + checksum.
- File extension `.chaindb` so the format identity is project-controlled,
  not backing-store-controlled.

## Headline CEs

- **CE-N-D-1** — chain DB survives 1,000 random kill-9 events on a
  synthetic workload with zero corruption (checksum-verified).
- **CE-N-D-2** (Tier 5 improvement target) — warm restart latency
  ≤ 30s for state at chain tip. No comparable cardano-node SLA.
- **CE-N-D-3** (Tier 5 improvement target) — on-disk state size
  ≤ 50% of cardano-node's equivalent at the same slot.

## Slices

| Slice | Surface | Tier | State |
|---|---|---|---|
| [S-33](S-33.md) | `ChainDb` trait + `InMemoryChainDb` | 1/5 | Closed (`994203b`) |
| [S-34](S-34.md) | `PersistentChainDb` (redb), `SyncCadence`, `ChainDbError` | 5 | Closed (`fb4a5d4`) |
| [S-35](S-35.md) | `SnapshotStore` trait + impls | 1/5 | Closed (`e52fe9f`) |
| [S-36](S-36.md) | Snapshot + forward-replay recovery (`Recoverable`) | 1 | Closed (`5eecc8a`) |
| [S-37](S-37.md) | Subprocess SIGKILL stress harness; closes CE-N-D-1 | 5 | Bundled in `2047c42` (IDD scaffolding commit) |

## Engineering surface

`ade_runtime::chaindb`:
- `ChainDb` trait — block storage
- `SnapshotStore` trait — state snapshots
- `InMemoryChainDb` — `Mutex<BTreeMap>`-backed, test-only
- `PersistentChainDb` — redb-backed, the real path
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

## Tier discipline

- Backing-store crate (`redb`) imports appear only in
  `crates/ade_runtime/src/chaindb/persistent.rs`.
- No `ade_ledger` dependency in `ade_runtime` — recovery is generic over
  `Recoverable`.
- Schema versioning: chaindb file format is v2 (S-35 added the
  `snapshots_by_slot` table). v1 files open and auto-upgrade.
- Single-file backup: `cp file.chaindb backup.chaindb` is enough.

## Exit gate

CE-N-D-1 closed at `<S-37 commit>` per the stress-kill-1000 evidence
log (see `docs/active/phase_4_status.md`). CE-N-D-2 and CE-N-D-3 are
Tier 5 improvement targets that accumulate evidence as cluster N-D
matures; they do not gate cluster closure.

Cluster is **near-closed**: ships when CE-N-D-1 evidence is logged
under the canonical layout and `/cluster-close` is run.
