# Ade — HEAD Deltas (Changes Since Baseline)

> **Status:** Living architectural document. Regenerated; not hand-edited.
>
> Regenerate with `/head-deltas <baseline>` after every cluster close. Baseline is recorded in `.idd-config.json` `head_deltas_baseline`.

> Baseline: `01e7e08` (no tag, 2026-05-29 00:54:20 +0700)
> HEAD: `273c887` (docs(registry): PHASE4-N-X close — CN-WIRE-08 enforced + strengthenings, 2026-05-29 04:20:00 +0700)
> 9 commits, 36 files changed, +1664 / -313 lines

This window is exactly two pieces:

1. The **PHASE4-N-W tail** (`fea32c0` close + `97faf6d` seed note) — **no code or behavior change**. `fea32c0` is the N-W cluster *close* commit: it refreshed the four grounding docs (CODEMAP / SEAMS / HEAD_DELTAS / TRACEABILITY) for the producer-Praos-VRF state, archived the completed `PHASE4-N-W` cluster doc + slice docs from `docs/clusters/` into `docs/clusters/completed/`, and bumped `.idd-config.json` `head_deltas_baseline` `22eef90 → 01e7e08` (the previous regeneration's recommendation, applied). `97faf6d` seeded the serve-side tag-24 wire-wrap follow-on as a planning note. Together these are the source of the four-grounding-doc rows and the cluster-doc rename rows in `git diff --stat` — content of the renamed N-W docs is unchanged.
2. The **PHASE4-N-X** cluster — *N2N Tag-24 Wire Envelope Authority* (`b932fd6` cluster doc + invariants + plan + `CN-WIRE-08` declaration; `15c1e40` S1 BLUE authority; `84a60a7` S2 block-fetch serve composition; `129eeef` S3 chain-sync header serve composition; `86312f0` S4 RED-unwrap migration + CI gate; `6fb366b` the per-cluster security HIGH fix; `273c887` close). This is the only code change in the window.

> **Baseline bump (this close):** on the PHASE4-N-X close, `.idd-config.json` `head_deltas_baseline` should be bumped from `01e7e08` to **`273c887`** so the next cluster narrates from this point. (That config edit is made separately, outside this regeneration.)

---

## 1. Commit Log

Verbatim from `git log --oneline --no-merges 01e7e08..HEAD`, newest-first. Type is the conventional-commits prefix on the subject; no editorial.

| Hash | Type | Summary |
|------|------|---------|
| `273c887` | docs | PHASE4-N-X close — CN-WIRE-08 enforced + strengthenings |
| `6fb366b` | fix | PHASE4-N-X — fail-closed CBOR length-arg overflow (CN-WIRE-08 N-7) |
| `86312f0` | feat | PHASE4-N-X S4 — migrate RED unwraps to the tag-24 authority + gate |
| `129eeef` | feat | PHASE4-N-X S3 — chain-sync header tag-24 serve composition |
| `84a60a7` | feat | PHASE4-N-X S2 — block-fetch tag-24 serve composition |
| `15c1e40` | feat | PHASE4-N-X S1 — BLUE tag-24 wire-envelope authority |
| `b932fd6` | docs | PHASE4-N-X cluster doc + invariants + plan + CN-WIRE-08 |
| `97faf6d` | docs | seed the serve-side tag-24 wire-wrap follow-on |
| `fea32c0` | docs | Close PHASE4-N-W — producer Praos VRF authority matches the validator |

Type histogram: feat ×4, docs ×4, fix ×1. Unclassified by prefix: 1 — `fea32c0` ("Close PHASE4-N-W …") carries no conventional-commits prefix; its diff is docs-only (grounding-doc refresh + cluster-doc archive + `.idd-config.json` baseline bump), so it is classified `docs` by scope.

---

## 2. New Modules

One new module this window.

### `ade_codec::cbor::tag24` — BLUE

- **Color:** BLUE — pure, deterministic byte wrap/unwrap; no protocol knowledge, no I/O, no allocation beyond the wrap output.
- **Purpose:** The single workspace authority for the N2N **tag-24 CBOR-in-CBOR byte envelope** (`CN-WIRE-08`). Cardano's Ouroboros N2N protocols carry serialised blocks and headers as CBOR-in-CBOR: the inner CBOR item is serialised, then wrapped in a CBOR tag-24 (`#6.24`, "encoded CBOR data item") byte string — the two marker bytes `0xd8 0x18`. This module owns the wrap/unwrap of that envelope and **nothing else** — per-protocol composition (where the era tag sits relative to the wrap) lives in the `ade_network` codecs.
- **Key items:**
  - `wrap_tag24(inner: &[u8]) -> Vec<u8>` — emits the canonical tag-24 marker + definite-length byte-string header + verbatim inner bytes.
  - `unwrap_tag24(bytes: &[u8]) -> Result<&[u8], TagEnvelopeError>` — zero-copy borrow of the inner bytes; fails closed (typed error, no panic, no lenient path) on a missing/wrong `0xd8 0x18` marker, a non-byte-string payload, a truncated inner, or trailing bytes.
  - `TagEnvelopeError` — closed failure sum carrying only non-secret wire primitives.
- **Added in:** PHASE4-N-X / `15c1e40` (S1). The security fix `6fb366b` later added the `unwrap_rejects_huge_declared_length_without_panic` regression test alongside the shared-primitive overflow fix (see §3 / §5).

**Cross-reference (CODEMAP @ `273c887`):** **WARNING — CODEMAP is stale on this module.** The four grounding docs were last regenerated at `fea32c0` (the N-W *close*, the first commit in this span), which predates `tag24.rs` (`15c1e40`). `grep tag24 docs/ade-CODEMAP.md` returns 0 hits. `ade_codec::cbor::tag24` must be added to the CODEMAP BLUE module table on its next regeneration — run `/codemap`.

---

## 3. Modules Modified

Modules that existed at baseline with non-trivial changes. The N-W tail (piece 1) touched no code, so it produces no §3 entry. Every entry below is PHASE4-N-X.

| Module | Scope | Key changes |
|--------|-------|-------------|
| `ade_codec::cbor` (`mod.rs`) | +29 / -3 lines | **N-X (S1 + security fix):** the new `tag24` submodule is declared here. The **security HIGH fix** (`6fb366b`) hardens the three shared length-bounded readers — `read_bytes`, `read_text`, and `skip_item`'s definite-length arm — replacing the overflow-prone `*offset + len > data.len()` bounds check with an overflow-proof `(*offset).checked_add(len).map_or(true, |end| end > data.len())`. Adversarial wire bytes declaring a near-`u64::MAX` length previously panicked (debug arithmetic overflow / release slice-bounds), reachable from untrusted peer input via the BlockFetch admission path and ChainSync RollForward header path — a remote DoS on the exact authority `CN-WIRE-08` hardens. Valid inputs are unaffected; only malformed huge-length inputs change from panic to a typed `UnexpectedEof`/`Truncated`. |
| `ade_network::codec::block_fetch` | +47 / -16 lines | **N-X (S2):** adds the per-protocol BlockFetch composition `compose_blockfetch_block` / decompose, which calls the shared `wrap_tag24` authority. A served `MsgBlock` payload is `tag24(bytes([era, block]))` — **era inside the wrap**. The composition is pinned byte-identically against the committed real-capture oracle `corpus/network/n2n/block_fetch/local_preprod_tip_msg_01_block.cbor` (`82 04 d8 18 …` = `[4, tag24(bytes([era,block]))]`, captured against docker preprod cardano-node 11.0.1, negotiated N2N v15). No bare `[era, block]` may be served. |
| `ade_network::codec::chain_sync` | +118 / -14 lines | **N-X (S3):** adds the per-protocol ChainSync RollForward composition `compose_rollforward_header` / decompose, calling the shared `wrap_tag24` authority. A served RollForward header is `[era_tag, tag24(bytes(header_cbor))]` — **era tag outside the wrap**, single wrapper. The chain-sync wire era index is the cardano-node **consensus** index (Conway = 6 = ade storage discriminant 7 − 1), distinct from the EBB-aware BlockFetch MsgBlock index (Conway = 7); the codec maps the era through the consensus index. Decompose rejects a non-tag-24 inner. |
| `ade_network::block_fetch::server` / `chain_sync::server` | +93 / -19 lines across 2 files | **N-X (S2 + S3):** the serve paths now emit the composed bytes through the per-protocol authorities (`compose_blockfetch_block` / `compose_rollforward_header`) rather than placing bare `[era, block]` / bare header on the wire. |
| `ade_node::admission::runner` | +7 / -27 lines | **N-X (S4):** the hand-rolled tag-24 parsing in the RED admission path is deleted and replaced by a call to the shared `unwrap_tag24` authority — a net deletion (the deleted `unwrap_block_fetch_envelope` is CI-guarded against reappearing). |
| `ade_core_interop::follow` | +9 / -9 lines | **N-X (S4):** the RED chain-following path's hand-rolled tag-24 unwrap is migrated onto the shared `unwrap_tag24` authority; the served ChainSync header shape (`[era_tag, tag24(header_cbor)]`, single wrapper) was verified here against a real preprod RollForward frame. |
| `ade_network::bin::capture_chain_sync` | +43 / -2 lines | **N-X (S3 + security fix):** the capture tool that produced the new Conway RollForward corpus fixtures; the security fix also added a `parse_hash32` `s.is_ascii()` guard (a review LOW) so a 64-byte non-ASCII `--intersect-hash` cannot panic on a non-char-boundary slice. |

### New corpus fixtures

`84a60a7` (S2) reuses the pre-existing BlockFetch oracle. `129eeef` / the capture tool add the **ChainSync RollForward golden** under `corpus/network/n2n/chain_sync/`:

- `preprod_conway_rollforward_frame_00_recv.cbor` (96 B), `…_frame_01_recv.cbor` (922 B, the Conway header frame), `…_intersect_recv.cbor` (96 B), and `…_meta.toml` — captured from docker preprod cardano-node 11.0.1 by FindIntersect at Conway slot 124137045 then RequestNext (negotiated N2N v15). The fixtures pin the served RollForward header wire shape `[2, [era_idx=6, tag24(bytes(header_cbor))], tip]`, with the tag-24 inner byte-identical to `ade_ledger::block_validity::accepted_block_header_bytes` over the same block.

### Strengthenings recorded this window (registry `strengthened_in`)

Not new rules — cross-cutting invariant strengthenings PHASE4-N-X carried forward (see §7):

- **DC-CONS-17** — `strengthened_in += ["PHASE4-N-X"]` (producer-side block-fetch wire-byte transmission now routes through the tag-24 authority).
- **DC-CONS-18** — `strengthened_in += ["PHASE4-N-X"]` (producer-side chain-sync RollForward header projection now routes through the tag-24 authority).
- **CN-FORGE-03** — `strengthened_in += ["PHASE4-N-X"]` (the serve-side tag-24 wire-wrap — left as a NAMED FOLLOW-ON at the N-V close — is now shipped; the forge↔decode envelope authority extends to the served wire shape).

---

## 4. Feature Flags

No feature-flag deltas this window. No `Cargo.toml` (workspace root or any member) was modified between `01e7e08` and `273c887`, so no `[features]` table, `optionalDependencies`, build tag, or `extras_require` changed. No `compile_error!`-coupled flag was introduced or removed.

---

## 5. CI Checks

Every CI check added or materially modified since baseline. CI scripts live as `ci/ci_check_*.sh` (no `.github/workflows` in this repo yet, per `.idd-config.json` `ci_dirs`). Count: **98 → 99** (+1 new, 0 modified, 0 removed).

### PHASE4-N-X checks

| Check | Status | What it checks |
|-------|--------|----------------|
| `ci_check_tag24_wire_authority.sh` | New (`86312f0`) | The single N2N tag-24 wire-envelope authority: (1) `wrap_tag24` and `unwrap_tag24` are each defined **exactly once**, in the BLUE authority `crates/ade_codec/src/cbor/tag24.rs`; (2) no hand-rolled tag-24 parse (`0xd8`/`0x18` byte-literal sniffing, or a `read_tag(..)==24` + `read_bytes` pair) survives in the RED serve/admission/interop consumer paths — they call the authority; (3) the serve paths compose via the per-protocol authorities (`compose_blockfetch_block` / `compose_rollforward_header`), so no bare `[era, block]` / bare header is placed on the wire; (4) the deleted hand-rolled `unwrap_block_fetch_envelope` does not reappear anywhere under `crates/`. Enforces CN-WIRE-08. |

**Cross-reference (CODEMAP @ `273c887` / TRACEABILITY):** **WARNING — CODEMAP and TRACEABILITY are stale on this gate.** `ci_check_tag24_wire_authority.sh` is bound to `CN-WIRE-08` in `docs/ade-invariant-registry.toml` (`ci_script` field), but `grep ci_check_tag24_wire_authority docs/ade-CODEMAP.md docs/ade-TRACEABILITY.md` returns 0 hits — those docs were last regenerated at `fea32c0`, which predates the gate. Confirm the gate appears in the CODEMAP CI table and in TRACEABILITY mapped to CN-WIRE-08 on their next regeneration — run `/codemap` and `/traceability`.

---

## 6. Canonical Type Registry Delta

n/a — `.idd-config.json` `canonical_type_registry` is `null`. Canonical-type rules live inline in the invariant registry under family **T**; no family-T entries were added or removed this window.

---

## 7. Normative / Invariant Rule Delta

Source: `docs/ade-invariant-registry.toml` (the project's canonical append-only invariant registry; `invariant_registry` in `.idd-config.json`). Counts by `^[[rules]]` entries.

- Rules at baseline (`01e7e08`): **291**
- Rules at HEAD (`273c887`): **292**
- Net additions: **1** (`CN-WIRE-08`, declared at `b932fd6` and enforced at `273c887`, both inside this window — so it is a new ID relative to baseline)
- Removals: **0** (append-only discipline upheld).

### New rule

| ID | Tier | Cluster | One-line summary |
|----|------|---------|------------------|
| `CN-WIRE-08` | derived | N-X | N2N tag-24 CBOR-in-CBOR payload envelopes are constructed and stripped through ONE shared BLUE byte authority in `ade_codec` (`wrap_tag24`/`unwrap_tag24`). Protocol-specific composition lives in `ade_network` BLUE codecs: a served BlockFetch `MsgBlock` payload is `tag24(bytes([era,block]))` (era inside the wrap); a served ChainSync RollForward header is `[era_tag, tag24(bytes(header_cbor))]` (era_tag outside the wrap). Both pinned byte-identically against captured cardano-node 11.0.1 wire fixtures. No bare `[era,block]` over BlockFetch, no bare header over ChainSync RollForward, no hand-rolled tag-24 parse in RED. `unwrap_tag24` fails closed on non-`(0xd8 0x18)`, wrong inner length, or trailing bytes; inner bytes copied verbatim. Status `declared → enforced` within this window; enforced by `ci_check_tag24_wire_authority.sh`. |

### Modified rules (strengthenings)

The three strengthenings listed in §3 had `PHASE4-N-X` appended to `strengthened_in`; no statement was weakened:

- **DC-CONS-17** — `strengthened_in: ["PHASE4-N-R-B"] → ["PHASE4-N-R-B", "PHASE4-N-X"]`.
- **DC-CONS-18** — `strengthened_in: ["PHASE4-N-R-A", "PHASE4-N-S-A", "PHASE4-N-T", "PHASE4-N-V"] → [… , "PHASE4-N-X"]`.
- **CN-FORGE-03** — `strengthened_in: [] → ["PHASE4-N-X"]`; its `open_obligation` was rewritten from "serve-side tag-24 wire-wrap is a NAMED FOLLOW-ON" to "SHIPPED in PHASE4-N-X / CN-WIRE-08 (in-process); the remaining obligation is the OPERATOR-PASS live leg" (RO-LIVE-01 / CN-CONS-06 gated).

### Honest residual

`CN-WIRE-08` proves the in-process tag-24 wrap↔unwrap symmetry, the single-authority + no-second-parser discipline, and oracle-shape match against real cardano-node 11.0.1 captures (BlockFetch block, ChainSync Conway header). It does **not** prove a live peer accepts the served bytes. Live cardano-node peer acceptance of the served block over block-fetch (after chain-sync header acceptance) remains **RO-LIVE-01 / CN-CONS-06 operator-pass gated** (`blocked_until_operator_pass_executed`) — that is an operator-action leg, not a codec gap. The serve-side tag-24 follow-on left open at the N-V close is now mechanically backed; the planning-doc's `[serialisationInfo, tag24(...)]` guess was disproven (the wire form is a bare tag-24, no `serialisationInfo` word).
