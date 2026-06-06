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

**Likely-clean workaround for the next attempt (no Ade change):** recover **far enough
behind** the relays' frozen tip that the follow catches up *before* the forge-tick fires
(capture an earlier block's point during bootstrap), so Ade adopts the relay tip then forges
the successor — the shape the recover-at-158 run already exhibited.

---

## What the C2-LOCAL rehearsal DID prove (do not overstate)

Ade can **recover a non-Origin private Conway tip** and **forge as an active-staked pool**
(`forge_result:succeeded`) from that recovered state, on a faithful cardano-testnet venue
(the `d`-bridge / cold-start stays entirely outside Ade). It did **not** prove a Haskell
peer adopts the forged block — that is gated on Gap 1 (competing producers) and Gap 2
(forge-on-followed-tip + serve continuity) above.

Status: **#1–#7 proven; #8–#9 not proven; two Ade gaps recorded for later slices.**
