<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-099 — CodeRabbit on GitHub pull requests

**Status:** Backlog
**Branch:** `feature/FEAT-099-coderabbit-on-github-pull-requests`
**Screens:** Pull requests (1H).
**Raised by:** the author, through
[`docs/proposals/extensions-and-coderabbit.md`](../../docs/proposals/extensions-and-coderabbit.md),
§11.
**Depends on:** FEAT-096, FEAT-097.

## Problem

CodeRabbit already reviews pull requests on GitHub. Its summary, its inline
findings and its check run are in three different places on the host, and
asking it for another pass means typing a comment in a browser.

## Change

- A typed, forge-neutral **pull request snapshot** in the core — head and base
  SHAs, top-level discussion, inline review comments with author type and the
  commit they were made on, review records, check runs, and whether each list
  was read completely — read through the existing HTTP boundary with the
  connected account. GitHub is implemented; other hosts return an explicit
  unsupported answer.
- A typed **top-level pull request comment** operation, distinct from the inline
  reply, that reports an uncertain delivery instead of retrying.
- The CodeRabbit panel in the pull request workspace, drawn by the host's
  generic `reviewStatus` renderer: not observed, requested, running, completed,
  stale or unavailable — never green because nothing was said.
- Two explicit actions, *Request incremental review* and *Request full review*,
  each previewing the exact pull request and comment body before anything is
  sent. A requested review is "requested", not "reviewed".
- CodeRabbit is identified by the bot account's type and login, not by a
  display name or a mention in a human comment.

## Scope

Proposal §11.

## Non-scope

GitLab and Bitbucket CodeRabbit integration, webhooks, permanent polling,
changing branch protections, approving, resolving or merging on CodeRabbit's
behalf, automatic fixes.
