# Pre-preprod local-first work streams (drain before the stake gate)

> **Purpose.** Maximise what we de-risk on the **local testnet / hermetic corpus** before
> transitioning to preprod. The C2 guide §7b principle: *exercise each failure class in the
> CHEAPEST venue that can produce it* — a bug found locally costs hours; the same bug in
> preprod costs the **~2-epoch (~10-day) stake-snapshot gate** to retest. Iterate locally first.

## Context (where we are, 2026-06-13)

- **PHASE4-N-AO CLOSED** — `CN-CONS-03` enforced (multi-candidate fork-choice SELECT, NATURAL
  CE-AO-6 pass). §7b **rung 2 fully done** (FOLLOW + SELECT). Producer-accept (`CN-CONS-06`) +
  recover (`RO-LIVE-05`) already enforced.
- **Rung 3 = preprod** is the only RO-LIVE-flipping rung, and it is **stake/time-gated**
  (ADE1 active ~epoch 295 + faucet delegation), **not code-gated**. So: drain everything
  local-exercisable now; touch preprod only when the stake matures.
- Registry at HEAD: **132 open rules** (113 declared + 19 partial). Most CN-\*/T-\* are
  *constitutional* (enforced by lower-level DC rules + existing code, not yet status-flipped);
  several "open" rules already carry substantial tests but lack a CI gate. The genuinely-open,
  locally-exercisable, bounty-relevant work is the three streams below.

## Execution order (STRICT): **Stream 3 → Stream 1 → Stream 2**

Rationale: Stream 3 (hermetic enforcement sweep) sizes the real backlog and isolates
real-gaps from already-enforced bookkeeping **before** we commit to the behavioral work;
it also clears the `CN-META-01` / `RO-REL` "every invariant has mechanical enforcement"
release-eligibility bar cheaply (no venue). Then Stream 1 (the #1 release-blocker), then
Stream 2 (the gap-prone epoch transition).

---

## Stream 3 — Enforcement-mapping sweep  *(FIRST; hermetic, no venue)*

**Goal:** for every declared/partial rule, determine its true state — (a) already enforced
(tests + a real gate) but un-flipped → write/map the gate + flip; (b) a real behavioral gap →
document + route to Stream 1/2 or a new cluster; (c) constitutional (enforced by sub-rules) →
flip with cross-ref; (d) preprod/external-gated → mark out-of-scope. Surface gaps, never hide.

**Why it exists:** rules like `DC-PROTO-02` (45 tests), `DC-CONSENSUS-02` (16), `DC-LEDGER-05`
(9) are tested but `declared`/`partial` with **no `ci_scripts`** — that is bookkeeping, not new
work. Backs `CN-META-01` ("every invariant has a mechanical enforcement point"), `RO-REL-01/03`,
`T-CI-01` ("every true invariant has CI enforcement; no waivers").

**Done =** every open rule is classified; every already-enforced rule has a CI gate + is flipped;
every real gap is documented with its target stream/cluster; the registry status tally reflects
reality (no tested-but-declared rules left without an explicit "real gap" reason).

**Step 1 (this stream's first deliverable):** the enforcement classification of all 132 open
rules — `{flip-now, needs-gate-then-flip, real-gap, constitutional-flip, preprod/external}`.

---

## Stream 1 — Differential validation agreement + false-accept hunt  *(SECOND; bounty #1, local)*

**Goal:** Ade's BLUE validation verdict agrees with cardano-node on every tested block/tx, and
**no false-accept** survives an adversarial negative corpus. This is the half of the bounty
`CN-CONS-03` does not cover; recorded priority: *tx-validity agreement > live-following; a
false-accept is release-blocking.*

**Rules:** `DC-LEDGER-03` (validity agrees, all eras — partial), `CN-LEDGER-09` + `DC-LEDGER-05`
(witness binding completeness, incl. required-signer enumeration — partial, no CI), `CN-LEDGER-07/08`
(conservation, no double-spend), `CN-LEDGER-03` (matches reference), `CN-PLUTUS-*` (script
determinism), harden `DC-DIFF-01` (differential harness localizes first divergence).

**Local how:** push a corpus of real preprod blocks/txs **+ adversarial mutations** through Ade's
validation and diff verdicts against a local cardano-node; the adversarial negative corpus IS the
false-accept hunt. Cheapest venue; catches the release-blocker before preprod.

**Done =** validity agreement proven across all supported eras on a named corpus + an adversarial
negative corpus with zero false-accepts; witness-binding completeness (incl. required-signers)
enforced per era with gates.

---

## Stream 2 — Epoch transition  *(THIRD; rung-1 remainder, local single-producer)*

**Goal:** a single-producer C2-LOCAL run crosses an epoch boundary (slot 2000) with adoption
continuing — exercising nonce roll, stake re-snapshot, leadership recompute, reward calc live.
The C2 guide flags this as *"the path most likely to surface a gap — find it here, not in preprod."*

**Rules:** `DC-NODE-19` (forge-loop continuation past EOF — declared, 0 tests), `DC-LEDGER-04`
(epoch-boundary stake/rewards match Haskell — partial, no CI), `CN-EPOCH-01`/`DC-EPOCH-01`
(activation timing, governance enactment atomic at boundary).

**Local how:** the reusable single-producer harness (`~/.cardano-rung1-host/rung1-auto.sh`); run
past slot 2000; verify Ade rolls the nonce / re-snapshots stake / recomputes leadership and keeps
forging + being adopted across the boundary. Non-promotable (flips no RO-LIVE rule), but high
gap-risk.

**Done =** one epoch crossed with adoption continuing; epoch-boundary computations differ-checked
against cardano-node; `DC-NODE-19` exercised + enforced.

---

## Out of scope here (preprod / external-gated — cannot drain locally)

`RO-LIVE-01` (preprod block accepted → `Ba02Manifest`), `CN-EPOCH-03` (canonical stake-snapshot
derivation at scale), `RO-SYNC-EVIDENCE-01`, `RO-MITHRIL-IMPORT-01` / `RO-GENESIS-REPLAY-01`
(external/import), `DC-NODE-17` (real observed-peer advancement), `RO-LIVE-02`, and the
`CN-NET`/`OP-NET` topology-diversity rules (need a real multi-region network). These wait for
the preprod pass or dedicated external work.
