# Invariant Slice — PHASE4-N-F-G-M S2: FindIntersect request-decode compat + Origin reply

> **Status:** Planning artifact (non-normative). Normative authority is the registry + CI.

## §2 Slice Header
- **Slice:** PHASE4-N-F-G-M S2 — make Ade's serve-side ChainSync server (A) **decode** a real cardano-node
  `MsgFindIntersect` whose points list is a CBOR indefinite-length array, and (B) **reply**
  `IntersectFound[Origin]` for an Origin intersection — both pinned to captured real-node fixtures.
- **Cluster:** PHASE4-N-F-G-M — Real cardano-node ChainSync FindIntersect compatibility.
- **Status:** planned.
- **CE addressed:** CE-G-M-1 (decode + reply). [S3 = live, operator-gated.]

## §3 Dependencies
- The instrumented C1 rerun (2026-06-04): the follower's real `MsgFindIntersect` bytes `82 04 9f 80 80 ff`
  and the decode reject `indefinite-length array not allowed`, reproduced across 9 reconnects.
- Fixtures: `corpus/network/n2n/chain_sync/c1privnet_follower_findintersect_recv.cbor` (request, S2) +
  `c1privnet_origin_intersect_recv.cbor` (reply, S1).
- `decode_chain_sync_message` / `decode_array_header` (codec); `producer_chain_sync_serve` (reducer);
  `ServedHeaderLookup::intersect` (lookup).

## §4 Intent (invariant impact)
Close the live blocker that holds the C1 genesis rehearsal at `SingIntersect`. The fix is grounded in the
REAL cardano-node bytes (not the Ade↔Ade loopback, which passes because Ade round-trips its own definite
encoding). Enforces `CN-WIRE-11`.

## §5 Scope / What is built
1. **(A) Decode** — a scoped two-form points-list decoder in `ade_network` codec: the `MsgFindIntersect`
   points list is accepted whether definite- or indefinite-length (`9f … ff`). Each element is the existing
   closed `decode_point`; the indefinite form requires the `ff` break; the message is full-consumed.
   `decode_array_header` is UNCHANGED (definite-only) for every other array.
2. **(B) Reply** — `producer_chain_sync_serve` FindIntersect arm resolves the first client-listed point on the
   served chain; `Origin` always intersects (universal ancestor) → `IntersectFound[Origin]`; block points use
   the existing closed `intersect`; no match → `IntersectNotFound`.
3. **Pin tests** (real-node fixtures): decode `c1privnet_follower_findintersect_recv.cbor` →
   `FindIntersect{[Origin, Origin]}`; reducer fed that → `IntersectFound[Origin]` with a 2-element tip; the
   reply grammar matches `c1privnet_origin_intersect_recv.cbor` (`[5, Origin, [tipPoint, blockNo]]`).
4. **CI gate** + **registry** `CN-WIRE-11` → enforced.

**Out of scope:** live confirmation (S3); BlockFetch; handshake; forge/PrevHash.

## §6 Execution Boundary (TCB color)
Closed ChainSync wire grammar (`ade_network` codec + serve reducer). No ledger/consensus BLUE change; the
served chain is already-self-accepted state. No new canonical type, no replay weight.

## §7 Invariants
- **Enforces `CN-WIRE-11`** (real cardano-node ChainSync FindIntersect compat: request-decode + Origin reply).
- **Preserves:** `DC-NODE-07/09`, `CN-WIRE-08/10`, `RO-LIVE-01/06` (no flip).

## §11 Replay / Crash / Epoch Validation
None — ChainSync serving is pre-block-delivery RED I/O; the codec/reducer are pure + deterministic
(round-trip + replay-equivalence covered by the existing server-reducer tests).

## §12 Mechanical Acceptance Criteria
- [ ] A pinned test decodes `c1privnet_follower_findintersect_recv.cbor` to
  `FindIntersect { points: [Origin, Origin] }` (indefinite points list accepted).
- [ ] A pinned test drives `producer_chain_sync_serve(Idle, FindIntersect[Origin], served-with-block0)` →
  `IntersectFound { point: Origin, tip: <2-element, Ade's served tip> }`; the reply grammar matches
  `c1privnet_origin_intersect_recv.cbor`.
- [ ] `decode_array_header` still rejects indefinite-length arrays (the scope guard): a test/gate proves the
  general array decoder is unchanged.
- [ ] CN-WIRE-11 enforced in the registry; a CI gate asserts the scoped decode + the Origin reply + the
  no-broadening guard.
- [ ] No regression: the existing ChainSync codec round-trip + server-reducer + handshake suites pass.

## §14 Hard Prohibitions
- no broad indefinite-array support (scoped to the FindIntersect points list ONLY);
- no decoder fallback / no catch-all / no unknown-CBOR acceptance; require the `ff` break + full-consume;
- no ChainSync semantic widening beyond the Origin intersect;
- no BlockFetch / handshake / forge / PrevHash change; no second serve authority;
- no RO-LIVE flip; no acceptance claim without the follower log through `correlate`;
- fixtures MUST be real cardano-node captures, never Ade↔Ade.

## §15 Explicit Non-Goals
Live confirmation (S3, operator-gated); BlockFetch; the initiator ChainSync path; durable progression; the
handshake (G-L, done).
