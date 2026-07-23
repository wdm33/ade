# CE-4A.2 — boundary outputs byte-match the cardano reference at BOTH self-derived boundaries

> **Status: OPEN (scoped, doc-before-impl).** The byte-exact strengthening of CE-4A.1
> (`9c6fc3c4`, production-loop continuous self-sufficiency across two real boundaries).
> CE-4A.1 proved the two boundaries are *crossed* through the production composition; CE-4A.2
> proves the self-derived boundary *outputs* at each crossing are byte-identical to the
> cardano-node reference. Same harness, same continuous run — read-only extraction added.

**Cluster:** LIVE-LEDGER-EPOCH-TRANSITION. **Parent:** `SLICE-CE-4A-CONTINUOUS-SELF-SUFFICIENCY.md` §4.
**Depends on:** CE-4A.1 (`9c6fc3c4`), CE-3d (byte-exact at 1340), S4-pre-2 (`8cdd1471`, nesPd 658/658 at 1342).

---

## 1. The claim (exact — non-overclaiming)

> Within the CE-4A.1 continuous run, Ade's self-derived boundary outputs at 1341 and 1342
> byte-match the cardano-node reference.

The value: CE-3d proved byte-exactness at ONE boundary (1340) behind the `co_advance` harness.
S4-pre-2 proved the frozen nesPd at 1342 behind its own probe. CE-4A.2 proves byte-exactness at
BOTH self-derived boundaries **inside the single production continuous run** — the freeze at
boundary K and the promotion at boundary K+1 produce outputs that byte-match cardano at K and K+1.

**CE-4A.2 MAY say:**
- self-derived boundary outputs byte-match cardano at two consecutive boundaries
- byte-exactness holds through the production continuous run (not just an isolated harness)

**CE-4A.2 MAY NOT say:**
- literal three-boundary N→N+3 proof complete (CE-4B)
- restart/rollback replay-equivalence proven (CE-4A.3)
- live preview/preprod operation proven
- bounty-ready continuous operation certified

---

## 2. Surfaces (byte-match required at BOTH 1341 AND 1342)

Mandatory (a mismatch on any is a TEST FAILURE):

1. **rewards** — the reward update / reward-account map produced at the boundary.
2. **pots** — treasury, reserves, and fees (the three pot components), post-boundary.
3. **go snapshot** — the rotated go stake-snapshot distribution.
4. **frozen leadership nesPd** — the boundary-frozen `FrozenLeadershipPoolDistr` pool set,
   `(active_stake, vrf_keyhash)` per pool; **658/658 at 1342** (the DC-EPOCH-24 delegation-image
   membership, the S4-pre-2 count).
5. **authority fingerprints** — the EpochAccumulator canonical hash and the FrozenLeadershipPoolDistr
   canonical hash (the durable authority commitments), each byte-equal to the value derived from the
   reference snapshot.

Optional (include if extractable without new machinery):

6. **eta0 / nonce transcript** — the Praos `epoch_nonce` (eta0) at each boundary vs the reference
   `praos_nonces.epoch`. eta0(1341) is already validated *implicitly* in CE-4A.1 (epoch-1341 headers
   pass VRF, whose input is eta0); CE-4A.2 promotes it to a DIRECT byte-compare and adds eta0(1342).

---

## 3. Reference data (CONFIRMED present — the feasibility gate)

All three POST-boundary references exist as cardano-node LedgerDB `*_db-analyser` snapshots
(`meta` / `tables` / `state`), produced by `dba.sh --store-ledger <firstSlotOfEpoch>`
(`~/.cardano-ce3d-extract/extract_refs.sh`):

| Boundary | requested slot | stored snapshot | location |
|---|---|---|---|
| POST-1340 | 115776011 | `115776011_db-analyser` | `~/.cardano-ce3d-extract/db/ledger/` (CE-3d ref) |
| **POST-1341** | 115862400 | `115862416_db-analyser` | `~/.cardano-ce3d-extract/db/ledger/` (+ `ref_1341.tar.gz`) |
| **POST-1342** | 115948800 | `115948834_db-analyser` | `~/.cardano-ce3d-extract/db/ledger/` |

Both boundary logs confirm `Snapshot stored` (`ref_1341.log`, `ref_1342.log`). The `state` blob under
each `*_db-analyser/` is decoded by `ade_ledger::ledgerdb_state::decode_native_nonutxo_state`
(`ledgerdb_state.rs:1573`) — the exact decoder CE-3d and S4-pre-2 already use for the 1340/1342
comparisons. The existing tests locate them by env var: `CE3D_REF_1341` → `…/115862416_db-analyser/state`,
`CE3D_REF_1342` → `…/115948834_db-analyser/state` (base dir `CE3D_REF`).

**These references are LOCAL, uncommitted extraction artifacts — NOT committed fixtures** (the ~400 MB
`state`/`tables` blobs are not in git; the existing differential tests are `#[ignore]`'d for exactly
this reason). Like CE-4A.1, CE-4A.2 is therefore an `#[ignore]`, local-evidence run (fail-loud,
machine-readable bundle), **NOT a CI-gating test.** Capturing the three `state` blobs as committed
fixtures so the comparison can gate CI is a SEPARATE obligation (§6) — explicitly out of CE-4A.2 scope.

**FAIL-LOUD** if either reference snapshot is missing or incomplete — this is an `#[ignore]`
fixture-heavy evidence run; it must never silently skip and appear green.

---

## 4. Design — extend the CE-4A.1 harness, read-only (THE HARD RULE holds)

CE-4A.2 adds NO change to the production composition. It extends the existing
`#[cfg(test)] mod ce4a_continuous_self_sufficiency` in `crates/ade_node/src/node_lifecycle.rs`:

- The CE-4A.1 `drive()` already ends the continuous run holding live handles to the self-derived
  state: `epoch_accumulator` (EpochAccumulatorStore), `reduced_checkpoint` (ReducedUtxoCheckpoint),
  `acc.epoch_state`, and `chain_dep.epoch_nonce`.
- **Leadership + go are epoch-indexed**, so BOTH boundaries are reachable post-run without
  re-running: `leadership_authority_for_epoch(EpochNo(1341))` **and** `(EpochNo(1342))` both resolve.
- The reference side reuses machinery that already exists. The **authority functions are public
  `ade_ledger` lib** (reusable cross-crate from the ade_node test module):
  - `ledgerdb_state::decode_native_nonutxo_state` (`:1573`) → `NativeSnapshotNonUtxoState`
    (rewards = `cert_state.delegation.rewards`; pots = `treasury`/`reserves`/`epoch_fees`; go =
    `snapshots.go`; nesPd = `pool_distr`, the literal `nes[5]` incl. zero-stake + retired).
  - `frozen_leadership::canonical_hash` (`:259`) + `epoch_accumulator::encode_epoch_accumulator`
    (`:1771`) + `reduced_epoch_view::stake_view_canonical_hash` (`:250`) — the authority fingerprints.
  - The thin projection/diff helpers (`ref_post_state`, `ade_post_state`, `compare`, `diff_map`,
    `ref_nes_pd`, `ade_leadership_map`) live in `crates/ade_testkit/tests/ce3d_boundary_differential.rs`
    (`:198`–`:367`). They are short projections over the public decoder; CE-4A.2 re-types the ~5 it
    needs in the ade_node CE-4A module (the authority is the lib function, not the test helper).
- **THE HARD-RULE DISTINCTION from the existing `ce3d_boundary_differential_1341_1342` (`:521`):** that
  test already crosses 1340→1341→1342 and compares vs the cardano refs — but through the `co_advance`
  differential harness, NOT the production loop. CE-4A.2's `ade_post_state` reads the **CE-4A.1
  PRODUCTION-loop `epoch_accumulator`** (the state the real `run_relay_loop_with_sched` produced). That
  is the whole point — byte-exactness of what the *production composition* derived, not a re-run of the
  differential harness.

**Gate-adds-value (the real work, not a trivial re-run).** The existing vs-cardano comparison for
rewards/pots/go is **observational only** — `ce3d_boundary_differential_1341_1342` `eprintln!`s
`MATCH`/`MISMATCH` (`:562`/`:584`); it does NOT `assert!` byte-equality vs cardano. Only **nesPd**
hard-asserts vs the reference (`:1717 assert_eq!`), and the `crae_*` reward-map tests pin report hashes
for 1340/1341. CE-4A.2 must promote **every mandatory surface (§2 1–5) to a fail-loud `assert!`** vs the
cardano POST reference at BOTH 1341 and 1342 — turning observed matches into a gate. This is what
distinguishes CE-4A.2 from the evidence CE-3d already printed.

**Open implementation question (resolve at implement-time, does NOT change the claim):**
rewards/pots for the 1341 boundary must be captured for comparison. Leadership, go, and the
authority fingerprints are epoch-indexed and survive to end-of-run. IF the reduced-checkpoint /
`epoch_state` rewards+pots retain per-epoch history to end-of-run, extract both post-run (one run,
simplest). IF only the latest boundary's rewards/pots survive, capture the 1341 output via EITHER a
second `drive(max_slot = EPOCH_1342_FIRST_SLOT)` that stops at the 1341 boundary, OR a read-only
snapshot of the durable stores at the 1341 crossing. **Both permitted options are read-only and MUST
NOT re-implement or hook into `run_relay_loop_with_sched` — a snapshot reads the durable stores, it
does not alter the loop.** Prefer the single-run post-run extraction if the state is retained.

---

## 5. Acceptance (CE-4A.2 is green only when ALL hold)

- Every mandatory surface (§2 items 1–5) byte-matches the cardano reference at **1341 AND 1342**.
- The comparison decodes the real `*_db-analyser` reference snapshots (§3) — not a hand-authored or
  Ade-derived stand-in for the reference.
- The run is the CE-4A.1 production continuous run (or a read-only extension of it); THE HARD RULE
  holds — no production-composition change, no loop re-implementation.
- **FAIL-LOUD** on: a missing/incomplete reference; any surface mismatch; the nesPd count ≠ 658 at
  1342; an authority fingerprint that does not reproduce from the reference.
- **Machine-readable evidence bundle** (`ce4a-2-evidence.json`) — per-surface, per-boundary match
  booleans plus the reference snapshot paths and the compared canonical hashes:
  ```json
  {
    "slice": "CE-4A.2",
    "claim": "self-derived boundary outputs byte-match cardano at 1341 and 1342",
    "boundaries": {
      "1341": { "reward": true, "pots": true, "go": true, "nesPd": true, "nesPd_count": [N,N],
                "acc_fp": true, "leadership_fp": true, "eta0": true, "ref": ".../115862416_db-analyser" },
      "1342": { "reward": true, "pots": true, "go": true, "nesPd": true, "nesPd_count": [658,658],
                "acc_fp": true, "leadership_fp": true, "eta0": true, "ref": ".../115948834_db-analyser" }
    },
    "hard_rule_no_loop_reimpl": true
  }
  ```

---

## 6. Invariants (byte-exact EVIDENCE — not a registry status flip)

CE-4A.2 is a local `#[ignore]` evidence run (§3), so it does NOT by itself flip any registry status to
CI-enforced or append `strengthened_in`. It provides byte-exact *evidence*, through the production loop,
for four already-tracked invariants:

- **DC-EPOCH-23** (bootstrap fee/pot reduction, `enforced`) — pots/fees byte-match cardano at 1341 AND
  1342, promoted from CE-3d's observed `MATCH` to a fail-loud `assert!`.
- **DC-EPOCH-24** (snapshot pool-set / go membership, `enforced`) — go + nesPd membership byte-match at
  1341 and 1342 (658/658 at 1342), reusing the S4-pre-2 hard-assert.
- **DC-EPOCH-25** (frozen leadership authority, `declared`) — exercised on the byte-exact axis at two
  self-derived boundaries through the production promotion path.
- **DC-EPOCH-19** (self-sufficiency, `declared`) — CE-4A.1 evidenced "crossed"; CE-4A.2 evidences
  "byte-exact at the crossing" at the mechanical scope.

**Follow-on obligation (out of CE-4A.2 scope, tracked here):** to convert this evidence into a standing
CI gate — and to justify a `strengthened_in` bump / a `declared → enforced` flip for DC-EPOCH-19/25 —
the three POST `state` blobs must be captured as committed fixtures and the byte-exact comparison wired
as a CI check. That is a deliberate, separate slice (large-binary-fixture policy + CI wiring), not a
side effect of CE-4A.2.

If a mandatory surface has no existing comparator (e.g. a pot component not covered by the CE-3d
`compare`), that gap is recorded and its comparator added WITHIN this slice — no silent omission.

---

## 7. Commit boundary

CE-4A.2 commits as its own slice. The commit message claim stays exactly §1 — no literal N→N+3, no
live, no bounty language, no restart/rollback (that is CE-4A.3). Next after CE-4A.2: **CE-4A.3**
(restart + rollback inside the production-loop harness), then decide whether **CE-4B** needs a 1343
extraction for the literal three-boundary run.
