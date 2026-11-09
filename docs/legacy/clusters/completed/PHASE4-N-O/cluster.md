# PHASE4-N-O — Ade-native KES key-gen + closed envelope (cluster doc)

> **Status:** Planning. Single-slice closure of the OP-OPS-04
> `open_obligation` recorded by PHASE4-N-C for the **Ade-native
> KES operator flow**. The complementary obligation —
> importing cardano-cli's 608-byte expanded `Sum6KES` skey —
> is explicitly deferred to PHASE4-N-P.

**Predecessor:** PHASE4-N-C (block production, Tier 1 producer
half). OP-OPS-04 was opened as `enforced + open_obligation` on
the Sum6KES expanded-tree skey loader.

**Successor:** PHASE4-N-P (Sum6KES expanded compatibility —
full `ade_crypto::kes_sum` reimplementation; cardano-cli
expanded skey deserialization; cross-impl vectors; CI gate).

## §1 Primary invariant

> Ade's KES hot-signing key is loaded **only** from an Ade-native
> closed envelope `ade.kes.seed.v1` carrying:
>
> - `format = "ade.kes.seed.v1"`
> - `role = "kes_hot_signing_key"`
> - `crypto = "Sum6KES-Ed25519DSIGN"`
> - `seed_32` (32-byte hot signing seed)
> - `period_idx` (u32 — KES period at which the envelope was
>   produced; loader fast-forwards `Sum6Kes::update_kes` exactly
>   `period_idx` times before returning the `KesSecret`)
> - `format_version = 1`
>
> All other shapes — cardano-cli's `KesSigningKey_ed25519_kes_2^6`
> envelope (32-byte seed, 608-byte expanded, 612-byte, any other
> payload size), unknown `format` strings, missing `seed_32`,
> wrong `role`, unsupported `crypto`, malformed JSON / CBOR —
> MUST fail-closed at the loader with a structured
> `KeyLoadError` variant. No fallback parser. No heuristic
> guess. No partial accept.
>
> `ade_node key-gen-KES --out-file PATH` produces an
> `ade.kes.seed.v1` envelope sourced from `/dev/urandom` (or an
> operator-supplied 32-byte file via the seam used by tests),
> emits the four allowed CLI lines (filename, format, role,
> VK fingerprint), and writes the envelope to PATH with mode
> `0600`. Private-key bytes (the 32-byte seed) NEVER appear in
> any output channel: stdout, stderr, JSONL logs, admission
> transcripts, shadowbox evidence, panic messages, structured
> errors, debug formatting.

### Why this matters

The OP-OPS-04 obligation as written by N-C said:
> "cardano-crypto 1.0.8 exposes `gen_key_kes_from_seed_bytes`
> but no `raw_deserialize_signing_key_kes` for Sum6Kes; the
> keys loader currently round-trips synthetic seed-encoded
> skeys but does not yet load real cardano-cli's 612-byte
> expanded-tree Sum6KES serialization."

Two operationally distinct paths exist:
- **Ade-native flow** — Ade generates the KES seed, persists it
  in a closed envelope, and reloads it for signing. The
  operator hands cardano-cli the corresponding VK to issue an
  opcert. No cardano-cli `.skey` ever crosses Ade's boundary.
- **cardano-cli import flow** — operator runs
  `cardano-cli node key-gen-KES`, Ade loads the resulting
  608-byte expanded `.skey`. Closed by `cardano-crypto`
  exposing only the seed-based constructor; closure requires a
  full `ade_crypto::kes_sum` reimplementation (PHASE4-N-P).

For the bounty acceptance test, the Ade-native flow is
sufficient — the bounty requires *Cardano-valid block
production*, not *compatibility with every cardano-cli operator
key-file format*. PHASE4-N-O ships the Ade-native flow and
makes the cardano-cli import path **explicitly unsupported**
with a structured `UnsupportedExpandedKesKeyFormat` error.

The doctrine boundary: Ade's signing path remains RED/shell-
only (private-key custody in `ade_runtime::producer::signing`),
verification stays deterministic in BLUE
(`ade_crypto::kes::verify_kes_signature`). The closed envelope
gives the operator a stable on-disk format owned by Ade with
no dependency on cardano-cli's serialization choices.

The 608-byte payload was confirmed by running
`cardano-cli node key-gen-KES` inside the docker preprod peer
(`cardano-cli 11.0.0.0`, `ghc-9.6`). Layout matches Haskell's
`rawSerialiseSignKeyKES (Sum6KES Ed25519DSIGN)` recurrence:
`32 + 6 * (32 + 2 * 32) = 32 + 576 = 608`. The OP-OPS-04 memo's
"612 bytes" figure was off by 4; N-O treats 608 as the
canonical cardano-cli expanded payload size, and 612 as a
separately-rejected malformed size (defense-in-depth).

## §2 Scope

### In scope

- `crates/ade_runtime/src/producer/ade_kes_envelope.rs` (new):
  closed JSON-envelope reader/writer for `ade.kes.seed.v1`.
  Pure: takes/returns `[u8; 32]` seed + `u32` period_idx.
  No I/O — file I/O lives in `keys.rs`. Serde-derive on a
  struct with `#[serde(deny_unknown_fields)]` for the
  forbidden-extension surface; optional metadata fields
  collected into a sidecar struct deliberately separate from
  the load-bearing payload.

- `crates/ade_runtime/src/producer/keys.rs`:
  - `load_ade_kes_signing_key(path)` — reads the envelope file,
    delegates parsing to `ade_kes_envelope::parse`, then calls
    `KesSecret::from_seed_at_period(seed, period)` to construct
    the `KesSecret` advanced to `period_idx`.
  - `load_kes_signing_key_skey(path)` — kept as the
    cardano-cli envelope-type-discriminator path, but every
    payload size now maps to
    `KeyLoadError::UnsupportedExpandedKesKeyFormat`. The
    cardano-cli envelope is recognized only to be rejected.
  - `KeyLoadError` adds closed variants:
    `UnsupportedExpandedKesKeyFormat`,
    `UnknownEnvelopeFormat`, `MissingSeed32`,
    `MalformedPeriodIdx`, `WrongKeyRole`, `UnsupportedCryptoTag`.
    Existing variants kept.
  - Old test `cardano_cli_skey_envelope_round_trips_through_keys_loader`
    is split into two: a new positive test against the
    Ade-native envelope, and a negative test asserting the
    cardano-cli envelope is rejected even with a 32-byte
    payload.

- `crates/ade_runtime/src/producer/signing.rs`:
  - New `KesSecret::from_seed_at_period(seed: &[u8; 32], period: u32)
    -> Result<Self, SigningError>` — calls
    `Sum6Kes::gen_key_kes_from_seed_bytes(seed)`, then
    `Sum6Kes::update_kes(&(), inner, p)` for each `p` in
    `0..period`, returning the final `KesSecret { inner,
    current_period: KesPeriod(period), evolutions_remaining }`.
    Existing `from_bytes_zeroizing` remains usable internally
    but is no longer the loader entry point.

- `crates/ade_node/src/cli.rs`:
  - Add `Mode::KeyGenKes`. New optional flags:
    `--out-file PATH` (required when mode is `KeyGenKes`),
    `--period-idx N` (default 0; u32),
    `--seed-file PATH` (test seam; when present, read 32
    bytes from PATH instead of `/dev/urandom`).
  - `CliError` adds: `KeyGenMissingOutFile`,
    `InvalidPeriodIdx(String)`.

- `crates/ade_node/src/key_gen.rs` (new):
  - `run_key_gen_kes(cli)` — RED-only: opens `/dev/urandom` (or
    `--seed-file`), reads 32 bytes, writes the envelope, derives
    + prints the VK fingerprint. Returns `ExitCode`.
  - Filesystem permissions `0o600` enforced via
    `std::os::unix::fs::OpenOptionsExt::mode`.

- `crates/ade_node/src/main.rs`:
  - Add `Mode::KeyGenKes` dispatch arm; no admission writer
    used for key-gen.

- `crates/ade_node/src/lib.rs`:
  - Re-export `key_gen::run_key_gen_kes` for the binary +
    tests.

- `crates/ade_core_interop/src/bin/live_block_production_session.rs`:
  - Switch from `load_kes_signing_key_skey` to
    `load_ade_kes_signing_key`.

- `docs/active/op-ops-04-ade-native-kes-flow.md` (new) —
  operator-facing README excerpt verbatim from the user spec
  (challenge-build KES key flow, fail-closed semantics, claim
  boundary).

- `docs/ade-invariant-registry.toml`:
  - OP-OPS-04: clear `open_obligation`, update statement +
    code_locus + tests + ci_script with the Ade-native flow;
    append PHASE4-N-O to `strengthened_in`; carry the new
    `open_obligation` text for the cardano-cli expanded
    import deferral to PHASE4-N-P.

- `ci/ci_check_kes_envelope_closed.sh` (new) — mechanical
  fail-closed assertions:
  - `KeyLoadError` enum contains all required closed variants.
  - No `KesSigningKey_ed25519_kes_2^6` envelope payload
    branches that return `Ok(_)`.
  - No `seed_32` substring in any test fixture / log /
    transcript file under the repo.
  - Closed CLI vocabulary on `key-gen-KES` success: the four
    allowed lines (filename, format, role, VK fingerprint)
    and no others.

### Out of scope (explicit)

- cardano-cli 608-byte expanded `Sum6KES` skey deserialization.
  This is the PHASE4-N-P deliverable. The N-O loader recognizes
  the cardano-cli envelope only to fail-close it.
- Reimplementing Sum6KES in `ade_crypto`. The N-O signing path
  continues to use `cardano-crypto`'s `Sum6Kes` directly via
  `gen_key_kes_from_seed_bytes` + `update_kes`.
- Hardware key custody (HSM, TPM). Operator's filesystem +
  `0600` permissions is the N-O custody boundary; HSM is a
  future operational concern outside Ade's scope.
- Multi-network metadata enforcement. The optional
  `genesis_hash`, `network_magic`, `created_at_slot`,
  `created_by` fields may be parsed into a sidecar struct but
  do not change signing semantics in N-O — they would require
  their own mechanical-enforcement story which is a future
  slice.
- Rotation tooling. KES-period rotation policy is an
  operational concern documented in the README, not mechanized
  in N-O.

## §3 Slice index

| Slice | Purpose | New rules / strengthenings |
|---|---|---|
| **S1** | Closed Ade KES envelope + loader + `ade_node key-gen-KES` + cardano-cli fail-closed + OP-OPS-04 statement update | strengthens OP-OPS-04 (Ade-native flow clears the open_obligation); introduces DC-CRYPTO-06 (closed envelope format), DC-CRYPTO-07 (cardano-cli expanded fail-closed) |

## §4 Exit criteria (cluster-level MACs)

1. `ade_node --mode key_gen_kes --out-file /tmp/k.skey` writes
   an `ade.kes.seed.v1` envelope readable by
   `load_ade_kes_signing_key`; the loaded `KesSecret` signs +
   verifies through `kes_sign` + `verify_kes_signature` at
   `KesPeriod(0)`.
2. `ade_node --mode key_gen_kes --out-file /tmp/k.skey
   --period-idx 5` writes an envelope whose `load_ade_kes_signing_key`
   yields a `KesSecret` at `KesPeriod(5)`; signing at period 5
   round-trips through `verify_kes_signature`; signing at any
   `period < 5` returns `SigningError::PastEvolutionsRemaining`.
3. `load_kes_signing_key_skey` (cardano-cli envelope path) on a
   32-byte payload returns
   `Err(KeyLoadError::UnsupportedExpandedKesKeyFormat)`.
4. Same loader on a 608-byte payload returns
   `Err(KeyLoadError::UnsupportedExpandedKesKeyFormat)`.
5. Same loader on a 612-byte payload returns
   `Err(KeyLoadError::UnsupportedExpandedKesKeyFormat)`.
6. `load_ade_kes_signing_key` returns the correct closed-error
   variant for each of: unknown `format` string, missing
   `seed_32`, non-u32 `period_idx`, wrong `role`, unsupported
   `crypto` string, malformed JSON, `seed_32` of wrong length,
   period_idx > `SUM6_MAX_PERIOD` (64).
7. `ade_node` CLI success output for `key_gen_kes` contains
   exactly four lines (filename, format, role, VK fingerprint)
   and no others; in particular contains no hex seed substring.
8. No JSONL log / admission transcript / evidence file in the
   working tree contains the literal substring `"seed_32"` or
   a 64-character hex fragment matching the test seed.
9. `cargo test --workspace` clean.
10. `ci/ci_check_kes_envelope_closed.sh` exits 0; added to
    `ci/CHECKS.md` and the cluster-close gate set.
11. OP-OPS-04 registry entry: `open_obligation` cleared for the
    Ade-native flow; `strengthened_in += PHASE4-N-O`;
    statement, code_locus, tests, ci_script updated.
12. `docs/active/op-ops-04-ade-native-kes-flow.md` committed
    with the user-provided verbatim README + claim boundary.
13. PHASE4-N-P queued as a planning placeholder (cluster doc
    sketch) so the cardano-cli expanded-import deferral is
    visible in the planning tree.
14. Commit + push with the project-override trailer.

## §5 Hard prohibitions

- No fallback parser. If the loader sees a payload it does not
  recognize, it returns the corresponding closed-error variant.
  Never `Ok(_)` for an unknown shape.
- No private-key bytes in stdout, stderr, JSONL logs,
  admission transcripts, panic messages, debug formatting, or
  structured errors. `KeyLoadError` carries closed
  `&'static str` detail strings and envelope-type strings,
  never raw seed/key bytes.
- No `Co-Authored-By:` removal from commits — the project
  override requires the trailer on every commit per
  `~/.claude/projects/.../ade/CLAUDE.md`.
- No I/O / eprintln in BLUE. `ade_crypto` stays pure; the
  loader + key-gen + envelope serde live in RED
  (`ade_runtime` and `ade_node`).
- No deferring the cardano-cli expanded fail-closed to
  PHASE4-N-P. N-O MUST fail-closed cardano-cli envelopes —
  the deferral is only of the *positive* expanded-import
  path.

## §6 Replay obligations preserved

- T-DET-01 — unchanged. The envelope serde is deterministic
  (key ordering enforced via `serde_json::Map::insert` order;
  base64 / hex encoding is deterministic).
- T-KEY-01 — strengthened. The Ade-native envelope is the
  closed boundary at which operator-supplied seeds enter RED
  custody; cardano-cli envelopes are now closed-out.
- OP-OPS-04 — Ade-native half closed; cardano-cli half re-
  emitted as `open_obligation =
  "blocked_until_phase4_n_p_kes_sum_reimpl"`.

## §7 References

- Predecessor: `df56e2d` (PHASE4-N-C close).
- User spec (verbatim) committed at
  `docs/active/op-ops-04-ade-native-kes-flow.md`.
- Doctrine: [[feedback-shell-must-not-overstate-semantic-truth]]
  (closed CLI vocabulary on key-gen success),
  [[feedback-fail-closed-validation]] (each unsupported
  envelope shape has a negative test),
  [[feedback-codec-closed-grammar]] (envelope is closed
  grammar, no extension surface).
- Successor cluster sketch:
  `docs/clusters/PHASE4-N-P/cluster.md` (planning placeholder
  added by N-O S1).
