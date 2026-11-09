# S4 — the sealed authority flip: accumulator-derived consensus view replaces the seed-window read

> **Status: OPEN.** Both admissibility preconditions (§3) are GREEN and committed: CE-3d byte-exact
> (`e476415a`) and S5 restart/controlled-rollback replay-equivalence (`8d6bf874` 2b + `687fea98` 2c). This is
> the enforcing consumer that flips DC-EPOCH-19/20/21/22 (and DC-EPOCH-17 / ECA-B3) from `declared` to
> `enforced`. Declares nothing new.

**Cluster:** LIVE-LEDGER-EPOCH-TRANSITION. **Depends on:** S3 CE-3d (byte-exact boundary differential) +
S5 (restart/rollback replay equivalence) — both GREEN are the precondition, not a confidence call.

---

## 1. The one-line contract

For any selected-chain prefix, there is **exactly one** leadership/header-validation authority, and it is
derived **only** from the maintained `EpochAccumulator` (+ the reduced UTxO checkpoint) at that prefix — never
from the seed-anchored EVIEW window replay.

```
authority_for(prefix) = PoolDistrView {
    epoch,
    per-pool active_stake   ← accumulator.go.pool_stakes            (byte-exact, CE-3d)
    per-pool vrf_keyhash    ← accumulator.cert_state.pool.pools     (boundary-frozen, SNAP-before-POOLREAP)
    active_slots_coeff      ← the bound consensus profile           (no unbound param read)
}
```

The accumulator is currently **observe-only** (S2/S3): consensus reads
`PoolDistrView::from_seed_epoch_consensus_inputs` (node_lifecycle.rs:596/777/3133, node_sync.rs:3547). S4
makes the accumulator the authority and retires those reads.

## 2. Prohibited (this is the load-bearing part, from the single-authority + replay laws)

- **No fallback.** No "prefer accumulator / otherwise seed" resolution. If the accumulator cannot answer for
  a prefix, that is a fail-closed terminal, never a silent seed read.
- **No dual mode.** No two code paths that can each produce a leader authority. One source, one path.
- **No feature gate.** No build/env/CLI flag decides *whether* the accumulator is authority — that would be a
  semantic activation gate ([[feedback_no_semantic_activation_gate]]). The flip is unconditional in the slice.
- **Deleted in the SAME sealed slice:** the seed-window read surface (`from_seed_epoch_consensus_inputs` on
  the authority path) **and** the seed+2 ceiling (`epoch_wire.rs:626-627`
  `WindowReplayPrepare("window-replay beyond seed+2 not yet wired")`). They do not coexist with the new
  authority for even one commit.

## 3. Hard preconditions — the promotion contract (ALL green before S4 opens)

S4 is admissible ONLY when the self-derived boundary state is proven byte-identical to cardano — this is
S4's admissibility proof, not a pre-S4 test:

1. **CE-3d byte-exact at ≥2 self-derived boundaries** — RUPD, pots (reserves/treasury), reward distribution
   (per reward account), mark/set/go rotation, and pool/VRF leader inputs all equal cardano-node's
   `db-analyser` reference. **Zero residuals — including any B3c-class stake discrepancy — not "small
   enough."** A non-zero residual is the next invariant, not a tolerance.
2. **S5 replay equivalence** — warm restart in each epoch phase + one controlled rollback re-derive the
   IDENTICAL accumulator and the IDENTICAL authority from durable state.

## 4. Acceptance tests (S4's own gate, once unblocked)

- **Single authority:** both `run_node_sync` header validation AND the forge/leadership wall resolve the
  accumulator-derived `PoolDistrView`; a CI grep-gate asserts `from_seed_epoch_consensus_inputs` no longer
  appears on the authority path.
- **Ceiling removed:** a CI grep-gate asserts the seed+2 `WindowReplayPrepare` fail-close is gone; a live/
  hermetic proof crosses **past seed+2** (seed+3 and beyond) with no `rc=43`.
- **No dual path:** a grep-gate asserts no "prefer/otherwise"/fallback branch between accumulator and seed.
- **Replay-equivalence:** the resolved authority is byte-reproducible from the durable accumulator + WAL
  (the recovery re-fold), across restart and rollback.
- **Same-epoch byte-identical:** within an already-followed epoch, the accumulator-derived authority equals
  what the seed view produced (no behavior change except the source), proven on the existing corpus.

## 5. IDD classification (fixed up front)

- **True (unconditional):** one authoritative, replay-equivalent selected-chain state; no alternate semantic
  path; exactly one leader-authority source per prefix.
- **Derived (Cardano-compatible):** RUPD, snapshot rotation mark→set→go, per-pool stake/VRF leader inputs,
  and the promoted epoch authority — all reproduced from self-maintained state.
- **Release / evidence:** CE-3d byte-exact differential (§3.1) + S5 crash/warm-restart equivalence (§3.2).
- **Operational (separate track, does not gate S4's semantics):** opportunistic BA-02 capture on a public
  peer; the 10-day BA-08 memory certification run (which S4 makes possible by crossing epochs unattended).

## 6. What S4 does NOT do

No new invariant IDs. No governance-coverage expansion (unsupported ratified kinds stay the safe fail-closed
terminal `UnsupportedRatifiedAction` — T-EPOCH-01 partial — until a separate slice). No operational
reconnect/forge gates (those ride alongside S6). S4 is the narrow, sealed flip and nothing else.

## 7. Execution plan (the mergeable units)

The exactly-these scope items:
1. Replace all PRODUCTION `PoolDistrView::from_seed_epoch_consensus_inputs` leadership/stake reads with
   accumulator-derived authority. Production sites (file-verified): `node_lifecycle.rs` 658, 840, 3397;
   `node_sync.rs` 3556. Every OTHER hit is a test / oracle / codec-def site and stays.
2. Route authority through a narrow `PoolDistrView::from_accumulator(acc, profile)` (or
   `EpochConsensusView::from_accumulator` → `to_pool_distr_view`): stake ← `go.pool_stakes`, per-pool
   `vrf_keyhash` ← the accumulator's boundary-frozen pool params (SNAP-before-POOLREAP), asc ← the bound
   consensus profile. Fail-closed if not leadership-complete (a staked pool with no VRF keyhash).
3. Delete the seed+2 fail-close ceiling (`epoch_wire.rs:627` `"window-replay beyond seed+2 not yet wired"`).
4. Preserve seed-derived reads ONLY in tests/oracle comparison paths, explicitly named non-authoritative.
5. Add a CI/static grep-gate so production code cannot reintroduce a seed-window leadership read or a
   prefer-accumulator-else-seed branch.

**Commit shape (either is acceptable; NEVER an intermediate state with two production authority paths):**
- **(A) one sealed commit** — `from_accumulator` + swap all 4 sites + ceiling delete + guard + proof; OR
- **(B) two commits** — (1) static guard + `from_accumulator` + call-site preparation, NO behavior change;
  (2) authority flip + ceiling delete + proof. Every intermediate commit must remain correct (no dual
  authority, no fallback).

**S4's own gate (once flipped):** §4 acceptance tests — single authority (grep-gate: no production
`from_seed_epoch_consensus_inputs` on the authority path); ceiling removed (grep-gate + a hermetic cross PAST
seed+2 with no `rc=43`); no dual/fallback path (grep-gate); replay-equivalence (the resolved authority is
byte-reproducible from the durable accumulator across restart + rollback — reuses the S5 2c differential);
same-epoch byte-identical (the accumulator-derived authority equals the seed view within an already-followed
epoch, source-only change). **Negative:** removing/corrupting accumulator state fails closed; seed authority
cannot silently resume control.
