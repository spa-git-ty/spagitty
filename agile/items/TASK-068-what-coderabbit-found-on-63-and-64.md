<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-068 — What CodeRabbit found on pull requests #63 and #64

**Status:** Done — on `bugfix/BUG-076-merger-says-pushed-when-the-push-failed`; unmerged.
**Screens:** 1S, 1R, 1K, 1Q.
**Raised by:** the author, 2026-10-12: fix what CodeRabbit found on the two pull requests, and close the threads that are not worth a change.

## Fixed

| Finding | Where | What was done |
| --- | --- | --- |
| A Farm task merged with a squash, a rebase or in another direction can never be finished: the Farm looks for the task's own commit in its target | `src/lib/merger/store.svelte.ts`, `MergerPlan.svelte` | Opened for a Farm task, Merger offers the merge commit and the fast-forward only, into the task's target only, and says why (BUG-077). |
| A push that ends before its token comes back is never heard, and whoever waits on it waits for good | `src/lib/network/store.svelte.ts` | A done event that arrives while an operation is starting is kept and read once the token is known (BUG-076). |
| The description check does not run again when the description is edited | `.github/workflows/gates.yml` | The pull request trigger takes `edited` too (TASK-064). |
| *Generated with ChatGPT* passes the attribution check | `tools/attribution.mjs` | A *Generated with* or *by* line is read for every tool the check knows (TASK-064). |
| `--auto-approve` is passed to plain `pi`, which has other flags | `crates/spagitty-farm/src/agent/adapters/pi.rs` | Only Oh My Pi (`omp`, `ohmypi`) is given it (BUG-065). |
| An id Google gave that begins `call-` is taken for a made-up one and not sent back | `crates/spagitty-core/src/models.rs` | Made-up ids begin `spagitty-call-` (BUG-071). |
| Twenty merges of something else use up the limit and hide a merge of the target | `crates/spagitty-core/src/remerge.rs` | The limit counts merges of the target only (BUG-068). |
| The manifest test passes with a source manifest missing | `crates/spagitty-extensions/tests/bundled_manifests.rs` | Only the generated bundle may be absent (BUG-066). |
| At *Ask before sending or merging* the tooltip says the agent merges | `src/lib/agents/AssignPopover.svelte` | It says the agent asks before it merges (TASK-060). |
| A line under the ruler and inside the comment range shows the range only | `src/lib/review/RoomDiff.svelte` | Both are drawn, one over the other (BUG-069). |
| Three documents said less than is true | `agile/` | FEAT-117 says a fork's branch is not pushed; BUG-071's plan says the first call is the signed one among parallel calls; TASK-062's status says the three are merged. BUG-076's test record says *bypasses* where it said *goes round*. |

## Not changed, and why

| Finding | Why |
| --- | --- |
| Hyphenate *read only* in BUG-074's title | It is not the adjective. The limit is read, only between steps. |
| Reword *one the host has not decided, or can merge* (BUG-063) | It says what it means. |
| `Step::say` has no cap (`record.rs`) | It never had one: before FEAT-118 a person's message and a refused finding were pushed uncapped too. What a person types and what one step's findings refuse are small. |
| Settings *Test* sends two lines, so an agy `.cmd` fails | The two lines are deliberate, so *Test* takes the path a real prompt takes. agy installs as an `.exe`, and a `.cmd` agent that cannot read standard input is refused with what to change (BUG-070). |
| Push with `--force-with-lease` when the resolution rewrote history | Not from here. A push git rejects is shown with git's words; forcing somebody's pull request is theirs to choose. |
| A multi-line template literal escapes `tools/copy.test.ts` | The check reads a line at a time by design. Reading literals across lines is a larger change to the check than the gap is worth; it is left as it is. |
| *The author's to make* (TASK-063's sweep) | The line was replaced when the author decided. |

## Acceptance criteria

- Each finding is fixed or answered on its thread, and every thread is resolved. *Met.*
- Each fix that can have a test has one. *Met, except the merge limit, which needs twenty-one merges in a fixture.*
