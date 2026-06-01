# PHASE4-N-F-G-A — Slice S1b: Private-net genesis reference extraction

> **Status:** slice doc (IDD Part IV). Companion to `cluster.md` (the S1 hard-fork contingency,
> now realized as a prerequisite) and `S1-genesis-consistency-pinning-harness.md`.
> **Re-orders G-A: S1b → S1 → S2 → S3 → S4.** S1b runs **before** S1.
>
> **Why this slice exists (the hard fork resolved).** S1's pinning harness needs a committed
> **Ade-as-leader** genesis-derived reference. The extraction *methodology* already exists and is
> proven (`ci/build_consensus_inputs_bundle.sh`, run against preprod → `docs/evidence/
> phase4-n-m-c-consensus-inputs.json`), but the only committed bundle is **public preprod, where
> Ade is not a pool** — useful as follower/reference data, **not** an Ade-as-leader leadership
> proof. The missing piece is a private net where **Ade's pool is actually in the stake/VRF set**.
> S1b establishes that reference, **offline**, and commits only non-secret artifacts so S1 stays
> hermetic.

## 1. Slice identity
- **Cluster:** PHASE4-N-F-G-A (forge fidelity on the node spine).
- **Slice:** S1b — private-net genesis reference extraction. **Runs before S1.**
- **Module:** an **offline operator/reference-data extraction** (RED, via Docker + cardano-cli) +
  committed **non-secret reference artifacts**. Reuses the existing
  `ci/build_consensus_inputs_bundle.sh` extraction path (the only likely change is its
  preprod-specific stake-scaling, parameterized for the private magic). **No new BLUE authority,
  no new bootstrap authority, no production runtime change.**

## 2. Cluster Exit Criteria — relationship
S1b does **not** itself close a CE-G-A. It **establishes the committed reference fixture that
CE-G-A-1 (S1) consumes** — the Ade-as-leader genesis-derived `{eta0, per-pool active_stake,
total_active_stake, ASC, per-pool vrf_keyhash}` + the minimal private genesis + provenance. Until
S1b lands, S1's pinning tests cannot be written meaningfully. CE-G-A-1 is closed by **S1**, against
this fixture.

## 3. Intent (invariant impact)
Make the S1 genesis-consistency proof **possible and hermetic** by committing an Ade-as-leader,
genesis-derived reference — **without** introducing any runtime authority. The committed fixture
is **evidence input, not runtime authority**: it exists only so S1 can prove that the
WarmStart-recovered seed epoch matches private-genesis-derived reference values. It must **not**
become a production source of `eta0`, stake, ASC, or VRF keyhash (those come only from the
recovered surface via the single bootstrap authority — guard d / CN-NODE-01).

## 4. Scope
**Allowed:**
- Create an isolated private genesis/devnet fixture (`cardano-cli genesis create-staked` or an
  equivalent minimal Shelley/Conway devnet genesis).
- Create/use **Ade-controlled pool keys** for the fixture (cold/VRF/KES) — local only.
- Start an **isolated private-node container** (epoch 0 suffices — the initial `eta0` is queryable
  from a fresh node).
- Run the **existing** consensus-inputs extraction path (`ci/build_consensus_inputs_bundle.sh`,
  pointed at the private node via its `$CONTAINER`/`$MAGIC`/`$SOCKET` parameters; tweak only the
  preprod stake-scaling if needed) to query `protocol-state` (eta0), `stake-distribution`,
  `pool-state`, `protocol-parameters`, `tip`.
- Commit **only non-secret reference artifacts** needed by S1:
  - the private genesis/config fixture (minimal Shelley/Conway genesis JSON),
  - extracted `eta0` (`epoch_nonce_hex`),
  - active stake (per-pool) / total active stake,
  - ASC (`numer`/`denom`),
  - pool VRF keyhash(es),
  - protocol/reference metadata (network magic, era, epoch window, protocol-params hash),
  - provenance notes + the extraction-command transcript.

**Forbidden:**
- Touching the public `cardano-node-preprod` container.
- Committing cold/KES/VRF **private keys** (or any secret material).
- Making any live accept / BA-02 / RO-LIVE claim.
- Adding a new bootstrap authority.
- Replacing Mithril-first recovery.
- Using the fixture as runtime authority.

**Out of scope:** S1's pinning harness/tests (that is S1, against this fixture); any forge / serve
/ live / operator-accept work (G-B/G-C); S2–S4.

## 5. Hard containment (strict)
- **Separate container name** (e.g. `cardano-node-nfg-a-privnet`) — **never** `cardano-node-preprod`.
- **Separate volume / db**, **separate ports**, **separate socket path**.
- A **scratch directory clearly marked private-net-only**, **outside the repo** (private keys live
  there and are never committed).
- Tear down the private-net container after extraction; leave `cardano-node-preprod` untouched
  throughout.

## 6. TCB color (execution boundary)
- **RED (offline reference extraction):** Docker + cardano-cli queries via the existing
  `build_consensus_inputs_bundle.sh` path; the private-net container; any preprod-stake-scaling
  tweak to that script.
- **Reference data (committed, non-secret):** the private genesis fixture + extracted bundle +
  provenance — **not** a runtime surface, **not** authoritative state.
- **BLUE / GREEN production:** **unchanged** (no production code). No new authority of any color.

## 7. Extraction procedure (the implementation)
1. **Isolated keys + genesis** — in a private-net-only scratch dir outside the repo: generate
   Ade-controlled cold/VRF/KES keys (KES via `ade_node --mode key_gen_kes` to match Haskell
   `expand_seed`); `cardano-cli genesis create-staked` (or a minimal devnet genesis) delegating
   ~all stake to the Ade pool; issue the opcert. Keys never leave the scratch dir.
2. **Isolated private node** — start `cardano-node` in a **new** container (distinct
   name/volume/ports/socket) on the private genesis. Confirm it is up and the socket responds;
   `cardano-node-preprod` is not started/stopped/touched.
3. **Extract** — run `build_consensus_inputs_bundle.sh` against the private node (`$CONTAINER` =
   the private container, `$MAGIC` = the private magic, `$SOCKET` = the private socket); capture
   `protocol-state` (eta0), `stake-distribution`, `pool-state --all-stake-pools`,
   `protocol-parameters`, `tip`. Verify the Ade pool appears in `pool_distribution` +
   `pool_vrf_keyhashes` with the expected stake.
4. **Reduce to non-secret reference** — emit the private-net consensus-inputs bundle (same
   `RawConsensusInputs` JSON shape as the preprod one) + the minimal genesis fixture + a provenance
   note (cardano-node/cli versions, the exact query commands, the private magic, a one-line
   statement that keys are uncommitted and the values are genesis-derived).
5. **Tear down** the private node; **commit only** the non-secret artifacts.

## 8. Committed artifacts (non-secret only)
- `crates/ade_testkit/...` (or a committed fixtures path; exact location set at implement) — the
  private-net **consensus-inputs bundle** (`network_magic`=private, `epoch_nonce_hex`=eta0,
  `pool_distribution` incl. the Ade pool, `pool_vrf_keyhashes`, `active_slots_coeff`, `epoch_no`,
  protocol metadata).
- the minimal private genesis fixture (JSON).
- `PROVENANCE` note + the extraction-command transcript (versions, commands, magic; secrets-free).
- **No** private keys, **no** secret material, **no** live-accept/peer-log claim.

## 9. Mechanical acceptance criteria
- [ ] The committed bundle parses as the canonical `RawConsensusInputs` shape and the Ade pool is
      present in `pool_distribution` + `pool_vrf_keyhashes` with non-zero stake.
- [ ] `eta0` (`epoch_nonce_hex`), ASC (`numer`/`denom`), per-pool + total active stake, and the
      pool VRF keyhash are all present and well-formed (hex widths / value ranges).
- [ ] The provenance transcript is committed (cardano-node/cli versions + exact query commands +
      private magic), and contains **no** secret material (CI/grep: no cold/KES/VRF skey bytes,
      no `*.skey` committed).
- [ ] `git status` shows **no** secret files staged; the private-net scratch dir is outside the
      repo and untracked.
- [ ] **Verification target:** the committed fixture is sufficient for the S1 pinning harness to
      run **without** Docker, cardano-cli, or a live node — i.e. S1 reads only committed bytes.
- [ ] `cardano-node-preprod` untouched (still `Up`, unmodified) after S1b.

## 10. Hard prohibitions (inherits cluster Forbidden + S1b-specific)
- No touching `cardano-node-preprod`; isolated container/volume/ports/socket/scratch only.
- No committed private keys / secrets (cold/KES/VRF skey, opcert counter secrets).
- No new bootstrap authority; no replacement of Mithril-first recovery; no runtime production
  change.
- The fixture is **evidence input, not runtime authority** — it must not become a production
  source of eta0/stake/ASC/vrf (§3).
- No live accept / BA-02 / RO-LIVE / peer-acceptance claim; no serve / live-feed work.
- No new BLUE authority/canonical type.

## 11. Explicit non-goals
No S1 pinning harness/tests (that is S1). No forge / serve / live / operator-accept. No S2–S4. No
cross-epoch. No production consumption of the fixture.

## 12. Completion checklist
- [ ] Isolated private net stood up + extracted; `cardano-node-preprod` untouched; private net torn
      down.
- [ ] Non-secret reference artifacts committed (bundle + genesis fixture + provenance transcript);
      **no** secrets staged.
- [ ] The committed fixture is shown sufficient for a hermetic S1 (no Docker/cli/node at S1 time).
- [ ] S1b slice doc committed standalone (`docs:`) **before** the extraction; the fixture +
      provenance committed **separately** after extraction.
- [ ] After S1b lands, resume the original S1 harness against the committed fixture.

## Authority
S1b establishes committed **reference evidence**; it introduces, strengthens, and removes **no
registry invariant**. It preserves `CN-NODE-01` (no new bootstrap authority; Mithril-first recovery
untouched) and `DC-COMPAT-01` (the fixture is reference data, never an internal-state-hash
authority). The cluster doc `cluster.md`, the S1 slice doc, and `docs/ade-invariant-registry.toml`
are authoritative; this slice doc refines, it does not override.
