<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-125 — A choice on a conflict can be cancelled, and Pick lines starts empty

**Status:** Done — on `feature/FEAT-124-an-agent-that-shows-it-is-working-and-asks-at-the-top`, with FEAT-124, FEAT-126, FEAT-127 and TASK-070; unmerged. Covered by mounted tests; not looked at in a browser or in the running app.
**Screens:** 1S, 1D.
**Raised by:** the author, 2026-10-12, on 2.2.0.

## Problem

In the three-column resolver (`src/lib/resolver/`), used by Merger and by Conflicts:

1. **There was no cancel where the choice is made.** After *Take main*, *Both*, *Pick lines* or *Edit by hand*, the way back was a small *Reset* chip at the far end of the card's header, away from the buttons that were just pressed. The author did not find it and asked for a cancel. After *All from …* for a whole file there was no way back at all, short of *Reset* on each conflict or *Abort* for the whole merge.
2. **Pick lines started with every line ticked.** Pressing it resolved the conflict at once as both sides, and picking meant unticking what was not wanted. The author asked for it to start with nothing ticked.

## Change

- **Cancel** is at the end of each card's row of choices, shown once anything is chosen there. It returns the conflict to unresolved, whichever way it was chosen. The header's *Reset* is gone: one control, where the hand already is.
- **Cancel all** is beside *All from …* in the file's header, shown when any conflict of the file has a choice. It returns every conflict of the file to unresolved. When any of them was typed by hand it asks first, since that is the one choice a press cannot make again.
- **Pick lines starts with no line ticked.**

## Decisions

- **Pick lines with nothing ticked is not a resolution.** Starting empty would otherwise resolve the conflict as *keep nothing* on the press that only opened the boxes, and *Complete merge* would then delete both sides. So it counts as unresolved until a line is ticked (`settles` in `model.ts`): the status stays *Unresolved*, the result column says *Tick the lines to keep*, the dot stays hollow and the merge cannot be completed. Unticking the last line makes it unresolved again. Keeping nothing on purpose is still possible: *Edit by hand*, with the box cleared.
- **The lines are not faded while nothing is ticked.** Unticked lines fade once a line is ticked, to show what will not be kept. Before that, fading every line would dim exactly the text the reader has to read to choose.
- **An agent's proposal is not dismissed by opening Pick lines.** The engine is told the person decided otherwise only once the choice settles the conflict.
- **The word is Cancel**, the author's. *Abort* keeps its meaning: the whole merge, back to the plan.

## Acceptance criteria

- Any choice on a conflict can be cancelled from the row it was made in.
- A whole file's choices can be cancelled at once.
- Pick lines opens with nothing ticked, and resolves nothing until a line is.

## What was done

- `src/lib/resolver/model.ts`: `settles`; `pickNone` replaces `pickAll`; `chosenLines` and `counts` go by `settles`, so the result text, what is written and the layout all treat an empty pick as unresolved.
- `src/lib/resolver/ConflictCard.svelte`: *Cancel* in the choices; the status, the tint and the fading go by `settles`.
- `src/lib/resolver/Resolver.svelte`: *Cancel all*; the dots, the per-file count and *Next unresolved* go by `settles`.
- `src/lib/merger/agent.svelte.ts`: `personChose` waits for a choice that settles.
- Conflicts (1D) uses the same resolver and has all of it.
