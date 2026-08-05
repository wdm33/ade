# SLICE PREPROD-NONCE-2 — epoch activation waits for candidate-freeze finality (DC-EPOCH-16)

> **OPEN — SEALED SLICE, NOT YET IMPLEMENTED.** Doc before impl. Fix for the PREPROD-NONCE-1 root
> cause. Blocks preprod LIVE-2 until closed.

## Intent

**No durable epoch activation record may be written until every consensus input inside that record is
final.** For `eta0(N+1)` that means the candidate nonce must already be frozen.

```
before the freeze slot :  leadership may be known/previewed,
                          but NO durable activation record with an eta0 commitment
at/after the freeze slot: candidate frozen -> eta0 final -> activation record may be written
```

## The mechanism — CORRECTED from the NONCE-1 first reading

NONCE-1 proved the arithmetic beyond doubt:

```
last_epoch_block_nonce = 151dc584…                       <- IDENTICAL on both sides
blake2b256(candidate@SEED   ‖ 151dc584…) = e3402a2b…     == the committed value
blake2b256(candidate@FROZEN ‖ 151dc584…) = 74f10bea…     == the boundary tick
```

It also stated the commitment was *"promoted at the seed point"*, reading the activation record's
`transition_point = 129813427`. **That reading was wrong.** `activation_record_for` sets
`transition_point: view.source_point`, and `source_point` is the FROZEN LEADERSHIP object's MARK
source (`s_prev`) — not the slot at which promotion ran. The field cannot support that claim.

The actual writer is the **bootstrap bridge**. `target_epoch 305 == seed_epoch 304 + 1`, so this is the
seed+1 BRIDGE path (DC-EPOCH-15), where `BootstrapNextEpochAuthority.epoch_nonce` supplies eta0. Its
own doc comment states:

> *"The seed+1 leadership nonce (eta0) — **the candidate nonce frozen for the next epoch**."*

That is true only when the seed sits **at or after** the freeze slot. It is built at IMPORT from the
Mithril seed state, and on snapshot 6009 the seed precedes the freeze by 132,173 slots — so the bridge
binds a candidate that is still tracking evolving and guaranteed to move.

**The premature commitment is written at bootstrap/import, not by promotion.** The fix therefore
belongs at the bridge/seed-authority boundary, not in the promotion path.

## REFERENCE-PROVEN against cardano-node — CE-N2-3's reference half is already satisfied

Ade's internal arithmetic being self-consistent does not say WHICH value Cardano considers correct.
ECA-5 is the precedent for why that matters: it found `extract_praos_nonces_v2` had **evolving and
candidate swapped**, caught *"by value against the live node's epoch nonce"*, and notes the original
deadlock *"had always masked this (no real boundary had ever been crossed)"*. Same shape as here, so
the assumption was checked rather than trusted:

```
cardano-node preprod, epoch 305 (query protocol-state):
  epochNonce          = 74f10bea2b467cac73efbd02b36307fe12a123b098a94cfcfe4c33ce4ef10b62
  lastEpochBlockNonce = 60b3a0aea44e3977baa949c27c5053c984001dc858048e4d296ddebcf8b0dc67
```

| value | source | verdict |
|---|---|---|
| `74f10bea…` | Ade's boundary tick from `candidate@FROZEN` | **matches cardano-node byte-for-byte** |
| `e3402a2b…` | the bootstrap bridge's committed eta0 | **wrong** |

The post-tick bookkeeping agrees independently: Ade's pre-tick `lab = 60b3a0ae…` becomes
`last_epoch_block_nonce'`, which is exactly what cardano now reports.

**So Ade's Praos boundary combine is CORRECT against the Haskell reference.** The defect is isolated
to WHEN the bridge takes its commitment — nothing about the tick, the freeze, or the operand order
needs to change. This bounds the fix tightly and rules out an ECA-5-style swapped-operand repeat.

## Why not a mutable commitment

A durable activation record must be a **sealed fact, not a promise to fix the fact later**. A record
that exists with one non-final field creates exactly the authority ambiguity this project has been
eliminating: the record exists, a later rewrite changes its meaning, and restart timing decides which
version is observed. That is a replay hazard, and it is why "promote early, then re-derive" is
rejected as the authority model.

## Shape: split the concepts, do not mutate the record

If earlier forge/leadership visibility is needed, add a typed distinction rather than a mutable record:

| | `PendingEpochAuthority` | `ActiveEpochView` |
|---|---|---|
| leadership / frozen leadership | available | available |
| eta0 commitment | **none** | final |
| durable activation record | **forbidden** | allowed |
| restart-authoritative | **no** | yes |
| usable as promoted eview | **no** | yes |

Pending must not be able to masquerade as final. The type system should make "a durable record with a
non-final eta0" unrepresentable, in the spirit of the closed `RemediationAction` in P6.

## Tier classification

| tier | statement |
|---|---|
| **true** | Durable authority records may only contain finalized deterministic inputs. |
| **derived** | Cardano Praos `eta0(N+1)` must use the candidate nonce at the candidate-freeze point, never at the Mithril seed point. |
| **release** | The preprod 304→305 eta0 differential must pass before LIVE-2 resumes. |
| **operational** | None. Operators must not work around this. |

## Acceptance criteria

| CE | Criterion |
|---|---|
| **CE-N2-1** | Seed BEFORE freeze: no final activation record is written carrying `candidate@SEED` |
| **CE-N2-2** | At/after freeze: the activation record uses `candidate@FROZEN` |
| **CE-N2-3** | preprod `eta0(305)` equals `blake2b256(candidate@FROZEN ‖ last_epoch_block_nonce)` and matches the reference |
| **CE-N2-4** | Restart BEFORE freeze does not recover a premature activation |
| **CE-N2-5** | Restart AFTER freeze recovers the final activation and matches |
| **CE-N2-6** | `ActivationAboveDurableTip` remains terminal |
| **CE-N2-7** | Multi-venue differential covers seed-position × freeze geometry: seed before freeze, seed after freeze, and boundary at/around the freeze |
| **CE-N2-8** | Negative-tested: each gate proven to FAIL when its violation is introduced |

CE-N2-7 is **DC-EPOCH-38**, opened with this slice's fix so the gate encodes what was proven. NONCE-1
established the surface is **seed-position**, not venue: the freeze sits at `1 − RSW/epoch_length`
into the epoch and that ratio is identical across venues (0.4), so a once-per-venue differential would
pass straight through this defect.

## Hard prohibitions

- No patching eta0 from a reference value.
- No preprod special case; no venue special case.
- No mutable final activation record.
- No weakening `ActivationAboveDurableTip`.
- No fallback to the seed candidate after the freeze.

## Open design question for implementation

The bridge is built at import, but the freeze slot for seed+1 may lie far ahead of the seed (132,173
slots here). So "wait for the freeze" means the seed+1 authority cannot be fully sealed at import at
all when the seed precedes the freeze. Two shapes to weigh at implementation time:

1. **Import emits `PendingEpochAuthority` only**, and the seed+1 `ActiveEpochView` is sealed later, at
   the freeze slot, from the live fold.
2. **Import refuses to bind seed+1 eta0** when the seed precedes the freeze, and the boundary path
   derives it, with the bridge carrying leadership only.

Both satisfy the invariant; they differ in how much the bootstrap receipt can promise. This is
resolved with evidence during implementation, not guessed here.

## Not claimed

No fix yet, no invariant registered. This records the approved direction, the corrected mechanism, and
the acceptance criteria the fix must meet.
