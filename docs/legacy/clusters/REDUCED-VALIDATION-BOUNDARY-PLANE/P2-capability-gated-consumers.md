# P2 — capability-gated consumers

Complete the typed non-interchangeable result pair and the verdict capability, so a reduced projection can never
reach epoch authority — not by construction (P1 already removed the raw `.mark/.set/.go` fields, forcing every
read through `as_authoritative()`), and now also not by a widening conversion, a promoted verdict, or an
un-checked boundary result. Independently safe: adds types + fail-closed doors + tests; changes no authoritative
byte and no production control flow (the accumulator remains the sole boundary-crosser).

## Grounding (what P1 already established, verified by reading the code)

- The reduced plane (`block_validity`, `track_utxo=false`) does **not** cross epoch boundaries or touch snapshots
  at all — `process_epoch_boundary` calls only `apply_nonce_input` (Praos nonce). So no production path builds a
  `ReducedBoundaryProjection` today; it is a typed latent capability that P3's recovery/fork path will drive.
- The sole production boundary-crosser is the authoritative `EpochAccumulator` (`cross_epoch_boundary`), which
  P1 already fails closed at (`rules.rs` + `epoch.rs`: `as_authoritative().ok_or(FullBoundaryStateRequired)?`).
- Leadership/forging read a projected `PoolDistrView`, not raw `EpochStakeSnapshots`; CE-3d reads `Authoritative`
  snapshots via `as_authoritative()`. No production consumer uses `as_authoritative().unwrap()` (audited — all
  unwraps are `#[cfg(test)]` / harness).

So P1 already satisfies gates 4/6 *structurally*. P2 makes the guarantee **explicit in the type system** and
pins it with tests, and completes the `FullEpochBoundaryResult` / `LedgerBoundaryVerdict` /
`StructuralValidity` sibling types that `reduced_boundary.rs`'s doc-comments already promise.

## New types (ade_ledger, in `reduced_boundary.rs`)

```rust
/// The authoritative boundary result — the full transition's output. Carries the post-boundary LedgerState +
/// EpochBoundaryAccounting. The ONLY thing that may feed ActiveEpochAuthority, leadership, forging, CE-3d, or a
/// full-ledger verdict (I-RVB-4).
pub struct FullEpochBoundaryResult { pub ledger: LedgerState, pub accounting: EpochBoundaryAccounting }

/// The two non-interchangeable boundary results (I-RVB-1). A caller extracts the full result ONLY via
/// `require_full()`, which returns `FullBoundaryStateRequired` on `Reduced` — there is NO `From<Reduced>` and no
/// field access that widens a reduced projection into authority (N-RVB-4).
pub enum LedgerBoundaryVerdict { Full(FullEpochBoundaryResult), Reduced(ReducedBoundaryProjection) }
impl LedgerBoundaryVerdict {
    pub fn require_full(self, boundary_point: SlotNo) -> Result<FullEpochBoundaryResult, GovernanceTerminal>;
}

/// Verdict capability (I-RVB-3). A reduced-plane structural check yields `StructuralValidity`; it can NEVER be
/// promoted to `FullLedgerValidity` merely because a boundary crossed without error. `require_full_ledger()`
/// returns `FullBoundaryStateRequired` on `StructuralValidity`.
pub enum LedgerValidityCapability { StructuralValidity, FullLedgerValidity }
impl LedgerValidityCapability {
    pub fn require_full_ledger(self, boundary_point: SlotNo) -> Result<(), GovernanceTerminal>;
}
```

## Changes

1. Add the three types above in `reduced_boundary.rs` (co-located with `ReducedBoundaryProjection`), each with a
   single fail-closed door to authority and no widening path.
2. Keep the accumulator's authoritative boundary as the sole `Full` producer; P2 does not add a reduced
   dispatcher (that is P3) — it provides the return type P3 will use.
3. No change to authoritative encoding, fingerprints, or control flow.

## P2 acceptance (gates 4 & 6, I-RVB-3)

- **Gate 4 / I-RVB-4.** `EpochStakeSnapshots::ReducedUnavailable.as_authoritative()` is `None`; the authority
  door yields `FullBoundaryStateRequired` (test). No production consumer bypasses it (audit: no non-test
  `as_authoritative().unwrap()`).
- **Gate 6 / I-RVB-1.** `LedgerBoundaryVerdict::Reduced(_).require_full(pt)` → `FullBoundaryStateRequired`;
  `Full(_).require_full(pt)` → the result (test). No `From<ReducedBoundaryProjection> for FullEpochBoundaryResult`.
- **I-RVB-3.** `LedgerValidityCapability::StructuralValidity.require_full_ledger(pt)` → `FullBoundaryStateRequired`;
  `FullLedgerValidity` → `Ok(())` (test).
- ade_ledger + ade_node compile; all prior tests stay green; authoritative bytes unchanged.

## Not in P2

The reduced-boundary dispatcher + its live/recovery/fork-switch exercise (P3); the authoritative post-RUPD mark
correction (its own slice, sequenced before P3); the CE-3d rerun.
