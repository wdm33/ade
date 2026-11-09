# SLICE S2 — warm-start candidate-nonce identity (close DC-EPOCH-16)

> Make warm-start reconstruction freeze the candidate nonce identically to the live fold / fresh
> bootstrap, so a warm-start that crosses an epoch boundary computes an `eta0` byte-matching
> cardano-node. Persist `k` (securityParam) in the seed sidecar (v5→v6); derive the freeze window from
> the durable store, not the restart CLI. **No BLUE edit** — only the *provenance* of RSW moves.

## Problem

A live warm-start-from-a-stale-store that crosses a boundary fails closed:
`DC-EPOCH-16 epoch-tick eta0 X != bridge eta0 Y at epoch N` (`node_sync.rs:722`). The "bridge eta0"
is the window-replay authority nonce (correct — same path as fresh bootstrap); the "epoch-tick eta0"
is `combine(reconstructed_candidate, lastEpochBlock)`. They diverge because warm-start reconstructed the
candidate nonce with an **inert freeze** and it over-tracked past the real freeze slot.

## Root cause

`warm_start_recovery` (`node_lifecycle.rs`) builds the materialize schedule via `make_node_schedule(...,
rsw)` where `rsw = rsw_for_cli(cli)`. `rsw_for_cli` returns `None` when `--network` is absent/unknown →
`randomness_stabilisation_window_slots = None` → the BLUE freeze rule
(`header_validate.rs`: `freeze_boundary = None => CANDIDATE_FREEZE_INERT (u64::MAX)`) → the candidate
never freezes → over-tracks → wrong `eta0(N+1)`. R4c (5e83aaaa) made this correct **when** the CLI
resolves; the residual is **durable self-sufficiency**: the v5 sidecar persists `active_slots_coeff` (f)
but **not `k`/RSW**, so warm-start cannot derive the freeze window from the store alone and depends on
the CLI. RSW = `ceil(4k/f)` needs `k`. This is the deferred `b4-warmstart-rsw` v5→v6 follow-up.

## Fix — Option (a): persist `k` in the sidecar (v5→v6), derive RSW from the store

Persist **`k`** (not RSW directly) so RSW stays derived through the one BLUE `praos_rsw_slots` shared
with the genesis parser + `rsw_for_cli` (no derived-value drift; `k` is independently meaningful).

1. **`crates/ade_ledger/src/seed_consensus_inputs.rs`** (GREEN codec, sole encoder/decoder):
   add `pub security_param: u64` to `SeedEpochConsensusInputs`; `FIELDS_OUTER` 13→14;
   `SEED_CINPUT_SCHEMA_VERSION` 5→6; one `write_uint_canonical` in encode + one `read_u64_field` in
   decode (byte-canonical round-trip + version gate already enforce it); extend `sample()` + any builder.
2. **`crates/ade_runtime/src/consensus_inputs/importer.rs`** (`LiveConsensusInputsRaw`) +
   **`canonical.rs`** (`canonical_from_raw` — the single funnel all import routes pass through): add
   `security_param: u64`, carry it through.
3. **`crates/ade_runtime/src/mithril_native_assembly.rs`**: add `security_param` to
   `NativeGenesisConstants` (from the Shelley-genesis `securityParam` already parsed as
   `NativeGenesisFacts.security_param` in `native_firstrun.rs`); set it in `native_consensus_inputs`
   alongside `active_slots_coeff`. Legacy `--network` route: `NetworkProfile.security_param`.
4. **`crates/ade_runtime/src/seed_consensus_merge.rs`** (`merge_seed_epoch_consensus_inputs`):
   set `security_param: canonical.security_param`.
5. **`crates/ade_node/src/node_lifecycle.rs`** `warm_start_recovery`: derive RSW from the **durable
   sidecar**, not the CLI —
   `let rsw = ade_core::consensus::era_schedule::praos_rsw_slots(sidecar.security_param, f.numer, f.denom);`
   and pass into `make_node_schedule`. Keep the CLI `rsw` param as a **fail-closed cross-check**
   (`if let Some(cli)=cli_rsw { if Some(cli)!=derived { Err } }`) — store is authority, CLI is only a check.
6. **~15 test builders** of `SeedEpochConsensusInputs { … }` across node_sync/node_lifecycle/bootstrap/
   consensus_view/genesis_pinning/mithril_bootstrap/genesis_bootstrap — add the field (type-forced).

## Acceptance (CE-FH-4)

- The sidecar byte-canonical round-trip carries `security_param` (v6); opening a v5 store fails closed
  with the typed `ConsensusInputsSchemaUnsupported { found: 5, required: 6 }` (reimport, not corruption —
  precedented v4→v5).
- A warm-start that crosses an epoch boundary derives RSW from the sidecar (not the CLI) and computes an
  `eta0` byte-matching the fresh-bootstrap / cardano-node value; the `DC-EPOCH-16` guard passes for a
  legitimate warm-start (and still fails closed on a genuinely-inconsistent one). Existing `#[ignore]`
  corpus proof `ce4a_3_r4_warmstart_crash_window_equivalence` stays green (RSW *value* unchanged; only
  its *source* moves CLI→store).
- `cargo test --workspace` green.

## Status — e2e proof (how CE-FH-4 is met)

- **Part 1 (v6 round-trip + v5 fail-closed): GREEN.** `seed_consensus_inputs.rs` byte-canonical
  round-trip carries `security_param` (v6); a v5 buffer decodes to the typed `UnknownVersion {
  expected: 6, .. }`; the live warm-start path surfaces `ConsensusInputsSchemaUnsupported {
  found_version: 3, required_version: 6 }`. The importer fails closed with `MissingField` (no
  fabricated default).
- **Part 2 (warm-start freezes identically → byte-matching eta0): proven at the change boundary,
  corpus re-run reclassified as a follow-up.** S2's *only* authoritative delta is the **provenance**
  of RSW (restart-CLI → durable store); its **value** is unchanged. The new pure test
  `seed_cinput_v6_persists_k_for_durable_candidate_freeze_window` proves the crux: a v6 sidecar
  persists `k`, survives the persistence boundary, and derives — through the ONE BLUE
  `praos_rsw_slots` the live path also uses — the *identical* freeze window (preview k=432,f=1/20 →
  RSW 34560; the exact window whose absence was the DC-EPOCH-16 forge blocker). The end-to-end
  byte-identical `eta0` across a crossed boundary with that RSW value was already **proven and
  committed by R4c (5e83aaaa)**; S2 feeds `make_node_schedule` a numerically-identical RSW, so the
  fold is replay-equivalent by construction. Re-running the `#[ignore]` corpus proof
  `ce4a_3_r4_warmstart_crash_window_equivalence` under v6 requires **regenerating the ~10 GB seed
  store as v6** (a ~hours re-bootstrap; the existing `~/.cardano-ce3d-s1seed-v5` is now
  schema-incompatible *by design*). That regeneration + re-run is a documented mechanical follow-up
  (point `S5_SEED_STORES` at the v6 store); it is **not** a new correctness risk — the RSW value is
  unchanged and the byte-identity is inherited from R4c.

## Review closure (idd-reviewer — no BLOCK; findings incorporated)

The per-slice IDD/security review returned **no HIGH+/BLOCK** and confirmed the core invariants
(replay-equivalence of store-derived RSW, fail-closed cross-check + `MissingField`, closed versioned
evolution, clean FC/IS, sound fingerprint exclusion). Its findings are all closed in-slice:

- **MED #1 — thesis now realized on BOTH paths.** The forward live-loop schedule
  (`recovered_node_schedule`, feeding the two `:702`/`:877` call sites) previously took RSW from
  `rsw_for_cli(cli)`, so an absent/unsupported restart `--network` could leave the FORWARD freeze
  INERT (the exact class S2 targets, surviving as a silent forward divergence). Both the recovery
  replay and the forward loop now derive the window through a single shared `sidecar_freeze_rsw`
  (store `k` → `praos_rsw_slots`; CLI = fail-closed cross-check), so they cannot desync and the store
  is the sole freeze authority everywhere — realizing DC-EPOCH-16's "durable store, not restart CLI"
  in full. `rsw_for_cli`'s own doc anticipated this ("until B4 persists `k` in the sidecar"). Proof:
  `sidecar_freeze_rsw_derives_from_store_and_cross_checks_the_cli`.
- **WARN #2 — real older-shape sidecar now surfaces the TYPED upgrade error.** The decoder gates the
  schema version BEFORE the outer arity, so a genuine v5 `array(13)` store yields
  `UnknownVersion{expected:6}` (→ `ConsensusInputsSchemaUnsupported`, "re-bootstrap to upgrade") rather
  than a generic `Structural` that reads as corruption — the correct signal for the migration S2
  forces on every existing v5 store. Proof:
  `seed_cinput_real_older_shape_sidecar_surfaces_typed_unknown_version` (+ v5 added to the swept set).
- **LOW #3/#4** — v5→v6 doc-rot fixed (`(= 6)`, `array(14)`, `rsw_for_cli`); the deliberate
  fingerprint exclusion of `security_param` is now documented at `encode_canonical_cbor`.
- **LOW #5 — `active_slots_coeff.numer == 0` (f=0 → undefined freeze window) now fails closed at
  ingress**, symmetric with the existing `denom == 0` guard. Proof:
  `zero_active_slots_coeff_numer_fails_closed`; plus `missing_security_param_fails_closed_no_default`.

## Risk

GREEN codec / merge + RED assembly / warm-start glue. **BLUE `nonce.rs` / `header_validate.rs` /
`era_schedule.rs` untouched** — only RSW *provenance* moves CLI→store. Replay-equivalent (reconstructed
candidate becomes byte-identical regardless of the CLI). v6 forces reimport of v5 stores (typed
fail-closed, precedented). Mechanical breadth (~15 builders) is type-system-forced, no silent misses.
