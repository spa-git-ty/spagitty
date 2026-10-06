<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-098 — CodeRabbit as a farm review gate

**Status:** Backlog
**Branch:** `feature/FEAT-098-coderabbit-as-a-farm-review-gate`
**Screens:** Farm (1Q).
**Raised by:** the author, through
[`docs/proposals/extensions-and-coderabbit.md`](../../docs/proposals/extensions-and-coderabbit.md),
§10.
**Depends on:** FEAT-096, FEAT-097.

## Problem

The farm's path to `Done` is verification and then an independent agent's
review. A repository that wants a tool such as CodeRabbit in that path today has
no way to say so, and nothing would stop a merge that skipped it.

## Change

A **supplemental review** interface declared by the farm and supplied by the
desktop composition layer, so the extension host never owns farm state and
there is no dependency cycle. Repository policy with three modes:

| Mode | Behaviour |
| --- | --- |
| Off (default) | Nothing runs by itself; an explicit request still can. |
| Advisory | Runs after verification when autonomy and consent permit; findings are shown, nothing new is required to merge. |
| Required | A current, complete result with no blocking findings is an additional merge requirement. |

Evidence is bound to the task, the provider and its version, the base and head
commits, the scope, the policy version and the provider's configuration, is
persisted beside the farm, and is re-checked at **every** merge entry point —
automatic and manual. A disabled, removed, crashed, unauthenticated,
incompatible, incomplete, cancelled, unknown or stale provider blocks with its
own reason. Blocking findings go back through the farm's existing
change-request path within a bounded repair budget (default two cycles).

CodeRabbit supplements the independent reviewer; it never replaces the
`AgentId` check, and nothing it says can set a task to `Done`.

## Scope

Proposal §10 and the farm half of §9.5 and §12.

## Non-scope

Waivers. Replacing the independent reviewer. Changing the existing autonomy
levels.
