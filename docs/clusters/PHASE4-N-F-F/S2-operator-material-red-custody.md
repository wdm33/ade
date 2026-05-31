# PHASE4-N-F-F — Slice S2: Operator material loading into RED custody

> **Status:** slice doc (IDD Part IV). Companion to
> `../../planning/phase4-n-f-f-cluster-slice-plan.md` (S2 row). Code-verified
> against HEAD `3c4bcca` (S1 merged) at authoring.

> **Slice S2 in one line:** a RED `ade_node::operator_forge` ingress site that
> consumes the S1 `ForgePaths` and builds a `ProducerShell` by **reusing** the
> existing cold/VRF/KES/opcert loaders (no reimpl, no duplication), with key
> custody RED-confined and a CI gate proving the module leaks no private key
> bytes. Lands tested-but-unwired (S3 assembles the `ForgeActivation` and wires
> the binary).

## 1. Slice identity
- **Cluster:** PHASE4-N-F-F (operator-key ingress → forge-on flip).
- **Slice:** S2 — Operator material loading into RED custody.
- **Module (new):** `crates/ade_node/src/operator_forge.rs` (RED, `//! RED`
  banner) — the single named node-path operator-material ingress site.
- **Reuses (pub(crate)):** `produce_mode::load_kes_skey_any_format` +
  `produce_mode::parse_simple_opcert_json` (visibility widened, behavior
  unchanged); `ade_runtime::producer::keys::{load_cold_signing_key_skey,
  load_vrf_signing_key_skey}` (already pub); `ProducerShell::init`.
- **Lands tested-but-unwired** — nothing in the binary path calls
  `load_operator_producer_shell` until S3.

## 2. Cluster Exit Criteria addressed (verbatim)
- **CE-F-2** — Operator material loads only through the existing RED loaders into
  `ProducerShell`; no private key bytes enter the GREEN coordinator/planner,
  node/loop state, WAL, log, snapshot, or evidence surface; tests/debug never
  print/serialize/hash-for-evidence/compare private key bytes (CI gate).
- contributes to **CE-F-6** — the new gate is additive only; the N-F-E
  forge-containment gate is untouched.

(CE-F-1 done in S1; CE-F-3/F-4/F-5 are S3/S4.)

## 3. Intent (invariant impact)
Lands the **RED-custody-loading half of `CN-NODE-03`**: operator key material
enters the node only through the existing RED-parse → BLUE-structural-validate →
canonical-type loaders, terminating in a `ProducerShell` that is the sole custody
holder. The module exposes no private-key bytes — no byte accessor, no
serialization, no logging — so "a key byte escaped RED custody into glue, logs,
or evidence" is mechanically foreclosed for the node path. KES-period-vs-opcert
freshness is enforced at `ProducerShell::init` (carried, not re-implemented).

## 4. Pre-conditions
- S1 merged (`3c4bcca`): `ade_node::forge_intent::ForgePaths` exists.
- The reused loaders exist and are tested under OP-OPS-04: `load_cold_signing_key_skey`,
  `load_vrf_signing_key_skey` (pub); `load_kes_skey_any_format`,
  `parse_simple_opcert_json` (produce_mode-private → widened to `pub(crate)`).
- `ProducerShell::init(kes, vrf, cold, opcert) -> Result<Self, ShellInitError>`
  (closed) enforces the opcert shape + KES-period bounds.

## 5. Implementation boundary
- **New module** `operator_forge.rs` (`//! RED`):
  - `pub enum OperatorForgeError` (closed; secret-free — carries only structured
    loader/parse/init errors, never path strings or key bytes):
    `ColdKeyLoad(KeyLoadError)`, `VrfKeyLoad(KeyLoadError)`,
    `KesKeyLoad(KeyLoadError)`, `OpcertParse(&'static str)`,
    `ShellInit(ShellInitError)`. `Display` + `std::error::Error` impls.
  - `pub fn load_operator_producer_shell(paths: &ForgePaths) -> Result<ProducerShell, OperatorForgeError>`
    — loads cold → vrf → kes → opcert via the reused loaders, maps each failure
    to the structured variant, then `ProducerShell::init`. Returns the shell by
    value (custody holder); exposes no private bytes.
- **Visibility widening (behavior-preserving):** `load_kes_skey_any_format` and
  `parse_simple_opcert_json` in `produce_mode.rs` become `pub(crate)`. No logic
  change; `produce_mode`'s own call sites and tests are untouched.
- **No genesis parse, no `ForgeActivation`, no `CoordinatorState`, no clock, no
  binary wiring** (all S3). No `pool_id` (S3, derived in one named place).

## 6. TCB color
- **RED:** `ade_node::operator_forge` (new — opens/reads operator key files via
  the reused loaders; `//! RED` banner). Reused loaders are RED
  (`ade_runtime::producer`). `produce_mode` visibility widening is RED-internal.
- **GREEN:** consumes `ForgePaths` (S1 GREEN type) — no GREEN change.
- **BLUE:** none — the KES structural validator (`Sum6Kes::raw_deserialize_signing_key_kes`)
  is reached only *inside* the existing RED loader; S2 adds no BLUE reference.

## 7. Invariants preserved (must not weaken) — by registry ID
- **CN-PROD-02 (T-tier key custody)** — KES/VRF/cold private material stays inside
  `ProducerShell`; S2 adds no path that copies it into GREEN coordinator state,
  the planner, the WAL, a log, or evidence. The existing
  `ci_check_private_key_custody.sh` stays green (no new key type defined outside
  `producer/`; no raw-byte accessor).
- **OP-OPS-04** — the reused loaders are unchanged; both KES flows + VRF/cold
  text-envelopes still load exactly as before.
- **DC-NODE-05 / CE-F-6** — the relay loop + forge-containment gate are untouched.
- **CN-NODE-02** — no loop/planner change.
- All BLUE invariants — no BLUE crate modified.

## 8. Invariants strengthened (one family: CN-NODE-03)
- **CN-NODE-03** (`declared`) — lands its **RED-custody-loading half**: the node
  path's operator-material ingress is a single named RED site that reuses the
  existing loaders and leaks no private bytes. Contributes **CE-F-2**. (Flips
  `declared → enforced` at cluster close; no `strengthened_in` bumps this slice.)

## 9. Replay / determinism obligations
- `load_operator_producer_shell` is I/O (RED) and not a replay surface; it
  introduces no authoritative state, no canonical type, no WAL/checkpoint. Given
  fixed key files it is deterministic (same files ⇒ same shell public surface ⇒
  same structured error). No corpus entry.

## 10. Mechanical acceptance criteria
- [ ] `operator_forge.rs` exists with `//! RED` banner; `OperatorForgeError`
      closed (no `#[non_exhaustive]`); `load_operator_producer_shell` defined and
      returns `ProducerShell` by value.
- [ ] `produce_mode::{load_kes_skey_any_format, parse_simple_opcert_json}` are
      `pub(crate)`; `produce_mode` behavior unchanged (`cargo test -p ade_node
      produce` green).
- [ ] Test `load_operator_producer_shell_builds_shell_from_complete_material` —
      using the real-format fixture idiom (ade-native KES envelope, cardano-cli
      VRF/cold text-envelopes, opcert JSON), loads a `ProducerShell` and asserts
      its **public** surface (`cold_vk`, `vrf_verification_key`, `opcert()`
      sequence/period, `public_metadata`) — never private bytes.
- [ ] Test `load_operator_producer_shell_missing_cold_fails_closed` — a
      nonexistent cold path ⇒ `Err(OperatorForgeError::ColdKeyLoad(..))` (and
      analogous for vrf/kes/opcert), fail-closed, structured.
- [ ] Test `load_operator_producer_shell_kes_period_past_opcert_fails_closed` —
      a KES/opcert mismatch ⇒ `Err(OperatorForgeError::ShellInit(..))` (the
      carried CN-PROD-02 / I5 freshness bound at init).
- [ ] Test `operator_forge_error_carries_no_path_or_key_bytes` — a load failure's
      Debug/Display contains neither a supplied path string nor key bytes.
- [ ] **New CI gate** `ci/ci_check_operator_forge_no_secret_leak.sh` (exit 0):
      `operator_forge.rs` carries `//! RED`; production body (comments +
      `#[cfg(test)]` stripped) contains none of `println!`, `eprintln!`, `dbg!`,
      `to_bytes`, `as_bytes`, `Serialize`, `Deserialize`, `unsafe`,
      `CoordinatorState`; no `pub fn` returns `[u8`/`Vec<u8>`/`&[u8]`; positively
      calls `ProducerShell::init(` and defines `load_operator_producer_shell`.
      **Additive only — touches no existing gate** (CE-F-6 preserved).
- [ ] `cargo build -p ade_node` clean; `cargo test -p ade_node operator_forge`
      green (count > 0); `cargo test -p ade_node produce` still green;
      `rustfmt` applied; the new gate + `ci_check_private_key_custody.sh` both pass.

## 11. Forbidden in this slice (inherits the cluster Forbidden list)
- **No relaxation of the N-F-E forge-containment gate** (hard rule, CE-F-6).
- No private-key byte accessor / serialization / logging in `operator_forge`; no
  `dbg!`/`println!`/`eprintln!` of the shell or any key; no `Serialize`/
  `Deserialize` on key-bearing types.
- No `pool_id` derivation, no genesis parse, no `CoordinatorState`, no clock, no
  `ForgeActivation`, no binary wiring (all S3).
- No reimplementation/duplication of the KES/VRF/cold/opcert loaders — reuse only.
- No new BLUE reference; no new canonical type / WAL / checkpoint.
- No `#[non_exhaustive]` on `OperatorForgeError`.

## 12. Slice completion checklist
- [ ] `operator_forge.rs` added (error + loader + tests); registered in `lib.rs`.
- [ ] `produce_mode` helpers widened to `pub(crate)` (no behavior change).
- [ ] `ci/ci_check_operator_forge_no_secret_leak.sh` added, executable, exits 0;
      no existing gate modified; `ci_check_private_key_custody.sh` still green.
- [ ] `cargo build/test -p ade_node` green; `rustfmt` applied; gates pass.
- [ ] Slice doc committed standalone (`docs:`) before impl; impl (`feat:`) after green.

## Authority
Registry IDs `CN-NODE-03` (RED-custody half; `declared`), `CN-PROD-02` /
`OP-OPS-04` / `DC-NODE-05` / `CN-NODE-02` (preserved). The cluster-slice-plan and
`docs/ade-invariant-registry.toml` are authoritative; this slice doc refines.
