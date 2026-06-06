# Ade — HEAD Deltas (Changes Since Baseline)

> **Status:** Living architectural document. Regenerated; not hand-edited.
>
> Regenerate with `/head-deltas <baseline>` after every cluster close. Baseline is recorded in `.idd-config.json` `head_deltas_baseline`.

> Baseline: `c6e7fafe` (Close PHASE4-N-AB — outbound mux segmentation (CN-SESS-05), 2026-06-06 03:48)
> HEAD: `1d54abb4` (Close PHASE4-N-AC — KES signing evolves key to current period (DC-CRYPTO-10), 2026-06-06 11:08)
> Span: **a grounding-doc refresh + the PHASE4-N-AC cluster** — the prior-lead's PHASE4-N-AB close-refresh commit (`6184eb0b`, which refreshed CODEMAP/TRACEABILITY/SEAMS/HEAD_DELTAS for the N-AB close) followed by the single closed cluster **PHASE4-N-AC — KES signing evolves key to current period** (a live-readiness fix surfaced by the item-4 C1 re-run).
> 5 commits (no merges), 12 files changed, +1029 / −340 lines.

> **Baseline note (load-bearing — read before §0).** This window's baseline is **`c6e7fafe`**, the
> `.idd-config.json` `head_deltas_baseline` set by the *previous* (PHASE4-N-AB close) regen — and it is
> **valid**: `git rev-parse c6e7fafe` resolves and `git merge-base c6e7fafe HEAD == c6e7fafe` (it is a
> strict ancestor of HEAD; `c6e7fafe` carries no tag). HEAD is **`1d54abb4`** (the PHASE4-N-AC close).
> The span has **two parts**: (1) the span-opening commit `6184eb0b` — the *grounding-doc refresh for the
> PHASE4-N-AB close* (it refreshed all four grounding docs CODEMAP/TRACEABILITY/SEAMS/HEAD_DELTAS for the
> N-AB close; it is the tail of the *prior* N-AB lead, included here because it sits inside this span);
> and (2) the **PHASE4-N-AC cluster** (4 commits) — a **RED-only** live-readiness cluster that makes the
> producer signing shell **evolve the operator KES key forward to the requested period before signing**,
> so the forge works once the chain's KES period advances past 0. The closer bumps `head_deltas_baseline`
> `c6e7fafe → 1d54abb4` after this regen so the next cluster measures from here.

This window is **led by a single closed cluster: PHASE4-N-AC — KES signing evolves key to current
period.** It is a **live-readiness fix surfaced by the item-4 C1 re-run** and it closes a real forge gap
that only manifests once a chain ages past its first KES period. Before N-AC, the forge's **only real KES
sign** (`produce_mode.rs:891`, `shell.kes_sign_header(kes_period, …)`) required
`kes.current_period() == kes_period`, and **nothing evolved the minted-at-period-0 operator key forward** —
so once the private net aged past one KES period (slot ≈ 249659, `slotsPerKESPeriod = 129600` → KES
period 1) the forge returned `KesPeriodNotCurrent { requested: 1, current: 0 }` on **every** leader slot
(`succeeded = 0`), and the waiting follower `KeepAlive`-timed-out. This was **not** a regression from
N-U / N-AA / N-AB (the wire/serve path still handshook and found the chain-sync intersection) — it was a
latent gap in the producer key-custody shell. N-AC closes it in one slice:

- **S1 — evolve KES key to current period before signing (`68a85dbe` doc, `7d4a4a72` impl;
  `DC-CRYPTO-10 → enforced`).** A new **RED** producer-shell method
  `ProducerShell::kes_sign_header_advancing(period, pre_image)` =
  **`kes_advance_to(period)` then `kes_sign_header(period, pre_image)`** — it evolves the operator KES
  signing key forward to the requested period via the **existing deterministic `Sum6KES` update**
  (`kes_advance_to → kes_update`, which is idempotent when `current == period`), then signs. It
  **fails closed** if the requested period is before the key start (`Signing(EvolutionBackwards)`) or
  beyond the key lifetime / unreachable (`Signing(EvolutionExhausted)`, above `SUM6_MAX_PERIOD = 63`).
  The forge's **single real KES sign** (`produce_mode.rs:891`) is **rewired** from the raw
  `kes_sign_header` to this evolving variant (the period is passed **verbatim** — no `± N`). **Signing
  stays RED**; the `Sum6KES` algorithm, the KES verifier, forge eligibility, and the wire rules are all
  unchanged. New gate `ci_check_kes_evolution_before_sign.sh`.

**The headline:** the **RED producer signing shell now evolves the operator KES key to the requested
period before signing**, so the forge works **across KES periods** instead of only at the minted period 0.
The **item-4 C1 re-run proved it live**: with the fix, Ade **forged 3 period-1 blocks** and **self-accepted
them**, and the real `cardano-node` **downloaded the period-1 header with no KES (or parse) rejection**
(pre-fix: `failed = 5 / succeeded = 0`, `KesPeriodNotCurrent`). **Both gating reviews PASS** (per-slice and
per-cluster) — **no HIGH+ this cluster**. The window is **RED-only**: **0 BLUE canonical-type change**
(458 unchanged), no `RO-LIVE` flip, no behavior change to the authoritative core.

## 0. Headline

| Count | Baseline (`c6e7fafe`) | HEAD (`1d54abb4`) | Δ |
|---|---|---|---|
| CI gates (`ci/ci_check_*.sh`) | 137 | **138** | **+1** — **one NEW gate**, `ci_check_kes_evolution_before_sign.sh` (S1, `DC-CRYPTO-10`; added — `--diff-filter=A`). **No gate modified, no gate removed** in `ci/` this span (`--diff-filter=M` and `--diff-filter=D` over `ci/` are both empty). |
| Registry rules (`docs/ade-invariant-registry.toml`) | 335 | **336** | **+1** — one NEW rule **`DC-CRYPTO-10`** (`tier = derived`, `introduced_in = "PHASE4-N-AC"`, `status = enforced`). **Zero removed** (`diff` of the sorted `id =` lists shows the single addition `DC-CRYPTO-10` and no removal). |
| Registry status (enforced / partial / declared) | 203 / 20 / 112 | **204 / 20 / 112** | **+1 enforced** — `DC-CRYPTO-10` lands `enforced` at the S1 close (declared at cluster scoping → enforced at the same close). |
| Registry strengthenings | — | **1** | `strengthened_in += "PHASE4-N-AC"` on exactly one rule: **`CN-KES-HEADER-01`** (the KES signature is over the canonical unsigned-header pre-image — N-AC adds that the signing key is now evolved to the block's KES period before that sign). A strengthening, **not** a new rule. |
| BLUE canonical types | 458 | **458** | **0** — **RED-only span.** No `ade_core` / `ade_codec` / `ade_types` / `ade_crypto` / `ade_plutus` / `ade_ledger` / `ade_network`-BLUE (`mux::frame` / `codec` / `handshake` / `chain_sync` / `block_fetch` / `tx_submission` / `keep_alive` / `peer_sharing` / `n2c`) source change. The two touched source files are `ade_runtime::producer::producer_shell` (RED shell, +137) and `ade_node::produce_mode` (binary/shell entry, +7 / −2). |
| Grounding docs | CODEMAP/SEAMS/TRACEABILITY refreshed for N-AB in `6184eb0b` (span-opening) | **CODEMAP/SEAMS/TRACEABILITY refreshed for the N-AC close** | The span-opening commit `6184eb0b` refreshed all four grounding docs for the **N-AB** close. This **N-AC** close refreshes CODEMAP/SEAMS/TRACEABILITY again to carry `DC-CRYPTO-10` (alongside this HEAD_DELTAS). The on-disk CODEMAP header carried the N-AB figures (458 types / 137 CI) at the instant of writing; the parallel refresh brings the CI count to 138 and folds in the `DC-CRYPTO-10 ↔ ci_check_kes_evolution_before_sign.sh` binding. The registry already records the rule + binding authoritatively at HEAD (336 rules). |

This is a **single-cluster lead** (PHASE4-N-AC) preceded by the prior-lead's N-AB close-refresh tail. The
slice↔rule↔gate map for the cluster:

| Slice | Rule | Gate | What shipped |
|---|---|---|---|
| **S1** (`7d4a4a72`) | **`DC-CRYPTO-10`** (NEW, enforced) | **`ci_check_kes_evolution_before_sign.sh`** (NEW) | New RED `ProducerShell::kes_sign_header_advancing` = `kes_advance_to(period)` then `kes_sign_header`; the forge's single real KES sign (`produce_mode.rs:891`) is rewired to it; period passed verbatim; fail-closed `EvolutionBackwards` (backwards) / `EvolutionExhausted` (beyond lifetime). Signing stays RED. |

The per-commit shape:

| Commit | Kind | What it did | Code / CI / registry effect |
|---|---|---|---|
| `6184eb0b` | docs (prior-lead tail) | Grounding-doc refresh for the PHASE4-N-AB close (CODEMAP/TRACEABILITY/SEAMS/HEAD_DELTAS) | **0 code / 0 CI**; touched the four grounding docs + `.idd-config.json` (baseline bump for the N-AB close); no rule added/removed |
| `cbe6633f` | docs (cluster doc) | PHASE4-N-AC cluster doc; **declare `DC-CRYPTO-10`** | **0 code / 0 CI**; registry: `DC-CRYPTO-10` added `declared` |
| `68a85dbe` | docs (slice doc) | S1 slice doc (evolve KES key before sign) | **0 code / 0 CI / 0 registry** |
| `7d4a4a72` | feat(producer) | S1 impl — `kes_sign_header_advancing` (evolve-then-sign) + rewire of the forge's single real KES sign; `kes_advance_to` fail-closed; 4 shell tests | **RED code** (`producer_shell.rs` +137, `produce_mode.rs` +7 / −2); **+1 CI** (`ci_check_kes_evolution_before_sign.sh`); registry: `DC-CRYPTO-10 → enforced` |
| `1d54abb4` | chore (close) | Close PHASE4-N-AC — archive cluster/slice docs; `strengthened_in += "PHASE4-N-AC"` on `CN-KES-HEADER-01`; C1 reproduction README genesis-window addendum | **0 code / 0 CI**; registry: 1 strengthening (no new rule); 2 doc moves to `docs/clusters/completed/PHASE4-N-AC/`; 1 evidence-README addendum |

## 1. Commit Log (newest first)

| Hash | Type | Summary |
|------|------|---------|
| `1d54abb4` | chore (close) | Close PHASE4-N-AC — KES signing evolves key to current period (DC-CRYPTO-10) |
| `7d4a4a72` | feat | evolve KES key to current period before signing (PHASE4-N-AC S1, DC-CRYPTO-10) |
| `68a85dbe` | docs | slice doc PHASE4-N-AC S1 evolve KES key before sign |
| `cbe6633f` | docs | cluster doc PHASE4-N-AC KES signing evolves key to current period + declare DC-CRYPTO-10 |
| `6184eb0b` | docs | grounding-doc refresh for PHASE4-N-AB close (CODEMAP/TRACEABILITY/SEAMS/HEAD_DELTAS) |

No merge commits in the span. **5 commits, zero unclassified** — one carries an explicit
conventional-commits prefix (`feat(producer):`), three are `docs:`, and the close commit `1d54abb4`
("Close PHASE4-N-AC …") is a `/cluster-close`-style record (its diff scope is exclusively `docs/` +
`docs/ade-invariant-registry.toml`, so it classifies `chore`/`docs`). The shape is **N-AB-close refresh →
declare → S1 → close**: the prior-lead grounding refresh (`6184eb0b`), the cluster doc declaring
`DC-CRYPTO-10` (`cbe6633f`), the single slice (S1 doc `68a85dbe`, impl `7d4a4a72`), and the close
(`1d54abb4`). The cluster work landed 2026-06-06 (the close at 11:08).

> **Note (commit-attribution policy).** Per this repo's `CLAUDE.md` override (vibe-coded-node bounty
> trailer requirement), commits in this repo carry a `Co-Authored-By:` model-attribution trailer; that
> is an Ade-local override of the global no-AI-attribution rule and applies to **commit messages
> only**. It does not affect this doc's content.

## 2. New Modules

**None.** `git diff --diff-filter=A --name-only c6e7fafe..1d54abb4 -- '*.rs'` shows **no new `.rs` source
file**, no new crate, no new `Cargo.toml`, no new workspace. The only added files this span are **one CI
gate** (`ci/ci_check_kes_evolution_before_sign.sh`, §5) and **two cluster/slice docs**
(`docs/clusters/completed/PHASE4-N-AC/{cluster.md, S1-evolve-kes-before-sign.md}`). The span is
**modification only** in code — the two touched source files are the **existing** RED producer-shell
module `ade_runtime::producer::producer_shell` and the **existing** binary entry `ade_node::produce_mode`.

> **Cross-reference (CODEMAP/SEAMS) — `DC-CRYPTO-10` carried by the parallel N-AC refresh.** This span adds
> **no new module**, but it adds **new RED surface to an existing module**: the new method
> `ProducerShell::kes_sign_header_advancing` (and its private `kes_advance_to` helper) inside
> `ade_runtime::producer::producer_shell`. The `producer` module and `ProducerShell` are already in
> CODEMAP. CODEMAP/SEAMS are **refreshed for this N-AC close** to fold in the evolve-then-sign surface
> and the `DC-CRYPTO-10` rule. The registry already records `DC-CRYPTO-10` authoritatively
> (`code_locus = crates/ade_runtime/src/producer/producer_shell.rs … ; crates/ade_node/src/produce_mode.rs`).

## 3. Modules Modified

Two modules changed this span (both RED-shell `.rs` files; **zero BLUE**):

| Module | Scope | Key changes |
|--------|-------|-------------|
| `ade_runtime::producer::producer_shell` (`crates/ade_runtime/src/producer/producer_shell.rs`) | +137 (RED shell) | **S1 (`7d4a4a72`).** Adds the new RED method **`ProducerShell::kes_sign_header_advancing(period, pre_image)`** = **`self.kes_advance_to(period)?` then `kes_sign_header(period, pre_image)`** — it evolves the operator KES signing key **forward** to the requested period via the **existing deterministic `Sum6KES` update** (`kes_advance_to → kes_update`, idempotent when `current == period`), then signs the canonical unsigned-header pre-image. **Fail-closed** on a non-reachable period: `period < current` → `ShellSignError::Signing(SigningError::EvolutionBackwards{…})`; `period > SUM6_MAX_PERIOD` (= 63) / beyond the key lifetime → `Signing(EvolutionExhausted{…})`. The period is **passed verbatim** (no `± N`). Adds **4 shell tests** (`tests::shell_kes_sign_header_advancing_*`): evolves-then-signs at a forward period; signs at the current period (idempotent); fails closed backwards (with **no** signature emitted — `kes_advance_to` replaces the key before `kes_update`, so a failed advance leaves no usable key); fails closed beyond lifetime. **No new BLUE type, no signature change to any BLUE surface, signing stays RED** (the standing `ci_check_no_signing_in_blue.sh` remains the BLUE fence). |
| `ade_node::produce_mode` (`crates/ade_node/src/produce_mode.rs`) | +7 / −2 (binary/shell entry) | **S1 (`7d4a4a72`).** Rewires the forge's **single real KES sign** site (`run_real_forge_inner`, the line previously `shell.kes_sign_header(kes_period, &preimage)`) to **`shell.kes_sign_header_advancing(kes_period, &preimage)`**, with a `// PHASE4-N-AC / DC-CRYPTO-10` comment recording *why* (the minted-at-period-0 key fails `KesPeriodNotCurrent` once the chain's KES period > 0). The error arm is unchanged (`ForgeFailed` on `Err`). This is the **only** real KES sign in the forge path. |

> **No BLUE-authority change (load-bearing).** This span touches **no BLUE source file** — the two code
> changes are in the **RED** producer shell (`ade_runtime::producer::producer_shell`) and the binary
> entry (`ade_node::produce_mode`); neither is in the BLUE `core_paths`. The KES evolution reuses the
> **existing deterministic `Sum6KES` update** (`kes_update`) — it introduces **no new KES algorithm, no
> new verifier, no new canonical type**. The BLUE canonical-type count is **458 → 458**. Forge
> eligibility (the VRF leader check) and the wire/serve rules are unchanged.

## 4. Feature Flags

**No project feature-flag deltas.** Ade declares no `[features]` table in any workspace `Cargo.toml`,
and **no `Cargo.toml` changed in this window** (`git diff --name-only c6e7fafe..1d54abb4 --
'**/Cargo.toml' 'Cargo.toml'` is empty). No `#[cfg(feature = …)]` gate was introduced. The KES-period
bound is governed by the **fixed** `SUM6_MAX_PERIOD = 63` `Sum6KES` ceiling (not a feature flag, CLI flag,
env var, or config knob); the new gate `ci_check_kes_evolution_before_sign.sh` fences that the shell
passes the period **verbatim** and retains its fail-closed `EvolutionBackwards` / `EvolutionExhausted`
guards.

## 5. CI Checks (137 → 138; +1 new, 0 modified, 0 removed)

One new gate this span; no gate modified, no gate removed. `git diff --diff-filter=A c6e7fafe..1d54abb4
-- ci/` lists exactly the one gate below; `--diff-filter=M` and `--diff-filter=D` over `ci/` are both
**empty**.

### PHASE4-N-AC KES-evolution gate (`7d4a4a72`)

| Check | Status | Origin / change | What it checks |
|-------|--------|-----------------|----------------|
| `ci_check_kes_evolution_before_sign.sh` | **New** | PHASE4-N-AC S1 (`7d4a4a72`); `DC-CRYPTO-10` | The forge's real KES sign must **evolve** the operator KES key to the requested period **before** signing, via `kes_sign_header_advancing → kes_advance_to → kes_update` (the deterministic `Sum6KES` update), passing the period **verbatim**; and `kes_update` must keep its fail-closed backwards + exhausted guards. Three fences (over a comment-stripped, pre-`#[cfg(test)]` production view of `produce_mode.rs` + `producer_shell.rs` + `signing.rs`): **(a)** the forge real KES sign uses `kes_sign_header_advancing`, **not** the raw `kes_sign_header` / `kes_sign_at`; **(b)** `kes_sign_header_advancing` evolves (`kes_advance_to(period)`) before signing, with the period passed verbatim (no `period ± N`); **(c)** `kes_update` retains the `EvolutionBackwards` + `EvolutionExhausted` guards. Signing stays RED (the standing `ci_check_no_signing_in_blue.sh` is the BLUE fence). |

> **Cross-reference (TRACEABILITY) — `DC-CRYPTO-10 ↔ ci_check_kes_evolution_before_sign.sh` carried by the
> N-AC refresh.** TRACEABILITY is **refreshed for this N-AC close** (alongside this HEAD_DELTAS) to add
> the `DC-CRYPTO-10 ↔ ci_check_kes_evolution_before_sign.sh` row. The registry already records the binding
> at HEAD (`DC-CRYPTO-10.ci_script = "ci/ci_check_kes_evolution_before_sign.sh"`), so the rule↔gate link
> is **authoritative in the registry**; the TRACEABILITY doc is the view being brought current alongside
> this regen. **No rule↔gate binding was removed.** The new gate enforces a named, enforced invariant
> (`DC-CRYPTO-10`), so it is **not** an orphan gate.

## 6. Canonical Type Registry Delta

**n/a — no separate canonical-type registry is configured** (`canonical_type_registry: null`);
canonical-type rules live inline in the invariant registry under family **T**. **No canonical type was
added or removed in this window** — this is a **RED-only** span (BLUE count unchanged, **458 → 458**, per
the CODEMAP header). The KES evolution reuses the existing `Sum6KES` update primitive; it adds **no BLUE
canonical type**. No `Cargo.toml` changed.

## 7. Normative / Invariant Rule Delta (335 → 336; +1 enforced rule, 1 strengthening, zero removals)

**One rule ID was added; zero removed** (335 → 336; `diff` of the sorted `id =` lists shows the single
addition `DC-CRYPTO-10` and no removal). The status tally moves **203 → 204 enforced** (20 partial /
112 declared unchanged) — the new rule lands `enforced` at the S1 close.

*(The configured `normative_docs` — the CE-79 tier-gate statement + addendum, the three contract docs,
and `CLAUDE.md` — were **not** changed this span: `git diff --name-only c6e7fafe..1d54abb4` over those
paths is empty. The rule-count delta is entirely the invariant-registry change below.)*

**New rule (`+1`, enforced):**

| Rule | Family / Tier | Statement (summary) |
|------|---------------|---------------------|
| `DC-CRYPTO-10` | DC / `derived` (`enforced`; `introduced_in = "PHASE4-N-AC"`) | **The RED signing shell evolves the operator KES key to the requested period before signing.** Using the existing deterministic `Sum6KES` update primitive, the shell advances the key forward to the requested KES period, then signs; it **fails closed** if the requested period is **before** the key start, **beyond** the key lifetime, or **cannot be reached** by sequential evolution. `ci_script = ci/ci_check_kes_evolution_before_sign.sh`; `cross_ref = [CN-KES-HEADER-01, T-KEY-01, DC-CRYPTO-04, DC-CRYPTO-09, CN-FORGE-03]`. Lets the forge sign across KES periods (not just the minted period 0); the item-4 C1 re-run proved it live (Ade forged 3 period-1 blocks; the real cardano-node downloaded the period-1 header with no KES rejection). |

**Strengthening (`strengthened_in += "PHASE4-N-AC"`) — exactly one, no rule weakened:**

| Rule | Family / Tier | Strengthening |
|------|---------------|---------------|
| `CN-KES-HEADER-01` | CN / `derived` (`enforced`, unchanged) | **KES signature over the canonical unsigned-header pre-image — now signed with the period-evolved key.** CN-KES-HEADER-01 (PHASE4-N-S-A) fixes that the header KES signature is over the branded `UnsignedHeaderPreImage` and that arbitrary-byte signing is unrepresentable. N-AC strengthens the producer side: the **single** `kes_sign_header` call in the forge is now reached **only** via `kes_sign_header_advancing`, which evolves the key to the block's KES period first (`DC-CRYPTO-10`). The pre-image / single-source-of-truth contract is preserved **and** extended — the signing key is now correct for the block's period across period boundaries. |

> **Both gating reviews PASS — no HIGH+ this cluster (load-bearing).** The PHASE4-N-AC per-slice review
> (S1) and the per-cluster cross-slice review **both PASS with no HIGH (or higher) finding.** The
> evolve-then-sign design reuses the existing deterministic `Sum6KES` update, passes the period verbatim,
> and fails closed on a non-reachable period; the live C1 re-run (KES period 1) is the end-to-end
> evidence. Two **pre-existing fail-closed INFO items** were recorded honestly and handed to C2 (neither
> reachable via the forge today): **(1)** `kes_advance_to` zeroes the key on a *failed* advance
> (`std::mem::replace` before `kes_update`) — unreachable via the forge (the `kes_period_in_window` /
> `kes_period_for_slot` bounds prevent it) and **fail-safe** (a zeroed key signs nothing a peer accepts);
> **(2)** the opcert window upper bound `opcert_start + 63` can diverge from the absolute `Sum6KES`
> ceiling `63` when `opcert_start > 0` (real preprod opcerts) — still fail-closed (`EvolutionExhausted`),
> but the C2 config must derive `kes_max_period` from the **absolute** ceiling, not `opcert_start + 63`.

**No rule was removed (expected: 0).** The registry delta is **one new enforced rule + one
`strengthened_in` append** — purely additive / strengthening, consistent with append-only registry
discipline.

## Working tree at HEAD `1d54abb4`

Clean of tracked changes from this span — the cluster + close are all committed. `git status --short`
shows only an untracked `.mithril-scratch/` (operator scratch, ignored). **This regen runs *after* all
five span commits** (the close commit `1d54abb4` is HEAD for this window); the CODEMAP/SEAMS/TRACEABILITY
refresh + the baseline bump (`c6e7fafe → 1d54abb4`) are the close-pass follow-on actions.

## Honest residual (window scope)

PHASE4-N-AC **makes the forge sign correctly across KES periods** — and that is the entire claim. The
honest boundary:

- **Live-readiness fix, NOT a capability flip.** N-AC closes a latent forge gap (the minted-at-period-0
  KES key never evolved forward). **No `RO-LIVE` rule was flipped** — `RO-LIVE-01` stays operator-gated.
  No authoritative behavior changed; the span is **RED-only** (0 BLUE change, 458 canonical types
  unchanged). It makes the producer signing shell live-correct across KES-period boundaries; it does not
  by itself complete the bounty.
- **RED shell, single sign site, reuses the deterministic `Sum6KES` update.** The change lives entirely
  in the RED producer shell + the binary entry, **reuses the existing `kes_update` primitive** (no new
  KES algorithm / verifier), and the forge has exactly **one** real KES sign — now reached only via the
  evolving variant (gate-enforced). Signing stays RED; the BLUE `Sum6KES` algorithm, the KES verifier,
  forge eligibility, and the wire rules are unchanged. The period is passed **verbatim** and the shell
  **fails closed** on any non-reachable period.
- **Live proof (non-promotable rehearsal).** The item-4 C1 re-run (HEAD `7d4a4a72`, private net at slot
  ≈ 251740, KES period 1) confirmed the fix: Ade **forged 3 period-1 blocks** and self-accepted them, and
  the real `cardano-node` **downloaded the period-1 header with no KES/parse rejection** (pre-fix:
  `failed = 5 / succeeded = 0`, `KesPeriodNotCurrent`). This is the **C1 genesis-rehearsal harness**
  (acceptance #3) — a non-promotable rehearsal, **not** a bounty/preview/preprod completion claim.
- **Genesis-window finding (structural, recorded honestly).** On this net `slotsPerKESPeriod = 129600`
  **equals** the Cardano genesis density window `3k/f = 129600` (`k = 2160`, `f = 0.05`), so KES period 1
  begins **exactly** when the genesis window closes. The two halves of "forge at KES period 1 **and** the
  follower adopts" are therefore **mutually exclusive on a from-genesis net**: in period 0 (slots
  < 129600) the genesis window is open so adoption works, but the key is at its minted period 0 so **no
  KES evolution is exercised**; in period 1+ (slots ≥ 129600) KES evolution **is** exercised, but the
  follower rejects with **`CandidateTooSparse`** — a **genesis-density-window** rejection, **KES-
  independent**, not a KES rejection. The **cross-period end-to-end forge → adopt** path is proven on the
  **C2 tip path** (a dense current tip, no genesis window), **not** by resetting the net (which returns to
  period 0 and re-proves only the narrow period-0 case). The C1 reproduction README carries this addendum
  (close commit `1d54abb4`).
- **CODEMAP/SEAMS/TRACEABILITY refreshed for this close.** This N-AC close regenerates all four grounding
  docs to carry `DC-CRYPTO-10` (the evolve-then-sign surface, the `CN-KES-HEADER-01` strengthening, and
  the `DC-CRYPTO-10 ↔ ci_check_kes_evolution_before_sign.sh` binding). The registry records the rule + its
  gate binding authoritatively at HEAD (336 rules).

---

## Historical — PHASE4-N-AB close + cluster window (`b0365df0 → c6e7fafe`)

> The section below is the **previous** HEAD_DELTAS lead, preserved in condensed form. It was a
> **grounding-doc refresh + the PHASE4-N-AB cluster**, narrating the `b0365df0 → c6e7fafe` span. Counts
> in this Historical section are the figures **at `c6e7fafe`** (335 rules, 137 CI gates, 458 canonical
> types); the current window measures **forward** from `c6e7fafe`. The full §§0–7 narrative is recoverable
> from this doc's git history at `c6e7fafe`.

> Baseline: `b0365df0` (Close PHASE4-N-AA — bounded peer-driven serve range (DC-SERVEMEM-01), 2026-06-06 01:43)
> HEAD: `c6e7fafe` (Close PHASE4-N-AB — outbound mux segmentation (CN-SESS-05), 2026-06-06 03:48)
> Span: **a grounding-doc refresh + the PHASE4-N-AB cluster** — 5 commits, 10 files, +1130 / −406.

PHASE4-N-AB was **pre-RO-LIVE hardening item 2** and closed a **receive/send asymmetry** that
PHASE4-N-M-FRAG left half-open: Ade could *receive* a block fragmented across multiple mux frames
(CN-SESS-04 inbound reassembly), but it could **not transmit one** — `handle_outbound` (and the
single-frame encoder `encode_inner_frame`) **errored `OutboundPayloadTooLarge`** for any payload above
`MAX_PAYLOAD = 65535` bytes. N-AB closed that in one slice:

- **S1 — outbound mux segmentation (`02e6e557`; `CN-SESS-05 → enforced`).** The **GREEN** session
  reducer's `handle_outbound` (`crates/ade_network/src/session/core.rs`) now **segments** a payload in
  the range `MAX_PAYLOAD < len <= MAX_OUTBOUND_PAYLOAD_BYTES` into **ordered `<= MAX_PAYLOAD` mux frames**
  (`payload.chunks(MAX_PAYLOAD)`, each encoded via the **single `encode_inner_frame` authority**) and
  **fails closed above** a new **fixed, non-configurable** const `MAX_OUTBOUND_PAYLOAD_BYTES = 16 MiB`
  (symmetric with the inbound `MAX_REASSEMBLY_TAIL_BYTES` / DC-LIVEMEM-01). Every segment carries the
  **same** mini-protocol id + mode + the **same captured `timestamp`** (GREEN — no per-segment clock);
  concatenating the segment payloads reconstructs the original byte-for-byte. New gate
  `ci_check_outbound_segmentation.sh`.

**N-AB headline (at `c6e7fafe`):** Registry **334 → 335** (+1 enforced `CN-SESS-05`; +2 strengthenings
`CN-SESS-04` + `DC-SERVEMEM-01`; 0 removed). CI gates **136 → 137** (+1 `ci_check_outbound_segmentation.sh`).
**GREEN-only — BLUE canonical types 458 → 458.** Outbound inverse of CN-SESS-04 inbound reassembly; closes
the receive/send wire-layer asymmetry. **No `RO-LIVE` flip.**

---

## Historical — PHASE4-N-AA close + cluster window (`999199f8 → b0365df0`)

> The section below is the **PHASE4-N-AA** lead, preserved in condensed form. It was a **focused grounding
> refresh + the PHASE4-N-AA cluster**, narrating the `999199f8 → b0365df0` span. Counts here are the
> figures **at `b0365df0`** (334 rules, 136 CI gates, 458 canonical types). The full §§0–7 narrative is
> recoverable from this doc's git history at `b0365df0`.

> Baseline: `999199f8` (repair 10 pre-existing gate-vs-code drifts (gate hygiene), 2026-06-05 19:28)
> HEAD: `b0365df0` (Close PHASE4-N-AA — bounded peer-driven serve range (DC-SERVEMEM-01), 2026-06-06 01:43)
> Span: **a focused grounding refresh + the PHASE4-N-AA cluster** — 8 commits, 15 files, +1254 / −492.

PHASE4-N-AA was **pre-RO-LIVE hardening item 1** and closed the **MEDIUM** finding the PHASE4-N-U
cross-slice security review left open: *the `--mode node` serve path could be driven by a peer into
unbounded memory + O(N²) CPU work.* N-AA closed that across two slices + an in-cluster security fix:

- **S1 — bounded hash-free ChainDb read primitives (`6b8f1779`; CE-1).** Two new **bounded,
  slot-ordered, hash-free** `ChainDb` trait primitives: `range_bytes_capped(from, to, max)` and
  `last_block_bytes()`; new **RED type** `CappedSlotRange { blocks, truncated }`; the unbounded
  `iter_from_slot` / `tip` doc-fenced as TRUSTED-CALLER reads (internals unchanged).
- **S2 — serve projection cap + fail-closed (`3d853ec0`; `DC-SERVEMEM-01 → enforced`).**
  `ChainDbServedSource` switched onto the S1 bounded primitives behind a fixed `const
  MAX_SERVE_RANGE_BLOCKS = 256`; new **RED enum** `ServeRangeOutcome { Served | Empty | CapExceeded |
  ReadError }` — every non-`Served` maps to wire `NoBlocks`. New gate `ci_check_serve_range_bounded.sh`.
- **In-cluster security-review MEDIUM (`5c9f6cf6`).** An inverted-range (`from > to`) panic was found +
  fixed in-cluster (a `from > to → empty` guard on both `ChainDb` impls + tests).

**N-AA headline (at `b0365df0`):** Registry **333 → 334** (+1 enforced `DC-SERVEMEM-01`; +2 strengthenings
`DC-NODE-13` + `DC-LIVEMEM-01`; 0 removed). CI gates **135 → 136** (+1 `ci_check_serve_range_bounded.sh`).
**RED-only — BLUE canonical types 458 → 458.** Serve-side analog of `DC-LIVEMEM-01`; closes the N-U
cross-slice MEDIUM. **No `RO-LIVE` flip.**

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
gate in place** — adding no gate, removing no gate, weakening no invariant.

**N-U-close-window headline (at `999199f8`):** CI gates **135 → 135** (0 net — 11 gates repaired in
place); registry **333 → 333** (identical ID set — the lone edit was the `DC-NODE-06` strengthening);
status **201 / 20 / 112** unchanged; BLUE types **458 → 458**. The full `ci/ci_check_*.sh` sweep was
**135 passed / 0 failed** at HEAD. **No `RO-LIVE` flip, no behavior change** — pure
enforcement-trustworthiness work.

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
blocks use, with NO second tip-advance path?* It closed that across three slices:

- **S1 — own-forged durable admit through the pump (`DC-NODE-12` + `DC-CONS-23` + `DC-WAL-04` prior-fp
  clause).** A fenced RED driver `ade_node::node_sync::admit_forged_block_durably` feeds the
  self-accepted bytes into the **same** `forward_sync::pump_block` chokepoint received blocks use. New
  gate `ci_check_forged_durable_admit_via_pump.sh`.
- **S2 — forged-tip crash recovery + replay-equivalence (`T-REC-05`, `DC-WAL-04` no-orphan clause).**
  Production `warm_start_recovery` forward-replays from the nearest snapshot ≤ tip and reconciles the
  WAL tail; an un-WAL'd forged orphan is dropped. `T-REC-05` is test-enforced.
- **S3 — serve-as-durable-chain projection (`DC-NODE-13`; strengthens `CN-CONS-07`, `DC-NODE-11`).** The
  `--mode node` served view became a deterministic read-only projection of the durable ChainDb (the NEW
  RED module `ade_runtime::network::served_chain_projection`). New gate
  `ci_check_served_chain_projection.sh`; retired gate `ci_check_served_chain_stability.sh`.

**N-U headline (at `4e358e92`):** Registry **328 → 333** (+5 enforced: `DC-NODE-12`, `DC-CONS-23`,
`DC-WAL-04`, `T-REC-05`, `DC-NODE-13`; +2 strengthenings: `CN-CONS-07`, `DC-NODE-11`; 0 removed). CI
gates **134 → 135** (+1 net: +2 new, −1 retired). **One new RED module** (`served_chain_projection`).
**BLUE canonical types 458 → 458.** **No `RO-LIVE` flip.**

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

### Regen `c6e7fafe → 1d54abb4` (PHASE4-N-AC — KES signing evolves key to current period — current lead)

- **Baseline valid; single-cluster lead (RED-only) preceded by the prior-lead's N-AB close-refresh
  tail.** Run against the config baseline `c6e7fafe` (the PHASE4-N-AB close HEAD), which `git rev-parse`
  resolves and `git merge-base c6e7fafe HEAD` confirms is a strict ancestor of HEAD `1d54abb4`
  (`c6e7fafe` carries no tag). The span is the **grounding-doc refresh for the N-AB close** `6184eb0b`
  (the prior lead's tail — CODEMAP/TRACEABILITY/SEAMS/HEAD_DELTAS refresh) **plus the PHASE4-N-AC cluster**
  (4 commits: cluster doc + S1 doc + S1 impl + close). The closer bumps `head_deltas_baseline`
  `c6e7fafe → 1d54abb4` after this regen.
- **Counts are mechanical (git/grep/ls):** commit log + `--shortstat` over `c6e7fafe..1d54abb4`
  (**5** commits, no merges / **12** files / **+1029 / −340**); CI gate count via
  `git ls-tree -r --name-only <ref> ci/ | grep -c 'ci_check_.*\.sh$'` at each ref (**137 → 138**;
  `--diff-filter=A` over `ci/` = exactly `ci_check_kes_evolution_before_sign.sh`; `--diff-filter=M` and
  `--diff-filter=D` over `ci/` both **empty**); registry rule count via `grep -cE '^\[\[rules\]\]'` at
  each ref (**335 → 336**; `diff` of sorted `id =` lists shows the single addition `DC-CRYPTO-10`, zero
  removals); registry status via `grep -E '^status = ' | sort | uniq -c` (**203 → 204 enforced**, 20
  partial / 112 declared unchanged); strengthening via the registry diff (**1**: `CN-KES-HEADER-01`
  gained `strengthened_in = ["PHASE4-N-AC"]`); BLUE canonical types via the CODEMAP header (**458 → 458**).
- **RED-only span — no BLUE touch, +0 canonical type, no Cargo.toml change.** `git diff --name-status
  c6e7fafe..1d54abb4` shows **no new `.rs` source file** (only `A` for one CI gate + two cluster/slice
  docs, `M` for the two `.rs` files + the registry + grounding docs, `D`/`A` for the two cluster-doc
  archive moves, `M` for the C1 README). The two touched `.rs` files are
  `crates/ade_runtime/src/producer/producer_shell.rs` (RED shell) and
  `crates/ade_node/src/produce_mode.rs` (binary/shell entry) — **neither in the BLUE `core_paths`**. `git
  diff --name-only … '**/Cargo.toml' 'Cargo.toml'` is empty (no feature-flag delta).
- **Registry delta is +1 enforced rule + 1 strengthening, NOT a removal.** `DC-CRYPTO-10` is the new rule
  (declared at the cluster doc `cbe6633f`, enforced at the S1 impl `7d4a4a72`); `CN-KES-HEADER-01` gained
  `strengthened_in += "PHASE4-N-AC"` at the close (`1d54abb4`). The sorted-id `diff` confirms zero
  removals.
- **Normative docs unchanged this span.** `git diff --name-only c6e7fafe..1d54abb4` over the configured
  `normative_docs` (CE-79 statement + addendum, the three contract docs, `CLAUDE.md`) is empty — the §7
  delta is entirely the invariant-registry change.
- **Doc-refresh — all four grounding docs regenerated for this close.** The span-opening `6184eb0b`
  refreshed the four docs for the **N-AB** close. This **N-AC** close regenerates them again: CODEMAP +
  SEAMS + TRACEABILITY are refreshed (alongside this HEAD_DELTAS) to carry `DC-CRYPTO-10` (the
  evolve-then-sign surface, the `CN-KES-HEADER-01` strengthening, and the `DC-CRYPTO-10 ↔
  ci_check_kes_evolution_before_sign.sh` binding). The registry already records the rule + gate binding
  authoritatively at HEAD (336 rules); the CODEMAP/SEAMS/TRACEABILITY refresh brings the narrative docs
  current alongside this regen.
- **Both gating reviews PASS — no HIGH+ this cluster.** The PHASE4-N-AC per-slice (S1) and per-cluster
  cross-slice security reviews both pass with no HIGH+ finding. Two pre-existing fail-closed INFO items
  (the `kes_advance_to` zero-on-failed-advance — unreachable via the forge, fail-safe; and the
  `opcert_start + 63` vs absolute-`63` window divergence — still fail-closed) were recorded and handed to
  C2. The live evidence is the item-4 C1 re-run (KES period 1): Ade forged 3 period-1 blocks; the real
  cardano-node downloaded the period-1 header with no KES rejection.
- **Genesis-window finding recorded honestly.** `slotsPerKESPeriod = 129600 == 3k/f = 129600`, so a
  from-genesis rehearsal cannot show forge-at-period-1 **and** follower-adopt simultaneously (the period-1
  follower rejection is `CandidateTooSparse` — a genesis-density-window limit, **KES-independent**).
  Cross-period end-to-end forge → adopt is the **C2 tip path** (dense current tip, no genesis window), not
  a net reset. The C1 reproduction README carries this addendum (close commit `1d54abb4`).
- **Working tree clean.** This regen runs *after* all five span commits (the close `1d54abb4` is HEAD for
  this window); `git status --short` shows only an untracked `.mithril-scratch/` (operator scratch,
  ignored). The remaining close-pass actions are the parallel CODEMAP/SEAMS/TRACEABILITY refresh + the
  baseline bump `c6e7fafe → 1d54abb4`.
