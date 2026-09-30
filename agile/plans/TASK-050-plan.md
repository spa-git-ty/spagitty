<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-050 — Plan

**Item:** [`agile/items/TASK-050-the-chatter-goes.md`](../items/TASK-050-the-chatter-goes.md)

## Approach

Settings section by section, read as a person would, applying the item's rule.
Then the rest of the application by listing every on-screen text node of eight
words or more in the markup (comments, scripts and styles stripped) and
applying the same rule to each; the list is a scratch tool, not part of the
tree.

Edits are exact replacements that must each match once. Where removing a
footer left an `{:else}` with nothing in it, the branch goes. svelte-check is
run until it reports no unused CSS selectors and no empty blocks, which is how
the styles of removed elements are found and removed.

Tests that asserted a removed sentence are rewritten to assert what replaced
it — the chip disabled with a title, the field holding the value, the Clear
button absent — or the sentence's absence.

## Files

Settings: `IdentitySection`, `SigningSection`, `ProfilesSection`,
`AccountsSection`, `AppearanceSection`, `ExternalToolsSection`,
`PersonalitySection`, `UpdateSection`, `LicenseSection`, `RemotesSection`,
`describe.ts`. Screens: `rebase`, `tags`, `reflog`, `repos`, `conflicts`,
`search`, `history`, `diff`, `changes`, `farm`, `requests`. Components:
`StashList`, `StashDetail`, `CommandLog`, `BlameStrip`, `CloneModal`,
`RepoCard`, `CommitDetail`, `Starter`. Their tests.

## Risks and rollback

- **A newcomer reads less.** What was only explanation is gone; what carried a
  fact is in a title. Rollback is a revert.
