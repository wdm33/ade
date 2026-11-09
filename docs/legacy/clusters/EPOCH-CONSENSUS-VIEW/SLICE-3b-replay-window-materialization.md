# EPOCH-CONSENSUS-VIEW — S3b (scope): temporary replay-window materialization

> **Status:** SCOPED (2026-06-20, pre-code) — UMBRELLA scope. The SECOND sub-slice of Slice 3 (`SLICE-3-scope.md`). Window-seed source RESOLVED = Option B (disk-backed reduced-UTxO checkpoint). SPLIT into S3b-1 (`SLICE-3b-1-reduced-utxo-checkpoint.md`) + S3b-2 (`SLICE-3b-2-windowed-advance.md`), each scoped + implemented one at a time. GREEN substrate wiring; no live producer-path change. No code until the sub-slice docs are accepted.

## Purpose
Drive a bounded replay window over Ade's OWN validated blocks/checkpoints — with `track_utxo=true` WITHIN the window only — materializing the reduced UTxO on disk (the Slice-1 `TransientEpochViewStore`, DC-EVIEW-01), accumulating the cert/delegation/pool/reward state, and populating the Slice-3a `PointerMap` (each `StakeRegistration`'s `(slot,txIx,certIx)` → credential), then DISPOSING the window. S3b produces the INPUTS that S3c's aggregation consumes (reduced UTxO + delegation map + reward balances + pointer map). It computes no aggregate, emits no view, alters no live path.

## TCB boundary (binding)
The temporary store is GREEN execution support — GREEN while built, read, and disposed; never authority, never "inside BLUE." The pure PROJECTION it later feeds (the S3c aggregate / the `EpochConsensusView`) is BLUE once emitted. GREEN store → BLUE projection.

## The window-seed source — RESOLVED: Option B (disk-backed reduced-UTxO checkpoint) [user, 2026-06-20]
`materialize_rolled_back_state` (`rollback/materialize.rs:43`) seeds a window from `reader.nearest_le(target.slot)`, but computing the stake snapshot at boundary E needs the FULL UTxO as of E, and **Ade's live path keeps NO UTxO** (every live checkpoint is `track_utxo=false`; the only `track_utxo=true` LedgerState sets are the test snapshot-loader `snapshot/ledger.rs:203` + a fingerprint test). The chosen shape:

**Option B — a disk-backed REDUCED-UTxO CHECKPOINT the window advances.** Ade persists a COMPACT reduced UTxO (`TxIn → (Coin, StakeRefClass)`) that the boundary window advances; each window replays only ~one epoch from the prior boundary's checkpoint. This is the "minimal native state" from the EPOCH-VIEW-MINIMUM-AUTHORITY analysis — the irreducible credential-attributed unspent-output state, as the LEDGER TRANSITION'S OWN disk-backed projection. It keeps the window genuinely bounded as the chain grows (Option A — replay-from-bootstrap — was rejected: O(chain-since-bootstrap), degrading).

**Hard framing (binding):**
- The reduced-UTxO checkpoint is the SINGLE ledger authority's own projection — NOT a permanent parallel StakeView, NOT a second stake computation. It is reconstructible by replay (a pure function of admitted blocks), so it is a durable CACHE of a BLUE-derivable projection, not an independent authority; if lost/corrupt, it is rebuilt by replay.
- It is advanced LAZILY at epoch boundaries by the windowed replay (with `track_utxo=true` IN THE WINDOW only) — NOT per-block. The live follow/forge path stays `track_utxo=false` (no per-block UTxO; the BA-08 memory posture is preserved). This is the key distinction from the rejected per-block parallel index.
- It is disk-backed, crash-safe, and replay-equivalent (DC-WAL-03 lineage). The reduced record drops datums/scripts/multi-asset — only `(Coin, StakeRefClass)` per live output.

### Option-B structure
1. **The durable reduced-UTxO checkpoint** (`TxIn → (Coin, StakeRefClass)`) + the accumulated cert/delegation/pool/reward state + the `PointerMap` at each epoch boundary. Built ONCE from the bootstrap UTxO import (the one full UTxO Ade held — reduced to `(coin, stake_ref)`), then advanced per-boundary.
2. **The boundary window:** at boundary E, load the E−1 reduced-UTxO checkpoint → replay epoch E−1's admitted blocks forward (the LEDGER'S OWN `track_utxo`-style apply — remove spent `TxIn`s, add new outputs as `(coin, stake_ref)`, process certs → delegation/pool/reward + `PointerMap` population) into the transient store → the result is the reduced UTxO at E → the new checkpoint (and the input S3c aggregates).
3. **Disposal:** the transient working store (Slice-1) is disposed after the window; the reduced-UTxO CHECKPOINT persists (that is its purpose). GREEN store → BLUE projection.

### DECIDED: split into S3b-1 + S3b-2 [user, 2026-06-20]
Option B makes S3b large enough that it is SPLIT (scoped + implemented one at a time):
- **S3b-1 — the durable reduced-UTxO checkpoint** (DC-EVIEW-04): the `(TxIn → (Coin, StakeRefClass))` checkpoint type + its disk backing + crash-safety + replay-equivalence + the one-time bootstrap reduction. The foundational minimal-state piece, provable alone. Scope doc: `SLICE-3b-1-reduced-utxo-checkpoint.md`.
- **S3b-2 — the windowed advance** (DC-EVIEW-04b): load checkpoint → replay one epoch (reduced ledger apply + cert/pool/reward + `PointerMap`) → produce the next checkpoint, bounded + crash-safe + replay-deterministic. Scope doc: `SLICE-3b-2-windowed-advance.md`.
This `SLICE-3b-*.md` is the umbrella scope; the two sub-slice docs carry the per-slice MAC + entry obligations.

## Reuse vs net-new (grounded)
- **Reuse:** `materialize_rolled_back_state` (`rollback/materialize.rs:43`, the snapshot-nearest_le + replay-forward engine) + the `SnapshotReader`/`BlockSource` traits (`rollback/traits.rs:27/38`); the cert/delegation/pool/reward accumulation (`delegation.rs`, runs naturally when the window's `LedgerState.track_utxo=true` via `process_block_certificates` `rules.rs:1403`, no cert-path change); `TransientEpochViewStore` (Slice-1, DC-EVIEW-01); `PointerMap` (Slice-3a, DC-EVIEW-03); the S3a/Slice-2 classifier for the reduced record's `stake_ref`.
- **Net-new:** the window-seed source (the OPEN decision above); a `SnapshotReader` that yields a `track_utxo=true` seed; routing the fold's produced outputs into the transient store (`materialize_batch`, disk-backed — proven standalone in DC-EVIEW-01 but with zero live callers today); the `PointerMap` population (threading `(slot, txIx)` alongside the existing `cert_index` at each `StakeRegistration`); window disposal.

## Invariant (proposed DC-EVIEW-04)
The window is BOUNDED (RollbackTooDeep / k=2160; Babbage|Conway only), DISK-BACKED (RssAnon bounded per DC-EVIEW-01 GATE-MEM — the reduced UTxO lives on disk, not the anonymous heap), CRASH-SAFE + FAIL-CLOSED-PURGED (DC-EVIEW-01 GATE-CRASH/PURGE), `track_utxo=true` CONTAINED to the window (GATE-NOT-LIVE: the live `--mode node` producer stays `track_utxo=false`; `ci_check_transient_view_not_live.sh` green), REPLAY-DETERMINISTIC (same admitted blocks → byte-identical window state, two-run; DC-WAL-03 lineage), and GREEN (the store never becomes authority; the projection it feeds is BLUE).

## MAC (hermetic; no leader slot)
1. **Window materialization:** a window replay over a fixture chain materializes the expected reduced UTxO on disk (`len()==N`, bounded RssAnon delta per DC-EVIEW-01) + the expected cert/delegation/pool/reward state + the populated `PointerMap` (a `StakeRegistration` at `(slot,txIx,certIx)` resolves to its credential).
2. **Crash-safe:** SIGKILL mid-window (mid-materialize and mid-dispose) leaves the durable ChainDb/WAL/checkpoint byte-unchanged, the next replay produces identical verdicts, and the transient root is empty/purged before resume (DC-EVIEW-01 GATE-CRASH/PURGE carried).
3. **Not-live:** the live producer path never enables `track_utxo=true` via S3b; the transient store is reachable only from the window driver (GATE-NO-FALLBACK/NOT-LIVE).
4. **Replay-deterministic:** two window runs over the same blocks → byte-identical reduced UTxO + cert state + pointer map.
5. **CI gate** `ci/ci_check_eview_replay_window.sh`: the window driver exists, `track_utxo=true` is contained (no live caller), the transient store is disposed, the gate is non-vacuous.

## Entry obligations
1. **Window-seed source — RESOLVED: Option B** (disk-backed reduced-UTxO checkpoint; see above).
2. **The reduced-UTxO checkpoint durability mechanism (answer before code):** how is the `(TxIn → (Coin, StakeRefClass))` checkpoint persisted + made crash-safe + replay-equivalent? Reuse the redb on-disk anchor (`chaindb/utxo_anchor.rs`, the same backend DC-EVIEW-01 proved transient)? a dedicated store? And its relationship to the existing checkpoint/WAL (a distinct artifact keyed by boundary epoch; NOT under the live `chain.db`/WAL authority). Define the crash-safety + the replay-equivalence obligation (rebuildable from the chain if lost).
3. **The reduced-UTxO record:** `TxIn → (Coin, StakeRefClass)` (from Slice-2/S3a) — drop datums/scripts/multi-asset. Confirm the transient store's `materialize_batch` shape accommodates it (DC-EVIEW-01 used a reduced TxOut already), and the bootstrap reduction (the one full UTxO → reduced once).
4. **PointerMap population:** thread `(slot, txIx)` into the cert accumulation alongside the existing `cert_index` (`rules.rs:1516`) so a `StakeRegistration` populates the map at `(slot,txIx,certIx)`. Confirm the slot/txIx are in scope at `process_block_certificates` (`rules.rs:1403`, per-block → has the slot).
5. **The window advance = the ledger's own apply (single authority):** confirm the windowed reduced apply (remove spent `TxIn`s + add reduced outputs + process certs) routes through the LEDGER's `track_utxo` apply path, NOT a parallel reimplementation — so the reduced UTxO is the ledger transition's projection, not a second computation.
6. **Window bounds + the SnapshotReader/BlockSource over Ade's durable store:** the window = (prior boundary checkpoint → next snapshot point], ~one epoch; the production `SnapshotReader`/`BlockSource` impls (PersistentSnapshotCache / ChainDbBlockSource); the k=2160 / RollbackTooDeep bounds.
7. **The k / stability interaction:** the window must not finalize a not-yet-k-deep boundary (that gate is S3d, but confirm S3b targets a settled point or hands the stability decision to S3d).
8. **Sub-split decision — RESOLVED: split** into S3b-1 (the checkpoint) + S3b-2 (the windowed advance) [user, 2026-06-20]. Obligations 2–7 are carried into the two sub-slice docs (the checkpoint durability/crash-safety/replay-equivalence + bootstrap reduction → S3b-1; the windowed advance + PointerMap population + ledger's-own-apply + window bounds + stability → S3b-2).

## Hard prohibitions / non-goals
- NO stake aggregation / per-pool sum (S3c); NO snapshot/emission (S3d/S3e); NO activation (DC-EVIEW-08); NO leader/header use.
- NO `track_utxo=true` on the live follow/forge path (only inside the disposed window).
- NO permanent parallel StakeView — any reduced-UTxO checkpoint (Option B) is the ledger transition's OWN projection, single authority, disk-backed, replay-equivalent, NOT a standing in-RAM index.
- The transient store is GREEN, disposed after use; never fallback authority.

## Where it sits
S3a (committed `c71a308f`, DC-EVIEW-03) gave the pointer decode + the `PointerMap` resolution algorithm. S3b drives the window that POPULATES that map + materializes the reduced UTxO + accumulates the cert/reward state — the inputs S3c aggregates into the per-pool stake (the linchpin, oracle-matched vs cardano-cli). S3b emits nothing live.
