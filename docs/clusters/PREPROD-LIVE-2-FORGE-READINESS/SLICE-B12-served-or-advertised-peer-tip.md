# SLICE B12 — the followed-peer-tip signal records only what a peer SAYS

**Entry state:** `8b9e9e38`. The BND line is closed and the accumulator is healthy live
(`56e0a4e4` / `f19be266` / `ba2e95e9`, store semantics v6). The B12 census RAN on 2026-08-09
(`fcbabb67`, evidence `b12-census-classified.txt`) and answered **candidate (1), benign observation
order**, 762/762 unanimous. The 2026-08-09 supersession that forbade acting on that classification
named exactly one precondition — *"a healthy accumulator, not a classification"* — and BND-2d
discharged it (`8b9e9e38`).

This slice takes the fix the census licensed. It changes **no predicate**, **no BLUE code**, and
**no durable state**.

---

## 0. WHAT IS ALREADY SETTLED — do not re-derive any of it

| settled | by | do not re-open |
|---|---|---|
| the `+1` is benign observation order | census `fcbabb67`, 762/762 `pre_admit` | the five-way classification |
| chain-selection mismatch is refuted | `local_parent_is_peer_tip=yes` **and** `peer_tip_on_our_chain=yes`, two independent routes | candidate (4) |
| "local legitimately ahead" is refuted | `cardano-cli query tip` on the peer's own N2C socket read our exact local hash for all 6 tuples | candidate (3) |
| stale cache is refuted | `announcements_since_admit=0`, announcements +6 / advances +6 over 6 blocks | candidate (2) |
| the accumulator is no longer the reason to wait | BND-2d live: cursor through 130,350,133, crossed 305→306, folded to durable tip 130,739,648 | candidate (5) as a blocker |
| the recorded tuple IS the tuple the gate decided on | `verdict` vs `gate_recheck`, 762/762 agree | the operand set's sufficiency |

**No new census.** [[census-first]] requires one before a *consensus mutation that can terminal on
reference-valid state*. This slice mutates no consensus rule, no authoritative state and no durable
artifact — and the census this work needed already ran and already answered.

---

## 1. THE CONTRACT — answered before any design

> **What constitutes evidence that a peer possesses a block, and which of those evidences may the
> forge-admissibility gate consume?**

DC-NODE-15 exists to stop Ade forging a successor to a tip the peer has not got. Its predicate is

```rust
// crates/ade_node/src/node_sync.rs:1157
durable.hash == peer.hash && durable.block_no == peer.block_no  =>  CaughtUp
```

and that predicate is **correct**. The question is entirely about its second operand.

### C1 — there are two kinds of evidence, and they are not equal

| evidence | what it is | can it be wrong about possession? |
|---|---|---|
| **advertisement** | the peer's chain-sync `tip` field says it holds B | yes — stale, absent, or (from a hostile peer) simply false |
| **service** | the peer delivered B's bytes to Ade over block-fetch **and Ade durably admitted them** | **no.** A peer cannot serve a block it does not have |

Service is strictly stronger evidence of possession than advertisement. It is a demonstration, not
testimony.

### C2 — the signal today records only the weaker one, and the protocol makes that lag

`FollowedPeerTipSignal::observe` (`node_sync.rs:1328`) is written from `AdmissionPeerEvent::TipUpdate`
and nothing else (`node_sync.rs:231`). So the gate compares Ade's durable tip against what the peer
last *said*.

At the steady-following frontier the peer announces once per block, carrying the **parent** of the
header it is delivering, then falls silent until the next block. The census measured this as a
structural property, not a race: `local_parent_is_peer_tip = yes` and
`announcements_since_admit = 0`, 762/762. A ForgeTick fires only on `NoWorkReady` — i.e. *after*
the delivered block is admitted — so at every instant the gate is consulted, Ade holds the successor
of what the peer last announced. `local − peer == +1`, always.

The peer served us that successor. It provably has it. The signal simply cannot say so.

### C3 — the answer

> **A peer demonstrably holds block B if it advertised B, or if it served B to Ade and Ade durably
> admitted it. The admissibility signal must report the strongest such evidence, not only the
> weakest.**

The predicate stays exactly as it is. The operand becomes truthful.

### C4 — why this cannot let Ade forge while genuinely behind

This is the property that makes the contract safe, and it is the one to test first, because it is
the one the 2026-08-09 supersession was protecting.

During catch-up the chain-sync `tip` field carries the peer's **real head**, which is far *ahead* of
the block it is currently feeding us. Ade at block 5,000,000 receiving from a peer at 5,042,000 sees
`advertised = 5,042,000` and `served = 5,000,000`. **The advertisement dominates**, the tips
disagree, and the gate refuses — exactly as today.

**This is measured, not assumed — and out of an artifact already in hand.** The census recorded
`peer_announcements=9798` against `peer_advances=13` on a run whose catch-up was ~9,800 blocks over
15m45s. Had the `tip` field carried the *parent of each delivered header*, the two counters would
have moved together and `advances` would read ~9,798. It reads **13** — one per ~73 s, which is the
preprod block interval. So across an entire ~9,800-block catch-up the advertisement sat almost
still, at the peer's own head, while service climbed underneath it. The `+1` is a property of the
**frontier** — `order=pre_admit`, `announcements_since_admit=0` — and not of following in general.

That is the discriminator this slice's safety rests on, and it cost no new run: `13` vs `9,798`.

The two evidences therefore split the two halves of DC-NODE-15's duty cleanly:

- **behind** → the advertisement leads, dominates, and refuses. Catch-up enforcement intact.
- **at the frontier** → the advertisement lags by one, service leads, and the gate can finally see
  that Ade is caught up.

Service can only ever *raise* the signal to a block Ade has itself durably admitted. It cannot raise
it past Ade's own tip, so it can never manufacture a `CaughtUp` for a tip Ade does not hold.

---

## 2. THE CHEAPER DISCRIMINATOR IS REFUTED — and that refutation becomes a test

The LIVE-2c handoff records an "order of attack, cheapest discriminator first" whose step 3 is:
declare the venue with `--participant-venue`; if the DC-NODE-15 gate clears once, the Participant
mode latches and ticks reach leadership without any B12 work.

**It cannot clear.** `participant_forge_decision` returns `UseInitialCatchupGate` in the initial
modes (`node_sync.rs:1862`), and the Participant arm routes that to the *same* `dc_node_15_refusal`
(`node_lifecycle.rs:4523`) that the default arm calls (`node_lifecycle.rs:4551`) — on operands bound
**once**, before the venue branch (`node_lifecycle.rs:4384` and `:4391`). Same function, same
values, same verdict. The Participant latch fires only on `CaughtUp`, so the structural `+1` blocks
it exactly as it blocks the default path.

That is a code read, and code reads on this cluster have been wrong four times. **CE-B12-8 converts
it into a test**: the census's real operand tuple is driven through all three venue routes and all
three must refuse. A live run to discover this would cost an hour and prove less.

---

## 3. INVARIANT

**DC-NODE-47 (new).** The followed-peer-tip admissibility signal reports the strongest available
evidence that the followed peer possesses a block: the tip it advertised, or a tip it **served** and
Ade **durably admitted**, whichever is higher by `block_no`. The served fact is written only after a
successful durable admit of a block delivered by the peer feed, is cleared by any rollback, and
never reaches `next_block`, `pump_block`, a chain selector, or any authoritative transition. It may
only make a forge admissible where the peer provably holds Ade's own durable tip; it can never
select, prefer, or replace a chain.

**DC-NODE-15 — STRENGTHENED, not modified.** The predicate is byte-unchanged. What changes is that
its operand is now the strongest available evidence rather than the weakest. Append this cluster to
`strengthened_in`; the statement stands as written.

**DC-NODE-34 — untouched, and deliberately so.** See §4.4.

---

## 4. DESIGN

### 4.1 The served fact

`FollowedPeerTipSignal` gains one field beside `latest`:

```rust
pub struct FollowedPeerTipSignal {
    latest: Option<TipPoint>,   // advertised — unchanged, still written only by observe()
    served: Option<TipPoint>,   // DC-NODE-47 — written only at a successful durable admit
    census: FollowedPeerTipCensus,
}
```

`tip()` stops returning `latest` and returns the combination rule of §4.3. Nothing else about the
struct's role changes: it is still consumed *only* as a forge-admissibility input, still write-only
from the drain path, still incapable of advancing a tip.

### 4.2 Where it is written — the durable admit, not the receipt

The served fact is recorded **after** the admit succeeds, and its value is the durable servable tip
projection *the gate's other operand already uses* (`ChainDbServedSource::new(chaindb).tip()`). Two
consequences worth stating:

- **no decode, no memoisation, no per-tick cost.** The census's own instrument design note warns
  that an operand needing a durable read + decode on the tip pair reintroduces exactly the fixed
  per-tick cost B6 removed. Reusing the existing projection at a boundary that already computes it
  avoids that entirely. The census measured `durable_eq_serve = true` — the two reads of "local"
  agree — so this is the same value by construction.
- **a rejected block records nothing.** Receipt is not possession-evidence Ade may act on; a block
  that failed validation was never admitted and never becomes a served fact.

Per DC-NODE-16 an idempotent re-admit (the block is already durably present, byte-identical) is a
no-op for state — but the peer still *served* it, so it still records. Possession is what is being
evidenced, not novelty.

### 4.3 The combination rule — pure, total, GREEN

```
tip() = match (advertised, served)
    (None,    None)    => None
    (Some(a), None)    => a
    (None,    Some(s)) => s
    (Some(a), Some(s)) => if s.block_no > a.block_no { s } else { a }
```

Strictly-greater, not `>=`. The tie case — same `block_no`, different hash — is a peer that served
one block and advertises a different one at that height, i.e. a fork the AO owns. **The
advertisement wins**, the tips disagree, and the gate refuses. Conservative by construction, and
`TipMismatch` is preserved as the diagnostic rather than collapsing to `NoFollowedPeerTip`.

### 4.4 What clears it

- **rollback.** `NodeSyncItem::RollBack` clears `served`, so the signal falls back to the
  advertisement rather than pinning on a block that may no longer be on the selected chain. A stale
  served fact would only ever *refuse* (the predicate compares hash and `block_no`), so this is a
  liveness and truthfulness fix rather than a safety one — but a signal that knowingly names an
  abandoned block is not a signal worth keeping.
- **nothing else.** In particular the signal is **not** peer-scoped, and that is a decision, not an
  oversight. The advertisement half is already peer-blind — `observe` ignores
  `TipUpdate { peer, .. }` (`node_sync.rs:231`) — and the live path is single-best-peer FOLLOW,
  one merged feed. Scoping one half and not the other would combine two differently-scoped facts,
  which is incoherent. Scoping *both* is per-peer candidate tracking, which is **DC-NODE-35's**
  job, latent until multi-peer follow lands. So DC-NODE-34 stays literally intact: peer identity
  remains provenance-only on this path (`node_sync.rs:644`), and this slice consumes none of it.

  **Handed forward, explicitly:** when multi-peer follow activates, the served fact and the
  advertisement must be scoped together, under DC-NODE-35.

### 4.6 POSSESSION vs TESTIMONY — the consumer split, found by auditing the mirrors

`tip()` has four consumers, and they are **not** asking the same question. Auditing them was not
optional: a fix to one documented path needs the others audited, and this one changes a value three
other call sites already read.

| consumer | question it asks | operand | why |
|---|---|---|---|
| the ForgeTick gate (`node_lifecycle.rs:4391`) | does the peer **hold** the tip I would build on? | `tip()` — combined | possession. The whole point of the slice |
| the fork-switch fence (`:4210`) | same question, same `forge_followed_tip_admission` predicate | `tip()` — combined | **two call sites of one predicate must not see two different operands.** Leaving this on the advertisement would preserve the exact defect being removed, in a mirror |
| convergence evidence (`:4085`, `:6519`) | what did the peer **say**? | `advertised()` | testimony |

The evidence split is load-bearing and was nearly missed. Both evidence sites feed
`derive(&outcome, &tip)` → `AgreementVerdict::{Agreed, Lagging, Diverged}`. The combined signal
folds in the block Ade just admitted, so feeding it there would make **every** admit read `Agreed`
by construction — an evidence stream that always agrees, silently. `advertised()` keeps both sites
byte-identical to their pre-slice behaviour.

So the signal exposes both halves separately and each consumer takes the one that answers its own
question. A single `tip()` for everything would have been simpler and wrong.

### 4.5 What does NOT change

`forge_followed_tip_admission` (`node_sync.rs:1153`), `dc_node_15_refusal`
(`node_lifecycle.rs:1859`), the three venue routes, `ForgeRefused`, the KES window, the forge base,
the co-advance pass order, and every BLUE file. The diff is one field, one write site, one clear
site, and one pure function.

---

## 5. MECHANICAL ACCEPTANCE CRITERIA

| CE | Criterion | judged by |
|---|---|---|
| **CE-B12-1** | The census's real frontier tuple (advertised = our tip's parent, served = our tip) resolves to `CaughtUp` | unit, operands taken from `b12-census-classified.txt` |
| **CE-B12-2** | **THE CATCH-UP CONTROL.** Advertised far ahead of served ⇒ advertised dominates ⇒ `NotCaughtUp{TipMismatch}`. Non-vacuity for the whole slice; the operands come from the measured catch-up shape of §C4 (`peer_advances=13` over ~9,800 blocks), not from an invented gap | unit |
| **CE-B12-3** | A self-forged durable tip is not a served fact: the signal still reports the peer's block and the gate refuses | unit |
| **CE-B12-4** | A rollback clears the served fact; the signal falls back to the advertisement | unit |
| **CE-B12-5** | Served is recorded only after a **successful** durable admit — a rejected block records nothing | unit |
| **CE-B12-6** | `forge_followed_tip_admission` is byte-unchanged and still requires equality on BOTH `hash` and `block_no` | structural gate (existing `ci_check_forge_followed_tip_admission.sh` assertion (b)) |
| **CE-B12-7** | Neither `served` nor `tip()` reaches `select_best_chain` / a chain selector / `next_block` / `pump_block` | structural gate (extends existing assertion (d)) |
| **CE-B12-7b** | The two convergence-evidence sites consume `advertised()`, not `tip()` — an `AgreementVerdict` is testimony and must not read `Agreed` by construction | structural gate (§4.6) |
| **CE-B12-8** | **All three venue routes** (SingleProducer / Participant / Unknown) refuse on the pre-fix census tuple — the `--participant-venue` discriminator was never available | unit |
| **CE-B12-9** | Same `block_no`, different hash ⇒ the advertisement wins ⇒ refuse | unit (tie-break) |
| **CE-B12-10** | **LIVE**: on preprod with a healthy accumulator, admitted ForgeTicks stop refusing `tip_mismatch` and reach leadership evaluation; the `known_pool_evaluated` branch marker appears ⇒ **CE-L2c-7/8/9** | live |
| **CE-B12-11** | Negative-tested | mutations below |

### Required mutations

Make `served` dominate unconditionally, ignoring the `block_no` comparison (must fail **CE-B12-2** —
this is the mutation that would reintroduce the exact danger the supersession named) · record
`served` on receipt instead of after admit (must fail CE-B12-5) · record the self-forged tip as
served (must fail CE-B12-3) · drop the rollback clear (must fail CE-B12-4) · make `served` win ties
(must fail CE-B12-9) · weaken the predicate to a `block_no`-only compare (must fail CE-B12-6) ·
route `tip()` into the chain selector (must fail CE-B12-7) · point either evidence site at `tip()` (must fail CE-B12-7b).

---

## 6. STORE SEMANTICS — NEUTRAL, and that is checkable

`STORE_SEMANTICS_VERSION` stays **6**. The signal is in-memory, rebuilt from the live feed on every
start, and nothing it holds is persisted, hashed, replayed or served. Neither `node_sync.rs` nor
`node_lifecycle.rs` is in the lock's declared surface (`ci/store-semantics-surface.lock`), so
`ci/ci_check_store_semantics_lock.sh` passes unchanged — and is **run anyway**, because the v3→v4
and v4→v5 bumps both skipped their own gate and only a 100-commit audit found it. A gate that is not
run enforces nothing, including a gate expected to pass.

---

## 7. EXPLICITLY NOT IN THIS SLICE

- **No predicate change.** DC-NODE-15's equality is untouched. If this slice ever needs the
  predicate relaxed to succeed, the design is wrong.
- **No peer scoping.** §4.4 — it belongs to DC-NODE-35 with the advertisement half, or to neither.
- **No BLUE change, no durable state, no store bump.**
- **No CLK (clock catch-up).** Consensus-sensitive, separately obligated.
- **No forge success or peer-acceptance claim.** CE-B12-10 closes CE-L2c-7/8/9 — a real
  *leadership decision* on the live `--mode node` path. Forging and Haskell-peer acceptance remain
  LIVE-3.
- **No `LeaderValueAboveThreshold` work.** See the venue note below; it is a separate open blocker
  and folding it in here is how scope creep starts.

---

## 8. VENUE, AND ONE KNOWN RISK TO THE LIVE LEG

Target is the local preprod container per `CLAUDE.md`; store `~/.cardano-live1/ade-preprod-v6`
(4.8 GB, v6, accumulator healthy at durable tip 130,739,648 as of 2026-08-16).

**The risk, stated before the run rather than after it:** the v6 store is ~13 days stale, so the
live leg needs a long catch-up, and a long catch-up is exactly where the open
`LeaderValueAboveThreshold` blocker fires — it killed BND-2d's leg 1 at ~383k slots. It did not
damage state (a warm restart resumed and finished the proof), so restarts are a workable
mitigation. **If CE-B12-10 cannot reach the frontier because of it, that is not a B12 failure**: it
promotes the `LeaderValueAboveThreshold` slice to a hard prerequisite, and B12's in-tree half lands
with CE-B12-10 recorded as blocked on a named, separate defect — reported, never claimed.

Disk: `/` at 92%, 38 GB free. `FROZEN-b6-census-s7` (8.2 GB) is v3 evidence and is **not**
retirable. Read `reference_machine_disk_reclaim` and `docs/evidence/run-stores/RETENTION.md` —
manifest first, then drop — before reclaiming anything.

---

## 9. STATUS AT IMPLEMENTATION — in-tree COMPLETE, live PENDING

`DC-NODE-47` is registered **`partial`**: every in-tree criterion is met and the live bar
(CE-B12-10) is not. It flips to `enforced` on that evidence and on nothing else.

| | |
|---|---|
| CE-B12-1/2/3/4/5/8/9 | **green** — 7 tests, `crates/ade_node/tests/b12_served_or_advertised_peer_tip.rs` |
| CE-B12-6/7/7b | **green** — `ci_check_followed_peer_tip_served_evidence.sh` + `ci_check_forge_followed_tip_admission.sh` |
| CE-B12-11 | **green** — all eight required mutations each break something (six caught by the gate, two by the tests) |
| CE-B12-10 | **PENDING** — the live leg, §8 |
| regressions | none: `ade_node` 644 passed / 0 failed, `ade_runtime` 581 passed / 0 failed |
| store semantics | NEUTRAL, version 6; the lock gate was **run** and passed unchanged |

### A gate repaired in passing — enforcement debt DISCOVERED, not created

CE-B12-6 cites `ci_check_forge_followed_tip_admission.sh`, so the slice ran it. **It had been failing
silently since ECA-5 (`26565bec`).** Its `prod_body` truncated at the first bare `#[cfg(test)]`, and
that commit inserted an inline `#[cfg(test)] async fn run_node_sync_no_eview` shim *above every
symbol the gate inspects* — so the gate saw an empty body and failed closed on emptiness rather than
on the code. DC-NODE-15's structural assertions have been unenforced for that whole span.

Both gates now truncate at the **trailing `#[cfg(test)]` module** only (`#[cfg(test)]` + optional
`#[allow(...)]` lines + `mod `), so production code below an inline test shim stays visible. The
repaired gate passes against current code.

This is the project's own lesson arriving from the other direction: *ask which gate your change
should make angry, and run THAT one.* Running it is what found that it could not see anything at all.
