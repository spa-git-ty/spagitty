<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# CodeRabbit for Spagitty

The official extension `spagitty.coderabbit`. It ships with Spagitty, is
**off until you turn it on** for a repository, and reviews your changes with
**your own CodeRabbit CLI** under **your own CodeRabbit account**.

This extension is free software (GPL-3.0-or-later). CodeRabbit's CLI and
service are not: they are CodeRabbit's, installed and signed into by you, and
Spagitty neither bundles nor redistributes them. You do not need Claude Code or
any other agent installed; CodeRabbit's own Claude Code plugin is a separate
thing (<https://docs.coderabbit.ai/cli/claude-code-integration>).

## Setting up

1. **Install the CLI** from <https://docs.coderabbit.ai/cli>. On Windows, use
   the signed native installer (<https://docs.coderabbit.ai/cli/windows>); WSL
   is not needed. Version **0.7.7 or newer** is required.
2. Open **Settings › Extensions › CodeRabbit**. It finds `coderabbit` (or `cr`)
   on your `PATH`; **Choose…** points it at another copy. **Check** shows the
   version it found.
3. **Turn it on for this repository.** You are told, before it is turned on,
   that reviews send the code you choose and its context to CodeRabbit under
   your account and can count against your plan. Turning it on reviews
   nothing.
4. **Sign in** with *Sign in to CodeRabbit* in the command palette. It runs
   `coderabbit auth login --agent --region <us|eu>` with the region you chose,
   which opens CodeRabbit's sign-in in your browser. The CLI keeps its own
   credentials; Spagitty never reads, copies or stores them, and never passes a
   key on a command line.

The setup panel on the card shows the state: CLI not found, too old, not
signed in, ready, not allowed to run its CLI here, or a check that failed.
*Check CodeRabbit sign-in* asks the CLI (`auth status --agent`); *Run CodeRabbit
diagnostics* runs `coderabbit doctor`, which contacts CodeRabbit's servers and
so only runs when you ask. Its report is in the card's Diagnostics.

## Reviewing

**Review changes with CodeRabbit** (Commit screen, or the palette) first shows
what would be reviewed: the base, every file, and what was left out and why.
Nothing is sent until you press **Start review**.

| You choose | Reviewed | CodeRabbit runs |
| --- | --- | --- |
| Uncommitted changes | staged and unstaged edits to tracked files | `coderabbit review --agent --uncommitted` |
| … and untracked files | the same, plus files not added to Git | `… --uncommitted --include-untracked` |
| Committed changes | the commits since the branch left the base you pick | `… --base-commit <merge base> --committed` |
| Committed and uncommitted | both | `… --base-commit <merge base>` |

The base is the merge base of `HEAD` and the branch you pick, pinned before the
review starts, so CodeRabbit reviews exactly what the preview listed. Nothing is
fetched to find a base. Untracked files are only included when you say so, and
your staging is never changed to include one.

While it runs you see what CodeRabbit is doing and how long it has taken, never
a made-up percentage, and **Cancel** ends the CLI and everything it started. If
the code changes while it is being reviewed, the result is marked **out of
date**.

Results show severities mapped from CodeRabbit's own (critical → Critical,
major → High, minor → Medium, trivial → Low, info and none → Info; anything else
→ Unknown, with CodeRabbit's word kept). A finding's location is shown only as
CodeRabbit gave it — no line is ever guessed. Suggestions are shown as text and
never run.

"Reviewed" means the run finished, not that the code is good. Spagitty tells
these apart: a review that found nothing; one with findings; one skipped
because there was nothing to review; one that failed after some findings; one
that missed files; and one waiting on a billing decision. An empty answer or a
successful exit code alone is never treated as a clean review.

## Billing

If your organisation uses on-demand usage-based reviews and you are past your
included reviews, CodeRabbit answers with a request to confirm a paid review.
Spagitty shows that request — files, maximum price — and stops. It **never**
adds `--use-credits`, never retries with credits, and never counts the request
as a review. To pay for that review, run it yourself in a terminal.

## Sending findings to an agent

Select findings and **Send to an agent**. For a review of **committed** changes
whose commit is still `HEAD`, Spagitty adds a **draft** task to the repository's
farm describing each finding, what code was reviewed, and that the findings are
evidence to weigh, not instructions that override the task or the repository's
rules. You ready it in the Farm; the agent works in its own worktree, so your
checkout is never touched. Findings on uncommitted changes cannot be sent:
commit them first, review the commit, then send. "Sent to an agent" never means
"fixed".

## What is stored

- **Review history** in the repository's `.spagitty/extensions/reviews/` (kept
  out of git): what was reviewed, the findings, their dispositions. No file
  contents. Secrets are removed before anything is written. The newest 50 are
  kept; the panel's **Clear** deletes them, and removing the extension offers
  to.
- **Your choices** in Spagitty's data directory: that it is on for this
  repository, its permissions, your consent, the region and the CLI path.

Nothing about CodeRabbit is stored in a repository file that could turn it on.

## Networks and proxies

The CLI connects to CodeRabbit itself; Spagitty does not proxy it. Behind a
corporate proxy or with a custom certificate authority, configure the CLI as
CodeRabbit describes (<https://docs.coderabbit.ai/cli/network-requirements>).
Spagitty never disables certificate checks.

## Troubleshooting

| You see | Do |
| --- | --- |
| "is not installed, or not on PATH" | Install the CLI, open a new Spagitty window, or **Choose…** its path |
| "older than 0.7.7" | `coderabbit update` (or `brew upgrade coderabbit`) |
| "Sign in to CodeRabbit to review" | Run *Sign in to CodeRabbit* |
| "could not read its stored credentials" | Your credential store was unavailable — not signed out. Unlock it and *Check CodeRabbit sign-in* |
| "too many files" with narrower scopes | Pick one of the scopes it lists and review again |
| "Out of date" | You edited, staged or committed during the review. Review again |
| It stopped unexpectedly three times | **Restart** on its card; Diagnostics shows what it said |

## Limits of this version

- Pull request reviews on GitHub are FEAT-099; the farm gate is FEAT-098.
- Paid continuation, API-key sign-in and self-hosted CodeRabbit are not offered.
- The adapter was built from CodeRabbit's published output contract; see
  `fixtures/README.md` for which fixtures are documentation-derived and how to
  capture real ones (`CODERABBIT_SMOKE=1`).
