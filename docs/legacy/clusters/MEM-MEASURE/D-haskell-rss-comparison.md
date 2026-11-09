# Slice MEM-COMPARE-D — Haskell-vs-Ade RSS comparison (BA-08)

### Cluster
MEM-MEASURE. Addresses **CE-MM-6** (the BA-08 side-by-side comparison).

### Status
In Progress — the comparison artifact. The memory optimization that *closes* the gap is the bounty-winning follow-on (a separate, BLUE/storage-touching effort), explicitly out of scope for this slice.

### Cluster Exit Criteria Addressed
- [ ] CE-MM-6 (`MEM-COMPARE-D`): a committed side-by-side Haskell-vs-Ade RSS comparison at the same preprod venue.

### Slice Dependencies
- MEM-MEASURE-A2 (`c54edb93`) — the committed Ade RSS reference (`docs/evidence/mem-measure-a2-preprod-memory.jsonl`, `memory_summary` p50/p95/peak).

---

## 3. Implementation Instruction (AI)
Commit a reproducible, sha256-bound side-by-side RSS comparison of Ade vs the Haskell `cardano-node-preprod` at the same preprod chain, establishing the BA-08 measurement methodology and recording the **current gap honestly** (Ade is heavier). This slice changes NO Ade code — it is offline measurement evidence. The optimization to win BA-08 is a follow-on. Commit carries this repo's `Co-Authored-By` trailer.

---

## 4. Intent
Establish the BA-08 measurement contract — *Ade's process RSS vs the Haskell node's, same venue, same chain* — as a committed, reproducible artifact, and record the current standing without spin.

---

## 5. Scope
- **New:** `docs/evidence/mem-compare-d-preprod.{jsonl,md}` (the comparison artifact), `ci/ci_check_mem_compare_evidence.sh` (vacuous-until-committed gate), `docs/clusters/MEM-MEASURE/cluster.md` (CE-MM-6 note).
- **Ade code:** none. The Haskell baseline is sampled offline (`/proc/<pid>/status` `VmRSS` of the `cardano-node-preprod` container); the Ade figure is read from the committed A2 transcript.
- **Out of scope:** the memory optimization that closes the gap (the UTxO representation — a separate slice/track); a true 10-day sustained average (the bounty's actual test — this slice establishes the methodology + a representative snapshot).

---

## 6. Execution Boundary
- **BLUE:** none.
- **GREEN:** the comparison gate (deterministic validation of the artifact).
- **RED:** the offline RSS sampling of the external Haskell node (`docker inspect` + `/proc`). Observes a foreign process; influences no Ade output.

---

## 7. Invariants Preserved
- Ade is unchanged (no runtime/authority impact). The comparison is pure external measurement.
- Evidence honesty (`feedback_shell_must_not_overstate_semantic_truth`): the artifact records that Ade currently **loses** BA-08; no favorable spin.

---

## 8. Invariants Strengthened / Introduced
- A reproducible BA-08 comparison methodology + a committed artifact (CE-MM-6). It does NOT flip a registry rule to a winning state — Ade does not yet match/beat Haskell.

---

## 9. Design Summary
Sample the Haskell `cardano-node-preprod` process `VmRSS` over a short window (nearest-rank p50/p95/peak, mirroring the A1 `RssWindow` math); read the committed Ade A2 `memory_summary`; emit a `mem-compare-d-preprod.jsonl` of the Haskell samples + a `comparison_summary` line `{ade_rss_kib, haskell_p50/p95/peak_kib, gap_kib, gap_pct, verdict}`; bind its sha256 in the `.md` with the methodology + the honest conclusion. The gate validates structure + binding.

**Current result:** Ade `--mode admission` preprod follow = 6.56 GB; Haskell `cardano-node-preprod` (full node) = 5.50 GB. **Ade is ~1 GB / ~19% heavier**, and was doing *less* work (admission-follow, not a full node) — so the efficiency gap is larger than the raw delta. The dominant Ade cost is holding the full preprod UTxO (~3.8 GB seed) as a fully-parsed in-memory map; Haskell's UTxO is far leaner (compact/binary, or on-disk UTxO-HD).

---

## 12. Mechanical Acceptance Criteria
- [ ] `docs/evidence/mem-compare-d-preprod.jsonl` — ≥6 Haskell `VmRSS` samples + one `comparison_summary` with `ade_rss_kib`, `haskell_*_kib`, `gap_kib`, `gap_pct`, `verdict`.
- [ ] `docs/evidence/mem-compare-d-preprod.md` — methodology + table + the honest conclusion + the `.jsonl` sha256.
- [ ] `ci/ci_check_mem_compare_evidence.sh` — `--self-test` green; default validates the committed artifact (sha256-bound, comparison_summary present).

---

## 14. Hard Prohibitions
- No Ade code change; no registry rule flipped to a winning/enforced state (Ade loses — do not overstate).
- No favorable framing of the gap; the verdict field is mechanical (`ade_heavier` when `ade_rss > haskell_peak`).

---

## 15. Explicit Non-Goals
- The memory optimization (UTxO representation) — the winning follow-on, separate slice.
- A 10-day sustained average — the bounty's actual test; this slice is the methodology + snapshot.
- A perfectly concurrent / equal-workload comparison (Ade admission-follow vs Haskell full-node is noted as a caveat).

---

## 17. Review Notes
- Follow-up implied: profile Ade (heaptrack/massif) to confirm the full-UTxO map is the lever, then the optimization to shave ≥1 GB and flip the verdict.
