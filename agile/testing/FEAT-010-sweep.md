<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-010 — Manual sweep

> **Refreshed 2026-10-19 (TASK-073).** This sweep checked Pull requests before it could reach a host. FEAT-017 connected accounts, so most of it is obsolete; the rail and the way to Settings still apply.

Test tickets for the Pull requests screen (1H).

**What this screen is in this pass.** The layout and an honest empty state. No
account can be connected and no request can be fetched, because Spagitty contains
no network client. Most of these tickets are therefore about what the screen
*says* and what it *does not do*.

| Field | Meaning |
| --- | --- |
| Priority | P1 blocks the item, P2 should be fixed before release, P3 cosmetic |
| Result | left blank for the tester: pass / fail |

---

### SWEEP-1H-01 — The rail reaches a real screen

- **Priority:** P1
- **Steps:** Press **Pull requests** in the nav rail.
- **Expected:** A screen with a header, a body and a footer — not the dashed
  "Not built yet" card the other unbuilt screens show. Acceptance criterion 5.
- **Result:**

### SWEEP-1H-02 — The empty state says what is actually missing

- **Priority:** P1
- **Steps:** Read the body.
- **Expected:** It says no account is connected, explains that Spagitty reads
  pull requests from whichever service hosts the repository, and says connecting
  one is not built yet. It does **not** say "not built yet" about the screen —
  the screen works; the account does not. Acceptance criterion 1.
- **Result:** Obsolete, 2026-10-19 — FEAT-017 connected accounts, and Pull requests reads from the host; the screen this checked, with no network client, is gone.

### SWEEP-1H-03 — The way to Settings → Accounts

- **Priority:** P2
- **Steps:** Press **Settings → Accounts**.
- **Expected:** The Settings screen opens **on the Accounts section**, since
  FEAT-011 has landed. The address ends `#accounts`.
- **Result:**

### SWEEP-1H-04 — No host is named anywhere

- **Priority:** P1
- **Steps:** Read every word on the screen, including button titles on hover
  and the footer.
- **Expected:** "Pull request", "account", "hosting service" — and no brand:
  no GitHub, GitLab, Bitbucket, Gitea, Forgejo or Azure DevOps. Acceptance
  criterion 4.
- **Result:** Obsolete, 2026-10-19 — FEAT-017 connected accounts, and Pull requests reads from the host; the screen this checked, with no network client, is gone.

### SWEEP-1H-05 — The layout is the design's

- **Priority:** P2
- **Preconditions:** None available in this pass — the list cannot be
  populated. Read the layout from the code review or from the automated tests'
  sample data instead, and mark this ticket **blocked** rather than failed.
- **Steps:** Confirm the intended shape: "Needs you" above "Waiting on others",
  the second dashed, detail panel to the side.
- **Expected:** As above. Re-run this ticket for real under FEAT-017.
  Acceptance criterion 3.
- **Result:** Obsolete, 2026-10-19 — FEAT-017 connected accounts, and Pull requests reads from the host; the screen this checked, with no network client, is gone.

### SWEEP-1H-06 — The footer's promise

- **Priority:** P1
- **Steps:** Read the footer.
- **Expected:** It states plainly that Spagitty does not talk to any hosting
  service, that there is no network client in the build, and that this screen
  makes no request.
- **Result:** Obsolete, 2026-10-19 — FEAT-017 connected accounts, and Pull requests reads from the host; the screen this checked, with no network client, is gone.

### SWEEP-1H-07 — It works with networking off

- **Priority:** P1
- **Steps:** Disable networking entirely on the machine (turn off Wi-Fi and
  unplug, or `sudo ip link set <iface> down`). Start Spagitty, open a repository,
  and use the Pull requests screen.
- **Expected:** Identical behaviour. No spinner, no timeout, no error, no delay.
  Acceptance criterion 2. Re-enable networking afterwards.
- **Result:** Obsolete, 2026-10-19 — FEAT-017 connected accounts, and Pull requests reads from the host; the screen this checked, with no network client, is gone.

### SWEEP-1H-08 — Nothing leaves the machine

- **Priority:** P1
- **Steps:** With a packet capture running (`sudo tcpdump -i any -n 'tcp port
  80 or tcp port 443'`), start Spagitty, open a repository, and visit the Pull
  requests screen several times.
- **Expected:** No traffic from the Spagitty process at all. This is the ticket
  that actually proves the non-scope; the automated test proves only that no
  client is linked in.
- **Result:** Obsolete, 2026-10-19 — FEAT-017 connected accounts, and Pull requests reads from the host; the screen this checked, with no network client, is gone.

### SWEEP-1H-09 — The screen survives being visited repeatedly

- **Priority:** P3
- **Steps:** Move between Pull requests and other screens ten times.
- **Expected:** The same empty state every time, no accumulation, no flicker of
  a list that is not there.
- **Result:** Obsolete, 2026-10-19 — FEAT-017 connected accounts, and Pull requests reads from the host; the screen this checked, with no network client, is gone.
