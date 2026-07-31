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

## Risk

GREEN codec / merge + RED assembly / warm-start glue. **BLUE `nonce.rs` / `header_validate.rs` /
`era_schedule.rs` untouched** — only RSW *provenance* moves CLI→store. Replay-equivalent (reconstructed
candidate becomes byte-identical regardless of the CLI). v6 forces reimport of v5 stores (typed
fail-closed, precedented). Mechanical breadth (~15 builders) is type-system-forced, no silent misses.
