# SLICE P2 — preprod reaches tip, then fails closed capturing a REDUCED state

> **SEALED — OBSERVED, NOT DIAGNOSED.** Found 2026-08-03 03:32Z on the first successful preprod
> bootstrap (after P1). Blocks LIVE-2 on preprod. Deliberately not investigated further at the time
> of discovery: it is consensus-adjacent and the session was ~7 h deep with a leader slot 10 minutes
> away — precisely the conditions under which a wrong fix gets written.

## What happened

P1 fixed, chunk-6009 snapshot bootstrapped cleanly, and the node caught up **without a single
error**:

```
follow: tip slot=129971821 behind=72892 slots from peer
follow: tip slot=129982521 behind=62192
... (monotonic, no faults) ...
follow: tip slot=130040378 behind=4335
follow: tip slot=130044713 -- AT PEER TIP (caught up, following live)
ade_node --mode node: relay run-loop sync step failed
    (Capture("Encode(ReducedStateNotSerializable)")); failing closed (no skip-past, no fallback).
                                                                                  exit 43
```

The failure is the **first capture attempt on reaching tip** — catch-up itself was clean.

## The error is BY DESIGN, so the bug is upstream of it

`encode_ledger_state` (`snapshot/ledger.rs:69,73`) refuses to serialise a state whose `cert_state` or
`gov_state` is `ReducedUnavailable`:

> *"A reduced follower's cert/gov is `ReducedUnavailable` — NOT serializable as a normal
> full-authority snapshot (fail closed rather than fabricate a normal CertState/gov; RVBP)."*

Pinned by `reduced_continuation_is_not_serializable_as_authority`. So the guard is correct and must
NOT be relaxed; the question is **why the preprod state is reduced at all**.

## Evidence

| | preprod | preview |
|---|---|---|
| occurrences of `ReducedStateNotSerializable` | 1, immediately at tip | **0** across hours (both `ade-rf1-ce5.log` and the baseline) |
| epoch boundary crossed during catch-up? | **no** — 129,813,427 → 130,044,713, all inside epoch 304 (129,686,400 – 130,118,400) | n/a |
| catch-up faults | none | n/a |

That the node never crossed a boundary matters: the reduced projection is associated with the
reduced-follower / window-replay path (`reduced_window_driver.rs:49,98`), and a boundary crossing is
the obvious way to enter it. **Neither applies here**, so the state appears to be reduced from
bootstrap — yet `seed_from_bootstrap_ledger` is documented to *reject* a `ReducedUnavailable` seed
(`mithril_native_assembly.rs:367`). Those two facts are in tension and that tension is the
investigation.

## Candidate directions (NONE selected — this is the open work)

1. The native Mithril bootstrap yields a full seed but the **live follow** transitions the state to a
   reduced projection before the first capture.
2. Something in the preprod state makes a cert/gov sub-projection unavailable that preview's never
   exercises — the P1 pattern again (*an empty collection on one venue hides its behaviour*).
3. The capture cadence differs between venues, so preview simply has not attempted the same capture.
   **Check this first** — it is the cheapest to falsify and would reframe the whole finding.

Explicitly NOT acceptable as a fix: relaxing the RVBP guard, or fabricating a full CertState/gov to
make the encode succeed. The guard exists so that nothing a reduced follower produced can be
rehydrated or fingerprinted as authority.

## Impact

- **LIVE-2 on preprod is blocked**, one layer deeper than P1.
- Epoch-304 leader slot 130045510 (03:45:10Z) was **missed** as a direct consequence. Remaining:
  130074818 (11:53Z) and 130081569 (13:46Z).
- Every operator prerequisite remains verified and unaffected: opcert valid to 2026-09-15 with
  KES vkey `fd2f1de3…` **proven equal** to the loaded key, stake
  `stakeGo == stakeSet == stakeMark == 1,009,506,139,807`, peer synced, bootstrap succeeded.
- Preview is unaffected and still running.

## NEW EVIDENCE 2026-08-05 (from the PREPROD-NONCE-2 CE-N2-4 run) — direction 3 is FALSIFIED

The CE-N2-4 live run bootstrapped preprod from the same snapshot 6009 and followed 129,813,427 →
130,118,424, this time CROSSING the 304→305 boundary. It hit the same error, but the surrounding facts
differ from the 08-03 observation in a way that answers P2's cheapest open question:

```
follow: ... 129,813,427 -> 130,118,358   (all of epoch 304)   -- ~152 capture opportunities, ALL SILENT
nonce1-boundary-operands: from_epoch=304 to_epoch=305 ... match=true   -- boundary crossed CORRECTLY
ade_node: relay run-loop sync step failed (Capture("Encode(ReducedStateNotSerializable)"))   rc=43
```

| question | answer from this run |
|---|---|
| **Direction 3** — "does preview simply not attempt the same capture?" (marked *check this first*) | **FALSIFIED for preprod.** The E4 cadence capture (`RECOVERY_CHECKPOINT_CADENCE_SLOTS = 2000`) runs every sync step. Over ~305k slots of epoch-304 catch-up that is ~152 attempts, and **every one succeeded**: zero `capture-refused` emits, zero errors. Preprod captures fine; it is not a cadence artifact. |
| **Direction 1/2** — is the state reduced *from bootstrap*? | **No, not in this run.** A state reduced from bootstrap would have failed the first cadence capture ~2000 slots in. It captured cleanly through the whole seed epoch and failed only *after* the boundary. |
| which call site fails | **The B3b yield-at-boundary capture** (`node_sync.rs:894`), not the cadence one. The boundary path `return`s early with `BoundaryPromoted`, so the E4 block is skipped and the unconditional boundary capture runs instead. |

So in this run the reduced projection appears **at/through the epoch-boundary crossing**, not at
bootstrap — which resolves the tension this slice recorded between "the state appears to be reduced
from bootstrap" and `seed_from_bootstrap_ledger` rejecting a `ReducedUnavailable` seed. The seed was
full, and it stayed full for an entire epoch of live follow.

**Held deliberately narrow.** The 08-03 symptom (failure at tip, *inside* epoch 304, no boundary) is
NOT the same as this one (failure at the boundary, after ~152 clean captures). Several clusters landed
between the two observations, so this run does not prove the original symptom still reproduces — only
that the boundary-triggered variant does, and that direction 3 does not explain it.

### Instrumentation gap this exposes

P2's own emit-only diagnostic (`capture-refused:`, added at `node_sync.rs:966`) is attached to the
**cadence** capture site only. The **boundary** capture site at `node_sync.rs:894` has no such emit, so
the failure that actually fires arrives as a bare `Capture("Encode(ReducedStateNotSerializable)")` with
none of the projection detail the emit was written to supply. Extending the emit to that site is the
cheapest next step and costs nothing behaviourally.

## Not claimed

No fix, no root cause, no invariant, no CE. This slice records a reproducible failure with its
evidence so it is not re-discovered, and records that the P1 fix genuinely worked — preprod
bootstrapped and reached tip, which had never happened before.
