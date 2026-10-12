<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-121 — Automated test record

**Item:** [FEAT-121](../items/FEAT-121-what-is-running-is-said-and-what-would-collide-waits.md)

| Suite | Cases |
| --- | --- |
| `src/lib/activity/store.test.ts` | Quiet when nothing runs; names the operation; moves on to the next part; shows git's progress and the name again without it; the newest when both lanes are taken. The repository is held against a second write and the network is not, and the other way round; a pull holds both and is refused while either is taken; a lane is let go when its operation ends and only then; a second `end` lets go of nobody else. `run` holds for as long as the work takes, passes a failure on, and runs nothing while the lane is taken. |
| `src/lib/network/store.test.ts` — *the network lane* | Held from the start of a fetch to its end, and the repository is not; a push is named a push; git's progress is passed on; let go when git says it failed, when it could not start, and when the worker finished before its token came back; not started while a pull holds the lane. |
| `src/lib/graph/actions.test.ts` — *said while it runs* | The work is named for as long as git takes; let go when git refuses; a second write is not sent to git and the notice names what is in the way; nothing is held while a confirmation is open; a stash is named a stash; a pull holds both lanes; the three parts of a pull over uncommitted work are named in turn; let go when the pull or the stash before it fails; no pull while a fetch runs, and nothing asked first; no pull over something that started while the question was open; no fetch or push during a pull; a fetch during a stash goes ahead. |
| `src/lib/branches/store.test.ts` — *said while it runs* | A switch is named; let go when git refuses; not started over a pull, with the reason as the write error; started while only the network is busy; rename, delete, delete several and create are named. |
| `src/lib/stash/store.test.ts` — *push, said while it runs* | The stash is named; not started over a checkout, with the reason. |
| `src/lib/chrome/chrome.test.ts` — *Toolbar, while something runs* | The capsule says what is running and goes when it ends; it follows the parts of a pull and git's progress; the button whose work runs shows the strands in its icon's place and the others keep their icons; what would collide waits with the reason as its title; Fetch and Push are left alone during a stash; a fetch holds the network actions and not the working copy's; a pull holds all five; a waiting button is never `disabled`; pressing one does nothing; the branch list and the ways to pull do not open while they would collide; everything comes back when the work ends; no layer is put over the window. |
| `src/lib/merger/landed.test.ts` | A merge is not landed over a write still running, and says which. |

## Results — 2026-10-12

- `bunx vitest run src/lib/activity src/lib/network src/lib/graph/actions.test.ts src/lib/branches/store.test.ts src/lib/stash src/lib/chrome`: 312 passed.
- `bunx vitest run`: 4039 passed in 189 files. `bun run check`: zero errors, zero warnings.
- No Rust was changed.
