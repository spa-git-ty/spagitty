<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-030 — Automated tests

**Item:** [`agile/items/BUG-030-the-toolbar-offers-what-it-cannot-do.md`](../items/BUG-030-the-toolbar-offers-what-it-cannot-do.md)

## What was tested

Five assertions in `src/lib/chrome/chrome.test.ts` and seven in the new
`src/lib/platform.test.ts`.

| Test | Asserts |
| --- | --- |
| offers no action it cannot perform | Undo and Redo appear nowhere, and no `.tool` carries a "not built" title. **This assertion used to require the opposite** — see below. |
| groups the actions rather than running them together | One divider, for two groups. It was two dividers for three; the first group held only the two dead buttons. |
| offers the pull and fetch alternatives from a real control | Two carets, each a `<button>` with `aria-haspopup="menu"`, `aria-expanded="false"` at rest, and an `aria-label` that is not the glyph. |
| opens the pull choices when the caret is pressed | A click on the caret sets `aria-expanded="true"` and puts all three pull modes on the page. This is the path that did not exist: the modes were reachable only by right-click. |
| keeps the right-click path it used to be hidden behind | The `contextmenu` handler still opens the same menu. |
| what counts as a Mac (4 tests) | `userAgentData` wins where present, `navigator.platform` next, the user agent string last, and no source at all answers `false` rather than throwing. |
| how a shortcut is written (3 tests) | `Ctrl+F` off a Mac, `⌘F` on one with no separator, and the alternate modifier named separately — `⌥` against `Alt+`, because spelling both from one function gives `⌘Alt`. |

## The test that guarded the bug

The first assertion is a rewrite, not an addition, and this is the interesting
part of the change. It read:

```ts
for (const label of ['Undo', 'Redo']) {
    const button = view.all('.tool').find((b) => b.textContent?.includes(label));
    expect(button?.getAttribute('title')).toBe('Not built yet');
}
```

Two prominent buttons took focus, took the pointer, announced themselves as
buttons and did nothing — and the suite's job was to check that the tooltip
apologising for it was still attached. It passed for as long as the defect
existed. A test written to describe the code rather than the requirement cannot
fail on the thing it is describing.

Nothing about that was caught by running the suite. It was caught by reading it.

## Run against the broken state first

Restoring the two `PENDING` entries to `GROUPS`:

```
 × offers no action it cannot perform
 × groups the actions rather than running them together
```

Removing the caret markup and leaving only `oncontextmenu`:

```
 × offers the pull and fetch alternatives from a real control
 × opens the pull choices when the caret is pressed
```

The right-click assertion stays green through both, which is the point of
having it: the fix adds a path rather than moving one.

## One thing worth writing down

The `contextmenu` assertion failed at first with the menu absent, because
`dispatchEvent` does not flush Svelte's queue — `click` from `src/testing/mount`
calls `flushSync` and a raw dispatch does not. The test calls it explicitly. A
component test that dispatches its own event and reads the DOM immediately is
reading the frame before the one it is testing.

## The whole suite

127 files, 2,662 tests, all passing. `bun run check`: 0 errors, 0 warnings over
1,143 files.

## What is not covered, and why

- **That the caret is actually visible.** It is a rendering question — size,
  contrast against the toolbar, whether it reads as one object with the button
  beside it. `SWEEP-002` and `SWEEP-003`.
- **Screen reader announcement.** The attributes are asserted; what VoiceOver
  and Orca say is `SWEEP-006`.
- **The macOS branch of `platform.ts` in a real window.** The detection table is
  tested against the strings each API produces; that WebKit on macOS produces
  one of them is `SWEEP-007`.

## Coverage

`src/lib/platform.ts` is first-party and fully covered by its own file: every
branch of the three-source fallback and both branches of each formatter. The
toolbar's changes are markup and styling inside a component whose tests already
run.
