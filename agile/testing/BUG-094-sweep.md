<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-094 — Sweep

**Item:** [BUG-094](../items/BUG-094-a-resumed-agent-skips-files-that-changed-since.md)

| Ticket | Preconditions | Steps | Expected result | Result |
| --- | --- | --- | --- | --- |
| SWEEP-BUG094-01 | A pull request; an agent assigned to review it | Stop the agent after a file or two, push a commit to the pull request, press **Resume** | A fresh run starts at the new head, reviews every file, and its note says the earlier run moved | Unit tests; desktop check not run |
| SWEEP-BUG094-02 | Same, with nothing pushed | Stop, then **Resume** | It carries on and skips the files it finished | Unit tests; desktop check not run |
| SWEEP-BUG094-03 | Merger with an agent on a merge | Stop it, commit to one branch, **Resume** | It starts over on the new tip | Unit tests; desktop check not run |
