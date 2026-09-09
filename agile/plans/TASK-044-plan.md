<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-044 — Plan

**Item:** [`agile/items/TASK-044-settings-stops-explaining-itself.md`](../items/TASK-044-settings-stops-explaining-itself.md)

## Approach

Move, do not delete. Every long paragraph in Settings says something true, and
several of them say what leaves the machine — which is the one thing this
application must never become quieter about. So the claims go to `title`
attributes and the visible line becomes what a person is scanning for.

The test change follows from that and is the part worth being careful about: an
assertion that reads only the rendered text would now pass if the claim were
deleted outright. `readable()` reads the text and every title, so the contract
survives the move.

## Decisions

- **A hover, not a disclosure control.** A details/summary would be a second
  control on a screen made of controls, and it would need a state. A `title` is
  free, is already how this application explains a control, and costs no layout.
- **Scopes stay visible.** They are instructions somebody must follow to make
  the screen work; hiding them behind a hover would send them to the host's
  documentation.
- **State messages are untouched.** "Reading…", "Not checked yet" are the screen
  answering, not explaining.
- **God mode's one-liners mostly stay.** Under a dozen words each, and each says
  what a debug control does that it cannot show. Two were shortened.
- **`readable()` in the test rather than a looser assertion.** Weakening the
  assertion to "contains 'keychain' somewhere in the DOM" would pass on a
  comment. Reading text plus titles is exactly the set a person can reach.

## Files

- `src/lib/settings/AccountsSection.svelte`, `LicenseSection.svelte`,
  `UpdateSection.svelte`, `RemotesSection.svelte`, `GodModeSection.svelte`
- `src/lib/settings/sections.test.ts`, `remotes-section.test.ts`
- `CHANGELOG.md`, `agile/` — this set and the index row

## Steps

1. Accounts: the two paragraphs the author quoted.
2. License, Updates, Remotes, God mode.
3. `readable()`, and the four assertions that read a title now.
4. Changelog, this set and the index row.

## Risks and rollback

- **A hover is not reachable by touch or keyboard.** Spagitty is a desktop
  application with no touch target and every one of these claims is also in
  `docs/`, but it is a real narrowing and it is the cost of the change.
- Rollback is per paragraph; nothing here is structural.
