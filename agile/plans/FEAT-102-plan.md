<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-102 — Plan

**Item:** [`agile/items/FEAT-102-resolve-every-conflict-every-way.md`](../items/FEAT-102-resolve-every-conflict-every-way.md)

## Approach

**The model is pure** (`src/lib/resolver/model.ts`). A conflicted file is its
merged text with diff3 markers, split on `\n` alone so `\r\n` survives, and
its regions, parsed exactly as `conflicts::regions` parses them so the two
agree on numbering. A choice per region is one of `a`, `b`, `ab`, `ba`,
`pick` (a tick per line of each side) or `edit` (text). From the file and the
choices the model derives every result line with its origin, the file's text
once all are chosen, and a layout: each region's context in each column and
the result's numbering, which follows the choices. A file with no regions —
deleted on one side, binary — is one region chosen whole and lands as a side
(`take`), which for the side that deleted it is a deletion.

**Lines on each side** are found in that side's own file by matching the
region's lines after the context before it (`locate`, the same rule in Rust
and TypeScript). Merger's backend does it to `blame -L` each side's lines over
`<base>..<tip>` for the commit that introduced them; Conflicts, which has no
range, locates in the browser and shows no commits.

**Merger's backend** (`merger/detail.rs`, `merger_conflicts`) is the dry run
again, read in full: every conflicted path's stages and merged blob, and per
region its lines and commits. It writes nothing; a test checks the index and
refs. Landing is FEAT-101's path with the resolutions filled in.

**Choices kept** go to `<app data>/merges/<key>.json` (`merger_state.rs`, the
same temporary-file-and-rename as `review_state.rs`), the key sixteen hex
digits of the repository, both names and the base, accepted from the webview
only as lowercase hex. Each choice is stored with a fingerprint of its
region's sides and dropped on return if they differ. Writes are debounced.

**The components**: `Resolver.svelte` (files, file head, column heads, cards,
pill) holds the view — chosen file, current conflict, Base — and none of the
choices, which the screen owns and is told of. `ConflictCard.svelte` draws one
region, `ResolverPill.svelte` the controls, `paint.ts` the colours. Merger's
`MergerResolve.svelte` wraps it with the header; `resolve.svelte.ts` holds
Merger's choices.

**Conflicts** keeps its header and operation buttons, and its store is rebuilt
on the model: every conflicted file read whole on load, choices on screen, and
`conflict_settle` — write and `git add` together — on Mark resolved. Choices
survive a reload only for regions whose sides are unchanged.

## Risks

- A file git merged with the default markers has no base per region in
  Conflicts; the strip says so rather than inventing one.
- Many conflicted files are all read on load in Conflicts, where it read one at
  a time; each read is the index's three blobs and the file, small next to the
  merge that made them.
