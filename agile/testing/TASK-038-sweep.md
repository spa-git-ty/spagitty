<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-038 — Manual sweep

**Item:** [`agile/items/TASK-038-settings-that-stop-lecturing.md`](../items/TASK-038-settings-that-stop-lecturing.md)

This task is a judgement about reading, so the sweep is a person reading. The
tickets are arranged so that the first two answer "did it work" and the rest
answer "did it take something with it" — which is the way this change fails.

| Ticket | Preconditions | Steps | Expected result | Priority | Pass/Fail |
| --- | --- | --- | --- | --- | --- |
| SWEEP-001 | Any build from this branch | Open Settings and visit every section without scrolling more than the controls require | Each section fits its controls. No section opens with a sentence restating its heading. Nothing has to be read past to reach a control. | High | |
| SWEEP-002 | The same | Pick a setting to change — say, turning the command log on — and time how long it takes to find it | Immediately. This is the complaint the task exists for. | High | |
| SWEEP-003 | The same | Hover every toggle chip in Behaviour, and the Clear buttons under You and Signing | Each shows the detail that used to be printed under it. Nothing that was moved to a hover has been lost on the way. | High | |
| SWEEP-004 | An account connected under You → Accounts | Read the paragraph under the Connect row | It still says: repositories are never uploaded; the request goes to the host you named with the token you issued; it reads and never writes; the token is in the keychain and disconnecting deletes it; the update check is the only other request. **All five.** This paragraph is four merged into one and is the ticket most likely to have lost something. | High | |
| SWEEP-005 | No account connected | Read the token instructions | The required scopes are still named — fine-grained needs Pull requests and Metadata, classic needs `repo` — and Enterprise hosts are still mentioned. Without these a person has to leave for the host's documentation. | High | |
| SWEEP-006 | A repository open | You → set the scope to "this repository", change the name, save. Then Clear it | The "In effect" line under the field still names where the live value comes from, and changes when the scope changes. This is what replaced the closing paragraph about `git config`. | Medium | |
| SWEEP-007 | Signing on, in the global config, with no local override | Settings → You → Signing, scope "this repository" | It says "This repository holds nothing", and Clear is disabled. Then switch to global: it says "Global holds true", and Clear is enabled. | Medium | |
| SWEEP-008 | Any build | Appearance → drag Text, then drag Zoom | Both say in a few words what they scale, and the Zoom row still names `Ctrl` `+`, `−`, `0`. The shortcut is the one thing on this screen a control cannot demonstrate. | Medium | |
| SWEEP-009 | Any build | Behaviour → look at "Ask before rewriting history" | It still says it is not honoured yet. A switch that silently does nothing is worse than one that admits it. | Medium | |
| SWEEP-010 | A narrow window, and the interface zoomed to 150% | Visit every section | Nothing overlaps or is clipped. Several notes moved from their own paragraph onto a control's row, and a row with one more thing on it is a row that can wrap badly. | Medium | |

## Negative paths this sweep deliberately covers

- **SWEEP-004 and SWEEP-005** are the two places where deleting the wrong
  sentence has a real cost: one is the privacy statement, the other is the only
  instructions on the screen.
- **SWEEP-003** is the whole premise. If the hovers do not work, this was not a
  trim, it was a deletion.
- **SWEEP-010** is the mechanical risk. The trim moved text onto rows that were
  laid out for fewer things.
