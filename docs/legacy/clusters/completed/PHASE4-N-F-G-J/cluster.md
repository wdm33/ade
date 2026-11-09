# PHASE4-N-F-G-J — Genesis-successor block correctness (PrevHash `null` authority)

> **Re-scope of the original G-J.** The original "feed ends → planner halts before `ForgeTick` →
> fix the planner" premise was **falsified by S1's own diagnostic**: on the live C1 sole-producer net
> the feed stays open/empty (eligible `no_block_available`), the planner **already** emits `ForgeTick`
> (×31), and the forge skips with `no_tip_available` (`forge_attempted = 0`) because both
> `ChainDb::tip()` and `recovered.tip` are `None` at genesis. The interim "forge from the recovered
> anchor as parent" framing was then **falsified by OQ1**: the genesis-successor parent is not a hash
> at all. **S1 (`CN-NODE-04`) stays valid and enforced** — it is what produced this diagnostic. S2–S5
> are re-scoped around the real defect: a Cardano **wire-grammar** incompatibility.
>
> Grounding: `docs/planning/phase4-n-f-g-j-genesis-successor-prevhash-invariants.md` +
> `docs/planning/phase4-n-f-g-j-rescope-cluster-slice-plan.md` (committed `b85a6170`).

## §0 Slices with sharply different IDD status

- **Done / enforced (S1):** the closed feed/forge diagnostics (`CN-NODE-04`) — `60303079`. It is the
  instrument that produced this re-scope.
- **Mechanical, hermetically closeable (S2 + S3 + S4):** the PrevHash wire type/codec (`CN-WIRE-09`),
  header-position validation + genesis-successor forge, and node-spine first-block reachability
  (`DC-NODE-08`) all close on hermetic tests + CI gates.
- **Operator-gated (S5):** the live C1 genesis-successor rerun stays
  `blocked_until_operator_c1_genesis_successor_rehearsal` — the mechanical scaffold closes; the live
  execution + any acceptance stay gated (G-H/G-D precedent).

## §1 Primary invariant

A genesis-successor block (`block_number 0` on a from-genesis chain) carries
`prev_hash = PrevHash::Genesis`, serialized as **CBOR `null` (`0xf6`)** — never a hash. Ade forges it
only from the explicitly recovered seed-epoch lineage, under existing slot/epoch/KES/leader checks,
only via `self_accept → SelfAcceptedHandoff → ServedChainView`. **Registry:** `CN-WIRE-09` (the wire
grammar) + `DC-NODE-08` (node-spine reachability), both `declared` → enforced across this cluster.

## §2 The resolved compatibility fact (OQ1 — proven, not assumed; read first)

```
PrevHash for the genesis-successor block = PrevHash::GenesisHash → CBOR null (0xf6)
  ≠ all-zero Hash32     ≠ anchor_fp     ≠ Shelley genesis hash
```

- cardano-ledger `BHeader.hs`: `encCBOR GenesisHash = encodeNull`; `decCBOR … TypeNull → GenesisHash`.
- `babbage.cddl`: `header_body = [ block_number, slot, prev_hash : $hash32 / null, … ]`.
- IntersectMBO/cardano-ledger#1317 (the GenesisHash rationale).

An all-zero `Hash32` is a `BlockHash` value (a nonexistent parent) — a real peer rejects it. **This is
a wire representation/codec defect, not forge-base selection.**

## §3 The load-bearing FC/IS split (position-blind codec vs position-aware validator)

- **BLUE codec (S2) is POSITION-BLIND** — it decodes `null → Genesis` and `hash32 → Block` *without*
  knowing `block_number`, and encodes the inverse, canonically. It has no position semantics.
- **BLUE validator (S3) is POSITION-AWARE** — `block_number 0` requires `Genesis`; `block_number > 0`
  requires `Block`. This rule lives in header/position validation, **never** the byte decoder alone.

## §4 The recovered-lineage doctrine (preserves the Mithril/recovered-state line)

The recovered seed-epoch lineage gates **permission** to forge from the genesis-successor position (it
proves the base is recovered, not raw genesis). It is **not** the source of the prev_hash bytes —
those are structurally `null`, from the Cardano header grammar. Recovered state remains bootstrap
authority, never the wire-format source.

## §5 Verified component inventory (colors + facts confirmed this session)

| Locus | Fact | Color | Touched by |
|---|---|---|---|
| `ade_codec/src/shelley/block.rs:144` | `prev_hash = read_hash32(...)` unconditional; no `null` branch | BLUE | **S2** |
| `ade_runtime/src/producer/tick_assembler.rs:43` | `pub prev_hash: Hash32` (flat) | BLUE | **S2** |
| `ade_runtime/src/producer/chain_evolution.rs:131-136` | `None => Hash32([0u8;32])` (all-zeros, wrong); `next_block_number()=0` at tip None | BLUE | **S3** |
| `ade_core` consensus/header validation (`header_summary.rs`, `block_validity`) | position-rule home (OQ-C) | BLUE | **S3** |
| `ade_node/src/node_lifecycle.rs:1055-1138` | `selected_tip = tip ?? recovered.tip`; `!forged → NoTipAvailable` (recovered.tip also None at genesis) | RED/GREEN | **S4** |
| `ade_node/src/node_sync.rs:482-518` | `forge_one_from_recovered(recovered, selected_tip, …)` — reused, base only | BLUE-driven | S3/S4 |
| `ade_runtime/src/bootstrap.rs:92` | `BootstrapState{ledger, chain_dep, tip:Option<ChainTip>, seed_epoch_consensus_inputs}` — **no anchor field**; `anchor_fp` ≠ header parent | — | (context) |
| `ade_node/src/live_log/sched_event.rs` | `FeedReason::is_forge_eligible` (S1, enforced) | GREEN | reused by **S4** |

## §6 TCB color map

- **BLUE** — `ade_codec` (`PrevHash` sum + header_body `$hash32 / null` codec, position-blind);
  `ade_core`/`ade_ledger` (header-position validation + forge emitting `Genesis` + header-hash over a
  null-prev header).
- **GREEN** — `ade_node` node-spine first-block *permission* decision (a pure selection over recovered
  state) + the reused `CN-NODE-04` `FeedReason` eligibility.
- **RED** — `ade_node` relay-loop wiring + the C1 rehearsal harness.

## §7 Slices (dependency chain: represent → validate/forge → reach → prove)

| Slice | Scope | CE | TCB | Registry | Status |
|---|---|---|---|---|---|
| **S1** | Closed feed/forge diagnostics, emit-only, no behavior change | CE-G-J-1 | GREEN+RED | `CN-NODE-04` | **DONE / ENFORCED** (`60303079`) |
| **S2** | `PrevHash = Genesis \| Block(Hash32)`; header_body codec `$hash32 / null`; canonical, position-blind round-trip; migrate flat `Hash32` → `PrevHash` | CE-G-J-2 | BLUE | `CN-WIRE-09` → enforced | planned |
| **S3** | Header-position validation (`0 ⇒ Genesis`, `>0 ⇒ Block`); forge emits `Genesis` for block 0 (all-zero parent gone) | CE-G-J-3 | BLUE | `DC-FORGE-01` reused | planned — **after S2** |
| **S4** | Node-spine first-block reachability: both tips `None` + recovered lineage + eligible feed + `ForgeIntent::On` → forge first block through the accepted path, once | CE-G-J-4 | GREEN+RED | `DC-NODE-08` → enforced | planned — **after S3** |
| **S5** | C1 operator-gated genesis-successor rehearsal; `correlate → PrivateRehearsalManifest` | CE-G-J-5 | RED | `RO-LIVE-01` cross-ref, no flip | planned — operator-gated |

## §8 Cluster Exit Criteria

- **CE-G-J-1** (mechanical, **met**) — `--mode node` emits the closed `CN-NODE-04` feed/forge
  diagnostic vocabulary, emit-only, no behavior change. Tests `node_sched_events_emit_closed_vocabulary`
  + `node_sched_event_allowlist_rejects_unknown_variants`; gate `ci_check_node_sched_events_emit_only.sh`.
- **CE-G-J-2** (mechanical) — the header_body `prev_hash` codec round-trips `Genesis ↔ null` and
  `Block(h) ↔ hash32` canonically through one position-blind BLUE authority; a genesis-successor
  null-prev header round-trips in the corpus. Named tests + gate resolved in the S2 slice doc;
  `CN-WIRE-09` declared → enforced.
- **CE-G-J-3** (mechanical) — header-position validation rejects `Block` at `block_number 0` and
  `Genesis` at `block_number > 0`; the forge emits `PrevHash::Genesis` for the first block (hermetic);
  the all-zero parent is gone. Named tests resolved in the S3 slice doc.
- **CE-G-J-4** (mechanical) — a hermetic first-block-from-empty-feed forge tick fires, self-accepts →
  handoff → served from the recovered lineage when both tips are `None` + eligible feed +
  `ForgeIntent::On`, exactly once. Named test resolved in the S4 slice doc; `DC-NODE-08` declared →
  enforced.
- **CE-G-J-5** (operator-gated) — a C1 rerun harness + runbook: a real Haskell follower **is expected
  to** validate/fetch the Ade-forged genesis-successor block **if the block is protocol-valid**; the
  only acceptance claim comes from the follower log through `correlate → PrivateRehearsalManifest`.
  No RO-LIVE flip. `blocked_until_operator_c1_genesis_successor_rehearsal` (the mechanical harness
  closes; live execution stays gated).

## §9 Replay obligations

- **S2 introduces a new canonical wire type** — `PrevHash` (header_body `prev_hash`: flat `hash32` →
  `$hash32 / null`). The canonical-type count moves; a **genesis-successor null-prev header round-trip
  corpus entry** is owed in S2.
- **S3/S4** — the forged first block + `self_accept`/handoff/serve effects stay byte-identical
  (`DC-FORGE-01`) for a given recovered surface; no further new canonical types.
- **S1** — `CN-NODE-04` events stay operational-tier, outside the replay weight class.

## §10 Invariants

- **Preserves:** `CN-NODE-04` (S1, enforced, byte-unchanged), `DC-FORGE-01` / `CN-FORGE-01..04`
  (forged-block bytes for a given base), `DC-NODE-06` (self-accept handoff), `DC-NODE-07` (single
  shared serve), `DC-EPOCH-03` (single-epoch forge containment), `CN-CINPUT-03` / `DC-CINPUT-02b`
  (forge base = recovered surface only), `RO-LIVE-01/06` (no flip).
- **Strengthens:** `CN-WIRE-09` declared → enforced at S2 close (PrevHash wire grammar);
  `DC-NODE-08` declared → enforced at S4 close (node-spine genesis-successor reachability).

## §11 Forbidden during this cluster (slice-level hard prohibitions inherit)

Genesis predecessor is `null`/`Genesis` only — no hash stand-in (all-zeros / `anchor_fp` / genesis
hash); no forge from raw/unanchored genesis; no from-genesis consensus-input constructor; no bypass of
`import_live_consensus_inputs`; no stale base; no durable tip advance from forge scheduling alone; no
skip around `self_accept`; no serve of non-self-accepted bytes; **no second PrevHash representation /
parallel header encoder**; the position rule lives in the validator, never the byte codec alone;
**S4 ships only after S2 + S3** (never make Ade emit a prev_hash a real peer must reject); no
RO-LIVE-01/06 flip (peer-accept operator-gated via `ba02_evidence::correlate`); no co-producer
workaround; no private-only / C1-only flag.

## §12 Open questions (carried to the slice docs)

- **OQ-A** — does the header-hash domain cover `prev_hash`, so a null-prev header hashes correctly
  (block 2's future parent)? Resolve before S3.
- **OQ-B** — `null` is scoped to the header_body `prev_hash` field **only**; chain-sync / block-fetch
  Points & Tips stay `hash32`, never `null`. The codec change must not leak `null` into Point/Tip.
- **OQ-C** — where exactly do `block_validity` / `self_accept` / `ade_core` `header_summary.rs` enforce
  the position rule?
- **OQ-D** — S4 eligibility reuses S1 `FeedReason::is_forge_eligible` (`no_block_available |
  clean_empty`); `unknown_disconnected` never reaches first-block forge.
- **OQ-E** — `PrevHash` sum-type home: `ade_types` / `ade_codec` / `ade_core` consensus.

## §13 Non-goals

Tip-path forge behavior for an existing real parent remains out of scope **except** for migrating the
representation from flat `Hash32` to `PrevHash::Block(hash32)` and preserving byte-identical `hash32`
encoding (a representation migration, not a semantic change); Mithril FirstRun changes; any RO-LIVE
flip or bounty BA-02 completion; proactive serve beyond the existing handoff; preprod execution (C1
rehearsal only).
