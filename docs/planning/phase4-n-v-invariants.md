# PHASE4-N-V — Invariant Sketch
## Producer/validator codec symmetry (forge envelope fix)

> IDD Part I artifact. Produced before `/cluster-plan`. No implementation.
> Baseline HEAD: post-N-T close (`be748ff`). Touches **BLUE** forge/codec authority.

## 0. Root cause (verified, do not re-derive)

`forge_block` → `ShelleyBlock::ade_encode` emits the **bare** Conway block
`[header, tx_bodies, tx_witnesses, aux, invalid_txs]` (`0x85…`).
`decode_block` → `decode_block_envelope` (envelope.rs:29,45–48) requires the
era-tagged `[era, block]` form (CBOR `array(2)`: era uint, then the block).
`array(5) ≠ 2` → `BlockValidityError::Body(Decoding(InvalidStructure))` at
offset 0 (header_input.rs:244), **before** any header/KES/leader/self_accept
logic. Reproduced in isolation during root-cause (forge_block(empty tick) →
decode_block → that exact error).

Latent since N-C: no test round-trips `forge_block` output through
`decode_block`; the `self_accept` tests use real **enveloped** corpus blocks
(envelope.rs test = `82 00 83…`; the `ConwayValidityCorpus` blocks are the
`[era, block]` storage form). Consequence: the producer has **never** emitted
a block its own validator can decode — the binding blocker for the bounty
block-production leg (`CN-CONS-06` / `RO-LIVE-01`).

**Two serialization forms exist (load-bearing):**
- **Storage/validator form** — `[era, block]` (corpus, `decode_block_envelope`,
  `self_accept`, served-chain `push_atomic` decode). Conway era tag = 7
  (conway/mod.rs:17) — pin against corpus (OQ1).
- **Wire/block-fetch form** — `[serialisationInfo, tag24(bytes([era,block]))]`
  (block_fetch.rs:101–104). Receive path unwraps tag-24 (N-M-FRAG); a live
  serve path would need the symmetric wrap.

**Pure-transformation check: PASSES.** `forge_block: ProducerTick →
enveloped_bytes` and `decode_block: bytes → DecodedBlock` are pure; the
round-trip `decode_block(forge_block(tick)) ≡ tick` is a pure property. No
nondeterminism introduced. Concept understood.

## Scope lock (DECIDED)

- **OQ2 → storage form + self-decode only.** N-V makes `forge_block` emit the
  `[era, block]` storage form so it round-trips through Ade's own
  `decode_block` / `self_accept` (unblocking the first in-process
  `ForgeSucceeded`). The **serve-side tag-24 wire-wrap** for a live
  cardano-node peer (`[serialisationInfo, tag24(...)]`) is a **named follow-on
  cluster** (the operator-pass leg), NOT N-V.
- **OQ3 → new `ade_codec::encode_block_envelope`.** A single canonical
  envelope encoder in `ade_codec`, symmetric to the existing
  `decode_block_envelope`; `forge_block` calls it. One authority for both
  directions.

## 1. What must always be true

- **A1 — producer/validator codec symmetry (NEW, CN-FORGE-03).** Every block
  `forge_block` emits decodes via the *same* `decode_block` path that
  validates received blocks, yielding a `DecodedBlock` whose header-body
  fields + 4 body-bucket bytes equal what `forge_block` put in.
- **A2 — era envelope (NEW, CN-FORGE-03).** `forge_block` output is the
  `[era, block]` form (era = Conway discriminant), byte-matching the
  storage/corpus form `decode_block_envelope` expects — not a bare array.
- **A3 — body-hash binding survives the envelope (strengthens DC-CONS-18).**
  The `body_hash` `forge_block` computes equals the hash `decode_block`
  recomputes from the round-tripped body; the envelope wrap leaves the
  preserved body-bucket bytes byte-identical.
- **A4 — single envelope authority (NEW).** One canonical
  `ade_codec::encode_block_envelope`, symmetric to `decode_block_envelope`;
  no second/parallel block serializer.
- Carries: CN-FORGE-01 (self_accept gate — now *reachable*), CN-KES-HEADER-01
  (N-S-A pre-image — now reachable past decode), DC-CONS-16 (preserved-byte
  body buckets — unchanged).

## 2. What must never be possible

- `forge_block` emitting bytes `decode_block` cannot parse, or that decode to a
  *different* block than forged (codec asymmetry).
- A bare-block (no-envelope) forge output surviving to broadcast — caught by a
  permanent **forge↔decode round-trip gate** (the test whose absence hid this).
- A second/parallel block serializer diverging from `decode_block_envelope`'s
  form.
- Re-encoding the body buckets (must stay preserved bytes — DC-CONS-16).
- `ForgeSucceeded` for a block that doesn't round-trip (structurally tied:
  `self_accept` decodes first).

## 3. What must remain identical across executions
- `forge_block(tick).bytes` (deterministic; envelope is fixed bytes).
- `decode_block(forge_block(tick).bytes)` → `DecodedBlock` (deterministic).

## 4. What must be replay-equivalent
- **R1** — `forge_block(tick).bytes` byte-identical across runs for a fixed
  tick. *(carries DC-CONS-18)*
- **R2 (round-trip, NEW)** — `decode_block(forge_block(tick).bytes)` yields a
  `DecodedBlock` structurally equal to the tick's header + body inputs.
- **R3 (self-accept reachability, NEW)** — for a consistent eligible tick,
  `forge_block → self_accept → Accepted`: the **first in-process
  `ForgeSucceeded`**. *(strengthens CN-FORGE-01)*

## 5. State transitions in scope
```
T0  Encode envelope (BLUE, NEW): (era, block_bytes) -> enveloped_bytes  [ade_codec::encode_block_envelope]
T1  Forge (BLUE, MODIFIED): (ProducerTick) -> Result<(ForgedBlock{enveloped bytes}, effects), ForgeError>
T2  Decode (BLUE, exists):  (enveloped_bytes) -> Result<DecodedBlock, BlockValidityError>   [now succeeds on forge output]
T3  Self-accept (BLUE, exists): (enveloped_bytes, ledger, chain_dep, schedule, view) -> Result<AcceptedBlock, SelfAcceptError>  [now reachable]
T4  Round-trip gate (test/CI): forge_block(tick) -> decode_block -> assert structural equality
```

## 6. TCB color hypothesis
- **BLUE (NEW):** `ade_codec::encode_block_envelope` (symmetric to
  `decode_block_envelope`).
- **BLUE (MODIFIED):** `ade_ledger::producer::forge::forge_block` (wrap output
  via the new encoder).
- **BLUE (unchanged):** `decode_block`, `decode_block_envelope`, `self_accept`,
  `block_body_hash`, `ShelleyBlock::ade_encode` (still emits the bare block;
  the envelope wraps around it).
- **No new GREEN/RED.** Round-trip gate = BLUE-crate test + CI grep.

## 7. Open questions / proof obligations (slice-entry, per `feedback_proof_discipline`)
- **OQ1 (era tag byte).** Pin the Conway envelope era-tag against a real corpus
  block's first bytes (`82 <tag> …`); confirm it equals the
  `CardanoEra::try_from` value `decode_block_envelope` uses (conway/mod.rs says
  7).
- **OQ4 (failures behind decode).** After the envelope fix, does a consistent
  eligible tick actually `self_accept` (first in-process `ForgeSucceeded`), or
  does header validation surface further mismatches (VRF-cert keyhash binding,
  `body_size`, nonce)? Scope "drive to `ForgeSucceeded`" as an explicit slice
  with an honest fallback if a deeper mismatch appears.
- **OQ5 (other consumers).** Confirm no current caller relies on the bare-block
  `forge_block`/`ade_encode` output (likely none — nothing round-trips it).

## 8. Proposed registry entries (declared at `/cluster-doc`)
- **NEW `CN-FORGE-03`** — Producer/validator codec symmetry: `forge_block`
  emits era-enveloped `[era, block]` bytes that round-trip through
  `decode_block` (same path that validates received blocks); a forged block
  that does not self-decode is CI-gated impossible.
  `cross_ref = [CN-FORGE-01, DC-CONS-18, DC-CONS-16, CN-PROD-04]`.
- **STRENGTHEN** `CN-FORGE-01` (ForgeSucceeded reachable end-to-end),
  `DC-CONS-18` (forge transcript equivalence now includes envelope +
  round-trip).
- Hold the append until `/cluster-doc` firms the slice→rule mapping (matches
  the N-T discipline).

## 9. Follow-on (explicitly NOT N-V)
- **Serve-side wire-wrap** — the block-fetch serve path wrapping `[era,block]`
  into `[serialisationInfo, tag24(...)]` so a live cardano-node peer accepts
  the served block (the operator-pass leg of `CN-CONS-06` / `RO-LIVE-01`).
- **Forged-block durability** — N-U (WAL/ChainDB/snapshot/warm-start), carried
  from N-T.
