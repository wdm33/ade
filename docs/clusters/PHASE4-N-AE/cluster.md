# PHASE4-N-AE — Recover→Serve Continuity and Forge Admissibility (DC-NODE-14 / DC-NODE-15 / DC-CONS-24)

> **Grounded in the committed PO2 resolution + a read-only code trace of the `--mode node` recover/follow/forge/serve seam.** Invariants sketch: `docs/planning/phase4-n-ae-slice-a-invariants.md`; discovered-gaps: `docs/planning/c2-local-discovered-gaps.md`; venue + status: `docs/active/c2-preprod-tip-guide.md §5b`. Red acceptance spine (uncommitted, folds into the slices): `crates/ade_node/tests/phase4_n_ae_recover_serve_continuity_diag.rs`. This closes **Gap 2** (C2-LOCAL #8–#9 non-adoption); Gap 1 (multi-producer fork-choice) is explicitly out — a separate future cluster.

## §1 Primary invariant (DC-NODE-14 / DC-NODE-15 / DC-CONS-24)
A `--mode node` forge produces a **peer-adoptable** successor: it is admissible **only** when the durable servable tip equals the followed peer tip (**DC-NODE-15**), its parent hash **byte-equals** that peer-visible selected tip (**DC-CONS-24**), and every claimed forge parent is itself a **servable or peer-intersectable** point in the durable served lineage (**DC-NODE-14**). The recovered snapshot anchor is never silently used as a forge base or served as a chain head from which a peer cannot `FindIntersect`.

## §2 The problem (resolved by PO2, not hypothesis)
Proven by the committed code trace + the three red fixtures in `phase4_n_ae_recover_serve_continuity_diag.rs` (run `--ignored --nocapture`):
- `admission::seed_to_snapshot` persists a ledger **snapshot** at the recovered anchor (`PersistentSnapshotCache::capture`, CN-STORE-08) — **no servable `StoredBlock`**. So after recover `ChainDb::tip() == None` and `ChainDbServedSource::intersect([anchor]) == None` (fixture `recovered_anchor_is_not_peer_intersectable`).
- The ForgeTick `selected_tip` **falls back to the recovered anchor** when the durable tip is empty (`node_lifecycle.rs:1098-1103: None => act.recovered.tip.clone()`), and the forge sets the successor's parent = `selected_tip.hash` (`node_sync.rs:559`). A forge firing on a `NoWorkReady` gap (before the follow stored the peer tip) therefore builds on the snapshot-only anchor (fixture `forge_base_falls_back_to_snapshot_anchor`; end-to-end `forged_successor_on_recovered_anchor_is_not_peer_adoptable` — a **real** forged block 8 whose parent is not servable).
- Result: the served chain has no point the relay can intersect → the relay falls back to Origin → `UnexpectedBlockNo (N)(0)` → non-adoption.
- **Narrow fix confirmed:** the follow path `run_node_sync → pump_block → put_block` **does** store followed blocks (incl. the peer's tip) as servable `StoredBlock`s. So gating the forge on the followed tip closes the common case without anchor materialization.

## §3 The design — gate the forge on the followed tip; make the anchor intersectable only for the degenerate case
- **AE.A (primary, C2-LOCAL closer):** a forge-admissibility gate — a closed classifier sibling to `forge_epoch_admission` (DC-EPOCH-03), computed **before** leadership, fail-closed to a typed `ForgeRefused::NotCaughtUp`. Remove the `recovered.tip` fallback as a forge base. In the recover-behind shape the follow stores the peer tip T as a `StoredBlock`; the gate makes the forge wait for `durable_servable_tip == T`, the relay intersects at T, and adopts T+1 — no anchor work.
- **AE.B (secondary, edge case):** make a recovered anchor intersectable for the recover-at/near-tip case (zero followed blocks). **Option A (preferred):** fetch the anchor block envelope bytes, `blake2b_256`-verify against the recovered anchor point, and admit through a **sanctioned** durable writer (`bootstrap_initial_state` already writes `StoredBlock`s and is a CN-CONS-07 sanctioned writer) → the anchor becomes a real `StoredBlock` and the serve intersects it for free. **Option B (fallback):** project the snapshot anchor as a `FindIntersect` point only — never servable bytes. **Never** synthesize servable block bytes from snapshot/ledger/block-no/hash.

## §4 Authority surface / compatibility boundary / permitted divergence
- **Authority surface:** deciding when a recovered/followed chain is eligible to forge and serve a **peer-adoptable** successor.
- **Compatibility boundary (the Haskell oracle):** a real Haskell peer must `FindIntersect → RollForward → validate → adopt` Ade's forged successor. Essential, externally-binding behavior: the peer can intersect at the forged parent; the forged parent hash **byte-equals** the peer-visible selected tip; `next_after(peer_tip)` returns the Ade forged block.
- **Permitted divergence (Tier 5 — `docs/active/CE-79_tier5_addendum.md`):** Ade may represent recovered anchors internally however it likes (snapshot + WAL/checkpoint lineage, not the Haskell ChainDB layout) — but the **served protocol behavior** must remain equivalent. Internal storage shape may differ; chain facts on the wire must not.

## §5 Entry conditions (what prior clusters guarantee)
- **N-U** (DC-NODE-12/13, DC-CONS-23, CN-CONS-07): forged + received blocks become durable ONLY via `pump_block`; the `--mode node` served view is a read-only projection of the durable ChainDb (`ChainDbServedSource`); the durable chain is extend-only; serve provenance = sanctioned writers `pump_block` + `bootstrap_initial_state`.
- **N-M-*** (RO-LIVE-05, CN-SEED-01, CN-ANCHOR-01): recover a non-Origin Conway tip via `seed_to_snapshot` (snapshot at the anchor slot) + `import_live_consensus_inputs`.
- **N-F-C/D/E/F** (CN-NODE-01/02/03, DC-SYNC-01/02, DC-NODE-05): the relay loop advances the tip ONLY via `run_node_sync → pump_block`; forge is subordinate to the sync spine; `forge_one_from_recovered` + `forge_header_position`; **DC-EPOCH-03** off-epoch fail-closed gate (the precedent for AE.A's gate).
- **N-F-G-P/Q** (DC-CINPUT-04, DC-NODE-10): feed header-validation view from the recovered surface; forge-successor position from the evolved admitted spine.
- The follow path durably stores followed blocks (incl. the peer tip) as servable `StoredBlock`s (proven, PO2).

## §6 TCB color map (FC/IS partition)
- **BLUE (reused, unchanged):** `ade_ledger::{block_validity (incl. header_position), receive::admit_via_block_validity, wal, producer::{forge, self_accept}}`, `ade_core::consensus`. `fork_choice`/`select_best_chain` is **not** on this path (stays the follow/`chain_selector` authority — untouched).
- **GREEN (new closed classifier + reused):** the forge-admissibility classifier (a pure GREEN-by-fn in `node_sync`, sibling to `forge_epoch_admission`); `forward_sync::reducer`; the loop planner. The followed-peer-tip signal is converted to a **structured, replayable admissibility input** here (GREEN), never a chain-selection authority.
- **RED (new wiring + changes):** `ade_node::node_sync` (the gate call + typed `ForgeRefused::NotCaughtUp`), `ade_node::node_lifecycle` (remove the `recovered.tip` fallback; thread the followed-peer-tip signal; AE.B serve/recover wiring), `ade_runtime::network::served_chain_projection` (AE.B Option B intersect-only, if taken), `ade_runtime::bootstrap` / recover path (AE.B Option A anchor materialization through the sanctioned writer, if taken).
- **No new BLUE authority or canonical type.** `ForgeRefused` is a closed RED/GREEN error sum, not a canonical type.

## §7 Slices

> **AE.C (added 2026-06-07):** the AE.A CE-A5 live run surfaced a pre-existing recover→follow WAL prior-fp mis-seed — a `--mode node` warm-start of a recover→**followed** store fails `ChainBreak` (exit 42). AE.C fixes the live `ForwardSyncState` prior-fp seed (gates **CE-A5 retryability**); it **strengthens DC-WAL-02 + T-REC-05** and mints no new ID. AE.A / AE.B unchanged.

| Slice | Scope | CE | Registry → status | TCB |
|---|---|---|---|---|
| **AE.A** — Forge-on-Followed-Tip Gate + Followed-Block Serve Continuity | Forge admissible iff `durable_servable_tip.{hash,block_no} == followed_peer_tip.{hash,block_no}`; typed `ForgeRefused::NotCaughtUp`; remove the `recovered.tip` forge-base fallback; forged parent byte-equals followed peer tip; `next_after(followed_peer_tip) == forged`; peer-tip signal is admissibility-only. Closes C2-LOCAL #8–#9. | CE-A1..A5 | DC-NODE-15 + DC-CONS-24 → **enforced**; DC-NODE-14 → **partial** (followed-tip lineage clause enforced; recovered-anchor clause still declared/open) | GREEN classifier + RED wiring |
| **AE.B** — Recovered-Anchor Intersectability | Make a recovered anchor peer-intersectable for the recover-at/near-tip (zero-followed-block) case — Option A (hash-verified anchor bytes via a sanctioned writer → `StoredBlock`) or Option B (intersect-point-only projection, no servable payload). | CE-B1..B2 | DC-NODE-14 → **enforced** (anchor clause closes the umbrella rule) | RED |
| **AE.C** — Recover→Follow WAL Prior-FP Seeding | Seed the live `ForwardSyncState` prior_fp from `fingerprint(&state.ledger)` (the recovered ledger tip being extended), not zero — so the first followed `AdmitBlock` chains from the WAL-tail post_fp and a recover→followed store warm-starts replay-equivalently. Surfaced by the CE-A5 run; gates CE-A5 retryability. | CE-C1..C2 | DC-WAL-02 + T-REC-05 → **strengthened** | RED |

## §8 Cluster Exit Criteria (CI-verifiable)
All mechanical; each names the gate + key tests the slice **adds** (none exist yet — declared). Fixtures live in `crates/ade_node/tests/phase4_n_ae_recover_serve_continuity_diag.rs` (committed with the slice that un-ignores them).

**AE.A:**
- **CE-A1 (DC-NODE-15):** new gate `ci/ci_check_forge_followed_tip_admission.sh` — the ForgeTick `selected_tip` has **no `recovered.tip` fallback**; forge fires only when `durable_servable_tip == followed_peer_tip` (hash AND block_no); a not-caught-up forge returns the typed `ForgeRefused::NotCaughtUp { local_servable_tip, followed_peer_tip, reason }` (not a log line); the peer-tip signal never reaches `select_best_chain`/`chain_selector`. Fixtures: `forge_base_falls_back_to_snapshot_anchor` (green) + a positive caught-up forge test + a `forge_refused_not_caught_up` test.
- **CE-A2 (DC-CONS-24):** `forged_parent_byte_equals_followed_peer_tip` — forged `prev_hash` byte-equals the followed peer tip hash AND `block_no == followed_tip.block_no + 1`; parent identity is the canonical hash, never inferred from block number.
- **CE-A3 (DC-NODE-14, followed-tip lineage clause):** `served_chain_intersects_at_followed_tip_and_rolls_to_forged` — after recover-behind + follow to T, `ChainDbServedSource::intersect([T]) == Some(T)` and `next_after(T)` projects the forged T+1.
- **CE-A4 (T-REC-05 strengthened):** `recover_follow_forge_two_runs_byte_identical` — same recovered anchor + same followed canonical blocks → byte-identical served chain + forged successor.
- **CE-A5 (live closure, criterion 6 — operator-gated):** the non-producing-relay C2-LOCAL venue → `ba02_evidence::correlate` manifest with **forged hash == adopted hash** (`AddedToCurrentChain`). Non-promotable rehearsal venue — flips no RO-LIVE rule on its own.

**AE.B:**
- **CE-B1 (DC-NODE-14, anchor clause):** new gate `ci/ci_check_recovered_anchor_intersectable.sh` — a recovered anchor is peer-intersectable via Option A (anchor bytes `blake2b_256`-verified against the recovered point + admitted through a sanctioned durable writer → `StoredBlock`) **or** Option B (snapshot anchor projected as a `FindIntersect` point only). Fixtures: `recovered_anchor_is_not_peer_intersectable` (green) + `forged_successor_on_recovered_anchor_is_not_peer_adoptable` (green).
- **CE-B2 (no synthetic bytes — strengthens CN-CONS-07 / DC-CONS-23):** the gate forbids synthesizing servable `StoredBlock` bytes from snapshot/ledger/block-no/hash; Option A admits only hash-verified original bytes through a sanctioned writer; Option B's BlockFetch for the anchor **refuses structurally** (serves no bytes).
**AE.C:**
- **CE-C1 (DC-WAL-02 / T-REC-05):** `crates/ade_node/tests/phase4_n_ae_recover_follow_wal_lineage.rs` — recover→follow→kill→`warm_start_recovery` reaches the same tip; first followed `AdmitBlock.prior_fp == fingerprint(recovered ledger).combined`; a zero-seed variant reproduces `ChainBreak` (the exit-42 failure, red guard); two consecutive recover→follow runs are byte-identical (WAL image + checkpoint cursor + served tip).
- **CE-C2 (live-wiring gate):** new gate `ci/ci_check_recover_follow_wal_lineage.sh` — both `node_lifecycle` `ForwardSyncState::new` prior-fp seeds are `fingerprint(&state.ledger)`, never `Hash32([0u8;32])`/zero/`default()`; WAL `verify_chain`/`replay_from_anchor` carry **no** new accept-break/skip path (the fix seeds the chain correctly, it does not loosen recovery).

- **Cluster-wide:** `cargo test --workspace` green (the three diagnostic fixtures now run, un-ignored, and pass); **no containment-gate regression** — `ci_check_node_run_loop_containment.sh`, `ci_check_served_chain_projection.sh`, and `ci_check_loop_planner_closed.sh` remain green and continue to prove no new unauthorized authority path (their allow-list / expected call graph may be updated for the new gate, but no containment invariant is relaxed).

## §9 Proof obligations (slice-entry, per proof discipline)
- **AE.A:** PO2 is resolved (committed fixtures). Remaining: confirm the followed-peer-tip signal source on the live `--mode node` path — `AdmissionPeerEvent::TipUpdate` is currently **skipped** by `NodeBlockSource` (`node_sync.rs:135,282`), so AE.A must introduce a caught-up signal as a structured admissibility input (not revive TipUpdate as a sync tip authority).
- **AE.B (Option A gate):** does the recovery artifact carry the anchor block's **envelope bytes**? Today recover has `--seed-block-hash` (hash) + UTxO seed + consensus inputs, not the anchor block. Option A needs an added anchor-block fetch + hash-verify; if not cheaply available, AE.B takes Option B (intersect-only). This PO is answered **before** AE.B commits to A vs B.

## §10 Invariants
- **Adds:**
  - **DC-NODE-14** — *Every claimed forge parent must be servable or peer-intersectable in the durable served lineage.* One semantic law (a peer must be able to stand on the parent Ade claims to extend), enforced in two clauses across the two slices. **Status after AE.A: `partial`** — the **followed-tip lineage clause is enforced**, the **recovered-anchor clause is still declared/open** (it is NOT fully satisfied by the C2-LOCAL pass). **Status after AE.B: `enforced`** — the recovered-anchor clause closes the umbrella rule.
  - **DC-NODE-15** — forge admissibility = `durable_servable_tip == followed_peer_tip`, else structured `ForgeRefused::NotCaughtUp` (AE.A `enforced`).
  - **DC-CONS-24** — forged parent hash byte-equals the peer-visible selected tip (AE.A `enforced`).
- **Strengthens** (`strengthened_in += "PHASE4-N-AE"`): **DC-NODE-13** (serve projection now also covers recovered-anchor intersectability), **CN-CONS-07** (serve provenance: a sanctioned-writer-only anchor materialization, or an intersect-point-without-bytes), **DC-CONS-23** (extend-only now also fences the no-synthetic-anchor-bytes rule), **DC-EPOCH-03** (a sibling fail-closed forge-admissibility boundary), **T-REC-05** (replay determinism extends to the recovered→followed→forged served chain; **AE.C** extends it to recover→follow→warm-start crash recovery), **DC-WAL-02** (**AE.C**: the first followed `AdmitBlock.prior_fp` chains from the recovered ledger-tip fingerprint on the live recover→follow path — first-entry clause enforced live).
- **Preserves / cross-ref (NOT strengthened):** **DC-NODE-05** (forge subordinate to the sync spine), **DC-NODE-12** (durable admit via `pump_block`), **DC-CONS-03** (Praos fork-choice authority — explicitly untouched; AE.A adds **no** fork-choice; multi-producer fork-choice is a separate future cluster), **CN-FORGE-01** (self-accept token), **CN-STORE-08** (snapshot capture).

## §11 Forbidden during this cluster (hard boundaries)
- **No synthetic servable `StoredBlock` bytes** from snapshot/ledger/block-number/hash (hash-critical paths require preserved original bytes).
- **No `recovered.tip` fallback as a forge base.**
- **No parent-hash inference from block number** — parent identity is the canonical hash.
- **No peer-tip signal as a chain selector** — it may *prevent* a forge (admissibility), never *select* the chain.
- **No C2-only bypass** of normal ChainSync/BlockFetch semantics.
- **No fork-choice / multi-producer intake** (Gap 1 — separate cluster); no ChainDB redesign.
- **No new BLUE authority or canonical type**; no second durable tip-advance path (go through `pump_block`); no containment-gate regression.
- **No RO-LIVE flip** on local/hermetic evidence — only a committed `correlate` manifest over a real peer advances RO-LIVE-01.

## §12 Open questions
- **AE.B A-vs-B** decided at `/slice-doc PHASE4-N-AE.B` once the anchor-bytes PO (§9) is answered.
- **Followed-peer-tip signal shape** (structured admissibility input) finalized at `/slice-doc PHASE4-N-AE.A`.
