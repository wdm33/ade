# SLICE LV-9 — the relay-loop containment gate gives the same answer every run

**Kind:** assurance. No node code changes, and no invariant is weakened or reworded.

## 1. The defect

`ci/ci_check_node_run_loop_containment.sh` fences the relay loop body (`run_relay_loop_with_sched`)
for CN-NODE-02, DC-SYNC-02 and DC-NODE-12. It runs under `set -uo pipefail` and checks the body with
pipelines like:

```
if ! echo "$LOOP_FN" | grep -qE 'run_node_sync\('; then print_fail …
```

The body is 51,276 bytes, more than a pipe buffer. When `grep -q` finds its match it exits at once.
If `echo` is still writing, it gets SIGPIPE and exits 141, and `pipefail` makes the whole pipeline
fail. So a match that was found can read as "not found". The positive guards then fail at random.
The negative guards are not affected: when nothing matches, `grep` reads to the end.

Measured in LV-8: 4 of 6 runs failed on the LV-8 tree and 7 of 8 on the pre-LV-8 file. Each failure
was the positive `run_node_sync(` guard, and the loop body is the same size in both.

## 2. The repair

Every check reads the body from a here-string (`grep -qE '…' <<<"$LOOP_FN"`), with no pipe. The
patterns, the scope (the stripped body of `run_relay_loop_with_sched`), the guards and the messages
stay the same. Only how the text reaches `grep` changes.

## 3. MECHANICAL ACCEPTANCE CRITERIA

| CE | Criterion | Mechanism |
|---|---|---|
| **CE-LV9-1** | No `echo … \| grep` pipeline is left in the gate | grep of the script |
| **CE-LV9-2** | On the unchanged tree, the repaired gate passes 50 of 50 runs | repeated run |
| **CE-LV9-3** | The gate still fails, every time and with the intended message, for each of: the `run_node_sync(` call removed from the loop body; a direct `pump_block(` call added to it; a second `forge_one_from_recovered(` call added to it. Each mutation is asserted applied and the tree restored byte-exact | mutation runs, 10 runs each |
| **CE-LV9-4** | A full CI sweep, recorded as the new baseline | sweep |

## 4. OUT OF SCOPE

Other `ci/` scripts use the same `echo "$X" | grep -q` shape. They are counted in §5 but not changed
here. Any of them that reads a large input can flake the same way.

## 5. RESULTS

Not yet run.
