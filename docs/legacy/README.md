# Legacy IDD record

Ade was developed under the legacy IDD methodology until 2026-11-08, when it moved to IDSE v3.0. The migration plan is `docs/planning/idse-migration-plan.md`.

Everything in this directory is a frozen historical record. It is never edited, except to fill in the disposition map below.

| Path | What it is |
|---|---|
| `idd-invariant-registry.toml` | The legacy registry: 482 entries, with IDs such as `T-DET-01`, `DC-NODE-48` and `CN-WIRE-12` |
| `idd-to-idse-map.toml` | One record per legacy ID: its Define batch, its disposition (`pending`, `migrated`, `merged` or `excluded`), the IDSE IDs it maps to, and the approving record |
| `clusters/` | Legacy cluster and slice documents. Closed ones are under `clusters/completed/` |
| `planning/` | Legacy invariant sketches and plans |
| `idd-config.json` | The legacy project configuration |

New IDSE documents cite IDSE IDs (`FAM-NNN`, `CNNN`, `CNNN-SNN`) only. To follow a legacy ID forward, look it up in the map.
