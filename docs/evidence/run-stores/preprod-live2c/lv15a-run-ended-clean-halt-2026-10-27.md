# LV-15a live run — ended with an unexplained clean halt on 2026-10-27

preprod. Store: `~/.cardano-live1/ade-preprod-lv11`. Binary: `bin-lv15a` (code `bbde0036`). Log:
`~/.cardano-live1/lv15a-ce10.stdout`. Recorded 2026-11-02. Cause: **unknown**.

## Facts

- The run restarted at 137,174,181 (CE-LV15a-10) and followed at the peer tip.
- Its last durable tip was **137,461,307** (block 5,343,225), at about 23:43 UTC on 2026-10-27.
  - That is 1,093 slots before the 321→322 boundary at 137,462,400.
  - The previous boundary had been crossed on this run without incident.
- The last iteration's sync step took **93,698 ms**. Earlier iterations took about 16,000–18,000 ms.
- The loop's next step was `loop_step=HaltCleanly` with `sync_status=WorkAvailable`, followed by
  `relay run loop exited (… followed the live peer until shutdown / feed-end)`.
- No failure line, panic, terminal or refusal appears anywhere in the log. Nothing in the session sent a
  signal.
- The peer container has been up since 2026-10-10, with no restarts. Its docker log holds nothing for
  that window (the retained log has no lines after 2026-08-17 except the most recent), so the peer side
  could not be checked.
- **Restart crosses.** On 2026-11-02 a copy of this store was restarted with the same binary. It crossed
  321→322 at 137,462,401 with no failure line (LV-15b-0 census, capture 322). The halt is therefore not a
  deterministic failure of that boundary.

## Status

This is the second clean stop with no explanation, after LV-13's. It stays recorded as unknown. The
reports to the owner on 2026-10-31 and 2026-11-01 saying the node was still running were wrong. The check
counted failure lines, and `pgrep -f lv15a` matched its own command line rather than the node.

Shutdown is also slow. SIGTERM did not stop a catching-up node within 30 s during the census runs. That
falls under the existing SIGINT operational debt.
