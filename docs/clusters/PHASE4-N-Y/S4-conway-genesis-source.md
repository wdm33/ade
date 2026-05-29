# Invariant Slice — S4: Conway-genesis bootstrap source

## §2 Slice Header

- **Slice Name:** Conway genesis as a second bootstrap source — through the same closed `bootstrap_initial_state` authority; non-Conway fails closed; genesis→initial-state deterministic.
- **Cluster:** PHASE4-N-Y.
- **Status:** Proposed.
- **Cluster Exit Criteria Addressed** (verbatim):
  - [ ] **CE-Y-10.** Conway genesis enters the same authority; test `conway_genesis_bootstrap_through_single_authority`; non-Conway → `genesis_non_conway_fail_closed`; `genesis_to_initial_state_deterministic` (two runs byte-identical).
  - [ ] **CE-Y-11.** Internal cross-path determinism (private Conway net): test `genesis_path_fp_equals_snapshot_path_fp` — bootstrap from genesis+blocks, snapshot it, re-bootstrap from the snapshot, fingerprints equal.
  - [ ] **CE-Y-15** *(partial):* `DC-GENESIS-SRC-01` introduced; `CN-GENESIS-01`/`CN-ANCHOR-01`/`CN-NODE-01` `strengthened_in += "PHASE4-N-Y"`.
- **Slice Dependencies:** none structurally (parallel to S2/S3); CE-Y-11 reuses the S1 anchor + S2/S3 store to round-trip. Sequenced 4th per the plan (enables the two-Haskell-node testnet in S5).

## §3 Implementation Instruction (AI)

Reuse the **existing** `parse_shelley_genesis` (CN-GENESIS-01) + the Conway genesis. Produce the initial `(LedgerState, PraosChainDepState)` + `BootstrapAnchor` and feed the **same** `bootstrap_initial_state` `genesis_initial` path. No Byron→Conway historical replay. No second bootstrap authority. §12 is the contract.

## §4 Intent

Make it impossible to bootstrap from a controlled genesis except through the single closed bootstrap authority, impossible to bootstrap a non-Conway genesis in this cluster (fail-closed), and guarantee the genesis→initial-state transform is a pure deterministic function.

## §5 Scope

- **BLUE:** the genesis→canonical-initial-state transform (genesis config + initial funds/staking → `LedgerState` UTxO + `PraosChainDepState::genesis(initial_nonce)`); Conway-era guard (fail-closed on non-Conway).
- **RED:** genesis file read (reuse `parse_shelley_genesis` + the Conway genesis parse); CLI wiring of a `--conway-genesis` / genesis-bootstrap mode into the same `bootstrap_initial_state` call.
- **BLUE (reused):** `bootstrap_anchor`, `fingerprint`, `snapshot` encode/decode (for CE-Y-11 round-trip).
- **Persistence:** none new (produces a `genesis_initial` pair + anchor).
- **Out of scope:** mainnet Byron→Conway historical replay ([[RO-GENESIS-REPLAY-01]], deferred), forward-sync (S2), recovery (S3), evidence (S5), Mithril (S1).

## §6 Execution Boundary (TCB color)

- **BLUE:** genesis→initial-state transform + Conway-era guard; `fingerprint`; `bootstrap_anchor` mint inputs.
- **GREEN:** `bootstrap_initial_state` (reused, closed authority).
- **RED:** genesis file read / parse (`parse_shelley_genesis`), CLI wiring.

Color resolved. The genesis→state transform is a pure function (BLUE); only file ingress is RED.

## §7 Invariants Preserved

[[CN-GENESIS-01]] (the parser contract — fail-closed on missing/malformed, extra-keys-inert, byte-equal `GenesisAnchor`), [[CN-NODE-01]] (single bootstrap authority), [[CN-ANCHOR-01]]/[[DC-ANCHOR-01]] (anchor shape; the S1 `SeedProvenance::CardanoCliJson`/genesis variant), [[T-DET-01]], [[CN-SEED-01]], the BLUE forbidden-pattern gates.

## §8 Invariants Strengthened or Introduced

**One family — genesis bootstrap source:**
- **Introduces `DC-GENESIS-SRC-01`** — a controlled genesis enters initial state **only** through `bootstrap_initial_state` (`genesis_initial`); the genesis→initial-state transform is pure/deterministic; a non-Conway genesis fails closed in this cluster (no Byron→Conway replay path is invoked).
- Side-effect strengthenings: [[CN-GENESIS-01]] (parser now drives a bootstrap source, not just the producer opcert/genesis), [[CN-ANCHOR-01]] (genesis-sourced anchor), [[CN-NODE-01]] (second source through the one authority).

## §9 Design Summary

`genesis_initial_state(conway_genesis, shelley_genesis_anchor) -> (LedgerState, PraosChainDepState)` is a pure BLUE function: UTxO from the genesis initial funds, `PraosChainDepState::genesis(initial_nonce)` where `initial_nonce` is the genesis-derived value for the controlled net, era guarded to Conway (`CardanoEra::Conway`, else `GenesisSourceError::NonConwayEra`). The result feeds `BootstrapInputs.genesis_initial`. CE-Y-11 proves round-trip determinism: genesis→state→`encode_snapshot`→`decode_snapshot`→`bootstrap_initial_state` warm-start → same `fingerprint`.

## §10 Changes Introduced

- **Types:** closed `GenesisSourceError` enum (incl. `NonConwayEra`); a genesis-bootstrap CLI flag (RED).
- **State transitions:** the pure `genesis_initial_state` transform.
- **Persistence:** none new.
- **Removal/refactors:** none.

## §11 Replay / Crash / Epoch Validation

- **Replay:** `genesis_to_initial_state_deterministic` (two runs byte-identical), `genesis_path_fp_equals_snapshot_path_fp` (genesis→snapshot→re-bootstrap fingerprint identity, CE-Y-11).
- **Crash/restart:** n/a here (recovery is S3; CE-Y-11 only exercises the encode/decode round-trip).
- **Epoch boundary:** single-epoch bootstrap (no historical era transitions).

## §12 Mechanical Acceptance Criteria

- [ ] `conway_genesis_bootstrap_through_single_authority` — genesis bootstraps only via `bootstrap_initial_state`.
- [ ] `genesis_non_conway_fail_closed` — a non-Conway genesis → `GenesisSourceError::NonConwayEra`, no state produced.
- [ ] `genesis_to_initial_state_deterministic` — two runs byte-identical `(LedgerState, PraosChainDepState)`.
- [ ] `genesis_path_fp_equals_snapshot_path_fp` — genesis-derived fingerprint == snapshot-round-tripped fingerprint.
- [ ] `cargo test --workspace` clean; `ci_check_bootstrap_anchor_closure.sh`, `ci_check_snapshot_encoder_closure.sh`, `ci_check_dependency_boundary.sh` pass; the S1 `ci_check_mithril_uses_bootstrap_initial_state.sh` negative (no `trait *Anchor`) still passes (genesis source adds no trait seam).

## §13 Failure Modes

Non-Conway genesis → `GenesisSourceError::NonConwayEra` (fail-fast). Malformed genesis → existing `GenesisParseError` (fail-closed, CN-GENESIS-01). Both deterministic; occur before any authoritative write.

## §14 Hard Prohibitions

**Inherited (cluster §7).** **Slice-specific:** no Byron→Conway historical replay; no second bootstrap authority or `*Anchor` trait; no implicit genesis defaults (CN-GENESIS-01); no `String`/float/`HashMap`/clock in the BLUE transform; no stringly era fallback.

## §15 Explicit Non-Goals

No historical replay, no forward-sync/recovery/evidence (S2/S3/S5), no Mithril (S1), no new protocol surface, no multi-era bootstrap.

## §16 Completion Checklist

- [ ] Genesis state replay-derivable + canonically encodable; non-Conway fail-closed; determinism + round-trip fingerprint identity proven; CI enforces single-authority entry.

## §17 Review Notes

Risk: a genesis path sneaking around `bootstrap_initial_state` → CE-Y-10 + the no-second-init grep. Risk: initial-nonce derivation correctness for the controlled net → CE-Y-11 round-trip + (live agreement is S5's job, observable-surface only).

## §18 Authority Reminder

Planning aid only; registry + CI authoritative.
