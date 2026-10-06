<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-097 — Manual sweep

**Item:** [`agile/items/FEAT-097-coderabbit-reviews-local-changes.md`](../items/FEAT-097-coderabbit-reviews-local-changes.md)

These need the real CodeRabbit CLI and a CodeRabbit account. Reviews in 03–08
send code to CodeRabbit and may use the account's allowance; use a scratch
repository.

## Sweep tickets

| Ticket | Preconditions | Steps | Expected result | Priority | Result |
| --- | --- | --- | --- | --- | --- |
| SWEEP-FEAT097-01 | A release build; no CodeRabbit CLI installed | 1. Settings › Extensions › CodeRabbit<br>2. Turn it on; open the palette | It is listed as Official and off; turning it on asks for consent naming CodeRabbit; *Review changes with CodeRabbit* is greyed with "not installed, or not on PATH" | P1 | |
| SWEEP-FEAT097-02 | CLI 0.7.7+ installed, signed out | 1. **Check** on the card<br>2. *Check CodeRabbit sign-in*<br>3. *Sign in to CodeRabbit* with region EU | The version shows; sign-in says not signed in; the browser opens CodeRabbit's EU sign-in; after it, the setup panel says signed in, region EU | P1 | |
| SWEEP-FEAT097-03 | Signed in; a scratch repository with staged, unstaged and untracked changes | 1. *Review changes with CodeRabbit*<br>2. Read the scope dialog<br>3. Switch to "… and untracked files" | The dialog lists staged and unstaged files and names the untracked ones as left out; switching includes them; nothing was sent before Start | P1 | |
| SWEEP-FEAT097-04 | 03 | 1. Start review<br>2. Watch the panel | Progress and elapsed time update; findings appear as they arrive; the result names the CLI version; locations appear only where CodeRabbit gave them | P1 | |
| SWEEP-FEAT097-05 | 03 | 1. Start a review<br>2. Cancel within ten seconds | The panel says cancelled; Task Manager / `ps` shows no `coderabbit` process left | P1 | |
| SWEEP-FEAT097-06 | 03 | 1. Start a review<br>2. Edit a file in scope before it finishes | The result is "Out of date" | P1 | |
| SWEEP-FEAT097-07 | A branch with commits | 1. Choose "Committed changes" against `main` | The dialog shows the base and the commits' files; the review's command in Diagnostics has `--base-commit` and the merge-base id | P2 | |
| SWEEP-FEAT097-08 | An account past its included reviews with on-demand billing | 1. Review | "Needs your action" with the file count and price; nothing was charged; no retry happened | P2 | |
| SWEEP-FEAT097-09 | A committed review with findings, commit still HEAD, a farm created | 1. Select two findings<br>2. Send to an agent | A draft task appears in the Farm describing both, naming the reviewed commits, rules first; the findings say "sent to an agent"; the checkout is unchanged | P1 | |
| SWEEP-FEAT097-10 | An uncommitted review with findings | 1. Send to an agent | Refused: commit first; nothing is created | P1 | |
| SWEEP-FEAT097-11 | — | 1. Kill the worker process from Task Manager during a review | The review fails with "stopped unexpectedly"; the window stays responsive; the next review starts it again | P2 | |
| SWEEP-FEAT097-12 | — | 1. Uninstall nothing; turn CodeRabbit off for the repository | Its actions and panels leave Commit and the palette at once; the CLI is still installed | P2 | |
