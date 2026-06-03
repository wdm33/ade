# Invariant Sketch — PHASE4-N-F-G-J (RE-SCOPED): Genesis-successor block correctness (PrevHash `null` authority)

> **Status:** Planning artifact (non-normative). Normative authority is the registry + CI.
> **Supersedes** `docs/planning/phase4-n-f-g-j-invariants.md` (the original "planner halts before
> ForgeTick" premise — **falsified by S1**) and the anchor-as-parent framing of the interim
> re-scope (**falsified by OQ1**). **S1 / CN-NODE-04 stays enforced, unchanged.**

## 0. The resolved fact this is built on (OQ1)

Proven against the cardano-ledger source (not assumed):

- `data PrevHash = GenesisHash | BlockHash !HashHeader`
- `instance EncCBOR PrevHash where encCBOR GenesisHash = encodeNull; encCBOR (BlockHash h) = encCBOR h`
- CDDL: `header_body = [ block_number, slot, prev_hash : $hash32 / null, ... ]`

Sources: cardano-ledger `libs/cardano-protocol-tpraos/src/Cardano/Protocol/TPraos/BHeader.hs`;
`eras/babbage/test-suite/cddl-files/babbage.cddl`; issue IntersectMBO/cardano-ledger#1317.

**The genesis-successor (first) block's `prev_hash` is CBOR `null` (`0xf6`)** — the
`PrevHash::GenesisHash` variant. It is structurally *not* a hash. Ade today models `prev_hash` as a
flat `Hash32` (`producer/tick_assembler.rs:43`, `producer/chain_evolution.rs:131`), forges it as
all-zeros at cold-start, and decodes it with `read_hash32` only (`ade_codec/src/shelley/block.rs:144`,
no `null` branch). Ade therefore cannot represent, encode, or decode the genesis-successor parent.
**The defect is wire representation/codec, not forge-base selection.** `seed_epoch_consensus_inputs.anchor_fp`
is an initial-ledger fingerprint, irrelevant to the header parent.

The C1 diagnostic that surfaced this: `--mode node` reaches `ForgeTick` (`forge_tick_considered` ×31)
but skips with `forge_result{no_tip_available}` (`forge_attempted = 0`) because both `ChainDb::tip()`
and `recovered.tip` are `None` at genesis (`node_lifecycle.rs:1075-1080`). The feed was eligible
(`no_block_available`), never `unknown_disconnected`.

## 1. What must always be true

1. **PrevHash is a closed sum** — `PrevHash = Genesis | Block(Hash32)`, mirroring cardano-ledger; the
   header_body `prev_hash` field is the closed wire grammar `$hash32 / null`.
2. **Genesis-successor parent is `null`** — a block at `block_number = 0` on a from-genesis chain
   carries `PrevHash::Genesis`, serialized as CBOR `null`. Never `Hash32([0;32])`, never `anchor_fp`,
   never the Shelley genesis hash.
3. **Non-genesis parent is `hash32`** — a block at `block_number > 0` carries
   `PrevHash::Block(parent_header_hash)`.
4. **Single shared BLUE codec authority, position-blind** — one encode/decode site in `ade_codec`;
   canonical; round-trips both variants. The raw byte codec decodes `null → Genesis` and
   `hash32 → Block` **without** knowing `block_number` (it is position-blind by design).
5. **Position rule lives in the validator** — the *position-aware* check (block 0 ⇒ `Genesis`,
   block > 0 ⇒ `Block`) is enforced by header validation / header-position checks, **not** the byte
   decoder.
6. **Recovered-lineage gates permission, not bytes** — Ade may forge from the genesis-successor
   position only when the explicitly recovered/imported seed-epoch lineage is present, `ForgeIntent::On`,
   the feed is eligible, and the slot/epoch/KES/leader guards pass. The lineage authorizes the
   *position*; it is **not** the source of the prev_hash bytes (which are structurally `null`).
7. **Same accepted path** — the first block flows through `self_accept → SelfAcceptedHandoff
   (DC-NODE-06) → ServedChainView (DC-NODE-07)`, identical to every later block.
8. **Sequencing** — wire/type representation is correct (S2) **before** the forge emits it (S3)
   **before** the node spine reaches it (S4). Reaching the forge before the codec/validation is correct
   only produces blocks a real peer must reject.

## 2. What must never be possible

- ❌ A genesis-successor block whose `prev_hash` is any 32-byte hash (all-zeros / `anchor_fp` /
  genesis hash) — rejected by the header-position check.
- ❌ A non-genesis block whose `prev_hash` is `null` — rejected by the header-position check.
- ❌ Forging the first block without a recovered lineage (raw genesis / cold-start) — fail closed.
- ❌ Shipping the node-spine first-block path (S4) **before** PrevHash type+codec (S2) and position
  validation + forge semantics (S3) — the ordering hard line.
- ❌ A second PrevHash representation or a parallel header encoder.
- ❌ Durable tip / serve / admit of a non-self-accepted first block; re-firing the first-block path
  once a tip exists; any RO-LIVE-01/06 flip or peer-accept claim without `ba02_evidence::correlate`.

## 3. What must remain identical across executions

The genesis-successor header bytes (including the `0xf6` null prev_hash) and the resulting header hash
(block 2's future parent) are a pure function of `(recovered lineage, slot, kes_period, keys, pparams,
era_schedule)`. The `null` encoding is a fixed constant. No wall-clock / rand / float on the
parent-hash surface.

## 4. What must be replay-equivalent

- The PrevHash codec: `decode(encode(x)) == x` for both variants; encode canonical.
- Same recovered surface + same slot → byte-identical first block + identical `(self_accept, handoff,
  served)` effects (DC-FORGE-01 weight class).
- **New canonical surface (replay obligation):** `PrevHash` is a new canonical wire type; the
  header_body `prev_hash` field changes from flat `hash32` to `$hash32 / null`. Canonical-type count
  moves; a **genesis-successor-header round-trip corpus entry** (null prev_hash) is owed.

## 5. State transitions in scope

- **Codec (position-blind):** `bytes[…0xf6…] ↔ BHBody{prev: Genesis}`; `bytes[…hash32…] ↔
  BHBody{prev: Block(h)}` (round-trip; N>0 behavior unchanged).
- **Validator (position-aware) fail-closed:** `(BHBody{prev: Block(_)}, block_number 0) → Err`;
  `(BHBody{prev: Genesis}, block_number > 0) → Err`.
- **Forge:** `(recovered lineage, tip=None, eligible feed, guards pass, due slot) →
  block{number:0, prev:Genesis} → self_accept → handoff → served`.
- **Forge fail-closed:** `(no recovered lineage) → Err` (raw-genesis forbidden).
- **Node-spine:** `(ChainDb::tip None ∧ recovered.tip None ∧ lineage present ∧ eligible feed ∧
  ForgeIntent::On) → forge first block (prev:Genesis)` [was `NoTipAvailable` skip];
  `(tip present) → existing selected_tip path (prev: Block)`.

## 6. TCB color hypothesis

- **BLUE** — the `PrevHash` sum type + its closed-grammar codec (`$hash32 / null`, position-blind); the
  header-position validation (block 0 ⇒ Genesis); the header-hash computation over a null-prev header;
  the forge emitting `Genesis` for block 0.
- **GREEN** — the node-spine first-block *permission* decision (tip None + lineage + eligible →
  permit), a pure selection over recovered state.
- **RED** — relay-loop wiring; the C1 rehearsal harness.

## 7. Open questions (carry to `/cluster-doc`)

- **OQ-A (header-hash domain).** Does the header-hash computation (block 2's future parent) already
  cover the `prev_hash` field, so a null-prev header hashes correctly? Verify before S3.
- **OQ-B (null scope).** Confirm `null` is scoped to the header_body `prev_hash` field **only** —
  chain-sync / block-fetch Points & Tips always use `hash32`, never `null`. The codec change must not
  leak `null` into Point/Tip encoding.
- **OQ-C (validator ripple).** Where exactly do `block_validity` / `self_accept` /
  `consensus/header_summary.rs` enforce the position rule (block 0 ⇒ Genesis, N>0 ⇒ Block)? (S3.)
- **OQ-D (eligibility reuse).** First-block eligibility reuses S1's `FeedReason::is_forge_eligible()`
  (`no_block_available | clean_empty`); `unknown_disconnected` can never reach first-block forge.
  (Keeps S1 load-bearing.)
- **OQ-E (PrevHash home).** Where does the `PrevHash` sum type live (ade_types / ade_codec / ade_core
  consensus)? Single home, imported by codec + forge.

## 8. Slice order (carried to `/cluster-plan` — not part of this sketch's authority)

S1 (closed feed/forge diagnostics, CN-NODE-04) — **already complete + enforced**.
S2 — PrevHash type + codec authority (null/hash32 round-trip, canonical; **position-blind**, no
position semantics except tests where appropriate). **CN-WIRE-09.**
S3 — header validation / forge genesis-successor semantics (block 0 ⇒ `Genesis`, block > 0 ⇒ `Block`;
forge emits `Genesis` for the first block).
S4 — node-spine first-block reachability (tip None + recovered.tip None + lineage present + eligible
feed → forge first block through the existing self_accept/handoff/serve path). **DC-NODE-08.**
S5 — C1 operator-gated rehearsal (real Haskell follower validates/fetches/logs; `correlate` →
`PrivateRehearsalManifest`).

Default to keeping S2 and S3 separate unless the diff is trivially small — the wire-grammar change is
consensus-critical and deserves a clean boundary.

## 9. Registry impact

- **CN-WIRE-09 (new, `declared`, `derived`)** — PrevHash wire/type authority: `$hash32 / null`;
  `Genesis ↔ null`, `Block ↔ hash32`; one shared position-blind BLUE codec; the validator /
  header-position checks fail closed on the wrong variant for the block position.
- **DC-NODE-08 (reframed, stays `declared`)** — node-spine genesis-successor forge reachability from
  the recovered lineage; the lineage gates permission, the first block carries `PrevHash::Genesis`
  (CN-WIRE-09) through self_accept → handoff → serve; fires exactly once.
