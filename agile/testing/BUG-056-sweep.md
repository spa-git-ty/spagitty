<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-056 — Manual sweep

**Item:** [BUG-056](../items/BUG-056-review-requests-and-profile-navigation.md)

| Ticket | Preconditions | Steps | Expected result | Priority | Result |
| --- | --- | --- | --- | --- | --- |
| SWEEP-BUG056-01 | Supported repository with pull requests | Visit Review and Pull requests | Same header spacing and repository pill; Review has no scope controls | P1 | Pass, 2026-10-07: production bundle with local IPC fixtures; both headers are 52px tall and show the same pill. Populated screens remain readable at 900 × 650. |
| SWEEP-BUG056-02 | No account for the host | Visit both screens and resize the window | Same neutral state at the window centre; Settings → Accounts works | P1 | Pass, 2026-10-07: both states centred at (640, 360) in a 1280 × 720 viewport; Review stayed centred at approximately (450, 325) at 900 × 650. Accounts recovery opened `/settings#accounts` and displayed Accounts. |
| SWEEP-BUG056-03 | Identity shown in the status strip | Open the profile menu, choose Manage Profiles | Settings → You opens without reloading; the repository stays open | P1 | Pass, 2026-10-07: from Review, the menu opened `/settings#you` and showed Identity Profiles. A document boot marker was unchanged across both Settings links, and the fixture repository tab remained open. |
| SWEEP-BUG056-04 | Host failure or unsupported remote | Visit both screens | Host explanation or unsupported-remote state stays distinct from missing account | P2 | Pass, 2026-10-07: both showed the host error in the error style; an unsupported remote showed its own neutral message without a repository pill. |

The sweep used the production bundle in the Codex in-app browser with deterministic IPC responses, without connecting a real forge account or running a native Tauri backend. The local fixture server is `.spagitty/bug056-qa-server.mjs`; visual evidence is in `.spagitty/bug056-screens/` (both ignored). Native window controls and live forge access were outside this change's sweep.
