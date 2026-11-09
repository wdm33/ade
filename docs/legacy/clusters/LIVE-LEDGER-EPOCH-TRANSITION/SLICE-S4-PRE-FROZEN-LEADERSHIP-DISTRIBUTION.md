# S4-pre — Frozen Leadership Distribution Authority

> **Status: OPEN. Prerequisite for S4.** Persist + replay the EXACT cardano leadership `PoolDistr` authority
> that Praos leader checks / forging consume, without relying on the seed-window EVIEW after bootstrap. A
> DERIVED cardano-compatibility invariant supporting the replay law: observable behaviour must match the
> reference node; the internal state shape may differ (Haskell node = compatibility oracle, not architecture
> template). S4 (the narrow flip) stays BLOCKED until S4-pre is green.

**Why (proven, `67890681`):** the Leadership Distribution Authority Trace proved leadership uses **SET-derived
stake + snapshot-FROZEN pool params/VRF**, including zero-stake registered pools AND the retired/POOLREAP'd 1M
ADA pool whose VRF is absent from active params. "Derive leadership later from go + active params" is a
DISPROVEN hypothesis (`from_accumulator_go_active_params_for_test_only`), not an optimization target.

## 1. The self-contained authority object (do NOT derive at use time)

```
FrozenLeadershipPoolDistr {
    epoch,
    source_boundary,
    pools: PoolKeyHash -> LeadershipPoolEntry { active_stake, vrf_keyhash },
}
```

`from_frozen_leadership(acc, asc) -> PoolDistrView` reads THIS object and nothing else. Leadership
reconstruction must NOT depend on `cert_state.pool.pools` (active), the go snapshot, `future_pools`, the
`retiring` map, or any retired-pool supplement at use time. Those are freeze-time SOURCE inputs, never runtime
leadership authority.

## 2. Bootstrap import

At bootstrap, import the frozen leadership distribution from a NAMED, manifest-bound artifact: `epoch`,
source point/boundary, pool count, `pool_id -> active_stake -> vrf_keyhash`, canonical hash, reference
provenance. **Missing / malformed leadership data = terminal typed failure** — no empty default, no inferred
fallback. Old v5/schema-v4 stores WITHOUT this field are NOT leadership-certified: re-bootstrap or explicit
re-materialization is required before S4 can use them.

## 3. Boundary freezing

At each cardano leadership freeze point, freeze the NEXT leadership distribution from the snapshot-frozen
stake + snapshot-frozen pool params/VRF. **No active-param lookup at leadership-use time** — the lesson from
`67890681` is that active params can forget a pool that stays leadership-relevant.

## 4. Recovery extension (S5 must cover this new surface)

S5 recovery equivalence must extend to this authority: clean advance, warm restart, within-k rollback, reset
+ refold — all reproduce the same frozen leadership distribution hash + `PoolDistrView`. The replay law
governs consensus outcomes, state roots, WAL, checkpoints, receipts, routing — byte-identical outputs from
the same canonical inputs.

## 5. Acceptance (S4-pre is green only when ALL hold)

- bootstrap frozen leadership distr == the seed leadership `PoolDistr`: **659/659 pools**, stake exact, VRF
  exact, zero-stake pools included exactly where cardano includes them, the retired 1M ADA pool included with
  its frozen VRF, canonical bytes exact;
- warm restart exact; within-k rollback/refold exact;
- missing frozen leadership fails closed; malformed frozen leadership fails closed;
- an old store cannot silently become leadership authority;
- CI/static guard: `from_accumulator_go_active_params_for_test_only` stays test/oracle/negative-regression
  only (never a production leadership path).

## 6. Slice shape

- **S4-pre-1** — types, canonical encoding, store schema, bootstrap import, seed identity test.
- **S4-pre-2** — boundary freeze from snapshot-frozen stake + params/VRF.
- **S4-pre-3** — re-bootstrap the v5 lineage + recovery replay evidence + registry flip.

Do NOT touch the three production seed-window call sites yet. Do NOT delete the seed+2 ceiling yet. That is
S4 proper, which resumes only after S4-pre proves `from_frozen_leadership(accumulator) == seed leadership
PoolDistr`.

## 7. Progress

**S4-pre-1a — the type + seed identity (DONE, `501bf89a`).** `ade_ledger::frozen_leadership`:
`FrozenLeadershipPoolDistr { epoch, source_slot, source_hash, pools: Hash28 -> LeadershipPoolEntry {
active_stake, vrf_keyhash } }` + `to_pool_distr_view(asc)` (reads THIS object only) +
`from_seed_epoch_consensus_inputs` (the bootstrap import from the manifest-bound seed record). SEED IDENTITY
PROVEN (`ce3d_boundary_differential::s4pre_frozen_leadership_seed_identity`): the frozen distr projects
BYTE-EXACT to `from_seed` — 659/659, incl. the zero-stake registered pools + the retired 1M-ADA pool's frozen
VRF, with no go/active-param/retiring lookup.

**S4-pre-1b — the durable schema/import authority (DONE).** Narrowly the durable leadership-authority schema
slice (not the re-bootstrap/evidence slice, which is 1c). Shipped:
- **Canonical codec** (`frozen_leadership.rs`): `encode_frozen_leadership` / `decode_frozen_leadership`
  (`array(5)[version, epoch, source_slot, source_hash, map{ pool_keyhash -> array(2)[stake, vrf] }]`,
  `FROZEN_LEADERSHIP_SCHEMA_VERSION = 5`) + `canonical_hash` (blake2b-256). Fail-closed `FrozenLeadershipError`:
  unknown version, structural, duplicate/unsorted pool keys, field overflow, trailing bytes, non-canonical
  bytes (re-encode ≠ input). Zero-stake pools preserved.
- **Durable persistence** (`epoch_accumulator_store.rs`): `seal_frozen_leadership` (blob + a store-level
  leadership-schema-v5 marker in ONE atomic redb commit), `frozen_leadership` (raw accessor), and the
  fail-closed `leadership_authority` read. The accumulator BLOB codec is UNCHANGED (still v4-decodable) — a
  legacy v4 / pre-S4-pre store fails closed as `OldAccumulatorSchemaNotLeadershipCertified` on the leadership
  path while non-authority observe-only follow still decodes its accumulator blob. `MissingFrozenLeadershipDistr`
  / `MalformedFrozenLeadershipDistr` cover a torn / corrupt certified store. `reset_to_bootstrap` DELIBERATELY
  preserves the frozen leadership (epoch-frozen; a within-epoch reorg does not change `nesPd`).
- **Bootstrap import**: `seal_frozen_leadership_from_seed_record` — source binding (`FrozenLeadershipSourceMismatch`
  if the record's frozen point ≠ the expected bootstrap point) + an encode→decode canonical self-check
  (`FrozenLeadershipCanonicalDecodeFailed`) before it seals.
- **Tests**: 7 codec (round-trip, stable + content-bound hash, zero-stake preserved, wrong-version / duplicate /
  unsorted / trailing rejected) + 8 store (seal/read round-trip, legacy-store fail-closed, reopen durability,
  reset preserves, source-binding seed import, wrong-version-marker / missing-object / malformed-object
  fail-closed).

1b did NOT wire the import into the live `native_firstrun` bootstrap and did NOT re-bootstrap the v5 lineage —
those are **S4-pre-1c** (re-bootstrap the v5 lineage from the seed record so the real fixture store carries the
frozen leadership + its hash, S5 recovery evidence over the new surface, registry flip, and the CI guard that
`from_accumulator_go_active_params_for_test_only` stays test-only). No production seed-window site touched, no
seed+2 ceiling deleted, no SNAP boundary freeze (S4-pre-2), no S5 closure claim.
