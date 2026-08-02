# Run-store retention

A live proof run creates an ade data dir (`--data-dir`). Those dirs are **4–7 GB each** and were
never retired, so by 2026-08-02 `~/.cardano-*` held **186 GB** — more than the entire `~/Code`
tree — across ~30 directories, most of them superseded generations of the same experiment
(`s1seed` → `-v3` → `-v4` → `-v5`, `rebootstrap` → `2` → `.bak` → `.bak2`).

## What is actually worth keeping

A run store is **reconstructible**. Its size breaks down as:

| component | share | origin |
|---|---|---|
| `chain.db` | ~85 % | re-fetchable from the peer |
| `reduced-checkpoint.redb` | 733 MB | re-materialisable from the Mithril snapshot (identical across every store) |
| `epoch-accumulator.redb` | 115–162 MB | derived by folding |
| `node.log` | **KB** | **the evidence — not reconstructible** |

All thirteen retired CE-3d / S4 / CE-3c stores together held **56 KB** of `node.log`. The other
73 GB was a chain the peer still has and a snapshot Mithril still serves.

A run is fully reproducible from **(Mithril snapshot cert + binary commit/SHA + peer)**. So the
store is a cache; the log and the provenance are the artifact.

## The rule

**On run close:**

1. Copy `node.log` and any `ref_*` / `*-evidence.json` / parked `*.patch` out of the store.
   - small text (logs, manifests) → `docs/evidence/run-stores/` (in-repo, durable, KB-scale)
   - bulky bundles (`*.tar.gz`, extracted block sets) → `~/.cardano-evidence/run-stores/<name>/`
     (**off-repo** — never commit these; `.git` is already 44 GB from the tracked corpus)
2. Write a `MANIFEST.md` recording: store size at retirement, last-written date, venue/network,
   binary commit + SHA, Mithril snapshot cert, what it proved, and which repo docs reference it.
3. **Then** drop the store.

**Keep a store only while it is an active reproducer**, and say so in its name — e.g.
`KEEP-eview-r1-reproducer`. A store with no `KEEP-` prefix and no repo reference is retirable.

## Do not

- Do not commit multi-GB bundles to the repo. The corpus is already tracked (17,029 files) and is
  why `.git` is 44 GB; every clone pays that.
- Do not delete a store before the manifest exists. The 2026-08-02 EVIEW-R1 store was released
  correctly *because* a forensic record was written first —
  `~/.cardano-live1/KEEP-eview-r1-reproducer/`.
- Do not trust the directory name. `.cardano-ce3d-extract` held **CE-4A/CE-4B** evidence plus a
  parked patch that existed nowhere else. Check contents before retiring.
- Do not assume "unreferenced" means worthless, or "referenced" means needed. Reference-count
  against `docs/` + `ci/` is a *signal*, not the decision.

## 2026-08-02 retirement

Harvested 14 stores → 2.1 MB off-repo + 116 KB in-repo, then dropped:

| dropped | size | basis |
|---|---|---|
| `ce3d-rebootstrap.bak`, `.bak2`, `rebootstrap2` | 17.3 GB | backups/duplicates of a store that still exists; 0 repo references |
| `ce3d-s1seed-v3`, `-v4` | 13.4 GB | superseded by `-v5` (which is referenced); 0 repo references |
| `s4-3c-live`, `ce3d-post1339` | 10.5 GB | 0 repo references |
| `ce3d-extract/db` | 25 GB | consumed source — the corpus it fed is extracted and committed |
| `ce3d-extract/harness-work-{s5,v5}` | 6.8 GB | harness scratch |

Preserved: `ref_1341.tar.gz` (369 MB), `corpus_blocks/` (49 MB), all `ce4a-*`/`ce4b-*` evidence
JSONs, `ce4a-3-r4-parked-fixes-ab-and-harness.patch`, every `node.log`.

**Free space 15 GB → 87 GB.** Retained by reference: `ce3d-s1seed`, `ce3d-s1seed-v5`,
`ce3d-rebootstrap`, `ce3c-firstrun`, `s4-3b-v11`, `s4-1b-v2`.

Next candidate under the same rule (not actioned): `~/.cardano-preview-judge` (43 GB — a 15 GB
`preview-snapshot` plus `fresh-data`, `fresh-data3`, `fresh-data4` at 4.9 GB each).
