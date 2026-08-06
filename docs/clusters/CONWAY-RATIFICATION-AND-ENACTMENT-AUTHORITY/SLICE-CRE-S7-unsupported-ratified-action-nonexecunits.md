# SLICE CRE-S7 — classify and handle the Conway ratified action kind `NonExecUnitsField`

> **ALL EIGHT CEs MET — LIVE-PROVEN 2026-08-06.** `minPoolCost` (Conway key 16) joins the enactable
> subset; the preprod action `e641ec80…#0` now ENACTS to cardano-node's `currentPParams`, and the
> epoch-305 accumulator anchor ESTABLISHES:
> `epoch-accumulator: CROSSED boundary 304 -> 305 at slot 130118424` — 0 stalls, where the pre-fix
> store stalled on all 7 attempts. Classified **`EnactedEffectful`**, not inert.
>
> `STORE_SEMANTICS_VERSION` bumped **2 → 3**: enacting a fingerprinted parameter changes what a replay
> produces, so pre-fix stores are correctly refused — including this slice's own former reproducer.

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
| **CE-S7-3** | Typed classification: the disposition is carried by the closed `UnsupportedActionKind` + the decoder's `EnactableParamUpdate` (a key is either decoded into a typed field or recorded in `unsupported_fields` — never a bool, never a string) | **MET** — `minPoolCost` classified `EnactedEffectful`, i.e. moved into the enactable subset |
| **CE-S7-4** | Effectful: the state transition is implemented and compared against the Cardano reference (`minPoolCost` 170,000,000 → **75,000,000**; exec-units memory 16.5M→17.5M and 72M→77.5M, steps preserved) | **MET** — `cre_s7_preprod_e641ec80_enacts_to_the_cardano_reference` (decoder + `build_enacted_pparams`) and `cre_s7_preprod_action_enacts_through_the_boundary_path` (full enactment path) |
| **CE-S7-5** | If inert: durable evidence recorded that it is inert — never a silent skip | **n/a** — classified effectful. The criterion STAYS so a future genuinely-inert case cannot skip silently |
| **CE-S7-6** | The epoch-305 accumulator anchor ESTABLISHES, and the observe-only stall is gone | **MET — LIVE**: `epoch-accumulator: CROSSED boundary 304 -> 305 at slot 130118424 (mark from s_prev 130118358)`, 0 stalls (was 7), node at tip in 305, 0 halts |
| **CE-S7-7** | Negative-tested: each new gate proven to FAIL when its violation is introduced | **MET** — six mutations; one initially slipped (see below) |
| **CE-S7-8** | LIVE-2 may resume only after CE-S7-6 | **UNBLOCKED by this slice** — CE-S7-6 is green. LIVE-2 resumption is its own decision |

## What shipped

| | |
|---|---|
| decoder | `PPU_KEY_MIN_POOL_COST = 16` decoded as a bare `Coin` uint into `EnactableParamUpdate::min_pool_cost`; a duplicate is `DuplicateKey`, a non-uint is the new `MalformedCoin` |
| enactment | `build_enacted_pparams` applies it — the field already had a working apply path, so this widened the decoded subset rather than adding ledger semantics |
| naming | `ExecUnitsParamUpdate`→`EnactableParamUpdate`, `decode_exec_units_param_update`→`decode_enactable_param_update`, `NonExecUnitsField`→`UnsupportedPParamField`, `NoExecUnitsField`→`NoEnactableField`. The subset is `{16, 20, 21}`; "exec-units" had become actively wrong and would have sent the next reader to the wrong place |
| diagnostic | `UnsupportedRatifiedAction` gains `unsupported_keys: CanonicalFieldSet`, and `CanonicalFieldSet` has a hand-written `Debug` rendering `minPoolCost(16)` — so **every pre-existing `{:?}` emit gains the operand without being touched** |
| semantics | `STORE_SEMANTICS_VERSION` 2 → 3 (bump, not neutral): enacting a *fingerprinted* parameter changes what a replay produces |

### The diagnostic, before and after

```
before:  UnsupportedRatifiedAction { action_id: …, kind: NonExecUnitsField }
after:   UnsupportedRatifiedAction { action_id: …, kind: UnsupportedPParamField,
                                     unsupported_keys: [minPoolCost(16)] }
```

Identifying the blocking field previously required an off-chain `gov-state` query plus a `previousPParams`
/ `currentPParams` diff. It is now in the halt line.

### CE-S7-7 — one mutation initially slipped

Six mutations: key 16 decoded but not enacted; key 16 returned to `unsupported_fields`; the terminal
stops carrying the keys; the renderer drops names; the preprod action refused at the boundary path;
`minPoolCost` enacted with a wrong value. **The "terminal stops carrying the keys" mutation initially
PASSED** — the tests asserted the decoder's `unsupported_fields` and the renderer, but nothing asserted
the operand on the error the boundary actually returns. `cre_s4_3c_foreign_pparam_field_is_unsupported`
now asserts it on the returned terminal, and catches it. Same shape as the CE-N2-9 miss: *a check that
verifies a value near the thing it cares about, rather than the thing itself.*

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
