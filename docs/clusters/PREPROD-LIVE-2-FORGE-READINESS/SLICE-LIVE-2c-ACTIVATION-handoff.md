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

Candidate fix, named but NOT taken here: the signal is under-truthful, not the predicate.
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
