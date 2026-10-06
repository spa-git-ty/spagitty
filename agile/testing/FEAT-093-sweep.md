<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-093 — Manual sweep

**Item:** [`agile/items/FEAT-093-threads-done-properly.md`](../items/FEAT-093-threads-done-properly.md)

| Ticket | Preconditions | Steps | Expected result | Priority | Result |
| --- | --- | --- | --- | --- | --- |
| SWEEP-FEAT093-01 | — | 1. The room with the handoff's sample, backend answered by fixtures 2. A pending comment, a range being written, Finish review | Warm pending card under its line, the range tinted with its box under the last line, the Finish card with the count | P1 | Pass, 2026-10-04, dev server in headless Chrome |
| SWEEP-FEAT093-02 | A GitHub pull request | 1. Comment on a line and on a range across a removed and an added line 2. Quit and reopen Spagitty 3. Finish review → Comment | Both comments still pending after the restart; on GitHub, one review with both, the range as a range | P1 | |
| SWEEP-FEAT093-03 | The author's work GitLab | 1. As 02 | One batch of notes; the range placed on the right lines | P1 | |
| SWEEP-FEAT093-04 | A thread on GitHub | 1. Resolve in Spagitty 2. Look on GitHub 3. Reopen on GitHub, Refresh | Resolved there; open again here | P1 | |
| SWEEP-FEAT093-05 | A discussion on GitLab | 1. As 04 | The same | P1 | |
| SWEEP-FEAT093-06 | As 02, then the author pushes | 1. Open the pull request | The pending comment is listed as written before the last push, not on a line | P2 | |
| SWEEP-FEAT093-07 | A comment on the pull request as a whole on each host | 1. Open it | Listed as *whole PR* | P2 | |
