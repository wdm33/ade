# PHASE4-N-F-G-I — Shared admission/pre-seed bootstrap persists the seed-epoch anchor lineage

> Single-slice follow-up to the **closed** PHASE4-N-F-G-H (node-spine serve-to-peer).
> Surfaced by the C1 `--mode node` forge dry-run: the shared admission/pre-seed bootstrap
> **mints the `BootstrapAnchor` then discards it** (`let _anchor = mint(...)`,
> `crates/ade_node/src/admission/bootstrap.rs:170`), so it never writes the **seed-epoch
> anchor lineage** that `--mode node` WarmStart recovery requires. Result: a forge-capable
> WarmStart store is unreachable by any non-Mithril operator path — the dry-run fails closed
> with `WarmStartNoAnchorLineage` ("warm start detected … no persisted seed-epoch anchor
> lineage to recover").
>
> **Framing (operator decision, authorized):** this persists what the **shared imported
> consensus path already computed** — it is *not* a bootstrap-from-genesis. Forbidden is
> `genesis → Ade derives consensus inputs / ledger authority`; allowed is
> `--json-seed + import_live_consensus_inputs → mint existing anchor → persist the lineage
> WarmStart requires`.

## Primary invariant (the gap closed)

The shared non-Mithril/admission bootstrap that consumes `--json-seed` +
`import_live_consensus_inputs` MUST persist the seed-epoch anchor lineage (the
anchor-fp-keyed sidecar + the WAL provenance commit) it already computes — the **same**
lineage `mithril_bootstrap` and `genesis_bootstrap` persist — so a `--mode node` WarmStart
can recover a forge-capable store seeded purely from the shared extraction/import path. The
lineage is derived **only** from `--json-seed` (imported UTxO → anchor fingerprints) +
`import_live_consensus_inputs` (the canonical seed-epoch consensus inputs); **never** from a
genesis-derived eta0/stake/ASC/ledger constructor.

## Hard prohibitions (carried verbatim from the authorized "A" decision)

- Do **not** wire `bootstrap_from_conway_genesis` to any CLI mode (no genesis-shaped operator path).
- Do **not** add a non-Mithril FirstRun — `CN-NODE-01` "FirstRun is Mithril-only" stays intact.
- Do **not** add a from-genesis consensus-input / ledger constructor.
- Do **not** add any new `--mode node` / admission flag.
- Do **not** create C1-only bootstrap logic — the fix is in the **shared** admission bootstrap
  used by every venue.

## Change (TCB: RED shell — store/WAL persistence; deterministic glue over shared helpers)

1. **Extract** the byte-identical `persist_seed_epoch_consensus_inputs` (duplicated in
   `ade_runtime::genesis_bootstrap` + `ade_runtime::mithril_bootstrap`, differing only in the
   error type) into one shared `ade_runtime::seed_epoch_lineage::persist_seed_epoch_consensus_inputs`
   with a closed error enum; repoint `genesis_bootstrap` + `mithril_bootstrap` at it (behavior
   **byte-identical** — same sidecar-put-then-WAL-append A3a order, same helpers).
2. In `ade_node::admission::bootstrap`, **keep** the minted anchor (drop the `_`) and call the shared
   persist fn after the file WAL is opened (`snapshot_store = chaindb`, `wal = wal_store`,
   `anchor`, `seed_consensus_inputs = canonical`).

## Cluster Exit Criteria

- **CE-G-I-1** (mechanical): new test `admission_bootstrap_persists_seed_epoch_anchor_lineage`
  asserts the seed-epoch sidecar **and** WAL provenance entry are present after the admission
  bootstrap, keyed by the anchor's `initial_ledger_fingerprint`, for the imported bundle's epoch.
- **CE-G-I-2** (mechanical, no-regression): `cargo test -p ade_node -p ade_runtime` green — the
  admission consume path (RO-LIVE-05), `genesis_bootstrap`, and `mithril_bootstrap` tests all pass;
  `ci/ci_check_mithril_seed_point_independence.sh` stays green (mithril copy repointed,
  behavior-preserving).
- **CE-G-I-3** (operational): a `--mode node` WarmStart recovers from an admission-pre-seeded C1
  store — the C1 dry-run resumes past the prior `WarmStartNoAnchorLineage` failure.

## Invariants

- **Preserves:** `CN-NODE-01` (FirstRun Mithril-only), `DC-MITHRIL-02` (manifest seed-point
  independence — mithril copy repointed but byte-identical; independence gate stays green),
  `CN-REHEARSAL-FIDELITY-01` (no from-genesis constructor, no new flag, no private-only branch),
  the admission consume path (`RO-LIVE-05`).
- **Strengthens:** the bootstrap-anchor seed-epoch persistence contract — admission now persists
  the same anchor lineage as `genesis_bootstrap` / `mithril_bootstrap` (the precise registry rule —
  a `CN-CINPUT` / `DC-WAL` family entry governing the seed-epoch sidecar/provenance — resolved at
  implement time; `strengthened_in += PHASE4-N-F-G-I`).

## Acceptance (the authorized "A" criteria, mapped)

1. admission/pre-seed persists the lineage it mints → the new persist call (CE-G-I-1).
2. derived only from `--json-seed` + `import_live_consensus_inputs` → inputs are `utxo`/`utxo_fp` +
   `canonical`; no other source.
3. no genesis-derived consensus-input constructor → `ci_check_node_path_fidelity.sh` green.
4. no new `--mode node` flag → pinned 28-flag allow-list unchanged.
5. `CN-NODE-01` intact → FirstRun arm untouched.
6. WarmStart consumes the C1 store → CE-G-I-3.
7. RO-LIVE-05 / consume path no regression → CE-G-I-2.
8. C1 run resumes from the same shared path → CE-G-I-3 + the live resume.
