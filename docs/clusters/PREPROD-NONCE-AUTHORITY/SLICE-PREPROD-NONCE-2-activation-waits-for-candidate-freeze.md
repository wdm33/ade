# SLICE PREPROD-NONCE-2 — epoch activation waits for candidate-freeze finality (DC-EPOCH-16)

> **OPEN — SEALED SLICE, NOT YET IMPLEMENTED.** Doc before impl. Fix for the PREPROD-NONCE-1 root
> cause. Blocks preprod LIVE-2 until closed.

## Intent — NARROWED after the reference proof

Make the **bootstrap bridge's** durable epoch activation agree with the same Praos nonce value
cardano-node will use at the real boundary.

The governing invariant is unchanged — *no durable epoch activation record may be written until every
consensus input inside it is final; for `eta0(N+1)` the candidate nonce must already be frozen* — but
the **fix target is now bridge-specific**, not activation timing in general.

```
KNOWN-GOOD (reference-proven, DO NOT TOUCH):
    the live boundary tick computes eta0(305) BYTE-IDENTICAL to cardano-node
KNOWN-BAD:
    the bootstrap bridge committed eta0(305) early, from candidate@SEED
FIX BOUNDARY:
    alter bridge commitment TIMING / PROVENANCE only
```

### Explicitly OUT OF SCOPE — these are proven correct, changing them is a regression

- the candidate-freeze rule and freeze-slot arithmetic
- the candidate / evolving operand order (`extract_praos_nonces_v2`)
- the Praos boundary combine `epoch_nonce' = candidate ⭒ last_epoch_block_nonce`
- the live tick logic in `node_sync`
- venue `k` / `f` / RSW geometry

A change to any of the above would be "fixing" something the reference proof shows is already right.

## METHODOLOGICAL RULE for this slice (first-class, not advice)

> **ECA-5 showed this exact code class had an operand-order bug that stayed masked until a real
> boundary was crossed. Therefore this slice MUST verify against cardano-node reference values BEFORE
> changing any nonce logic.**

`26565bec` records the precedent verbatim: *"`extract_praos_nonces_v2` had evolving and candidate
swapped … proven by value against the live node's epoch nonce. The original deadlock had always masked
this (no real boundary had ever been crossed)."* Preprod 304→305 is the first real preprod boundary
ever attempted, so the identical masking condition applied here. Internal self-consistency is NOT
sufficient evidence in this code; a reference value is.

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

| CE | Criterion | status |
|---|---|---|
| **CE-N2-1** | cardano-node preprod `epochNonce(305)` == Ade's boundary-tick `eta0(305)` | **MET** — both `74f10bea…` |
| **CE-N2-2** | cardano-node `lastEpochBlockNonce(305)` == Ade's post-tick bookkeeping value | **MET** — both `60b3a0ae…` (see the precision note below) |
| **CE-N2-3** | The bridge no longer commits `e3402a2b…` | open |
| **CE-N2-4** | The bridge commitment resolves to `74f10bea…` | open |
| **CE-N2-5** | Restart after the bootstrap bridge recovers the SAME value | open |
| **CE-N2-6** | `ActivationAboveDurableTip` remains terminal | open |
| **CE-N2-7** | Seed BEFORE freeze: no final activation record carries `candidate@SEED`; at/after freeze it carries `candidate@FROZEN` | open |
| **CE-N2-8** | Multi-venue differential covers seed-position × freeze geometry (seed before freeze, seed after freeze, boundary at/around the freeze) — **DC-EPOCH-38** | open |
| **CE-N2-9** | Negative-tested: each gate proven to FAIL when its violation is introduced | open |

### Precision note on CE-N2-2 — do NOT read this as validating the combine operand

Getting this backwards is the ECA-5 mistake in miniature, so state it exactly:

| value | which one | matches reference? |
|---|---|---|
| cardano `lastEpochBlockNonce = 60b3a0ae…` | **post**-tick bookkeeping | == Ade's **pre**-tick `lab`, which becomes `last_epoch_block_nonce'` ✓ |
| Ade `last_epoch_block_nonce = 151dc584…` | the **combine operand** at the tick | validated only INDIRECTLY — the combine OUTPUT matches |

So CE-N2-2 confirms the bookkeeping rotation, **not** the operand. The operand is confirmed by
CE-N2-1 (the output matching). Anyone reading CE-N2-2 as "the operand is wrong, `151dc584` should be
`60b3a0ae`" would break a correct combine.

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

## STATE 2026-08-05 — the dangerous behaviour is BLOCKED; the durable completion remains

```
before:  the bridge could write a wrong FINAL eta0 into a durable activation record
now:     the bridge REFUSES before the WAL commit-point when eta0 is not final
```

So this is no longer an emergency guard. What remains is the durable completion: making the
node write the RIGHT value, safely and replay-equivalently.

| landed | |
|---|---|
| `81df0bac` | CE-N2-3 — `BridgeNonceNotFinal` fails closed before the WAL commit-point. Proven live on preprod: `{ target_epoch: 305, seed_slot: 129813427, candidate_freeze_slot: 129945600 }`, rc=41, no bootable partial state |
| `0481db1b` | CE-N2-4 step 1 — the typed `BridgeEta0Finality` decision, single-sourced between the FirstRun builder and (next) the boundary binder |

### NEXT SESSION — start here, in this order (binding)

1. **Read the history first**: ECA-5 (`26565bec`) and the bridge-nonce lineage. The methodological
   rule above is not optional — this code class had an operand-order bug that stayed masked until a
   real boundary crossed.
2. **Re-read `bind_bridge_view`** (`epoch_wire.rs:570`) and both callers: live promotion (`:769`) and
   warm-start recovery (`:941`).
3. **Implement final nonce authority**: the boundary-tick `eta0` is AUTHORITATIVE; the bridge-stored
   nonce is a CROSS-CHECK only when `BridgeEta0Finality::Final`.
4. **Prove live path == recovery path** (byte-identical binding — the constraint `bind_bridge_view`'s
   doc states, and the reason it is the sole binder).
5. **Prove preprod eta0(305) resolves to `74f10bea…`** (CE-N2-4).
6. **Prove `e3402a2b…` cannot reach the WAL** (CE-N2-3 regression + CE-N2-9 negative test).
7. **Keep `ActivationAboveDurableTip` terminal** (CE-N2-6).

**No LIVE-2 until that is green.**

### Feasibility already verified (do not re-derive)

- `bind_bridge_view` is the SOLE binding path, so one edit covers both callers.
- The live caller has `chain_dep` in scope and can tick exactly as the frozen-leadership path does
  (`epoch_wire.rs:703`).
- Recovery already receives `RecoveredEpochNonce { epoch, eta0 }` independently of the bridge.
- **No durable schema change is required**: the bridge carries `source_point_slot`, and the caller has
  the `era_schedule` geometry, so finality is derivable at bind time.

### Explicitly NOT to be done

- No post-freeze-snapshot watcher. It is a legitimate temporary operational unblock, but it adds
  operational noise and tempts the reader into treating timing luck as progress. The durable path is
  already clear.

## Not claimed

No fix yet, no invariant registered. This records the approved direction, the corrected mechanism, and
the acceptance criteria the fix must meet.
