# Invariant Sketch — Local selected durable chain forge-base authority (DC-NODE-20 / DC-NODE-21)

**Cluster (proposed):** PHASE4-N-AH — the correcting dependency that retires the operator adoption
certificate as forge-loop authority. **Rung-1, single-producer only.**

**Motivation (live, run-4, 2026-06-08):** Ade forged block 11 and a real Haskell relay adopted it,
but Ade then returned `no_tip_available` on every subsequent tick. Root cause (grounded in
`node_lifecycle.rs`): the `proceed_to_forge` gate requires `durable_servable_tip ==
followed_peer_tip` (the DC-NODE-15 catch-up check), which fails the instant Ade self-admits its own
block — the non-producing relay adopts but never re-announces, so `durable(11) ≠ followed(10)` →
gate false → `NoTipAvailable`. The only escape today is cert-promotion into extend mode, which raced
(and lost to) the follow-link EOF. The operator adoption certificate had leaked from **evidence**
into **forge-loop authority**. A normal producer extends its **local selected durable tip** (block
11 was already in Ade's ChainDB via `pump_block`; `selected_tip = ChainDb::tip` is already computed).
DC-NODE-20 moves forge-base authority back to that local tip; DC-NODE-21 fences the cert to
evidence-only. Run-4 logs preserved at `~/.cardano-rung1-host/run4-dcnode20-motivation/` (operator
scratch, not committed).

## 1. What must always be true
- **DC-NODE-20 (new, primary):** In a declared rung-1 single-producer venue, after Ade self-admits a
  valid forged block through `pump_block` onto its local durable ChainDB spine, the next forge base
  is Ade's **local selected durable tip** (the head of its own admitted ChainDB spine — `ChainDb::tip`),
  **not** `followed_peer_tip` and **not** an operator adoption certificate. Holds **only** while ALL
  six fence conditions in §2 hold; otherwise fail closed. **Relay adoption is evidence, not a
  forge-loop precondition.**
- **DC-NODE-21 (new, paired):** the file-based operator adoption certificate is a rung-1 RED
  evidence-only shim — it may prove relay adoption for the transcript/bounty bundle, but must never
  control forge-base selection or any durable authority, and must never appear in
  multi-producer/preprod/production forge paths; it must be removed/replaced by node-local
  selected-chain / fork-choice authority before rung 2 / preprod.
- **DC-NODE-05 (preserved):** `pump_block` remains the sole durable admit authority. DC-NODE-20 only
  *reads* the tip `pump_block` produced (fence condition 4); it advances no tip.
- **DC-NODE-12 (preserved):** own-forged durable admit chokepoint.
- **DC-NODE-15 (preserved, scoped):** the `durable == followed` gate governs **initial** catch-up
  (before the first own-forge). DC-NODE-20 supersedes only the **repeated** post-self-admit
  `durable == followed` re-check — initial catch-up is unchanged.
- **DC-NODE-18 (core preserved; promotion-mechanism superseded):** own-spine forge stays; its
  cert-promotion into extend mode is replaced by local self-admit.
- **DC-NODE-19 (preserved, declared/partial):** continue-past-EOF in the extend state stays valid
  hermetic infrastructure. DC-NODE-20 changes how the extend state is **entered** (local self-admit,
  not cert); the live sustained-past-k proof re-homes onto this path.
- **DC-CONS-03 (preserved, untouched):** fork-choice is the rung-2 successor authority. In rung 1
  "selected" is **degenerate** — no competing candidate ⇒ the local spine head *is* the selected tip.
- **T-REC-03 / T-REC-05 (preserved):** the local-tip forge base stays replay-equivalent.

## 2. What must never be possible (illegal states / forbidden transitions)
- The operator adoption certificate **must never** control forge-base selection (evidence-only,
  DC-NODE-21).
- The certificate **must never** appear in multi-producer / preprod / production forge paths.
- The forge base **must never** silently fall back to `followed_peer_tip` or the cert when the fence
  fails — it **fails closed**.
- **DC-NODE-20's six-condition fence** — the local-tip forge base engages ONLY while ALL hold; any
  failure → fail closed (no fallback):
  1. `VenueRole == SingleProducer`;
  2. **no competing block has been observed on the canonical peer receive stream** since initial
     catch-up / self-admit — an **observed-feed fence, NOT fork-choice**; if one is observed, fail
     closed, do **not** attempt to resolve it (that is rung 2);
  3. the relay is non-producing;
  4. the forged block was admitted through `pump_block`;
  5. the ChainDB spine is contiguous and servable;
  6. no fork-choice decision is required — in rung 1 **mechanically derived from (2)**: no competing
     candidate observed ⇒ the local spine head is the degenerate selected tip; a competing candidate
     observed ⇒ fork-choice required ⇒ DC-NODE-20 disabled.
- The forge **must never** advance the durable tip outside `pump_block` (DC-NODE-05).
- Forging on a stale / non-local tip **must be impossible** (fails closed in `pump_block`,
  extend-only `block_validity` / `prior_fp` — DC-CONS-23).
- A competing candidate chain **must never** be silently resolved in rung 1 — observing one trips
  condition 2/6 → fail closed (real fork-choice is rung 2, not this rule).

## 3. What must remain identical across executions (deterministic surface)
- Given the same local durable ChainDB spine, the forge base (`ChainDb::tip`) is a deterministic
  function of the ChainDB head — **no** wall-clock, **no** followed-peer arrival timing, **no**
  cert-file write timing influences it.
- The six-condition fence evaluation is deterministic over the canonical received-block + venue
  state (condition 2's "competing block observed" is a deterministic predicate over the canonical
  peer receive stream).

## 4. What must be replay-equivalent
- The same recovered checkpoint + WAL → the same local durable spine → the **same forge base** → the
  **same forged successors** (byte-identical). Removing the cert + followed-peer timing from the
  forge base is what *makes* the post-self-admit forge replay-equivalent over the local durable state
  alone — today a non-replayable RED file/timing leaks into the authority path.
- T-REC-03 (loop-as-replay) and T-REC-05 (forged-chain warm-start) extended: replaying the local
  durable spine reproduces the same forge base + successors, independent of the cert.

## 5. State transitions in scope
- **Mode entry (the core change — `FirstOwnBlockServed` folded out):**
  `(CaughtUpToPeerTip, self-admit valid own block via pump_block onto own spine ∧ DC-NODE-20 fence holds)`
  → `Ok(SingleProducerExtendOwnDurableSpine { current_tip = ChainDb::tip }, ∅)` — **direct**, no
  cert-wait intermediate, **no** cert read.
  *(Today: `(CaughtUpToPeerTip) → FirstOwnBlockServed → (cert present ∧ matches) → extend`.)*
- **Forge-base selection:** `(SingleProducerExtendOwnDurableSpine, ForgeTick)` →
  `Ok(forge on ChainDb::tip, ∅)` — admissibility derives from the local durable tip authority, NOT
  from `durable == followed` and NOT from the cert.
- **Competing-block fence:** `(receive | ForgeTick, competing block observed on the canonical peer stream)`
  → `Err(fail-closed; DC-NODE-20 disabled; no resolve)`.
- **Certificate:** `(cert file present)` → `Ok(evidence record for transcript, ∅)` — never a
  forge-base input (DC-NODE-21).

## 6. TCB color hypothesis
- **BLUE (reused, unchanged — no new authority):** `ChainDb::tip` (local durable selected tip),
  `pump_block` (durable admit authority), `forge_one_from_recovered`, `block_validity` / `prior_fp`
  (DC-CONS-23).
- **GREEN:** the forge-base **selection** — the direct `CaughtUpToPeerTip → SingleProducerExtendOwnDurableSpine`
  transition + the `proceed_to_forge` gate rewire that derives admissibility from the local durable
  tip (not `durable == followed` / the cert); the six-condition fence including the observed-feed
  competing-block predicate. Deterministic glue; affects scheduling, never the durable surface.
- **RED:** the adoption certificate — **demoted to evidence-only** (transcript record, never the
  forge base); `followed_peer_tip` signal (stays RED, no longer a forge-base authority post-self-admit).
- **Color resolved (was OQ-20-1):** condition 2's "competing block observed" is a **GREEN**
  predicate over the canonical received-block stream (an observed-feed fence), not a RED peer-state
  query and not fork-choice.

## 7. Resolved design decisions (from /invariants review)
- **OQ-20-1 → resolved:** condition 2 = "no competing block observed on the canonical peer receive
  stream since initial catch-up / self-admit" (observed-feed fence, fail-closed, no resolution);
  condition 6 mechanically derived from condition 2 in rung 1.
- **OQ-20-2 → resolved:** fold `FirstOwnBlockServed` out of the authority path — direct
  `CaughtUpToPeerTip + self-admit → extend`; no cert-free intermediate (it existed only because the
  cert was promotion authority).
- **OQ-20-3 → resolved:** DC-NODE-21 is its **own** rule (separately enforceable; hard rung-2 removal
  boundary; prevents the shim creeping back as authority).
- **OQ-20-4 → resolved:** DC-NODE-15 stays the initial catch-up gate; DC-NODE-20 supersedes only the
  repeated post-self-admit `durable == followed` re-check.
- **OQ-20-5 → resolved (cluster bookkeeping):** PHASE4-N-AG closes/archives as "hermetic core (S1–S3)
  complete; live CE-AG-5 superseded/re-homed by DC-NODE-20; DC-NODE-19 stays declared/partial, not
  enforced" — an honest superseded/partial close, never a normal "complete" close.
- **Out of scope (explicit):** follow-link keep-alive (OQ-KA); real fork-choice / multi-producer
  (rung 2, DC-CONS-03 successor); preprod.

## Can this be expressed as canonical input → canonical output?
**Yes.** `forge_base(local durable ChainDB spine, venue fence) → local selected durable tip` is a
pure function of canonical state. DC-NODE-20 *removes* the nondeterministic RED inputs (cert file,
followed-peer timing) from the authority path — it makes the forge base **more** deterministic /
replay-equivalent, not less. No new nondeterminism is introduced.

## Addendum — DC-NODE-22 (single-producer warm-start re-entry), from S4 run-2 (2026-06-08)

The S4 live run-2 (`docs/evidence/phase4-n-ah-live-run-2-partial.md`) surfaced the next invariant: the
DC-NODE-20 forge base is correct on the clean path, but **warm-start re-initializes
`forge_mode = InitialCatchupRequired`**, requiring a fresh follow-link catch-up; when the follow link
EOFs first, the node stalls in `NoTipAvailable` — re-introducing through restart the follow-link
dependency DC-NODE-20 retired (96 `forge_tick_considered` / 0 `forge_attempted` post-restart).

- **Must always be true:** in a declared rung-1 single-producer venue, if warm-start recovery yields a
  durable local `ChainDb::tip` **above the bootstrap anchor** (own-forged continuation), `forge_mode`
  re-enters `SingleProducerExtendOwnDurableSpine{current_tip = ChainDb::tip}` under the DC-NODE-20
  fence, **without** a fresh followed-peer catch-up — the warm-start analog of DC-NODE-20.
- **Must never be possible:** re-entering extend at the bare anchor / non-single-producer / on a
  recovery error / with a competing peer block / with a cert — all FAIL CLOSED to
  `InitialCatchupRequired`. NOT a general restart rule for multi-producer / preprod (DC-CONS-03
  untouched).
- **Replay-equivalent:** warm-start recovery of the durable tip / served chain is unchanged
  (T-REC-05); DC-NODE-22 only sets the post-recovery forge mode. `pump_block` stays the sole durable
  admit.
- **TCB:** GREEN predicate (recovered tip + anchor + venue facts) + RED `node_lifecycle` warm-start
  arm; BLUE unchanged. Registry: **DC-NODE-22** (declared); strengthens DC-NODE-20/19 + T-REC-05 +
  CN-NODE-02.
