<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-044 — Manual sweep

**Item:** [`agile/items/TASK-044-settings-stops-explaining-itself.md`](../items/TASK-044-settings-stops-explaining-itself.md)

| Ticket | Steps | Expected result | Priority | Pass/Fail |
| --- | --- | --- | --- | --- |
| SWEEP-001 | Open Settings and read every tab without hovering anything | Nothing reads as a lecture. The judgement the author asked for: terse is the failure mode of this change, and if a tab now reads as curt rather than calm, say which. | High | |
| SWEEP-002 | Settings → Remotes, on a repository with an account | Hover the two lines under the token field | The scopes are visible without hovering. The privacy claim is one line, and the hover carries the rest — host you named, never approves or merges, the update check. | High | |
| SWEEP-003 | Settings → License | The licence line, the "what is linked into this binary" line and the undeclared count are each one line, and each hover carries what was cut. | Medium | |
| SWEEP-004 | Settings → Behaviour, with updates on | `No account, no identifier.` is visible; the hover says what the request is and that turning it off stops it. | Medium | |
| SWEEP-005 | Settings → God mode | The one-line notes under each heading are still there. They were kept deliberately: each says what a debug control does that it cannot show. | Low | |
| SWEEP-006 | Any tab, at 130% text | Nothing wraps oddly now the lines are short. | Low | |
