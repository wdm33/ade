# SLICE: LEDGER-VALUE-QUANTITY-CORRECTNESS / S1 — Output asset quantity is Word64

## Narrow claim (the only thing this slice proves)

> Ade's authoritative UTxO **output** representation preserves the full valid Cardano
> Word64 asset-quantity domain (`0 ..= 2^64-1`).

Mint/burn decoding and signed conservation remain **separate future work** (S-13). This slice
does NOT touch them beyond defining a dormant boundary type.

## Why (grounding)

The Stage-2 MemPack decoder (`DC-MITHRIL-02`) decodes real preprod outputs with multi-asset
quantities at and above `i64::MAX` (one observed = 1.8e19 > i64::MAX). Ade's authoritative value
model stores asset quantities as `i64`, so promoting a decoded snapshot UTxO into `UTxOState`
currently crosses an authority boundary (loss/rejection of valid outputs). This slice fixes the
authoritative representation. It is the prerequisite for Stage-3 Mithril bootstrap integration.

## Typed model — NEWTYPES, not aliases

Both live in the shared value-types layer **`crates/ade_types/src/mary/value.rs`** (alongside
`AssetName`/`MultiAsset`; `Coin(u64)` is a sibling in `ade_types/src/tx.rs`). `ade_ledger` imports
`OutputAssetQuantity` from `ade_types` (it must NOT define its own).

```rust
/// A UTxO OUTPUT asset quantity: the non-negative Cardano Word64 domain (0 ..= 2^64-1).
/// Non-negative BY CONSTRUCTION; canonical output encoding is u64; checked add/sub only;
/// cannot be passed where a mint/burn delta is expected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct OutputAssetQuantity(pub u64);

impl OutputAssetQuantity {
    pub const ZERO: Self = OutputAssetQuantity(0);
    pub fn checked_add(self, o: Self) -> Option<Self> { self.0.checked_add(o.0).map(OutputAssetQuantity) }
    pub fn checked_sub(self, o: Self) -> Option<Self> { self.0.checked_sub(o.0).map(OutputAssetQuantity) }
    pub fn is_zero(self) -> bool { self.0 == 0 }
}

/// A mint/burn DELTA: the signed domain. DORMANT until mint decoding (S-13).
/// Cannot enter UTxO output state (never placed in MultiAsset). Defined here only to fix the
/// future boundary so the distinction is explicit, per the value-model correction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct MintBurnQuantity(pub i64);
```

**Do NOT use a universal `i128`.** It is wide enough but blurs the non-negative-stored-output vs
signed-mint/burn distinction. Storage types are `u64` (output) and `i64` (mint), distinct.

## Implementation order (do all, one mergeable unit — do NOT commit intermediate pieces)

1. **Introduce `OutputAssetQuantity(u64)` + dormant `MintBurnQuantity(i64)`** in
   `ade_types/src/mary/value.rs`. Migrate BOTH `MultiAsset` defs
   (`ade_types/src/mary/value.rs:20`, `ade_ledger/src/value.rs:22`) from
   `BTreeMap<AssetName, i64>` → `BTreeMap<AssetName, OutputAssetQuantity>`. `ade_ledger` re-uses the
   `ade_types` newtype (import it; do not duplicate). The two `AssetName` types stay as they are
   (pre-existing duplication, out of scope).
2. **Migrate snapshot encode/decode** (`ade_ledger/src/snapshot/utxo_state.rs` ~239-316:
   `write_multi_asset` / `read_multi_asset` / `write_int_i64` / `read_int_i64`). The OUTPUT quantity
   encodes as a canonical CBOR **unsigned** integer (u64), so a quantity > i64::MAX round-trips. For
   representable values (≤ i64::MAX) the bytes are IDENTICAL (a non-negative int and a u64 ≤ i64::MAX
   both encode as the same CBOR major-0 uint) — that is the byte-identity guarantee. Keep the signed
   `write/read_int_i64` only if still needed elsewhere (mint is opaque, so likely remove from the
   output path).
3. **Migrate output arithmetic** (`ade_ledger/src/value.rs`: `value_add` / `value_sub` /
   `multi_asset_add` / `multi_asset_sub`) to checked u64.
   - `multi_asset_add`: `checked_add`; overflow → a structured `LedgerError` (reuse/extend the
     existing conservation/value error family).
   - `multi_asset_sub` / `value_sub`: **CRITICAL** — an output underflow (subtrahend qty > minuend
     qty for an asset) MUST return a structured authoritative `LedgerError` (add a variant, e.g.
     `AssetUnderflow { policy, name }`). It must NOT wrap and MUST NOT silently produce/delete a
     negative entry. The existing `*current -= qty` (value.rs:179) is the unchecked site to fix.
   - `prune_zeros` (removing an asset whose quantity became exactly 0 after a checked sub) is
     **canonical Cardano value normalization** — KEEP it, and note in a comment that it is canonical
     (this is the "already canonically specified" normalization the reviewer flagged; zero removal is
     not silent deletion of value, it is the specified empty-bundle form).
4. **Remove the output-domain non-negative check** now type-impossible: `check_non_negative`
   (value.rs:131) — the multi-asset `qty < 0` loop is dead once quantities are `u64`. Its only caller
   is `mary.rs:65`; keep the coin/structural part if any remains, drop the asset-sign part, and
   adjust the caller (the negative-asset path is now unreachable by type).
5. **Update conservation + fixtures**: `mary.rs:131/144` (output value sums — now u64; `:206`
   `value_add(consumed_ma, minted)` stays — `minted` is the empty placeholder, so it remains a no-op
   until S-13; do NOT change its shape). Update the in-file value.rs tests, the snapshot
   `utxo_state.rs` negative-quantity test (`~:415` — it asserted a negative output qty round-trips;
   that is now type-impossible for outputs → convert it to a u64 > i64::MAX round-trip test, OR move
   it to the dormant mint boundary), and any `conway_conservation_*` fixtures (output fixtures use
   `OutputAssetQuantity`).
6. **Dormant `MintBurnQuantity(i64)`** — define it (step 1) and reference it only where it marks the
   future boundary (a doc comment on `parse_mint_field` / the mint field). Do NOT thread it through
   live output arithmetic or `MultiAsset`.
7. **Prove byte-identity**: the existing replay / snapshot / codec tests must pass UNCHANGED (same
   canonical bytes for representable values). Add an explicit test: a value with asset qty ≤ i64::MAX
   encodes to the SAME bytes as before the change (golden/round-trip).
8. **Prove the > i64::MAX path end-to-end**: a test that takes an output asset quantity > i64::MAX
   (e.g. `i64::MAX as u64 + 1`, and `u64::MAX`), builds the canonical value / UTxOState, encodes the
   snapshot, decodes it back, and asserts the quantity is preserved exactly. If feasible, drive it
   from a `read_txout` (MemPack) decode → `UTxOState` to exercise the real Stage-2 → value-model seam.

## Forbidden (mechanically gated — `ci/ci_check_value_quantity_domain.sh`)

- NO `u64 → checked i64 → reject` adapter anywhere on the output path.
- NO `i128` storage type (the value model stores u64/i64; i128 is reserved for FUTURE transient
  conservation arithmetic, not this slice).
- NO truncating `as i64` / `as u64` cast on the value/quantity path.
- NO silent zero-asset deletion beyond the canonical `prune_zeros` (which stays, documented).

## Acceptance criteria (all required; tests are the proof)

- [ ] every valid Word64 output quantity round-trips EXACTLY (incl. `i64::MAX`, `i64::MAX+1`, `u64::MAX`);
- [ ] negative output quantities are UNREPRESENTABLE (type-level: `OutputAssetQuantity(u64)`);
- [ ] mint/burn stays SIGNED (`MintBurnQuantity(i64)`) and CANNOT enter UTxO outputs (type-level: `MultiAsset` holds `OutputAssetQuantity`);
- [ ] canonical encoding is unambiguous (output quantity = CBOR unsigned int);
- [ ] ALL output arithmetic uses CHECKED operations (overflow AND underflow → structured `LedgerError`, never wrap/negative);
- [ ] the old representable corpus produces BYTE-IDENTICAL canonical outputs (existing tests green, + an explicit byte-identity test);
- [ ] a u64 quantity > i64::MAX survives snapshot decode → UTxOState → persisted snapshot → recovery;
- [ ] replay + hash + conservation tests cover the output domain.

## CI gate

`ci/ci_check_value_quantity_domain.sh` (new): assert both `MultiAsset` defs use `OutputAssetQuantity`
(no `AssetName, i64` remains); `OutputAssetQuantity` + `MintBurnQuantity` are newtypes in
`ade_types/src/mary/value.rs`; no `as i64`/`as u64` on the value path; output arithmetic is checked
(`AssetUnderflow` variant present); `MintBurnQuantity` is not used as a `MultiAsset` value type; the
hermetic value + snapshot tests pass.

## Invariant

Add `DC-LEDGER-VALUE-01` (tier=true) to `docs/ade-invariant-registry.toml`: "Ade's authoritative UTxO
output asset quantity preserves the full Cardano Word64 domain (0..2^64-1) via the
`OutputAssetQuantity(u64)` newtype; output arithmetic is checked (overflow/underflow → structured
error, never wrap or negative); mint/burn is the distinct signed `MintBurnQuantity(i64)` and cannot
enter outputs; representable values stay byte-identical." Cross-ref `DC-MITHRIL-02`.

## Scope fence

OUTPUT domain ONLY. Mint decoding + signed conservation = separate future work (S-13).
`MintBurnQuantity` is defined but dormant. Do NOT touch Mithril routing / admission / ECA wiring.
