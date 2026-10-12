<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-125 — Plan

**Item:** [FEAT-125](../items/FEAT-125-a-choice-on-a-conflict-can-be-cancelled-and-pick-lines-starts-empty.md)

## Steps

1. **One question, asked in one place.** `settles(choice)` in the model: a choice settles its region unless it is Pick lines with nothing ticked. `chosenLines` answers null for a choice that does not, so `resultText`, `landing` and `layout` need no change of their own, and `counts` asks the same question.
2. **Pick lines starts empty.** `pickNone` in place of `pickAll`.
3. **The card.** *Cancel* at the end of the choices, in place of the header's *Reset*. Whatever read `choice !== null` to mean *resolved* reads `settles` instead; whatever means *something is chosen here* (the boxes showing, *Cancel* showing) still reads the choice.
4. **The resolver.** *Cancel all* calls the same `onchoose(path, index, null)` the card's *Cancel* does, once per conflict that has a choice, so both screens that use the resolver get it without a new property. It asks first when a hand edit would go.
5. **The agent.** Opening Pick lines is not a decision against its proposal.

## What decides it is right

- Pressing Pick lines and then *Complete merge* is not possible without ticking a line.
- A pick kept from before this change, with lines ticked, reads exactly as it did.
- No new state: an empty pick is the same stored choice it always could have been.
