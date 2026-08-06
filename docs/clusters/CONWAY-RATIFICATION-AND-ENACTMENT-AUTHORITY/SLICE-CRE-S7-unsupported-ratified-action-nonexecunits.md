# SLICE CRE-S7 — the preprod 305 boundary ratifies an action Ade cannot enact (`NonExecUnitsField`)

> **SEALED — OBSERVED, NOT DIAGNOSED.** Found 2026-08-05, on the first preprod run ever to cross an
> epoch boundary and catch up on the far side. **This is now the top blocker for preprod LIVE-2.**
> Deliberately not investigated at discovery: it is Conway governance enactment authority, the run that
> exposed it had just closed two other slices, and a wrong fix here changes what the ledger enacts.

## What happened

Preprod bootstraps from Mithril, crosses 304→305 with the reference-correct `eta0(305)`, follows to
tip, and warm-restarts cleanly. But the epoch-accumulator's own boundary cross does not complete:

```
epoch-accumulator: reduced checkpoint REWOUND onto boundary point 130118358 before sealing
                   (it sat past it -- DC-EPOCH-32)
epoch-accumulator: boundary cross stalled at 130118424 (observe-only):
  GovernanceEpochTerminal(UnsupportedRatifiedAction {
      action_id: GovActionId {
          tx_hash: e641ec802bb109e150e920c6c0387e85f2efd30944a46d08d08212bde540f69c,
          index: 0 },
      kind: NonExecUnitsField })
```

A real governance action on preprod, ratified at the 304→305 boundary, changes a protocol-parameter
field Ade's Conway enactment does not implement (`NonExecUnitsField`). It is reported **observe-only**,
so it does not halt the node — which is why the follow path looks entirely healthy.

## Why this is the blocker, despite nothing failing closed

The accumulator is the **leadership authority** — the `PoolDistrView` a forge-capable node resolves
leader eligibility from. A stalled boundary cross means epoch 305's leadership authority never
completed, and the observed downstream symptoms line up with that:

- the post-boundary `recovery_admit` ran `reset_and_refold` with `anchor_before=absent` and
  `anchor_after=absent` — the anchor was never established for 305;
- the follow path is unaffected because a follower reads headers, not the accumulator.

So a node in this state **follows correctly and would forge on unproven leadership.** Failing closed
elsewhere is what has been protecting us; here the guard is observe-only.

## Do NOT

- **Do not make the stall fail closed as a first move.** It would convert a working follower into a
  dead one on a real preprod boundary, and the correct behaviour depends on whether the action is
  actually inert for leadership.
- **Do not implement `NonExecUnitsField` enactment from the field name.** The action's real content
  must be read off-chain first (`cardano-cli query gov-state`, or decode the proposal from
  `e641ec80…#0`) so the change is driven by what preprod actually ratified.
- **Do not resume LIVE-2 / forge on this store.** Leadership for 305 is not established.

## First steps when picked up

1. Read the actual action: `e641ec802bb109e150e920c6c0387e85f2efd30944a46d08d08212bde540f69c#0` —
   what parameter, what value, and did cardano-node enact it at 305?
2. Decide the tier: is this a parameter Ade's ledger *uses*? If it is inert for leadership and for the
   fields Ade validates, the honest fix may be explicit, typed, recorded no-op enactment — not silence.
3. Only then choose between enacting it and refusing it, and make the observe-only stall terminal in
   whichever direction the evidence supports.

## Reproducer

| | |
|---|---|
| store | `~/.cardano-live1/ade-preprod-p2` — tip in epoch 305, reproduces on every restart |
| logs | `docs/evidence/run-stores/preprod-nonce-1/ce-p2-5-boundary-and-follow-green.log`, `…/ce-n2-5-warmstart-green.log` |

The stall re-emits on each warm start, so no re-bootstrap is needed to observe it.

## Not claimed

No fix, no root cause, no invariant, no CE. Records a reproducible, named blocker with the exact
`action_id` so the next session starts from the action rather than from the symptom.
