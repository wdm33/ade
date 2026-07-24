# CE-4A.2 — boundary outputs byte-match the cardano reference at BOTH self-derived boundaries

> **Status: PROVEN (evidence run green).** The byte-exact strengthening of CE-4A.1
> (`9c6fc3c4`, production-loop continuous self-sufficiency across two real boundaries).
> CE-4A.1 proved the two boundaries are *crossed* through the production composition; CE-4A.2
> proves the self-derived boundary *outputs* at each crossing are byte-identical to the
> cardano-node reference on **6 hard surfaces** — with `fees` reclassified as a reported
> representation-diff (Ade `epoch_fees` is a boundary-consumed reward-input accumulator; cardano
> `utxosFees` is a running residual pot; different observable quantities — see §2). Local
> `#[ignore]` evidence run; read-only extraction over two single-call production runs (§4).

**Cluster:** LIVE-LEDGER-EPOCH-TRANSITION. **Parent:** `SLICE-CE-4A-CONTINUOUS-SELF-SUFFICIENCY.md` §4.
**Depends on:** CE-4A.1 (`9c6fc3c4`), CE-3d (byte-exact at 1340), S4-pre-2 (`8cdd1471`, nesPd 658/658 at 1342).

---

## 1. The claim (exact — non-overclaiming)

> Inside the CE-4A.1 continuous production-loop run, Ade's self-derived boundary outputs at POST-1341
> and POST-1342 byte-match the cardano reference for **rewards, treasury, reserves, go snapshot,
> frozen leadership/nesPd, and authority fingerprints**.
>
> Fee economics are proven transitively through byte-exact rewards, treasury, and reserves. Raw
> fee-pot fields are reported separately because Ade's `epoch_fees` and cardano's `utxosFees`
> represent different intermediate quantities.

The value: CE-3d proved byte-exactness at ONE boundary (1340) behind the `co_advance` harness, and
only *observationally* (an `eprintln! MATCH`, not a hard assert). S4-pre-2 hard-asserted the frozen
nesPd at 1342 behind its own probe. CE-4A.2 hard-asserts byte-exactness at BOTH self-derived
boundaries, reading the PRODUCTION-loop accumulator — the freeze at boundary K and the promotion at
K+1 produce outputs that byte-match cardano at K and K+1.

**CE-4A.2 MAY say:**
- self-derived boundary outputs byte-match cardano at two consecutive boundaries (6 hard surfaces)
- byte-exactness holds through the production composition (not just an isolated harness)
- fee economics proven transitively via rewards + treasury + reserves

**CE-4A.2 MAY NOT say:**
- **fees byte-match cardano**, **raw `utxosFees` equivalence**, or **all seven surfaces byte-match**
- literal three-boundary N→N+3 proof complete (CE-4B)
- restart/rollback replay-equivalence proven (CE-4A.3)
- live preview/preprod operation proven
- bounty-ready continuous operation certified

---

## 2. Surfaces

**Hard vs-cardano asserts (a mismatch on any is a TEST FAILURE), at BOTH boundaries:**

1. **rewards** — the reward-account map produced at the boundary (90k+ accounts).
2. **treasury** — the treasury pot, post-boundary.
3. **reserves** — the reserves pot, post-boundary.
4. **go snapshot** — the rotated go stake-snapshot distribution (626 pools).
5. **frozen leadership nesPd** — the boundary-frozen `FrozenLeadershipPoolDistr` pool set,
   `(active_stake, vrf_keyhash)` per pool; **658/658 at 1342** (the DC-EPOCH-24 delegation-image
   membership, the S4-pre-2 count).
6. **authority fingerprint** — the `stake_view_canonical_hash` over the go pool-stake distribution
   (the leader-election stake commitment), computed IDENTICALLY from Ade's accumulator AND from the
   reference go, byte-equal. The raw EpochAccumulator / FrozenLeadershipPoolDistr canonical hashes
   have NO cardano counterpart (Ade-internal encodings with prev-buffers/metadata), so they are
   REPORTED as durability evidence, never asserted vs cardano.

**Reported-with-note — NOT a hard assert — `fees`.** Ade's `epoch_fees` is a boundary-consumed
reward-input accumulator (zeroed at the boundary, re-accumulated for the new epoch — `rules.rs:162`;
consumed via `total_reward = delta_r1 + epoch_fees`, `rules.rs:874`). Cardano's decoded `epoch_fees`
is `UTxOState.utxosFees`, a running live *residual* fee pot (not zeroed at the boundary). Different
observable quantities at the same instant (measured: Ade ~1–2 ADA = the new epoch's first block;
cardano ~1445–1499 ADA = the undistributed residual). **Fee economics are proven transitively** by
byte-exact rewards + treasury + reserves — if fees were mishandled those would diverge, and they are
byte-identical. The harness emits `{ade_epoch_fees, cardano_utxosFees, representation,
fee_consensus_proven_by:[rewards,treasury,reserves], hard_assert:false}`.

> **`utxosFees` compatibility risk (recorded).** If a future N2C query, persisted compatibility
> surface, or audit claim exposes cardano `LedgerState.utxosFees` as a cardano-equivalent field, Ade
> must either materialize that residual field byte-exactly or expose it through a named adapter.
> CE-4A.2 does NOT claim raw `utxosFees` equivalence — permitted internal divergence, not an
> accidental incompatibility.

**Not separately implemented — eta0.** eta0(1341)/eta0(1342) are validated *implicitly* through the
production run (epoch-1341/1342 headers pass VRF, whose input is eta0; the observed
eta0(1341)=`70ad69bd…` matches CE-4A.1's reference). A direct byte-compare was optional; the
header-validation path already binds it, so it is not separately added.

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
for 1340/1341. CE-4A.2 promotes **every hard surface (§2 items 1–6; `fees` EXCLUDED — reported-with-note)
to a fail-loud `assert!`** vs the cardano POST reference at BOTH 1341 and 1342 — turning observed matches
into a gate. This is what distinguishes CE-4A.2 from the evidence CE-3d already printed.

**Resolved capture design (`drive_capture_at`): TWO independent single-call runs.** pots/go/rewards
are current-only in `epoch_state` (overwritten at the next crossing); only the leadership nesPd is
epoch-indexed. So POST-1341's pots/rewards must be read AT the 1341 boundary. The first attempt split
one continuous fold into two `run_relay_loop_with_sched` calls over the SAME stores (capturing between
them) — but the second call RE-ENTERS the eview warm-start-across-boundary recovery at POST-1341 and
fails closed `Activate(EpochViewPostPromotionMismatch)`. That is a production EPOCH-CONSENSUS-VIEW
limitation (cf. `dabb4210`, warm-start-across-boundary), and CE-4A.2 must NOT patch production to make
a test pass. RESOLUTION — capture each boundary from its OWN single-call run:
- **POST-1341** from `drive_capture_at(max_slot = 115_862_416)` — a production run HALTED at the 1341
  boundary (a deterministic single-boundary prefix; one loop invocation, no re-entry).
- **POST-1342** from `drive_capture_at(max_slot = 115_948_834)` — the FULL continuous two-boundary run
  (the literal CE-4A.1 run; one invocation crossing 1340→1341→1342).

By S5 replay-equivalence the halted run's POST-1341 state is byte-identical to what the continuous run
passes through at 1341, so the byte-match holds for the continuous run. Each run isolates + preps +
folds + cleans up its own copy; both are pure read-only over an UNMODIFIED `run_relay_loop_with_sched`.
`drive_capture_at` mirrors `drive()`'s warm-start SELF-CONTAINED, so the proven CE-4A.1 `drive()` is
never perturbed.

---

## 5. Acceptance (CE-4A.2 is green only when ALL hold)

- Every hard surface (§2 items 1–6) byte-matches the cardano reference at **1341 AND 1342**. `fees` is
  reported-with-note (Ade `epoch_fees` ≠ cardano `utxosFees`), NOT asserted; fee economics are proven
  transitively by the byte-exact rewards + treasury + reserves.
- The comparison decodes the real `*_db-analyser` reference snapshots (§3) — not a hand-authored or
  Ade-derived stand-in for the reference.
- POST-1342 is the full continuous two-boundary run; POST-1341 is a production run halted at the 1341
  boundary (the deterministic prefix). THE HARD RULE holds — no production-composition change, no loop
  re-implementation (§4).
- **FAIL-LOUD** on: a missing/incomplete reference; any hard-surface mismatch; the nesPd count ≠ 658 at
  1342; the stake-view authority fingerprint not reproducing from the reference go.
- **Machine-readable evidence bundle** (`ce4a-2-evidence.json`) — per-surface, per-boundary match
  booleans, the raw fee values + representation note, and the `utxosFees` compatibility note:
  ```json
  {
    "slice": "CE-4A.2",
    "claim": "inside the CE-4A.1 continuous production-loop run, ... byte-match ... for rewards, treasury, reserves, go snapshot, frozen leadership/nesPd, and authority fingerprints",
    "hard_asserts": ["rewards","treasury","reserves","go","nesPd","authority_fingerprint_stake_view_hash"],
    "boundaries": {
      "1341": { "reward": true, "pots": {"treasury": true, "reserves": true}, "go": true,
                "nesPd": true, "nesPd_count": [658,658], "authority_fingerprint_stake_view_hash": true,
                "fees": {"ade_epoch_fees": 1676268, "cardano_utxosFees": 1445011078,
                         "hard_assert": false}, "ref": ".../115862416_db-analyser/state" },
      "1342": { "...": "...", "nesPd_count": [658,658] }
    },
    "utxos_fees_compatibility_note": "... Ade must materialize utxosFees byte-exactly or expose it via a named adapter; CE-4A.2 does NOT claim raw utxosFees equivalence",
    "hard_rule_no_loop_reimpl": true
  }
  ```

---

## 6. Invariants (byte-exact EVIDENCE — not a registry status flip)

CE-4A.2 is a local `#[ignore]` evidence run (§3), so it does NOT by itself flip any registry status to
CI-enforced or append `strengthened_in`. It provides byte-exact *evidence*, through the production loop,
for four already-tracked invariants:

- **DC-EPOCH-23** (bootstrap fee/pot reduction, `enforced`) — treasury + reserves byte-match cardano at
  1341 AND 1342 (promoted from CE-3d's observed `MATCH` to a fail-loud `assert!`); the fee ECONOMICS are
  proven transitively via rewards, while the raw `utxosFees` field is a representation-diff (reported,
  not asserted — Ade `epoch_fees` ≠ cardano `utxosFees`).
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
