# PHASE4-N-X — Slice Plan (N2N Tag-24 Wire Envelope Authority)

> Ordered slice plan derived from `docs/planning/phase4-n-x-invariants.md`.
> Primary rule: `CN-WIRE-08` (declared → enforced). Cross-refs: `CN-FORGE-03`
> (block-fetch arm discharges its `open_obligation`), `T-ENC-01`, `DC-CONS-18`,
> `RO-LIVE-01`. Pick-up HEAD: `97faf6d`.

## Authority cut (the whole cluster in one line)

`ade_codec` owns the **tag-24 byte primitive**; `ade_network` codecs own
**per-protocol composition**; serve reducers **emit composed bytes**; RED
admission/interop **call the shared authority** (no hand-rolled parse).

## Ordered slices

### S1 — BLUE `ade_codec` tag-24 wrap/unwrap authority
- Add `wrap_tag24(&[u8]) -> Vec<u8>` and `unwrap_tag24(&[u8]) -> Result<&[u8], TagEnvelopeError>`
  to `ade_codec` (next to the block envelope codec). Closed `TagEnvelopeError`.
- Tests: wrap⇄unwrap symmetry over varied lengths (incl. the CBOR length-class
  boundaries 0/23/24/255/256/65535/65536); fail-closed on non-`0xd8 0x18`, wrong
  inner length, trailing bytes; verbatim inner copy.
- Enforces: I-1, N-7. No serve/codec wiring yet.
- TCB: BLUE only. No behavior change to any wire path.

### S2 — block-fetch composition + serve + oracle pin (discharges CN-FORGE-03)
- `ade_network::codec::block_fetch`: add `compose_blockfetch_block` /
  `decompose_blockfetch_block` calling the S1 authority; correct the misleading
  `[serialisationInfo, tag24]` comment + fixture.
- `producer_block_fetch_serve` (or the serve glue just above it) emits
  `tag24(bytes([era,block]))` instead of bare `AcceptedBlock.as_bytes()`.
- Oracle pin: assert the composed payload's shape equals the captured
  `corpus/network/n2n/block_fetch/local_preprod_tip_msg_01_block.cbor` framing
  (`82 04 d8 18 …`) — NOT an opaque round-trip (defeats N-5).
- Symmetry: `decode_block(decompose_blockfetch_block(serve_payload))` is `Ok`.
- Enforces: I-2, I-4 (block arm), N-1, N-4, N-5 (block arm). Discharges
  `CN-FORGE-03.open_obligation`.
- TCB: BLUE.

### S3 — chain-sync header composition + serve + committed golden fixture
- `ade_network::codec::chain_sync`: add `compose_rollforward_header(era_tag, hdr)`
  / `decompose_rollforward_header` = `[era_tag, tag24(bytes(header_cbor))]`;
  correct the stale comment.
- Chain-sync server emits the composed `[era_tag, tag24(hdr)]` from the bare
  `accepted_block_header_bytes` projection (ledger stays bare-projection only).
- **Q-1 obligation:** commit a golden ChainSync `RollForward` header fixture
  pinning the **served** shape (capture from docker preprod peer, or pin against
  the existing `corpus/network/n2n/chain_sync` capture with a served-shape
  assertion).
- Enforces: I-3, I-4 (header arm), N-2, N-5 (header arm).
- TCB: BLUE.

### S4 — migrate RED unwraps + CI gate + registry binding
- Replace `ade_node::admission::runner::unwrap_block_fetch_envelope` and
  `ade_core_interop::follow::project_header_from_n2n_rollforward`'s hand-rolled
  tag-24 parse with calls to the BLUE `unwrap_tag24` / decompose authority.
- New `ci/ci_check_tag24_wire_authority.sh`: (a) `wrap_tag24`/`unwrap_tag24`
  defined exactly once in `ade_codec`; (b) no hand-rolled `0xd8`/`0x18` tag-24
  parse in RED serve/admission/interop outside the authority; (c) serve paths do
  not emit bare `[era,block]` / bare header.
- Bind `CN-WIRE-08` (tests + ci_script, flip to `enforced`) at cluster close.
- Enforces: I-1 (RED arm), N-3, N-6 (gate keeps live claim out).
- TCB: RED edits + CI.

## Exit criteria → see `docs/clusters/PHASE4-N-X/cluster.md` (CE-X-1 … CE-X-N).

## Honest-fallback posture
If a deeper serve/peer mismatch surfaces beyond the tag-24 shape (e.g. point/tip
framing), land the tag-24 authority + symmetry + oracle pin with CEs proven, and
pin the next blocker as a **named follow-on**, not scope-creep into N-X. Live
peer acceptance stays `RO-LIVE-01` operator-gated regardless.
