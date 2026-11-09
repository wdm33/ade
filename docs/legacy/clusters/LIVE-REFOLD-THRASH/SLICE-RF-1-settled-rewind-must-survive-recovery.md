# SLICE RF-1 — a bounded settled rewind must survive the recovery pass that follows it

> **OPENED 2026-08-02 from the EVIEW-R2 live run.** Investigation complete; implementation is held
> until R2 closes (CE-R2-5). EVIEW-R2 closed the correctness half of the same causal chain — a
> refold is now harmless, but it is still ruinously expensive and self-reinforcing.
>
> **SEVERITY RAISED 2026-08-02 23:41Z — this is a FORGE BLOCKER, demonstrated, not a performance
> defect.** On the CE-R2-5 store the refold outgrew the inter-rollback interval and the node stopped
> converging entirely:
>
> | | |
> |---|---|
> | refold distance | 165,210 → 169,209 → **171,449** slots, monotonic |
> | refold duration | ~30 min each (~96 slots/s) |
> | last rollback interval | ~20 min — **shorter than one refold** |
> | rollbacks resolved via `reset_to_settled` | **0 of 3** — the settled point never exists to try |
> | time holding tip after 23:00Z | **none**, 40+ min of continuous refolding |
>
> It was still mid-refold when the live 1377→1378 boundary arrived, so it could **not promote
> leadership at that boundary**. That is verbatim the failure `DC-EPOCH-30` claims to prevent:
> *"a refold that outgrows the inter-reorg interval means leadership can never be promoted at a
> boundary, i.e. the node cannot forge."* Measured on the shipped binary.
>
> Consequence for sequencing: RF-1 is a **precondition for any preprod forge attempt**, not a
> follow-up tidy-up. A node that cannot hold the tip cannot forge regardless of stake.

## The finding — the bounded rewind is applied, then discarded one pass later

The working hypothesis was that the settled rewind point is never *available*: rollback → not
admissible → full refold → node never sits at tip for `k` blocks → still not admissible → loop.

**That is not what happens.** The live trace shows the settled rewind being applied *successfully*
and then thrown away:

```
18:38:29.823Z rollback_admit  action=reset_to_settled   reason=rollback_admission
                              anchor_before=119039873/4534207/e98682f7   anchor_after=absent
18:39:50.958Z recovery_admit  action=reset_and_refold   reason=anchor_absent
                              anchor_before=absent      anchor_after=absent
18:41:01.654Z REFOLD re-crossed boundary 1375 -> 1376 ... 153565 slots left to refold
```

A bounded rewind to a settled point in **epoch 1377**, and 90 seconds later a full refold from the
bootstrap anchor in **epoch 1375**. The mechanism, confirmed in code:

1. `accumulator_admit_and_clear_for_rollback` admits the rollback and calls `reset_to_settled`,
   which restores the accumulator to the settled point — and clears `LAST_ADVANCED_POINT`.
2. The next `advance_ledger_state_to_durable_tip` pass runs `accumulator_recover_admit`, reads an
   absent anchor, and `reconcile_recovery` returns `ResetAndRefold { AnchorAbsent }`.
3. That handler calls `reset_to_bootstrap()`, which discards the bounded rewind **and deletes
   `SETTLED_{BLOB,POINT,LEADERSHIP}_KEY`** — so the next rollback has no bounded target either.

`ResetReason::AnchorAbsent`'s own doc names the trap: *"the transitional state a PREVIOUS reset left
behind (**both reset paths** clear `LastAdvancedPoint`)"*.

**Consequence: ACCUMULATOR-REFOLD-BOUND S1 is effectively inert in production.** DC-EPOCH-26..31 are
correctly enforced as *unit* properties — the settled point is staged, promoted at `k`, bounded to
`2k`, and replay-equivalent — but the production wiring discards the result before it can be used.
CE-AR-6 ("a live run showing refold no longer grows with uptime") was left OUTSTANDING for exactly
this reason and should now be read as **failed, not merely unobserved**.

The two failures compound into the self-reinforcing loop:

```
rollback -> reset_to_settled (bounded, correct) -> anchor cleared
         -> next pass: anchor absent -> reset_to_bootstrap
         -> settled point DELETED + full refold from bootstrap (~27 min observed)
         -> node is not at tip during the refold
         -> next rollback arrives before it has held tip for k=432 blocks
         -> no settled point exists -> straight to bootstrap again
```

Both arms were observed live inside 40 minutes: `reset_to_settled` at 18:38:29 (settled point
existed) and `reset_to_bootstrap` at 19:15:50 (it no longer did).

## The constraint that makes this non-trivial

The anchor clear is **not a bug**. It is S5's deliberate pre-clear:

> *durably CLEAR the lineage anchor BEFORE the caller commits the ChainDB rollback, so a crash in
> the window leaves an anchor-absent (uncertified) store that the next advance refolds from
> canonical.*

At the moment of the rewind the ChainDB rollback has **not yet committed**. An anchor written there
would claim lineage on a chain that is about to change, and a crash inside that window would leave a
certified-but-wrong store. That property must survive this slice untouched.

So the anchor cannot simply be kept. It must be **re-established after the rollback is durable, and
re-proved against the post-rollback canonical chain.**

## Candidate invariant

**INV-RF-1 (draft).** When recovery finds no lineage anchor, it must prefer a **re-certified settled
point** over the bootstrap baseline — but only when that point independently re-proves, against the
**post-rollback** canonical chain, every condition `settled_rewind_admissible` requires:

1. `k` BLOCKS of separation from the current durable tip (block units, no ASC assumption),
2. its header hash still resolves canonically at its slot on the chain as it now stands,
3. the accumulator state restored from `SETTLED_BLOB` is the state at exactly that point.

If any condition fails, or any read faults, it falls back to `reset_to_bootstrap` — the current
behaviour, unchanged. **The fallback is always safe, so this can only ever cost refold time.**

Corollary **INV-RF-2**: `reset_to_bootstrap` must not delete a settled point that is still canonical
and still `k`-settled, since deleting it converts one expensive refold into an unbounded series.
(Ordering-sensitive: the re-certification attempt has to happen *before* any wipe.)

## Hard line

Carried verbatim from the direction on this slice:

> Use settled rewind only when its lineage and k-bound proof are mechanically valid. Otherwise fail
> closed or full-refold safely.

Specifically **NOT** acceptable as a fix: keeping the anchor through the rollback window; trusting
the settled point without re-proving lineage on the post-rollback chain; widening the `k` bound;
relaxing `admit_rollback`; or making the refold cheaper by folding less than the canonical prefix.
This slice may only change **which valid baseline** a refold starts from — never what is proven
about it.

## Security analysis

**Against the consensus/lineage threat model: safe, and for a structural reason.** The accumulator
is the frozen-leadership authority, so accepting a wrong state corrupts the leader schedule. The
re-certification pins the settled point by **header hash**, and a Cardano header commits to
`prev_hash` — so a hash at slot `S` transitively commits to the *entire* chain prefix up to `S`. If
the settled point's hash still resolves canonically at its slot on the post-rollback chain, the
prefix that produced the stored state is byte-identical to the current canonical prefix, and the
state derived from it therefore cannot differ. That is the same hash-chaining argument the protocol
itself rests on, not a heuristic. Supporting rails:

- `k`-block separation is re-proved against the **new** tip, in block units.
- Reorgs deeper than `k` are already refused by `admit_rollback`, so this inherits the existing `k`
  dependency rather than adding one.
- The forward fold applies the identical deterministic transitions a full refold would.
- `SETTLED_BLOB` is the node's own deterministic output — never attacker-supplied, never wire data.
- Any check failing falls back to `reset_to_bootstrap`, i.e. exactly today's behaviour.

Note this slice does **not** introduce a new trust assumption: `reset_to_settled` already restores
from `SETTLED_BLOB` today, gated by `settled_rewind_admissible`. RF-1 stops the result being
discarded; it does not newly decide to trust it.

**The one genuine increase in exposure — durable-store integrity.** Today that trust is
*vestigial*: because recovery immediately resets to bootstrap, the node in practice always
re-derives from the Mithril-certified baseline. That yields an accidental but valuable property —
**self-healing by recomputation**: a silently corrupted accumulator is recomputed away on the next
rollback. RF-1 removes that self-heal on the bounded path.

The accumulator blob carries **no fingerprint**. The codec fails closed on malformed bytes, so
structural damage is caught — but a flipped bit inside an otherwise valid numeric field would decode
cleanly and be trusted. And this failure class is not theoretical here: EVIEW-R1/R2 was precisely a
durable store that became internally inconsistent and was only caught much later, at a comparison
boundary, on restart.

### CE-RF-6 is a HARD GATE, not a companion

**RF-1 must not be implemented unless CE-RF-6 lands first, or in the same sealed slice.** Restoring
a settled accumulator blob is only acceptable if that blob is integrity-bound; otherwise the slice
trades a proven-by-recomputation property for one trusted by convention.

The fingerprint must bind **at least**:

| binding | why |
|---|---|
| settled point `slot` / `block_no` / `header_hash` | ties the triple to one chain position |
| accumulator blob canonical hash | detects semantic bit-rot the codec decodes cleanly |
| settled leadership canonical hash | leadership is the promotion authority; it must not drift from its blob |
| schema / version | a v-N blob read under v-N+1 semantics is a silent reinterpretation |
| network / profile identity (where available) | prevents cross-venue restore (preview blob into preprod) |

That makes the restored state **mechanically comparable rather than trusted by convention**.
Precedent exists: `settled_leadership_encoding_roundtrips_and_fails_closed_when_torn` already does
torn-write detection for the leadership half — this extends the same discipline to the triple.

A periodic full bootstrap refold / cold-start self-heal cadence is worthwhile **operational**
hardening, but it is explicitly **not a substitute** for fingerprint verification: it bounds how
long corruption can persist, it does not detect it.

### The safe RF-1 shape

**Allowed:**

```
rollback occurs
  -> clear lineage anchor BEFORE the ChainDB rollback commit      (S5 pre-clear, unchanged)
  -> rollback commit succeeds
  -> re-certify settled point against the POST-rollback canonical chain
  -> verify settled-triple fingerprint
  -> restore settled accumulator
  -> write/re-establish LastAdvancedPoint for the restored post-rollback lineage
  -> continue forward fold
```

**Forbidden:**

- keeping the old anchor across the rollback commit
- restoring the settled blob without fingerprint verification
- restoring a settled point no longer canonical on the post-rollback chain
- restoring when not `k`-settled against the **new** tip
- silently accepting missing / torn / corrupt settled state
- weakening `admit_rollback`

### Tier classification

| tier | claim |
|---|---|
| **true** | restored state must be replay-equivalent to a deterministic refold from the same canonical prefix |
| **derived** | settled-point validity depends on Cardano hash-chain lineage and the `k`-bounded rollback assumption |
| **release** | CE-RF-6 fingerprint + the RF-1 bounded-refold proof must both be green before "thrash fixed" may be claimed |
| **operational** | periodic full bootstrap refold / cold-start self-heal cadence — hardening, never a substitute for CE-RF-6 |

### Implementation order (binding)

1. **CE-RF-6** — settled-triple fingerprint (stage + verify).
2. **RF-1** — re-establish the anchor after the durable rollback, using verified settled state.
3. Proof: a bounded settled rewind **survives** the next recovery pass.
4. Proof: **no** full bootstrap refold follows a successful settled rewind.
5. Negative: a corrupted / torn settled blob falls back to `reset_to_bootstrap`.

The conservative "same-epoch-only" variant (never re-certify across an epoch boundary) is **not**
adopted. Across-boundary settled rewind is safe once lineage, `k`-bound and settled-triple integrity
are all mechanically verified; it is held in reserve only if fingerprinting proves harder than
expected.

## Mechanical acceptance criteria (draft)

- **CE-RF-1** — a settled point that is canonical and `k`-settled on the post-rollback chain is
  re-certified and forward-folded from; the accumulator never touches the bootstrap baseline.
- **CE-RF-2** — a settled point that is NOT canonical on the post-rollback chain, or not `k`-settled
  against the new tip, is REFUSED and falls back to `reset_to_bootstrap`. Negative test; must fail
  if the re-certification ever trusts the stored point without re-checking.
- **CE-RF-3** — replay equivalence: forward-folding from a re-certified settled point yields state
  byte-identical to a full bootstrap refold over the same canonical chain (mirrors DC-EPOCH-31,
  which proves this for the rewind itself; here it must hold across the *recovery* path).
- **CE-RF-4** — crash-window safety preserved: a crash between the pre-clear and the ChainDB
  rollback commit still leaves an anchor-absent store that refolds from canonical. Must be pinned
  as a test, since this slice is precisely where it could be eroded.
- **CE-RF-5** — live: **CE-AR-6 discharged for real** — a sustained run showing post-rollback refold
  cost bounded and **not growing with uptime**.

  **Measured baseline to beat (2026-08-02 preview, two independent stores):**

  | time | refold distance | how the rollback resolved |
  |---|---|---|
  | 18:41 | 153,565 slots | `reset_to_settled` → anchor absent → bootstrap refold |
  | 19:18 | 155,796 slots | same |
  | 21:55 | **165,210 slots** | `reset_to_bootstrap` **directly** — no settled point existed to try |

  ~11,600 slots of growth in ~3 hours, ~27 min per refold. Both arms of the loop are now observed
  live: the settled rewind being *discarded* (18:41, 19:18) and, once `reset_to_bootstrap` had
  deleted the settled triple, no bounded target existing *at all* on the next rollback (21:55).
  This is precisely the unbounded growth `DC-EPOCH-30` claims to bound, so it is also the concrete
  evidence that **CE-AR-6 failed rather than went unobserved**.

- **CE-RF-6** — settled-triple integrity (see the hard gate above): the triple is fingerprinted when
  staged and verified before restore; mismatch → `reset_to_bootstrap`. **Lands first, or in the same
  sealed slice — RF-1 may not ship without it.**

Registry: `DC-EPOCH-34` (settled-triple integrity — fingerprint-bound, never trust-on-read),
`DC-EPOCH-35` (a bounded settled rewind survives the recovery pass that follows it), `DC-EPOCH-36`
(re-certification is proof-carrying against the post-rollback chain). `enforced` only once
CE-RF-1..4 and CE-RF-6 exist as tests **and** the CI gate lands; CE-RF-5 is supporting live
evidence, never the reason.

## IMPLEMENTED 2026-08-03 — CE-RF-6 then RF-1, in the required order

**CE-RF-6 first** (`a6d584e2`, `DC-EPOCH-34`) — the settled triple is fingerprint-bound and verified
before any restore. Landed as the hard gate, additive only.

**RF-1 second** (`DC-EPOCH-35`) — after the ChainDb rollback COMMITS, the settled point is re-proved
against the chain as it now stands and the anchor is re-established there:

```
pre-clear (anchor cleared)  ->  apply_chain_event(RolledBack)  ->  re-certify + re-anchor
        [S5 crash window, unchanged]        [durable]        [canonical? k-settled? fingerprint? cursor?]
```

Order is the safety property: the anchor is never carried ACROSS the rollback window, so a crash in
that window still refolds from canonical. The uncertified window is **closed afterwards, never
widened**. Any failed proof leaves the anchor absent — the unchanged pre-slice behaviour — so this
can only ever save refold time.

### Gates, as required

| # | requirement | how it is held |
|---|---|---|
| 1 | anchor cleared before the rollback commit | gate asserts `pre-clear < apply_chain_event` line order |
| 2 | re-certification checks canonical / k-settled / fingerprint | three separate checks; k in **block** units, slot comparison explicitly forbidden |
| 3 | restored state writes a new `LastAdvancedPoint` | `recertify_settled_anchor_writes_the_anchor_at_the_settled_point` |
| 4 | next pass does not reset on `AnchorAbsent` | `a_recertified_settled_point_makes_the_next_pass_forward_fold` asserts `ResetAndRefold` **before** and `ForwardFold` **after** — so it cannot pass vacuously |
| 5 | no bootstrap refold after a successful bounded rewind | same test asserts the accumulator is still at the settled point, not the seed |
| 6 | final state equals the full-refold result | inherited from `DC-EPOCH-31` (`refold_from_settled_point_equals_fold_from_bootstrap`) — this slice moves only the *starting point* of a deterministic re-derivation |

Negatives, all landing on `reset_to_bootstrap`: corrupt blob / bad fingerprint / point not canonical
on the new chain / not k-settled against the new tip / missing-or-absent settled triple / cursor not
at the settled point.

### The gate exists because nothing else catches the worst regression

Deleting the post-commit call **compiles clean and every unit test still passes** — the tests
exercise the function directly. Only the structural gate catches an unwired or mis-ordered call.
Three mutations were verified caught: unwiring it, moving it before the commit, and weakening
`DC-EPOCH-29` (making `reset_to_settled` stop de-certifying) to make re-certification easier.

## OPEN OBSERVATION 2026-08-03 — arming the bound may be rarer than discarding it was

RF-1 stops a settled point being **discarded**. Separately, and not addressed by this slice,
**establishing** one is harder on a rollback-frequent network than the design assumes.

`roll_settled_rewind_point` stages the current point only at `ReachedTip`, and promotes it only once
the tip has advanced `k` blocks past it. On preview `k = 432`, `f = 0.05`, so promotion needs
**≈ 8,640 slots ≈ 2.4 h of uninterrupted tip-hold**. And **both reset paths clear `PENDING_*`**, so
any rollback destroys the staged point and restarts the k-block clock from zero.

Measured inter-rollback gaps on the CE-RF-5 store: **37 min, 160 min, 65 min, ~40 min.** Only one of
four exceeded 2.4 h. So on observed behaviour a settled point can be banked roughly **1 attempt in
4**, and a node that is repeatedly knocked off tip may never bank one at all.

Consequences, stated carefully:

- This is **not an RF-1 defect.** RF-1 does exactly what it claims when a settled triple exists.
- It **does** mean the bound engages less often than DC-EPOCH-30's framing implies, because that
  rule reasons about the rewind DISTANCE and is silent on how often a rewind target exists.
- It makes **CE-RF-5 probabilistic to observe** rather than reliably reproducible on preview: the run
  must catch a >2.4 h rollback-free window followed by a rollback.
- The natural follow-up (NOT taken here, and not a claim) is the staging cadence itself: staging only
  at `ReachedTip` is what couples arming the bound to sustained tip-hold, which is precisely the
  property a thrashing node lacks. A cadence that could stage from a settled-by-depth point without
  first reaching tip would decouple them — but that is a new invariant needing its own proof, not a
  tweak, and it must not weaken the k-rule.

Recorded now rather than after CE-RF-5 resolves, so a slow or absent live signal is read as *the
bound rarely arming*, not as *RF-1 failing*.

## Not claimed

- No claim that this eliminates refolds — only that a refold starts from a bounded, proven baseline.
- No change to rollback admission, the `k` bound, the crash-window pre-clear, or EVIEW-R2's seal
  positioning.
- DC-EPOCH-26..31 are not weakened or withdrawn; they remain correct as unit properties. What
  changes is the honest statement of their **production reach**, and CE-AR-6's status.
