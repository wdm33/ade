# Cluster/Slice Plan — Ade · PHASE4-N-F-G-J (RE-SCOPED): Genesis-successor block correctness

> **Supersedes** `docs/planning/phase4-n-f-g-j-cluster-slice-plan.md` (the falsified planner-halt
> plan). Single re-scoped cluster. Grounding:
> `docs/planning/phase4-n-f-g-j-genesis-successor-prevhash-invariants.md`. Does **not** touch the
> central plan `docs/active/phase_4_cluster_plan.md`.

The real compatibility defect this plan is centered on:

```
PrevHash for the genesis-successor block
  = PrevHash::GenesisHash  ->  CBOR null (0xf6)
  != all-zero Hash32
  != anchor_fp
  != Shelley genesis hash
```

Proven against cardano-ledger `BHeader.hs` (`encCBOR GenesisHash = encodeNull`) and `babbage.cddl`
(`prev_hash : $hash32 / null`). Ade today models `prev_hash` as a flat `Hash32`, forges all-zeros
(`producer/chain_evolution.rs:131`), and decodes `read_hash32`-only (`ade_codec/src/shelley/block.rs:144`)
— it cannot represent, encode, or decode the genesis predecessor. The defect is **wire
representation/codec**, not forge-base selection.

## Cluster Index (Dependency Order)

1. **PHASE4-N-F-G-J** — Genesis-successor block correctness — primary invariant: *the first block on a
   from-genesis chain carries `prev_hash = PrevHash::Genesis` (CBOR `null`), forged only from the
   recovered seed-epoch lineage through the existing self-accept → handoff → serve path.*

This is a single re-scoped cluster; its **slice** order is itself the dependency chain
(represent → validate/forge → reach → prove). Each slice is a mergeable unit that leaves the system
correct.

## PHASE4-N-F-G-J — Genesis-successor block correctness (PrevHash `null` authority)

- **Primary invariant:** A genesis-successor block (`block_number 0`) carries
  `prev_hash = PrevHash::Genesis`, serialized as CBOR `null` — never a hash (not all-zeros, not
  `anchor_fp`, not the genesis hash). Ade forges it only from the explicitly recovered lineage, under
  existing slot/epoch/KES/leader checks, only via self_accept → SelfAcceptedHandoff → ServedChainView.
  The recovered lineage gates **permission** to forge from the genesis-successor position; it is **not**
  the source of the prev_hash bytes. The prev_hash bytes come from the Cardano header grammar.

- **TCB partition:**
  - **BLUE** — `ade_codec` (the `PrevHash` sum + header_body `$hash32 / null` codec, position-blind);
    `ade_core` / `ade_ledger` (header-position validation, forge emitting `Genesis`, header-hash over a
    null-prev header).
  - **GREEN** — `ade_node` node-spine first-block *permission* decision (a pure selection over recovered
    state).
  - **RED** — `ade_node` relay-loop wiring; the C1 rehearsal harness.

- **Cluster Exit Criteria:**
  - **CE-G-J-1** — `--mode node` emits the closed CN-NODE-04 feed/forge diagnostic vocabulary,
    emit-only, no behavior change. *(S1 — already enforced.)*
  - **CE-G-J-2** — the header_body `prev_hash` codec round-trips `Genesis ↔ null` and
    `Block(h) ↔ hash32` canonically through one position-blind BLUE authority; a genesis-successor
    null-prev header round-trips in the corpus. *(CN-WIRE-09 enforced.)*
  - **CE-G-J-3** — header-position validation rejects `Block` at `block_number 0` and `Genesis` at
    `block_number > 0`; the forge emits `PrevHash::Genesis` for the first block (hermetic), the
    all-zero parent gone.
  - **CE-G-J-4** — a hermetic first-block-from-empty-feed forge tick fires, self-accepts → handoff →
    served from the recovered lineage when both tips are `None` + eligible feed + `ForgeIntent::On`,
    exactly once. *(DC-NODE-08 enforced.)*
  - **CE-G-J-5** — a C1 rerun harness + runbook (operator-gated): a real Haskell follower **is expected
    to** validate/fetch the Ade-forged genesis-successor block **if the block is protocol-valid**; the
    only acceptance claim comes from the follower log through `correlate` → `PrivateRehearsalManifest`.
    No RO-LIVE flip.

- **Slices:**
  - **S1** — Closed feed/forge diagnostics — invariant: emit-only closed CN-NODE-04 vocabulary, no
    behavior change — addresses: CE-G-J-1 — TCB: GREEN+RED — **DONE / ENFORCED** (HEAD `60303079`).
  - **S2** — PrevHash type + codec authority — invariant: header_body `prev_hash` is the closed grammar
    `$hash32 / null`; `PrevHash = Genesis | Block(Hash32)`; canonical, position-blind round-trip —
    addresses: CE-G-J-2 — TCB: **BLUE** (`ade_codec` + the `PrevHash` type) — strengthens **CN-WIRE-09**
    declared → enforced.
  - **S3** — Header-position validation + forge genesis-successor semantics — invariant:
    `block_number 0 ⇒ Genesis`, `> 0 ⇒ Block` (validator); the forge emits `Genesis` for block 0 —
    addresses: CE-G-J-3 — TCB: **BLUE** (validation + forge). *(Depends on S2.)*
  - **S4** — Node-spine first-block reachability — invariant: both-tips-`None` + recovered lineage +
    eligible feed + `ForgeIntent::On` ⇒ forge the first block (`Genesis`) through the existing accepted
    path, exactly once — addresses: CE-G-J-4 — TCB: GREEN gate + RED wiring — strengthens **DC-NODE-08**
    declared → enforced. *(Depends on S3.)*
  - **S5** — C1 operator-gated genesis-successor rehearsal — invariant: a real Haskell follower **is
    expected to** validate/fetch the null-prev first block **if protocol-valid**; the only acceptance
    claim comes from the follower log via `correlate` → `PrivateRehearsalManifest`; no RO-LIVE flip —
    addresses: CE-G-J-5 — TCB: RED (operator harness). *(Depends on S4; live execution
    `blocked_until_operator_c1_genesis_successor_rehearsal`.)*

- **Replay obligations:** **S2 introduces a new canonical wire type** — `PrevHash` (header_body
  `prev_hash`: flat `hash32` → `$hash32 / null`); the canonical-type count moves and a
  **genesis-successor null-prev header round-trip corpus entry** is owed in S2. S3/S4: the forged first
  block + self_accept/handoff/serve effects stay byte-identical (DC-FORGE-01) for a given recovered
  surface — no further new canonical types. S1 (CN-NODE-04) events stay operational-tier, outside the
  replay weight class.

- **FC/IS partition:** BLUE = `ade_codec` (PrevHash sum + header_body `$hash32/null` codec,
  position-blind) + `ade_core`/`ade_ledger` (header-position validation + forge emitting `Genesis` +
  header-hash over a null-prev header); GREEN = `ade_node` node-spine first-block permission decision;
  RED = `ade_node` relay-loop wiring + the C1 rehearsal harness.

- **Hard prohibitions (whole cluster):** genesis predecessor is `null`/`Genesis` only — no hash
  stand-in (all-zeros / `anchor_fp` / genesis hash); no forge from raw/unanchored genesis; no
  from-genesis consensus-input constructor; no bypass of `import_live_consensus_inputs`; no stale base;
  no durable tip advance from forge scheduling alone; no skip around self_accept; no serve of
  non-self-accepted bytes; no second PrevHash representation / parallel header encoder; the position
  rule lives in the validator, never the byte codec alone; **S4 ships only after S2 + S3** (never
  produce a block a real peer must reject); no RO-LIVE-01/06 flip (peer-accept operator-gated via
  `correlate`); no co-producer workaround; no private-only / C1-only flag.

- **Open questions → `/cluster-doc`:** OQ-A (the header-hash domain covers `prev_hash` so a null-prev
  header hashes correctly — verify before S3); OQ-B (`null` scoped to header_body `prev_hash` only —
  chain-sync/block-fetch Points & Tips stay `hash32`, never `null`); OQ-C (where `block_validity` /
  `self_accept` / `consensus/header_summary.rs` enforce the position rule); OQ-D (S4 eligibility reuses
  S1 `FeedReason::is_forge_eligible` — `no_block_available | clean_empty`; `unknown_disconnected` never
  reaches first-block forge); OQ-E (`PrevHash` sum-type home: `ade_types` / `ade_codec` / `ade_core`
  consensus).
