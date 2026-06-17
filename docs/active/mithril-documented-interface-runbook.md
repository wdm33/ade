# Mithril documented-interface bootstrap — decision record + operator runbook

> **Status:** Decision record + operator guidance (not a cluster, not an implementation
> slice). Records how Ade satisfies `RO-MITHRIL-IMPORT-01` item (a), provides the
> operator procedure for the documented-interface Mithril path, and (slice
> `RO-MITHRIL-IMPORT-01-EVIDENCE-SCHEMA`) wires up the **turnkey capture + validation
> tooling** for the remaining item (c). `RO-MITHRIL-IMPORT-01` remains **partial** —
> no registry flip and no `ci_check_*` gate ship until a real, validated,
> operator-witnessed bundle is committed.

## Decision

**Native decoding of Mithril ancillary / UTXO-HD / LedgerDB bytes is a Tier-4 non-goal
for this bounty path.** `RO-MITHRIL-IMPORT-01` item (a) is satisfied by the
**documented-interface path**:

```
Mithril-bootstrapped Cardano peer
  → documented cardano-cli / query extraction (cardano-cli query utxo, consensus-input extraction)
  → Ade seed_import
  → CN-MITHRIL-01 / DC-MITHRIL-02 binding (verify_mithril_binding, fail-closed)
```

A native byte path, if ever needed, belongs under `RO-GENESIS-REPLAY-01` via **certified
block replay** of ImmutableDB chunks — **not** UTXO-HD reverse engineering.

**Why** (the artifact-type spike): a Mithril *cardano-db* snapshot is the ImmutableDB
block chunks (→ forward-replay = `RO-GENESIS-REPLAY-01`, deferred); the
`--include-ancillary` ledger-state snapshot is cardano-node's **UTXO-HD InMemory**
binary, converted via `mithril client utxo-hd snapshot-converter` to LMDB/Legacy — i.e.
cardano-node's **private storage serialization**, not an Ade-stable interchange format.
Decoding it in Ade would pull cardano-node's private serialization into Ade's authority
surface, contra [[feedback-oracle-seed-then-ade-owns]] and
[[feedback-mithril-is-peer-infra-not-ade-authority]] ("documented cli interfaces over
reverse-engineering utxohd").

## Default evaluator startup path (bounty)

For the bounty judge and external evaluators, **Mithril-certified snapshot restore is the
default startup path** — not from-genesis replay. The public challenge allows sync from
*either* a recent Mithril snapshot *or* genesis up to tip, so the snapshot path is
legitimate; it is also simpler for the judge, safer for us, and reproducible from a
**fresh scratch directory** (no dependency on our long-lived local peer). The
evaluator-facing path is:

    download certified snapshot → restore a FROZEN node → extract the certified state
      → start Ade from that state → follow tip → produce/serve

From-genesis (`genesis replay → full historical validation → tip`) remains **owed** as a
separate compatibility milestone (`RO-GENESIS-REPLAY-01`) with its own invariant slices
and evidence — important, but **not** the default judge path today. The Mithril path is
kept honest and boring: `certified_point` comes from Mithril metadata,
`operator_seed_point` from the frozen restored node, the two are compared, Ade imports
that exact state, and a negative control proves a mismatch is rejected.

## Operator runbook — the documented-interface path

The Ade-side composition (steps 3–6) is a **single** `ade_node --mode node` first-run
invocation; `ci/capture_mithril_documented_evidence.sh` automates the whole sequence
and emits an evidence bundle.

1. **Acquire the certified snapshot into a FRESH SCRATCH dir** (never the canonical peer
   DB): `mithril-client cardano-db download <DIGEST> --include-ancillary --download-dir
   <scratch>`, verifying against the **canonical** genesis + ancillary verification keys
   (fetched fresh from the Mithril repo — NOT the stale genesis key hardcoded in
   `ci/mithril_restore_preprod_peer.sh`, whose last 4 bytes differ and fails cert-chain
   verify). Mithril is acquisition infra, **never an Ade trust root**.
2. **Bring a THROWAWAY node to the FROZEN certified state:** start a disposable
   `cardano-node` container on the scratch DB with an **empty topology** (no peers, no
   network sync) so its tip cannot drift past the certified immutable boundary. The
   `--include-ancillary` ledger snapshot lets it bootstrap in minutes rather than
   replaying from genesis for hours.
3. **Extract at the FROZEN boundary via documented interfaces:** `cardano-cli query utxo
   --whole-utxo` (the **seed**); `ci/build_consensus_inputs_bundle.sh` (epoch nonce /
   stake / ASC / epoch window); and the **operator seed point** (slot + block hash) from
   `cardano-cli query tip`. The capture script first verifies the frozen tip's epoch
   equals the cert epoch — proof it has not synced past the boundary — before treating it
   as the certified point.
4. **Build the Mithril manifest** (`RawMithrilManifest` JSON: `artifact_type`,
   `certificate_hash_hex`, `network_magic`, `genesis_hash_hex`,
   `certified_point{slot, block_hash_hex}`, `immutable_range{lo,hi}`,
   `source_mithril_client_version`, `source_command`) — the provenance carrier passed
   via `--mithril-manifest-path`.
5. **Import + bind + bootstrap in one invocation:**
   ```
   ade_node --mode node \
     --genesis-path <bundle> --network <net> \
     --json-seed utxo.json --consensus-inputs-path consensus-inputs.json \
     --mithril-manifest-path mithril-manifest.json \
     --seed-point-slot <slot> --seed-block-hash <hash> \
     --network-magic <m> --genesis-hash <gh> \
     --snapshot-dir <snap> --wal-dir <wal>
   ```
   The FirstRun arm (`node_lifecycle::first_run_mithril_bootstrap`) runs
   `import_cardano_cli_json_utxo` → recomputes `seed_artifact_hash`
   (`blake2b_256_of_file(utxo.json)`) → checks the cert slot is in the consensus epoch
   window → mints the `BootstrapAnchor` with `seed_point` = the **operator-extracted**
   point (independent origin, DC-MITHRIL-02) and `seed_provenance` from the manifest →
   runs `verify_mithril_binding` (fail-closed on mismatch — CN-MITHRIL-01 / DC-MITHRIL-02)
   → `bootstrap_initial_state`. Success prints
   `first-run Mithril bootstrap complete (anchor initial_ledger_fingerprint=…, epoch=…)`.
6. **Ade owns all state after the anchor** (forward-sync).

## What this deliberately does NOT do

- Ade does **not** decode the Mithril ledger-state ancillary (UTXO-HD/LedgerDB bytes) —
  Tier-4 non-goal.
- It does **not** forward-replay ImmutableDB chunks (that is `RO-GENESIS-REPLAY-01`).
- It does **not** treat the Mithril manifest as a BLUE trust root: the manifest only
  carries provenance; the anchor `seed_point` is minted from the operator's independent
  extraction, and `ci_check_mithril_seed_point_independence.sh` structurally forbids
  laundering the manifest into the seed point.

> Note: the `--mithril-manifest-path` flag **is** wired into `--mode node` first-run
> (parsed in `cli.rs`; consumed by `first_run_mithril_bootstrap`; covered by
> `node_cli_parses_mithril_manifest_path_from_argv` + the FirstRun fail-closed tests).
> An earlier draft of this runbook said no such flag existed — that was stale. What
> remains a future operator-UX nicety is a one-shot `--mode mithril-bootstrap`
> convenience wrapper, not the capability, which exists today.

## Evidence shape — turnkey tooling (this slice) + the remaining gap

The bundle schema, a capture orchestrator, and a validator now exist so the remaining
item (c) — a *committed, reproducible, operator-witnessed* documented-interface pass —
is turnkey:

| Tool | Role |
|---|---|
| `docs/evidence/schemas/mithril-documented-evidence.schema.md` | field schema + promotion path. |
| `docs/evidence/schemas/mithril-documented-evidence.manifest.template.toml` | fill-in manifest template. |
| `ci/capture_mithril_documented_evidence.sh` | RED operator orchestrator, **non-destructive scratch venue**: downloads the snapshot into a fresh dir, runs a throwaway **frozen** (empty-topology) node, sources `certified_point` from the cert + frozen boundary (verifying tip epoch == cert epoch) independent of `operator_seed_point`, runs the positive `--mode node` first-run **and** a flipped-hash negative control, emits the sha256-bound bundle. Canonical `.cardano-node-preprod/db` is never touched. |
| `ci/validate_mithril_documented_evidence.sh` | validator: vacuous-PASS when no bundle is committed; strict (required fields + sha256-bound artifacts + `binding_result=pass` + negative-control fail-closed) when one is. |

A green bundle proves the **full documented-interface chain** ran on real artifacts AND
that the binding **discriminates** (negative control fail-closed) — not "Mithril cert
verified" alone. (The uncommitted `.mithril-scratch/` only records the cert-verify link
and is self-described as "strong but not airtight"; it is not item-(c)-grade.)

**No `ci_check_*` gate and no registry flip ship with this slice** — the validator is a
standalone tool (vacuous-green), deliberately *not* wired onto `RO-MITHRIL-IMPORT-01`.
This preserves the no-document-only / no-theater line: the obligation flips only on real
evidence, never on the presence of a schema. **Promotion path** (the future item-(c)
flip slice): capture a real bundle → commit it under `docs/evidence/` →
`ci/validate_mithril_documented_evidence.sh` green → copy the validator body to
`ci/ci_check_mithril_documented_evidence.sh` → append it to `RO-MITHRIL-IMPORT-01.ci_script`
(+ the bundle to `evidence`) → flip `partial → enforced`.

## References

- [[feedback-oracle-seed-then-ade-owns]], [[feedback-mithril-is-peer-infra-not-ade-authority]].
- PHASE4-N-Z (`bootstrap_from_mithril_snapshot`, `verify_mithril_binding`, `DC-MITHRIL-02`);
  PHASE4-N-F-C L2 (`first_run_mithril_bootstrap`, `--mithril-manifest-path`).
- `RO-MITHRIL-IMPORT-01` (item (a) reclassified here; item (c) operator-witnessed),
  `CN-MITHRIL-01`, `CN-SEED-01`, `RO-GENESIS-REPLAY-01`.
- Tooling: `docs/evidence/schemas/mithril-documented-evidence.schema.md`,
  `ci/capture_mithril_documented_evidence.sh`, `ci/validate_mithril_documented_evidence.sh`,
  `ci/build_consensus_inputs_bundle.sh`.
- Mithril docs: bootstrap a Cardano node; mithril-client (`cardano-db --include-ancillary`,
  `utxo-hd snapshot-converter`).
