# Invariant Slice — PHASE4-N-F-G-O S1: feed-side BlockFetch tag-24 unwrap before authoritative decode

> **Status:** Planning artifact (non-normative). Normative authority is the registry + CI.

## §2 Slice Header
- **Slice:** PHASE4-N-F-G-O S1 — the feed/receive BlockFetch path strips the tag-24 wire wrapper via the
  existing single `ade_codec` authority (`decompose_blockfetch_block` = `unwrap_tag24`) before `pump_block` /
  `decode_block`, so the authoritative decoder receives only bare `[era, block]`.
- **Cluster:** PHASE4-N-F-G-O — Feed-side BlockFetch tag-24 unwrap compatibility.
- **Status:** planned.
- **CE addressed:** CE-G-O-1 (the mechanical unwrap + pins + fail-closed). [S2 = live C1, operator-gated.]

## §3 Dependencies
- Captured real-node bytes (G-N S2): the feed's `block_bytes` (830 B) = `d8 18 59 03 39 82 07 …` =
  `tag24([era 7, Conway block 0, prev null])` — `decode_block` `UnexpectedType @ offset 0`.
- `decompose_blockfetch_block` / `ade_codec::unwrap_tag24` (existing, fail-closed) — the inverse of the serve
  `compose_blockfetch_block` / `wrap_tag24` (CN-WIRE-08).
- The feed: `node_sync.rs` `run_node_sync` → `pump_block` → `decode_block`; the wire pump forwards raw bytes.

## §4 Intent (invariant impact)
Close the proven feed-side crash so Ade stays alive after the follower fetches the block. Enforces
`CN-WIRE-12`. Uses the EXISTING unwrap authority — no new decoder, no `decode_block` change.

## §5 Scope / What is built
1. **Feed unwrap** — on the feed receive path, call `decompose_blockfetch_block(&wire_bytes)` (the single
   authority) before `pump_block`; pass the returned bare `[era, block]` to `pump_block` → `decode_block`. A
   non-tag-24 / malformed payload fails closed (the existing `unwrap_tag24` `Err`) as a structured feed error
   (peer drop / fail-closed run-loop), never a silent pass-through.
2. **Pin tests:** (a) the captured real-node wrapped payload (`d8 18 …`) → `decompose_blockfetch_block` → bare
   `[era, block]` → `decode_block` yields Ade's block 0 with `PrevHash::Genesis`; (b) a non-tag-24 payload →
   the unwrap fails closed (structured error), no decode attempted.
3. **Registry + CI:** `CN-WIRE-12` → enforced; a CI gate asserts the feed calls the unwrap authority before
   `decode_block`, no duplicate unwrap helper, and the serve-side wrap is unchanged.

**Out of scope:** the live C1 confirmation (S2); the admission-runner path (OQ-O1, follow-on); any serve /
forge / VRF change.

## §6 Execution Boundary (TCB color)
RED feed glue (`node_sync`) calling the single BLUE `ade_codec` tag-24 authority. `decode_block` /
`pump_block` unchanged; recovery/restart (bare WAL/db bytes) untouched.

## §11 Replay / Crash / Epoch Validation
The unwrap is a pure deterministic byte transform; same wrapped payload ⇒ same bare bytes. Covered by the S1
pin + the existing tag-24 round-trip tests. No new authoritative transition.

## §12 Mechanical Acceptance Criteria
- [ ] Captured wrapped payload unwraps to bare `[era, block]` via the single authority.
- [ ] Bare inner bytes decode as Ade's block 0 with `PrevHash::Genesis`.
- [ ] The feed receive path calls `unwrap_tag24` (via `decompose_blockfetch_block`) before `decode_block`.
- [ ] A non-tag-24 / malformed payload fails closed (structured error), no silent pass-through.
- [ ] Existing serve-side wrap tests (CN-WIRE-08 / N-X) stay green; no duplicate unwrap helper.
- [ ] `CN-WIRE-12` enforced; CI gate present.
- [ ] No regression: ade_node feed/sync + ade_runtime + ade_network block_fetch suites pass.

## §14 Hard Prohibitions
- no `decode_block` loosening; no second tag-24 unwrap implementation; no serve-side change;
- no forge / VRF / PrevHash change; fail-closed on malformed / non-tag-24 / inner-not-`[era,block]`;
- no RO-LIVE flip; no acceptance claim without the follower log through `correlate`.

## §15 Explicit Non-Goals
The live C1 confirmation (S2, operator-gated); the admission-runner unwrap (OQ-O1); durable block-1+
progression; any serve/forge/VRF change.
