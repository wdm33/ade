# PHASE4-N-F-F — Slice S1: Forge intent classification

> **Status:** slice doc (IDD Part IV). Companion to
> `../../planning/phase4-n-f-f-cluster-slice-plan.md` (S1 row) and
> `../../planning/phase4-n-f-f-invariants.md`. Code-verified against HEAD
> `b2a2df6` at authoring.

> **Slice S1 in one line:** a pure GREEN tri-state classifier
> `classify_forge_intent` over the *presence* of the five operator-key CLI flags
> — complete set ⇒ `On(paths)`, none ⇒ `Off`, any partial subset ⇒ structured
> `PartialKeySet` error — so a partial key set can never reach a forge. Lands
> **tested-but-unwired** (S3 wires it into the `--mode node` arm).

## 1. Slice identity
- **Cluster:** PHASE4-N-F-F (operator-key ingress → forge-on flip).
- **Slice:** S1 — Forge intent classification.
- **Module (new):** `crates/ade_node/src/forge_intent.rs` (GREEN-by-content,
  `//! GREEN` banner) — sibling to the GREEN `run_loop_planner`.
- **Lands tested-but-unwired** — nothing consumes `ForgeIntent` until S3.

## 2. Cluster Exit Criteria addressed (verbatim)
- **CE-F-1** — Forge intent is a pure, total tri-state function of CLI key-flag
  presence: complete required set ⇒ `On`; none ⇒ `Off`; any partial subset ⇒
  structured fail-closed error. A partial key set can never produce a forge
  (mechanically: no path from `PartialKeySet` to `Some(activation)`).

(CE-F-2..F-6 are out of S1 scope — S2/S3/S4 + the close-diff gate check.)

## 3. Intent (invariant impact)
Lands the **intent-classification half of `CN-NODE-03`**: the decision "may this
node forge?" becomes a pure, total, deterministic function of which operator-key
flags are present, with the partial-key case **structurally fail-closed**.
Because `ForgeIntent` is a closed two-variant sum (`On`/`Off`) and the only other
outcome is an `Err`, the state "forge with a partial / missing / fabricated key
set" is unrepresentable as a classifier result. No file is read and no key byte
is touched here — this slice decides *intent from presence*, not *material from
bytes*.

## 4. Pre-conditions
- N-F-F invariants + plan committed (`a3eee84`, `b2a2df6`); `CN-NODE-03` is
  `declared`.
- The `Cli` struct already carries `cold_skey`, `kes_skey`, `vrf_skey`, `opcert`,
  `genesis_file` as `Option<PathBuf>` (`crates/ade_node/src/cli.rs`) — the
  presence inputs this classifier reads.
- No new dependency beyond `std::path::{Path, PathBuf}`.

## 5. Implementation boundary
- **New closed types** (`forge_intent.rs`, no `#[non_exhaustive]`):
  - `struct ForgePaths { cold: PathBuf, kes: PathBuf, vrf: PathBuf, opcert:
    PathBuf, genesis: PathBuf }` — the presence-validated complete path set
    (paths only; no secrets, no file contents).
  - `enum ForgeIntent { On(ForgePaths), Off }` — closed two-variant.
  - `enum ForgeIntentError { PartialKeySet { present: Vec<&'static str>, missing:
    Vec<&'static str> } }` — closed; carries only static flag-name strings, never
    path bytes or key material.
- **New pure fn:** `pub fn classify_forge_intent(cold: Option<&Path>, kes:
  Option<&Path>, vrf: Option<&Path>, opcert: Option<&Path>, genesis:
  Option<&Path>) -> Result<ForgeIntent, ForgeIntentError>`. Pure: no `&self`, no
  I/O, no clock, no `await`, no allocation beyond the error's flag-name vecs.
- **Decision rule (total over all 2⁵ = 32 presence combinations):**
  - all five `Some` ⇒ `Ok(ForgeIntent::On(ForgePaths{…}))`
  - all five `None` ⇒ `Ok(ForgeIntent::Off)`
  - any other combination (1–4 present) ⇒ `Err(ForgeIntentError::PartialKeySet{
    present, missing})`
- **`ForgePaths` may carry path values only after the complete-set condition has
  already been proven** — path ownership is a *result* of classification, not
  evidence of key validity. (No path is materialized into `ForgePaths` on the
  `Off` or `PartialKeySet` outcomes.)
- **Required set is exactly these five flags.** `pool_id` is **not** a CLI flag
  and **not** part of intent — it is derived from the cold key in one named place
  in S3; it never appears here.
- **No wiring** into `cli.rs` parsing, `node_lifecycle`, `run_relay_loop`, or the
  binary path (that is S3).

## 6. TCB color
- **GREEN:** `ade_node::forge_intent` (new — `ForgePaths`, `ForgeIntent`,
  `ForgeIntentError`, `classify_forge_intent`; `//! GREEN` banner; pure,
  content-blind over path values, secret-free). No BLUE change; no RED behavior;
  no `await`/I/O/clock/secret.
- **RED:** none.
- **BLUE:** none.

## 7. Invariants preserved (must not weaken) — by registry ID
- **CN-NODE-02** — the relay-loop owner / GREEN planner are untouched; S1 adds no
  loop step, no `run_relay_loop` change, no authority path.
- **DC-NODE-05 / CE-F-6** — the N-F-E forge-containment gate is untouched; S1 adds
  no forge path, no `forge_one_from_recovered` reference, no serve/admit/gossip/
  tip token.
- **CN-PROD-02 (T-tier key custody)** — S1 references no `KesSecret`/
  `VrfSigningKey`/`ColdSigningKey`; it sees only `Option<&Path>` presence.
- **T-CORE-01/02, T-DET-01** — the new module is pure, deterministic, free of
  clock/rand/float/`HashMap`.
- All BLUE invariants — no BLUE crate referenced.

## 8. Invariants strengthened (one family: CN-NODE-03)
- **CN-NODE-03** (`declared`) — this slice lands its **intent-classification
  half**: forge intent is a pure total tri-state of CLI key-flag presence, with
  the partial case fail-closed and partial-key forge unrepresentable. (Flips
  `declared → enforced` only at cluster close, when CE-F-1..F-6 are all green; S1
  contributes **CE-F-1**.) No `strengthened_in` bumps on any rule this slice
  (those land at close).

## 9. Replay / determinism obligations
- `classify_forge_intent` is a total pure function: identical flag-presence
  inputs ⇒ identical result. No corpus entry; proven by unit tests. No
  authoritative state, no canonical type, no WAL/checkpoint impact. This is the
  determinism precondition the S3 `Some`/`None` gating and the S4 replay proof
  build on.

## 10. Mechanical acceptance criteria
- [ ] `forge_intent.rs` exists with the `//! GREEN` banner; `ForgePaths` /
      `ForgeIntent` / `ForgeIntentError` are closed (no `#[non_exhaustive]`);
      `classify_forge_intent` defined.
- [ ] Test `classify_forge_intent_all_present_is_on` — all five `Some` ⇒
      `On(ForgePaths)` carrying the exact five paths.
- [ ] Test `classify_forge_intent_none_present_is_off` — all five `None` ⇒ `Off`.
- [ ] Test `classify_forge_intent_total_over_all_32_flag_combinations` — iterate
      all 2⁵ presence combinations; assert exactly the all-present ⇒ `On` and
      all-absent ⇒ `Off`, and the other **30** ⇒ `Err(PartialKeySet)` (totality +
      the load-bearing "partial never maps to `On`" property).
- [ ] Test `classify_forge_intent_partial_lists_present_and_missing_flags` — a
      representative partial set yields the correct `present`/`missing`
      static-name vecs.
- [ ] Test `forge_intent_error_carries_no_path_bytes` — `ForgeIntentError`
      Debug/Display contains only static flag names, never a supplied path string.
- [ ] Test `classify_forge_intent_is_deterministic` — repeated calls with
      identical inputs return identical results.
- [ ] **New CI gate** `ci/ci_check_forge_intent_closed.sh` (exit 0):
      `forge_intent.rs` carries `//! GREEN`; `ForgeIntent`/`ForgeIntentError`
      closed (no `#[non_exhaustive]`); module names none of `std::fs`, `tokio`,
      `await`, `SystemTime`, `Instant`, `HashMap`, `KesSecret`, `VrfSigningKey`,
      `ColdSigningKey`, `read(`/`File`; `classify_forge_intent` defined; **no
      wildcard arm in the classification decision that can collapse unenumerated
      presence combinations into `On` or `Off`**. **Additive only — touches no
      existing gate** (CE-F-6 preserved).
- [ ] `cargo build -p ade_node` clean; `cargo test -p ade_node forge_intent`
      green (count > 0); `rustfmt --edition 2021 crates/ade_node/src/forge_intent.rs`;
      the new gate passes.

## 11. Forbidden in this slice (inherits the cluster Forbidden list)
- **No relaxation of the N-F-E forge-containment gate**
  (`ci_check_node_run_loop_containment.sh`) — S1 must not touch it (hard rule,
  CE-F-6).
- No file reads, no key parsing, no `KesSecret`/`VrfSigningKey`/`ColdSigningKey`,
  no `ProducerShell` (that is S2).
- No wiring into `cli.rs` parse, `node_lifecycle`, `run_relay_loop`, or the binary
  path (that is S3).
- No clock/rand/float/`HashMap`, no `await`/I/O, no `unsafe`.
- No `#[non_exhaustive]` on any new type; **no wildcard arm in the classification
  decision that can collapse unenumerated presence combinations into `On` or
  `Off`** (a `_` used elsewhere — test helpers, formatting — is fine; the
  constraint is the classifier decision surface).
- No new canonical type / WAL entry / checkpoint / BLUE change; no `pool_id`
  fabrication or derivation here.
- No `--forge` boolean (presence of the complete set is the switch, per OQ2).

## 12. Slice completion checklist
- [ ] `forge_intent.rs` added (types + `classify_forge_intent` + tests);
      registered in the `ade_node` `lib.rs`/`main.rs` module tree (declaration
      only — no caller).
- [ ] `ci/ci_check_forge_intent_closed.sh` added, executable, exits 0; no existing
      gate modified.
- [ ] `cargo build/test -p ade_node` green; `rustfmt` applied; the new gate passes.
- [ ] Slice doc committed standalone (`docs:`) before implementation; impl
      committed (`feat:`) after green.

## Authority
Registry IDs `CN-NODE-03` (intent-classification half; `declared`), `CN-NODE-02`
/ `DC-NODE-05` / `CN-PROD-02` (preserved). The committed cluster-slice-plan and
`docs/ade-invariant-registry.toml` are authoritative; this slice doc refines, it
does not override. No `cluster.md` was generated for N-F-F (the cluster-slice-plan
is the CE source).
