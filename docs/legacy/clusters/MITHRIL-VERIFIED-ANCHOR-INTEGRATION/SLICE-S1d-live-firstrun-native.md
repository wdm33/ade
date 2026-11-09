# SLICE: MITHRIL-VERIFIED-ANCHOR-INTEGRATION / S1d — live FirstRun → native bootstrap

## Narrow claim (the ONLY thing this slice proves)
> Ade can invoke the native Mithril bootstrap path from live `--mode node` FirstRun inputs.

Cold restart from persisted artifacts and ChainSync remain SEPARATE proof gates (S2). This slice is the
end-to-end INVOCATION wiring — it does not change S1a/S1b/S1c (all shipped + proven) or add new state
machinery; it routes the manifest + snapshot files + genesis through the native chain that already exists.

## The live-route contract
**native FirstRun REQUIRES** (all present, else terminal before any decode):
- `--mithril-manifest-path` — the verified Mithril manifest (provenance authority: certified_point,
  network_magic, epoch);
- `--mithril-state-path` — the V2 LedgerDB `state` file (S1a input);
- `--mithril-tables-path` — the Stage-2 `tables` MemPack file (S1c input);
- the GENESIS metadata (the "required metadata file") — the Shelley genesis JSON providing
  `maxLovelaceSupply` + `activeSlotsCoeff` (→ `NativeGenesisConstants`) and `epochLength` + `systemStart`
  (→ the `EraSchedule` for `derive_epoch_window`). Via `--genesis-path` (or the owner's existing genesis
  config if already loaded).

**native FirstRun FORBIDS** (terminal, NO fallback, NO silent ignore):
- `--json-seed-path`, `--consensus-inputs-path`, any cardano-cli extraction fallback, any operator-supplied
  consensus seed. If a forbidden flag is supplied ALONGSIDE the native inputs → a structured terminal error
  (no ambiguous / half-authoritative path).

## The wiring (`crates/ade_node/src/node_lifecycle.rs` `first_run_mithril_bootstrap`, ~2752)
Replace the CLI-seed body (the `import_cardano_cli_json_utxo` + `import_live_consensus_inputs` path) with,
when `--mithril-state-path` + `--mithril-tables-path` are present:
1. read manifest bytes → `import_mithril_manifest_from_bytes` → the report (certified_point, network_magic,
   epoch) → build `VerifiedManifestBinding`;
2. read the `state` file → `decode_native_nonutxo_state(&state_cbor, certified_point, manifest_epoch,
   network_magic)` → `(s1a, s1a_commitment)`;
3. read the `tables` file → `materialize_tables_to_utxo(&tables, CONWAY_ERA_INDEX, None)` → `UTxOState`;
4. load the genesis → `NativeGenesisConstants` + build the `EraSchedule` (reuse `make_node_schedule` /
   the owner's genesis-loading — covering the S1a epoch);
5. `bootstrap_from_native_mithril_snapshot(&s1a, s1a_commitment, utxo, &binding, &genesis, &manifest_bytes,
   &chaindb, &chaindb, &mut wal, &era_schedule, &ledger_view)` → `MithrilBootstrapOutput`;
6. return `BootstrapState` from the output (ledger / chain_dep / tip / the seed-epoch consensus inputs).
The owner's ChainDb / FileWalStore / genesis are REUSED (not re-opened). The native entry routes through the
UNCHANGED single closed `bootstrap_from_mithril_snapshot`.

## Failure semantics (TERMINAL before authority visibility)
Authority is visible only after the WAL commit-point inside the persist (the sole discovery gate). Every
failure BEFORE it is terminal and leaves NO bootable partial state:
- missing / mixed snapshot components (manifest / state / tables / genesis not all present) → terminal
  before any decode;
- manifest / point / network / era mismatch → terminal (the S1a epoch cross-check + the S1b coherence gate);
- decode (S1a) / materialization (S1c) / assembly (S1b) failure → terminal;
- persistence (WAL commit) failure → terminal, no discoverable anchor lineage (warm-start sees first-run-empty).

## Forbid the CLI seed (mechanically gated)
- The native route MUST NOT reach `import_cardano_cli_json_utxo` / `import_live_consensus_inputs`. A
  `ci/ci_check_native_firstrun_no_cli_seed.sh` gate greps the native branch for those tokens + the forbidden
  flags and fails on any (negative-controlled).
- The CLI-seed path may remain as a SEPARATE explicitly-selected diagnostic route, or be removed — but it is
  NEVER a fallback from the native route.

## Acceptance
- the native FirstRun (manifest + state + tables + genesis, no CLI/JSON seed) invokes the native bootstrap →
  a `MithrilBootstrapOutput`, and the durable artifacts persist (the anchor lineage is discoverable via
  `load_recovered_anchor_point`);
- the persisted artifacts equal the function-level S1b for the same inputs (same assembled seed / commitments);
- a missing / mixed component (e.g. state present, tables absent) → terminal, nothing persisted;
- a forbidden flag (`--json-seed-path`) with the native inputs → terminal (no fallback);
- a point / network / era mismatch → terminal before persist;
- LIVE INVOCATION (if the real manifest is available): `--mode node` FirstRun with the real preprod
  manifest + `.mithril-scratch` state + tables + the preprod genesis → the native bootstrap persists, with
  NO `--json-seed-path` / `--consensus-inputs-path`. If no real manifest exists for the snapshot, FLAG it
  (do not fabricate) and prove the wiring hermetically + with a constructed point-coherent manifest.

## Scope fence
The live FirstRun INVOCATION wiring ONLY. NOT cold restart / warm-start recovery / ChainSync (S2). NOT the
Conway per-byte min-UTxO calculator (DC-LEDGER-PARAMS-01). S1a/S1b/S1c are FROZEN (reused unchanged). No new
state machinery — the remaining gap after this is genuinely just the restart/follow PROOF (S2).
