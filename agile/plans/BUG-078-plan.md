<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-078 — Plan

**Item:** [BUG-078](../items/BUG-078-the-attribution-check-refuses-a-person-named-like-an-agent.md)

The check tested one pattern against the whole trailer, so a word that is both
a product and a first name refused a person.

Split the value into `Name <address>` and ask three things:

- `PRODUCT`: the name starts with a word nobody is called.
- `PRODUCT_BY_WHAT_FOLLOWS`: *Claude* followed by *Code*, a model name or a
  number; *Devin AI*; *Cody AI*.
- `AGENT_ADDRESS`: the address is at a vendor's domain, or carries `[bot]`.

Any one makes it an agent. The lines that are attribution whoever they name
(`Claude-Session:`, *Generated with*) are untouched.

Tests name the people who were refused and the agents who share their names,
and the run over the last 120 commits on `main` still finds the old trailers.
