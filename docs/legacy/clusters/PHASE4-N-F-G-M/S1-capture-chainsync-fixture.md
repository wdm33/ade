# Invariant Slice — PHASE4-N-F-G-M S1: Capture + fixture-pin the real cardano-node ChainSync IntersectFound

> **Status:** Planning artifact (non-normative). Normative authority is the registry + CI.

## §2 Slice Header
- **Slice:** PHASE4-N-F-G-M S1 — capture the canonical real-cardano-node ChainSync `IntersectFound`
  (responder reply to `MsgFindIntersect[Origin]`) and commit it as the corpus fixture S2's serve fix is
  pinned against.
- **Cluster:** PHASE4-N-F-G-M — Serve-side ChainSync FindIntersect compatibility.
- **Status:** done (fixtures committed) — capture clean; the serve fix is S2.
- **CE addressed:** CE-G-M-1 (capture half). [S2 = answer half; S3 = live, operator-gated.]

## §3 Dependencies
- The live C1 failure (G-L rerun): `ExceededTimeLimit (ChainSync … ServerHasAgency (SingIntersect))`.
- `ade_chain_sync_capture` (the existing capture bin) — dials a real node, `FindIntersect[Origin]`,
  records the `IntersectFound` + frames.

## §4 Intent (invariant impact)
Ground the serve ChainSync fix in REAL cardano-node bytes before changing code — the Ade↔Ade loopback
passes and structurally can't catch a real-node incompatibility (the exact lesson from G-L:
`[[feedback_real_interop_finds_codec_bugs]]`). Capture the canonical `IntersectFound` grammar from a real
cardano-node 11.0.1 (the EXACT failing c1 peer at genesis + the pre-existing preprod populated set) as
the pin target. Declares `CN-WIRE-11`; S2 enforces it.

## §5 Scope / What is built
1. **Captured fixtures:**
   - `corpus/network/n2n/chain_sync/c1privnet_origin_intersect_recv.cbor` (NEW) — the EXACT failing peer
     (c1, magic 42, genesis): `83 05 80 82 80 00` = `[IntersectFound, Origin, tip=(Origin, 0)]`.
   - `corpus/network/n2n/chain_sync/preprod_origin_5_frames_*` (pre-existing, UNTOUCHED) — preprod
     (magic 1, populated): the full `IntersectFound → RollBackward → RollForward` exchange.
2. **The pin target (for S2):** the `IntersectFound` grammar `[5, point, [tipPoint, blockNo]]` (point
   byte-fixed for the Origin intersect; tip Ade-specific). S2 adds the pin test.

**Out of scope:** the serve fix (S2); live confirmation (S3); any decoder change; BlockFetch.

## §6 Execution Boundary (TCB color)
- **RED capture tooling** (`ade_chain_sync_capture`) produced the fixture; the fixture is corpus DATA. No
  serve / codec change in S1 (the fix is S2).

## §7 Invariants
- **Declares `CN-WIRE-11`** (serve-side ChainSync FindIntersect compat) — the grounding fixture S2's
  enforcement is pinned against. No rule flips to enforced in S1.

## §11 Replay / Crash / Epoch Validation
None — ChainSync serving is pre-block-delivery RED I/O; the fixture is captured corpus data.

## §12 Mechanical Acceptance Criteria
- [x] `ade_chain_sync_capture` cleanly captured a real cardano-node 11.0.1 `IntersectFound`
  (`FindIntersect[Origin]`) from the c1 failing peer (genesis) + preprod; committed under
  `corpus/network/n2n/chain_sync/`.
- [x] The `IntersectFound` grammar is decoded + recorded (`[5, Origin, [tipPoint, blockNo]]`) in the meta.
- [ ] (S2) Ade's serve ChainSync server answers `MsgFindIntersect` with `IntersectFound` matching the
  captured grammar; a pinned test loads the captured `.cbor`.

## §14 Hard Prohibitions
Inherits cluster §11. Slice-specific: the fixture MUST be a real cardano-node capture (never Ade↔Ade); do
not synthesize the "expected" bytes; do not modify the pre-existing `preprod_origin_5_frames_*`.

## §15 Explicit Non-Goals
The serve fix (S2); live confirmation (S3); any decoder change; BlockFetch; the handshake (G-L).
