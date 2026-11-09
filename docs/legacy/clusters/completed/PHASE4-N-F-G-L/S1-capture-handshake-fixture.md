# Invariant Slice — PHASE4-N-F-G-L S1: Capture + fixture-pin the real cardano-node V15 handshake

> **Status:** Planning artifact (non-normative). Normative authority is the registry + CI.

## §2 Slice Header
- **Slice:** PHASE4-N-F-G-L S1 — capture the canonical real-cardano-node N2N V15 `MsgAcceptVersion`
  (responder) encoding and commit it as the corpus fixture S2's encoder fix is pinned against.
- **Cluster:** PHASE4-N-F-G-L — Serve-side N2N handshake cardano-node compatibility.
- **Status:** Merged (`e42cb249`) — fixtures committed; the encoder fix landed in S2 (`853344f7`, `CN-WIRE-10` enforced + live-confirmed: the real follower reaches HOT with Ade's serve).
- **CE addressed:** CE-G-L-1 (capture half). [S2 = encode half; S3 = live, operator-gated.]

## §3 Dependencies
- The live C1 failure (G-K rerun): `HandshakeDecodeError NodeToNodeV_15 "unknown encoding: TInt 1"`.
- `ade_handshake_capture` (the existing capture bin) — dials a real node, records its `MsgAcceptVersion`
  bytes verbatim.

## §4 Intent (invariant impact)
Ground the fix in REAL cardano-node bytes BEFORE changing any code, because the Ade↔Ade hermetic
loopback already passes and structurally cannot surface a cardano-node-specific decode incompatibility
(`[[feedback_real_interop_finds_codec_bugs]]`). Capture the canonical V15 responder encoding from a real
cardano-node 11.0.1 (the exact failing C1 peer + a public preprod reference) and commit it as the pin
target. Declares `CN-WIRE-10`; S2 enforces it.

## §5 Scope / What is built
1. **Captured fixtures (committed):**
   - `corpus/network/n2n/handshake/c1privnet_v11_v16_propose_{recv,sent,meta}` — the EXACT failing peer
     (C1 private-net cardano-node 11.0.1, magic 42). recv payload `83010f84182af500f4` =
     `[1, 15, [42, true, 0, false]]`.
   - `corpus/network/n2n/handshake/preprod_v11_v16_propose_*` (pre-existing, UNTOUCHED) — the public
     preprod canonical (magic 1), recv payload `83010f8401f500f4` = `[1, 15, [1, true, 0, false]]`.
   Both: negotiated **V15** (cardano-node 11.0.1 max-common), versionData = **4-element array**.
2. **The pin target (for S2):** Ade's serve responder must emit this 4-element V15 versionData. S2 adds
   the byte-level pin test against the fixture.

**Out of scope:** the encoder fix (S2); the live confirmation (S3); any decoder change.

## §6 Execution Boundary (TCB color)
- **RED capture tooling** (`ade_handshake_capture`) produced the fixture; the fixture is corpus DATA. No
  BLUE / codec change in S1 (the codec fix is S2).

## §7 Invariants
- **Declares `CN-WIRE-10`** (serve-side N2N handshake cardano-node compatibility) — the grounding
  fixture S2's enforcement is pinned against. No rule flips to `enforced` in S1.

## §11 Replay / Crash / Epoch Validation
None — the handshake is pre-chain; the fixture is captured corpus data; no authoritative state.

## §12 Mechanical Acceptance Criteria
- [x] `ade_handshake_capture` cleanly captured a real cardano-node 11.0.1 `MsgAcceptVersion(v=15)` from
  the C1 failing peer (magic 42) + preprod (magic 1); the fixture is committed under
  `corpus/network/n2n/handshake/`.
- [x] The captured V15 versionData shape is decoded + recorded (4-element array) in the fixture meta.
- [ ] (S2) Ade's serve responder encodes byte-identically to the fixture (payload-level); a pinned test
  loads the captured `.cbor` and asserts equality.

## §14 Hard Prohibitions
Inherits cluster §11. Slice-specific: the fixture MUST be a real cardano-node capture (never an Ade↔Ade
round-trip); do not synthesize / hand-author the "expected" bytes; do not modify the pre-existing
`preprod_v11_v16_propose_*` fixture.

## §15 Explicit Non-Goals
The encoder fix (S2); live confirmation (S3); any decoder change; N2C handshake.
