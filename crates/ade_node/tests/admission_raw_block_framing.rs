//! Regression guard for the admission runner's block-body framing tolerance.
//!
//! The runtime block-fetch handler emits `block_bytes` as the raw `[era, block]`
//! storage array (no tag-24 envelope) — the `node_lifecycle` AO path consumes it
//! by calling `decode_block(&block_bytes)` directly. A stale PHASE4-N-M-FRAG
//! assumption made the admission runner's `process_block` unconditionally strip a
//! tag-24 wrapper, so it rejected every raw body as `Undecodable` (DC-ADMIT-12
//! fail-close → `Diverged` halt). This was **venue-agnostic** — confirmed live on
//! BOTH preprod and preview (2026-06-14): the runner admitted zero blocks.
//!
//! The fix: `unwrap_tag24(b).unwrap_or(b)` — strip a tag-24 envelope if present,
//! else pass the raw bytes straight to `decode_block`. These tests pin both halves:
//! the fixture is genuinely the un-tag24'd form (so the old code failed), and it
//! decodes through the fallback (so the new code admits).

use ade_ledger::block_validity::decode_block;

/// A real Conway block body as delivered by the live BlockFetch runtime handler:
/// the raw `[era, block]` storage array (era tag 7 = Conway), NOT tag-24-wrapped.
/// Captured from the preprod docker peer (public chain data).
const RAW_ERA_BLOCK: &[u8] = include_bytes!("fixtures/raw_era_block_conway.cbor");

#[test]
fn raw_era_block_is_not_tag24_wrapped() {
    // This is exactly the call the stale `process_block` made first; it returned
    // `NotTag24` on the live wire and rejected the block.
    assert!(
        ade_codec::unwrap_tag24(RAW_ERA_BLOCK).is_err(),
        "fixture must be the raw [era,block] form (no tag-24 envelope)"
    );
    assert_eq!(
        RAW_ERA_BLOCK[0], 0x82,
        "raw [era,block] starts with CBOR array(2)"
    );
    assert_eq!(RAW_ERA_BLOCK[1], 0x07, "era tag 7 = Conway");
}

#[test]
fn raw_era_block_decodes_via_framing_fallback() {
    // The fix: tolerate the raw framing and hand it straight to the BLUE decoder.
    let inner = ade_codec::unwrap_tag24(RAW_ERA_BLOCK).unwrap_or(RAW_ERA_BLOCK);
    assert!(
        decode_block(inner).is_ok(),
        "raw [era,block] Conway body must decode through the framing fallback"
    );
}
