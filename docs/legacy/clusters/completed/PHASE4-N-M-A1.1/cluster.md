# PHASE4-N-M-A1.1 — Reference-script seed-import strengthening (cluster doc)

> **Status:** Planning. Single-slice strengthening of PHASE4-N-M-A
> slice A1. Closes the A1.1 gate that holds `DC-EVIDENCE-01` and
> `RO-LIVE-05` at `enforced_scaffolding`. Flips both to `enforced`
> on close.

**Predecessors:** PHASE4-N-M-A (oracle-seed importer + WAL),
PHASE4-N-M-C (live operator pass — bounded closure on RO-LIVE-05
with the A1.1 carve-out).

**Successor:** none planned. After A1.1 closes, the next N-M
question is the genesis-→-P replay cluster (`RO-GENESIS-REPLAY-01`),
which is a separate concern.

**Sketch:** this cluster is small enough that the cluster doc
substitutes for a separate invariants sketch — see §1 below.

## §1 Primary invariant

> The `import_cardano_cli_json_utxo` BLUE/GREEN authority MUST
> ingest every UTxO entry that a cardano-cli `query utxo
> --whole-utxo` JSON dump contains, including entries with a
> populated `referenceScript` field of any of the four
> cardano-node-supported variants (`SimpleScript`,
> `PlutusScriptV1`, `PlutusScriptV2`, `PlutusScriptV3`). The
> imported `UTxOState` MUST carry a canonical Babbage-shape
> TxOut field 3 (`script_ref` = `tag(24, bytes(.cbor script))`)
> for each such entry, and the resulting `UtxoFingerprint`
> MUST differ from the equivalent entry with a `null`
> `referenceScript` (i.e., reference scripts are part of the
> canonical fingerprint).
>
> Unrecognized referenceScript shapes (unknown `type` string,
> missing `cborHex`, non-hex `cborHex`) MUST fail-fast via a
> closed error — never silently dropped, never substituted
> with a placeholder.

## §2 Why this matters

Per [[feedback-shell-must-not-overstate-semantic-truth]] and
[[feedback-oracle-seed-then-ade-owns]]: the oracle seed at
bootstrap point P is authoritative — Ade cannot silently
re-shape what the operator imported. Dropping reference-script
TxOuts is a silent partial seed, which would let the BLUE
admission path see a UTxOState that is NOT the one the operator
extracted, breaking the bootstrap-anchor invariant chain
(`UtxoFingerprint` → `BootstrapAnchor` → `WalEntry` chain →
admitted blocks).

This is also the literal gate identified in PHASE4-N-M-C's
closure record: full `BlockAdmitted` transcript on a current-tip
preprod snapshot was bounded out of N-M-C because the import
fail-fast prevented loading the full preprod UTxO. Closing A1.1
removes that bound.

## §3 Scope

### In scope

- Extend `ade_runtime::seed_import::json` to deserialize the
  full referenceScript shape:
  ```json
  "referenceScript": {
    "script": {
      "cborHex": "<hex>",
      "description": "<string>",
      "type": "PlutusScriptV1" | "PlutusScriptV2" | "PlutusScriptV3" | "SimpleScript"
    },
    "scriptLanguage": "<string>"
  }
  ```
  The `description` and `scriptLanguage` fields are accepted
  but ignored (they're cardano-cli metadata, not part of the
  canonical bytes).

- Extend `ade_runtime::seed_import::importer` to:
  - Map each `type` to its canonical script-variant tag
    (`SimpleScript` → 0, `PlutusScriptV1` → 1, `PlutusScriptV2`
    → 2, `PlutusScriptV3` → 3).
  - Build the canonical Babbage `script_ref` CBOR for field 3:
    `tag(24, bytes(cbor([variant_tag, raw_cborHex_bytes])))`
    where `raw_cborHex_bytes` is the hex-decoded `cborHex`
    content (already a CBOR value — `bytes(...)` for Plutus
    variants, `array(...)` for native scripts).
  - Append field 3 to the canonical TxOut map after field 2
    (datum_option), keeping field-ordering canonical.

- Add a closed-error case `BadReferenceScript { detail:
  &'static str }` to `JsonSeedError` for: unknown `type`
  string, missing `cborHex`, non-hex `cborHex`, missing
  `script` sub-object.

- Replace the existing `UnsupportedTxOutFeature {
  feature: "referenceScript" }` fail-fast with the new
  acceptance path; keep `UnsupportedTxOutFeature` for any
  future genuine non-support cases (none currently).

### Out of scope (explicit)

- Plutus-script semantic validation (Phase 5 cluster).
- Native-script witness evaluation.
- Decoding/re-encoding the inner `cborHex` content (we treat
  it as already-canonical CBOR per cardano-node's contract;
  see §6).
- Reference-script use in admission (already wired —
  `admit_via_block_validity` consumes the canonical TxOut bytes
  unchanged; this slice only changes import, not validation).
- Mithril or non-cardano-cli sources.

## §4 Slice index

| Slice | Purpose | New rules / strengthenings |
|---|---|---|
| **A1.1** | Reference-script seed-import support | strengthens CN-SEED-01 + DC-SEED-01; unblocks DC-EVIDENCE-01 + RO-LIVE-05 |

Single-slice cluster — see `A1.1.md`.

## §5 Exit criteria (cluster-level MACs)

1. `CN-SEED-01` registry entry strengthened (no functional
   change; `strengthened_in` appends `PHASE4-N-M-A1.1`; new
   tests appended; evidence_notes updated to note the 129/129
   full-import smoke).
2. `DC-SEED-01` registry entry strengthened similarly.
3. `DC-EVIDENCE-01` flipped from `enforced_scaffolding` to
   `enforced`. `open_obligation` field removed. `evidence_notes`
   updated to reflect the A1.1 gate close + full preprod-tip
   `BlockAdmitted` transcript (or an explicit honest statement
   if the live pass is operator-action-deferred for the same
   reason RO-LIVE-04 was originally — see §6).
4. `RO-LIVE-05` flipped from `enforced_scaffolding` to `enforced`
   similarly; bounded statement updated to drop the A1.1
   carve-out.
5. `ci/ci_check_seed_import_closure.sh` still passes
   (single-pub-fn discipline preserved).
6. `cargo test --workspace` clean.
7. `cargo clippy --workspace --all-targets -- -D warnings`
   clean.
8. Replay-equivalence preserved (`cargo test -p ade_testkit`
   clean).
9. Smoke against the committed 129-entry preprod sample:
   129/129 parse, deterministic fingerprint captured in the
   closure record.
10. Smoke against the full ~3.4M-entry preprod dump (operator
    action, evidence-only): no fail-fasts, fingerprint
    captured (or, if not runnable in this turn, a clear
    runbook step recorded in the closure record).
11. Commit + push to `main` with the project-override
    `Co-Authored-By: Claude` trailer.

## §6 Hard prohibitions

- No silent partial seed — any entry that fails to map MUST
  return an error.
- No `UnsupportedTxOutFeature { feature: "referenceScript" }`
  in the success path.
- No re-encoding of `cborHex` content — the canonical-input
  contract is that cardano-cli emits canonical CBOR for the
  inner script payload; we include it raw. (If a future
  finding shows cardano-cli emits non-canonical Plutus
  envelopes for any variant, that's a separate slice — open
  it then, don't add a re-encoder pre-emptively.)
- No `HashMap` / `HashSet` / float / wall-clock / rand in any
  added path.
- No second `pub fn import_cardano_cli_json_utxo` or
  `parse_utxo_seed_json`.

## §7 References

- Source: closure record `docs/clusters/completed/PHASE4-N-M-A/CLOSURE.md`
  §"Reference script TxOut features" (declares this as the A1.1
  follow-on).
- Doctrine: [[feedback-oracle-seed-then-ade-owns]],
  [[feedback-shell-must-not-overstate-semantic-truth]],
  [[feedback-fail-closed-validation]].
- Predecessor closures: `c7e2a23` (N-M-A), `8843e20` (N-M-C).
- Babbage CDDL `script_ref` shape: per Cardano ledger spec
  (`#6.24(bytes .cbor script)`, where
  `script = [0, native_script // 1, plutus_v1_script // 2,
   plutus_v2_script // 3, plutus_v3_script]`).
