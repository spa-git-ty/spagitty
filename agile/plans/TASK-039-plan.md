<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-039 — Plan

**Item:** [`agile/items/TASK-039-one-colour-system-in-every-dialog.md`](../items/TASK-039-one-colour-system-in-every-dialog.md)

## Approach

The list in `flat.test.ts` names the files, so the mechanical half is a mapping
applied to eleven known components. The part worth planning is everything the
list does **not** name: a literal colour reads no token, so the assertion that
found these files was blind to the larger half of the same defect in the same
files. Fixing only what the list knew about would have left every dialog with a
fixed amber, a fixed red and a near-black scrim, and left the list empty — the
worst outcome, because the debt would then look paid.

So the order is: map the undefined tokens, then sweep the literals, then
**raise the contract** so the second half cannot come back, and only then delete
the list.

## Decisions

- **`--bg-3` is resolved per use, not mapped.** One fallback shade (`#2a2a2d`,
  sometimes `#222`, sometimes `#141416`) was doing three unrelated jobs — a chip
  fill, a well, a raised card. A single mapping would have been faithful to the
  old rendering and wrong about all three; the surface stack in `app.css` has a
  token for each.
- **`--soft` for chips, not `--surface-2`.** These dialogs float on
  `--glass-thick`, and `--soft` is a translucent ink wash that composites over
  glass, where an opaque surface shade would read as a second card on top of
  the first.
- **The checkerboard is derived, not added to the palette.** Same argument
  `--graph-bg` makes: a shade nobody chooses deliberately does not want eight
  hand-maintained copies.
- **`--on-accent` for text on any filled semantic colour**, including `--ok` in
  `PRDiffPane`. It is defined as contrast-safe text over a filled accent, and it
  flips with the mode — white in light families, the dark ground in dark ones —
  which is exactly the behaviour a label on a filled green needs too.
- **The type mapping collapses 10/11px onto `--fs-mono` and 12/13px onto
  `--fs-secondary`.** It grows the smallest labels, deliberately. The point of
  the token set is that the application has one type scale; keeping a 10px step
  purely because these files had one would be preserving the defect under a
  token name. Nothing here is shrunk to make a layout fit.
- **`KNOWN` stays as an empty array rather than being deleted** along with its
  two assertions. The mechanism — a new offender fails one test, a stale row
  fails another — is what stops the debt reopening quietly. An empty array with
  a live check is a contract; no array is an invitation.
- **The new assertion strips comments before scanning.** The accounts of why
  these colours were wrong quote the colours they replaced, and a check that
  forbade `#eee` in prose would delete its own evidence.
- **`CloneModal` and `PRDiffPane` are in scope** though they were never on the
  list. They carry the same two defects, and the new assertion would have
  failed on them anyway; excluding them to keep the count at eleven would have
  been arithmetic rather than work.

## Files

- The eleven: `chrome/StatusStrip`, `diff/BinaryDiff`, `diff/ImageDiff`,
  `history/FileHistoryView`, `requests/CreatePRModal`, `requests/PRWorkspace`,
  `settings/ProfilesSection`, `submodules/SubmodulesModal`,
  `worktrees/AddWorktreeModal`, `worktrees/WorktreesModal`,
  `routes/history/+page`
- Plus `clone/CloneModal` and `requests/PRDiffPane`
- `src/lib/ui/flat.test.ts`
- `CHANGELOG.md`, `agile/` — this set and the index row

## Steps

1. Map the undefined tokens; `--bg-3` by hand, per use.
2. Sweep the literals; the checkerboard, the handle label and the blame gutter
   by hand, since each needed a decision rather than a substitution.
3. Strip the dead fallbacks on defined tokens.
4. Tokenize the 83 type sizes.
5. Empty `KNOWN`; add the literal-colour assertion; run both against a
   deliberately regressed file.
6. Changelog, this set and the index row.

## Risks and rollback

- **Type grows.** The smallest labels move from 10–11px to 12px and 12–13px to
  13.2px, inside dialogs with fixed widths. Nothing here can measure a rendered
  dialog, so the sweep checks each at 100% and 130% text and at a narrow window.
- **`--surface` for `--bg-2` is a direction change, not a shade change.** The
  old fallback was *darker* than the dialog around it; `--surface` is a step
  *up* from the ground in both modes, which is the stack `app.css` documents.
  On a dark theme the affected cards therefore read slightly lighter than
  before. That is the intended reading — a card lifted off the ground — and the
  sweep is where it is looked at rather than reasoned about.
- Rollback is per file: nothing here changes markup, behaviour or state.
