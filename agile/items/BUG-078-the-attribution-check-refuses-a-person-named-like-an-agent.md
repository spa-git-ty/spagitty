<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-078 — The attribution check refuses a person whose name an agent shares

**Status:** Fixed — on `bugfix/BUG-076-merger-says-pushed-when-the-push-failed`; unmerged.
**Screens:** —.
**Raised by:** a review of the 2.0.1 stack on 2026-10-12, before TASK-064 was merged.

## What happens

TASK-064's check refuses a `Co-Authored-By:` line when the words *claude*, *devin*, *cody* and others appear anywhere in it. Those are people's names too:

```
Co-Authored-By: Claude Dupont <claude.dupont@example.org>
Co-authored-by: Devin Smith <d@x.io>
Co-authored-by: Cody Lee <c@x.io>
```

All three failed gate 1, though TASK-064's own criteria say a person named as a co-author passes.

## Acceptance criteria

- A person whose first name is Claude, Devin or Cody passes as a co-author, with or without a surname.
- The agents with those names are still refused: *Claude Opus 5.5*, *Claude Code*, *Devin AI*, and any name at an address only an agent writes from.
- Everything TASK-064 refused before is still refused.

## Fix

`tools/attribution.mjs` reads the name and the address of a co-author apart. It is an agent when the name is one only a product has (*Codex*, *Copilot*, *Cursor*, *Gemini*, *Antigravity* and the rest), when a model, *Code* or *AI* follows a name a person could have, or when the address is at a vendor's domain or is a `[bot]`. A bare *Claude* at an ordinary address is a person.
