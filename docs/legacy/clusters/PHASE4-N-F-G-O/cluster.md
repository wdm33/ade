# PHASE4-N-F-G-O — Feed-side BlockFetch tag-24 unwrap compatibility (CN-WIRE-12)

> **Grounded in a proven bug captured live (G-O is the fix).** With G-N's eta0 fix in, the C1 cardano-node
> follower accepted Ade's block-0 header VRF and **fetched the block** — then Ade crashed fail-closed (exit 43)
> on its FEED/receive side: `run_node_sync` → `pump_block` → `decode_block` rejected the received block with
> `Body(Decoding(UnexpectedType @ offset 0))`. The captured 830-byte payload is `d8 18 59 03 39 82 07 85 …` =
> **CBOR tag-24 (`d8 18`) wrapping `bytes(825)` = `[era 7, Conway block]`** (Ade's own block 0, slot 107405,
> prev_hash null = Genesis — echoed back from the follower). `decode_block` expects the BARE `[era, block]`
> (`0x82…`); it hit the `0xd8` tag. The feed receive path never strips the tag-24 wrapper — the missing
> mirror of the N-X serve-side wrap (`compose_blockfetch_block` / CN-WIRE-08).
> Grounding: `[[project_phase4_c1_genesis_rehearsal_live_state]]`.

## §1 Primary invariant (CN-WIRE-12)
The feed/receive BlockFetch path MUST remove the protocol tag-24 wrapper using the **single `ade_codec` unwrap
authority** (`decompose_blockfetch_block` = `ade_codec::unwrap_tag24`) before authoritative block decode.
`decode_block` / `pump_block` receive ONLY bare `[era, block]` bytes. This is the receive-side mirror of the
serve-side `compose_blockfetch_block` (`wrap_tag24`, CN-WIRE-08) — NOT a new block decoder, NOT a second
unwrap implementation. Fail-closed: a malformed tag-24, a non-tag-24 payload where the BlockFetch protocol
requires tag-24, or inner bytes that are not `[era, block]` → a structured decode error / peer drop (never a
silent pass-through or skip).

## §2 The defect (proven from captured bytes, not hypothesis)
`decode_block_fetch_message` deliberately preserves the raw `MsgBlock` payload verbatim:
`BlockFetchMessage::Block { bytes }` = `tag24(bytes([era, block]))` (block_fetch.rs:171-179, for byte-identical
round-trip). The serve path composes it via `compose_blockfetch_block` = `wrap_tag24` (block_fetch.rs:204);
the inverse authority `decompose_blockfetch_block` = `unwrap_tag24` (block_fetch.rs:210, fail-closed) EXISTS.
But the receive path forwards the wrapped bytes unchanged: the wire pump "only forwards bytes"
(`admission/wire_pump.rs` ≈468, comment ≈1092), and the feed consumer
(`node_sync.rs` `run_node_sync` → `pump_block` → `decode_block`) never calls the unwrap. So `decode_block`
gets `d8 18 …` → `UnexpectedType @ offset 0` → the relay run-loop fails closed (exit 43), tearing down the
serve. (`pump_block` is also called by recovery/restart with BARE WAL/db bytes — so the unwrap belongs on the
FEED path, NOT inside `pump_block`.)

## §3 The fix — call the existing receive-side unwrap authority on the feed
On the feed receive path, strip the tag-24 wrapper via `decompose_blockfetch_block` (the single `ade_codec`
authority) before `pump_block`, so the feed delivers bare `[era, block]` to the authoritative decoder — the
symmetric inverse of the serve's `compose_blockfetch_block`. Fail-closed on a non-tag-24 / malformed payload
(the existing `unwrap_tag24` already returns `Err`). No `decode_block` change; no second unwrap; no serve
change; recovery/restart (bare bytes) untouched.

## §6 TCB color
The closed BlockFetch wire grammar (receive-side tag-24 unwrap) — RED feed glue calling the single BLUE
`ade_codec` tag-24 authority. No new canonical type; no ledger/consensus change; the block bytes, once
unwrapped, flow through the UNCHANGED BLUE `decode_block` / `pump_block`.

## §7 Slices
| Slice | Scope | CE | Registry | Status |
|---|---|---|---|---|
| **S1** | Feed receive path calls `decompose_blockfetch_block` (the existing authority) before `pump_block`; pin to the captured real-node wrapped payload (unwrap → bare `[era, block]` → decodes as block 0 with `PrevHash::Genesis`); fail-closed test on a non-tag-24 payload | CE-G-O-1 | CN-WIRE-12 → enforced | **closed** (`f539aa7a`; live-confirmed) |
| **S2** | Live C1 rerun: Ade's feed no longer crashes on the echoed tag-24 block; serve stays alive; `correlate` decides whether the follower adopted | CE-G-O-2 | operator-gated | **partial** — decode crash gone live (`UnexpectedType` count = 0); a SEPARATE feed-side header-VRF blocker now tears the feed down before serve/`correlate` → PHASE4-N-F-G-P |

## §8 Cluster Exit Criteria
- **CE-G-O-1 (mechanical):**
  1. The captured real-node wrapped payload (`d8 18 59 03 39 …`) unwraps to bare `[era, block]` via the single authority.
  2. The bare inner bytes decode as Ade's block 0 with `PrevHash::Genesis`.
  3. The feed receive path calls `unwrap_tag24` (via `decompose_blockfetch_block`) before `decode_block`.
  4. Existing serve-side wrap tests (CN-WIRE-08 / N-X) stay green.
  5. No duplicate tag-24 unwrap helper.
  6. (covered by S2) the C1 feed no longer crashes with `UnexpectedType @ offset 0`.
- **CE-G-O-2 (operator-gated):** a C1 rerun shows Ade's feed path no longer crashes on the echoed tag-24
  block, the serve remains alive, and the follower's adoption (or not) is decided only by the follower log
  through `correlate`. `blocked_until_operator_c1_genesis_successor_rehearsal`; no RO-LIVE flip.

## §9 Replay obligations
The unwrap is a pure, deterministic byte transform (same wrapped payload ⇒ same bare bytes). No new
authoritative transition; covered by the S1 pin + the existing tag-24 round-trip tests.

## §10 Invariants
- **Adds:** `CN-WIRE-12` (feed-side BlockFetch tag-24 unwrap), declared → enforced at S1.
- **Preserves / cross-ref:** `CN-WIRE-08` (serve-side tag-24 wrap — the mirror, unchanged), `CN-WIRE-10/11`
  (handshake / FindIntersect), `T-REC-04` + `DC-CINPUT-03` (G-N eta0), `DC-SYNC-02` (durable-before-tip),
  `RO-LIVE-01` (no flip).

## §11 Forbidden during this cluster (hard boundaries)
- **no `decode_block` loosening** — it still requires bare `[era, block]`.
- **no second tag-24 unwrap implementation** — use the existing `decompose_blockfetch_block` / `ade_codec::unwrap_tag24`.
- **no serve-side change** (the serve wrap is correct; the follower unwrapped Ade's block fine).
- **no forge / VRF / PrevHash change.**
- **fail-closed** on malformed tag-24, non-tag-24 where required, or inner-not-`[era, block]` → structured
  error / peer drop (never a silent pass-through, skip-past, or fallback).
- **no RO-LIVE flip; no acceptance claim** without the follower log through `correlate`.

## §12 Open questions
- **OQ-O1:** the admission runner (`--mode admission` `process_block`) consumes the SAME wire-pump
  `AdmissionPeerEvent::Block` and would hit the identical wrapped-block decode — but it admitted nothing at
  C1 genesis, so it is unexercised. Whether to also unwrap there (or share a single feed/receive unwrap
  point) is a follow-on; G-O is scoped to the FEED path that the C1 rehearsal exercises.

## §13 Close record — S1 (2026-06-04)
**G-O CLOSED with a narrow claim.** `CN-WIRE-12` enforced: the feed/receive BlockFetch path strips the tag-24
wrapper via the single `ade_codec` authority (`decompose_blockfetch_block`) before `decode_block`, emitting
bare `[era, block]`; fail-closed `BlockFetchDecode` on a non-tag-24 payload. Mechanical (CE-G-O-1): the
genesis-successor pin `feed_unwrap_decodes_genesis_successor_block_zero` (captured-shape wrapped block 0 →
unwrap → decode → block 0 / `PrevHash::Genesis`) + the wire-pump unit pins
(`block_fetch_unwraps_tag24_emitting_bare_block`, `block_fetch_fails_closed_on_non_tag24_payload`) + the
end-to-end node-spine serve loopback (now bare delivery) + `ci/ci_check_feed_tag24_unwrap.sh`. The serve-side
wrap (CN-WIRE-08) is unchanged. (Preceded by a G-M test-scope hotfix, `275a2318`: G-M's raw-CBOR `c1privnet_*`
fixtures had broken `chain_sync_real_capture_corpus` — a latent workspace-red since G-M.)

**LIVE-CONFIRMED (narrow, CE-G-O-1.6):** the C1 `--mode node` rerun (2026-06-04 12:23Z, both fixes in the
binary) shows the old crash `Body(Decoding(UnexpectedType @ offset 0))` is GONE (`UnexpectedType` count = 0) —
the feed now DECODES the tag-24-wrapped block.

**NOT claimed:** block adopted (this run); serve stays alive; C1 rehearsal complete; RO-LIVE flip;
preprod/bounty success. CE-G-O-2's serve-alive / `correlate`-adoption half is NOT reached — a separate
receive-side blocker tears the feed down first. (A real cardano-node DID adopt an Ade-forged genesis-successor
block — `ChainDB AddedToCurrentChain` / `ValidCandidate`, slot 107405, hash `52e3ae88…be2652d`, 11:20:17Z —
but in a PRIOR run; strong standing evidence, NOT a project acceptance claim until `correlate` binds the
follower log.)

**NEW separate blocker → PHASE4-N-F-G-P (Feed-side header VRF eta0 fidelity):** with the body decode fixed,
the feed advances to header validation and fails closed —
`Receive(Validity(Header(VrfCert(VerificationFailed))))`. Ade's feed (recovered tip = genesis) re-syncs the
follower's tip (Ade's own slot-107405 block, which the real node validated) and REJECTS its header VRF. Strong
hypothesis: the receive/pump-path validator's eta0 sourcing (genesis/ZERO vs the recovered `953a4c34`) — the
RECEIVE-side mirror of G-N's FORGE eta0 bug. Proof-first: instrument the feed/header validator to log
`(block_no, slot, eta0, praos_vrf_input, pool_vrf_keyhash, proof len/hash)`, rerun C1, THEN fix; do NOT fix
from hypothesis.
