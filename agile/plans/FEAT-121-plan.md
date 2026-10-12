<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-121 — Plan

**Item:** [FEAT-121](../items/FEAT-121-what-is-running-is-said-and-what-would-collide-waits.md)

## The decision

Not a layer over the window. The item gives the reasons; the plan is what replaces it.

## Steps

1. **One store that knows what is running.** `src/lib/activity/store.svelte.ts`, with two lanes: `repository` and `network`. `begin(kind, what, lanes)` answers a handle or null; `run` wraps a promise; `blocked(...lanes)` answers the sentence, or null.
2. **The network store takes the `network` lane** in `start` and lets go in `ended` and when the start fails. Progress events go to the handle, so there is one label to draw.
3. **`perform` takes what the work is called** and holds `repository`. A refusal is a notice with the same headline a failure has, and git is not called.
4. **`pull` holds both lanes** for all three of its parts and renames itself between them. It says it cannot start before it asks anything.
5. **The other stores that write:** Branches (`run`), the Stash screen's stash, the commit, Merger's landing.
6. **The toolbar.** Each action declares its kind and its lanes. The capsule is a child of the pill, placed over it, so nothing in the row moves. A waiting button gets `aria-disabled` and a title, never `disabled`.
7. **A fifth of a second** before the capsule and the dimming show, in CSS, so no timer has to be cancelled.

## What decides it is right

- The lanes: a stash does not stop a fetch, a fetch does not stop a stash, a pull stops both.
- A lane is always let go: on success, on git's refusal, on a start that failed, and when the worker finished before its token came back.
- Nothing is put over the window.
