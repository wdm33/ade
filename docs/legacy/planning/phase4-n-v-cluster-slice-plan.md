# Cluster/Slice Plan — Ade · PHASE4-N-V

> Built from `docs/planning/phase4-n-v-invariants.md`. Scope-locked (OQ2):
> storage form `[era,block]` + self-decode round-trip only; serve-side tag-24
> wire-wrap deferred to a follow-on. OQ3: new canonical
> `ade_codec::encode_block_envelope`. Conway era discriminant = 7
> (`CardanoEra::Conway = 7`). Touches BLUE forge/codec authority.
> Cluster-ID format: `named` (cluster `PHASE4-N-V`, slices `S1`–`S3`).

## Cluster Index (Dependency Order)

1. **PHASE4-N-V — producer/validator codec symmetry (forge envelope fix)** —
   primary invariant: *every block `forge_block` emits decodes via the same
   `decode_block` path that validates received blocks (era-tagged `[era, block]`
   round-trip), so a forged block its own validator cannot decode is
   impossible.*

Single cluster, 3 slices. Depends only on already-enforced authorities
(`decode_block_envelope`, `decode_block`, `self_accept`, `forge_block`, the
N-S-A KES bridge) — no forward dependency.

---

## PHASE4-N-V — producer/validator codec symmetry

- **Primary invariant:** `forge_block` emits era-enveloped `[era, block]` bytes
  (era = Conway discriminant `7`) that round-trip through the single
  `decode_block` authority — producing a `DecodedBlock` whose header-body
  fields and 4 preserved body-bucket bytes equal what was forged. A bare-block
  (no-envelope) forge output, or any forge↔decode asymmetry, is CI-gated
  impossible. *(CN-FORGE-03)*

- **TCB partition:**
  - **BLUE (new):** `ade_codec::encode_block_envelope` (symmetric to
    `decode_block_envelope`).
  - **BLUE (modified):** `ade_ledger::producer::forge::forge_block` (wrap output
    via the new encoder).
  - **BLUE (unchanged, consumed):** `decode_block` / `decode_block_envelope` /
    `decode_conway_block`, `self_accept`, `block_validity`, `block_body_hash`,
    `ShelleyBlock::ade_encode` (still emits the bare inner block).
  - **GREEN / RED:** none new. Tests in `ade_codec` + `ade_ledger`; one CI gate.

- **Cluster Exit Criteria:**
  - **CE-1** — `ade_codec::encode_block_envelope(era, block_bytes)` exists and
    is round-trip-inverse to `decode_block_envelope`
    (`decode(encode(era,b)) == (era, b)`); a unit test pins the Conway envelope
    head (`82 07 …`) against a real `ConwayValidityCorpus` block's first bytes
    (resolves OQ1). *(CN-FORGE-03)*
  - **CE-2** — `forge_block` output decodes via `decode_block`: a forge↔decode
    round-trip test (`forge_block(tick) → decode_block` → Ok, with structural
    equality of header fields + the 4 body buckets) passes, and a CI gate
    forbids a bare-block forge path. No remaining consumer relies on the
    bare-block output (OQ5 confirmed). *(CN-FORGE-03; strengthens DC-CONS-18)*
  - **CE-3** — a consistent eligible-leader tick forges a block that
    **self-accepts**: integration test `forge_to_self_accept_succeeds` asserts
    `forge_block → self_accept → Accepted` (the first in-process
    `ForgeSucceeded`). *(strengthens CN-FORGE-01)*
  - **CE-4** — `CN-FORGE-03` flips to `enforced` (populated tests + ci_script);
    `CN-FORGE-01` + `DC-CONS-18` get `strengthened_in += "PHASE4-N-V"`;
    `cargo test --workspace` clean; cluster closes.

- **Slices:**
  - **S1 — `ade_codec::encode_block_envelope` (new BLUE encoder)** — invariant:
    a single canonical envelope encoder
    `encode_block_envelope(era, block_bytes) -> Vec<u8>` =
    `array(2) ‖ uint(era) ‖ block_bytes`, round-trip-inverse to
    `decode_block_envelope`; unit tests prove the round-trip + pin the Conway
    head `82 07 …` against a real corpus block (OQ1). Standalone, no consumer
    yet. — addresses: CE-1 — TCB: BLUE (`ade_codec`).
  - **S2 — `forge_block` emits enveloped bytes + forge↔decode round-trip gate
    (core fix)** — invariant: `forge_block` wraps its `ShelleyBlock::ade_encode`
    output via `encode_block_envelope(Conway, …)`; `ForgedBlock.bytes` is now
    `[7, block]`. Adds the missing forge↔decode round-trip test + CI gate;
    confirms OQ5 (served-chain `push_atomic` decode + `forge_handler_variants`
    now get past decode; any test asserting the bare-block byte shape updated —
    `ForgedBlock.block` value is unchanged). After S2, `self_accept` is
    reachable past decode for forged blocks. — addresses: CE-2 — TCB: BLUE
    (`forge`), test/CI.
  - **S3 — first in-process `ForgeSucceeded` + close** — invariant: a consistent
    eligible-leader fixture (operator pool registered with
    `vrf_keyhash = blake2b256(vrf_vk)`, positive stake, ASC 1/1, matching eta0
    + era schedule) drives `forge_block → self_accept → Accepted`; integration
    test asserts `ForgeSucceeded`. Resolves OQ4: if header validation surfaces a
    deeper binding mismatch (VRF-cert keyhash, nonce), it is root-caused and
    fixed within this slice — **no faked success**; if a mismatch needs BLUE
    work beyond this cluster's premise, the cluster does not close on CE-3 until
    re-scoped with the user. Flip `CN-FORGE-03` enforced; record strengthenings;
    cluster close. — addresses: CE-3, CE-4 — TCB: BLUE (any header/forge binding
    fix), test.

- **Replay obligations:** `DC-CONS-18` strengthened — the forge transcript now
  includes the envelope, and forge↔decode round-trip becomes a
  replay-equivalence property (R2: `decode_block(forge_block(tick))`
  structurally stable; R1: `forge_block(tick).bytes` byte-identical, carried).
  **No new on-disk replay corpus** (S1 reuses `ConwayValidityCorpus` for the
  era-tag pin). **No new canonical types** (`encode_block_envelope` returns
  `Vec<u8>`; `ForgedBlock` shape unchanged).

- **Registry (declared at `/cluster-doc`, enforced at S3 close):**
  - **NEW `CN-FORGE-03`** — producer/validator codec symmetry (forge emits
    `[era,block]` that round-trips through `decode_block`; bare-block forge
    CI-gated impossible). `cross_ref = [CN-FORGE-01, DC-CONS-18, DC-CONS-16,
    CN-PROD-04]`. `ci_script = ci/ci_check_forge_decode_round_trip.sh` (or an
    extension of `ci_check_forge_purity.sh`).
  - **STRENGTHEN** `CN-FORGE-01` (`ForgeSucceeded` reachable end-to-end),
    `DC-CONS-18` (transcript equivalence now includes envelope + round-trip).
    `strengthened_in += "PHASE4-N-V"` at close.

## Follow-on (explicitly NOT N-V)
- **Serve-side wire-wrap** — block-fetch serve path wrapping `[era,block]` into
  `[serialisationInfo, tag24(...)]` for a live cardano-node peer (the
  operator-pass leg of `CN-CONS-06` / `RO-LIVE-01`).
- **Forged-block durability** — N-U (WAL/ChainDB/snapshot/warm-start), carried
  from N-T.
