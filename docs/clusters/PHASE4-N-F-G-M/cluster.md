# PHASE4-N-F-G-M — Real cardano-node ChainSync FindIntersect compatibility (CN-WIRE-11)

> **SCOPE CORRECTED (2026-06-04) — the live fixture changed the truth.** The original G-M expectation was
> "Ade decodes the request → fails to *answer* `MsgFindIntersect`" (a RESPONSE-grammar defect). An
> instrumented C1 rerun proved the failure is **earlier, in the REQUEST DECODE**: the real cardano-node
> follower sends `MsgFindIntersect` bytes `82 04 9f 80 80 ff`, and Ade's ChainSync decoder **rejects** them
> (`indefinite-length array not allowed`) → dispatch drops the frame → no reply → the follower times out at
> `SingIntersect`. So G-M's true scope is **real cardano-node ChainSync FindIntersect compatibility**:
> (A) request decode accepts the real points-list encoding, and (B) the reducer replies `IntersectFound[Origin]`
> for an Origin intersection. This is NOT "loosen the decoder to accept junk" — it is *accept the real
> cardano-node ChainSync FindIntersect points-list encoding*, which is the exact compatibility target.
>
> Grounding: `[[project_phase4_c1_genesis_rehearsal_live_state]]`; the live failure + the captured bytes are
> reproduced byte-identically across 9 consecutive follower reconnects.

## §0 Slices with sharply different IDD status
- **Mechanical, fixture-pinned (S1 + S2):** S1 captured the canonical real-node `IntersectFound` *reply*
  (`c1privnet_origin_intersect_recv.cbor`, done). S2 captures the real-node `MsgFindIntersect` *request*
  (`c1privnet_follower_findintersect_recv.cbor`) and makes Ade (A) **decode** it and (B) **reply**
  `IntersectFound[Origin]`. Closes on the captured-fixture pins + a re-capture check, NOT Ade↔Ade.
- **Operator-gated (S3):** the live C1 confirmation — the follower's hot session sustains (no `SingIntersect`
  timeout), proceeds intersect → `RequestNext` → toward header/BlockFetch — stays operator-gated; resumes the
  rehearsal. No RO-LIVE flip; acceptance only via the follower log through `correlate`.

## §1 Primary invariant (CN-WIRE-11)
Real cardano-node ChainSync `FindIntersect` compatibility, served from the **single** `ServedChainView`
authority (G-B/G-H/G-J), in two halves:
- **(A) Request decode** — Ade's ChainSync decoder MUST accept the points list of `MsgFindIntersect` whether a
  real cardano-node encodes it as a CBOR **indefinite-length** array (`9f … ff`) or Ade encodes it
  definite-length. Scoped to **that list only**: the outer message array, the points themselves, and the tip
  stay strictly definite (`decode_array_header` is unchanged for them).
- **(B) Reply** — `Origin` is the universal common ancestor (every chain descends from genesis), so a
  `FindIntersect` whose points include `Origin` MUST be answered `IntersectFound[Origin]` (matching the real
  cardano-node, which answers `IntersectFound[Origin]` even with an empty chain). Block points still resolve
  through the existing closed `ServedHeaderLookup::intersect`; the reply is served from `ServedChainView`,
  never a placeholder/silence.

## §2 The defect + the captured fixtures (proven, not assumed)
Observed (instrumented live C1 rerun, real cardano-node 11.0.1 follower, hot peer, 9 identical reconnects):
```
peer_connected cs_v=ChainSyncVersion(15)                 <- G-L holds: handshake completes, V15
cs_decode_ERR  bytes=[82,04,9f,80,80,ff]
               err="indefinite-length array not allowed"  <- the actual blocker (request decode)
dispatch_err   ReducerError                                <- frame dropped -> silence -> SingIntersect timeout
```
Captured fixtures (both real cardano-node 11.0.1, magic 42, genesis):
```
REQUEST (S2, c1privnet_follower_findintersect_recv.cbor):
  82 04 9f 80 80 ff  = MsgFindIntersect { points: [Origin, Origin] }, points list = INDEFINITE array (9f..ff)
REPLY   (S1, c1privnet_origin_intersect_recv.cbor):
  83 05 80 82 80 00  = MsgIntersectFound [Origin, [Origin, 0]]
```
Ade↔Ade loopback passed because Ade *encodes* the points list definite-length (`81 80` for `[Origin]`), so it
round-trips its own bytes — the real node uses indefinite, which Ade rejected. (The "real interop finds codec
bugs" pattern: `[[feedback_real_interop_finds_codec_bugs]]`.)

## §3 Root cause (resolved, not a lead)
- **(A)** `crates/ade_network/src/codec/chain_sync.rs` `decode_chain_sync_message` decodes the `FindIntersect`
  points list via `decode_array_header`, which rejects indefinite-length arrays — so the real-node request
  never decodes. Fix: a **scoped** two-form points-list decoder (definite OR indefinite, `ff`-break-required,
  full-consume), used by THIS list only.
- **(B)** `crates/ade_network/src/chain_sync/server.rs` `producer_chain_sync_serve` FindIntersect arm calls
  `served.intersect(&points)`, whose impl (`ade_runtime/.../served_chain_lookups.rs`) only matches
  `Point::Block` and ignores `Point::Origin` → would reply `IntersectNotFound` even once decode succeeds. Fix:
  resolve `Origin` (universal ancestor) → `IntersectFound[Origin]` in the reducer, keeping block-point
  resolution on the existing closed lookup.

## §6 TCB color
The closed Cardano **ChainSync wire grammar** (`ade_network` codec + the serve ChainSync server) — a
deterministic codec/protocol authority, a closed semantic surface. NOT the ledger/consensus BLUE core. No new
canonical type; the served chain is already-self-accepted state (no replay weight).

## §7 Slices
| Slice | Scope | CE | Registry | Status |
|---|---|---|---|---|
| **S1** | Capture the canonical real-node `IntersectFound` *reply* (genesis + the pre-existing preprod set); pin the reply grammar | CE-G-M-1 (reply half) | CN-WIRE-11 (declared) | done |
| **S2** | Capture the real-node `MsgFindIntersect` *request*; (A) accept its points-list encoding on decode + (B) reply `IntersectFound[Origin]`, both fixture-pinned | CE-G-M-1 (decode+reply) | CN-WIRE-11 → enforced | planned |
| **S3** | Live C1 confirmation: the follower's hot session sustains (no `SingIntersect` timeout) + proceeds toward header/BlockFetch | CE-G-M-2 | operator-gated | planned |

## §8 Cluster Exit Criteria
- **CE-G-M-1 (mechanical):** fed the captured real-node `MsgFindIntersect` bytes `82 04 9f 80 80 ff`, Ade
  (A) decodes `FindIntersect { points: [Origin, Origin] }` (indefinite points list accepted) and (B) the serve
  ChainSync server replies `IntersectFound[Origin]` with a valid `[tipPoint, blockNo]` tip (= Ade's served
  tip) served from `ServedChainView`. Pinned to the captured request + reply fixtures, NOT Ade↔Ade.
- **CE-G-M-2 (operator-gated):** a real cardano-node follower's hot session SUSTAINS (no `ExceededTimeLimit` at
  `SingIntersect`) — proceeds intersect → `RequestNext` → toward header/BlockFetch of block 0. Resumes the
  rehearsal. `blocked_until_operator_c1_genesis_successor_rehearsal`; no RO-LIVE flip; acceptance only via the
  follower log through `correlate`.

## §9 Replay obligations
None new — ChainSync serving is RED I/O over already-self-accepted state; no canonical type, no authoritative
transition.

## §10 Invariants
- **Preserves:** `DC-NODE-07` (single shared serve), `DC-NODE-09` (serve lifetime), `CN-WIRE-08` (tag-24),
  `CN-WIRE-10` (handshake — unchanged), `RO-LIVE-01/06` (no flip).
- **Adds:** `CN-WIRE-11` (real cardano-node ChainSync FindIntersect compatibility: request-decode + Origin
  reply), declared at S1 → enforced at S2.

## §11 Forbidden during this cluster (hard boundaries)
- **no broad indefinite-array support** — indefinite acceptance is scoped to the `MsgFindIntersect` points list
  ONLY; `decode_array_header` stays definite-only for every other array (outer message, points, tip, and all
  other mini-protocols).
- **no decoder fallback** — no catch-all, no "accept unknown CBOR"; the two-form points decoder requires the
  `ff` break and full-consumes the message.
- **no ChainSync semantic widening beyond Origin intersect** — (B) adds only `Origin`-as-universal-ancestor;
  block-point resolution is unchanged.
- **no BlockFetch changes; no handshake changes; no forge / PrevHash changes.**
- **no second serve authority** (answer from the single `ServedChainView`).
- **no RO-LIVE flip; no acceptance claim** until the follower log through `correlate` says so.
- the validation fixtures MUST be real cardano-node captures, never Ade↔Ade.

## §12 Open questions (resolved by the live capture)
- **OQ-M1 (resolved):** `serve_dispatch` DOES route `MsgFindIntersect` to `producer_chain_sync_serve`; the
  failure is upstream in the codec decode, not a missing handler.
- **OQ-M2 (resolved):** the real follower's points are `[Origin, Origin]` encoded as a CBOR indefinite-length
  array; Ade must accept that encoding. No HardForkBlock-specific point encoding is involved — the points are
  plain `Origin` (`80`).
- **OQ-M3 (S3):** after `IntersectFound[Origin]` the follower sends `RequestNext`; whether the served block-0
  `RollForward` + BlockFetch then succeed is the S3 live question (and a possible sibling blocker if BlockFetch
  uses the same indefinite encoding — out of scope here).
