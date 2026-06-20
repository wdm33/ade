# EPOCH-CONSENSUS-VIEW — S3a (scope): pointer decoding / resolution compatibility

> **Status:** SCOPED (2026-06-20, pre-code). The FIRST sub-slice of Slice 3 (`SLICE-3-scope.md`). Scoped ALONE — the varint/PV behavior is too load-bearing to bundle with materialization (S3b) or aggregation (S3c). Pure BLUE, independently mergeable, replay-verifiable. No live-path change. Proposed invariant `DC-EVIEW-03`. No code until accepted.

## Purpose
Two pure pieces, isolated from the window and the aggregate:
1. **Era-parameterized pointer-varint DECODER** that matches cardano-ledger's behavior EXACTLY for the bound protocol-major version — replacing Slice-2's `stake_ref::decode_varint` (which diverges from both regimes: it rejects `>u64`). The decoder is the cross-implementation tx-validity compatibility surface.
2. **Pre-Conway pointer RESOLUTION** — a pure function `resolve(pointer_map, PointerRef) -> Option<StakeCredential>` that maps a decoded `(slot,txIx,certIx)` to the stake credential registered by the cert at that exact chain location. The pointer MAP itself is POPULATED by the windowed cert accumulation (S3b) — S3a provides the resolution ALGORITHM, fully testable against a synthetic map (no window dependency).

## Binding compatibility rule (PINNED — match cardano-ledger, never substitute a cleaner rule)
CIP-19 is silent on canonicality → cardano-ledger's implementation is the sole authority. The strict check is a bit-WIDTH check, NOT a minimal-form check. Stored `Ptr` = (u32 slot, u16 txIx, u16 certIx). Era-parameterized on the block's bound protocol major:

| Protocol major (era) | Over-WIDTH varint | Bounded leading-zero alias (`[0x80,0x01]`==`[0x01]`) | Trailing bytes after the 3rd coord |
|---|---|---|---|
| **Conway (9+)** | **REJECT** (bounded group counts; u32 slot / u16 txIx / u16 certIx width check) | **ACCEPT** | **REJECT** |
| **Babbage (7–8)** | **NORMALIZE** — decode u64 (drop bits past 64), then clamp the WHOLE 3-tuple to (0,0,0) if ANY coord overflows its width | **ACCEPT** | **REJECT** |
| **≤Alonzo (2–6)** | NORMALIZE (clamp-3-tuple-to-0) | **ACCEPT** | accept + crop |

(Replaying a pointer address already accepted into Ade's own store uses the fully-lenient form — never re-reject on replay.) The parser exists in every era (pointers stay spendable post-Conway); only its strictness is era-gated. Stake-retirement (PV9 → `Null`) is the SEPARATE Slice-2 rule, unchanged here.

## Scope
- **Modules:** a new era-parameterized pointer decoder (home TBD by obligation 1 — likely `ade_codec` next to `decode_address`, or `ade_ledger` next to `stake_ref`); a pure `resolve_pointer` over a `PointerMap` type. Slice-2's `stake_ref` classifier is UNCHANGED (it stays correct pre-Conway, resolves nothing); S3c will route the resolution path through the new decoder, not slice-2's.
- **The `PointerMap`:** the TYPE + the resolution function are S3a; POPULATING it from registration certs is S3b (the window). S3a tests use a synthetic map.
- **Out of scope (explicit):** the windowed materialization (S3b); stake aggregation (S3c); snapshot/stability (S3d); emission (S3e); activation (DC-EVIEW-08). NO live wiring; NO `track_utxo`; NO leader/header use.

## Invariant (proposed DC-EVIEW-03)
Pointer decoding matches cardano-ledger's era-gated behavior byte-for-byte — accept bounded aliasing in all eras; Conway reject over-width + trailing; Babbage/≤Alonzo normalize-clamp-the-3-tuple; ≤Alonzo crop trailing. No "canonicalization preference" overrides the ledger result. Pointer resolution is pre-Conway only, total, deterministic, fail-closed (an unresolvable pointer → `None`, never a fabricated credential). Pure: no HashMap/wall-clock/rand/float.

## MAC (concrete compatibility — binding; all hermetic, no leader slot)
1. **Per-era equality:** for EACH target era/protocol-version fixture, Ade's pointer-decode result must EQUAL the pinned cardano-ledger behavior, asserted explicitly:
   - **alias acceptance:** `[0x80,0x01]` decodes to `1` in Conway, Babbage, and Alonzo fixtures (NOT rejected).
   - **width rejection (Conway):** an over-`u16` txIx / over-`u32` slot, and a continuation past the max group count, FAIL; trailing bytes FAIL.
   - **normalization (Babbage / ≤Alonzo):** an over-width coordinate clamps the WHOLE `(slot,txIx,certIx)` to `(0,0,0)` — not per-field masking, not wrapping.
   - **trailing bytes:** rejected in Conway + Babbage; cropped in ≤Alonzo.
2. **No canonicalization override:** a test asserting Ade does NOT reject a bounded non-minimal encoding (the divergence guard).
3. **Resolution:** `resolve_pointer(map, ptr)` returns the credential registered at `(slot,txIx,certIx)`; an absent/ambiguous coordinate → `None` (fail-closed).
4. **Slice-2 divergence replaced:** the new decoder is used on the resolution path; a regression test pins that the new decoder — NOT slice-2's `>u64`-rejecting `decode_varint` — governs resolution.
5. **CI gate** `ci/ci_check_eview_pointer_compat.sh`: asserts the era-parameterized decoder exists + is keyed on a bound protocol-major/era input (not ambient), the per-era fixtures exist, and the no-canonicalization-override test exists.
6. **Replay-verifiable:** decode + resolve are pure functions → two-run byte-identical (trivially; recorded for DC-WAL-03 lineage).

## Entry obligations — RESOLVED (2026-06-20, grounded)
1. **Decoder home + the era/PV input — RESOLVED.** The era-parameterized pointer-varint DECODER lives in `ade_codec::address` (next to `decode_address`, the decode chokepoint; `ade_codec` deps `ade_types`, so it takes a typed `CardanoEra`) — reachable by BOTH `ade_ledger::tx_validity/` and the S3c resolution path (`ade_ledger` deps `ade_codec`). The RESOLUTION (`PointerRef` + `PointerMap` → `StakeCredential`) lives in `ade_ledger` (it yields a ledger credential + needs the map). The era input is a TYPED `CardanoEra` bound to the block, sourced from `era_schedule.locate(slot).era` — never ambient/config/clock. Slice-2's `stake_ref` classifier + its internal `decode_varint` are UNCHANGED (frozen, pre-Conway-correct); the S3c resolution path uses the NEW `ade_codec` decoder, not slice-2's.
2. **Golden vectors — RESOLVED.** Per-era fixtures are constructed from the pinned matrix (which IS cardano-ledger's behavior, verbatim-grounded) and cross-checked against cardano-ledger's own `AddressSpec.hs` property tests: `propDecompactErrors` (over-width / over-long pointer → strict decode FAILS), `propDecompactAddrWithJunk` (trailing junk rejected v7+ / cropped ≤v6), `prop "RoundTrip-invalid"` (a normalizable bad-pointer round-trips only through v6/Alonzo), and the concrete `addressWithExtraneousBytes` golden hex. At implementation, lift the concrete golden hex vectors from `AddressSpec.hs`; flag any vector not cross-checkable.
3. **The `PointerMap` shape + resolution semantics — RESOLVED.** No existing map (net-new). `PointerMap = BTreeMap<(SlotNo, TxIx=u16, CertIx=u16), StakeCredential>`. The key triple is fully constructible from EXISTING position tracking: slot = the block's slot; txIx = the tx's position in the block (`enumerate`, e.g. `rules.rs:442`); certIx = the cert's position in the tx (`delegation.rs:121` `cert_index: u16`, already threaded through `apply_cert`). The map is POPULATED at `StakeRegistration` application (S3b, in the window); S3a builds the TYPE + `resolve_pointer(map, ptr) -> Option<StakeCredential>` (fail-closed: an absent coordinate → `None`, never a fabricated credential), tested against a synthetic map. cardano-ledger rule: a pointer resolves to the credential registered by the cert at that exact `(slot,txIx,certIx)`; unresolvable → no stake.
4. **cardano-node 11.0.1 re-verification — RESOLVED (web-confirmed).** cardano-node 11.0.1 keeps the ledger in the **Conway era** and adds PV11 as an INTRA-era hard fork (no era transition) — so PV9, PV10, PV11 are ALL Conway, all ≥ 9, all STRICT; Preview forked to PV11 on 2026-05-08, so the live target (preview, 11.0.1) runs Conway-strict for current-tip ingest → the strict decoder is the live-relevant path. The `decodePtr` @9 strict boundary is stable; `ProtVerHigh ConwayEra` is now 11 (matches the earlier flag). Ade's gate `era == CardanoEra::Conway` (≡ PV9–11) → strict is correct. At implementation, pin the exact ProtVer constants from 11.0.1's cardano-ledger dependency.

## Hard prohibitions / non-goals
- NO reject-all-non-canonical; NO canonicalization preference overriding the ledger result.
- NO windowed materialization, NO stake aggregation, NO snapshot/emission, NO activation.
- NO live wiring; NO `track_utxo=true`; NO leader decision or header-validation change.
- Do NOT modify Slice-2's `stake_ref` classifier (it stays correct pre-Conway); S3a adds the resolution-path decoder alongside it.
