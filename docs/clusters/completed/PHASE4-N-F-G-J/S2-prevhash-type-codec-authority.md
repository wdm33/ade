# Invariant Slice — PHASE4-N-F-G-J S2: PrevHash type + codec authority

> **Status:** Planning artifact (non-normative). Normative authority is the registry + CI.

## §2 Slice Header

- **Slice:** PHASE4-N-F-G-J S2 — PrevHash type + codec authority.
- **Cluster:** PHASE4-N-F-G-J — Genesis-successor block correctness (`c167cd41`).
- **Status:** Merged (`3b24c572`).
- **Cluster Exit Criteria addressed — CE-G-J-2 (verbatim):** the header_body `prev_hash` codec
  round-trips `Genesis ↔ null` and `Block(h) ↔ hash32` canonically through one position-blind BLUE
  authority; a genesis-successor null-prev header round-trips in the corpus. *(CE-G-J-3/4/5 out of
  scope.)*

## §3 Slice Dependencies

S1 (`CN-NODE-04`, enforced) — independent; reused unchanged. No dependency on S3/S4. **S3 and S4
depend on this slice** — they cannot represent `Genesis` until `PrevHash` exists.

## §4 Intent (invariant impact)

Make the Cardano header_body `prev_hash` field a **closed wire grammar `$hash32 / null`** in Ade, by
replacing the flat `Hash32` representation with a closed sum `PrevHash = Genesis | Block(Hash32)` and a
**position-blind** codec (`Genesis ↔ CBOR null 0xf6`, `Block(h) ↔ hash32`). This introduces the
representation the genesis-successor parent *requires* without yet changing any forge or validation
behavior — `CN-WIRE-09` declared → enforced. **No semantic change:** every existing (Block) site stays
byte-identical; the all-zero genesis value stays `Block([0;32])` until S3.

## §5 What is built

1. **`ade_types`** — new closed `PrevHash = Genesis | Block(Hash32)` (alongside `Hash32`);
   `ShelleyHeaderBody.prev_hash: Hash32 → PrevHash` (`src/shelley/block.rs:44`).
2. **`ade_codec/src/shelley/block.rs`** — `decode_header_body:144`: peek CBOR null → `PrevHash::Genesis`,
   else `PrevHash::Block(read_hash32)` — **without consulting `block_number`** (read at `:142`);
   `AdeEncode:236`: `Genesis → write_null`, `Block(h) → write_bytes_canonical(h.0)`.
3. **`ade_ledger`** — the 3 `ShelleyHeaderBody` constructors (`producer/forge.rs:290`,
   `block_validity/unsigned_header_pre_image.rs:98`, `block_body_hash.rs:74`) provide
   `PrevHash::Block(existing_hash)` — **byte-identical**; the genesis all-zeros stays `Block([0;32])`
   (wrong-but-byte-identical; the `→ Genesis` flip is S3).
4. **CI gate** `ci/ci_check_prevhash_single_wire_authority.sh` — one header_body `prev_hash` codec
   authority; no parallel/second prev_hash encoder; the `null` grammar appears only in the
   `ShelleyHeaderBody` codec, never in the Point/Tip codec.

**Out of scope (S3):** the `ade_runtime` producer representation (`TickInputs`/`ProducerTick`/
`chain_evolution.prev_hash()` stay `Hash32`, wrapped to `PrevHash::Block` at the `ade_ledger`
construction boundary); the all-zero `→ Genesis` forge change; the position-aware validator.

## §6 Execution Boundary (TCB color)

- **BLUE** — `ade_types` (`PrevHash` + the `ShelleyHeaderBody.prev_hash` field); `ade_codec` (the
  `$hash32/null` decode/encode, **position-blind**); `ade_ledger` (the 3 `ShelleyHeaderBody`
  constructors, byte-identical `Block` wrap).
- **Unchanged** — `ade_runtime` producer (`prev_hash: Hash32` upstream, S3); `ade_network` Point/Tip
  codec (separate, stays `hash32`/`Origin`, OQ-B); `ade_node` (S4).
- **Position-blind confirmed:** the decoder's `prev_hash` decision is a pure function of the CBOR token
  (null vs bytestring), never `block_number`. The position rule (`block 0 ⇒ Genesis`) is S3.

## §7 Invariants Preserved

- **`CN-WIRE-08`** (tag-24 wire authority) — header bytes still wrapped/served identically; the `Block`
  case encodes byte-identical `hash32`.
- **`DC-FORGE-01` / `CN-FORGE-01..04`** — forged-block bytes unchanged (genesis stays `Block([0;32])`;
  no forge behavior change in S2).
- **`CN-NODE-04`** (S1) — untouched.
- **Point/Tip `hash32` grammar** (OQ-B) — `ade_network` Point/Tip codec unchanged; `null` not leaked
  into Point/Tip.
- The real-block (Conway mainnet/preprod) corpus round-trips **byte-identically** (the `Block` path).

## §8 Invariants Strengthened

**`CN-WIRE-09`** declared → enforced. Binds tests `prevhash_genesis_round_trips_as_null`,
`prevhash_block_round_trips_as_hash32`, `prevhash_codec_is_position_blind`,
`genesis_successor_header_round_trips_with_null_prev`,
`block_header_prev_hash_byte_identical_after_migration`; CI gate
`ci/ci_check_prevhash_single_wire_authority.sh`. (`CN-WIRE-09.introduced_in = PHASE4-N-F-G-J`, same
cluster — no `strengthened_in` self-bump.)

## §9 Open questions resolved in this slice

- **OQ-E** → `PrevHash` lives in **`ade_types`** (with `Hash32` + `ShelleyHeaderBody`).
- **OQ-B** → the `null` grammar is scoped to `ShelleyHeaderBody.prev_hash` only; `Point`/`Tip`
  (`ade_network`) are a separate codec (`Point::Origin` = `array(0)`, hash field = `Hash32`),
  unaffected.
- *(OQ-A header-hash domain + OQ-C validator-position home → S3; OQ-D eligibility → S4.)*

## §11 Replay / Crash / Epoch Validation

- **New canonical wire type** — `PrevHash`; the header_body `prev_hash` field `flat hash32 →
  $hash32 / null`; the canonical/wire type inventory changes. New corpus round-trip
  `genesis_successor_header_round_trips_with_null_prev` (a `ShelleyHeaderBody` with `prev_hash =
  Genesis` encodes to `…0xf6…` and decodes back).
- **No regression** — `block_header_prev_hash_byte_identical_after_migration`: the existing real Conway
  block-header corpus encodes byte-identically (the `Block` path) — replay equivalence preserved.

## §12 Mechanical Acceptance Criteria

- `cargo test -p ade_types -p ade_codec -p ade_ledger` green, including the 5 named tests above + the
  existing header/block corpus round-trips (byte-identical `Block` path).
- `bash ci/ci_check_prevhash_single_wire_authority.sh` green (single authority; no `null` in the
  Point/Tip codec).
- No `cargo fmt -p`. Verify scope: `ade_types` + `ade_codec` + `ade_ledger` (not `ade_runtime` /
  `ade_node`).

## §14 Hard Prohibitions (inherit cluster §11 + slice-specific)

The codec is **position-blind** (no `block_number` semantics — that is S3); **no second `PrevHash`
representation / parallel header encoder**; the `Block`-case encoding is **byte-identical `hash32`**
(no tip-path semantic change); **`null` is never emitted into Point/Tip encoding**; **no forge
behavior change** (all-zero `→ Genesis` is S3); no validation / position rule (S3); no `ade_node`
node-spine change (S4); no RO-LIVE claim.
