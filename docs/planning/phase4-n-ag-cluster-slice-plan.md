# Cluster/Slice Plan — Ade · DC-NODE-19 (PHASE4-N-AG)

> IDD Part IV artifact (cluster planning). Overall plan only — full cluster doc is `/cluster-doc N-AG`.
> Source sketch: `docs/planning/single-producer-loop-continuation-after-feed-eof-invariants.md`
> (approved + committed `7e1e8276`). Registry: `DC-NODE-19` declared/derived (last entry).
> Plan approved 2026-06-08 — 3 code/evidence slices (S1–S3) + 1 operator-gated live leg (S4);
> CE-AG-3 = **7** certified-run fail-closed conditions; S3 kept separate.

## Cluster Index (Dependency Order)

1. **PHASE4-N-AG** — single-producer extend-mode forge-loop continuation after follow-link EOF —
   primary invariant: *in a declared single-producer venue already in the DC-NODE-18 extend state, a
   `LoopState::Ending` caused solely by structural feed EOF must not terminate the forge loop.*

One cluster — DC-NODE-19 is a single, narrow invariant. It depends only on already-enforced invariants
(DC-NODE-05 / DC-NODE-09 / DC-NODE-12 / DC-NODE-15 / DC-NODE-18, T-REC-03 / T-REC-05, CN-NODE-02,
DC-CONS-03 — all enforced at HEAD `f87d0056`); no later cluster's invariant is required.

## PHASE4-N-AG — single-producer forge-loop continuation after follow-link EOF

- **Primary invariant:** DC-NODE-19 (declared → enforced at close) — in `VenueRole::SingleProducer` **and**
  `ForgeMode::SingleProducerExtendOwnDurableSpine`, a `LoopState::Ending` caused **solely by structural
  feed EOF** does not terminate the loop; it continues forging on the own certified durable spine, fenced
  to the certified single-producer run, until shutdown / fatal error / an existing BLUE forge-validity
  bound / a competing chain. Relocates the loop's termination authority off feed-liveness (the DC-NODE-09
  move, now for the forge loop) while preserving DC-NODE-05's deeper invariant (`pump_block` sole durable
  tip authority; feed work drains via `SyncOnce` before any `ForgeTick`).

- **TCB partition:**
  - **BLUE:** *none changed* — `pump_block`, `forge_one_from_recovered`, `block_validity`, `chain_selector`
    untouched (mirrors DC-NODE-18). No new canonical type, no new authoritative state, no new WAL entry type.
  - **GREEN:** `ade_node::run_loop_planner` (`plan_loop_step` + new closed `VenuePolicy` + the
    `(VenueRole, ForgeMode) → VenuePolicy` projection); `ade_node::node_sync` (reuse the existing DC-NODE-18
    `single_producer_forge_decision` / `SingleProducerFenceReason` fence).
  - **RED:** `ade_node::node_lifecycle` (`run_relay_loop_with_sched` threads the policy; `Idle`-under-feed-end
    clock-tick wakeup; per-continuation certificate re-validation). **No `cli.rs` change** — DC-NODE-18's
    `--single-producer-venue` already declares the venue; continuation is an automatic consequence of the
    extend state.

- **Cluster Exit Criteria:**
  - **CE-AG-1:** `plan_loop_step` is pure/total over **5** closed inputs (32-case, no wildcard); reduces
    **exactly** to the prior 16-case table when `VenuePolicy = HaltOnFeedEnd` (default); the new
    `Ending` + `ContinueInSingleProducerExtend` cells return `ForgeTick`(Due) / `Idle`(NotDue). Content-blind
    (no tip / hash / verdict; `SlotNo` only in `forge_slot_status`).
  - **CE-AG-2:** in a declared single-producer venue in the extend state, a structural feed-EOF does **not**
    terminate the loop (continues forging the own durable spine); the default `VenueRole::Unknown` venue halts
    **verbatim** (`HaltCleanly`). Only a clean structural EOF continues — a fatal source failure still exits
    via `Err` / fail-fast.
  - **CE-AG-3:** continuation **fails closed** (no continuation; verbatim `HaltCleanly` / typed refusal) on
    each of the **7** certified-run conditions, by **reusing** the DC-NODE-18 fence (+ per-continuation
    certificate re-validation), never reimplementing fork-choice:
    1. not `VenueRole::SingleProducer`;
    2. not `ForgeMode::SingleProducerExtendOwnDurableSpine`;
    3. operator shutdown requested;
    4. existing forge-validity bounds fail (off-epoch DC-EPOCH-03 / beyond forecast horizon DC-CONS-09 /
       KES-period invalid);
    5. a competing chain was observed **before** EOF;
    6. relay-producing evidence exists;
    7. the venue certificate is absent or malformed.

    The `Idle`-under-dead-feed wait wakes on the next clock tick / shutdown (no busy-spin, no starved
    forge cadence).
  - **CE-AG-4:** replay-equivalence over a chain that includes successors forged **after** a feed EOF — two
    clean runs byte-identical (T-REC-03) + kill / warm-start byte-identical (T-REC-05); the feed-end event is
    replay-neutral (appends nothing to the WAL).
  - **CE-AG-5 (operator-gated; hard close gate = CE-AF-6b):** sustained **> k** Ade blocks settle into the
    relay's ImmutableDB across **≥ 1** follow-link EOF, warm-start replay byte-identical; **committed live
    transcript** (rung1-auto, C2-LOCAL).
  - **CE-AG-6 (close):** new CI gate green; **DC-NODE-19 declared → enforced**; `strengthened_in +=
    PHASE4-N-AG` on DC-NODE-05 / CN-NODE-02 / T-REC-03 / T-REC-05 / DC-NODE-18 (DC-CONS-03 untouched); all
    four grounding docs refreshed **including the CODEMAP + SEAMS deferred from N-AF** (baseline `f87d0056`
    → N-AG HEAD).

- **Slices:**
  - **S1 — GREEN planner refinement** — invariant: `plan_loop_step` gains an explicit 5th content-blind
    `VenuePolicy` input → 32-case total table + the `(VenueRole, ForgeMode) → VenuePolicy` projection; the
    default `HaltOnFeedEnd` reduces to the prior 16-case behavior. — addresses: **CE-AG-1** — TCB: **GREEN**.
    *(Mergeable: a pure function gains an input; the RED caller passes the default until S2 → behavior-preserving.)*
  - **S2 — RED loop continuation + certified-run fence + Idle wakeup** — invariant: `run_relay_loop_with_sched`
    derives the policy from `(venue_role, forge_mode)` and threads it in; **continues past structural feed
    EOF only after DC-NODE-18 certificate promotion** (i.e., in `SingleProducerExtendOwnDurableSpine`); the
    default `Unknown` venue halts verbatim; a fatal source failure still fails fast; reuses the DC-NODE-18
    fence for conditions 5–7 (+ per-continuation certificate re-validation for #7); the `Idle`-under-feed-end
    clock-tick wakeup (OQ-19-1); **new CI gate** `ci_check_single_producer_loop_continuation.sh`. —
    addresses: **CE-AG-2, CE-AG-3** — TCB: **RED** (+ reuse GREEN fence).
  - **S3 — replay-equivalence over a post-feed-end chain** — invariant: T-REC-03 two-clean-runs byte-identical
    + T-REC-05 kill / warm-start byte-identical over a chain that includes post-feed-end forges; the feed-end
    event appends nothing to the WAL (replay-neutral). — addresses: **CE-AG-4** — TCB: tests over existing
    BLUE/RED machinery (no new code).
  - **S4 — operator-gated live acceptance (CE-AF-6b)** — invariant: a real run on rung1-auto (C2-LOCAL)
    sustains > k Ade blocks into the relay's ImmutableDB across ≥ 1 follow-link EOF, warm-start byte-identical;
    commit the transcript. — addresses: **CE-AG-5** — TCB: **RED** (operator harness + evidence).
    *(Operator-gated — depends on a synced docker peer + rung1-auto, exactly as DC-NODE-18's CE-AF-6 / c2t7
    was; the enforcement-flip trigger.)*

- **Replay obligations:** **No new** canonical type, authoritative state, WAL entry type, or replay-corpus
  entry. The obligation is to **extend** T-REC-03 (loop-as-replay) + T-REC-05 (forged-chain warm-start) to
  cover a chain with post-feed-end forges, and to prove the feed-end event is **replay-neutral** (no WAL
  append) — S3. Each continued successor is a normal `AdmitBlock` via `pump_block` (existing durable path,
  DC-NODE-12 / DC-WAL-04). At close: `strengthened_in += PHASE4-N-AG` on T-REC-03 + T-REC-05.

- **FC/IS partition:** BLUE **unchanged**; GREEN = `ade_node::run_loop_planner` + the reused
  `ade_node::node_sync` fence; RED = `ade_node::node_lifecycle` loop. Dependencies flow inward only
  (RED → GREEN; no GREEN → RED, no BLUE touch). Mirrors DC-NODE-18's GREEN + RED-only shape.

## Notes

- **Complete-work-only.** Every CE is reachable within N-AG. CE-AG-5 is operator-gated but in-cluster (the
  operator executes the rung1-auto run, exactly as CE-AF-6 / c2t7 closed DC-NODE-18). DC-NODE-19 flips to
  `enforced` only when the hermetic gates (S1–S3) **and** the committed CE-AF-6b transcript (S4) both land —
  the project's hard-closure discipline.
- **One open design detail (cluster-doc).** OQ-19-1 — the exact `Idle`-under-dead-feed wakeup mechanism (a
  clock-tick branch in the cancellation-safe `select!`). Resolved at `/cluster-doc N-AG`; lands in S2.
- **Grounding-doc debt.** The N-AG close must regenerate CODEMAP + SEAMS (deferred at the N-AF close;
  baseline `f87d0056`) in addition to HEAD_DELTAS + TRACEABILITY.
