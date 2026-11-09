# CE-4B — literal three-boundary continuous-operation proof (N→N+1→N+2→N+3)

> **Status: OPEN (scoped, doc-before-impl).** The deferred strengthening of CE-4A (§6 of
> `SLICE-CE-4A-CONTINUOUS-SELF-SUFFICIENCY.md`): the LITERAL three-boundary continuous run that CE-4A's
> mechanical gate (CE-4A.1, two boundaries 1340→1342) deliberately did not claim. Corpus extracted LOCALLY
> (no AWS). Builds on CE-4A.1 (`9c6fc3c4`, continuous self-sufficiency) and CE-4A.3 (`fd3826fd`,
> restart/rollback replay-equivalence).

**Cluster:** LIVE-LEDGER-EPOCH-TRANSITION.

---

## 1. Intent (the claim)

> In ONE continuous production-loop run, Ade crosses AT LEAST THREE consecutive epoch boundaries
> (1340→1341→1342→1343 = seed+2 → seed+5) and remains SELF-SUFFICIENT across all three — it seals its own
> frozen leadership for each successive candidate epoch, promotion-certifies it, derives eta0, and never
> runs out of authority (no fail-closed halt, no seed-window exhaustion).

The value: CE-4A.1 proved two boundaries (1340→1342, past the seed+3 ceiling). CE-4B proves the LITERAL
N→N+1→N+2→N+3 (three boundaries, reaching seed+5) — the node is self-sustaining well past the seed window,
end to end, in one run.

**CE-4B MAY say:** three consecutive boundaries crossed self-sufficiently in one continuous run.

**CE-4B MAY NOT say** (user-ratified boundaries):
- crash-window recovery after rollback-before-refold (that is CE-4A.3-R4, PARKED / R4c open);
- full failure-recovery closure;
- bounty recovery certification;
- live preview/preprod operation certified.

---

## 2. Corpus (extracted LOCALLY — no AWS)

The prior corpus covered 1338→1342 (ends slot 115953100). Extended via the existing `slice_chunks.py` over
the LOCAL preview ImmutableDB (`~/.cardano-ce3d-extract/db/immutable`, chunks 26841–26861, read directly via
the `.secondary` index → per-slot `.cbor`) — **2676 blocks** (115953129 → 116041708) merged into
`corpus_blocks` (backup `manifest.json.pre-1343.bak`). The corpus is now **8491 blocks, 115758773 →
116041708, continuous**, and crosses all three boundaries:

| boundary | first-slot | last-below / first-above | crossable |
|---|---|---|---|
| 1340→1341 | 115862416 | 115862325 / 115862416 | ✓ |
| 1341→1342 | 115948834 | 115948765 / 115948834 | ✓ |
| 1342→1343 | ~116035200 | 116035198 / 116035206 | ✓ |

179 blocks fall in epoch 1343 (past 116035200) — enough to cross the third boundary and confirm 1343.

---

## 3. The proof (extend the CE-4A.1 drive; production path only)

- A `drive_multi_boundary`-style run in the CE-4A mod (`node_lifecycle.rs`), reusing the CE-4A.1 setup
  (isolate, warm_start, prep-refold, fixture-lineage refresh, assemble) + `run_relay_loop_with_sched`.
- Fold POST-1340 (115778775) → **`EPOCH_1343_FIRST_SLOT`** (~116035200 + a margin into 1343) in ONE
  continuous loop call — crossing 1341, 1342, AND 1343.
- Capture the self-derived authority fingerprint (extend `capture_authority_fp` to the 1343 band).

---

## 4. Hard asserts (self-sufficiency across all three boundaries)

1. **final durable tip in epoch 1343** (crossed all three boundaries in one run).
2. **frozen leadership sealed** for the successive candidates: 1342, 1343, AND **1344**
   (`leadership_authority_for_epoch` resolves — the node keeps looking ahead past 1343).
3. **promotion-certified authority** for 1342, 1343, AND 1344 (`promotion_leadership_authority_for_epoch`).
4. **eta0 derived** at each boundary (the accumulator's epoch nonce advances 1341→1342→1343, self-derived).
5. **no fail-closed halt** — the loop completes cleanly (the node did NOT run out of authority at any
   boundary; the seed-window exhaustion that halted at seed+3 pre-S4 does NOT recur).
6. **forbidden_paths = false** (no reimport / cli_oracle / seed_window_replay / materialize_bootstrap_into).

**FAIL-LOUD** on any divergence; machine-readable `ce4b-evidence.json`. Local `#[ignore]` evidence run
(like CE-4A.1/#12/#13). **Optional strengthening (deferred):** byte-exact vs a POST-1343 cardano reference
(`dba.sh --store-ledger 116035200`) — the CE-4A.2 comparators at 1343; NOT required for the continuity claim.

---

## 5. Hard prohibitions

- no production-composition change (a needed change is its own sealed slice, as CE-4A.3-R1/R3 were);
- no seed-window replay / no materialize_bootstrap_into;
- no claim beyond §1 (no crash-window recovery, no failure-recovery closure, no bounty, no live).

---

## 6. Commit boundary

1. This authority doc (doc-before-impl).
2. Implement the 3-boundary drive.
3. Run the long proof.
4. Commit CE-4B only if green (self-sufficient across all three boundaries). Per-slice review first. The
   claim in the commit message is EXACTLY §1 — no crash-window / failure-recovery / bounty / live language.

---

## 7. Invariants

- **DC-EPOCH-19** (self-sufficiency) — the LITERAL three-boundary demonstration (CE-4A.1 was the two-boundary
  mechanical gate; this is the deferred strengthening obligation, cluster CE-4 language).
- **DC-EPOCH-25** (frozen leadership authority) — self-derived and self-sufficient across 1341/1342/1343.
- No new IDs unless a genuine production gap surfaces (as #12/#13 surfaced the recovery gaps → R1/R3).
