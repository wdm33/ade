# EPOCH-CONSENSUS-VIEW — Slice 3 (scope): native next-epoch view via bounded replay

> **Status:** SCOPED (2026-06-20, pre-code). The aggregation + replay-window + emission slice of the cluster, DECOMPOSED into 5 independently-mergeable sub-slices S3a–S3e + a deferred final ACTIVATION step. No code until accepted. Grounded by two read-only investigations (pointer-varint canonicality vs cardano-ledger; the temporary-replay authority boundary).

## Intent
Form the next epoch's stake/consensus view as a **pure projection of the single ledger authority** — by classifying each output (Slice 2, done), resolving pre-Conway pointers, aggregating stake per registered+delegated pool, forming the mark/set/go snapshots, and emitting one **bound, immutable `EpochConsensusView`** — all by materializing a bounded, disk-backed, **transient** replay window (Slice 1's `TransientEpochViewStore`, DC-EVIEW-01) over Ade's OWN validated blocks/checkpoints, then pruning it. The view is a projection of the existing ledger transition (no second/parallel stake engine, no per-epoch external oracle). This unblocks continuous cross-epoch operation — but **only the final activation step touches the live producer path, and only after the view is differentially proven against Cardano.**

## Hard constraints (binding on every sub-slice)
1. **Pointer decoding uses the PINNED era-parameterized ledger behavior** (below) — match cardano-ledger EXACTLY, even where it accepts bounded aliasing. Ade must NOT substitute a cleaner parser rule that diverges from network semantics.
2. **No permanent StakeView or parallel stake authority.** The aggregation is a projection computed INSIDE the single ledger transition; nothing standing, nothing parallel.
3. **Temporary UTxO is GREEN execution support only** — never fallback authority (DC-EVIEW-01 GATE-GREEN / GATE-NO-FALLBACK carry forward).
4. **No `track_utxo=true` on the normal live follow/forge path.** `track_utxo=true` exists ONLY inside the transient replay window, disposed after emit. The live `--mode node` producer stays `track_utxo=false` (ci_check_transient_view_not_live.sh stays green).
5. **Every emitted `EpochConsensusView` binds**: network, era, epoch, source chain point, checkpoint commitment, nonce, snapshot phase, and a canonical-bytes hash. A view missing any binding is inert.
6. **No slice may make leader decisions or affect header validation until the FULL view is differentially proven against Cardano.** S3a–S3e are observe-only / non-live; activation is the deferred final step, gated on the oracle proof.
7. **Each sub-slice is independently mergeable, replay-verifiable (DC-WAL-03), and leaves NO live producer path altered until the final activation slice.**

## Differential oracle (the acceptance bar, applies at S3c/S3e + the activation gate)
Ade's derived view must match cardano-node EXACTLY at chosen chain points (bootstrap-only role; never a runtime authority): `EpochConsensusView.stake_by_pool` / `total_active_stake` vs `cardano-cli query stake-snapshot` (stakeSet/stakeGo) and `query stake-distribution`; the derived leader schedule vs `cardano-cli query leadership-schedule` for ADE1 — across ≥2 Conway-era boundaries, plus a deliberate pointer-address fixture (proves 0 Conway stake contribution).

---

## PINNED pointer-varint rule (grounded vs cardano-ledger `master` + its tests; load-bearing for S3a)
CIP-19 is SILENT on canonicality → cardano-ledger's implementation is the SOLE authority. The strict check is a **bit-WIDTH check, NOT a minimal-form check** — bounded leading-zero-group aliasing (e.g. `[0x80,0x01]` == `[0x01]`) is **ACCEPTED in all eras**; reject-all-non-canonical would FALSE-REJECT txs cardano-node accepts = divergence. Stored `Ptr` = (u32 slot, u16 txIx, u16 certIx). Era-parameterized on the block's bound protocol major:

| Protocol major (era) | Over-WIDTH varint | Bounded leading-zero alias | Trailing bytes after the 3rd coord |
|---|---|---|---|
| **Conway (9+)** | **REJECT** (bounded group counts; u32 slot / u16 txIx / u16 certIx width check) | **ACCEPT** | **REJECT** |
| **Babbage (7–8)** | **NORMALIZE** — decode u64, clamp the WHOLE 3-tuple to (0,0,0) if ANY coord overflows its width | ACCEPT | **REJECT** |
| **≤Alonzo (2–6)** | NORMALIZE (clamp-3-tuple-to-0) | ACCEPT | accept + crop |

(Replaying an address already accepted into Ade's own store uses the fully-lenient form — never re-reject on replay.) The parser is era-independent in EXISTENCE (pointers still parse post-Conway, spendable); only its STRICTNESS is era-gated. Stake-retirement (PV9 → Null) is a SEPARATE rule already correct in Slice 2. **Re-verify the PV9 gate vs the pinned cardano-node 11.0.1 tag at S3a entry** (the major-9 strict boundary is stable; the ProtVerHigh upper bounds are tag-sensitive).

---

## S3a — Pointer decoding / resolution compatibility (BLUE, pure; lowest risk, no deps)
**Purpose:** the era-parameterized pointer-varint decoder (matching the pinned rule EXACTLY) + the pre-Conway `Ptr → credential` resolution (against a pointer map built from registration certs at `(slot,txIx,certIx)`). Replaces Slice-2's `decode_varint` (which diverges from both regimes — it rejects `>u64`) for the resolution path; Slice-2's classifier stays unchanged (pre-Conway, resolves nothing).
**Reuse / build:** reuse `stake_ref::classify_output_stake_ref` (DC-EVIEW-02) for the form; net-new = the era-parameterized varint decoder + the pointer-map resolution.
**Invariant (proposed DC-EVIEW-03):** pointer decoding matches cardano-ledger's era-gated behavior byte-for-byte (accept bounded aliasing; Conway width-reject; Babbage/Alonzo normalize-clamp); pointer resolution is pre-Conway only, total, deterministic, fail-closed.
**MAC (concrete compatibility — binding):** for EACH target era / protocol-version fixture, Ade's pointer-decode result must EQUAL the pinned cardano-ledger behavior — including alias ACCEPTANCE, NORMALIZATION/CLAMPING (Babbage/≤Alonzo: clamp the whole 3-tuple to (0,0,0) on width overflow), and WIDTH REJECTION (Conway: over-width + trailing → fail). No "canonicalization preference" may override the ledger result. Concretely: the full era matrix above; CIP-19 + cardano-ledger golden vectors; pointer-map resolution to the registered credential; an unresolvable pointer fails closed.
**Prohibitions:** NO reject-all-non-canonical; NO canonicalization preference overriding the ledger; NO aggregation/snapshot/emission/live wiring.

## S3b — Temporary replay-window materialization (GREEN substrate wiring)
**Purpose:** drive a bounded one-epoch replay window over Ade's OWN durable blocks into the Slice-1 `TransientEpochViewStore` — `track_utxo=true` WITHIN the window only — materializing the reduced UTxO on disk, then disposing. Wires the transient store (proven standalone in DC-EVIEW-01, zero live callers) to the replay engine.
**Reuse / build:** reuse `materialize_rolled_back_state` (`rollback/materialize.rs:43`, the snapshot-nearest_le + replay-forward engine) + the `SnapshotReader`/`BlockSource` traits + `TransientEpochViewStore`; net-new = a `SnapshotReader` seeding `track_utxo=true` + routing the fold's produced outputs into `materialize_batch` (disk-backed, the GATE-MEM path).
**Invariant (proposed DC-EVIEW-04):** the window is bounded (≤1 epoch, RollbackTooDeep/k=2160), disk-backed (RssAnon bounded per DC-EVIEW-01 GATE-MEM), crash-safe + fail-closed-purged (DC-EVIEW-01 GATE-CRASH/PURGE), `track_utxo=true` CONTAINED to the window (GATE-NOT-LIVE green), and replay-deterministic (same blocks → same window state).
**TCB boundary (binding):** the temporary store is GREEN execution support — GREEN while it is built, read, and disposed; it never becomes authority and is NEVER "inside BLUE." The pure PROJECTION it feeds (the aggregated snapshot / `EpochConsensusView`, S3c–S3e) is BLUE once emitted. GREEN store → BLUE projection; the store itself is not the authority.
**MAC:** a window replay over a fixture chain materializes the expected reduced UTxO on disk (bounded RssAnon, len==N); crash mid-window leaves durable state unchanged + purges; the live path never sees `track_utxo=true`.
**Prohibitions:** NO aggregation/snapshot/emission/live wiring; the window is GREEN, disposed after use.

## S3c — Stake contribution aggregation (BLUE linchpin; highest correctness risk)
**Purpose:** replace the STUBBED `new_mark` builder (`rules.rs:1098-1115`, zero-fills pool_stakes + substitutes reward balances) with the REAL aggregate: walk the window's reduced UTxO, classify each output (S3a), resolve pre-Conway pointers (S3a), sum `coin + reward_balance` per credential restricted to REGISTERED+DELEGATED credentials, group per pool. Threaded INTO the boundary authority as a PARAMETER (the window computes it; `apply_epoch_boundary_with_registrations` consumes it as `new_mark`) — so the aggregation lives inside the single ledger transition, not a second engine.
**Reuse / build:** reuse the cert/delegation maps (`delegation.rs`, accumulated in the S3b window), CE-71 reward accounting (`rules.rs:625-1310`); net-new = `aggregate_pool_stake(window_utxo, delegation_map, reward_balances, era) -> StakeSnapshot` + the `rules.rs:1098` rewire to consume it.
**Invariant (proposed DC-EVIEW-05):** the pool stake aggregate = Σ(UTxO coin + reward balance) per registered+delegated credential, grouped per registered pool, era-correct (Conway pointer = 0); a pure projection of the window's terminal ledger state; single authority (no second computation).
**MAC + ORACLE:** `EpochConsensusView.stake_by_pool` / `total_active_stake` byte/numeric-equals `cardano-cli query stake-snapshot` (stakeSet) at ≥2 Conway boundaries; the deliberate pointer fixture contributes 0 in Conway; deposit excluded; deregistered/undelegated excluded.
**CE-71 gate (binding):** CE-71 (the reward accounting) is NOT promoted to live-authoritative use merely because corpus tests pass. Its first live-path use remains gated on ALL of: deterministic replay tests (DC-WAL-03, two-run byte-identical), crash/recovery tests, differential stake/reward results at committed chain points (vs cardano-cli), and NO change to current live consensus decisions. S3c is observe-only; CE-71 goes live-authoritative only at the activation gate (DC-EVIEW-08), and only once those four hold.
**Prohibitions:** NO live wiring; NO leader/header use; the aggregate feeds only the snapshot builder.

## S3d — Snapshot formation and stability gating (BLUE)
**Purpose:** form mark/set/go from the S3c aggregate (reuse `rotate_snapshots`, `epoch.rs:94`) AND add the **snapshot-stability gate** that Ade currently LACKS — a snapshot/view is finalized/usable ONLY once its defining boundary is `> k` (2160) deep (cardano-ledger forces the lazy MARK thunks "after one stability window"; 3k/f stability, 3k/f→4k/f randomness at Conway). The SET snapshot drives leadership (2-epoch lag); GO drives rewards (3-epoch).
**Reuse / build:** reuse `rotate_snapshots` + the k=2160 machinery (`node_lifecycle.rs:1273`); net-new = the stability gate (a view is not finalized pre-k-immutability) + the rollback interaction (a snapshot must not finalize until its boundary is settled).
**Invariant (proposed DC-EVIEW-06):** a snapshot is finalized/emittable ONLY when its boundary block is `> k` deep; leadership reads SET (E−2); a rollback before finalization re-derives the snapshot from the rolled-back ledger.
**MAC:** the stability gate refuses a not-yet-k-deep snapshot; a rollback across the boundary re-derives correctly; SET/GO lags match Cardano.
**Prohibitions:** NO live leadership wiring yet; emission is S3e.

## S3e — EpochConsensusView emission / binding (BLUE/GREEN; observe-only)
**Purpose:** emit the compact, immutable `EpochConsensusView` from the finalized snapshot, BOUND to all of {network, era, epoch, source chain point, checkpoint commitment, nonce, snapshot phase, canonical-bytes hash}. The TYPE is ABSENT today (this builds it; model = `SeedEpochConsensusInputs`, `seed_consensus_inputs.rs:56`). **Observe-only:** the view is emitted + persisted but NOT wired to live leadership/header-validation.
**Reuse / build:** model on `SeedEpochConsensusInputs` (the bound-record + canonical encode/decode); net-new = the `EpochConsensusView` type + its canonical serialization + the binding + the WAL canonicalization (a DISTINCT WAL variant, preserving the bootstrap `SeedEpochConsensusInputsImported` single-import; relax `DuplicateProvenance` `replay.rs:170` only for the new variant).
**Invariant (proposed DC-EVIEW-07):** an `EpochConsensusView` activates/finalizes ONLY when bound to all 8 bindings (a missing binding ⇒ inert); it is a pure projection; WAL-canonicalized so replay reproduces it (DC-WAL-03 two-run byte-identical, no network/wall-clock in BLUE).
**MAC:** a view round-trips canonically; an unbound/partially-bound view is inert; replay of the WAL reproduces the view byte-identically; the differential oracle (stake-snapshot + leadership-schedule) matches at the emitted view's epoch.
**Prohibitions:** NO live leadership/header use (that is the activation step); NO live-path change.

## Activation (the deferred FINAL step — its own slice, gated)
**Purpose:** wire the proven `EpochConsensusView` into the live producer so the NEXT epoch's leadership reads the DERIVED view instead of the seed — the ONLY step that alters the live path. Attaches at `node_sync.rs:1544` (the DC-EPOCH-03 OffEpoch→ForgeNotLeader wall, which today fail-closes off the single seed epoch) + `:1560` (the seed `PoolDistrView` bind); requires a NEW rebind seam in `run_relay_loop_with_sched` (which today borrows `ledger_view` immutably — no in-loop rebind).
**Gate (binding):** NOT entered until S3a–S3e are merged AND the view is DIFFERENTIALLY PROVEN against Cardano (stake-snapshot + leadership-schedule match across ≥2 Conway boundaries). NO leader decision or header-validation change before that proof. Live acceptance = a preview transcript where ADE1's derived leadership across a real boundary matches `cardano-cli leadership-schedule` and the forge produces on the new epoch.
**Invariant (proposed DC-EVIEW-08):** activation flips DC-EPOCH-03's single-epoch containment into BOUND cross-epoch production; it never silently enables `track_utxo` live, never signs past a boundary with a stale/unbound view, and the era-schedule/view rebind is replay-deterministic.

---

## Dependency + risk order
S3a (pure, no deps) → S3b (window, reuses materialize) → S3c (linchpin, needs S3a+S3b, oracle-matched) → S3d (snapshot+stability, needs S3c) → S3e (emission, needs S3d) → activation (final, needs all + the differential proof). Correctness (S3a–c) precedes any live touch (activation), per "a faster incorrect follow is still incorrect." Heaviest dependency: CE-71 reward accounting (correct vs corpus, never live) — S3c is its first live-path exercise.

## What this scope does NOT decide
The per-sub-slice slice docs (entry obligations answered, MAC finalized) are written + reviewed + approved one at a time. Nothing is coded until each sub-slice doc is accepted. S3a is the natural first (pure, no deps, closes the pinned varint obligation).
