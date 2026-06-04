# PHASE4-N-F-G-M — Serve-side ChainSync FindIntersect compatibility (CN-WIRE-11)

> **Grounded in a live failure + a captured real-node fixture.** After G-L fixed the handshake, the C1
> rerun showed the real cardano-node follower reach HOT with Ade's serve — then its ChainSync hot session
> times out ~10 s in: `ExceededTimeLimit (ChainSync … ServerHasAgency (SingIntersect))`. The follower
> (ChainSync client, MiniProtocol 2) sent `MsgFindIntersect` (HardForkBlock points) and Ade's serve
> ChainSync **server had agency at `SingIntersect` and never replied** → demote/cycle; no block fetched.
> S1 captured the canonical real-node `IntersectFound` reply
> (`corpus/network/n2n/chain_sync/c1privnet_origin_intersect_recv.cbor` — the EXACT failing peer, magic
> 42, genesis — plus the pre-existing `preprod_origin_5_frames_*`, magic 1, populated).
>
> Grounding: `[[project_phase4_c1_genesis_rehearsal_live_state]]`; the live failure transcript is the
> follower docker log.

## §0 Slices with sharply different IDD status
- **Mechanical, fixture-pinned (S1 + S2):** capture the canonical real-node ChainSync `IntersectFound`
  (S1 — done, fixtures committed) + make Ade's serve ChainSync server answer `MsgFindIntersect` with that
  grammar (S2). Closes on the captured-fixture pin + a re-capture check, NOT Ade↔Ade.
- **Operator-gated (S3):** the live C1 confirmation — the follower's hot session sustains (no
  `SingIntersect` timeout), completes intersect → `RequestNext` → toward BlockFetch of block 0 — stays
  operator-gated; resumes the rehearsal. No RO-LIVE flip.

## §1 Primary invariant (CN-WIRE-11)
Ade's serve-side ChainSync server must answer a real cardano-node follower's `MsgFindIntersect` at
`SingIntersect` using the closed Cardano ChainSync grammar — `MsgIntersectFound [point, [tipPoint,
blockNo]]` (or `MsgIntersectNotFound [tip]`) — served from the **single** `ServedChainView` authority
(G-B/G-H/G-J). No second serve authority; no change to forged-block semantics; the server answers from
the served chain, never a placeholder/silence.

## §2 The defect + the captured fixture (proven, not assumed)
Observed (live C1, real cardano-node 11.0.1 follower, hot peer):
`ExceededTimeLimit (ChainSync … ServerHasAgency (SingIntersect))` ~10 s in — Ade's serve ChainSync server
didn't answer `MsgFindIntersect`.

Captured canonical `IntersectFound` (S1), real cardano-node 11.0.1, `FindIntersect[Origin]`:
```
MsgIntersectFound = [5, point, [tipPoint, blockNo]]
c1 (magic 42, genesis):    83 05 80 82 80 00                 = [5, Origin, [Origin, 0]]
preprod (magic 1, popul.): 83 05 80 82 82 [slot,hash] [blkNo]  (corpus preprod_origin_5_frames_*)
```
The canonical sequence is `FindIntersect → IntersectFound → (RequestNext) RollBackward[Origin] →
(RequestNext) RollForward[block]` (the full grammar is in `preprod_origin_5_frames_*`).

## §3 Root-cause lead (S2's first task) + loci
The follower (ChainSync client) sends `MsgFindIntersect`; Ade's serve must reply `IntersectFound`. The
Ade↔Ade loopback passed (Ade's own client's intersect differs / Ade-server handles Ade-client), but the
real cardano-node times out → Ade's serve ChainSync server doesn't answer the real `MsgFindIntersect`.
Loci: `ade_runtime/src/network/serve_dispatch.rs` (`PeerN2nServerChainSyncFrame` →
`OutboundCommand::ChainSync` — does it handle `MsgFindIntersect`, or only `RequestNext`?), the ChainSync
server reducer it calls, and `ade_network/src/codec/chain_sync.rs` (decode `MsgFindIntersect` incl. the
HardForkBlock points / encode `IntersectFound`). S2 roots out the exact gap against the fixture.

## §3.1 The pin nuance (read before S2)
`IntersectFound`'s `tipPoint`/`blockNo` are **node-specific**: the c1 node (empty chain) reports
`tip=(Origin, 0)`; Ade (chain has self-accepted block 0) reports `tip=(block-0 point, 0)`. So the pin is
the `IntersectFound` GRAMMAR — tag 5 + `point` (byte-fixed `0x80` for the Origin intersect) + a 2-element
`[tipPoint, blockNo]` tip — NOT byte-identity of the dynamic tip. The captured fixture is the grammar
authority; Ade emits its own served tip.

## §6 TCB color
The closed Cardano **ChainSync wire grammar** (`ade_network` codec + the serve ChainSync server) — a
deterministic codec/protocol authority, a closed semantic surface. NOT the ledger/consensus BLUE core;
codec/protocol-correctness, not RED glue — closed-grammar discipline (fixture-pinned, single serve
authority, no decoder loosening). No new canonical type; the served chain is already-self-accepted state
(no replay weight).

## §7 Slices
| Slice | Scope | CE | Registry | Status |
|---|---|---|---|---|
| **S1** | Capture + commit the canonical real-node ChainSync `IntersectFound` (the failing peer genesis + the pre-existing preprod set); pin the grammar | CE-G-M-1 (capture half) | CN-WIRE-11 (declared) | done |
| **S2** | Make Ade's serve ChainSync server answer `MsgFindIntersect` with `IntersectFound` (from `ServedChainView`), pinned to the fixture grammar | CE-G-M-1 (answer half) | CN-WIRE-11 → enforced | planned |
| **S3** | Live C1 confirmation: the follower's hot session sustains (no `SingIntersect` timeout) + proceeds toward BlockFetch | CE-G-M-2 | operator-gated | planned |

## §8 Cluster Exit Criteria
- **CE-G-M-1 (mechanical):** Ade's serve ChainSync server, fed the captured real-node `MsgFindIntersect`
  (Origin), replies `IntersectFound` matching the captured grammar (tag 5 + `point=Origin` byte-fixed + a
  valid `[tipPoint, blockNo]` tip = Ade's served tip), served from `ServedChainView`. Pinned to the
  captured fixture, NOT Ade↔Ade.
- **CE-G-M-2 (operator-gated):** a real cardano-node follower's hot session SUSTAINS (no
  `ExceededTimeLimit` at `SingIntersect`) — completes intersect → `RequestNext` → toward BlockFetch of
  block 0. Resumes the rehearsal. `blocked_until_operator_c1_genesis_successor_rehearsal`; no RO-LIVE
  flip; acceptance only via the follower log through `correlate`.

## §9 Replay obligations
None new — ChainSync serving is RED I/O over already-self-accepted state; no canonical type, no
authoritative transition.

## §10 Invariants
- **Preserves:** `DC-NODE-07` (single shared serve), `DC-NODE-09` (serve lifetime), `CN-WIRE-08`
  (tag-24), `CN-WIRE-10` (handshake — unchanged), `RO-LIVE-01/06` (no flip).
- **Adds:** `CN-WIRE-11` (serve-side ChainSync FindIntersect compat), declared → enforced at S2 close.

## §11 Forbidden during this cluster
No forge / PrevHash changes; no handshake changes unless a fixture proves a handshake regression; no
BlockFetch changes yet (until intersect passes); no proactive `advance_tip` unless the real fixture
proves it required; **no second serve authority** (answer from the single `ServedChainView`); no decoder
loosening (fix the server's RESPONSE grammar, fixture-pinned); no RO-LIVE flip; no acceptance claim
without the follower log through `correlate`; the validation fixture MUST be a real cardano-node capture,
never Ade↔Ade.

## §12 Open questions
- **OQ-M1:** does `serve_dispatch` handle `MsgFindIntersect` at all, or only `RequestNext`? → S2.
- **OQ-M2:** must Ade's serve decode the follower's HardForkBlock-encoded intersect points, or are
  Origin / `[slot,hash]` points sufficient for the genesis scenario? → S2 against the fixture.
- **OQ-M3:** after `IntersectFound[Origin]` the canonical sequence is `RollBackward[Origin]` then
  `RollForward[block]` — does the hot session need the full sequence to sustain, or does the
  `IntersectFound` reply alone stop the `SingIntersect` timeout? The immediate blocker is the
  `IntersectFound`; `RollForward` of block 0 (what actually delivers the block) may be S3 or a sibling.

## §13 Non-goals
BlockFetch (until intersect + roll-forward pass); proactive serve / `advance_tip`; cross-epoch; the
handshake (G-L, done); the initiator ChainSync path; durable progression.
