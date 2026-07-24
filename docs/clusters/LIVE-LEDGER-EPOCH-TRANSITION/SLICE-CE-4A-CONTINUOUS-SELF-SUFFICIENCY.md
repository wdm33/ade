# CE-4A — mechanical continuous self-sufficiency: two consecutive real boundaries through the production run-loop

> **Status: OPEN (scoped, doc-before-impl).** Delivers the LIVE-LEDGER-EPOCH-TRANSITION cluster's CE-4
> (self-sufficiency, DC-EPOCH-19) as a MECHANICAL gate on the existing 1340→1342 corpus. Admissible now
> because S4-L2 (`db702a54`, the sealed authority flip) removed the last seed-window leadership dependency.
>
> **CE-4 is delivered in two layers:**
> - **CE-4A (this doc) — mechanical continuous self-sufficiency.** Ade crosses TWO consecutive real epoch
>   boundaries past the former seed-window ceiling through the PRODUCTION run-loop composition, with no
>   re-import, no CLI oracle, no seed-window leadership authority, and no materialize/window-replay
>   promotion. Corpus: 1340→1341 (seed+3, the former ceiling) → 1342 (seed+4, recurrence beyond it).
> - **CE-4B (deferred strengthening, §6) — literal extended continuity.** Ade crosses AT LEAST THREE
>   consecutive boundaries in one continuous run (the literal N→N+1→N+2→N+3). Requires a 1343 extraction or
>   a confirmed continuous 4-boundary earlier range. A strengthening obligation; it does NOT gate CE-4A.
>
> **Wording (load-bearing): this doc's mechanical gate is CE-4A, NOT the literal three-boundary N→N+3 in
> `cluster.md` §6 CE-4. CE-4A does not claim N→N+3 closure.**

**Cluster:** LIVE-LEDGER-EPOCH-TRANSITION. **Depends on:** S4-L2 (`db702a54`) + S5 + CE-3d (all GREEN).

---

## 1. The claim (exact — non-overclaiming)

CE-4A proves, hermetically over the existing 1340→1342 corpus:

> Ade crosses two consecutive real epoch boundaries past the former seed-window ceiling through the
> production run-loop composition, with no re-import, no CLI oracle, no seed-window leadership authority,
> and no materialize/window-replay promotion.

The value: it proves the exact loop S4 was meant to unlock —
**freeze at boundary K → persist frozen leadership → later promote from that frozen authority →
validate/admit the next epoch → repeat once more.**

**CE-4A MAY say:**
- continuous two-boundary self-derived operation proven
- former ceiling crossed
- recurrence beyond the former ceiling proven
- production composition exercised

**CE-4A MAY NOT say:**
- literal three-boundary N→N+3 proof complete
- live sustained operation proven
- bounty-ready continuous operation certified

---

## 2. Why the pieces are not enough (the gap CE-4A closes)

Every constituent is already proven, but only in isolation, each behind its own harness:
- accumulator freeze + byte-exact rewards/go/pot + recovery equivalence → `co_advance` (S5 / ce3d).
- promotion (`prepare_authority_for_candidate_slot` crossing seed+2/seed+3) → the S4-L2 unit test
  (synthetic store).

**Neither runs the PRODUCTION composition.** CE-4A closes that: one continuous pass of the real relay-loop
composition, folding the corpus across two boundaries, so the freeze at boundary K feeds the promotion at
boundary K — self-derived, end to end.

---

## 3. The production composition (what the harness MUST drive)

The relay loop composes two production entry points per sync pass (node_lifecycle relay loop):
- **`advance_ledger_state_to_durable_tip`** — folds admitted selected blocks into the EpochAccumulator + the
  reduced checkpoint; at a boundary, binds the mark at `s_prev`, finalizes the checkpoint commitment, and
  `cross_accumulator_over_boundary_block` freezes `nesPd_{target+1}` (the S4-pre-2 boundary freeze).
- **`run_node_sync` / the pump → `prepare_authority_for_candidate_slot`** — admits blocks, and at a boundary
  PROMOTES the candidate authority from `promotion_leadership_authority_for_epoch(candidate)` (the S4-L2
  frozen promotion, `candidate ≥ seed+2`).

CE-4A.1 feeds the corpus as a `NodeBlockSource` into THIS composition. It does not re-implement the loop.

---

## 4. Acceptance (CE-4A is green only when ALL hold)

### CE-4A.1 — real production-loop composition over corpus 1340→1342
**Locked claim:** *CE-4A.1 proves production-loop continuous self-sufficiency across two real boundaries.* It
does NOT claim byte-exact boundary equivalence, restart/rollback equivalence, live preview/preprod operation,
or bounty readiness.

- The harness drives `run_relay_loop_with_sched` + `advance_ledger_state_to_durable_tip` (the production entry
  points), NOT a re-composition. A `NodeBlockSource::InMemory` over the corpus blocks feeds the real loop. The
  setup is the production warm-start (`warm_start_recovery` + the production input assembly), not a hand-built
  state.
- **Critical guard:** `run_relay_loop_with_sched` is invoked with ALL THREE authority inputs present —
  `Some(reduced_checkpoint)`, `Some(eview_activation)`, `Some(epoch_accumulator)` — asserted. A green run with
  any of them `None` does NOT count for CE-4A.
- Both boundaries (1340→1341, 1341→1342) are crossed IN ONE RUN via the frozen freeze → promotion.
- Every block in the final epoch (1342) validates against the self-derived authority.
- No re-import, no CLI oracle, no seed-window leadership read, no materialize/window-replay promotion (the
  S4-L2 resurrection guard holds over the run).

**FAIL-LOUD (this is an `#[ignore]`, fixture-heavy evidence run — it must NEVER silently skip and appear
green).** Each of the following is a TEST FAILURE, not a skip:
- the v5/v6 fixture missing or incomplete (chain.db / epoch-accumulator.redb / reduced-checkpoint.redb / wal)
- the corpus missing
- any of the three authority inputs not `Some(...)`
- an expected boundary not crossed
- an expected frozen-leadership target epoch not sealed
- an expected promotion WAL record absent
- any forbidden-path marker present

**Machine-readable evidence bundle** — the test emits this JSON (auditable, not buried in logs); a green run
means every field below holds:
```json
{
  "slice": "CE-4A.1",
  "claim": "production-loop continuous self-sufficiency across two real boundaries",
  "start_epoch": 1340,
  "crossed_boundaries": ["1340->1341", "1341->1342"],
  "frozen_leadership_targets": [1342, 1343],
  "promotion_source": "FrozenLeadershipPoolDistr",
  "promotion_certified": true,
  "authority_inputs_present": {
    "reduced_checkpoint": true, "eview_activation": true, "epoch_accumulator": true
  },
  "forbidden_paths": {
    "reimport": false, "cli_oracle": false, "seed_window_replay": false, "materialize_bootstrap_into": false
  }
}
```

### CE-4A.2 — boundary outputs byte-match the cardano reference at BOTH boundaries
> **Slice doc (scoped, doc-before-impl):** `SLICE-CE-4A-2-BOUNDARY-BYTE-EXACT.md`.

At 1341 AND 1342, the self-derived boundary output byte-matches the cardano reference:
- rewards
- pots (treasury / reserves / fees)
- go snapshot
- frozen leadership nesPd (658/658 at 1342)
- authority fingerprints (the accumulator + leadership canonical hashes)

### CE-4A.3 — restart + rollback INSIDE the production-loop harness
> **Slice doc (scoped, doc-before-impl):** `SLICE-CE-4A-3-RESTART-ROLLBACK.md`. Resolves the
> `EpochViewPostPromotionMismatch` finding as first-class (harness-only re-entry artifact vs. a sealed
> production fix).

- A warm restart mid-run + one controlled within-k rollback + refold, driven through the SAME production
  composition (NOT `co_advance`).
- The recovered state (accumulator + checkpoint + leadership authority) is BYTE-IDENTICAL to the
  uninterrupted run.

**THE HARD RULE: if the harness bypasses the production composition, CE-4A does not count.**

---

## 5. Invariants (enforcement, no new IDs unless a gap is found)

- **DC-EPOCH-19** (self-sufficiency) `declared` → `enforced` at the CE-4A mechanical scope (two boundaries).
- **DC-EPOCH-17** (continuous crossing past seed+2) confirmed by the two-boundary continuous run.
- **DC-EPOCH-25** (frozen leadership authority) exercised on the PRODUCTION promotion path in a continuous
  run (not just the synthetic unit test).
- The S4-L2 resurrection guard `ci/ci_check_frozen_promotion_no_seed_window.sh` holds over the run.

---

## 6. CE-4B — deferred strengthening obligation (does NOT gate CE-4A)

Extract a 1343 boundary (blocks + POST-1343 reference) OR confirm a continuous 4-boundary earlier range
(candidate: the ~1087–1091 reference range — corpus continuity unconfirmed), then prove AT LEAST THREE
consecutive boundaries in one continuous run — the literal N→N+1→N+2→N+3 language of `cluster.md` CE-4.
Tracked as an open obligation; it is a strengthening, not a blocker for CE-4A.

---

## 7. Commit boundary

CE-4A commits per-slice (4A.1, 4A.2, 4A.3) as each seals, or as one CE-4A commit if they land together. The
claim in every commit message stays exactly §1 — no literal-N→N+3, no live, no bounty language.
