<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-121 — What is running is said, and what would collide waits

**Status:** Done — on `feature/FEAT-121-say-what-is-running`; unmerged. Seen in the built frontend against fixtures; not yet in the running app.
**Screens:** chrome, 1A, 1C, 1F, 1G, 1S.
**Raised by:** the author, 2026-10-12, on 2.1.0.

## Problem

A pull, a stash or a checkout takes time, and while it ran nothing on screen said so.

- The button looked the same as before it was pressed.
- Pressing it again started a second `git` over the first, and the two fought for `index.lock`.
- A fetch and a push did say what they were doing, in a line of their own that nothing else could use.
- That line sat inside the toolbar's pill. The pill is centred, so when the line appeared every button slid sideways from under the pointer.

## What was asked for, and what was built instead

The request was a loader on a half-transparent layer over the whole window, blocking everything until the work ended. The author asked for that to be argued with, and named the alternative in the same message: dim the buttons, show the loader, block nothing.

The layer was not built. The reasons:

- **It holds the wrong things.** A fetch on a large repository takes a minute. Reading a diff, scrolling the graph or walking to another screen does not collide with it. A layer stops all of them to prevent two that do.
- **It has no way out when git hangs.** A push waiting on a network that has gone away would leave a window that can do nothing, with no control under the layer to reach.
- **It flashes.** Most checkouts and stashes end inside a fifth of a second. A layer over the window for each of them is a flicker of the whole screen.
- **It contradicts a rule the project already has.** `CONTRIBUTING.md`: *never block on walking the whole history*. A fetch already runs on a worker so the window stays free while it does.

What needs keeping apart is two writes to the same thing, and that is decided where the write starts.

## Change

One store, `src/lib/activity/store.svelte.ts`, knows what is running. An operation takes the lanes it writes to:

| Lane | Is | Taken by |
| --- | --- | --- |
| `repository` | the working copy, the index and the refs | checkout, stash, reset, revert, cherry-pick, rebase, commit, branch and tag writes, a Merger merge |
| `network` | the one worker that talks to a remote | fetch, push |

A pull takes both. A second operation that needs a taken lane is not started.

While something runs:

- **It is named.** A capsule over the toolbar says what, with the strands: *Stashing your changes…*, *Pulling…*, *Switching to main…*, or git's own progress, *Receiving objects 45%*. It is over the pill and not in it, so nothing moves. It takes no clicks from what is under it.
- **The button that started it shows the strands** in its icon's place.
- **What would collide waits.** Its button is dimmed and its title says what for: *Pulling is still running*. A waiting button is still a button: it keeps the focus and can be hovered.
- **Nothing else is held.** During a stash, Fetch and Push stay as they are. During a fetch, Stash, Branch and the branch switcher do.
- **The capsule and the dimming wait a fifth of a second** before they show, so quick work does not flash.

When it ends the capsule goes, the buttons come back, and the notice says how it went, as before.

A second write asked for from somewhere with no button to dim — a menu on the graph, the palette — is refused with a notice that names what is in the way.

## Acceptance criteria

- A pull, a stash, a checkout and the other writes are named while they run, and the name goes when they end, whether they worked or not.
- A pull over uncommitted work names its three parts in turn.
- A fetch or a push shows git's own progress in the same place.
- Buttons whose work would collide are dimmed with the reason; the others are not.
- A waiting button is never `disabled`, and pressing it does nothing.
- No layer is put over the window, and nothing in the toolbar moves when the capsule appears.
- A write that would collide is not sent to git, wherever it was asked for.

## What was done

- `activity` store with the two lanes, `begin`/`run`, and `blocked`, which answers the sentence.
- The network store holds the `network` lane from the start of a fetch or a push to its done event, and passes git's progress on.
- `perform` in `src/lib/graph/actions.ts` takes what the work is called and holds the `repository` lane; every action through it is named. `pull` holds both lanes and names *Stashing your changes*, *Pulling*, *Putting your changes back*.
- The Branches store, the Stash screen's stash, the commit and Merger's landing hold the `repository` lane.
- The toolbar reads all of it: the capsule, the strands on the button, the dimming with its reason.

## Not in this change

- Staging and unstaging on the Working copy screen are not named. They are quick and constant, and a capsule for each would be noise.
- Reflog's and Tags' own writes keep their own busy state and are not on a lane yet.
- `pull` still holds the session for as long as git runs, so a screen that asks the core for something during a pull waits for it. That is in `src-tauri/src/commands.rs` and is older than this item; a pull on the network worker would end it.
