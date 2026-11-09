# Ade: migration from legacy IDD to IDSE v3.0

Decided by the human on 2026-11-08. The migration happens all at once: every legacy registry entry gets a human-approved disposition before any new Ade work resumes. Doctrine: `/mnt/store/vadrant/docs/IDSE_Doctrine_v3.0.md`. Method: `~/.claude/methodology/idse.md`.

## Decisions

1. **Attribution exception (Ade only).** Commit messages keep the `Co-Authored-By` trailer, overriding Constitution rule 19 and Doctrine §25. It is recorded in `CLAUDE.md`. PR text, releases, issue comments and source comments still follow §25.
2. **Slice implementation** goes through the `slice-implementer` subagent (idse.md §3 step 4).
3. **All at once.** No new Ade cluster opens until every legacy ID has a disposition, the re-evidence pass is recorded and the grounding documents are regenerated. LV-17 (opened at `e4fa7f00`, not implemented) is paused and re-planned as IDSE work afterwards.
4. **ade-atlas** is redesigned on the IDSE explorer model, with one added page that tracks the bounty requirements.

## What stays true during the migration

- Every legacy CI script and test keeps passing at every migration commit. Moving a file never changes its bytes. Golden files and corpora are moved, never edited.
- No legacy ID is dropped silently. Each one ends with exactly one disposition in `docs/legacy/idd-to-idse-map.toml`:
  - `migrated` to a new `FAM-NNN`
  - `merged` into another legacy ID's new entry
  - `excluded`, with the reason approved in a Define batch
- The legacy registry is frozen at `docs/legacy/idd-invariant-registry.toml` and never edited again (Doctrine §6 rule 8).
- New IDSE documents cite only IDSE IDs. Legacy IDs appear only in the map and under `docs/legacy/`, because the ID lint reads tokens such as the `DET-01` inside `T-DET-01` as invariant references.

## M0: relocation and opt-in (one commit, no human gate)

- Move to `docs/legacy/`:
  - the registry, renamed `idd-invariant-registry.toml`
  - `docs/clusters/`, which becomes `docs/legacy/clusters/`
  - `docs/planning/*`, except this plan
  - `.idd-config.json`, which becomes `idd-config.json`

  `docs/active/` and `docs/evidence/` stay where they are, because IDSE tooling doesn't read them.
- Rewrite every path that CI scripts, tests and interop binaries read to the moved location. Comments that cite moved paths are updated too.
- Add `.idse-config.json` (`profile: strict`, `project: ADE`) and an empty schema-3 registry at `docs/ade-invariant-registry.toml`.
- Add `docs/legacy/README.md` and the map, with all 482 legacy IDs at `disposition = "pending"`.
- **Verify:**
  - every CI script and test whose path changed passes
  - `idse-id-lint.py` and `idse-export.py --check` report no errors

## M1: Define batches (human gate per batch)

The batches are ordered from the foundations up, so that a line's `depends_on` cites only invariants from the same or an earlier batch. Each batch follows idse.md §1:

1. Claude drafts `docs/planning/<slug>.define.md`.
2. Codex reviews it for at most 5 rounds, recorded in `<slug>.define.consensus.md`.
3. The batch goes to the human for line-by-line approval.
4. Approved lines are appended to the registry, and the map is updated.
5. Commit: `Define: <concept> — <IDs>`.

| Batch | Slug | Legacy areas | Legacy IDs |
|---|---|---|---|
| B01 | `foundations` | DET ENC CORE BUILD ERR BOUND META COLL KEY CONSERV NOSPEND CAUSAL TRANSPORT RESOURCE PLATFORM CI, plus global assumptions | 31 |
| B02 | `codec-wire-crypto` | WIRE CBOR INGRESS PREIMAGE CRYPTO KES OPCERT | 36 |
| B03 | `ledger` | LEDGER TXV VAL PLUTUS GOV | 55 |
| B04 | `epoch` | EPOCH | 52 |
| B05 | `consensus` | CONS CONSENSUS SEED | 39 |
| B06 | `consensus-inputs` | CINPUT EVIEW VIEW | 26 |
| B07 | `durability-recovery` | STORE WAL REC SNAPSHOT GENESIS | 40 |
| B08 | `network` | PROTO SESS NET PEER OUTBOUND SYNC FOLLOW COMPAT QUERY | 47 |
| B09 | `node` | NODE | 52 |
| B10 | `pipeline-production` | PUMP ADMIT FORGE PROD | 37 |
| B11 | `mithril-evidence` | MITHRIL ANCHOR REF DIFF EVIDENCE | 19 |
| B12 | `resources-release` | MEM LIVEMEM SERVEMEM OPS REL TEST LIVE OPERATOR CLOSE REHEARSAL | 48 |

**Drafting rules.**
- Each legacy entry becomes one draft line, is merged into one, or becomes an excluded settlement (`S<n>`) with its reason. Exclusions therefore pass through the same human reply as the lines.
- Families follow Doctrine §6, security first.
- Release and operational entries that state a delivery target rather than a falsifiable property are excluded from the registry. They move to the atlas bounty page as criteria.
- Each draft carries `legacy_ids = [...]`.
- After approval, the legacy fields `tests`, `ci_script(s)` and `code_locus` are carried into `tests`, `ci_scripts` and `enforcement_*` loci in the `<path>[::<item>] — <how>` form. These are agent-appendable fields, so carrying them over changes no approved text.

## M2: cutover

- **Retarget the CI scripts that read the legacy registry.** They assert legacy IDs' `status`. Each assertion is retargeted through the map to the new entry: it must exist with `status = "active"`. Their code-level assertions stay unchanged. Each retarget is reviewed: a check must not get weaker (Doctrine §22).
- **Re-evidence pass at one SHA.** Run each new entry's tests and CI scripts and append one `[[rules.evidence]]` record per entry. Claim status:
  - `partial` where the evidence passes. Its `limits` say that no IDSE cluster has declared obligations yet, and, for a critical invariant, that no mutation witness exists yet.
  - `unresolved` where nothing enforces the entry.
  - `falsified` where a check fails.

  Nothing is `supported` until an IDSE cluster declares and meets its obligations (Doctrine §10).
- Regenerate CODEMAP and TRACEABILITY (plus SEAMS and HEAD_DELTAS) in IDSE form, update the crate list in `CLAUDE.md`, and run `/explorer --build`.

## M3: ade-atlas redesign (repository `wdm33/ade-atlas`)

- Retarget the atlas parser to the IDSE sources: the registry, cluster and slice documents, consensus records, CODEMAP and TRACEABILITY.
- Rebuild its pages on the explorer's model: overview, invariants, clusters and slices, modules, checks, dependencies, gaps and zones.
- Add a **Bounty** page. Each acceptance requirement in its user-stated priority order is mapped to IDSE invariant IDs and CE checks, and its status is computed from their claim statuses. The page also shows the commit-attribution ratio.
- Update `notify-atlas.yml` to the new paths.

## M4: resume Ade work

The first IDSE cluster, `C001`, ships the verification infrastructure idse.md §2 requires of a first cluster: the in-repo mutation harness and the state explorer. The replay target already exists. The paused LV-17 recovery work is planned under the new IDs.
