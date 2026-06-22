# SLICE — BOOTSTRAP-CERTSTATE-PRODUCER

## Intent
Produce a complete, manifest-bound, self-inspecting bootstrap package from a **pinned**
Preview source point, in ONE deterministic command. The reconciliation audit (2026-06-22)
proved the import pipeline is correct (`import_bootstrap_cert_state`, commit `bd8b0def` /
DC-EVIEW-09: sibling `<bundle>.manifest` + `<bundle>.certstate`, manifest-bound, fail-closed,
populates `ledger.cert_state`) — the gap is that no producer guarantees the siblings exist, so
every live bundle hit the `(false,false)` empty-CertState branch. This slice closes that gap on
the PRODUCTION side only; the importer's sibling convention is preserved (good for usability).

This is NOT the ECA-5 live run. It is the prerequisite that makes ECA-5 judgeable rather than
operator-dependent.

## Status (2026-06-22)
**Live export/import-gate verified on Preview.** `ade_node --mode bootstrap_export --network preview
--output ade-inputs` emitted all four mutually-bound artifacts against the live Preview node, and the
EXISTING importer self-verified the package (`.inspect.json` `binding_verdict: "bound"`; 704 pools all
VRF-complete, 60329 delegations, 90099 rewards, 0 future, 1 retiring, at pinned slot 115455953; profile
`preview`). The live run was the arbiter: two defects the hermetic fixtures missed are fixed +
regression-covered — (1) a margin rendered in scientific notation (`1.0E-7`); (2) immutable-tip drift
across the ~15s multi-query capture (a bounded deterministic retry; a drift is a clean no-emit).

**Not yet release-complete** — the separate **BOOTSTRAP-CERTSTATE-ROUNDTRIP** gate, run on the
UNCHANGED committed binary + the already-emitted package:
- clean admission → warm-start persisted-state round-trip (the persisted `CertState` == the emitted
  `certstate_hash` + manifest bindings)
- oracle comparison for margin, reward account, and protocol-parameter hash vs cardano-node
- ECA-5 boundary continuity proof

Splitting the gate keeps the evidence clean: PRODUCER = the package can be generated, bound, and
importer-verified; ROUNDTRIP = the package is admitted, persisted, warm-recovered, and semantically
compared.

## Command contract (judge-facing)
```
ade_node --mode bootstrap_export \
  --network preview \
  --output ade-inputs-ep1336
```
The pinned point is the node's CURRENT immutable tip — cardano-cli cannot scope to an arbitrary
historical point, so the producer captures under the `--immutable-tip` contract + a Q==P drift check
(bounded deterministic retry) and emits the point into the manifest. `--network` resolves a committed,
CLOSED `NetworkProfile` (magic, Shelley genesis hash, ASC, epoch length); an unknown network fails
closed, and the live node's magic + epoch are verified against the profile before export. Emits,
atomically and fail-closed (unless ALL FOUR are produced and mutually bound):
- `ade-inputs-ep1335.json`            — the consensus-inputs bundle (existing format)
- `ade-inputs-ep1335.json.certstate`  — the canonical Ade `CertState` (full lifecycle)
- `ade-inputs-ep1335.json.manifest`   — the binding manifest
- `ade-inputs-ep1335.json.inspect.json` — the release-artifact inspection report

The subsequent judge-facing bootstrap stays simple — NO extra cert-state flags:
```
ade_node --mode admission --consensus-inputs-path ade-inputs-ep1335.json
```

## Producer contract (the 7 steps, in order)
1. Canonical cardano-node source at ONE pinned IMMUTABLE point. EVERY query targets `--immutable-tip`
   (never the volatile default, which can advance beyond the immutable tip); P is re-resolved after the
   capture and the package is accepted ONLY if Q == P (a drift → clean no-emit + bounded retry). The
   pinned point is emitted into the manifest so two judge reruns from the same point build the SAME world.
2. Extract the full `pstate` / `ssPoolParams` lifecycle state + delegations + rewards
   (`cardano-cli query pool-state` = pools incl. VRF + future_pools + retiring; `query
   ledger-state` / dstate = delegations + rewards; `query stake-snapshot` = stake; `protocol-state`
   = nonce; `protocol-parameters`).
3. Encode the Ade `CertState` canonically (`encode_cert_state`: delegations / rewards / pools /
   retiring / future_pools).
4. Emit the manifest binding (`BootstrapManifest { network_magic, seed_hash = bundle bytes hash,
   cert_state_hash = certstate bytes hash, source_commitment = era/profile/source-point commitment }`,
   `canonical_bytes()`).
5. Immediately DECODE/INSPECT the emitted certstate (postcondition of the SAME tool, not a separate
   slice): `verify_and_import_cert_state(manifest, seed, cert, magic, era)` must succeed.
6. Verify semantic counts and VRF consistency (every delegated-and-registered pool used by the seed
   distribution has an effective VRF key; future-pool/retirement decode canonically; artifact
   network/profile == the bundle's; no duplicate/malformed/unbound cert-state).
7. Write the inspection report as a RELEASE ARTIFACT (`.inspect.json`), not console output.

## Inspection report (`.inspect.json`) — the judge's audit surface
`source_point`, `bundle_hash`, `certstate_hash`, `manifest_hash`, `active_pool_count`,
`future_pool_count`, `retiring_count`, `delegation_count`, `reward_count`, `vrf_count`,
`binding_verdict`.

## Classification
- **True:** bootstrap recovery consumes only the bound durable artifacts.
- **Derived:** extraction must reproduce Cardano pool/delegation lifecycle state at the pinned point.
- **Release:** the export command and the subsequent admission bootstrap must pass the
  inspect-and-bind checks.
- **Operational:** live Preview access is used ONLY to capture the source state; once emitted, the
  bundle is portable and reproducible (a clean machine needs no live access).

## Acceptance
A clean machine receives the four files, runs one admission command, warm-starts WITHOUT external
cert-state input, and decodes the resulting store as carrying the SAME bound `CertState` (counts +
hashes match the `.inspect.json`).

## Grounded design (primitives that exist vs new logic)
EXIST: `encode_cert_state` / `decode_cert_state` (cert_state.rs, the full codec incl. future_pools
ECA-0a); `BootstrapManifest` + `canonical_bytes()` + `verify_and_import_cert_state`
(bootstrap_manifest.rs); the consensus-inputs bundle format (the existing cardano-cli + jq
extraction: `pool-state` is already queried, VRF projected out).
NEW: the `ade bootstrap-export` command (a new `--mode` / subcommand); the cardano-cli
`pool-state` + `ledger-state` JSON -> `CertState` parser (pools incl. `vrf_hash`, `future_pools`,
`retiring`, `delegations`, `rewards`); the tip==source-point pin check; the self-inspection +
`.inspect.json` writer; fail-closed-unless-all-four-bound.

## Implementation plan (order)
1. Capture sample cardano-cli `pool-state` + `ledger-state` JSON from the Preview node at a pinned
   point -> test fixtures (grounds the parser; the only live-dependent step).
2. The `CertState` parser (cardano-cli JSON -> `CertState`), hermetically tested against the
   fixtures (Derived: matches Cardano lifecycle semantics).
3. The manifest builder + the bundle hash binding + the self-inspect (decode + counts + VRF +
   verdict) + the `.inspect.json` writer — hermetic.
4. The `ade bootstrap-export` command orchestrating cardano-cli + parse + encode + bind + inspect +
   atomic-write-all-four / fail-closed (Operational).
5. Acceptance: clean-machine round-trip (4 files -> admission -> populated cert_state == inspect).
