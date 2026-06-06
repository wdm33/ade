# C2-LOCAL discovered gaps — recorded as later invariant slices

Surfaced by the 2026-06-06 cardano-testnet C2-LOCAL rehearsal (see
`docs/active/c2-preprod-tip-guide.md` §5b). The rehearsal proved **#1–#7** (venue +
extract + recover + **Ade forged real pool1 blocks from recovered non-Origin Conway
state**) but **#8–#9 (Haskell peer adopts → correlate) were NOT proven**: forge success is
local; external validity requires a Haskell peer to receive → validate → select/adopt →
correlate (adopted hash == forged hash). Two distinct Ade gaps blocked #8. Both are
recorded here as **later** invariant slices — they are real Cardano-compatibility
requirements, not venue failures, and must **not** be worked around by weakening Ade.

Doctrine note: deterministic chain selection is **true-tier**; oracle (Haskell) validity
agreement is **derived-tier**. Neither may depend on arrival order, scheduler effects, or
non-canonical observables.

---

## Gap 1 — multi-producer candidate intake + deterministic fork-choice

**Tier:** true / derived.

**Observed (#8, all-pools venue):** with node2/node3 left as *competing* producers
(pool2/pool3), Ade `--mode node` forges on its own recovered chain and follows the peer
separately; Ade's pool1 branch **loses fork-choice** to the peers' longer chain, and Ade's
extend-only receive-side **fails closed** (`BlockNoOutOfOrder { last:10, attempted:9 }`).
Ade has no candidate-chain intake / fork-choice to participate as one producer among
several.

**Why it matters:** a Cardano-compatible node must select among competing candidate chains
**deterministically** and converge on the same tip the Haskell nodes do.

**Proof obligation:** a competing-producer private net where Ade and ≥1 Haskell producer
both forge and **converge on the same tip** (same chain-selection outcome as Haskell),
with no dependence on block arrival order.

**Non-goal until then:** Ade must not be *judged* in a multi-producer venue; it cannot yet
handle competing producers, full fork-choice, alternative-chain merge, or adversarial
multi-producer races.

---

## Gap 2 — recover → forge → serve continuity (forge on the *followed* tip)

**Tier:** true / derived.

**Observed (#8, two-phase relay venue):** with node2/node3 as *non-producing relays* (Gap 1
removed), Ade still failed #8 by a second seam:
- Recover **at the peer's exact frozen tip** (block 51) → the follow re-derives that slot →
  `Store(InvalidOperation("snapshot at slot N already occupied by different bytes"))` (the
  UTxO-seed-derived snapshot ≠ the block-applied snapshot at the recovered slot).
- Recover **one block behind** (block 50) → Ade's **forge-tick raced the follow**: it forged
  its *own* block on the recovered tip (50) instead of first adopting the relay's block 51,
  so Ade's chain **forks** from the relay's, and the relay **rejected** Ade's served chain
  (`HeaderEnvelopeError (UnexpectedBlockNo (BlockNo 52) (BlockNo 0))`). Ade's
  recovered-then-forged served chain did not present an adoptable continuation of the
  relay's chain. (The exact `slot 13` rollback anomaly in the relay's reject needs deeper
  investigation; the *outcome* — non-adoption — is clear.)
- The earlier (recover-at-158, peer several blocks ahead) run *did* follow-then-forge,
  which is the shape that works: catch up to the peer tip, **then** forge the successor.

**Why it matters:** Ade's forge base must be the **followed/caught-up peer tip**, not the
static recovered snapshot tip, so the forged successor extends the peer's adoptable chain;
and the served chain must expose the recovered history's intersect point so a Haskell peer
can intersect and roll forward onto Ade's successor.

**Proof obligation:** a **non-producing-relay** private net where Ade recovers from a
non-Origin tip, **follows the relay to its current tip**, forges the successor **on that
tip**, serves it, and **the relay adopts** it (`ValidCandidate` + `AddedToCurrentChain`),
with **forged hash == adopted hash**.

**Recover-far-behind attempt (2026-06-06) — ISOLATED Gap 2 into 2a + 2b:** bounded
orchestration run — relays frozen at T=block 21, Ade recovered at anchor=block 8 (k=13,
both epoch 0), Ade followed node2-relay and **caught up cleanly with NO receive-side error**
(the `BlockNoOutOfOrder` from the close 1-block gap is GONE) and forged **BlockNo 22 =
followed-tip(21)+1**. But node2-relay (connected, HandshakeSuccess) **still rejected Ade's
served chain** (`UnexpectedBlockNo (BlockNo 22) (BlockNo 0)`, intersecting only at an early
point). So:

- **Gap 2a — forge-on-followed-tip:** *met by this orchestration at the BlockNo level* (Ade
  caught up then forged the successor at followed-tip+1, no fork). NOT mechanically enforced
  — it held by timing (large k), so the admission gate "forge admissible only when local
  selected tip == followed peer tip" is still owed as code.
- **Gap 2b — serve-continuity (the crux, UNMET):** Ade's durable **served** chain is not a
  continuous, peer-adoptable chain — node2-relay cannot intersect at the followed tip
  (block 21) and roll forward onto Ade's forged successor (block 22); it intersects only at
  an early common point and then sees BlockNo 22 where it expected the next block, and
  rejects. Ade must serve a chain that exposes the followed tip as an intersect point with
  the forged successor extending it (the followed history must be in the durable served
  chain, not only the recovered anchor + own-forged blocks).

**#8–#9 remain NOT proven.** Per the project rule, orchestration stops; the next real work
is the **Gap 2 implementation slice** (below), centred on **2b serve-continuity** (+ the
2a admission gate), then the Gap 1 slice.

---

## What the C2-LOCAL rehearsal DID prove (do not overstate)

Ade can **recover a non-Origin private Conway tip** and **forge as an active-staked pool**
(`forge_result:succeeded`) from that recovered state, on a faithful cardano-testnet venue
(the `d`-bridge / cold-start stays entirely outside Ade). It did **not** prove a Haskell
peer adopts the forged block — that is gated on Gap 1 (competing producers) and Gap 2
(forge-on-followed-tip + serve continuity) above.

Status: **#1–#7 proven; #8–#9 not proven; two Ade gaps recorded for later slices.**

---

## Implementation slices (next work, IDD discipline)

Orchestration is exhausted — the recover-far-behind run isolated the remaining gap to
**serve-continuity**. The next real work is code, in this order:

### Slice A — Gap 2 (do first; directly unblocks the C2-LOCAL adoption proof)
- **recover → serve continuity:** the durable served chain must be continuous from the
  recovered anchor through the followed blocks to the forged successor, so a Haskell peer
  can intersect at the followed tip and roll forward onto Ade's block. *(the 2b crux)*
- **forge-on-followed-tip admission gate:** forge admissible **only when** local selected
  tip == followed peer tip (mechanical enforcement, not timing). *(2a)*
- **peer intersection before forge** + **structured refusal when not caught up** (no
  forge on a stale/recovered-only tip; fail closed with a typed reason).
- **Closure proof:** the non-producing-relay venue (Gap 2 proof obligation above) — relay
  adopts Ade's forged successor; **forged-block parent == relay selected tip hash**; forged
  hash == adopted hash.

### Slice B — Gap 1 (after Slice A)
- **multi-producer candidate intake** + **deterministic fork-choice** + **competing-branch
  handling** (chain selection that matches Haskell, independent of arrival order).
- **Closure proof:** competing-producer private net where Ade + ≥1 Haskell producer
  converge on the same tip.

Until Slice A lands, a C2-LOCAL "Haskell peers adopted Ade's block" manifest is **not**
emittable; #8–#9 are proven only on preprod or after Slice A closes the local loop.
