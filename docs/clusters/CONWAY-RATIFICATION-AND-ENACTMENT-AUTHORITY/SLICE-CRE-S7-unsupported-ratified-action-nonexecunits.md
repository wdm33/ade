# SLICE CRE-S7 — classify and handle the Conway ratified action kind `NonExecUnitsField`

> **OPEN — DOC BEFORE IMPL. Step 1 (identify) and step 2 (Cardano reference effect) are DONE and
> measured; the behaviour change is not written.** Found 2026-08-05 on the first preprod run ever to
> cross an epoch boundary and catch up on the far side. **Top blocker for preprod LIVE-2.**

## Claim

> Classify and handle Conway ratified action kind `NonExecUnitsField` without weakening
> epoch-accumulator authority.

## What happened

Preprod bootstraps from Mithril, crosses 304→305 with the reference-correct `eta0(305)`, follows to
tip, and warm-restarts cleanly. The epoch-accumulator's own boundary cross does not complete:

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

Reported **observe-only**, so it does not halt the node — which is why the follow path looks healthy.

## Why this blocks LIVE-2 even though nothing fails closed

The accumulator is the **leadership authority** — the `PoolDistrView` a forge-capable node resolves
leader eligibility from. A stalled cross means epoch 305's leadership authority never completed:

- the post-boundary `recovery_admit` ran `reset_and_refold` with `anchor_before=absent` **and**
  `anchor_after=absent` — the 305 anchor was never established;
- the follow path is unaffected because a follower reads headers, not the accumulator.

So a node in this state **follows correctly and would forge on unproven leadership.** Follower success
is not forge readiness.

## STEP 1 — the action, identified

`cardano-cli conway query gov-state --testnet-magic 1` on the live preprod peer:

```
nextRatifyState.nextEnactState.prevGovActionIds.PParamUpdate.txId
  = e641ec802bb109e150e920c6c0387e85f2efd30944a46d08d08212bde540f69c
futurePParams = NoPParamsUpdate        proposals = []   (it is enacted and retired)
```

So cardano-node **did enact it**, and it is the most recent enacted `PParamUpdate`.

## STEP 2 — the Cardano reference effect, MEASURED

`previousPParams` vs `currentPParams` on the reference node differ in exactly three fields — this is
what the action enacted:

| field | prev | cur | Conway key | in Ade's S4.3c subset? |
|---|---|---|---|---|
| `maxTxExecutionUnits` | mem 16,500,000 / steps 10,000,000,000 | mem **17,500,000** / steps **unchanged** | 20 | **yes** |
| `maxBlockExecutionUnits` | mem 72,000,000 / steps 20,000,000,000 | mem **77,500,000** / steps **unchanged** | 21 | **yes** |
| `minPoolCost` | 170,000,000 | **75,000,000** | 16 | **no → the halt** |

Two observations that shape the fix:

1. **Both exec-units changes are memory-only with steps preserved** — precisely the subset S4.3c
   already implements. Ade is *one parameter* short of enacting preprod's real update.
2. The single unsupported field is `minPoolCost`, which `decode_exec_units_param_update` records in
   `unsupported_fields` (never dropped) and the enactment then refuses on.

## STEP 3 — classification of `minPoolCost` for Ade's authority surface

Measured, not assumed:

| question | answer |
|---|---|
| modelled? | **yes** — `ProtocolParameters::min_pool_cost: Coin` (`pparams.rs:116`), `ProtocolParamUpdate::min_pool_cost: Option<Coin>` (`:248`), with a working apply path (`:571`) |
| read by any ledger rule? | **NO** — no reward, POOLREAP, cert-validation or epoch rule in `ade_ledger` reads it. The only non-test readers are the fingerprint, the snapshot codec, and the ledgerdb/CLI importers |
| fingerprinted? | **YES** — `fingerprint.rs:497` writes it into the protocol-params fingerprint |
| durably persisted? | **YES** — `snapshot/gov_state.rs:72/117` |

This lands **between** the two obvious buckets and that is the whole point of the slice:

> `minPoolCost` is **inert in COMPUTATION** (no rule consumes it today) but **LIVE in the FINGERPRINT**
> (durable authority state binds it).

So "safely irrelevant to the current reduced follower authority surface" is **false**. Skipping it
would leave Ade's `currentPParams.minPoolCost` permanently at 170,000,000 while cardano-node says
75,000,000, and that disagreement is fingerprinted into durable state — a divergence that surfaces
later as an opaque fingerprint mismatch, which is exactly the P4 failure shape.

### Recommended disposition — ENACT, do not record inert

`EnactedEffectful` in the taxonomy below. Reasons, in order of force:

1. **Fingerprint divergence is a real effect**, even with no rule reading the value. "No rule reads it"
   is a statement about today's rule set; the fingerprint is a statement about durable identity.
2. The field is **already modelled with a working apply path** — this is widening the enactment subset
   by one decoded key, not new ledger semantics.
3. The reference value is **known and measurable** (75,000,000), so the enactment can be proven against
   cardano-node rather than asserted.
4. Recording it inert would be indistinguishable, six months from now, from the "broad unsupported
   governance ignored" this slice forbids.

## Tier classification

| tier | statement |
|---|---|
| **true** | Leadership authority may not advance past an unprocessed authoritative epoch transition. |
| **derived** | Cardano Conway ratified governance actions must be classified and enacted or rejected according to their ledger effect on epoch state. |
| **release** | Preprod LIVE-2 blocked until the 304→305 accumulator anchor is established. |
| **operational** | None. Do not work around with forge-off / operator judgement. |

## Acceptance criteria

| CE | Criterion | status |
|---|---|---|
| **CE-S7-1** | The ratified action is identified: id, kind, enacted epoch/slot, affected fields | **MET** — `e641ec80…#0`, enacted at the 304→305 boundary, three fields above |
| **CE-S7-2** | Cardano reference effect determined: ledger validation / Praos-leadership / accumulator-reward-governance / inert-for-reduced-authority | **MET** — measured: no rule reads `minPoolCost`, but it is fingerprinted + persisted, so NOT inert for durable authority |
| **CE-S7-3** | Typed classification added (`EnactedEffectful` / `EnactedInertForReducedAuthority` / `UnsupportedEffectful` / `Malformed`) — no free-form string, no bool | open |
| **CE-S7-4** | If effectful: the authoritative state transition is implemented and compared against the Cardano reference (`minPoolCost` 170,000,000 → **75,000,000**; exec-units memory 16.5M→17.5M and 72M→77.5M with steps preserved) | open |
| **CE-S7-5** | If inert: durable evidence recorded that it is inert for this authority surface — never a silent skip | n/a under the recommended disposition; the criterion stays so a future inert case cannot skip silently |
| **CE-S7-6** | The epoch-305 accumulator anchor ESTABLISHES on the reproducer store, and the observe-only stall is gone | open |
| **CE-S7-7** | Negative-tested: each new gate proven to FAIL when its violation is introduced | open |
| **CE-S7-8** | LIVE-2 may resume only after CE-S7-6 | open |

## Hard prohibitions

- No silent skip.
- No forge on an unestablished accumulator anchor.
- No broad "unsupported governance ignored".
- No fail-open.
- No hardcoded `action_id` exception.
- No treating follower success as forge readiness.
- **Do not make the observe-only stall terminal as a FIRST move.** It would convert a working follower
  into a dead one on a real preprod boundary before the classification is settled. Terminal is right
  *after* CE-S7-3 exists, in whichever direction the classification supports.

## Instrumentation gap this exposed

`UnsupportedRatifiedAction { kind: NonExecUnitsField }` names the CLASS but not the OPERAND — it does
not say WHICH field was unsupported, even though `decode_exec_units_param_update` has already collected
exactly that in `unsupported_fields` (`governance.rs:751`) and deliberately never drops it. Identifying
`minPoolCost` required an off-chain `gov-state` query and a pparams diff. Carrying the field set into
the error is nearly free and is the same "name the operand, not just the class" fix that
`nonce1-boundary-operands` applied to DC-EPOCH-16.

## Reproducer

| | |
|---|---|
| store | `~/.cardano-live1/ade-preprod-p2` — tip in epoch 305; the stall re-emits on EVERY warm start (7 occurrences so far), no re-bootstrap needed |
| logs | `docs/evidence/run-stores/preprod-nonce-1/ce-p2-5-boundary-and-follow-green.log`, `…/ce-n2-5-warmstart-green.log` |
| reference | `cardano-cli conway query gov-state --testnet-magic 1` against `cardano-node-preprod` |

## Not claimed

No behaviour change, no invariant, no CE beyond S7-1/S7-2. The classification is measured and the
disposition is recommended with its reasons; the enactment itself is not written.
