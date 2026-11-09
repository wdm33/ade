# Invariant Cluster — PHASE4-N-X (N2N Tag-24 Wire Envelope Authority)

> **Status:** Planning Artifact (Non-Normative). Organizes work; introduces no
> new requirements. Normative documents + CI enforcement win on any conflict.

## Cluster PHASE4-N-X — N2N Tag-24 Wire Envelope Authority

**Primary invariant:** `CN-WIRE-08` (declared → enforced) — N2N tag-24
CBOR-in-CBOR payload envelopes are constructed and stripped through ONE shared
BLUE byte authority in `ade_codec`; `ade_network` codecs own per-protocol
composition (BlockFetch `MsgBlock` = `tag24(bytes([era,block]))`, era inside;
ChainSync `RollForward` header = `[era_tag, tag24(bytes(header_cbor))]`, era_tag
outside); both pinned against captured cardano-node 11.0.1 wire fixtures; no bare
payload served; no hand-rolled tag-24 parse in RED. **Discharges**
`CN-FORGE-03.open_obligation` (the block-fetch arm).

**Normative anchors:**
- `docs/planning/phase4-n-x-invariants.md` (sketch — I1–I6 / N1–N7 / Q1–Q5
  resolved), `docs/planning/phase4-n-x-plan.md` (slice plan S1–S4).
- Registry: `CN-WIRE-08` (primary), `CN-FORGE-03` (block-fetch arm discharge +
  strengthen), `T-ENC-01` (preserved-byte fidelity), `DC-CONS-18` (bare header
  projection authority — unchanged), `RO-LIVE-01` / `CN-CONS-06` (live leg stays
  operator-gated).
- Cardano Ouroboros block-fetch `MsgBlock` + chain-sync `MsgRollForward` HFC
  CBOR-in-CBOR framing — compatibility target pinned by the real-capture corpus
  (`corpus/network/n2n/{block_fetch,chain_sync}`, cardano-node 11.0.1, preprod).

---

### Entry Conditions (guaranteed by prior clusters)

- **N-V / `CN-FORGE-03`:** the single `encode_block_envelope` / `decode_block_envelope`
  pair produces/consumes the storage-form `[era, block]`; forge output round-trips
  through `decode_block`. The tag-24 wire wrap is the layer above this pair.
- **N-A / `CN-WIRE-07`:** closed per-protocol mini-protocol codecs exist; the
  block-fetch + chain-sync real-capture corpora are committed
  (`*_real_capture_corpus` tests round-trip byte-identically — but opaquely, which
  this cluster tightens).
- **N-R-B / `CN-SNAPSHOT-02`:** `producer_block_fetch_serve` serves a
  `RequestRange` from the `ServedChainSnapshot` (`AcceptedBlock.as_bytes()`).
- **N-G / `DC-CONS-18`:** `accepted_block_header_bytes` is the single bare
  header-projection authority for the chain-sync server. **N-X does not modify the
  projection** — it wraps the projection at the network layer.
- **N-M-FRAG:** the receive side already unwraps tag-24 before `decode_block`
  (`unwrap_block_fetch_envelope`); N-X makes that unwrap call the shared authority.

---

### Exit Criteria (CI-Verifiable)

- [ ] **CE-X-1 (S1):** `ade_codec` exposes `wrap_tag24` / `unwrap_tag24` + a
  closed `TagEnvelopeError`; unit tests prove wrap⇄unwrap symmetry across CBOR
  length-class boundaries (0/23/24/255/256/65535/65536 inner bytes) and
  fail-closed on non-`0xd8 0x18`, wrong inner length, and trailing bytes; the
  inner copy is byte-verbatim.
- [ ] **CE-X-2 (S2):** a test asserts the served BlockFetch `MsgBlock` payload =
  `tag24(bytes([era,block]))` and that its framing matches the captured oracle
  `corpus/network/n2n/block_fetch/local_preprod_tip_msg_01_block.cbor`
  (`82 04 d8 18 …`) — a **shape** assertion, not an opaque round-trip.
- [ ] **CE-X-3 (S2):** a test asserts `decode_block(decompose_blockfetch_block(serve_payload))`
  is `Ok` and equals the forged `[era,block]` (block-arm wrap⇄decode symmetry);
  no bare `[era,block]` can be served (the serve path composes via the authority).
- [ ] **CE-X-4 (S3):** a test asserts the served ChainSync `RollForward` header =
  `[era_tag, tag24(bytes(header_cbor))]` where `header_cbor` is the bare
  `accepted_block_header_bytes` projection; pinned against a **committed golden
  served-shape header fixture** (Q-1 obligation).
- [ ] **CE-X-5 (S3):** a test asserts `decompose_rollforward_header(serve_header)`
  returns `(era_tag, header_cbor)` byte-identical to the projection (header-arm
  symmetry); no bare header can be served.
- [ ] **CE-X-6 (S4):** `ci/ci_check_tag24_wire_authority.sh` asserts (a)
  `wrap_tag24` + `unwrap_tag24` defined exactly once, in `ade_codec`; (b) no
  hand-rolled `0xd8`/`0x18` tag-24 parse in RED serve/admission/interop outside
  the authority; (c) serve paths emit no bare `[era,block]` / bare header.
- [ ] **CE-X-7 (S4):** `ade_node::admission::runner::unwrap_block_fetch_envelope`
  and `ade_core_interop::follow::project_header_from_n2n_rollforward` call the
  shared BLUE authority — no independent tag-24 parse survives; the live-admission
  tests stay green.
- [ ] **CE-X-8:** `CN-WIRE-08` in the registry has non-empty `tests` + `ci_script`
  and `status = "enforced"`; `CN-FORGE-03` carries the N-X strengthening and its
  `open_obligation` records the block-fetch wire-wrap as discharged. Verified at
  `/cluster-close`.
- [ ] **CE-X-9:** existing real-capture corpus tests stay green and the misleading
  `[serialisationInfo, tag24]` block-fetch comment/fixture is corrected to the
  real bare-`tag24` shape; no opaque-byte test masks a wrong envelope (N-5).

> No human review may substitute for these checks. Live peer acceptance is **not**
> a CE here — it remains `RO-LIVE-01` / `CN-CONS-06` operator-pass gated.

---

### Expected Slice Types

- **S1** — BLUE primitive introduction (`wrap_tag24` / `unwrap_tag24` +
  `TagEnvelopeError`) + symmetry/fail-closed tests. No wire wiring.
- **S2** — BLUE per-protocol composition (block-fetch) + serve rewire + oracle
  shape-pin; discharges `CN-FORGE-03.open_obligation`.
- **S3** — BLUE per-protocol composition (chain-sync header) + serve rewire +
  committed golden served-shape fixture.
- **S4** — RED migration (admission + interop unwraps → shared authority) + CI
  gate authoring + registry binding/flip.

---

### TCB Color Map (FC/IS Partition)

- **BLUE:** `ade_codec::cbor` (`wrap_tag24` / `unwrap_tag24` / `TagEnvelopeError`
  — the single byte authority); `ade_network::codec::{block_fetch, chain_sync}`
  (per-protocol compose/decompose); `ade_network::{block_fetch, chain_sync}::server`
  (serve reducers emit composed bytes); `ade_ledger::block_validity::header_input`
  (`accepted_block_header_bytes` — **unchanged**, bare projection).
- **GREEN:** none.
- **RED:** `ade_node::admission::runner` (`unwrap_block_fetch_envelope` → calls
  authority); `ade_core_interop::follow` (`project_header_from_n2n_rollforward` →
  calls authority). Both drop their hand-rolled tag-24 parse.

All colors resolved (Q-3: ledger projects bare, network composes). No open color
questions.

---

### Forbidden During This Cluster

- Serving a bare `[era, block]` over BlockFetch or a bare header over ChainSync
  RollForward (N1, N2).
- A second/parallel tag-24 wrap or unwrap implementation — including the surviving
  hand-rolled RED parsers (N3). One BLUE authority both directions.
- Inserting a `serialisationInfo` word into the BlockFetch payload (N4) — the
  captured oracle proves there is none.
- An opaque-byte round-trip test standing in for an envelope-shape assertion (N5).
- Claiming live peer acceptance from this cluster (N6) — `RO-LIVE-01` /
  `CN-CONS-06` operator-gated.
- Silent acceptance on malformed unwrap input (N7) — typed fail-closed only.
- Modifying the BLUE `accepted_block_header_bytes` projection or the
  `encode/decode_block_envelope` storage-form pair (the wrap is a layer above).
- TODO-based correctness or a semantic feature flag in the authoritative path.

---

> Planning aid only. All correctness rules live in the project's normative
> specifications + CI enforcement.
