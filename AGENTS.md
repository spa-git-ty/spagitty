<!-- Rules for coding agents working in this repository. -->
<!-- Spagitty reads this file and puts it in every agent's prompt. -->

# Agent rules

## Binding amendments

Before working in this repository, read and follow the shared amendments book:
[`/home/maxmya/dev/agents/docs/AMENDMENTS.md`](/home/maxmya/dev/agents/docs/AMENDMENTS.md).
Use its current text and Appendix A on precedence. The repository reference is
[`docs/AMENDMENTS.md`](docs/AMENDMENTS.md); it is a pointer, not a separate copy
of the rules. Skill summaries do not replace the canonical book.

## Product goal

Describe what this project is for.

## Architecture rules

- 

## Coding rules

- Prefer [`agile/`](agile/) over inventing process. An item's status there is
  authoritative.
- Do not invent brand copy. Approved wording lives in
  [`docs/branding.md`](docs/branding.md). Regenerate collateral with
  `python3 tools/make-brand.py` and `python3 tools/make-icons.py`; never
  hand-edit the derived PNGs.
- Never use the Git logo or Git orange (`#F05133`). Spagitty is independent.
- Changelog entries go under `## [Unreleased]` in the same change as the work
  (Amendment 20). Gate 6 reads that file for release notes.

## Forbidden changes

- Do not add a dependency without saying why in the handoff.
- Do not reformat files you are not otherwise changing.

## Testing requirements

- 

## Definition of done

A task is complete only when:

1. its acceptance criteria are met;
2. the tests pass;
3. the build succeeds;
4. no conflicts are left unresolved;
5. a review has completed, where one is required.
