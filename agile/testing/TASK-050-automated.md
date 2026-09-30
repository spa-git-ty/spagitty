<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-050 — Automated test record

**Item:** [`agile/items/TASK-050-the-chatter-goes.md`](../items/TASK-050-the-chatter-goes.md)

## What was tested

The tests that pinned a removed sentence were rewritten to pin what replaced
it:

- Signing: the switch shows on or off with no line repeating it; the signer
  and key are named; "In effect" appears only when the value comes from another
  scope; a scope holding nothing offers no Clear.
- You: the value is in its field with no origin line; the repository chip is
  disabled with its reason as its title.
- Accounts, Updates, Profiles: no connection note, no privacy line, no "not
  checked" line, no empty-state instruction; the token scopes stay.
- External tools: "Reading…", "Built-in", "(not installed)", no "(detected)";
  the two catalogue tests and the current-value test go with what they tested.
- Command log, clone dialog, starter, commit detail, repository card, blame,
  stash list: the sentence is absent, or is the short form.

## Test command and output

On Windows 11:

```
$ bun run check
COMPLETED 1166 FILES 0 ERRORS 0 WARNINGS 0 FILES_WITH_PROBLEMS
$ bun run test
Tests  3030 passed (3030)
```

## What is not covered automatically

How the screens read. See the sweep.
