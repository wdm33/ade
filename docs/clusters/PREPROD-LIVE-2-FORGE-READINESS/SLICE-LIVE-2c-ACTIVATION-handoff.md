# SLICE LIVE-2c ACTIVATION — the authority handoff, in three connected parts

> **DOC BEFORE IMPL.** Implementation contract for the ACTIVATION section of
> `SLICE-LIVE-2c-authoritative-forge-slot-wiring.md`. Everything that doc marks closed
> (`31f7c754`, `f050f472`, `352dfb95`, `8cf14529`) is inherited, not re-derived.
>
> The three parts land TOGETHER. Splitting them leaves two reachable slot authorities mid-way,
> which is the original defect class.

## Two measurements taken BEFORE writing this doc

Both were read off run 4's own artifacts (`wire_smoke.jsonl` + `live2-run4.log`, 2026-08-06 08:11,
354 admitted ForgeTicks). Both contradict what SLICE-LIVE-2b recorded from a code read. Neither is a
new run — the evidence was already on disk and had not been decomposed.

### M1 — B11 did NOT fire. The wrong slot was never caught by the KES window.

LIVE-2b concluded: *"`kes_period_for_slot(slot) == None` skips the entire forge attempt … a slot that
far out is comfortably outside any KES validity window, so `None` is the correct answer to a wrong
question"* and *"the only thing preventing it today is an accident of the KES range check."*

That accident does not exist on this venue. From the operator's own opcert and the venue genesis:

| quantity | value | source |
|---|---|---|
| opcert `kes_period` (start) | **970** | `ade-ops/preprod/ade-pool/keys/node.opcert` |
| `slotsPerKESPeriod` | 129_600 | preprod `shelley-genesis.json` |
| `maxKESEvolutions` | 62 | preprod `shelley-genesis.json` |
| naive logical slot (measured) | 131_976_696 | LIVE-2b probe |
| ⇒ absolute period | 1018 | `131_976_696 / 129_600` |
| ⇒ **evolution** | **48** | `1018 − 970`, inside `[0, 62]` |

`kes_period_for_slot(131_976_696) = Some(48)`. The 19-day-ahead slot passed the KES range check
**cleanly**. Corroborated end-to-end: all 354 `forge_result` records carry
`skip_reason = "tip_mismatch"`, which is only written inside the `Some(kes_period)` branch — and the
**first** record already carries it, while `last_forge_refused` starts `None`, so it cannot be a
stale sticky value.

This makes the slot defect **more** serious, not less: nothing downstream refused it. Had the tip
gate passed, Ade would have run leadership evaluation — and on election, signed a header — at a slot
19 days in the future. Part 2 is the only thing standing between that and a live invalid block.

### M2 — the measured live suppressor is a SEVENTH exit, not one of B1–B11. Name it **B12**.

All 354 admitted ticks were refused by the DC-NODE-15 catch-up gate, with a systematic operand
relationship:

```
354 / 354   forge_result  outcome=no_tip_available  skip_reason=tip_mismatch
354 / 354   local_tip_block_no − peer_tip_block_no == +1     (Ade's durable tip is AHEAD)
      4     distinct tip tuples  (the chain advanced 4 times during the ~6 min window)
```

Ade's durable tip is always **exactly one block ahead** of the peer-advertised tip — and every one of
those blocks was fetched *from that same peer*, so the peer demonstrably has it.

Mechanism, from `wire_pump.rs`: the followed-peer-tip signal is written **only** from the `tip` field
of a chain-sync message (`IntersectFound` / `RollForward` / `RollBackward` →
`FollowedPeerTipSignal::observe`). The message delivering block `N+1` advertises tip `N`. At the
chain tip no further message arrives until the *next* block, so the signal stays one block behind for
the entire inter-block interval (~88 s here, ~88 ticks). `durable_servable_tip == followed_peer_tip`
is therefore **structurally unsatisfiable while following a real cardano-node at the tip**.

The code already half-knows this: `CN-FOLLOW-01` exists because "the racing frontier makes [the
per-tick DC-NODE-15 exact-equality re-check] unsatisfiable". But `VenueRole::Unknown` — what run 4
used — takes the pure per-tick gate forever, and `VenueRole::Participant` only escapes it *after* one
caught-up instant clears the same gate once.

**B12 is NOT in this slice's scope**, and is deliberately not fixed here: changing a DC-NODE-15
operand or predicate is a consensus-adjacent mutation that needs its own census and its own negative
tests. It is recorded because it determines the live closure path (below), and because leaving it
undiagnosed would let a future session re-derive it at cost.

> ### ⚠ SUPERSEDED 2026-08-09 — do NOT implement the candidate fix below as written
>
> The paragraph that follows assumes B12 is an over-strict gate whose signal merely needs to be made
> more truthful. **That assumption is now unsafe.** The failed boundary crossing
> (`InvalidTxCarriesAuthorityEffect` at slot 130,350,133) leaves the accumulator **permanently one
> boundary behind**, and the accumulator is the frozen-leadership authority. Making the gate
> satisfiable while that holds could admit leadership evaluation against authority that is not
> aligned to the selected canonical chain — which is precisely what the gate exists to prevent.
>
> **B12 must be investigated AS A GATE, not assumed to be an unnecessary one.** It is the next
> runtime gate, not a confirmed defect. The `+1` may be benign observation order — or it may be a
> symptom of the still-open boundary defect, in which case the gate is doing its job and the fix
> belongs upstream.

Candidate fix, named but NOT taken here (see the supersession above): the signal is under-truthful,
not the predicate.
`FollowedPeerTipSignal` records only what a peer *says*; a block that peer *served and Ade durably
admitted* is stronger evidence that the peer has it. A signal of "highest block this peer has
demonstrably served **or** advertised" keeps the equality predicate intact, cannot let Ade forge while
genuinely behind (a higher advertised tip still dominates), and is strictly more truthful about peer
possession. That is a separate sealed slice.

## What this slice ships

### Part 1 — bootstrap-bound timing reconstruction

The complete wall-clock→absolute-slot timing history, constructed once from inputs the bootstrap has
**already verified**, and never rebuilt inside `operator_forge.rs`.

**The registry is selected by the STORE, not by the operator.** The durable sidecar records the
network's genesis hash at import (`SeedEpochConsensusInputs::genesis_hash`, bound at FirstRun against
the committed `NetworkProfile`; the live `ade-preprod-s7` receipt reads
`162d29c4e1cf6b8a84f2d692e67a3ac6bc7851bc3e6e4afe64d15778bed8bd86`, identical to the committed preprod
profile). The timing history is resolved **by that hash**. `--network` cannot choose it; a `--network`
that disagrees fails closed. This is proof 5 of the six-proof bar ("no operator configuration can
supply or override it") discharged by construction rather than by review.

Committed timing histories — timing/calendar fields only, no ledger semantics (the CE-L2c-12
boundary):

| network | `system_start_unix_ms` | segments `(start_slot, slot_length_ms, epoch_length_slots, start_epoch)` |
|---|---|---|
| preprod | 1_654_041_600_000 | `(0, 20_000, 21_600, 0)`, `(86_400, 1_000, 432_000, 4)` |
| preview | 1_666_656_000_000 | `(0, 1_000, 86_400, 0)` |

Each value is a venue fact, taken from that venue's own genesis rather than from folklore: preprod
byron-genesis `startTime = 1654041600`, `blockVersionData.slotDuration = 20000`, `protocolConsts.k =
2160` ⇒ 21_600-slot Byron epochs; shelley-genesis `systemStart` is the identical instant. Preview's
`TestShelleyHardForkAtEpoch = 0` ⇒ the Byron segment has **zero** slots, so preview is a single 1 s
segment from slot 0 and takes the same code path with no venue branch.

**The constants are not taken on faith — the store pins them.** The reconstructed calendar must
reproduce the durable bootstrap facts exactly:

```
reconstructed_epoch_start_slot(sidecar.epoch_no) == sidecar.epoch_start_slot
reconstructed_epoch_length (at that epoch)      == sidecar.epoch_length_slots
sidecar.seed_point_slot ∈ [epoch_start, epoch_start + epoch_length)
```

Live preprod: `86_400 + (304 − 4) × 432_000 = 129_686_400` == the sidecar's recorded
`epoch_start_slot=129686400`, seed point 129_813_427 inside it. Drop the Byron segment and the
reconstruction yields `129_600_000` — off by exactly the 86_400 × 19 s the defect is made of, and the
authority refuses to establish. The cross-check is mechanical and non-vacuous.

The domain start is `sidecar.seed_point_slot` — a bootstrap FACT, never the clock (CE-L2c-14, already
closed). Warm start reconstructs from the same durable inputs, so:

```
same bootstrap lineage + same timing inputs  ⇒  byte-identical DerivedTimingAnchor
```

**Warm-start lineage verification.** Before forging becomes active the authority re-derives the
history, re-derives the anchor, and verifies `anchor.is_derived_from(&history)` plus all three durable
bindings. Stated precisely, because it is easy to overclaim: the commitment binds *anchor ↔ history*
within the process; the three durable checks bind *history ↔ store*. Together they reject a wrong
`--network`, an edited timing table, and a store/venue mismatch. Nothing is persisted, so nothing is
verified against a persisted commitment.

### Part 2 — producer wiring, and REMOVAL of the naive path

`--mode node` receives the derived anchor and calls `anchor.slot_at(captured_ms)`. There is no second
reachable conversion, because the second conversion **ceases to exist**:

- `ForgeActivation::{anchor_millis, start_slot, slot_length_ms}` — **deleted**, replaced by the
  timing authority. The tick cadence and the idle poll read `timing.slot_cadence_ms()` (the active
  segment's own slot length), so there is exactly one source for that number too.
- `OperatorForgeMaterial::{anchor_millis, start_slot, slot_length_ms}` — **deleted**. This is the
  naive triple; `operator_forge.rs` stops emitting a conversion anchor at all. The genesis
  `systemStart` survives only as a fail-closed **cross-check** against the committed history.
- `ade_runtime::clock::{checked_millis_to_slot, SlotAlignmentError}` — **deleted**. Its sole
  production caller is `node_lifecycle.rs:3241`. Removal, not deprecation: unreachable-by-absence is
  the only form that cannot be walked back by a later "just keep the old path as a fallback".

`millis_to_slot` (saturating) survives only in `orchestrator/leadership_session.rs`, whose sole
entry point `run_node_until_shutdown` is reachable from **tests only** — no `Mode` dispatches to it.
Audited, not assumed: `Mode::{WireOnly, Admission, KeyGenKes, Produce, Node, BootstrapExport,
MithrilSnapshotFetch}`, and `--mode produce` derives slots by incrementing a counter from the
bootstrap tip, not from the wall clock. A CI gate pins both facts.

### Part 3 — B11 closure: a typed KES refusal

`kes_period_for_slot_checked(slot) -> Result<u32, KesSlotError>` becomes the single definition;
the existing `Option` accessor delegates to it (`.ok()`) so the two can never disagree.

```rust
enum KesSlotError {
    BeforeOperationalCertificateStart { slot: SlotNo, first_supported_slot: SlotNo },
    AfterOperationalCertificateEnd    { slot: SlotNo, last_supported_slot: SlotNo },
    PeriodArithmeticOverflow          { slot: SlotNo },
}
```

Routed as `ForgeRefused::KesWindow(KesSlotError)` with **three distinct** `ForgeSkipReason`
discriminators — the families must stay separable, so KES does not become one reason.

**And the outcome stops lying.** B11 was never literally silent: it fell through to
`ForgeResult { outcome: no_tip_available, skip_reason: null }` — indistinguishable from a genuine
absent tip. `ForgeOutcome::Refused` is added so an admitted tick that was refused reports a refusal;
`no_tip_available` narrows to what its name says.

`last_forge_refused` is also reset per tick. The mutation sweep narrowed this one from how it was
first written, and the narrower version is the true one: every path that *does* refuse overwrites the
field, so the sticky harm was confined to the single path that records nothing — no fence, no KES
failure, and no tip to build on. That tick re-emitted the previous tick's reason **and its tip
operands**. Narrow, but it is precisely the record an operator reads when nothing is happening, and
ruling it out by hand was a required step in diagnosing M1.

Completed invariant: **every admitted `ForgeTick` produces either a structured refusal or a
leader-schedule decision. No admitted tick may disappear, and none may report a reason that is not
its own.**

## Colour law

```
RED    capture UnixMillis (SystemClock)                      | sole wall-clock read
BLUE   DerivedTimingAnchor::slot_at(captured_ms) -> SlotNo   | authoritative
BLUE   KES window / forecast / leadership decisions          | authoritative
RED    signing and transmission
```

The venue timing registry is a **committed constant table** in `ade_node` (GREEN): it transports
reviewed venue facts inward and owns no conversion. The conversion and every binding check live in
`ade_core` (BLUE).

## Store-semantics decision — NEUTRAL, no bump

The anchor is reconstructed at every start from inputs already durable (`SeedEpochConsensusInputs`)
plus a committed in-binary constant table. **Nothing is persisted**: no snapshot, checkpoint, WAL,
store-metadata or fingerprint field gains a schedule commitment, and no existing persisted field
changes meaning. No stored data gains a new interpretation and no recovery behaviour changes — the
recovery path is byte-identical; only the forge's slot input changes.

`STORE_SEMANTICS_VERSION` stays at **v3**, so `~/.cardano-live1/ade-preprod-s7` opens unchanged.
Persisting the anchor would version that artifact and is deliberately not done.

## Mechanical acceptance criteria

| CE | Criterion | how it is judged |
|---|---|---|
| **CE-L2c-A1** | The complete timing history is reconstructed ONLY from bootstrap-verified inputs; the registry is selected by the durable genesis hash and no CLI value can override it | unit + a negative test where `--network` disagrees with the store ⇒ fail closed |
| **CE-L2c-A2** | The reconstruction reproduces the durable bootstrap facts (`epoch_start_slot`, `epoch_length_slots`, seed point in-epoch) or refuses | preprod 304 ⇒ 129_686_400; Byron-dropped ⇒ 129_600_000 ⇒ refusal |
| **CE-L2c-A3** | `same bootstrap lineage + same timing inputs ⇒ byte-identical anchor`; warm start verifies lineage before forging is active | equality over re-derivations + lineage assertion |
| **CE-L2c-A4** | An altered timing schedule is rejected | mutate a segment ⇒ `is_derived_from` false / binding mismatch |
| **CE-L2c-5** | The preserved preprod instant yields slot **130_338_561** through the ACTUAL node wiring | test drives the node-path type, not a local copy |
| **CE-L2c-1** | `--mode node` derives its forge slot ONLY through the derived anchor | wiring test asserts the forged slot equals `anchor.slot_at(tick)` |
| **CE-L2c-2** | The naive conversion is UNREACHABLE from forging | `checked_millis_to_slot` no longer exists; CI gate |
| **CE-L2c-6** | B11 returns a typed `ForgeRefused`, never a skip | unit + gate: no `if let Some(kes_period)` in the ForgeTick arm |
| **CE-L2c-A5** | An admitted tick never reports a reason that is not its own | per-tick reset test: refusal, then a clean tick, then a KES refusal ⇒ three distinct reasons |
| **CE-L2c-7/8/9** | The configured pool reaches `classify_leader_schedule`; the branch marker proves the KNOWN pool was evaluated; a decided `ForgeOutcome` is emitted | LIVE (see below) |
| **CE-L2c-10** | Replaying instant + history + binding reproduces the same slot and outcome | deterministic re-derivation test |
| **CE-L2c-11** | Negative-tested (mutations below) | each mutation must FAIL a named test or gate |

### Required mutations

restore the naive conversion · hardcode the preprod boundary · accept a truncated schedule as
complete · node path bypasses the anchor · B11 restored to `None` · diagnostic path fixed while the
node path stays old · known-pool evaluation replaced by `UnknownPool` · **drop the Byron segment from
the committed history** (must fail A2) · **let `--network` select the timing history** (must fail A1)
· **remove the per-tick refusal reset** (must fail A5).

## LIVE RESULT — run 5 / run 6, preprod, 2026-08-07

Store `~/.cardano-live1/ade-preprod-s7` (v3, unchanged — no semantics bump), peer docker
`cardano-node-preprod`, binary at `747b01ae`. Raw evidence:
`docs/evidence/run-stores/preprod-live2c/`.

**The timing authority establishes live, from the store's own facts.**

```
live2c-timing-authority: source=durable-genesis-hash venue=preprod
  store_genesis=162d29c4…bed8bd86   cli_network=preprod   genesis_cross_check=agreed
  bootstrap_epoch=304  durable_epoch_start_slot=129686400  anchor_slot=129813427
  domain_start_ms=1785496627000  cadence_ms=1000
  commitment=30fe202dcfbb1306af4cdd6ef5188e8ad5a912a7c0eacf5924969b38d25b45b5
```

Three things are worth reading off that line rather than assuming:

- `source=durable-genesis-hash` — the STORE selected the calendar. Both cross-checks (`--network`,
  the operator's real `shelley-genesis.json`) agreed rather than supplied anything.
- `durable_epoch_start_slot=129686400` — the committed calendar reproduced the store's own recorded
  epoch-304 start. A Byron-blind calendar yields 131_328_000 and refuses.
- The `commitment` is **byte-identical to the hermetic unit test's**, and identical again across two
  separate process starts (runs 5 and 6). That is CE-L2c-A3's reconstruction determinism, proven on
  the live venue and not only in a fixture.

**The 19-day defect is closed live.** The node's own tick probe, against the peer read seconds later:

| quantity | run 4 (before) | run 5 (after) |
|---|---|---|
| Ade `logical_slot` | 131,976,696 | **130,389,872** |
| peer tip at the same moment | 130,335,017 | 130,389,946 |
| **gap** | **+1,641,679** (≈19 days) | **−74** (seconds of read lag) |

E1/E2/E3 carried through: warm start under v3, `recovery_admit action=forward_fold
reason=forward_fold_no_reset` (a real anchor, never `anchor_absent`), KES VK fingerprint
`fd2f1de3…` matching the recorded identity, and `AT PEER TIP` sustained.

**CE-L2c-7/8/9 did NOT close, for reasons outside this slice's three parts.** Two distinct liveness
surfaces, both out of scope, and the second one was not visible before this run:

- **B12** (measured in run 4, §M2): the DC-NODE-15 gate is structurally unsatisfiable at the tip.
  Leadership is evaluated downstream of it.
- **B6, and it is not merely transient** (new, runs 5 and 6): `SyncStatus::WorkAvailable` preempted
  **every** planned iteration, so no `ForgeTick` was ever scheduled.

  | run | store at start | reduced checkpoint | loop rate | ForgeTicks |
  |---|---|---|---|---|
  | 4 (pre-slice) | at tip | 1.08 GB | ~1 / s | 354 of 363 probes |
  | 5 | 54k slots behind | 1.08 → 2.16 GB | ~1 / 60 s | 0 of 4 probes |
  | 6 | **at tip** | 2.16 GB | ~1 / 5 min | 0 of 2 probes |

  Run 5 alone would have read as "the catch-up backlog made iterations slow". **Run 6 refutes that**
  — it started at tip and was slower still. Whatever the cause, B6's severity is not a fixed small
  fraction, and the 8/363 sample LIVE-2b recorded came from the easy case. A deferral bound
  calibrated against that sample would be calibrated against the wrong one.

  **Cause not isolated, deliberately not asserted.** The loop time is spent inside `run_node_sync`
  and `advance_ledger_state_to_durable_tip`, both untouched by this slice, which makes the store's
  state the likely variable — but runs 4 and 6 differ in BOTH store state and binary, so these runs
  cannot separate them. The clean A/B (baseline binary, same store) was not run and is the next step
  if B6 is picked up.

  Run 6 also caught a real preprod reorg mid-measurement (`rollback_admit action=reset_to_settled
  rollback_target=130390901/5026202`, `anchor_after=absent`), which Ade handled with a typed rollback
  admission. Recovery then re-anchored and forward-folded — `recovery_admit action=forward_fold
  reason=forward_fold_no_reset anchor_before=130118424/5013815 durable_tip=130391033` — i.e. from the
  **epoch-305 start** to the tip, ≈272,600 slots. That is the bounded refold behaving as designed
  (`DC-EPOCH-26..31`), and it is also a concrete cost figure: with the settled rewind point at the
  epoch boundary, a reorg 272k slots into an epoch buys a fold of that length before the loop can
  plan another tick. Useful to whoever picks up B6; it changes nothing about parts 1–3.

### A starved loop derives a STALE slot — and it will look like this fix regressing

Recorded because the next person to see it will reasonably suspect the wrong thing.

`SystemClock::next_tick` returns the *scheduled* boundary and advances by exactly one slot per call;
it never skips forward to the current boundary. So the derived slot advances **once per loop
iteration**, not once per second of real time. Run 6's three probes, spanning ~15 minutes of wall
clock (≈900 slots), read `130390657 → 130390658 → 130390659` — **+1 each**.

The conversion is correct and the authority is correct; the *instant handed to it* is stale, because
a loop starved by B6 consumes ticks slower than they accrue. On a healthy loop (run 4, ~1 iteration
per second) the two rates match by construction and this never appears.

Two consequences worth stating plainly:

- **A stale `logical_slot` under starvation is NOT a slot-authority regression.** The discriminator
  is cheap: compare the probe's slot delta against the iteration count, not against wall clock. If
  it advances +1 per iteration, the clock is starved, not wrong.
- **Fixing B6's deferral does not automatically fix this**, and the obvious repair — skipping to the
  current boundary — changes *which slot a producer attempts*, which is consensus-adjacent. Out of
  scope here; named so it is a decision rather than an oversight.

Neither surface is caused by parts 1–3, and neither is fixed by them. Recorded rather than worked
around: changing a DC-NODE-15 operand, a sync-deferral bound, or the clock's tick-consumption model
is consensus-adjacent and needs its own census.

### B6 CENSUS — RUN 2026-08-07, and it answered. Hypothesis C.

Executed exactly as designed below: frozen store (`FROZEN-b6-census-s7`, `chmod a-w`), one
instrumented pass, all four candidates competing for the same elapsed time. Five passes, 662 s of
loop time. Evidence: `docs/evidence/run-stores/preprod-live2c/b6-census-arm-live2c.txt`.

| pass | blocks | sync | co-advance | total | dominant |
|---|---|---|---|---|---|
| 1 (catch-up) | 2974 | 127.2 s | 102.4 s | 229.6 s | sync 55% |
| 2 (at tip) | 9 | 17.8 s | **115.0 s** | 132.7 s | **co-advance 87%** |
| 3 (at tip) | 8 | 10.4 s | **84.6 s** | 95.0 s | **co-advance 89%** |
| 4 (at tip) | 6 | 10.4 s | **104.7 s** | 115.2 s | **co-advance 91%** |
| 5 (at tip) | 3 | 10.9 s | **78.9 s** | 89.8 s | **co-advance 88%** |

**A ~1000× range in work produces the same ~80–115 s.** `advance_ledger_state_to_durable_tip` is a
**fixed per-pass cost**, not a per-block one — 73% of all loop time, 87–91% of every at-tip pass.

| | verdict |
|---|---|
| **A** one sync call takes minutes | **REFUTED** — at tip, sync is 10–18 s of a 90–133 s pass |
| **B** unbounded work per pass | **REAL, NOT THE DRIVER** — pass 1 took on 2974 blocks in ONE dispatch, so the pass genuinely is unbounded; that explains the catch-up pass, not the steady state |
| **C** downstream recovery/refold dominates | **CONFIRMED** |
| **D** planner reached late | **ELIMINATED** — `to_planner_ms = 0` on all five passes |

`next_tick_ms = 0` on every pass independently re-confirms the starved-clock finding: the clock never
sleeps, it hands back the stale boundary immediately.

#### Why the starvation is self-sustaining — the mechanism, now named

Block arrival measured from the census itself: **11.9 s, 19.2 s, 29.9 s per block**. A pass costs
~90–133 s, dominated by a fixed ~80–115 s co-advance. So 3–9 blocks *always* queue while a pass runs,
`has_work_ready()` is therefore true every time the planner is consulted, and the planner returns
`SyncOnce` forever. **`NoWorkReady` — the state a ForgeTick requires — is unreachable while a pass
costs more than the block interval.** That is a design-level condition, not an unlucky run: the loop
cannot outrun the chain while paying a fixed six-figure-millisecond cost per pass.

**Implication for the B6 slice (named, not designed here): the lever is the FIXED COST, not the
deferral policy.** Making the co-advance proportional to work admitted — or amortising it — drops a
tip pass toward its ~10–20 s sync cost, below the block interval, at which point `NoWorkReady`
becomes reachable *without touching the deferral rule at all*. The obligation split below still
holds; this just says which side the evidence points at first.

#### The baseline arm was NOT run, and why

The A/B existed to ask "did LIVE-2c cause this?". The attribution answers it more directly: 100% of
measured loop time sits in three functions, and **no slice hunk falls inside any of them** — computed
by intersecting the `68e62c78..ef310eba` diff's line ranges against each body at HEAD
(`run_node_sync` 599–973, `advance_ledger_state_to_durable_tip` 2752–3011,
`maybe_activate_epoch_boundary` 3019–3061: **NONE**).

That is a **source-level** argument. It does not rule out a codegen-level effect from changes
elsewhere in the same crate. If that residual ever matters, the back-to-back A/B is still how to
settle it — the frozen store is kept for exactly that.

### The B12 entry point — findings-first, ONE planner iteration, five-way discriminator

Same method that resolved B6: capture the operands B12 actually compares, in a SINGLE instrumented
planner iteration, and let the combination name the branch. Do not touch the equality gate until the
five candidates below are mechanically distinguished.

**Operands to capture in one iteration:**

| operand | why |
|---|---|
| local selected tip — point / hash / block_no | what the gate compares from our side |
| peer-announced tip — point / hash / block_no | what it compares from theirs |
| ChainSync cursor / intersection state | whether the announcement is current |
| durable tip | whether local "selected" and "durable" agree |
| accumulator authority point | whether leadership authority is aligned to that prefix |
| **whether the peer value is sampled BEFORE or AFTER local admission** | the observation-order question, and the cheapest thing to get wrong |
| **B12's own verdict on that tuple** | makes the record SELF-CHECKING: the verdict must be derivable from the operands beside it. Without it the next reader infers what the gate did instead of reading it, which is how a wrong operand set survives review. |

**The five candidates for `local − peer == +1`:**

1. benign observation-order difference (peer sampled before local admission);
2. a stale peer-tip cache;
3. local durable state legitimately one block ahead;
4. a chain-selection mismatch;
5. a symptom of the still-open failed-boundary / refold defect.

Only (1)–(3) would make the gate over-strict. (4) and (5) mean the gate is CORRECT and the fix is
upstream of it. That is why the operand table includes the accumulator authority point: it is what
separates "the gate is too tight" from "the authority is stale and the gate caught it."

### Tier split for the remaining work

| tier | statement |
|---|---|
| **True** | No leadership evaluation from authority that is not aligned to the selected canonical chain. |
| **Derived** | Cardano forge readiness must bind slot, selected tip, and leadership authority to the SAME chain prefix. |
| **Release** | CE-L2c-7/8/9 require a real known-pool decision on the live `--mode node` path. |
| **Operational** | The large refold cost is operational ONLY if it cannot alter authoritative state. Because the crossing currently FAILS and leaves the accumulator behind, that part is a **correctness** issue, not merely performance. |

That last row is the one to keep. B6's cost looked purely operational right up until the decomposition
showed the cost was a failing crossing — at which point the same measurement became a correctness
finding. The remaining refold work inherits that: it is not a performance item while the crossing
fails.

### The B6 entry point — a CENSUS first, not a scheduling fix

The A/B is baseline binary + the same store, and it exists to answer ONE question before anyone
touches scheduling: **what consumes the 5–8 minutes per iteration?** Four candidates, mutually
distinguishable by instrumenting the authoritative path rather than by argument:

| | hypothesis | what would confirm it |
|---|---|---|
| **A** | one `SyncOnce` operation itself takes minutes | time inside a single `run_node_sync` call |
| **B** | the loop processes an unbounded / batched amount of sync work per pass | blocks admitted per pass grows with backlog |
| **C** | a downstream recovery / refold path dominates the iteration | time inside `advance_ledger_state_to_durable_tip` / the co-advancer |
| **D** | another liveness gate prevents the planner from returning | the planner is reached late, not slowly |

Run 6 already leans C (the post-reorg fold from the epoch start, ≈272,600 slots) but leans is not
proves, and run 5 leaned the same way for a reason run 6 refuted. Measure before fixing.

**Method: attribute elapsed time across A/B/C/D within ONE complete planner iteration.** This is a
single time-attribution pass, not four experiments — instrument one iteration end to end and see
where the 5–8 minutes actually goes. Four separate probes would let each hypothesis be confirmed in
isolation on different iterations, which is how a favourite hypothesis survives.

**The A/B trap: "same store" is a moving target.** The store drifts from the chain whenever it is
not running (run 5 opened 54k slots behind for exactly this reason), and run 6 left it changed again
— chain.db 8.59 → 10.47 GB, a reorg, and a ≈272,600-slot refold. So the two arms must run
**back-to-back from the same starting state**; a baseline arm today and a new-binary arm next week
compare two different stores and prove nothing. If the arms cannot be run back-to-back, snapshot the
store first and restore it between them — and treat that as part of the experiment design, not
cleanup.

*Practical constraint on that snapshot, measured 2026-08-07.* `ade-preprod-s7` is **8.8 GB actual but
12.8 GB apparent** — its `chain.db` / `.redb` files are sparse — against **21 GB free on a disk at
96%**. A sparse-preserving copy (`cp --sparse=always`, `rsync -S`) costs ~8.8 GB and leaves room;
a naive `cp -a` / `rsync` without it costs ~12.8 GB, i.e. 4 GB more than the directory appears to
need, *before* either arm grows `chain.db` by following (run 5 added ~1.9 GB catching up 54k slots).
Reclaim first if needed — `~/Code/rust/*/target` build caches, never the CE-4 corpus.

### Two SEPARATE obligations — do not merge them

They look like one problem ("the producer misses its slot") and are not:

1. **B6 bounded deferral** — due forging work may not be postponed indefinitely by sync. A liveness
   bound. Non-consensus.
2. **Clock catch-up semantics** — after a long deferral the producer must not replay obsolete slot
   boundaries one at a time. **Consensus-sensitive.** "Jump to the current slot" sounds like a
   performance fix and is not: it changes *which slot is evaluated for leadership*, and therefore
   which slot a producer may sign. It needs its own proof obligation, not a patch under B6.

Fixing (1) does not fix (2): a loop restored to ~1 iteration/s keeps up by construction, but any
future stall re-opens the same lag. Fixing (2) without (1) leaves the starvation in place.

## Live closure path — stated honestly, because M2 changes it

Parts 1–3 are necessary and mechanically provable on their own. They are **not sufficient** for
CE-L2c-7/8/9 on the live venue, because B12 refuses every tick before leadership is consulted.

Order of attack, cheapest discriminator first:

1. Land parts 1–3 with their unit/CI proofs. The slot becomes correct and every admitted tick becomes
   typed — independent of B12.
2. Add an emit-only probe naming, per admitted block, the block Ade admitted and the peer-advertised
   signal at that instant. One run turns M2's mechanism from inferred to measured.
3. Run the live venue with the venue **declared** (`--participant-venue` — a public multi-producer
   network is exactly `CN-FOLLOW-01`'s Participant venue, an operator declaration, not a semantic
   change). If the DC-NODE-15 gate clears once, the Participant mode latches and ticks reach
   leadership ⇒ CE-L2c-7/8/9 close.
4. If the latch never fires, B12 is a structural defect in the signal and gets its **own sealed
   slice** (the `served-or-advertised` signal above). LIVE-2c's live half is then reported as blocked
   on it — with parts 1–3 landed and proven — rather than claimed.

A non-leader result counts only via the `known_pool_evaluated` branch. Forging and Haskell-peer
acceptance remain LIVE-3.

## Not claimed

No forge success. No peer acceptance. No B6 (sync-deferral starvation) work. No DC-NODE-15 predicate
or operand change. No store-semantics bump.
