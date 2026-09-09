<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-042 — Plan

**Item:** [`agile/items/TASK-042-one-control-scale-and-a-platform-policy.md`](../items/TASK-042-one-control-scale-and-a-platform-policy.md)

## Approach

Two of the three pieces here are finishable and checkable, and one is not.

The type sizes and the hover motion are both "find every instance, fix it, and
add the assertion that stops the next one" — and the assertion is the point. A
pass that fixed eight sizes and nineteen hover rules without one would be a pass
that has to be run again in six months, which is what TASK-039 said about the
`KNOWN` list and is the same lesson.

The macOS window is the piece that cannot be finished here, because there is no
Mac. So it is arranged to be as verifiable as possible from a Linux machine: the
policy is a **file**, its shape is asserted against the base config, and the
frontend's two behavioural consequences — no controls, no resize edges — are
component-level facts that a test can hold. What is left is what only a Mac can
answer, and it is written as a sweep rather than claimed.

## Decisions

- **Hover changes colour; press keeps motion.** The review said "reserve motion
  for meaningful state changes" and this is the line that sentence draws. A key
  going down is a state change. A pointer being somewhere is not.
- **Cards and badges keep their lift.** The cost of hover motion is that a
  target moves while it is being aimed at, and that cost is paid where targets
  repeat in a row — a dialog's buttons, fourteen rail rows, a filter strip. A
  single card in a browsing grid is not that case, and lifting it is a real
  affordance. So the assertion is scoped to the shared kit and the chrome rather
  than made universal, and the scope is written down in the test.
- **`font-size: Npx` is forbidden; `em` and `calc(var(--fs-…) * n)` are not.**
  The rule is not "no arithmetic", it is "follow the scale". Both of those do.
- **9px becomes `--fs-mono`.** It is below anything the scale offers and below
  what anybody should be asked to read. Growing it is the fix, not tokenizing
  the 9.
- **A platform config file, not a runtime call.** Tauri merges
  `tauri.<platform>.conf.json` into the base at build time, which means the
  macOS window is *created* decorated rather than decorated afterwards. Calling
  `setDecorations(true)` at startup would produce a visible frame change on
  every launch and would leave the base config lying about what ships.
- **`Overlay`, not plain decorations.** Plain decorations put a system title bar
  *above* the application, which would mean two bars: the system's and
  Spagitty's workspace row. Overlay draws only the traffic lights, over the
  webview, which is what keeps the tab strip and the name where they are.
- **78px on the left, not a `--titlebar-*` token.** It is Apple's number, not
  Spagitty's; it does not scale with the interface zoom; and publishing it as a
  token would invite something else to lay itself out against it.
- **`decoratesItself` is written as "not Linux and not macOS".** Phrasing it as
  "is Windows" would make an unrecognised host — a browser during frontend work,
  a webview with a new agent string — come up as a hard-cornered rectangle with
  no shadow, which looks like a screenshot. The card is the safer default for
  the unknown case.
- **`platform.rs` is not touched.** The review asks for profiling on
  representative GPUs and an accessible path. There is one machine here, and
  removing a known stability fix on appearance grounds is what the review says
  not to do. TASK-043 records it honestly instead.

## Files

- `src-tauri/tauri.macos.conf.json` (new)
- `src/lib/chrome/TitleBar.svelte`, `ResizeEdges.svelte`, `window.ts`,
  `window.test.ts`
- `src/lib/ui/Btn.svelte`, `Chip.svelte`, `src/lib/chrome/NavRail.svelte`
- `src/lib/chrome/Toolbar.svelte`, `src/lib/delight/BadgeChip.svelte`,
  `src/routes/repos/+page.svelte`
- `src/lib/ui/flat.test.ts` — the two new contracts
- `CHANGELOG.md`, `agile/` — this set, TASK-043, and the index rows

## Steps

1. The macOS config, and `decoratesItself` following it.
2. The title bar and the resize edges, behind `isMac()`.
3. The hover motion, and the assertion that keeps it out.
4. The last eight type sizes, and the assertion that keeps them out.
5. TASK-043, so the deferred half is recorded rather than dropped.
6. Changelog, this set and the index rows.

## Risks and rollback

- **The macOS half is unproven.** If the config does not merge, the window comes
  up as it does today and the frontend hides controls that the platform is not
  drawing — a window with no controls at all. `SWEEP-005` is the first thing to
  run on a Mac, and reverting is deleting one file.
- **78px may be wrong** at a non-standard traffic-light spacing or in full
  screen, where the lights are hidden and the reservation becomes dead space.
  `SWEEP-007` and `SWEEP-008`.
- **Removing hover motion is a taste change** as well as an ergonomic one, and
  it is not reversible by a preference. It is one line per rule to put back.
- **Growing eight type sizes** can reflow the title bar and the badge grid.
  `SWEEP-002` and `SWEEP-003`.
