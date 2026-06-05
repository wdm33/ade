# Ade — HEAD Deltas (Changes Since Baseline)

> **Status:** Living architectural document. Regenerated; not hand-edited.
>
> Regenerate with `/head-deltas <baseline>` after every cluster close. Baseline is recorded in `.idd-config.json` `head_deltas_baseline`.

> Baseline: `b0365df0` (Close PHASE4-N-AA — bounded peer-driven serve range (DC-SERVEMEM-01), 2026-06-06 01:43)
> HEAD: `c6e7fafe` (Close PHASE4-N-AB — outbound mux segmentation (CN-SESS-05), 2026-06-06 03:48)
> Span: **a grounding-doc refresh + the PHASE4-N-AB cluster** — the prior-lead's PHASE4-N-AA close-refresh commit (`e9cd60fc`, which refreshed CODEMAP/TRACEABILITY/SEAMS/HEAD_DELTAS for the N-AA close + folded in a registry consistency fix) followed by the single closed cluster **PHASE4-N-AB — outbound mux segmentation** (the second pre-RO-LIVE hardening item).
> 5 commits (no merges), 10 files changed, +1130 / −406 lines.

> **Baseline note (load-bearing — read before §0).** This window's baseline is **`b0365df0`**, the
> `.idd-config.json` `head_deltas_baseline` set by the *previous* (PHASE4-N-AA close) regen — and it is
> **valid**: `git rev-parse b0365df0` resolves and `git merge-base b0365df0 HEAD == b0365df0` (it is a
> strict ancestor of HEAD; `b0365df0` carries no tag). HEAD is **`c6e7fafe`** (the PHASE4-N-AB close).
> The span has **two parts**: (1) the span-opening commit `e9cd60fc` — the *grounding-doc refresh for the
> PHASE4-N-AA close* (it refreshed all four grounding docs CODEMAP/TRACEABILITY/SEAMS/HEAD_DELTAS for the
> N-AA close and folded in a registry consistency fix; it is the tail of the *prior* N-AA lead, included
> here because it sits inside this span); and (2) the **PHASE4-N-AB cluster** (4 commits) — a
> **GREEN-only** hardening cluster that lets the `--mode node` serve path transmit a block larger than
> one mux frame. The closer bumps `head_deltas_baseline` `b0365df0 → c6e7fafe` after this regen so the
> next cluster measures from here.

This window is **led by a single closed cluster: PHASE4-N-AB — outbound mux segmentation.** It is
**pre-RO-LIVE hardening item 2** and it closes a **receive/send asymmetry** that PHASE4-N-M-FRAG left
half-open: Ade could *receive* a block fragmented across multiple mux frames (CN-SESS-04 inbound
reassembly), but it could **not transmit one** — `handle_outbound` (and the single-frame encoder
`encode_inner_frame`) **errored `OutboundPayloadTooLarge`** for any payload above `MAX_PAYLOAD = 65535`
bytes (the mux SDU limit). A served Conway BlockFetch `Block` larger than 64 KB therefore could not
leave the node. N-AB closes that gap in one slice:

- **S1 — outbound mux segmentation (`6e814ca9` doc, `02e6e557` impl; `CN-SESS-05 → enforced`).** The
  **GREEN** session reducer's `handle_outbound` (`crates/ade_network/src/session/core.rs`) now
  **segments** a payload in the range `MAX_PAYLOAD < len <= MAX_OUTBOUND_PAYLOAD_BYTES` into **ordered
  `<= MAX_PAYLOAD` mux frames** — `payload.chunks(MAX_PAYLOAD)`, each chunk encoded via the **single
  `encode_inner_frame` authority** — and **fails closed above** a new **fixed, non-configurable** const
  `MAX_OUTBOUND_PAYLOAD_BYTES = 16 MiB` (symmetric with the inbound `MAX_REASSEMBLY_TAIL_BYTES` /
  DC-LIVEMEM-01 reassembly cap). Every segment carries the **same** mini-protocol id + mode and the
  **same captured `timestamp`** (GREEN — no per-segment clock read); an empty payload still emits exactly
  one (empty) frame; concatenating the segment payloads reconstructs the original **byte-for-byte**. The
  per-frame `encode_inner_frame` guard (`payload.len() > MAX_PAYLOAD`) **stays strict** (it now always
  receives an already-`<= MAX_PAYLOAD` chunk). New gate `ci_check_outbound_segmentation.sh`.

**The headline:** Ade can now **SERVE a > 64 KB block**. Outbound mux segmentation is the **inverse of
the CN-SESS-04 inbound reassembly** — N-M-FRAG made Ade *reassemble* peer-fragmented block-fetch
responses; N-AB makes Ade *segment* its own outbound payloads the same way — **closing the receive/send
asymmetry** on the wire layer. **Both gating reviews PASS** (per-slice and per-cluster) — **no MEDIUM
this cluster**. The window is **GREEN-only**: **0 BLUE canonical-type change** (458 unchanged), no
`RO-LIVE` flip, no behavior change to the authoritative core.

## 0. Headline

| Count | Baseline (`b0365df0`) | HEAD (`c6e7fafe`) | Δ |
|---|---|---|---|
| CI gates (`ci/ci_check_*.sh`) | 136 | **137** | **+1** — **one NEW gate**, `ci_check_outbound_segmentation.sh` (S1, `CN-SESS-05`; added — `--diff-filter=A`). **No gate modified, no gate removed** in `ci/` this span (`--diff-filter=M` and `--diff-filter=D` over `ci/` are both empty). The full sweep at HEAD includes the new gate. |
| Registry rules (`docs/ade-invariant-registry.toml`) | 334 | **335** | **+1** — one NEW rule **`CN-SESS-05`** (`tier = derived`, `introduced_in = "PHASE4-N-AB"`, `status = enforced`). **Zero removed** (`comm` of the sorted id lists shows no removal). |
| Registry status (enforced / partial / declared) | 202 / 20 / 112 | **203 / 20 / 112** | **+1 enforced** — `CN-SESS-05` lands `enforced` at the S1 close (declared at cluster scoping → enforced at the same close). |
| Registry strengthenings | — | **2** | `strengthened_in += "PHASE4-N-AB"` on exactly two rules: **`CN-SESS-04`** (inbound multi-frame reassembly — N-AB adds the symmetric outbound segmentation) and **`DC-SERVEMEM-01`** (bounded peer-driven serve range — the serve path can now transmit a large block within the existing bound). Both are strengthenings, **not** new rules. |
| BLUE canonical types | 458 | **458** | **0** — **GREEN-only span.** No `ade_core` / `ade_codec` / `ade_types` / `ade_crypto` / `ade_plutus` / `ade_ledger` / `ade_network`-BLUE (`mux::frame` / `codec` / `handshake` / `chain_sync` / `block_fetch` / `tx_submission` / `keep_alive` / `peer_sharing` / `n2c`) source change. The lone code touch is in `ade_network::session::core` — the **GREEN** session reducer (per `.idd-config.json`: `mux::frame` is BLUE; **`session` is RED/GREEN**). |
| Grounding docs | CODEMAP/SEAMS/TRACEABILITY refreshed for N-AA in `e9cd60fc` (span-opening) | **CODEMAP on disk; SEAMS + TRACEABILITY refreshing in parallel with this regen for the N-AB close** | The span-opening commit `e9cd60fc` refreshed all four grounding docs for the **N-AA** close. This **N-AB** close refreshes them again: CODEMAP is on disk and SEAMS + TRACEABILITY are being regenerated in parallel with this HEAD_DELTAS to carry `CN-SESS-05`. See the cross-reference note in §2/§5. |

This is a **single-cluster lead** (PHASE4-N-AB) preceded by the prior-lead's N-AA close-refresh tail. The
slice↔rule↔gate map for the cluster:

| Slice | Rule | Gate | What shipped |
|---|---|---|---|
| **S1** (`02e6e557`) | **`CN-SESS-05`** (NEW, enforced) | **`ci_check_outbound_segmentation.sh`** (NEW) | GREEN `session::core::handle_outbound` segments payloads `> MAX_PAYLOAD` into ordered `<= MAX_PAYLOAD` mux frames via the single `encode_inner_frame` authority (one captured timestamp reused across a message's segments); new fixed const `MAX_OUTBOUND_PAYLOAD_BYTES = 16 MiB`, fail-closed above it; `encode_inner_frame` stays strict single-frame. |

The per-commit shape:

| Commit | Kind | What it did | Code / CI / registry effect |
|---|---|---|---|
| `e9cd60fc` | docs (prior-lead tail) | Grounding-doc refresh for the PHASE4-N-AA close (CODEMAP/TRACEABILITY/SEAMS/HEAD_DELTAS) + registry consistency fix | **0 code / 0 CI**; registry: consistency fix only (no rule added/removed) |
| `87713149` | docs (cluster doc) | PHASE4-N-AB cluster doc; **declare `CN-SESS-05`** | **0 code / 0 CI**; registry: `CN-SESS-05` added `declared` |
| `6e814ca9` | docs (slice doc) | S1 slice doc (outbound mux segmentation) | **0 code / 0 CI / 0 registry** |
| `02e6e557` | feat(session) | S1 impl — `handle_outbound` segmentation + `MAX_OUTBOUND_PAYLOAD_BYTES = 16 MiB` fail-closed; `encode_inner_frame` stays strict; 7 tests | **GREEN code** (`session/core.rs`, +192 / −3); **+1 CI** (`ci_check_outbound_segmentation.sh`); registry: `CN-SESS-05 → enforced` |
| `c6e7fafe` | chore (close) | Close PHASE4-N-AB — archive cluster/slice docs; `strengthened_in += "PHASE4-N-AB"` on `CN-SESS-04` + `DC-SERVEMEM-01` | **0 code / 0 CI**; registry: 2 strengthenings (no new rule); 2 doc moves to `docs/clusters/completed/PHASE4-N-AB/` |

## 1. Commit Log (newest first)

| Hash | Type | Summary |
|------|------|---------|
| `c6e7fafe` | chore (close) | Close PHASE4-N-AB — outbound mux segmentation (CN-SESS-05) |
| `02e6e557` | feat | outbound mux segmentation (PHASE4-N-AB S1, CN-SESS-05) |
| `6e814ca9` | docs | slice doc PHASE4-N-AB S1 outbound mux segmentation |
| `87713149` | docs | cluster doc PHASE4-N-AB outbound mux segmentation + declare CN-SESS-05 |
| `e9cd60fc` | docs | grounding-doc refresh for PHASE4-N-AA close (CODEMAP/TRACEABILITY/SEAMS/HEAD_DELTAS) |

No merge commits in the span. **5 commits, zero unclassified** — one carries an explicit
conventional-commits prefix (`feat(session):`), three are `docs:`, and the close commit `c6e7fafe`
("Close PHASE4-N-AB …") is a `/cluster-close`-style record (its diff scope is exclusively `docs/` +
`docs/ade-invariant-registry.toml` + `.idd-config.json`, so it classifies `chore`/`docs`). The shape is
**N-AA-close refresh → declare → S1 → close**: the prior-lead grounding refresh (`e9cd60fc`), the cluster
doc declaring `CN-SESS-05` (`87713149`), the single slice (S1 doc `6e814ca9`, impl `02e6e557`), and the
close (`c6e7fafe`). The cluster work landed 2026-06-06 (the close at 03:48).

> **Note (commit-attribution policy).** Per this repo's `CLAUDE.md` override (vibe-coded-node bounty
> trailer requirement), commits in this repo carry a `Co-Authored-By:` model-attribution trailer; that
> is an Ade-local override of the global no-AI-attribution rule and applies to **commit messages
> only**. It does not affect this doc's content.

## 2. New Modules

**None.** `git diff --diff-filter=A --name-only b0365df0..c6e7fafe` shows **no new `.rs` source file**,
no new crate, no new `Cargo.toml`, no new workspace. The only added files this span are **one CI gate**
(`ci/ci_check_outbound_segmentation.sh`, §5) and **two cluster/slice docs**
(`docs/clusters/completed/PHASE4-N-AB/{cluster.md, S1-outbound-mux-segmentation.md}`). The span is
**modification only** in code — the single touched source file is the **existing** GREEN module
`ade_network::session::core` (`crates/ade_network/src/session/core.rs`).

> **Cross-reference (CODEMAP/SEAMS) — `CN-SESS-05` carried by the parallel N-AB refresh.** This span adds
> **no new module**, but it adds **new GREEN surface to an existing module**: the new const
> `MAX_OUTBOUND_PAYLOAD_BYTES` and the segmentation logic inside `handle_outbound` (in
> `ade_network::session::core`). The `session` module itself is already in CODEMAP. CODEMAP/SEAMS are
> being **refreshed in parallel** with this HEAD_DELTAS for the N-AB close to fold in the outbound-
> segmentation surface and the `CN-SESS-05` seam; at the instant this doc was written, a `grep` for
> `MAX_OUTBOUND_PAYLOAD_BYTES` / `CN-SESS-05` in CODEMAP/SEAMS may still return 0 (the parallel refresh
> lands them). The registry already records `CN-SESS-05` authoritatively (`code_locus =
> crates/ade_network/src/session/core.rs`). This is a **refresh-in-flight** item, not a discipline gap.

## 3. Modules Modified

One module changed this span (a single GREEN-shell `.rs` file; **zero BLUE**):

| Module | Scope | Key changes |
|--------|-------|-------------|
| `ade_network::session::core` (`crates/ade_network/src/session/core.rs`) | +192 / −3 (GREEN reducer) | **S1 (`02e6e557`).** Adds **outbound mux segmentation** — the inverse of CN-SESS-04 inbound reassembly. `handle_outbound` now splits a payload in the range `MAX_PAYLOAD < len <= MAX_OUTBOUND_PAYLOAD_BYTES` into **ordered `<= MAX_PAYLOAD` chunks** (`payload.chunks(MAX_PAYLOAD)`), each encoded via the **single `encode_inner_frame` authority**, concatenated into one `SendBytes` effect; every segment carries the **same** mini-protocol id + mode and the **same captured `timestamp`** (GREEN — no per-segment clock); an **empty payload still emits exactly one (empty) frame**; `concat(segment payloads) == payload` (byte-preserving, lossless). New **fixed, non-configurable const `MAX_OUTBOUND_PAYLOAD_BYTES: usize = 16 * 1024 * 1024`** (symmetric with the inbound `MAX_REASSEMBLY_TAIL_BYTES` / DC-LIVEMEM-01) — `handle_outbound` **fails closed above it** (`SessionError::OutboundPayloadTooLarge`). The per-frame `encode_inner_frame` guard (`payload.len() > MAX_PAYLOAD`) is **unchanged** (it now always receives an already-`<= MAX_PAYLOAD` chunk — single-frame encoder authority preserved). Adds **7 tests** (`tests::outbound_*`) covering: `== MAX_PAYLOAD` is one frame; `MAX_PAYLOAD + 1` segments into two; segment order preserved; segments keep the same mini-protocol id + mode; a 70 KB block-fetch-shaped payload round-trips byte-identical through Ade's **own** inbound CN-SESS-04 reassembly; the upper bound (`MAX_OUTBOUND_PAYLOAD_BYTES`) is allowed (every byte segmented, none dropped); above the upper bound fails closed. **No new BLUE type, no signature change to any BLUE surface.** |

> **No BLUE-authority change (load-bearing).** This span touches **no BLUE source file** — the single
> code change is in the **GREEN** session reducer (`ade_network::session::core`; per `.idd-config.json`
> the BLUE network surface is `mux::frame` + `codec` + the per-protocol codecs, **not** `session`). The
> BLUE canonical-type count is **458 → 458**. Segmentation reuses the **existing** single mux-frame
> encoder authority (`encode_inner_frame → mux::frame::encode_frame`); it introduces **no second/parallel
> frame encoder** (the gate enforces exactly one `encode_frame(` call + one `MuxFrame {` construction in
> the file) and **no new BLUE type**. The new const `MAX_OUTBOUND_PAYLOAD_BYTES` is a GREEN compile-time
> literal.

## 4. Feature Flags

**No project feature-flag deltas.** Ade declares no `[features]` table in any workspace `Cargo.toml`,
and **no `Cargo.toml` changed in this window** (`git diff --name-only b0365df0..c6e7fafe --
'**/Cargo.toml' 'Cargo.toml'` is empty). No `#[cfg(feature = …)]` gate was introduced. The new bound
`MAX_OUTBOUND_PAYLOAD_BYTES = 16 MiB` is a **compile-time `const`** — deliberately **not** a feature
flag, CLI flag, env var, or config knob (it **cannot be disabled at runtime**, per `CN-SESS-05`); the
gate `ci_check_outbound_segmentation.sh` enforces that it stays a fixed non-configurable literal and
that the reducer reads no environment.

## 5. CI Checks (136 → 137; +1 new, 0 modified, 0 removed)

One new gate this span; no gate modified, no gate removed. `git diff --diff-filter=A b0365df0..c6e7fafe
-- ci/` lists exactly the one gate below; `--diff-filter=M` and `--diff-filter=D` over `ci/` are both
**empty**.

### PHASE4-N-AB outbound-segmentation gate (`02e6e557`)

| Check | Status | Origin / change | What it checks |
|-------|--------|-----------------|----------------|
| `ci_check_outbound_segmentation.sh` | **New** | PHASE4-N-AB S1 (`02e6e557`); `CN-SESS-05` | The GREEN session reducer (`session/core.rs`) must **segment** outbound mini-protocol payloads `> MAX_PAYLOAD` into ordered `<= MAX_PAYLOAD` mux frames (the outbound inverse of CN-SESS-04), reusing the **single** existing frame encoder authority, and **fail closed above** a fixed `MAX_OUTBOUND_PAYLOAD_BYTES`. Four fences (over a comment-stripped, pre-`#[cfg(test)]` production view): **(a)** exactly **one** `encode_frame(` call + **one** `MuxFrame {` construction (no second/parallel encoder); **(b)** `encode_inner_frame` keeps its per-frame `payload.len() > MAX_PAYLOAD` guard; **(c)** `handle_outbound` owns segmentation — `chunks(MAX_PAYLOAD)` + the `payload.len() > MAX_OUTBOUND_PAYLOAD_BYTES` fail-closed bound; **(d)** `MAX_OUTBOUND_PAYLOAD_BYTES` is a fixed `const …: usize = <literal>;`, with **no** `std::env` / `env::var` / `env!` / `option_env!` read in the reducer. |

> **Cross-reference (TRACEABILITY) — `CN-SESS-05 ↔ ci_check_outbound_segmentation.sh` carried by the
> parallel N-AB refresh.** TRACEABILITY is being **refreshed in parallel** with this HEAD_DELTAS for the
> N-AB close, to add the `CN-SESS-05 ↔ ci_check_outbound_segmentation.sh` row. The registry already
> records the binding at HEAD (`CN-SESS-05.ci_script = "ci/ci_check_outbound_segmentation.sh"`), so the
> rule↔gate link is **authoritative in the registry**; the TRACEABILITY doc is the view being brought
> current alongside this regen. **No rule↔gate binding was removed.** The new gate enforces a named,
> enforced invariant (`CN-SESS-05`), so it is **not** an orphan gate.

## 6. Canonical Type Registry Delta

**n/a — no separate canonical-type registry is configured** (`canonical_type_registry: null`);
canonical-type rules live inline in the invariant registry under family **T**. **No canonical type was
added or removed in this window** — this is a **GREEN-only** span (BLUE count unchanged, **458 → 458**,
per the CODEMAP header). The new const `MAX_OUTBOUND_PAYLOAD_BYTES` is a **GREEN** declaration, not a
BLUE canonical type. No `Cargo.toml` changed.

## 7. Normative / Invariant Rule Delta (334 → 335; +1 enforced rule, 2 strengthenings, zero removals)

**One rule ID was added; zero removed** (334 → 335; `comm` of the sorted id lists shows the single
addition `CN-SESS-05` and no removal). The status tally moves **202 → 203 enforced** (20 partial /
112 declared unchanged) — the new rule lands `enforced` at the S1 close.

**New rule (`+1`, enforced):**

| Rule | Family / Tier | Statement (summary) |
|------|---------------|---------------------|
| `CN-SESS-05` | CN / `derived` (`enforced`; `introduced_in = "PHASE4-N-AB"`) | **Outbound mini-protocol payloads are segmented across mux frames.** A payload larger than `MAX_PAYLOAD` is segmented into **ordered** mux frames, each `<= MAX_PAYLOAD`, **preserving mini-protocol id, mode, and byte order**; concatenating the segment payloads reconstructs the original **exactly**. Payloads above `MAX_OUTBOUND_PAYLOAD_BYTES` **fail closed**. Segmentation uses the **single** existing frame-encoder authority and is the **outbound inverse of `CN-SESS-04`** inbound reassembly. `ci_script = ci/ci_check_outbound_segmentation.sh`; `cross_ref = [CN-SESS-04, DC-LIVEMEM-01, CN-WIRE-08, DC-SERVEMEM-01, DC-CONS-17]`. Lets the `--mode node` serve path transmit a > 64 KB block, closing the receive/send asymmetry left by PHASE4-N-M-FRAG. |

**Strengthenings (`strengthened_in += "PHASE4-N-AB"`) — exactly two, no rule weakened:**

| Rule | Family / Tier | Strengthening |
|------|---------------|---------------|
| `CN-SESS-04` | CN / `derived` (`enforced`, unchanged) | **Inbound multi-frame reassembly — symmetric outbound segmentation added.** CN-SESS-04 (PHASE4-N-M-FRAG) made Ade **reassemble** inbound block-fetch responses fragmented across mux frames (Conway blocks exceeding the 65535-byte mux SDU limit). N-AB adds the **symmetric outbound** path (`CN-SESS-05`): Ade now **segments** its own outbound payloads the same way, so a large served block is transmittable. The wire-layer fragmentation discipline is preserved + extended to both directions (receive **and** send). |
| `DC-SERVEMEM-01` | DC / `derived` (`enforced`, unchanged) | **Bounded peer-driven serve range — large blocks now transmittable within the bound.** DC-SERVEMEM-01 (PHASE4-N-AA) caps each peer BlockFetch request at `MAX_SERVE_RANGE_BLOCKS = 256` blocks. N-AB makes the **per-block** transmit path complete: a served block larger than one mux frame is now segmented onto the wire (`CN-SESS-05`) rather than failing `OutboundPayloadTooLarge`. The serve path is bounded **and** can now actually transmit the (large) blocks within that bound — strengthening, not weakening, the serve contract. |

> **Both gating reviews PASS — no MEDIUM this cluster (load-bearing).** The PHASE4-N-AB per-slice review
> (S1) and the per-cluster cross-slice review **both PASS with no MEDIUM (or higher) finding** — in
> contrast to PHASE4-N-AA, whose cross-slice review found + fixed a MEDIUM inverted-range panic. The
> outbound-segmentation design is a strict GREEN reducer change that reuses the single existing frame
> encoder, captures one timestamp, is byte-preserving, and fails closed above a fixed bound; the
> round-trip-through-own-inbound-reassembly test (`outbound_large_payload_reassembles_byte_identical_via_inbound`)
> proves the segment→reassemble identity.

**No rule was removed (expected: 0).** The registry delta is **one new enforced rule + two
`strengthened_in` appends** — purely additive / strengthening, consistent with append-only registry
discipline.

## Working tree at HEAD `c6e7fafe`

Clean of tracked changes from this span — the cluster + close are all committed. `git status --short`
shows only an untracked `.mithril-scratch/` (operator scratch, ignored). **This regen runs *after* all
five span commits** (the close commit `c6e7fafe` is HEAD for this window); the CODEMAP/SEAMS/TRACEABILITY
parallel refresh + the baseline bump (`b0365df0 → c6e7fafe`) are the close-pass follow-on actions.

## Honest residual (window scope)

PHASE4-N-AB **lets the serve path transmit a large block** — and that is the entire claim. The honest
boundary:

- **Pre-RO-LIVE hardening, NOT a capability flip.** This is hardening **item 2** (outbound mux
  segmentation). **No `RO-LIVE` rule was flipped** — `RO-LIVE-01` stays operator-gated. No authoritative
  behavior changed; the span is **GREEN-only** (0 BLUE change, 458 canonical types unchanged). It closes
  the receive/send wire-layer asymmetry; it does not advance the bounty.
- **The bound is defensive, not semantic.** `MAX_OUTBOUND_PAYLOAD_BYTES = 16 MiB` is a **fixed
  implementation bound** (symmetric with the inbound `MAX_REASSEMBLY_TAIL_BYTES`), not a Cardano protocol
  parameter. A single outbound payload above 16 MiB fails closed (`OutboundPayloadTooLarge`); it
  **cannot be disabled at runtime**. Real Cardano blocks are far below this ceiling — the bound is a
  defensive ceiling on the outbound buffer, generous over any legitimate single block/item.
- **GREEN reducer, single encoder authority.** The change lives entirely in the GREEN session reducer
  and **reuses the single `encode_inner_frame` mux-frame encoder** — there is **no second/parallel frame
  encoder** (gate-enforced: exactly one `encode_frame(` + one `MuxFrame {`), **no per-segment clock**
  (one captured timestamp reused across a message's segments), and **no BLUE change**. The per-frame
  `encode_inner_frame` guard stays strict.
- **Receive/send symmetry now holds; the prior N-AA LOW residual item is closed.** The PHASE4-N-AA
  honest residual named a **[LOW]** follow-on: *> 64 KB block bodies not served (fail-closed) +
  unbounded inbound serve accept.* N-AB closes the **outbound > 64 KB serve** half (a served block larger
  than one mux frame now segments onto the wire). The **inbound serve accept** bound is governed by the
  existing CN-SESS-04 / DC-LIVEMEM-01 reassembly cap (16 MiB, symmetric). The matched receive/send
  fragmentation discipline (CN-SESS-04 ⟷ CN-SESS-05) now holds in both directions.
- **CODEMAP/SEAMS/TRACEABILITY refreshing in parallel.** This N-AB close regenerates all four grounding
  docs: CODEMAP is on disk and SEAMS + TRACEABILITY are being refreshed in parallel with this HEAD_DELTAS
  to carry `CN-SESS-05` (the outbound-segmentation seam + the rule↔gate binding). The registry records
  the rule + its gate binding authoritatively at HEAD (335 rules). A refresh-in-flight item at the
  instant of writing, not a discipline gap.

---

## Historical — PHASE4-N-AA close + cluster window (`999199f8 → b0365df0`)

> The section below is the **previous** HEAD_DELTAS lead, preserved in condensed form. It was a
> **focused grounding refresh + the PHASE4-N-AA cluster**, narrating the `999199f8 → b0365df0` span.
> Counts in this Historical section are the figures **at `b0365df0`** (334 rules, 136 CI gates, 458
> canonical types); the current window measures **forward** from `b0365df0`. The full §§0–7 narrative is
> recoverable from this doc's git history at `b0365df0`.

> Baseline: `999199f8` (repair 10 pre-existing gate-vs-code drifts (gate hygiene), 2026-06-05 19:28)
> HEAD: `b0365df0` (Close PHASE4-N-AA — bounded peer-driven serve range (DC-SERVEMEM-01), 2026-06-06 01:43)
> Span: **a focused grounding refresh + the PHASE4-N-AA cluster** — 8 commits, 15 files, +1254 / −492.

PHASE4-N-AA was **pre-RO-LIVE hardening item 1** and closed the **MEDIUM** finding the PHASE4-N-U
cross-slice security review left open: *the `--mode node` serve path could be driven by a peer into
unbounded memory + O(N²) CPU work.* Before N-AA, the serve projection (`ChainDbServedSource`, N-U S3)
fulfilled a BlockFetch RequestRange via `ChainDb::iter_from_slot` (materializes the full range into a
`Vec` + per-block hash-index scan) and read the tip via O(N) `chaindb.tip()`. N-AA closed that across
two slices + an in-cluster security fix:

- **S1 — bounded hash-free ChainDb read primitives (`6b8f1779`; CE-1).** Two new **bounded,
  slot-ordered, hash-free** `ChainDb` trait primitives: `range_bytes_capped(from, to, max)` (at most
  `max` blocks, `truncated` flag, no hash-index scan) and `last_block_bytes()` (highest-slot block's
  bytes without an O(N) tip walk). New **RED type** `CappedSlotRange { blocks, truncated }`; the
  unbounded `iter_from_slot` / `tip` doc-fenced as TRUSTED-CALLER reads (internals unchanged).
- **S2 — serve projection cap + fail-closed (`3d853ec0`; `DC-SERVEMEM-01 → enforced`).**
  `ChainDbServedSource` switched onto the S1 bounded primitives behind a fixed `const
  MAX_SERVE_RANGE_BLOCKS = 256` (symmetric with the receive-side `MAX_WIRE_PUMP_LOOKAHEAD`); new **RED
  enum** `ServeRangeOutcome { Served | Empty | CapExceeded | ReadError }` — every non-`Served` maps to
  wire `NoBlocks` (oversized ranges fail closed). New gate `ci_check_serve_range_bounded.sh`.
- **In-cluster security-review MEDIUM (`5c9f6cf6`).** An inverted-range (`from > to`) panic (peer
  controls both BlockFetch endpoints) was found + fixed in-cluster: a `from > to → empty` guard on both
  `ChainDb` impls + a contract test + a serve-path test.

**N-AA headline (at `b0365df0`):** Registry **333 → 334** (+1 enforced `DC-SERVEMEM-01`; +2 strengthenings
`DC-NODE-13` + `DC-LIVEMEM-01`; 0 removed). CI gates **135 → 136** (+1 `ci_check_serve_range_bounded.sh`).
**RED-only — BLUE canonical types 458 → 458.** Serve-side analog of `DC-LIVEMEM-01`; closes the N-U
cross-slice MEDIUM. **No `RO-LIVE` flip.** *(N-AB, the current lead, makes the bounded serve path
transmit large blocks — `DC-SERVEMEM-01 strengthened_in += "PHASE4-N-AB"`.)*

---

## Historical — PHASE4-N-U close + gate-hygiene window (`4e358e92 → 999199f8`)

> Preserved in condensed form. The **PHASE4-N-U cluster CLOSE + a gate-hygiene / close-correction
> tail**, narrating the `4e358e92 → 999199f8` span. Counts here are the figures **at `999199f8`** (333
> rules, 135 CI gates, 458 canonical types). The full §§0–7 narrative is recoverable from this doc's git
> history at `999199f8`.

> Baseline: `4e358e92` (refresh stale G-R serve-handoff comment in containment gate (post-N-U-S3), 2026-06-05 17:17)
> HEAD: `999199f8` (repair 10 pre-existing gate-vs-code drifts (gate hygiene; 0 invariants weakened), 2026-06-05 19:28)
> Span: **PHASE4-N-U cluster CLOSE + a gate-hygiene / close-correction tail** — 4 commits, 23 files, +1063 / −658.

The window was **not a feature cluster** — it was the **PHASE4-N-U close pass** (commit `7f00e75d`,
docs-only: archive + 4-grounding-doc refresh + baseline bump `65954fa3 → 4e358e92`) plus a
**gate-hygiene / close-correction tail** of three CI-only commits (`60deecf3`, `e92b40b7`, `999199f8`).
It answered one operational question the N-U close left open: *is the `ci/ci_check_*.sh` sweep
trustworthy as release evidence — does GREEN actually mean GREEN?* The window repaired **every failing
gate in place** — adding no gate, removing no gate, weakening no invariant: the close itself
(`7f00e75d`, docs-only, all four grounding docs to 458 types / 135 CI / 333 rules); the N-U-stranded
`DC-NODE-06` handoff-fence gate reconciled (`60deecf3`, repointed to the durable-provenance serve);
`ci_check_no_secrets.sh` made to actually run (`e92b40b7`, was exiting 126 on `ARG_MAX` — now 6756
files / 0 secrets); and ten pre-existing gate-vs-code drifts repaired (`999199f8`).

**N-U-close-window headline (at `999199f8`):** CI gates **135 → 135** (0 net — 11 gates repaired in
place); registry **333 → 333** (identical ID set — the lone edit was the `DC-NODE-06` strengthening);
status **201 / 20 / 112** unchanged; BLUE types **458 → 458**. The full `ci/ci_check_*.sh` sweep was
**135 passed / 0 failed** at HEAD (verified by running it). **No `RO-LIVE` flip, no behavior change** —
pure enforcement-trustworthiness work.

---

## Historical — PHASE4-N-U cluster window (`65954fa3 → 4e358e92`)

> Preserved in condensed form. The single-cluster lead **PHASE4-N-U — forged-block durability**,
> narrating the `65954fa3 → 4e358e92` span. Counts here are the figures **at `4e358e92`** (333 rules,
> 135 CI gates, 458 canonical types). The full N-U §§0–7 narrative is recoverable from this doc's git
> history at `4e358e92` / `999199f8`.

> Baseline: `65954fa3` (G-K…G-R + C1 catch-up close, 2026-06-04 23:32)
> HEAD: `4e358e92` (refresh stale G-R serve-handoff comment in containment gate (post-N-U-S3), 2026-06-05 17:17)
> Span: **PHASE4-N-U — forged-block durability** (own-forged durable admit → forged-tip crash recovery + replay-equivalence → serve-as-durable-chain projection) — 14 commits, 28 files, +3726 / −1802.

PHASE4-N-U answered: *once Ade forges its own block, does it become part of the **durable** chain —
survive a crash, replay byte-identically, and get served to a follower — through the SAME gate received
blocks use, with NO second tip-advance path?* Before N-U a forged block was a **local self-accept
artifact only** (`DC-NODE-05`). N-U closed that across three slices:

- **S1 — own-forged durable admit through the pump (`DC-NODE-12` + `DC-CONS-23` + `DC-WAL-04` prior-fp
  clause).** A fenced RED driver `ade_node::node_sync::admit_forged_block_durably` feeds the
  self-accepted bytes (`accepted.into_bytes()`, no re-encode) into the **same**
  `forward_sync::pump_block` chokepoint received blocks use (durable-before-tip, extend-only). New gate
  `ci_check_forged_durable_admit_via_pump.sh`.
- **S2 — forged-tip crash recovery + replay-equivalence (`T-REC-05`, `DC-WAL-04` no-orphan clause).**
  Production `warm_start_recovery` forward-replays from the nearest snapshot ≤ tip and reconciles the
  WAL tail; an un-WAL'd forged orphan is dropped. `T-REC-05` is **test-enforced** (`ci_script = ""`).
- **S3 — serve-as-durable-chain projection (`DC-NODE-13`; strengthens `CN-CONS-07`, `DC-NODE-11`).**
  The `--mode node` served view became a deterministic read-only **projection of the durable ChainDb**
  (the NEW RED module `ade_runtime::network::served_chain_projection` / `ChainDbServedSource`). New gate
  `ci_check_served_chain_projection.sh`; retired gate `ci_check_served_chain_stability.sh` (mechanism
  superseded).

**N-U headline (at `4e358e92`):** Registry **328 → 333** (+5 enforced: `DC-NODE-12`, `DC-CONS-23`,
`DC-WAL-04`, `T-REC-05`, `DC-NODE-13`; +2 strengthenings: `CN-CONS-07`, `DC-NODE-11`; 0 removed). CI
gates **134 → 135** (+1 net: +2 new, −1 retired). **One new RED module**
(`served_chain_projection`). **BLUE canonical types 458 → 458.** **No `RO-LIVE` flip** — durability +
coherent serve ≠ operator-witnessed peer acceptance. *(N-AA bounded this S3 serve projection —
`DC-NODE-13 strengthened_in += "PHASE4-N-AA"`; N-AB then made the bounded serve transmit large blocks —
`DC-SERVEMEM-01 strengthened_in += "PHASE4-N-AB"`.)*

---

## Historical — PHASE4-N-F-G-K … G-R + C1 window (`550eec3a → 65954fa3`)

> Preserved in condensed form. A **multi-cluster catch-up** narrating the `550eec3a..65954fa3` span —
> the PHASE4-N-F-G-J close-pass + eight clusters (G-K through G-R) + the C1 genesis-successor rehearsal
> reproduction evidence. Counts here are the figures **at `65954fa3`** (328 rules, 134 CI gates, 458
> canonical types). The full G-K…C1 §§0–7 narrative (and the G-J window before it) is recoverable from
> this doc's git history at `65954fa3` / `4e358e92` / `999199f8`.

> Baseline: `550eec3a` (PHASE4-N-F-G-J close, 2026-06-03 22:02)
> HEAD: `65954fa3` (run-2 genesis-rehearsal reproduction + runbook flag fixes + gate now covers c1 manifests, 2026-06-04 23:32)
> Span: **G-J close-pass → G-K, G-L, G-M, G-N, G-O, G-P, G-Q, G-R → C1 genesis-successor rehearsal evidence** — 28 commits, 73 files, +4967 / −243.

Ade closed **eight clusters** (G-K through G-R) plus a G-J close-pass and a C1 genesis-successor
rehearsal evidence pass, each peeling off the next blocker toward a live C1 genesis-successor follower
adopting an Ade-forged block 0: serve-listener lifetime (G-K, `DC-NODE-09`) → real-node handshake
compat (G-L, `CN-WIRE-10`) → real-node ChainSync FindIntersect compat (G-M, `CN-WIRE-11`, + the closed
BLUE enum `ArrayHead = Definite(u64) | Indefinite`, the window's only +1 canonical type, 457 → 458) →
recovered-eta0 WarmStart (G-N, `T-REC-04` + `DC-CINPUT-03`) → feed-side tag-24 unwrap (G-O, `CN-WIRE-12`)
→ feed-side leader-threshold view (G-P, `DC-CINPUT-04`) → forge-successor position (G-Q, `DC-NODE-10`) →
stable served block 0 via a monotone serve gate (G-R, `DC-NODE-11`) → and the C1 reproduction evidence.

**G-K…C1 headline (at `65954fa3`):** CI gates **126 → 134** (+8, one per cluster); registry **319 →
328** (+9, all `enforced`); BLUE canonical types **457 → 458** (+1 `ArrayHead`); no new module. **Note:**
the G-R gate `ci_check_served_chain_stability.sh` was **retired in PHASE4-N-U** (mechanism superseded by
serve-as-projection), and `DC-NODE-11` was strengthened there; `DC-NODE-11`'s stranded sibling
`DC-NODE-06` was reconciled in the N-U close window (`60deecf3`).

> *(The G-E…G-I leads were never re-led in HEAD_DELTAS — each was closed with its own grounding-doc
> refresh. The G-J lead before that is recoverable from this doc's git history at `65954fa3`.)*

---

## Generation notes

### Regen `b0365df0 → c6e7fafe` (PHASE4-N-AB — outbound mux segmentation — current lead)

- **Baseline valid; single-cluster lead (GREEN-only) preceded by the prior-lead's N-AA close-refresh
  tail.** Run against the config baseline `b0365df0` (the PHASE4-N-AA close HEAD), which `git rev-parse`
  resolves and `git merge-base b0365df0 HEAD` confirms is a strict ancestor of HEAD `c6e7fafe`
  (`b0365df0` carries no tag). The span is the **grounding-doc refresh for the N-AA close** `e9cd60fc`
  (the prior lead's tail — CODEMAP/TRACEABILITY/SEAMS/HEAD_DELTAS refresh + a registry consistency fix)
  **plus the PHASE4-N-AB cluster** (4 commits: cluster doc + S1 doc + S1 impl + close). The closer bumps
  `head_deltas_baseline` `b0365df0 → c6e7fafe` after this regen.
- **Counts are mechanical (git/grep/ls):** commit log + `--shortstat` over `b0365df0..c6e7fafe`
  (**5** commits, no merges / **10** files / **+1130 / −406**); CI gate count via
  `git ls-tree -r --name-only <ref> ci/ | grep -c 'ci_check_.*\.sh$'` at each ref (**136 → 137**;
  `--diff-filter=A` over `ci/` = exactly `ci_check_outbound_segmentation.sh`; `--diff-filter=M` and
  `--diff-filter=D` over `ci/` both **empty**); registry rule count via `grep -cE '^\[\[rules\]\]'` at
  each ref (**334 → 335**; `comm` of sorted `id =` lists shows the single addition `CN-SESS-05`, zero
  removals); registry status via `grep -E '^status = ' | sort | uniq -c` (**202 → 203 enforced**, 20
  partial / 112 declared unchanged); strengthenings via the registry diff (**2**: `CN-SESS-04` +
  `DC-SERVEMEM-01` each gained `strengthened_in = ["PHASE4-N-AB"]`); BLUE canonical types via the
  CODEMAP header (**458 → 458**).
- **GREEN-only span — no BLUE touch, +0 canonical type, no Cargo.toml change.** `git diff --name-status
  b0365df0..c6e7fafe` shows **no new `.rs` source file** (only `A` for one CI gate + two cluster/slice
  docs, `M` for the single `.rs` file + four grounding docs + the registry + `.idd-config.json`). The
  single touched `.rs` file is `crates/ade_network/src/session/core.rs` — the **GREEN** session reducer
  (per `.idd-config.json`: `mux::frame` is BLUE; `session` is RED/GREEN). `git diff --name-only …
  '**/Cargo.toml' 'Cargo.toml'` is empty (no feature-flag delta).
- **Registry delta is +1 enforced rule + 2 strengthenings, NOT a removal.** `CN-SESS-05` is the new rule
  (declared at the cluster doc `87713149`, enforced at the S1 impl `02e6e557`); `CN-SESS-04` +
  `DC-SERVEMEM-01` gained `strengthened_in += "PHASE4-N-AB"` at the close (`c6e7fafe`). `comm` confirms
  zero removals.
- **Doc-refresh — all four grounding docs regenerated for this close.** The span-opening `e9cd60fc`
  refreshed the four docs for the **N-AA** close. This **N-AB** close regenerates them again: CODEMAP is
  on disk and SEAMS + TRACEABILITY are being refreshed **in parallel** with this HEAD_DELTAS to carry
  `CN-SESS-05` (the outbound-segmentation seam + the `CN-SESS-05 ↔ ci_check_outbound_segmentation.sh`
  binding). The registry already records the rule + gate binding authoritatively at HEAD; the parallel
  CODEMAP/SEAMS/TRACEABILITY refresh brings the narrative docs current alongside this regen.
- **Both gating reviews PASS — no MEDIUM this cluster.** The PHASE4-N-AB per-slice (S1) and per-cluster
  cross-slice security reviews both pass with no MEDIUM+ finding (unlike N-AA, whose cross-slice review
  found + fixed a MEDIUM inverted-range panic). The segment→reassemble identity is proven by
  `outbound_large_payload_reassembles_byte_identical_via_inbound` (segments a 70 KB block-fetch-shaped
  payload outbound, feeds the wire bytes back through Ade's own CN-SESS-04 inbound reassembly, asserts
  byte-identical reconstruction).
- **Working tree clean.** This regen runs *after* all five span commits (the close `c6e7fafe` is HEAD
  for this window); `git status --short` shows only an untracked `.mithril-scratch/` (operator scratch,
  ignored). The remaining close-pass actions are the parallel CODEMAP/SEAMS/TRACEABILITY refresh + the
  baseline bump `b0365df0 → c6e7fafe`.
