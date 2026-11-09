# EPOCH-CONSENSUS-VIEW — S3b-1 (scope): the durable reduced-UTxO checkpoint

> **Status:** SCOPED (2026-06-20, pre-code). The FIRST half of S3b (umbrella `SLICE-3b-replay-window-materialization.md`, Option B). The foundational minimal-state piece, provable ALONE. GREEN durable cache; no live producer-path change; no advance (S3b-2), no aggregation (S3c). Proposed invariant `DC-EVIEW-04`. No code until accepted.

## Purpose
A DURABLE, disk-backed **reduced-UTxO checkpoint** — `TxIn → (Coin, ReducedStakeRef)` — built ONCE from Ade's bootstrap UTxO import (the one full UTxO Ade held), crash-safe and replay-equivalent. This is the "minimal native state" (Option B): the single ledger authority's own reduced-UTxO projection, reconstructible by replay, that S3b-2 advances per epoch boundary and S3c aggregates. S3b-1 builds + persists + proves the checkpoint; it does NOT advance it (S3b-2) and computes no stake (S3c).

## TCB boundary (binding)
The reduced-UTxO checkpoint is a GREEN durable CACHE of a BLUE-derivable projection — the ledger transition's own reduced UTxO, a pure function of admitted blocks (reconstructible by replay if lost/corrupt). It is NOT authority, NOT a second stake computation, NOT a permanent parallel StakeView, and NEVER on the live follow/forge path. The live producer stays `track_utxo=false`; this checkpoint is built/advanced lazily (S3b-2), off the per-block path. GREEN cache → the BLUE `EpochConsensusView` it eventually feeds.

## The reduced record + the ERA-INDEPENDENCE question (key entry obligation)
`TxOut` is `Byron{address: Address, coin} | ShelleyMary{address: Vec<u8>, value} | AlonzoPlus{raw, address: Vec<u8>, coin}` (`ade_ledger/src/utxo.rs:18-34`). The reduced record drops datums/scripts/multi-asset, keeping `(Coin, ReducedStakeRef)`:
- Byron → `(coin, Null)` (bootstrap, non-delegable).
- ShelleyMary / AlonzoPlus → `(value.coin / coin, <stake reference from the address bytes>)`.

**Representation — RESOLVED: option (b), Conway-specialized [user, 2026-06-20].** `ReducedStakeRef = Base(StakeCredential) | NonContributing`. Ade bootstrapped into Conway and only ever snapshots at Conway, where pointer stake is retired and only base-address stake credentials contribute — so a base address → `Base(cred)`, and pointer / enterprise / Byron / reward / malformed → `NonContributing`. This is correct for EVERY Conway snapshot, and the reduction reuses Slice-2's `classify_output_stake_ref(addr_bytes, CardanoEra::Conway)` directly: `Base(cred)` → `Base(cred)`; `Null` / `Pointer` / `Reject` → `NonContributing`. The era gate is thus trivially satisfied (Conway = Base-only); S3c sums `Base` credentials' coins per pool. (S3a's pointer-coords machinery stays available for the general tx-validity surface; the reduced stake checkpoint does not need it at Conway.)

## Reuse vs net-new (grounded)
- **Reuse:** the redb on-disk anchor backend (`ade_runtime/src/chaindb/utxo_anchor.rs`, the SAME backend DC-EVIEW-01 proved transient + crash-safe — here used DURABLY, not disposed); the bootstrap UTxO import (`admission/bootstrap.rs:165` `import_cardano_cli_json_utxo`, available before `drop(utxo)` at `:298`); Slice-2/S3a address decoding for the `ReducedStakeRef`.
- **Net-new:** the `ReducedStakeRef` type + the era-independent reduction `TxOut → (Coin, ReducedStakeRef)`; the DURABLE reduced-UTxO checkpoint store (distinct artifact, keyed by bootstrap anchor / epoch, NOT under the live `chain.db`/WAL authority); the completeness/crash-safety model (a partial bootstrap reduction must NOT be mistaken for a complete checkpoint); the one-time bootstrap reduction driver.

## Invariant (proposed DC-EVIEW-04)
The reduced-UTxO checkpoint is: DISK-BACKED + DURABLE (persists across restarts); CRASH-SAFE (a crash mid-build leaves no partial checkpoint treated as complete — fail-closed completeness marker + atomic commit; rebuild-on-incomplete); REPLAY-EQUIVALENT (a pure function of the source UTxO — two builds byte-identical; reconstructible if lost); ERA-STABLE (stores the era-independent reference; the era gate is S3c's); and GREEN (a reconstructible cache, never authority, never the live path, never `track_utxo=true` on live).

## MAC (hermetic; no leader slot)
1. **Bootstrap reduction:** a fixture UTxO reduces to the expected `(Coin, ReducedStakeRef)` per entry — base→`Base(cred)`; pointer / enterprise / Byron / reward / malformed → `NonContributing` (option b, Conway). `len()==N`.
2. **Durable round-trip:** the checkpoint persists across reopen (byte-identical), `len()==N`, every entry resolves to its stored `(coin, ref)`.
3. **Crash-safe build:** SIGKILL mid-bootstrap-reduction → on restart the partial checkpoint is detected as INCOMPLETE (not mistaken for complete) and rebuilt; the durable `chain.db`/WAL are byte-unchanged.
4. **Replay-equivalence:** two builds from the same source UTxO → byte-identical checkpoint (DC-WAL-03 lineage).
5. **Not-live / GREEN:** the live producer path never enables `track_utxo=true` via S3b-1; the checkpoint is reachable only from the S3b driver, never the live follow/forge/recovery path.
6. **Bounded build:** the bootstrap reduction is disk-backed (RssAnon bounded per DC-EVIEW-01 GATE-MEM — the reduced UTxO lives on disk, not the anonymous heap).
7. **CI gate** `ci/ci_check_eview_reduced_utxo_checkpoint.sh`: the checkpoint type + the era-independent reduction + the completeness marker exist; no live `track_utxo=true`; the store is not the live authority; non-vacuous.

## Entry obligations (answer before code)
1. **`ReducedStakeRef` representation — RESOLVED: (b) Conway-specialized `Base(StakeCredential) | NonContributing`** [user]. Derived from Slice-2's `classify_output_stake_ref(.., Conway)`; correct for every Conway snapshot; no pointer coords retained.
2. **The durable store + completeness/crash-safety** — reuse the redb anchor durably; the completeness marker (so a SIGKILL mid-reduction is detected + rebuilt, not mistaken for complete); the keying (bootstrap anchor / boundary epoch); its relationship to (and separation from) the live `chain.db`/WAL.
3. **The bootstrap reduction tap** — reduce `&utxo` at `bootstrap.rs:~165-297` BEFORE `drop(utxo)` (`:298`); confirm the full UTxO is in scope there and the reduction streams to disk (bounded RAM).
4. **Replay-equivalence obligation** — the exact two-run byte-identical contract + the reconstructible-if-lost rebuild path (the fallback to a fresh bootstrap reduction).

## Hard prohibitions / non-goals
- NO advance (S3b-2); NO aggregation/sum (S3c); NO snapshot/emission (S3d/e); NO activation (DC-EVIEW-08); NO leader/header use.
- NO `track_utxo=true` on the live follow/forge path; the checkpoint is built/advanced off the per-block path.
- NO permanent parallel StakeView / second stake computation — this is the ledger transition's OWN reduced-UTxO projection, a reconstructible durable cache, single authority.
- The era gate (Conway pointer retirement) is NOT baked into the stored record (it is S3c's, applied at the snapshot era).

## Where it sits
S3a (committed `c71a308f`) gave the era-parameterized pointer decode + the `PointerMap` resolution. S3b-1 builds the durable reduced-UTxO checkpoint (the minimal native state). S3b-2 advances it per boundary (reduced ledger apply + cert/pool/reward + `PointerMap` population). S3c aggregates the advanced checkpoint into the per-pool stake (oracle-matched vs cardano-cli). S3b-1 emits nothing live.
