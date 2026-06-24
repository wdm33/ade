# SLICE: MITHRIL-VERIFIED-ANCHOR-INTEGRATION / S2 — MITHRIL-FIRST-RUN-CONTINUITY

> The native Mithril FirstRun (S1d / `DC-MITHRIL-07`) bootstraps an authoritative ledger but is
> **not yet boundary-usable**: it does not build the EVIEW reduced checkpoint, so a Mithril-started
> node follows initially but goes **inert at the epoch boundary** (ECA needs checkpoint + sidecar +
> tip; only the sidecar + tip are produced). This slice makes the native FirstRun **complete for
> epoch continuity**, then puts the judge-facing command over the completed mechanism.

## Narrow claim (the ONLY thing this slice proves)

A node bootstrapped from a verified Mithril snapshot via the native route is a complete,
boundary-usable Cardano node: it builds its own next-epoch authority machinery **inline at
bootstrap**, persists its bootstrap lineage, follows, survives a cold restart, and **promotes the
next-epoch authority automatically when the boundary arrives** — with one documented command and
no cardano-cli / JSON-seed / operator consensus inputs anywhere on the path.

The continuity invariant (proposed `DC-MITHRIL-08`):

```
verified Mithril snapshot
  → native state/tables decode (S1a/S1c, FROZEN)
  → materialize authoritative UTxO (S1c, FROZEN)
  → build the EVIEW reduced checkpoint BEFORE drop(utxo)   ← Gap 2 (this slice's core)
  → atomically persist the bootstrap lineage (S1b, FROZEN)
  → start the follow loop (ChainSync)
  → usable at the next epoch boundary (ECA promotes from the checkpoint + sidecar)
```

## The continuity contract

- **Boundary-complete bootstrap.** When the decoded cert-state carries delegations (the EVIEW
  package), the native FirstRun MUST build the reduced checkpoint from the materialized UTxO BEFORE
  the UTxO is dropped — the same `build_live_reduced_checkpoint` the admission path uses, sealing
  the bootstrap baseline at `seed_slot = certified_point.slot`. After bootstrap,
  `(reduced-checkpoint.redb, seed sidecar, durable tip)` are ALL present ⇒ `eview_inputs` is `Some`
  ⇒ ECA activation is armed (not inert) at the boundary.
- **Byte-identical when there is no EVIEW package.** A snapshot whose cert-state has no delegations
  builds no checkpoint — the bootstrap output is unchanged (point 8, `DC-EPOCH-11`).
- **One command, no stitching.** A single `ade node run --bootstrap-mithril … --snapshot-dir …
  --network … --data-dir …` verifies the snapshot, decodes, materializes, builds the checkpoint,
  persists, prints the receipt, and enters ChainSync. No separate `--mode admission` pre-pass.
- **No silent fallback.** A legacy seed flag (`--json-seed-path` / `--consensus-inputs-path`)
  supplied alongside `--bootstrap-mithril` is a structured TERMINAL error before any decode (extend
  the existing `NativeRouteForbiddenFlag`). The native route reaches none of the CLI-seed importers.

## The wiring

1. **Gap 2 — inline checkpoint build (the boundary enabler).**
   - `crates/ade_node/src/admission/bootstrap.rs`: make `build_live_reduced_checkpoint` +
     `reduced_checkpoint_path` shareable (`pub(crate)`, or lift to a small shared module). No
     behaviour change to the admission caller.
   - `crates/ade_node/src/native_firstrun.rs` `native_first_run_bootstrap`: thread a
     `snapshot_dir: &Path`; after the UTxO is materialized (currently ~L423) and BEFORE it is
     consumed by `bootstrap_from_native_mithril_snapshot` (~L451), insert:
     `if !s1a.cert_state.delegation.delegations.is_empty() { build_live_reduced_checkpoint(snapshot_dir, &utxo, binding.certified_point.slot)?; }`
     mapping a build failure to a terminal `NativeFirstRunError::ReducedCheckpoint`.
   - `crates/ade_node/src/node_lifecycle.rs` `first_run_native_mithril_bootstrap`: pass
     `cli.snapshot_dir` (the data dir) down.
2. **Gap 1 — the judge command (UX over the completed mechanism).**
   - `crates/ade_node/src/cli.rs`: `--bootstrap-mithril <manifest>` + `--snapshot-dir <dir>` derive
     the state/tables paths; `--network <net>` selects the Shelley genesis + boundary. A new
     `Mode::MithrilVerify` (`ade … --mode mithril_verify` / "`ade mithril verify`") runs the manifest
     + component verification standalone.
   - The **bootstrap receipt** (printed + a canonical receipt artifact): certified anchor
     point/hash, state / tables / non-UTxO commitments, durable lineage id, explicit ChainSync-start
     status.
3. **Gap 3 — cold restart + ChainSync proof.** No new code expected (the loop already follows after
   FirstRun); the proof is the acceptance run: start via the command, kill, restart, recover the
   persisted authority + checkpoint, follow.

## Failure semantics (TERMINAL before authority visibility)

Mixed / missing components (manifest / state / tables / shelley genesis not all present); unsupported
layout or era; manifest / point / network / era mismatch; a decode / materialize / **checkpoint
build** / assemble / persist failure; a legacy seed flag combined with `--bootstrap-mithril`. Every
failure leaves NO bootable partial authority state and NO fallback to the CLI/JSON seed. The WAL
commit-point inside `bootstrap_from_native_mithril_snapshot` stays the SOLE discovery gate; the
checkpoint build runs BEFORE it (a build failure aborts before any authority is visible).

## Forbid the CLI seed (mechanically gated)

Extend `ci/ci_check_native_firstrun_no_cli_seed.sh`: the native body + the `--bootstrap-mithril`
dispatch reference none of `import_cardano_cli_json_utxo` / `import_live_consensus_inputs`; the
forbidden-flag terminal covers `--bootstrap-mithril` + a legacy seed flag.

## Acceptance (CEs)

- **CE-S2-1 (hermetic, the core).** After `native_first_run_bootstrap` over a synthetic snapshot whose
  cert-state has delegations, `reduced-checkpoint.redb` exists, is sealed at `certified_point.slot`,
  and `derive_stake_by_pool` over it is well-formed; a no-delegation snapshot builds no checkpoint
  (byte-identical output).
- **CE-S2-2 (hermetic, continuity).** After a native Mithril FirstRun, the `node_lifecycle`
  `eview_inputs` construction yields `Some` (checkpoint + sidecar + tip all present) — the boundary is
  armed, not inert.
- **CE-S2-3 (hermetic, the command).** `--bootstrap-mithril` + a legacy seed flag is terminal; missing
  components are terminal; the happy path prints the receipt and reports ChainSync-start.
- **CE-S2-4 (live).** Start a node via the command from a verified snapshot; kill it; restart; it
  recovers the persisted authority + checkpoint and follows the chain via ChainSync.
- **CE-S2-5 (live, the judge story).** A Mithril-started node crosses a real epoch boundary, ECA
  promotes the next-epoch authority automatically, forges when ADE1 is elected, and a Haskell peer
  adopts the block.

## Scope fence

S1a / S1b / S1c and the EVIEW/ECA machinery (`DC-EVIEW-*`, `DC-EPOCH-*`) are FROZEN — reused, not
modified. This slice adds the inline checkpoint build, the judge command + receipt, and the
continuity proof. No new state machinery, no second authority, no BLUE change (the reduce/aggregate
primitives are reused). TCB: GREEN glue (the checkpoint-build call + the receipt projection); RED
shell (the CLI flags, the dispatch, the FirstRun wiring). Implementation order: **Gap 2 → Gap 1 →
Gap 3**, so the boundary path is unblocked before the UX.
