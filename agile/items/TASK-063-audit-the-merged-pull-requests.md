<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-063 — Audit the pull requests merged since 2.0.0

**Status:** Partial — audited and filed, and the attribution check is in CI (TASK-064); waiting on the author's decision about the history already on `main` (2026-10-12).
**Screens:** —.
**Raised by:** the author, 2026-10-11: check the pull requests the author and Codex merged, for code style and correctness.

## Scope

| PR | What it is |
| --- | --- |
| #52 | Spagitty 2.0.0: agents in Review and Merger (FEAT-115), about 17,500 lines. Most of BUG-070 and TASK-060, and FEAT-118 and FEAT-119, start here. |
| #53 | BUG-059 — the Windows watcher and refs |
| #54 | BUG-060 — the Merger forecast on first open |
| #55 | BUG-061 — the Merger plan for the right pair |
| #56 | BUG-062 — back to the list after a merge |
| #58 | BUG-064 — a Review group for pull requests you reviewed |
| #61 | Restore release checks and prepare 2.0.1 (TASK-058) |

## Already found

**Attribution trailers on `main`.** Since 2026-10-01:

- 127 commit-message lines on `main` are `Co-Authored-By:` or `Claude-Session:` trailers, on 77 commits;
- 29 commits are authored as *Claude*;
- the descriptions of #52, #55, #56, #57 and #58 carry a *Generated with* line, and the review replies on #52 end with one.

`AGENTS.md` forbids all of it. Removing it from commits means rewriting history on a protected branch, which the author decides. The proposal is to leave history alone and stop new ones, which TASK-064 does.

## Verdicts, 2026-10-12

How this was done: each diff was read against `AGENTS.md` and the code around it, the tests were read for whether they would fail without the change, and the records and changelog entries were checked. For #52, which is too large to read line by line, the modules where a mistake costs the most were read in full — what an agent may write (`guard.rs`), what a remote agent may read (`remote.rs`, `world.rs`), the network client (`models/http.rs`), the prompt's framing of what it reads, and the front end for `{@html}` — and every CodeRabbit finding was checked against the code as it stands now. The Svelte screens and the engine's state machine were not read in full.

- **#52 — fine to have shipped, with three defects and one gap.** The design holds up where it matters: an agent's file reads go through `git show <rev>:<path>`, so a path cannot leave the repository; what an agent writes is snapshotted and put back; the client refuses plain `http` off this machine, refuses redirects and cuts a key out of any error; the prompt says what it reads is data; nothing renders model output as HTML; the key is never in the machine file. Of CodeRabbit's findings, most were fixed before merge or since; the ones checked against the code are the stop-marks-done, the restore, the local-endpoint test, the settings overwrite, the limits sheet, the stale load, the consent failure and the finish count. Left: BUG-073 (two Assign clicks start two agents), BUG-074 (a remote agent's token limit is read only between steps), BUG-075 (a reply that cannot be read is taken as empty), and one unresolved thread that was not reproduced (TASK-066). The gap is TASK-065: FEAT-115 was closed Done with seven of its eight sweep tickets not run, and the bugs of the next days (BUG-065, 070, 071) are what that sweep is for. `agents.rs` at 1,500 lines is listed in TASK-066.
- **#53 (BUG-059) — good.** The verbatim-prefix handling is right, takes whole path components, and has tests on both platforms, including a non-Unicode path on each. One edge, a `\\?\Volume{…}` path, is in TASK-066.
- **#54 (BUG-060) — good.** The forecast is kept when a refresh moves neither branch, and asked again when a branch moved or the last ask failed; the five tests cover each. Nothing to file.
- **#55 (BUG-061) — good.** The forecast is not handed out for a different pair. It relies on the backend naming each side exactly as it was asked for, which `merger::forecast` does (checked), so the screen cannot wait for a name that never comes. The test fixtures were changed to match that, which is correct.
- **#56 (BUG-062) — good.** Leaves the workspace for the list when the open request is gone, and says what was merged or closed. It also leaves for the list when the request vanishes for any other reason, such as the author closing it elsewhere, which is the right behaviour. Nothing to file.
- **#58 (BUG-064) — good, GitHub only.** *Reviewed by you* is built from `latestReviews`, ignores pending and dismissed reviews, and marks a pull request pushed past since. GitLab and Bitbucket answer none, so the group is empty there; the item says so. Nothing to file.
- **#61 (TASK-058) — good.** The cache limits are only on cache steps, `continue-on-error` is on those steps alone, the version is bumped in every manifest, and `docs/ci.md` says what it does. Nothing to file.

## Findings

| Finding | Where it went |
| --- | --- |
| Two Assign clicks start two agents on one pull request | BUG-073 |
| A remote agent's token limit is read only between steps | BUG-074 |
| A reply that cannot be read is taken as an empty one | BUG-075 |
| A remote agent with no key is told *Bad request* | BUG-072, fixed |
| FEAT-115 closed Done with its P1 sweep not run | TASK-065 |
| Agent attribution on `main`, in descriptions and in replies | TASK-064, the check; the history is for the author |
| `agents.rs` size, `design_handoff_agents/`, `plain()` edge, `release-notes.mjs` on Windows, an unreproduced thread | TASK-066 |

Not worth an item: the symlink case in `guard.rs`. `put_back` writes through a link, but only where a local agent could already write, since a local agent runs with the person's own rights; the guard is there to keep a review's worktree clean, not to confine the agent.

## What waits on the author

- **History.** Leave the 127 trailer lines and 29 *Claude*-authored commits as they are (the proposal), or rewrite `main`.
- **Descriptions.** The merged pull requests' descriptions (#52, #55, #56, #58) can be edited to take the line out without touching history. It has not been done, because they are merged.
- **FEAT-115's status**, as TASK-065 asks.

## Acceptance criteria

- Each pull request above has a written verdict, one paragraph, kept in this item. *Met.*
- Each finding has an item, or is listed here as not worth one, with the reason. *Met.*
- The attribution check runs in CI, and the author has decided what happens to the existing history. *The check is in; the decision is open.*

## For the agent who picks this up

**Who:** unassigned. Any agent working on this repository can start cold.

- What is left is the author's decision above, and TASK-064's sweep once the check has run on a real pull request.
