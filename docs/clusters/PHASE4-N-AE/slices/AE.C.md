# Invariant Slice — PHASE4-N-AE.C: Recover→Follow WAL Prior-FP Seeding

## §2 Slice Header
- **Slice Name:** recover→follow WAL prior-fp lineage continuity
- **Cluster:** PHASE4-N-AE (Recover→Serve Continuity and Forge Admissibility) — strengthens **DC-WAL-02** (WAL fingerprint-chain integrity, first-entry clause) + **T-REC-05** (replay-equivalent recovery). **No new invariant ID.**
- **Status:** Proposed
- **Cluster Exit Criteria Addressed:** **CE-C1** (DC-WAL-02 / T-REC-05 — recover→follow→kill→warm-start replays to the same tip; first followed `prior_fp == fingerprint(recovered ledger)`; zero-seed variant reproduces `ChainBreak`; two-run byte-identical), **CE-C2** (the live-wiring gate). Gates **CE-A5 retryability** (the recover→followed store must survive restart for a safe retry). *(Distinct from AE.A forge gate / AE.B anchor intersectability.)*

## §3 Dependencies
AE.A merged (`5f2afc2a`). Surfaced by the 2026-06-07 CE-A5 live run: a `--mode node` warm-start of a recover→**followed** store fails closed:
`ChainBreak { entry_index: 1, expected_prior_fp: d83372e4b0211461975f81cd4fcf1c2e74091dd3f758b2db39e6d3a15a44a6a4, actual_prior_fp: 0000…0000 }` (exit 42). Reuses N-U/N-AD durability (`pump_block`, `warm_start_recovery`, `replay_from_anchor`, `verify_chain`) and N-F-C/D follow (`run_node_sync`). The in-memory single-run path already works (AE.A's live run caught up + forge-attempted); only the **persisted** replay path is broken.

## §4 Intent (invariant impact)
When a node warm-starts a recovered ledger state and then follows peer blocks, the **first followed block's WAL entry must chain from the fingerprint of the ledger state being extended by the follow (the current recovered ledger tip's post_fp) — not from genesis/zero, and not from the original bootstrap anchor's initial fingerprint** (which is stale after any later warm-start that already includes followed blocks). This restores **DC-WAL-02** (every `AdmitBlock.prior_fp` chains from the prior entry's `post_fp`, or the ledger-tip fingerprint for the first followed entry) and **T-REC-05** (replay-equivalent recovery) on the live recover→follow path: same recovered tip + same followed blocks + same WAL/checkpoints must warm-start to byte-identical state. The challenge requires power-loss recovery; a store that recovers only until the first restart is non-compliant.

## §5 Scope / What is built
- **FIX — the two `ForwardSyncState::new` prior-fp seeds in `ade_node::node_lifecycle`** (`run_node_lifecycle_inner`):
  - **ForgeIntent::Off site (`:437`)** and **ForgeIntent::On site (`:523`)**: the prior-fingerprint argument `Hash32([0u8; 32])` → **`fingerprint(&state.ledger).combined`** — the fingerprint of the ledger state being extended (the current recovered ledger tip's post_fp). This is exactly the value `warm_start_recovery`'s own T-REC-05 reconciliation compares against the WAL-tail `post_fp` (`node_lifecycle.rs:1468`), so the first followed `AdmitBlock.prior_fp` equals the WAL-tail `post_fp` and the chain is continuous.
  - For the **Off** site, read the fingerprint into a local **before** `state.ledger` is moved into `ReceiveState::new` (the On site clones `state.ledger`, so `state` is still owned).
  - `fingerprint` is already imported (`node_lifecycle.rs:57`); no new dependency.
- **NEW hermetic test** `crates/ade_node/tests/phase4_n_ae_recover_follow_wal_lineage.rs` (or a `node_sync` test module): recover (seed a ledger + anchor with a **real, non-zero** fingerprint) → follow real corpus blocks that chain from the recovered tip (`run_node_sync` over an in-memory feed) → drop handles (kill) → `warm_start_recovery`. Asserts: first followed `AdmitBlock.prior_fp == fingerprint(recovered ledger).combined`; warm-start reaches the **same tip** as the pre-kill run; a **zero-seed variant** (`prior_fp = Hash32([0u8;32])`) reproduces the `ChainBreak` (red); two consecutive recover→follow runs are **byte-identical** (WAL image + checkpoint cursor + served tip).
- **NEW gate** `ci/ci_check_recover_follow_wal_lineage.sh`: asserts the live `node_lifecycle::run_node_lifecycle_inner` `ForwardSyncState::new` prior-fp seed is `fingerprint(&state.ledger)` (the ledger-tip fingerprint), and that **neither** lifecycle site seeds `Hash32([0u8; 32])` / a zero/`default()` fingerprint (fence against reintroduction); and that WAL `verify_chain`/`replay_from_anchor` are **not** weakened (no new "accept break"/skip path).

## §6 Execution Boundary (TCB color)
- **RED (changed):** `ade_node::node_lifecycle` — the two `ForwardSyncState::new` prior-fp seeds (`:437`, `:523`). Pure value substitution at the wiring seam; no control-flow change.
- **BLUE/GREEN (reused, NOT edited):** `ade_ledger::fingerprint::fingerprint` (BLUE), `ade_ledger::wal::{replay::replay_from_anchor, store_trait::verify_chain}` (BLUE), `ade_runtime::forward_sync::{reducer::ForwardSyncState, pump}` (GREEN/RED reused), `ade_runtime::wal` (RED reused), `ade_runtime::network::served_chain_projection` (RED, read-only).
- **No new BLUE authority or canonical type. No second WAL writer. No alternate durable path. No change to WAL verification semantics.**

## §7 Invariants Preserved
DC-WAL-01 (single WAL append authority — unchanged), DC-WAL-03 (WAL/snapshot/checkpoint provenance — unchanged), DC-WAL-04 (forged-block WAL chain integrity — unchanged; this slice fixes the *followed* first-entry seed, not the forged-entry seed), CN-CONS-07 / DC-NODE-12/13 (durable admit via `pump_block`; serve-as-projection — unchanged), DC-NODE-15 / DC-CONS-24 / DC-NODE-14 (AE.A forge gate — unchanged; AE.A tests stay green), DC-SYNC-01/02 / CN-NODE-02 (`pump_block` sole tip authority; containment gates not regressed).

## §8 Invariants Strengthened or Introduced
One invariant family — **recover→follow WAL lineage continuity** (the persisted WAL of a recovered-then-followed store replays byte-identically). No new ID:
- **DC-WAL-02** — `strengthened_in += "PHASE4-N-AE"`: the first-followed-entry-chains-from-the-ledger-tip clause is now enforced on the **live** recover→follow path (a new test + a new live-wiring gate; previously enforced only on hermetic/forged paths, which masked the live zero-seed).
- **T-REC-05** — `strengthened_in += "PHASE4-N-AE"`: replay-equivalent recovery now covers the recover→follow→warm-start sequence (kill-then-warm-start reaches the same tip; two-run byte-identical).

## §11 Replay / Crash / Epoch Validation
- **Crash recovery (the fix's core):** `recover_follow_kill_warm_start_chains_from_anchor` — recover→follow→drop→`warm_start_recovery` reaches the pre-kill followed tip (no `ChainBreak`); the zero-seed variant `recover_follow_zero_seed_chainbreaks` reproduces the exit-42 failure (red guard).
- **Replay (determinism):** `recover_follow_two_runs_byte_identical` — same recovered tip + same followed blocks ⇒ byte-identical WAL image, checkpoint cursor, and served tip across two runs.
- **Epoch:** unchanged (no epoch logic touched).
- **Live (optional, recommended):** after this lands, a CE-A5 rerun's warm-start sanity check (recover → follow to T → stop before forge → warm-start → recovered durable tip == T → served chain intersects at T) becomes a safe pre-forge gate.

## §12 Mechanical Acceptance Criteria
1. `cargo test -p ade_node` green incl. NEW `recover_follow_kill_warm_start_chains_from_anchor` — warm-start after recover→follow reaches the same tip; AND `recover_follow_zero_seed_chainbreaks` — a zero prior-fp seed reproduces `ChainBreak` (proving the fix is load-bearing).
2. `first_followed_admit_prior_fp_equals_recovered_ledger_fingerprint` — the first followed `AdmitBlock.prior_fp == fingerprint(recovered ledger).combined` (== the anchor post_fp on a first run; == the followed WAL-tail post_fp on a later warm-start).
3. `recover_follow_two_runs_byte_identical` — two-run WAL image + checkpoint + served tip byte-identical (T-REC-05).
4. NEW `ci/ci_check_recover_follow_wal_lineage.sh` green: both lifecycle `ForwardSyncState::new` prior-fp seeds are `fingerprint(&state.ledger)`; **no** `Hash32([0u8;32])`/zero/`default()` seed at either site; WAL `verify_chain`/`replay_from_anchor` carry no new accept-break/skip path.
5. Existing AE.A diagnostic fixtures (`phase4_n_ae_recover_serve_continuity_diag.rs`) + DC-WAL-02/04 + T-REC-05 tests remain green; `cargo test --workspace` green (`ade_testkit` corpus-suite environmental timeout reported honestly).
6. `ci/ci_check_node_run_loop_containment.sh`, `ci/ci_check_served_chain_projection.sh`, `ci/ci_check_loop_planner_closed.sh`, `ci/ci_check_forge_followed_tip_admission.sh` green (no regression).

## §13 Failure Modes
- Pre-fix: warm-start of a recover→followed store fails closed (`ChainBreak`, authority-fatal, exit 42) — correct fail-closed behavior over a mis-seeded WAL, but the WAL should never have been mis-seeded. Post-fix: the WAL chains from the recovered ledger tip; warm-start succeeds and is replay-equivalent. The fix **seeds the chain correctly; it does not relax `verify_chain`** — a genuinely broken WAL still halts fail-closed.

## §14 Hard Prohibitions
**Inherited (cluster §11):** no second durable tip-advance path (go through `pump_block`); no new BLUE authority or canonical type; no containment-gate regression; no RO-LIVE flip on local/hermetic evidence.
**Slice-specific (your boundaries):**
- **No resetting `prior_fp` to zero** after a recovered ledger state — the seed is `fingerprint(&state.ledger).combined`.
- **No using `state.initial_ledger_fingerprint`** as the live continuity seed (stale after a later warm-start that already includes followed blocks) — use the fingerprint of the ledger being extended.
- **No treating in-memory state as evidence of persistence correctness** — the test must kill + warm-start from disk.
- **No bypassing/loosening WAL validation on warm-start** — do not change `verify_chain`/`replay_from_anchor` to accept the old broken shape.
- **No making a CE-A5 retry pass by disabling the chain-break check.**
- **No second WAL writer / alternate durable path.**
- **No fresh-venue single-shot manifest over the known replay bug** (this slice is the prerequisite).

## §15 Explicit Non-Goals
The leadership-lottery / connection-window retry mechanics (operational, not durability); the recovered-anchor intersectability Option A/B (AE.B); fork-choice / multi-producer intake (Gap 1); any change to WAL verification semantics; any RO-LIVE flip. CE-A5 itself is run **after** this lands.
