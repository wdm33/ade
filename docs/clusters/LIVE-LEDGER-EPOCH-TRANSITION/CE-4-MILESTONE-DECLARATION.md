# CE-4 — milestone declaration (literal three-boundary continuous operation)

> **A frozen, auditable milestone boundary.** This declaration records exactly what CE-4B earned and,
> equally, what it does NOT claim. It is an evidence declaration, not a slice — no code, no new invariant.
> Written after CE-4B landed green (`c5bdc064`) and before any further work (R4c, live, bounty).

**Cluster:** LIVE-LEDGER-EPOCH-TRANSITION (DC-EPOCH-19 self-sufficiency).

---

## What CE-4B proves

**Literal three-boundary continuous operation:** in ONE production-loop run, Ade crosses
**1340 → 1341 → 1342 → 1343** (seed+2 → seed+5).

Ade remained **self-sufficient** across all three boundaries:

- **no re-import** (no bootstrap oracle seed after the run began);
- **no CLI oracle** (no cardano-cli / external authority);
- **no seed-window replay** (the retired seed-anchored window-replay is not used);
- **no `materialize_bootstrap_into`** (no bootstrap-state re-materialization);
- **promotion-certified frozen leadership through 1344** (the node self-derives + certifies its own
  candidate leadership past the last crossed boundary — it did not run out of authority).

The pre-S4 seed window that halted at seed+3 (1341) does NOT recur; the recurrence continues for three real
boundaries.

---

## Evidence (auditable)

- **Commit:** `c5bdc064` (`feat(ledger): CE-4B ...`); test `ce4b_three_boundary_continuous_self_sufficiency`
  (`#[ignore]`, ~2.9h GREEN); bundle `ce4b-evidence.json`.
- **Boundary crossings** (one continuous run, each with a self-derived eta0):
  1340→1341 @ 115862416, 1341→1342 @ 115948834, 1342→1343 @ 116035206.
- **Final durable tip** 116041708, epoch **1343**.
- **Frozen leadership sealed** 1341 / 1342 / 1343 / **1344** (look-ahead intact past seed+5). The
  self-derived `1342 = 014f96d3…` and `1343 = d1ba2eb2…` are **byte-identical to the CE-4A.3 #13 authority**
  (`fd3826fd`) — cross-proof consistency.
- **Promotion-certified** 1341 / 1342 / 1343 / 1344 (1345 not yet reached — correct).
- **eview promotions** 1341 / 1342 / 1343 durable in the WAL; **`forbidden_paths` clean**.
- **Corpus** extracted LOCALLY (no AWS): the preview ImmutableDB via `slice_chunks.py` (chunks 26841–26861);
  the corpus is 1338→1343 continuous.

Built on: CE-4A.1 (`9c6fc3c4`), CE-4A.2 (`af3dc9c7`), CE-4A.3 (R1 `7266f90c`, #13/R3 `fd3826fd`), S4 flip
(`db702a54`).

---

## NOT claimed (explicit — the boundary of this milestone)

CE-4 (and CE-4B) does **NOT** claim any of the following. Each is tracked as separate, open work:

- **live preview/preprod operation** (a sustained run against a real peer);
- **block production** (forging + adopting own blocks);
- **Haskell peer acceptance** (a cardano-node adopting Ade's chain);
- **bounty completion**;
- **warm-restart crash-window recovery after rollback-before-refold** — CE-4A.3-R4 is PARKED with R4c
  (a VRF/nonce reconstruction gap) OPEN; a hard precondition before any live/bounty *recovery* certification
  (`SLICE-CE-4A-3-R4-WARMSTART-ROLLBACK-PENDING-REFOLD.md`, findings `4bc49fa6`);
- **POST-1343 byte-exact reference equivalence** (Ade's 1343 outputs vs a cardano LedgerState reference — an
  optional strengthening, deferred).

---

## Milestone meaning

The core question — *"will the node run out of self-derived authority past the seed window?"* — is answered
for the proven corpus: **No, not across 1340→1343 (three real boundaries).** This is a Cardano-compatible
self-sustaining-ledger milestone. The known recovery caveat (R4c) is closed BEFORE bounty posture.

---

## Recommended order after this declaration

1. **This CE-4 milestone declaration** (locks the boundary).
2. **CE-4A.3-R4 / R4c** — the warm-restart-after-rollback crash-window fix (resume from the off-repo patch;
   before live/bounty recovery certification).
3. POST-1343 byte-exact strengthening (optional).
4. Live preview/preprod sustained operation.
5. Block production + Haskell acceptance.
6. Bounty certification package.
