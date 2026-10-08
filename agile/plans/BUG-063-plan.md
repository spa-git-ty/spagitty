<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-063 — Plan

**Item:** [BUG-063](../items/BUG-063-a-conflicting-pull-request-is-not-marked.md)

Use the existing `mergeable` field rather than adding one; treat only `false` as conflicting, since `null` means the host has not decided. Add a `base` chip to `chipsOf` and a danger fact first in `factsOf`, render the chip on `InboxCard` with a `.tag.danger` style, and add a *Conflicts with* note to the room's meta line. Add the `Chip` to `RequestRow` and the workspace header. Branch the merge dialog on `mergeable === false`: an explanation and *Resolve in Merger*, which calls `merger.present({ a: base, b: head, into: 'b' })` and `goto('/merge')`. Guard `merge()` in the store too. Test each surface, the hand-off, the guard and the mark clearing on refresh.
