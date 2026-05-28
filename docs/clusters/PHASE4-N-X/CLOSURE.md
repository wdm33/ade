# PHASE4-N-X — Closure Record (N2N Tag-24 Wire Envelope Authority)

> Cluster closed. Primary rule **CN-WIRE-08** flipped declared → **enforced**.
> Discharges the serve-side block-fetch tag-24 wire-wrap named in
> `CN-FORGE-03.open_obligation` (in-process); the live operator-pass leg stays
> `RO-LIVE-01` / `CN-CONS-06` gated.

## Commits (baseline `97faf6d` → close HEAD)

| Hash | Slice | Summary |
|------|-------|---------|
| `b932fd6` | docs | cluster doc + invariants + plan + CN-WIRE-08 (declared) |
| `15c1e40` | S1 | BLUE `ade_codec::cbor::tag24` — `wrap_tag24`/`unwrap_tag24` + `TagEnvelopeError` |
| `84a60a7` | S2 | block-fetch `compose/decompose_blockfetch_block`; serve emits `tag24([era,block])` |
| `129eeef` | S3 | chain-sync `compose/decompose_rollforward_header`; serve emits `[era_idx, tag24(hdr)]`; committed Conway golden fixture |
| `86312f0` | S4 | RED unwraps migrated to the authority; `ci_check_tag24_wire_authority.sh` |
| `6fb366b` | fix | fail-closed CBOR length-arg overflow (security-review HIGH) |

## Exit criteria

- **CE-X-1** ✓ `wrap_tag24`/`unwrap_tag24` + closed `TagEnvelopeError`; symmetry across CBOR length classes; fail-closed on every malformed input.
- **CE-X-2** ✓ block-fetch `MsgBlock` payload pinned to the real cardano-node 11.0.1 capture (`82 04 d8 18 …`); bare tag-24, no `serialisationInfo` word.
- **CE-X-3** ✓ served block-fetch payload decodes via the same `decode_block_envelope` authority (wrap⇄decode symmetry).
- **CE-X-4** ✓ chain-sync header pinned to a committed Conway golden fixture; `compose_rollforward_header(Conway, inner)` reproduces the real wire bytes byte-identically.
- **CE-X-5** ✓ DC-CONS-18 served fidelity through the wrap; bare header rejected.
- **CE-X-6** ✓ `ci_check_tag24_wire_authority.sh` — single authority, no hand-rolled parse, serve composes.
- **CE-X-7** ✓ admission + interop unwraps call the shared authority; tests green.
- **CE-X-8** ✓ CN-WIRE-08 `enforced` with 24 tests + ci_script; CN-FORGE-03 / DC-CONS-17 / DC-CONS-18 carry the N-X strengthening.
- **CE-X-9** ✓ misleading `[serialisationInfo, tag24]` block-fetch comment/fixture corrected; no opaque-byte test masks a wrong envelope.

## Load-bearing findings (verified against the live docker preprod peer, not assumed)

1. **Block-fetch `MsgBlock` = `[4, tag24(bytes([era,block]))]`** — a bare tag-24 wrap of the storage form; NO `serialisationInfo` word (the codec's own comment/fixture were wrong). Era index is the EBB-aware storage discriminant (Conway = 7) — needs no translation.
2. **Chain-sync `RollForward` header = `[era_idx, tag24(bytes(header_cbor))]`** with era_idx the cardano-node **consensus** index (Conway = **6** = storage − 1) — DIFFERENT from block-fetch. Captured a fresh Conway RollForward (`corpus/network/n2n/chain_sync/preprod_conway_rollforward_*`) to pin it; the pre-existing origin fixtures are Byron (era 0, unwrapped). Had the symmetry been assumed, a real peer would reject the served header.
3. **Security (per-cluster review HIGH, fixed in `6fb366b`):** the shared `read_bytes`/`read_text`/`skip_item` bounds check `*offset + len > data.len()` overflowed `usize` on a crafted huge length argument → panic reachable from untrusted peer input (remote DoS). Replaced with an overflow-proof `checked_add` guard; added length-class adversarial regression tests. This is what makes CN-WIRE-08's N-7 ("fails closed on wrong inner length") actually true.

## Out of scope / open

- **Live peer acceptance** of the served block/header — `RO-LIVE-01` / `CN-CONS-06`, `blocked_until_operator_pass_executed`. This cluster proves byte-shape + shared authority + wrap⇄unwrap symmetry + oracle-fixture agreement only.
- **N-U** (forged-block durability) remains the other open producer follow-on.
