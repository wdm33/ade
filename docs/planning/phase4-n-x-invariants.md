# Invariant Sketch — PHASE4-N-X: N2N Tag-24 Wire Envelope Authority

> IDD Part I planning artifact. Frames the concept in invariant terms before any
> cluster/slice/code work. Pick-up HEAD: `97faf6d`. Scope confirmed by the user:
> ONE cluster, BOTH N2N payload surfaces, ONE BLUE tag-24 byte authority.

**Concept.** Cardano peers expect served blocks and headers as **CBOR-in-CBOR**
(tag-24-wrapped) payloads. Ade currently serves the bare inner bytes on both
surfaces. This cluster establishes a *single* tag-24 wrap/unwrap byte authority in
BLUE `ade_codec`, has `ade_network` compose it per-protocol, and migrates the
existing hand-rolled RED unwraps onto it. In-process wrap⇄unwrap symmetry +
oracle-fixture agreement are provable here; real-peer acceptance stays a separate
operator-gated leg (RO-LIVE-01 / CN-CONS-06).

**Pure-transformation check.** ✅ Expressible as `canonical input → canonical output`:
- wrap: `inner_bytes → d8 18 ‖ cbor_bytes_header(len) ‖ inner_bytes`
- unwrap: `wire_bytes → Result<inner_bytes, TagEnvelopeError>` (total — rejects
  non-`d8 18`, bad length, trailing bytes)

No clock, rand, float, or I/O in the authority. Nondeterminism stays in the RED
shell (socket reads), which already canonicalize before reaching the chokepoint.

---

## 1. What must always be true

- **I-1 (single wrap authority).** Exactly one BLUE function pair in `ade_codec`
  constructs/strips the tag-24 CBOR-in-CBOR byte envelope. Every N2N wrap and every
  N2N unwrap — serve-side and receive-side, block and header — routes through it.
- **I-2 (block-fetch composition).** A served BlockFetch `MsgBlock` payload equals
  `tag24(bytes([era, block]))` — the tag-24 wrap of the canonical `[era,block]`
  storage form produced by `encode_block_envelope` (era *inside* the wrapped bytes).
  Verified shape: `d8 18 <len> 82 <era> …`.
- **I-3 (chain-sync header composition).** A served ChainSync `RollForward` header
  equals `[era_tag, tag24(bytes(header_cbor))]` — era_tag *outside* the tag-24, the
  bare `accepted_block_header_bytes` projection *inside* (no era prefix, no second
  wrap).
- **I-4 (wrap⇄unwrap symmetry).** For both surfaces, `unwrap(wrap(x)) == x`
  byte-identically, and `decode_block(unwrap(served_blockfetch_payload))` is `Ok`
  (the wrap composes with the N-V `decode_block_envelope` authority).
- **I-5 (oracle-pinned shape).** The emitted wire bytes match the **captured
  cardano-node 11.0.1 preprod fixtures**, not codec comments. The block-fetch
  fixture already exists; a chain-sync `RollForward` header fixture must be
  captured/committed (see Q-1, resolved as a slice-entry obligation).
- **I-6 (preserved-byte fidelity, strengthens T-ENC-01).** The wrap copies the inner
  bytes verbatim — no re-encode of the block/header on the way out (hash-bearing
  bytes are never reconstructed).
- **CN-FORGE-03 relationship.** Its `open_obligation` (serve-side block-fetch tag-24
  wrap) is the block-fetch arm of this cluster; it is **discharged by implementation
  (S2), not by this sketch** — left un-marked until S2 proves the served payload is
  tag-24 wrapped and CI-bound.

## 2. What must never be possible

- **N-1.** A bare `[era, block]` served over BlockFetch (currently the case —
  `82 04 82 07 …`).
- **N-2.** A bare `header_cbor` (or `[era_tag, header_cbor]` without tag-24) served
  over ChainSync RollForward.
- **N-3.** A second/parallel tag-24 wrap or unwrap implementation anywhere —
  including the existing hand-rolled `unwrap_block_fetch_envelope` (RED, `ade_node`
  admission) and `project_header_from_n2n_rollforward` (RED, `ade_core_interop`)
  surviving as independent parse authorities.
- **N-4.** Inserting a `serialisationInfo` word into the BlockFetch payload (the
  misleading codec comment/fixture) — the captured oracle proves there is none.
- **N-5.** An opaque-byte round-trip test that passes while the *internal* envelope
  shape is wrong (the current trap: `Block.bytes` is opaque, so `encode(decode(x))==x`
  says nothing about correctness).
- **N-6.** This cluster claiming live peer acceptance. Wire-correct ≠ peer-accepted.
- **N-7.** Silent acceptance on unwrap of non-`d8 18` bytes, wrong inner length, or
  trailing data — must fail-closed with a typed error.

## 3. What must remain identical across executions

- The wrap output for a given inner-byte input — fully deterministic (a pure byte
  transform; tag-24 + canonical CBOR bytes-header for the length).
- The unwrap result/error for given wire bytes.
- Iteration/serve order of `producer_block_fetch_serve` replies (already
  deterministic; unchanged).

## 4. What must be replay-equivalent

- Replaying the captured BlockFetch `MsgBlock` and ChainSync `RollForward` fixtures
  through unwrap → BLUE decode yields byte-identical inner bytes and identical
  `DecodedBlock` / projected header across runs.
- Serve-side: the same `AcceptedBlock` / accepted header projection, wrapped, yields
  byte-identical wire payloads across runs — and those equal the captured oracle
  bytes (shape + framing identical).

## 5. State transitions in scope

Pure byte/value transitions (no chain-state mutation):

```
wrap_tag24(inner: &[u8])                         -> Vec<u8>
    Pure deterministic transform over accepted in-memory byte slices;
    allocation failure is not a semantic branch.                          [BLUE ade_codec]

unwrap_tag24(wire: &[u8])                         -> Result<&[u8], TagEnvelopeError>
    Total over byte slices; fail-closed on non-(0xd8 0x18), wrong inner
    length, or trailing bytes.                                            [BLUE ade_codec]

compose_blockfetch_block(accepted_bytes)         -> Vec<u8> = wrap_tag24([era,block])     [BLUE ade_network]
compose_rollforward_header(era_tag, header_cbor) -> Vec<u8> = [era_tag, wrap_tag24(hdr)]  [BLUE ade_network]

decompose_blockfetch_block(payload)              -> Result<[era,block] bytes, Err>        [BLUE ade_network]
decompose_rollforward_header(payload)            -> Result<(era_tag, header_cbor), Err>   [BLUE ade_network]
```

Server reducers (`producer_block_fetch_serve`, chain-sync server) change only in
*what bytes they emit* — they call the composition functions instead of passing bare
bytes. Their state machines are unchanged.

## 6. TCB color hypothesis

- **BLUE — `ade_codec`:** `wrap_tag24` / `unwrap_tag24` + `TagEnvelopeError` (closed
  sum). The single byte authority. Pure, deny-attributed. Owns **only** the tag-24
  wrap/unwrap byte primitive — no protocol knowledge.
- **BLUE — `ade_network::codec::{block_fetch,chain_sync}`:** the per-protocol
  compose/decompose functions (era-inside vs era_tag-outside), calling the
  `ade_codec` authority. Stale comments corrected here.
- **BLUE — `ade_network::{block_fetch,chain_sync}::server`:** serve reducers emit
  composed bytes.
- **BLUE — `ade_ledger`:** `accepted_block_header_bytes` stays the **bare** accepted
  header projection authority (unchanged). The header *wire wrap* is a network-layer
  composition, not a ledger concern (Q-3 resolved: ledger projects bare; network
  composes).
- **RED — `ade_node` admission / `ade_core_interop`:** delete hand-rolled unwraps;
  call the BLUE `unwrap_tag24` / decompose authority. No independent decode authority
  remains in RED.

No new nondeterminism introduced.

## 7. Open questions — RESOLVED

| # | Question | Decision |
|---|----------|----------|
| **Q-1** | Header oracle sufficiency | A committed golden ChainSync `RollForward` header fixture is **required before the implementation slice that claims header-shape enforcement** (S3). The `follow.rs` verification is useful but not enough for registry enforcement. |
| **Q-2** | One rule or two | **One** new `CN-WIRE-08`. Do **not** split the header arm. The invariant is *single tag-24 wire-wrap authority across N2N payload surfaces*; the two protocol compositions are arms of the same authority rule. |
| **Q-3** | Header wrap home | **`ade_network`**, not `ade_ledger`. Ledger projects bare accepted header bytes; network owns protocol-specific wire composition; `ade_codec` owns only the tag-24 byte primitive. |
| **Q-4** | `Block.bytes` semantics | Keep `Block.bytes` opaque if needed, but **fixture-pin the real shape** and ensure serve constructs the wrapped item **through the BLUE authority**. Do not rely on opaque round-trip tests (N-5). |
| **Q-5** | Slice ordering | **S1** BLUE `ade_codec` tag-24 authority + symmetry tests → **S2** block-fetch composition + serve + fixture pin (discharges CN-FORGE-03 open_obligation) → **S3** chain-sync header composition + serve + committed header fixture → **S4** migrate RED unwraps + CI gates. |

## Live-evidence boundary (carry into every slice)

This cluster proves: **byte shape, shared authority, wrap/unwrap symmetry, and
oracle-fixture agreement.** It does **not** prove live peer acceptance. A real
cardano-node accepting the served block over block-fetch (after accepting the served
header over chain-sync) remains **RO-LIVE-01 / CN-CONS-06** operator-pass gated
(`blocked_until_operator_pass_executed`). Do not conflate "wrap is byte-correct" with
"peer accepted."

---

## Registry change

One new rule, appended `status = "declared"`. CN-FORGE-03 strengthening/discharge is
deferred to implementation/cluster-close (this sketch only cross-references it).

**`CN-WIRE-08`** (`tier = "derived"`) — single shared tag-24 N2N wire-envelope
authority covering both arms:
- BlockFetch MsgBlock: `[4, tag24(bytes([era,block]))]`
- ChainSync RollForward header: `[era_tag, tag24(bytes(header_cbor))]`

Cross-refs: `CN-FORGE-03`, `T-ENC-01`, `DC-CONS-18`, `CN-WIRE-06`, `CN-WIRE-07`,
`RO-LIVE-01`.
