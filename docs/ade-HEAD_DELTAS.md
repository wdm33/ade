# Ade — HEAD Deltas (Changes Since Baseline)

> **Status:** Living architectural document. Regenerated; not hand-edited.
>
> Regenerate with `/head-deltas <baseline>` after every cluster close. Baseline is recorded in `.idd-config.json` `head_deltas_baseline`.

> Baseline: `550eec3a` (PHASE4-N-F-G-J close — last state the grounding docs reflected, 2026-06-03 22:02)
> HEAD: `65954fa3` (run-2 genesis-rehearsal reproduction + runbook flag fixes + gate now covers c1 manifests, 2026-06-04 23:32)
> Span: **PHASE4-N-F-G-J close-pass → G-K, G-L, G-M, G-N, G-O, G-P, G-Q, G-R → C1 genesis-successor rehearsal evidence** — a multi-cluster catch-up.
> 28 commits (no merges), 73 files changed, +4967 / -243 lines.

> **Baseline note (load-bearing — read before §0).** This window's baseline is **`550eec3a`**, the
> **PHASE4-N-F-G-J slice-span HEAD** — the exact commit the four grounding docs were last
> regenerated against (the previous HEAD_DELTAS lead, preserved below as *Historical*, narrated the
> `13028d49..550eec3a` G-J span). The `.idd-config.json` `head_deltas_baseline` value at this regen
> is **`853344f7`**, which is **stale and mislabeled**: `853344f7` is mid-cluster **G-L S2** (the
> `feat(network): serve-side N2N handshake emits per-version versionData` impl commit), and its
> config note wrongly calls it the "G-K close." Measuring from `853344f7` would silently **skip the
> G-J close-pass commit `487bd829` and all of cluster G-K (`DC-NODE-09`)**. This regen therefore
> deliberately measures the **explicit `550eec3a..65954fa3` span** so the delta is gap-free from the
> last state the docs reflected. (The closer bumps `head_deltas_baseline` to `65954fa3` after this
> regen.)

This window is a **multi-cluster catch-up**. Ade closed **eight clusters** (G-K through G-R) plus a
G-J **close-pass** and a C1 **genesis-successor rehearsal evidence** pass in a single dense day,
each one peeling off the next concrete blocker on the path to a **live C1 private-testnet
genesis-successor follower** that adopts an Ade-forged block 0 over a real `cardano-node` peer. The
clusters form a sequential chain: serve-listener lifetime (G-K) → real-node **handshake** compat
(G-L) → real-node **ChainSync FindIntersect** compat (G-M) → recovered-eta0 **WarmStart** so the
follower's leader check stops failing (G-N) → feed-side **tag-24 unwrap** so block-fetch payloads
decode (G-O) → feed-side **leader-threshold view** from the recovered surface so the follower
validates and ingests block 0 (G-P) → **forge-successor position** from the evolved admitted spine
so the node survives past block 0 (G-Q) → **stable served block 0** via a monotone serve gate so the
follower can adopt it (G-R) → and finally the C1 **reproduction evidence** (two recorded runs).

Each of G-L, G-M, G-N, G-O, G-P, G-Q, G-R makes a **NARROW, live-confirmed** structural claim —
"live-confirmed" means *the specific failure that gated the follower is gone against the real
preprod/C1 `cardano-node` peer* (the next-blocker error string disappears). It does **NOT** mean
bounty completion, preprod acceptance, or any `RO-LIVE` flip. **`RO-LIVE-01` / `RO-LIVE-06` stay
operator-gated** throughout; the C1 genesis-successor rehearsal is **C1 rehearsal infrastructure,
not bounty evidence** — preview/preprod acceptance remains the single bounty deliverable, captured
separately. The honest residual is carried per-cluster in §§ G-K…C1 and summarized in the closing
"Honest residual" section.

## 0. Headline

| Count | Baseline (`550eec3a`) | HEAD (`65954fa3`) | Δ |
|---|---|---|---|
| CI gates (`ci/ci_check_*.sh`) | 126 | **134** | **+8 new**, **+1 modified in place** (`rehearsal_manifest_schema` — C1-manifest glob added), **0 removed** |
| Registry rules (`docs/ade-invariant-registry.toml`) | 319 | **328** | **+9 new** (`DC-NODE-09`, `CN-WIRE-10`, `CN-WIRE-11`, `T-REC-04`, `DC-CINPUT-03`, `CN-WIRE-12`, `DC-CINPUT-04`, `DC-NODE-10`, `DC-NODE-11`); **0 strengthenings recorded in-span**; **0 removed** |
| Test attributes (`#[test]`/`#[tokio::test]`, workspace) | 2305 | **2324** | **+19** (broad grep; CODEMAP's strict line-anchored matcher omits `multi_thread`-flavor tokio tests → it reads 2284→2301/**+17** over the same span — same tests, narrower matcher); concentrated in G-M (+6), G-L/G-O (+3 each) |
| BLUE canonical types | 457 | **458** | **+1** — G-M adds the closed BLUE enum `ArrayHead = Definite(u64) \| Indefinite` (`ade_network::codec::primitives`, CN-WIRE-11; codec BLUE submodule 38→39). G-L (`encode_n2n_version_params` fn) + G-N (`epoch_nonce` field + `SEED_CINPUT_SCHEMA_VERSION` 1→2) extend existing types, add no type |

The **+8 CI gates / +9 rules / +19 tests** are the net of nine close events. The one-to-one
gate↔cluster↔rule map:

| Cluster | New CI gate | Rule(s) introduced (`enforced`) |
|---|---|---|
| G-K | `ci_check_node_serve_lifetime.sh` | `DC-NODE-09` |
| G-L | `ci_check_n2n_handshake_versiondata_authority.sh` | `CN-WIRE-10` |
| G-M | `ci_check_chainsync_findintersect_compat.sh` | `CN-WIRE-11` |
| G-N | `ci_check_warmstart_eta0_overlay.sh` | `T-REC-04`, `DC-CINPUT-03` |
| G-O | `ci_check_feed_tag24_unwrap.sh` | `CN-WIRE-12` |
| G-P | `ci_check_feed_leader_threshold_view.sh` | `DC-CINPUT-04` |
| G-Q | `ci_check_forge_successor_evolved_spine.sh` | `DC-NODE-10` |
| G-R | `ci_check_served_chain_stability.sh` | `DC-NODE-11` |
| C1 | (modifies `ci_check_rehearsal_manifest_schema.sh`) | — (no rule; evidence only) |

> **Cross-reference (other grounding docs).** This is a **catch-up regen**: at the prior baseline
> `550eec3a` the four grounding docs were G-J-current (CODEMAP/SEAMS/TRACEABILITY headers all read
> 319 rules / 126 CI checks / 2284 tests). The CODEMAP/SEAMS/TRACEABILITY refresh for G-K…G-R + C1 is
> committed alongside this HEAD_DELTAS in the catch-up close-pass; their headers should read **328
> rules / 134 CI checks** at HEAD `65954fa3`. If a sibling doc still reads 319/126, it is stale for
> this window — regenerate it, do not patch.

## 1. Commit Log (newest first)

| Hash | Type | Summary |
|------|------|---------|
| `65954fa3` | evidence | run-2 genesis-rehearsal reproduction + runbook flag fixes + gate now covers c1 manifests |
| `129d25ac` | docs | C1 genesis-successor rehearsal reproduction runbook (post-G-R, two-phase, fidelity-fenced) |
| `0d7624d4` | docs | close PHASE4-N-F-G-R + bank C1 genesis-successor rehearsal manifest (follower adopted stable block 0) |
| `32e4498b` | feat | stable served block 0 via a monotone serve gate (PHASE4-N-F-G-R S1, DC-NODE-11) |
| `f17413f5` | docs | PHASE4-N-F-G-R cluster + S1 slice doc — served-chain stability for the genesis-successor rehearsal (DC-NODE-11) |
| `cd0d9d48` | docs | close PHASE4-N-F-G-Q — forge-successor live-confirmed (first stable node, block 1+, no successor crash) |
| `bd85892b` | feat | forge-successor position from the evolved admitted spine state (PHASE4-N-F-G-Q S1, DC-NODE-10) |
| `24c00644` | docs | PHASE4-N-F-G-Q cluster + S1 slice doc — forge-successor tip/block_no fidelity (DC-NODE-10) |
| `36c07cf1` | docs | close PHASE4-N-F-G-P — feed validates Step 5+7 + ingests block 0 live (VerificationFailed gone) |
| `609dc3cc` | feat | feed header-validation view from the recovered consensus surface (PHASE4-N-F-G-P S1, DC-CINPUT-04) |
| `2ec131ed` | docs | PHASE4-N-F-G-P cluster + S1 slice doc — feed-side leader-threshold stake-distribution fidelity (DC-CINPUT-04) |
| `f43727b2` | docs | close PHASE4-N-F-G-O — feed tag-24 unwrap live-confirmed (UnexpectedType gone) |
| `f539aa7a` | feat | feed-side BlockFetch tag-24 unwrap before decode (PHASE4-N-F-G-O S1, CN-WIRE-12) |
| `275a2318` | fix | scope chain-sync round-trip corpus to mux-framed captures (PHASE4-N-F-G-M follow-up) |
| `994e3bc0` | docs | PHASE4-N-F-G-O cluster + S1 slice doc — feed-side BlockFetch tag-24 unwrap (CN-WIRE-12) |
| `87dbe99e` | docs | close PHASE4-N-F-G-N — WarmStart eta0 fix live-confirmed (follower past VRFKeyBadProof) |
| `3235461a` | feat | WarmStart forge eta0 from the seed-epoch sidecar (PHASE4-N-F-G-N S1, T-REC-04 + DC-CINPUT-03) |
| `c32b548b` | docs | PHASE4-N-F-G-N cluster + S1 slice doc — persist eta0 in the seed-epoch sidecar + WarmStart overlay (T-REC-04, DC-CINPUT-03) |
| `b6805965` | feat | real cardano-node ChainSync FindIntersect compat — scoped indefinite decode + Origin reply (PHASE4-N-F-G-M S2, CN-WIRE-11) |
| `992d3537` | docs | PHASE4-N-F-G-M S2 scope correction + real-node FindIntersect request fixture (CN-WIRE-11) |
| `6ce9325a` | docs | PHASE4-N-F-G-M authority doc + S1 real-cardano-node ChainSync IntersectFound fixture (CN-WIRE-11) |
| `a5c39d68` | docs | close PHASE4-N-F-G-L — serve-side N2N handshake cardano-node compatible (CN-WIRE-10) |
| `853344f7` | feat | serve-side N2N handshake emits per-version versionData (PHASE4-N-F-G-L S2, CN-WIRE-10) |
| `e42cb249` | docs | PHASE4-N-F-G-L authority doc + S1 real-cardano-node handshake fixture (CN-WIRE-10) |
| `d78ff038` | docs | close PHASE4-N-F-G-K — node serve lifetime decoupled from feed end (DC-NODE-09) |
| `b8829a6a` | feat | decouple --mode node serve listener lifetime from feed end (PHASE4-N-F-G-K S1) |
| `c029a281` | docs | PHASE4-N-F-G-K cluster + S1 slice — node serve lifetime decoupled from feed end |
| `487bd829` | (close) | Close PHASE4-N-F-G-J — genesis-successor block correctness (PrevHash null wire authority) |

No merge commits in the span. **28 commits, zero unclassified** — every commit carries a
conventional prefix (`feat:` / `fix:` / `docs:` / `evidence:`) or is the G-J `Close …` close-pass.
The shape is regular: the **G-J close-pass** (`487bd829`, docs/registry only), then eight clusters
each as **doc → impl → close** (G-K/G-N/G-O/G-P/G-Q/G-R) or **doc(S1) → doc+fixture(S2) → impl(S2)
→ close** (G-L, G-M, the latter with a `fix(test)` follow-up `275a2318`), then the **C1 evidence**
pair (`129d25ac` runbook + `65954fa3` run-2 + gate coverage).

> **Note (commit-attribution policy).** Per this repo's `CLAUDE.md` override (vibe-coded-node bounty
> trailer requirement), commits in this repo carry a `Co-Authored-By:` model-attribution trailer;
> that is an Ade-local override of the global no-AI-attribution rule and applies to **commit messages
> only**. It does not affect this doc's content.

## 2. New Modules

**None.** No new source module (`crates/*/src/**.rs`) and no new crate were added in this window —
confirmed by `git diff --diff-filter=A --name-only 550eec3a..65954fa3 -- 'crates/*/src/'` (empty).
Every code change **extends an existing module**. The new `.rs` files in the span are all **test
fixtures / capture-replay tests** under `crates/*/tests/` adjuncts and real-node CBOR fixtures
(`crates/ade_network/tests/fixtures/{handshake,chain_sync}/…`), not new modules. The new files under
`docs/evidence/` are recorded C1 rehearsal artifacts, not code.

The work that lands in this window is therefore **modification of existing modules** (§3),
**new CI gates** (§5), and **new evidence** (C1, §"C1 evidence").

## 3. Modules Modified

Grouped by cluster. Each row names the cluster's BLUE/GREEN/RED touch and the rule it backs. The
per-cluster file/line stats are `git diff --shortstat` between the cluster's pre-first-commit parent
and its close commit.

### G-J close-pass (`487bd829`) — docs/registry only

`Close PHASE4-N-F-G-J — genesis-successor block correctness (PrevHash null wire authority)`. The
G-J **slice work was already narrated in the prior HEAD_DELTAS** (preserved as *Historical* below);
this is the close-pass commit at the head of the span. **11 files, +860 / -134** — the four
grounding-doc G-J regen (CODEMAP/SEAMS/TRACEABILITY/HEAD_DELTAS), the `.idd-config.json` baseline
bump, the five G-J slice-doc touches, and the G-J cluster-doc archive move. **No source change, no
new rule, no new gate** (the three G-J rules and gates were committed in-span at the slice HEADs).
Carried here only so the span is gap-free.

### G-K (`c029a281` → `b8829a6a` → `d78ff038`) — serve-listener lifetime, RED-only

`DC-NODE-09` `enforced` · 7 files, **+492 / -17** · +2 tests · +1 gate

| Module | Color | Key changes |
|--------|-------|-------------|
| `ade_node::node_lifecycle` | **RED** (relay-loop home, `crates/ade_node/`) | **G-K S1 (`b8829a6a`).** Decouples the `--mode node` On-arm **serve-listener lifetime from feed end**: when the live feed (WirePump) reaches its end/disconnect, the node **keeps the serve side up** so a downstream follower can still dial `:3002` and block-fetch the served chain, rather than tearing the whole node down with the feed. RED-only lifecycle change; **no BLUE change, no new canonical type**. New test file `node_spine_serve_loopback.rs` (+114) proves the serve listener outlives feed end. |

**New gate** `ci_check_node_serve_lifetime.sh` backs `DC-NODE-09`: pins that the serve-listener task
is not joined to / cancelled by the feed task's completion. **Live-confirmed** at close: against the
C1 follower, Ade stays up serving past feed-end and the follower reaches `:3002`. The next blocker
surfaced at close was a serve-side N2N **handshake** incompatibility (`HandshakeDecodeError
NodeToNodeV_15 'unknown encoding: TInt 1'`) — scoped as **G-L**.

### G-L (`e42cb249` → `853344f7` → `a5c39d68`) — serve-side N2N handshake, BLUE touch

`CN-WIRE-10` `enforced` · 12 files, **+407 / -18** · +3 tests · +1 gate

| Module | Color | Key changes |
|--------|-------|-------------|
| `ade_network::handshake::version_table` | **BLUE** (`core_paths` `crates/ade_network/src/handshake/`) | **G-L S2 (`853344f7`).** The serve-side N2N handshake now emits **per-version `versionData`** matching `cardano-node`'s `NodeToNodeV_15` encoding (the prior flat encoding produced the follower-side `'unknown encoding: TInt 1'` decode error). `version_table.rs` (+26) carries the per-version version-data shape. |
| `ade_network::session::handshake_driver` | **RED** (session is RED) | **G-L S2.** Drives the per-version version-data through the responder handshake (+14). |
| `ade_node::admission::bootstrap` | **GREEN/RED** (`crates/ade_node/`) | **G-L S2.** Call-site adjustment for the per-version version-data (+24 / churn). |

S1 (`e42cb249`) banked a **real-cardano-node handshake fixture**
(`fixtures/handshake/c1privnet_v11_v16_propose_{sent,recv}.cbor` + `_meta.toml`) and a responder
fixture test (`handshake_responder_cardano_node_fixture.rs`, +85). **New gate**
`ci_check_n2n_handshake_versiondata_authority.sh` backs `CN-WIRE-10` (per-version version-data is
the single serve-side authority, NodeToNodeV_15 cardano-node-compatible). **Live-confirmed**: the
handshake now completes against the real peer. NARROW — handshake compat only; not a full N2N
session, not acceptance.

### G-M (`6ce9325a` → `992d3537` → `b6805965`; follow-up `275a2318`) — ChainSync FindIntersect, BLUE touch

`CN-WIRE-11` `enforced` · 13 files, **+738 / -23** (+ follow-up +8) · +6 tests (+1 follow-up) · +1 gate

| Module | Color | Key changes |
|--------|-------|-------------|
| `ade_network::chain_sync::server` | **BLUE** (`core_paths` `crates/ade_network/src/chain_sync/`) | **G-M S2 (`b6805965`).** Real `cardano-node` **ChainSync `FindIntersect`** compat: the served chain-sync server now decodes the follower's `FindIntersect` request and, when no intersection is found, replies **`IntersectFound`/`Origin`** as the real node expects for a from-genesis follower (+74). |
| `ade_network::codec::chain_sync` | **BLUE** (`core_paths` `crates/ade_network/src/codec/`) | **G-M S2.** `FindIntersect` request decode wired into the chain-sync message codec (+47). |
| `ade_network::codec::primitives` | **BLUE** (`core_paths`) | **G-M S2.** **Scoped indefinite-length CBOR decode** primitive (+87) — handles the real node's indefinite-length array encoding in the `FindIntersect` points list (a scoped/bounded decode, not an open one). |

S1 (`6ce9325a`) + the S2 scope-correction (`992d3537`) banked two real-node ChainSync fixtures
(`fixtures/chain_sync/c1privnet_{follower_findintersect,origin_intersect}_*`) and a fixture test
(`chainsync_findintersect_cardano_node_fixture.rs`, +119). The follow-up **`275a2318`** (`fix(test)`)
**scopes the chain-sync round-trip corpus to mux-framed captures** (+8 in
`chain_sync_real_capture_corpus.rs`) so the corpus test replays only genuinely mux-framed captures.
**New gate** `ci_check_chainsync_findintersect_compat.sh` backs `CN-WIRE-11`. **Live-confirmed**: the
real follower's `FindIntersect` is decoded and answered. NARROW — FindIntersect/Origin compat only.

### G-N (`c32b548b` → `3235461a` → `87dbe99e`) — WarmStart recovered-eta0 overlay

`T-REC-04` + `DC-CINPUT-03` `enforced` · 13 files, **+438 / -10** · +1 test (inline) · +1 gate

| Module | Color | Key changes |
|--------|-------|-------------|
| `ade_ledger::seed_consensus_inputs` | **BLUE** (`core_paths` `crates/ade_ledger/`) | **G-N S1 (`3235461a`).** Persists **eta0** (the seed-epoch nonce) in the seed-epoch consensus-inputs sidecar so it survives a `WarmStart` recovery (+33). `consensus_view.rs` carries the field through (+1). Backs `T-REC-04` (the recovered surface is byte-faithful incl. eta0). |
| `ade_runtime::bootstrap` | **RED** (shell, `crates/ade_runtime/`) | **G-N S1.** The `WarmStart` recovery path now **overlays the recovered eta0** into the forge consensus surface (+64), so the follower's leader/VRF check uses the correct nonce. `genesis_bootstrap.rs` / `mithril_bootstrap.rs` / `seed_consensus_merge.rs` thread the field (+1/+1/+3). Backs `DC-CINPUT-03`. |
| `ade_node::{node_lifecycle, node_sync}`, `ade_testkit::consensus::genesis_pinning` | GREEN/RED / test | **G-N S1.** Call-site threading of recovered eta0 (+1/+4/+1). |

**New gate** `ci_check_warmstart_eta0_overlay.sh` backs both `T-REC-04` and `DC-CINPUT-03`. **The bug
this fixed:** before G-N the follower failed its leader check with **`VRFKeyBadProof`** because the
WarmStart-recovered surface dropped eta0 (`bootstrap.rs` predicted it; self-accept masked it because
Ade's own validator read the same wrong nonce on both sides). **Live-confirmed**: the follower now
gets **past `VRFKeyBadProof`**. (The boundary-grep test count is flat 2317→2317; the +1 net test-attr
is inlined in `bootstrap.rs`.) NARROW — the eta0 fix only; the next decode blocker is G-O.

### G-O (`994e3bc0` → `f539aa7a` → `f43727b2`) — feed-side BlockFetch tag-24 unwrap, RED

`CN-WIRE-12` `enforced` · 9 files, **+467 / -20** · +3 tests · +1 gate

| Module | Color | Key changes |
|--------|-------|-------------|
| `ade_runtime::admission::wire_pump` | **RED** (shell, `crates/ade_runtime/`) | **G-O S1 (`f539aa7a`).** The feed-side **BlockFetch** path now **unwraps the tag-24 CBOR envelope before decoding** the block body (+111) — the real node wraps block-fetch payloads in a `#6.24(bytes)` envelope, and decoding the wrapped bytes directly produced `UnexpectedType`. This is the **receive/feed** counterpart to the existing serve-side tag-24 authority (`CN-WIRE-08`). |
| `ade_node::node_sync` | GREEN/RED | **G-O S1.** Feed-pump call-site adjust (+3). |

Tests land in `forge_succeeds.rs` (+68) + `node_spine_serve_loopback.rs` (churn); the follow-up to
`chain_sync_real_capture_corpus.rs` (+8) is also in this commit range. **New gate**
`ci_check_feed_tag24_unwrap.sh` backs `CN-WIRE-12`. **Live-confirmed**: feed-side block-fetch
payloads now decode — **`UnexpectedType` gone**. NARROW — feed tag-24 unwrap only.

### G-P (`2ec131ed` → `609dc3cc` → `36c07cf1`) — feed-side leader-threshold view, GREEN

`DC-CINPUT-04` `enforced` · 6 files, **+441 / -8** · +1 test · +1 gate

| Module | Color | Key changes |
|--------|-------|-------------|
| `ade_node::node_lifecycle` | **GREEN/RED** (`crates/ade_node/`) | **G-P S1 (`609dc3cc`).** Builds the feed's **header-validation view (leader-threshold stake distribution) from the recovered consensus surface** (+50), so the follower validates the incoming block-0 header against the correct stake/threshold view — closing the gap that produced `VerificationFailed` at validation Step 5 + Step 7. The validated block 0 is then **ingested live**. |

Tests land in `forge_succeeds.rs` (+89). **New gate** `ci_check_feed_leader_threshold_view.sh` backs
`DC-CINPUT-04`. **Live-confirmed**: the follower **validates Step 5 + 7 and ingests block 0 live —
`VerificationFailed` gone**. NARROW — the feed-side validation view only.

### G-Q (`24c00644` → `bd85892b` → `cd0d9d48`) — forge-successor from evolved spine

`DC-NODE-10` `enforced` · 6 files, **+423 / -14** · +1 test · +1 gate

| Module | Color | Key changes |
|--------|-------|-------------|
| `ade_node::node_sync` | **GREEN/RED** (`crates/ade_node/`) | **G-Q S1 (`bd85892b`).** The forge-successor's **position (tip / block_no) is now read from the evolved admitted spine state**, not a stale snapshot (+145), so after block 0 is accepted-and-served the node forges **block 1+** at the correct successor position instead of crashing on a stale/duplicate position. `node_lifecycle.rs` threads the evolved tip (+2). |

**New gate** `ci_check_forge_successor_evolved_spine.sh` backs `DC-NODE-10`. **Live-confirmed: first
stable node — block 1+, no successor crash.** NARROW — successor-position fidelity only; durable
multi-block progression past the demonstrated successor is still downstream.

### G-R (`f17413f5` → `32e4498b` → `0d7624d4`) — stable served block 0, monotone serve gate

`DC-NODE-11` `enforced` · 9 files, **+388 / -3** · +2 tests · +1 gate

| Module | Color | Key changes |
|--------|-------|-------------|
| `ade_node::node_lifecycle` | **GREEN/RED** (`crates/ade_node/`) | **G-R S1 (`32e4498b`).** Serves a **stable block 0 via a monotone serve gate** (+59): the served chain head only ever advances monotonically, so a from-genesis follower that intersects at Origin and block-fetches block 0 sees a **stable** served block 0 (no flap/regression) and can **adopt** it. |

Tests land in `forge_succeeds.rs` (+59). **New gate** `ci_check_served_chain_stability.sh` backs
`DC-NODE-11`. The close (`0d7624d4`) also **banks the first C1 genesis-successor rehearsal manifest**
(`docs/evidence/c1-genesis-rehearsal-{follower.log, manifest.toml, peer-accept.jsonl}`). **NARROW
CLAIM (load-bearing):** the **follower adopted the stable block 0** in the recorded C1
genesis-successor rehearsal — this is the genesis-successor leg working end-to-end in the C1
rehearsal venue. It is **NOT** a preview/preprod bounty accept, flips **no** `RO-LIVE` rule, and the
banked manifest is **C1 rehearsal evidence under `CN-REHEARSAL-FIDELITY-01` non-promotability**, not
bounty evidence.

## 4. Feature Flags

**No project feature-flag deltas.** Ade declares no `[features]` table in any workspace `Cargo.toml`,
and **no `Cargo.toml` changed at all in this window** (`git diff --name-only 550eec3a..65954fa3 --
'**/Cargo.toml'` is empty). No `#[cfg(feature = …)]` gate was introduced; no coupling, no
`compile_error!` guard. The C1 genesis rehearsal operator harness remains gated by an **environment
variable** (`ADE_LIVE_C1_GENESIS_REHEARSAL`), not a Cargo feature — a `#[test]` skipped in CI, not a
compile-time flag and not a runtime node mode.

## 5. CI Checks (126 → 134; +8 new, +1 modified in place, 0 removed)

Eight new gates plus one in-place extension, repo-root-relative, mirroring the existing
`ci/ci_check_*.sh` convention. `git diff --diff-filter=A 550eec3a..65954fa3 -- ci/` lists exactly the
eight new gates; `--diff-filter=M` lists exactly `ci_check_rehearsal_manifest_schema.sh`;
`--diff-filter=D` over `ci/` is **empty** (no gate removed).

### New gates (one per cluster, G-K…G-R)

| Check | Status | Cluster origin | What it checks |
|-------|--------|----------------|----------------|
| `ci_check_node_serve_lifetime.sh` | **New** | G-K S1 (`b8829a6a`) | Backs **`DC-NODE-09`**. Pins that the `--mode node` On-arm **serve-listener lifetime is decoupled from feed end** — the serve task is not joined to / cancelled by the live-feed (WirePump) task completing, so a follower can still dial and block-fetch after feed-end. |
| `ci_check_n2n_handshake_versiondata_authority.sh` | **New** | G-L S2 (`853344f7`) | Backs **`CN-WIRE-10`**. Pins the serve-side N2N handshake **per-version `versionData`** as the single authority, `NodeToNodeV_15` cardano-node-compatible (closes the follower-side `'unknown encoding: TInt 1'`). |
| `ci_check_chainsync_findintersect_compat.sh` | **New** | G-M S2 (`b6805965`) | Backs **`CN-WIRE-11`**. Pins the served ChainSync **`FindIntersect` decode** (scoped indefinite-length CBOR) + the **`IntersectFound`/`Origin`** reply the real node expects for a from-genesis follower. |
| `ci_check_warmstart_eta0_overlay.sh` | **New** | G-N S1 (`3235461a`) | Backs **`T-REC-04` + `DC-CINPUT-03`**. Pins that **eta0 is persisted in the seed-epoch sidecar** and **overlaid into the forge consensus surface on WarmStart recovery** (closes `VRFKeyBadProof` on the follower). |
| `ci_check_feed_tag24_unwrap.sh` | **New** | G-O S1 (`f539aa7a`) | Backs **`CN-WIRE-12`**. Pins that the **feed-side BlockFetch** path **unwraps the tag-24 (`#6.24`) envelope before decoding** the block body (closes `UnexpectedType` on feed decode) — the receive-side counterpart to the serve-side `CN-WIRE-08` tag-24 authority. |
| `ci_check_feed_leader_threshold_view.sh` | **New** | G-P S1 (`609dc3cc`) | Backs **`DC-CINPUT-04`**. Pins that the **feed-side header-validation (leader-threshold stake-distribution) view is built from the recovered consensus surface** (closes `VerificationFailed` at validation Step 5 + Step 7, enabling live block-0 ingest). |
| `ci_check_forge_successor_evolved_spine.sh` | **New** | G-Q S1 (`bd85892b`) | Backs **`DC-NODE-10`**. Pins that the **forge-successor position (tip / block_no) is read from the evolved admitted spine state**, so block 1+ forges at the correct successor position with no stale-position crash. |
| `ci_check_served_chain_stability.sh` | **New** | G-R S1 (`32e4498b`) | Backs **`DC-NODE-11`**. Pins the **monotone serve gate** — the served chain head advances monotonically, so a from-genesis follower sees a **stable served block 0** and can adopt it (no head flap/regression). |

### Modified gate (C1 evidence)

| Check | Status | Origin / change | What it checks |
|-------|--------|-----------------|----------------|
| `ci_check_rehearsal_manifest_schema.sh` | **Modified in place** | G-D origin; G-J extension; **C1 extension (`65954fa3`)** | The rehearsal-manifest schema gate, **extended in place** so its glob set now also covers the **C1 genesis-successor rehearsal manifests** (`docs/evidence/c1-genesis-rehearsal-*` incl. the run-2 `-run2.toml`). The closed schema, the two non-promotability markers (`is_rehearsal` / `not_bounty_evidence`), and the `peer_log_file_sha256` binding are unchanged in shape — only the glob set widened (+19/-churn). Now that **real C1 rehearsal manifests are committed**, the gate is no longer vacuous for the C1 home: it validates the banked manifests against the closed schema. |

**No gate was removed; none was weakened.** The eight new gates each add a closed structural check;
the one modification widens a glob to bring committed C1 evidence under the existing schema.

> **Cross-reference (TRACEABILITY).** Each new gate is the `ci_script` of its rule in the registry at
> HEAD (`DC-NODE-09 → ci_check_node_serve_lifetime`, `CN-WIRE-10 → …versiondata_authority`,
> `CN-WIRE-11 → …findintersect_compat`, `T-REC-04` + `DC-CINPUT-03 → …warmstart_eta0_overlay`,
> `CN-WIRE-12 → …feed_tag24_unwrap`, `DC-CINPUT-04 → …feed_leader_threshold_view`, `DC-NODE-10 →
> …forge_successor_evolved_spine`, `DC-NODE-11 → …served_chain_stability`) — verified by reading the
> registry at `65954fa3`. The TRACEABILITY regen for this window should cite all eight.

## 6. Canonical Type Registry Delta

**n/a — no separate canonical-type registry is configured** (`canonical_type_registry: null`);
canonical-type rules live inline in the invariant registry under family **T**. **One new canonical type was introduced in this window** — the closed BLUE enum
`ArrayHead = Definite(u64) | Indefinite` (`ade_network::codec::primitives`, G-M / CN-WIRE-11; the
codec BLUE submodule count 38 → 39, total **457 → 458**). The other BLUE touches **add no type**:
G-L `handshake::version_table` adds the `encode_n2n_version_params` fn; G-M also adds the
`decode_array_head_two_form` fn; G-N `ade_ledger::seed_consensus_inputs` adds the `epoch_nonce`
field + `SEED_CINPUT_SCHEMA_VERSION 1 → 2` bump — all extend existing types. No new
`CoordinatorEvent` variant or field. The C1 evidence pass adds no type (it reuses the G-D
`PrivateRehearsalManifest` envelope).

## 7. Normative / Invariant Rule Delta (319 → 328)

**Nine rule IDs added, zero strengthenings recorded in-span, zero removed** (319 → 328). All nine new
rules are committed **`enforced`** in this span (verified by reading the registry at `65954fa3`:
each has `status = "enforced"`, `introduced_in` = its cluster, and a bound `ci_script`). **No
`declared → enforced` close-flip is owed** — each rule was committed enforced at its cluster close.

| Rule | Family / Tier | Introduced in | What it pins |
|------|---------------|---------------|--------------|
| `DC-NODE-09` | DC / `derived` | G-K | `--mode node` On-arm **serve-listener lifetime is decoupled from feed end** — serve outlives the live feed so a follower can dial/block-fetch past feed-end. `ci_script = ci/ci_check_node_serve_lifetime.sh`. |
| `CN-WIRE-10` | CN / `derived` | G-L | Serve-side N2N handshake **per-version `versionData`** is the single authority, **NodeToNodeV_15 cardano-node-compatible** (live-confirmed). `ci_script = ci/ci_check_n2n_handshake_versiondata_authority.sh`. |
| `CN-WIRE-11` | CN / `derived` | G-M | Served ChainSync **`FindIntersect`** is decoded (scoped indefinite-length CBOR) and answered with **`IntersectFound`/`Origin`** as the real node expects for a from-genesis follower. `ci_script = ci/ci_check_chainsync_findintersect_compat.sh`. |
| `T-REC-04` | T / `true` | G-N | The recovered seed-epoch consensus surface is byte-faithful **including eta0** — eta0 is persisted in the seed-epoch sidecar (recovery-stable). `ci_script = ci/ci_check_warmstart_eta0_overlay.sh`. |
| `DC-CINPUT-03` | DC / `derived` | G-N | On **WarmStart recovery** the recovered **eta0 is overlaid into the forge consensus surface**, so the follower's leader/VRF check uses the correct nonce (closes `VRFKeyBadProof`). `ci_script = ci/ci_check_warmstart_eta0_overlay.sh`. |
| `CN-WIRE-12` | CN / `derived` | G-O | The **feed-side BlockFetch** path **unwraps the tag-24 (`#6.24`) envelope before decoding** the block body (closes `UnexpectedType`) — receive-side counterpart to the serve-side `CN-WIRE-08`. `ci_script = ci/ci_check_feed_tag24_unwrap.sh`. |
| `DC-CINPUT-04` | DC / `derived` | G-P | The **feed-side header-validation (leader-threshold stake-distribution) view is built from the recovered consensus surface** (closes `VerificationFailed` at validation Step 5 + Step 7; enables live block-0 ingest). `ci_script = ci/ci_check_feed_leader_threshold_view.sh`. |
| `DC-NODE-10` | DC / `derived` | G-Q | The **forge-successor position (tip / block_no) is read from the evolved admitted spine state** — block 1+ forges at the correct successor position, no stale-position crash. `ci_script = ci/ci_check_forge_successor_evolved_spine.sh`. |
| `DC-NODE-11` | DC / `derived` | G-R | **Monotone serve gate** — the served chain head advances monotonically, so a from-genesis follower sees a **stable served block 0** and can adopt it. `ci_script = ci/ci_check_served_chain_stability.sh`. |

**No rule was removed (expected: 0); no in-span `strengthened_in` append was recorded** — the 319 →
328 delta is nine purely-additive IDs, each `enforced` with a bound CI gate. Family spread: **6 DC**
(`DC-NODE-09/10/11`, `DC-CINPUT-03/04`), **3 CN** (`CN-WIRE-10/11/12`), **1 T** (`T-REC-04`).

## C1 evidence — genesis-successor rehearsal reproduction (`129d25ac`, `65954fa3`)

After G-R closed the genesis-successor leg, two commits bank the **C1 reproduction evidence** — **no
source change** (`git diff --name-only 0d7624d4..65954fa3 -- 'crates/*/src/'` is empty); docs/evidence
+ the one gate-glob extension only. **5 files, +322 / -5.**

- **`129d25ac`** — `docs: C1 genesis-successor rehearsal reproduction runbook (post-G-R, two-phase,
  fidelity-fenced)`. A **two-phase, fidelity-fenced** operator runbook
  (`docs/evidence/c1-genesis-rehearsal-reproduction-README.md`, +291) for reproducing the
  G-R-demonstrated follower-adopts-block-0 run. "Fidelity-fenced" = it reuses the **same** `--mode
  node` accepted/served path (no C1-only flag or from-genesis constructor), consistent with the
  live-pass-path-fidelity discipline and `CN-REHEARSAL-FIDELITY-01`.
- **`65954fa3`** — `evidence(c1): run-2 genesis-rehearsal reproduction + runbook flag fixes + gate
  now covers c1 manifests`. Banks a **second recorded run** (`c1-genesis-rehearsal-{follower-run2.log,
  manifest-run2.toml, peer-accept-run2.jsonl}`), fixes runbook flag strings, and **extends
  `ci_check_rehearsal_manifest_schema.sh`** so its glob covers the C1 genesis-rehearsal manifests
  (so the committed run-1 + run-2 manifests are schema-validated, not vacuous).

**NARROW CLAIM (load-bearing).** The C1 evidence demonstrates the **genesis-successor leg
reproducing in the C1 private-testnet rehearsal venue** (follower adopts an Ade-forged stable block
0) across **two recorded runs**. It is **C1 rehearsal infrastructure**, validated under
`CN-REHEARSAL-FIDELITY-01` non-promotability (`is_rehearsal = true`, `not_bounty_evidence = true`).
It is **NOT** a preview/preprod bounty accept, flips **no** `RO-LIVE` rule, and is **not** bounty
evidence. **Preview/preprod acceptance remains the single bounty deliverable**, captured separately;
`RO-LIVE-01` / `RO-LIVE-06` stay operator-gated.

## Honest residual (window scope)

This window closed eight clusters that each removed one concrete blocker on the C1
genesis-successor follower path, plus banked the C1 reproduction evidence. The honest boundary:

- **A chain of NARROW, live-confirmed fixes — not a bounty accept.** Each of G-L…G-R confirms that
  *the specific next-blocker error against the real `cardano-node` peer is gone* (handshake
  `'unknown encoding'`, ChainSync FindIntersect, `VRFKeyBadProof`, feed `UnexpectedType`,
  `VerificationFailed`, successor crash, served-head flap). Together they let a **C1 follower adopt
  an Ade-forged block 0** in the rehearsal venue. None of this is preview/preprod acceptance.
- **NO `RO-LIVE` flip; no bounty/preview/preprod claim.** No `RO-LIVE` rule was flipped in this
  window. `RO-LIVE-01` / `RO-LIVE-06` stay operator-gated. The C1 genesis-successor rehearsal is
  rehearsal infrastructure under `CN-REHEARSAL-FIDELITY-01`, **not** bounty evidence.
- **C1, not preprod.** The live-confirmation venue is the **C1 private testnet** (operator-controlled
  stake, fast leader rights), which is the rehearsal target. The bounty deliverable is
  preview/preprod acceptance over a non-operator-controlled peer — a separate, still-owed capture.
- **No durable long-chain progression demonstrated.** G-Q demonstrates **block 1+** (the first
  stable successor, no crash) and G-R a stable served block 0; neither demonstrates a durable
  many-block forged chain. That is downstream of a sustained accepted+served run.
- **No BLUE-authority weakening.** The BLUE touches (G-L handshake, G-M chain-sync/codec, G-N
  seed-consensus) weaken no codec authority — they extend existing wire/recovery surfaces to match
  the real node (per-version version-data, scoped indefinite-length decode, eta0 in the recovered
  surface). The one **new** type is G-M's closed enum `ArrayHead` (457 → 458, +1); G-L adds a fn and
  G-N a field, no type. **+1 canonical-type delta (ArrayHead), 0 rule removals, 0 in-span
  strengthenings.**

---

## Historical — PHASE4-N-F-G-J window (`13028d49 → 550eec3a`)

> The section below is the **previous** HEAD_DELTAS lead, preserved in condensed form. It narrated the
> **PHASE4-N-F-G-J** cluster (`13028d49..550eec3a`) — genesis-successor block correctness. The
> **G-J close-pass** commit (`487bd829`) that completed it is the first commit of *this* window's
> span (see §3, "G-J close-pass"). Counts in this Historical section are the G-J figures (319 rules,
> 126 CI gates, 2284 tests at `550eec3a`); this window measures **forward** from `550eec3a`. The full
> G-J §§0–8 narrative (and the G-D window before it) is recoverable from this doc's git history at
> `550eec3a`.

> Baseline: `13028d49` (Close PHASE4-N-F-G-I — shared admission bootstrap persists seed-epoch anchor lineage, 2026-06-03 12:27)
> HEAD: `550eec3a` (C1 genesis-successor rehearsal harness — PHASE4-N-F-G-J S5, 2026-06-03 22:02)
> Cluster: **PHASE4-N-F-G-J — genesis-successor block correctness** (empty-feed forge scheduling → null PrevHash wire authority → position rule → cold-start reachability → C1 genesis rehearsal harness), slice span closed; close-pass commit `487bd829` (first commit of the current window).
> 17 commits (no merges), 48 files changed, +3587 / -84 lines.

This window narrated the **PHASE4-N-F-G-J cluster** — **genesis-successor block correctness** on the
`--mode node` spine. The cluster answered one structural question: *can the node legitimately forge
and serve the FIRST block of a from-genesis chain — the genesis-successor — with the CORRECT wire
shape and the CORRECT cold-start permission?* It did so across **five slices**, walking inward from a
diagnostic surface to the BLUE wire grammar and back out to a path-faithful rehearsal harness:

- **S1 (`60303079`) — emit-only feed/forge scheduling events.** New **GREEN**
  `ade_node::live_log::sched_event` (a closed, fail-closed-on-unknown JSONL event vocabulary —
  `feed_unavailable{reason}`, `forge_tick_considered`, `forge_tick_skipped`, `forge_attempted`,
  `forge_result`, all with closed reason/outcome enums and **no** catch-all variant) + new **GREEN**
  `ade_node::live_log::sched_writer`. The closed S1 reason set is exactly three —
  `NoBlockAvailable` and `CleanEmpty` are forge-**eligible**; `UnknownDisconnected` is **INELIGIBLE**
  (fail-closed-on-ambiguity). **Emit-only.** New gate `ci_check_node_sched_events_emit_only.sh`
  (later hardened by `36b2216f`). New rule **`CN-NODE-04`** (`enforced`).
- **S2 (`3b24c572`) — PrevHash null/hash32 wire authority.** New **BLUE** canonical sum type
  `PrevHash = Genesis | Block(Hash32)` in `ade_types` + the **POSITION-BLIND** `$hash32 / null`
  codec in `ade_codec`. New gate `ci_check_prevhash_single_wire_authority.sh`. Backs **`CN-WIRE-09`**
  clause 1.
- **S3 (`0c1939a1`) — genesis-successor position rule + genesis forge.** New **BLUE**
  `ade_ledger::block_validity::header_position` (`block_number 0 ⟺ PrevHash::Genesis`). Producer
  `prev_hash` migrated **`Hash32` → `PrevHash` end-to-end**. Backs **`CN-WIRE-09`** clause 2.
- **S4 (`3df8bd4f`) — node-spine cold-start first-block reachability.** **GREEN**
  `forge_header_position` + a `may_cold_start_forge` permission gate; the genesis-successor forge
  fires EXACTLY ONCE from the recovered seed-epoch lineage. New gate
  `ci_check_genesis_successor_reachability.sh`. New rule **`DC-NODE-08`** (`enforced`).
- **S5 (`550eec3a`) — C1 genesis-successor rehearsal harness.** A path-faithful, **non-promotable**
  rehearsal harness reusing `ba02_evidence::correlate` + the G-D `PrivateRehearsalManifest` envelope
  verbatim. The G-D rehearsal gate `ci_check_rehearsal_manifest_schema.sh` **extended in place**.
  **`CN-REHEARSAL-FIDELITY-01`** gained `strengthened_in += PHASE4-N-F-G-J`.

**G-J headline (at `550eec3a`):** CI gates **123 → 126** (+3 new + 1 modified in place); registry
**316 → 319** (+3 new `CN-NODE-04`, `CN-WIRE-09`, `DC-NODE-08`; 1 strengthening
`CN-REHEARSAL-FIDELITY-01`); BLUE canonical types **456 → 457** (+1 `PrevHash`); tests **2245 →
2284**. **NARROW CLAIM:** G-J enforced the genesis-successor forge MECHANISM + wire AUTHORITY +
rehearsal HARNESS — **not** a live C1 accepted block, **no** `RO-LIVE` flip, **no** BLUE-authority
weakening (the `Block`-path wire encoding is byte-identical after the `Hash32 → PrevHash` migration).
The live C1 genesis rehearsal stayed `blocked_until_operator_c1_genesis_successor_rehearsal` — the
work the current window's G-K…C1 clusters then carried forward toward a real C1 follower.

> *(The G-E…G-I leads were never re-led in HEAD_DELTAS — each was closed with its own grounding-doc
> refresh and lives in its own close-pass commit + the registry; they are not reconstructed here.
> This catch-up regen preserves the G-J lead in condensed form and measures the new
> `550eec3a..65954fa3` span above.)*

---

## Generation notes

### Regen `550eec3a → 65954fa3` (G-K…G-R + C1 catch-up — current lead)

- **Explicit span, NOT the config baseline.** This regen was run against the **explicit**
  `550eec3a..65954fa3` span. The `.idd-config.json` `head_deltas_baseline` at this regen is
  **`853344f7`**, which is **stale and mislabeled**: `853344f7` is mid-cluster **G-L S2** (the
  `feat(network): serve-side N2N handshake emits per-version versionData` impl), and its config note
  wrongly calls it the "G-K close." Measuring from it would silently drop the **G-J close-pass
  `487bd829`** and **all of cluster G-K (`DC-NODE-09`)**. The correct baseline is **`550eec3a`** —
  the G-J slice-span HEAD the four grounding docs were last regenerated against — so the delta is
  gap-free. The close-pass should bump `head_deltas_baseline` `853344f7 → 65954fa3` (and update the
  stale `_invariant_registry_doc` "321 entries" comment to **328**).
- **Multi-cluster catch-up (NOT a single-cluster lead).** This is a deliberate catch-up: eight closes
  (G-K…G-R) + a G-J close-pass + a C1 evidence pass landed since the docs were last regenerated.
  Structured as a per-cluster narrative (§3 + §"C1 evidence"), oldest→newest within the span ordering,
  rather than the usual single-cluster lead.
- Counts are mechanical (git/grep/ls only, no cargo): commit log + `--shortstat` over
  `550eec3a..65954fa3` (**28** commits, no merges / **73** files / **+4967 / -243**); CI gate count
  via `git ls-tree -r --name-only <ref> ci/ | grep -c 'ci_check_.*\.sh'` at each ref (**126 → 134**,
  **+8 new** — one per cluster G-K…G-R — plus `rehearsal_manifest_schema` **modified in place** by
  the C1 evidence commit; `--diff-filter=A` over `ci/` lists exactly the eight new gates,
  `--diff-filter=M` lists only `rehearsal_manifest_schema`, `--diff-filter=D` is empty); registry
  rule count via `grep -c '^\s*id\s*='` at each ref (**319 → 328**; `comm` of sorted id lists shows
  exactly nine adds — `DC-NODE-09`, `CN-WIRE-10`, `CN-WIRE-11`, `T-REC-04`, `DC-CINPUT-03`,
  `CN-WIRE-12`, `DC-CINPUT-04`, `DC-NODE-10`, `DC-NODE-11` — and **zero** removals, no duplicates);
  workspace test attributes via `git grep -hE '#\[(tokio::)?test'` over `crates/**/*.rs` + top-level
  `*.rs` (**2305 → 2324**, +19).
- **All nine new rules committed `enforced` in-span (NO close-flip owed).** Verified by reading the
  registry at `65954fa3`: each of the nine IDs is `status = "enforced"`, `introduced_in` = its
  cluster (G-K…G-R), with a bound `ci_script`; **none** records an in-span `strengthened_in` append,
  and a grep for `strengthened_in` tokens `PHASE4-N-F-G-[K-R]` over the HEAD registry returns empty.
- **No new module, +1 new canonical type, no Cargo.toml change.**
  `git diff --diff-filter=A --name-only 550eec3a..65954fa3 -- 'crates/*/src/'` is empty (no new source
  FILE; new `.rs` files are test fixtures / capture-replay tests) — but G-M adds a new BLUE *type*
  inside an existing file: the closed enum `ArrayHead = Definite(u64) | Indefinite` in
  `ade_network/src/codec/primitives.rs` (457 → 458). The `--diff-filter=A` file check does **not**
  catch a type added to an existing file — canonical-type deltas must be verified by introspecting
  `struct`/`enum` definitions over the BLUE `core_paths`, not by new-file diff. `git diff --name-only
  550eec3a..65954fa3 -- '**/Cargo.toml' 'Cargo.toml'` is empty (no feature-flag delta). The other BLUE
  touches (G-L `ade_network::handshake` `encode_n2n_version_params` fn, G-N
  `ade_ledger::seed_consensus_inputs` `epoch_nonce` field — all `core_paths` per `.idd-config.json`)
  extend existing types and add no type.
- **Cluster-doc archival state (observation, not a blocker).** At HEAD `65954fa3`, the **G-J / G-K /
  G-L** cluster docs are archived under `docs/clusters/completed/`, but the **G-M / G-N / G-O / G-P /
  G-Q / G-R** cluster docs are still at `docs/clusters/PHASE4-N-F-G-X/` (not yet moved to
  `completed/`). This is a docs-housekeeping residual, not a rule/CI/test discrepancy — all nine
  rules are `enforced` and all gates are on disk regardless. The next `/cluster-close` housekeeping
  pass should archive the six unmoved cluster docs.
- **Sibling-doc coherence (catch-up).** At the prior baseline `550eec3a` the four grounding docs were
  G-J-current (319 rules / 126 CI / 2284 tests). The CODEMAP / SEAMS / TRACEABILITY refresh for
  G-K…G-R + C1 is committed alongside this HEAD_DELTAS in the catch-up close-pass and should read
  **328 rules / 134 CI checks** at HEAD. **No rule removal, not a discipline violation** — the +9
  rules / +8 gates are all additive and `enforced`. The next gating work remains the
  **operator-witnessed bounty live pass** (preview/preprod acceptance over a non-operator-controlled
  peer), **not advanced by this window** — which ships only the C1 genesis-successor follower path +
  its rehearsal reproduction evidence ahead of it.
