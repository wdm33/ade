# SLICE LIVE-2 — reach the genuine forge-decision surface on established epoch-305 authority

> **OPEN — DOC BEFORE IMPL.** Unblocked by CRE-S7 (`8b41b9e3`), which established the epoch-305
> accumulator/leadership anchor. Runs on `~/.cardano-live1/ade-preprod-s7` (semantics v3).

## Intent

> Prove that a warm-started, post-CRE-S7 preprod node reaches the genuine forging decision surface
> using established epoch-305 authority.

## Hard boundary — what LIVE-2 is NOT

LIVE-2 proves **forge readiness**, not successful forging and not Haskell-peer acceptance. Those are
**LIVE-3**. A **non-leader result is acceptable, sufficient evidence** for LIVE-2 provided the full
legitimate decision path was reached — that is the point of the slice, not a consolation outcome.

Stated because the failure mode here is social, not technical: a run that reaches `not_leader` is easy
to under-claim ("nothing happened") or over-claim ("we can forge"). It is neither. It is proof that
every authority input a forge depends on was present, established, and consulted.

## Required evidence

| # | Evidence | how it is judged |
|---|---|---|
| **E1** | Warm start from `ade-preprod-s7` succeeds under semantics **v3** | store opens; no `StoreSemanticsVersionMismatch` |
| **E2** | The epoch-305 accumulator and leadership anchor are **restored and verified** | recovery reports a real anchor (`forward_fold` from an anchor, never `anchor_absent`), and the accumulator has a **crossed** 305 boundary — never a stalled one |
| **E3** | KES, VRF, operational certificate and pool identity load through the **RED custody paths** | forge-capable start; partial key sets fail closed (`EXIT_NODE_FORGE_KEY_INGRESS_FAILED = 44`) |
| **E4** | The production loop reaches **leader evaluation** for eligible slots | a `ForgeOutcome` of `not_leader` (or `succeeded`) — i.e. the evaluation ANSWERED. `no_tip_available` is NOT reaching the surface |
| **E5** | Every non-forge outcome emits a **typed** reason | `ForgeOutcome` + `ForgeSkipReason`; no untyped/log-only skip |
| **E6** | No silent skip, generic log-only failure, seed-window fallback, CLI oracle, bridge authority, or unestablished accumulator authority is used | structural + log assertions |
| **E7** | Restart and replay produce the **same** authority and forge-decision outputs | two runs, byte-comparable decision surface |

## The surface as it exists today — measured, before writing anything

| piece | state |
|---|---|
| `ForgeOutcome` | **exists, closed**: `Succeeded` / `NotLeader` / `Failed` / `NoTipAvailable`. `NotLeader` is documented as *"the recovered-surface leader check decided the operator is not the leader for this slot"* — so **E4's positive evidence already has a typed representation** |
| `ForgeSkipReason` | **exists, closed**, 7 variants — all **pre-evaluation** fences (`NoFollowedPeerTip`, `NoDurableServableTip`, `TipMismatch`, `SingleProducerFence`, `ReselectionPending`, `ParticipantFence`, `ForgeBaseChangedBeforeSign`) |
| `ForgeRefused` → `ForgeSkipReason` | total over the refusal variants; `None` when no typed refusal was recorded |
| operator keys | present at `~/Code/rust/ade-ops/preprod/ade-pool/keys/` (cold / kes / vrf / opcert / pool.id) |

So the two enums are **complementary, not redundant**: `ForgeSkipReason` says *why the loop refused
before evaluating*; `ForgeOutcome::NotLeader` says *the evaluation ran and answered no*. E5 must be read
against both, and LIVE-2's success looks like `outcome=not_leader` with `skip_reason` absent.

### The gap — WORSE than first written, and it had to be fixed BEFORE the evidence run

The first draft of this doc said the gap was a missing typed reason. Reading the forge path showed
something stronger:

```rust
Err(_) => None,                       // "Unknown pool / outside horizon => not a leader"
None   => CoordinatorEvent::ForgeNotLeader { .. }
```

All three `LeaderScheduleError` variants collapsed into a confident `not_leader`. So an **unestablished
leadership authority reported not-elected** — the CRE-S7 shape, but spoken by the forge path.

The decisive consequence is evidentiary, not just diagnostic: **a `not_leader` outcome could not be
trusted as proof that the decision path was reached**, because the identical outcome was produced when
it was not. E4 was therefore unprovable against that code *no matter how the live run went*, which is
why this landed before the run rather than after it.

Fixed in `4985a59b` — `classify_leader_schedule(res, pools_in_view)`, pure and total:

| input | outcome |
|---|---|
| `Ok(answer)` | evaluated; scheduled → on to the VRF leader check |
| `UnknownPool` + populated view | evaluated; not in the set → **real not-elected** |
| `UnknownPool` + **empty** view | authority never established → **cannot answer**, typed refusal |
| `OutsideForecastRange` / `HFC` | could not evaluate → **cannot answer**, typed refusal |

`Err` is the caller's signal to fail closed with a typed refusal naming `pools_in_view` and the error —
never a fallback to the bridge, the seed window or a CLI oracle, and never `ForgeNotLeader`. Extracted
as a pure function rather than left inline so the rule is unit- and mutation-testable. Negative-tested
three ways (restore the collapse; treat an empty view as an answer; route `OutsideForecastRange` to
not-leader) — all caught.

### The original framing of the gap

`forge_skip_reason(None) => None`, and its own doc comment says that case means *"no typed refusal was
recorded and a selected tip WAS available"*. That is the honest state today — but it means a tick that
never reached evaluation for a reason **outside** the seven fences is indistinguishable from one that
reached it. In particular there is **no typed outcome for "leadership authority was not established"**,
which is exactly the CRE-S7 condition that made a follower look healthy while epoch-305 leadership had
never crossed.

E6 forbids forging on unestablished accumulator authority. Enforcing that needs a **typed refusal**, not
an absence:

> add a `ForgeSkipReason` for *leadership authority unavailable / not established for this epoch*, raised
> from the forge path when the epoch's promotion-certified leadership object is missing — fail-closed,
> never a fallback to the bridge, the seed window, or a CLI oracle.

That is the one behaviour change LIVE-2 should need. Everything else is evidence.

## Acceptance criteria

| CE | Criterion | status |
|---|---|---|
| **CE-L2-1** | E1 — warm start from `ade-preprod-s7` under v3 | **MET — LIVE**: store opens, no `StoreSemanticsVersionMismatch`, node reaches `AT PEER TIP` in epoch 305 |
| **CE-L2-2** | E2 — epoch-305 anchor restored + verified; accumulator boundary CROSSED, not stalled | **MET — LIVE**: `recovery_admit action=forward_fold reason=forward_fold_no_reset anchor_before=130314448/5022699/0645f0d3` — a REAL anchor, never `anchor_absent`; store carries CRE-S7's `CROSSED boundary 304 -> 305`, 0 stalls |
| **CE-L2-3** | E3 — full operator key set loads through RED custody; a partial set fails closed rc=44 | **MET — LIVE, both halves**: complete set → `forge CAPABLE — operator keys loaded`, and the loaded `KES VK fingerprint = fd2f1de3…` equals the baseline `kes.vkey`, binding the material to the recorded identity. Partial set (missing `--genesis-file`) → `operator-key ingress failed … rc=44` |
| **CE-L2-4** | E4 — the loop reaches leader evaluation and emits a decided `ForgeOutcome` | **OPEN — BLOCKED**: forge-capable and at tip for ~17 min, but **zero ForgeTicks fired** (0 `forge-decision` emits, 0 forge lines). See the open question below |
| **CE-L2-5** | E5 — every non-forge outcome is typed (`ForgeOutcome` and/or `ForgeSkipReason`) | open |
| **CE-L2-6** | E6 — a typed refusal exists for unestablished leadership authority, and no forbidden fallback is reachable | **CODE HALF MET** — `4985a59b`; live half rides on the E1–E5 run |
| **CE-L2-7** | E7 — restart/replay produce the same authority + forge-decision outputs | open |
| **CE-L2-8** | Negative-tested: each new gate FAILS when its violation is introduced | open |

## OPEN QUESTION blocking E4 — the ForgeTick does not fire

Measured on run 3 (`live2-e1-e2-warmstart-anchor.log`): the node reports
`forge CAPABLE — operator keys loaded`, warm-starts, restores a real anchor, reaches `AT PEER TIP` in
epoch 305, and holds it — and emits **no forge activity whatsoever** (`grep -ci forge` = 0 beyond the
capability line). So the loop never reaches leader evaluation, and E4 cannot be judged.

This is NOT the CE-L2-6 defect (that is fixed on both paths) and NOT an authority problem — E1/E2/E3 are
green. It is the **ForgeTick admissibility gate or the slot-tick scheduling** not producing ticks on the
warm-started `--mode node` path. Deliberately not diagnosed here: it is a fresh thread, and the
session that found it had already corrected its own scoping error once (below).

First step when picked up: instrument or read `plan_loop_step`'s ForgeTick arm against this run — the
node is forge-capable and at tip, so the gate's inputs are the place to look, not the keys or the
authority. `wire_smoke.jsonl` from an earlier run shows 493 `outcome:no_tip_available`, which is the
pre-tick skip, so the sched writer path itself works.

## A scoping error this slice corrected mid-flight — recorded because it nearly shipped

CE-L2-6 was first fixed in `produce_mode.rs::apply_effects_with_forge_handler`. That function is called
**only** from `produce_mode.rs:395` — i.e. `--mode produce`, the DIAGNOSTIC path. The authoritative
`--mode node` forge path is `node_sync.rs:2061`, and it carried the **identical** `Err(_) => ForgeNotLeader`
collapse, with a comment saying it mirrored produce_mode deliberately.

Fixing only the diagnostic path would have left the authoritative one defective while the docs claimed
the defect closed — the worst possible outcome. Both now route through one shared
`classify_leader_schedule`, so they cannot drift apart again. The lesson generalises: *when two paths are
documented as mirroring each other, a fix to one is a half-fix by default.*

## Hard prohibitions

- No silent skip; no generic log-only failure.
- No seed-window fallback, no CLI oracle, no bridge authority on the forge path.
- No forging on unestablished accumulator authority.
- No treating follower success as forge readiness.
- No claiming LIVE-3 (a Haskell-peer-accepted block) from LIVE-2 evidence.

## Reproducer / venue

| | |
|---|---|
| store | `~/.cardano-live1/ade-preprod-s7` — v3, tip in epoch 305, accumulator CROSSED 304→305, 0 stalls |
| keys | `~/Code/rust/ade-ops/preprod/ade-pool/keys/{cold.skey,kes.skey,vrf.skey,node.opcert}` |
| peer | docker `cardano-node-preprod`, `127.0.0.1:3001` |

## Not claimed

Nothing yet. This records the intent, the boundary, the measured starting surface, and the single
behaviour gap (a typed refusal for unestablished leadership authority) before any code.
