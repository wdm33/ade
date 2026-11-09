# Slice PHASE4-N-F-A / A3a — WAL seed-epoch-consensus-inputs provenance entry

## 2. Slice Header
- **Slice Name:** Closed `WalEntry::SeedEpochConsensusInputsImported` variant + canonical codec + replay-reconstructed bootstrap-provenance view.
- **Cluster:** PHASE4-N-F-A. **Status:** Merged (`c507159`).
- **Cluster Exit Criteria Addressed:** **contributes to CE-A-3** (the WAL-provenance foundation; CE-A-3 is *completed* by A3b's production warm-start byte-identity proof).
- **Slice Dependencies:** A1 (sole sidecar codec), A2 (keyed sidecar surface + bootstrap population).

## 3. Implementation Instruction (AI)
Implement §9/§10 only. Add the WAL variant + codec + replay provenance view + the append at bootstrap. Do **not** wire warm-start consumption (A3b), projection (A4), or produce. Commit with the trailer.

## 4. Intent
Make the seed-epoch consensus-input import an **authoritative, replay-reconstructable WAL fact**: the WAL records `{anchor_fp, sidecar_hash, epoch_no}`, and replay yields a **typed `RecoveredBootstrapProvenance` view** (never a no-op validation) that A3b uses to locate + verify the sidecar. *(Introduces the WAL-provenance foundation of candidate `DC-CINPUT-01`.)*

## 5. Scope
- **Modules:** `ade_ledger::wal` (BLUE) — `event.rs` (new variant + `encode/decode_wal_entry` arm), `replay.rs` (provenance view); a new BLUE `RecoveredBootstrapProvenance` type. RED: the append site at the bootstrap composition (`ade_runtime::{genesis_bootstrap, mithril_bootstrap}` or the orchestration that owns the `WalStore`).
- **Persistence:** one additive WAL entry appended at bootstrap; **no** change to `AdmitBlock`, the `prior_fp`/`post_fp` block fingerprint chain, or the WAL store trait.
- **Out of scope:** warm-start consumption (A3b), projection (A4), produce wiring.

## 6. Execution Boundary (TCB)
- **BLUE:** `wal::event` (variant + codec), `wal::replay` (provenance view), `RecoveredBootstrapProvenance` — all in `ade_ledger`, deterministic.
- **GREEN:** none.
- **RED:** the append call at the bootstrap composition site (`ade_runtime`).

## 7. Invariants Preserved
- `DC-WAL-*` / WAL fingerprint chain — `AdmitBlock` meaning + the `prior_fp`/`post_fp` chain are **unchanged**; the new variant is additive (distinct tag) and does **not** participate in the block chain.
- `CN-CINPUT-01` (A1) — `sidecar_hash` is `blake2b256` of the **exact A1 canonical encoded bytes** (the sole encoder); no second encoding.
- `CN-ANCHOR-01`/`DC-ANCHOR-01`, `CN-NODE-01`, BLUE forbidden-pattern set, determinism, canonical byte-identical WAL codec.

## 8. Invariants Strengthened or Introduced
- **Introduces** the WAL-provenance foundation of candidate `DC-CINPUT-01` — *the seed-epoch consensus-input import is a canonical WAL fact, replay-reconstructable as a typed `RecoveredBootstrapProvenance`; exactly one per store/anchor.* (A3b completes `DC-CINPUT-01` with the production byte-identity proof.)

## 9. Design Summary
- New closed variant `WalEntry::SeedEpochConsensusInputsImported { anchor_fp: Hash32, sidecar_hash: Hash32, epoch_no: EpochNo }`, next **additive** wire tag; `encode_wal_entry`/`decode_wal_entry` arm; canonical, byte-identical round-trip.
- `sidecar_hash = blake2b256(encode_seed_epoch_consensus_inputs(&record))` (A1 sole encoder).
- **Append ordering:** at bootstrap, after A2's sidecar `put` commits → then append the WAL entry (the WAL append is the commit point; a crash between leaves "no provenance entry" = not-imported, never a half-state).
- **Replay (the locked wording):** the provenance entry **does not mutate ledger state or the `prior_fp`/`post_fp` block state**; it **updates/returns a `RecoveredBootstrapProvenance` view** (`{anchor_fp, sidecar_hash, epoch_no}`) that warm-start consumes. It is **not** a no-op validation-only entry that disappears after replay.
- **One per store/anchor:** a second `SeedEpochConsensusInputsImported`, or one whose `anchor_fp` mismatches the replay anchor, **fails closed** (typed `WalError`).

## 10. Changes Introduced
- **Types:** `WalEntry::SeedEpochConsensusInputsImported{…}` + its wire tag; new BLUE `RecoveredBootstrapProvenance{anchor_fp, sidecar_hash, epoch_no}`.
- **Codec:** `encode/decode_wal_entry` arm for the new tag.
- **Replay:** `replay_from_anchor` (or a sibling) returns the `RecoveredBootstrapProvenance` alongside the recovered ledger state.
- **Append:** the bootstrap composition appends the entry after the sidecar `put`.

## 11. Replay, Crash, Epoch Validation
- **Tests:** `wal_seed_cinput_entry_round_trips_byte_identical`; `replay_yields_bootstrap_provenance_view` (replay returns the typed view, ledger block state unchanged); `replay_rejects_duplicate_or_anchor_mismatched_provenance_entry`; `admit_block_chain_unaffected_by_provenance_entry`.
- **Crash:** sidecar-then-WAL ordering ⇒ no half-state (covered with A3b's restore test).

## 12. Mechanical Acceptance Criteria
- [ ] `wal_seed_cinput_entry_round_trips_byte_identical` passes.
- [ ] `replay_yields_bootstrap_provenance_view` passes (typed view returned; no ledger/block-chain mutation).
- [ ] `replay_rejects_duplicate_or_anchor_mismatched_provenance_entry` passes (fail-closed).
- [ ] `admit_block_chain_unaffected_by_provenance_entry` passes.
- [ ] `cargo build --workspace` + `cargo clippy` clean; `cargo test --workspace` green; A2's `ci_check_consensus_input_provenance.sh` still passes.

## 13. Failure Modes
Typed `WalError`, fail-closed, no panic: malformed/short-hash decode; duplicate provenance entry; provenance `anchor_fp` ≠ replay anchor. None mutate ledger/block state.

## 14. Hard Prohibitions
**Inherited (cluster).** **Slice-specific:** additive WAL tag only; no change to `AdmitBlock` meaning or the block fingerprint chain; the provenance entry must **not** mutate ledger/block state (but **must** surface the typed provenance view — not a no-op); one entry per store/anchor; `sidecar_hash` from A1's exact bytes; no warm-start consumption (A3b); no projection (A4); no produce wiring; no `--consensus-inputs-path` reference.

## 15. Explicit Non-Goals
No production warm-start restore (A3b). No projection (A4). No produce wiring. No META pointer (WAL is the authority).
