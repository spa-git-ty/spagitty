<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-060 — Agent permissions with standard names

**Status:** Done — on `task/TASK-060-agent-permissions-with-standard-names`; unmerged. The author confirmed the names on 2026-10-11.
**Screens:** 1K, 1R, 1S.
**Raised by:** the author, 2026-10-11, on 2.0.1.

## Problem

The levels an agent works at in Review and Merger (FEAT-115) have names the author finds unclear: *Suggest*, *Step by step*, *Sign off*, *Unattended*. A repository's limit is called the *highest* level. None of these says what the agent is allowed to do, which is what a permission name is for. Other agent tools use permission names people already know: read-only, ask, auto-accept, full access.

They are defined in `src/lib/agents/levels.ts` (`LEVELS`) and typed in `src/lib/agents/types.ts` (`Level`, `RepoRules.highest`, `neverUnattended`).

## Proposed names

Confirmed by the author on 2026-10-11:

| Today | Proposed | What the agent may do |
| --- | --- | --- |
| Suggest | **Read-only** | Read and propose. Nothing is applied until you accept it. |
| Step by step | **Ask every step** | Stops for you after the plan and after each file or conflict. |
| Sign off | **Ask before sending or merging** | Works through everything, and stops before the review is sent or the merge is made. |
| Unattended | **Full access** | Sends the review or makes the merge itself, within the repository's rules. |
| Highest level | **Maximum permission** | The most any agent may have in this repository. |

## Change

- The names and their one-line descriptions change everywhere they are shown: the Assign popover, the agent card, Settings › Agents, repository rules, notifications.
- The per-agent access choices added by pull request #59 (Codex *Full Access*, agy *Auto-approve tools*) use the same vocabulary.
- The stored values (`suggest`, `stepByStep`, `signOff`, `unattended`) stay, so saved settings keep working. Only what is shown changes.

## Acceptance criteria

- The author has confirmed the names (done, 2026-10-11).
- No screen shows the old names.
- `levels.test.ts` covers each name and description.

## For the agent who picks this up

**Who:** unassigned. Any agent working on this repository can start cold.

- **Where:** `src/lib/agents/levels.ts`, `AssignPopover.svelte`, `AgentCard.svelte`, `AgentsSection.svelte`, and every component that calls `levelWords` or `sentenceFor`.
- **Branch:** `task/TASK-060-agent-permissions-with-standard-names`, with plan, testing documents and a changelog entry when the work starts.

## What was done

The names in the table above are the ones shown: **Read-only**, **Ask every step**, **Ask before sending or merging** and **Full access**, with *Maximum permission* for the repository's limit and *Branches without full access* for the branches an agent may never act on alone. On a chip row the third reads *Ask at the end*. A level above the maximum says *The most allowed here is …* in place of *Capped at … here*. The Merger's record of who chose a resolution reads *Codex, full access* or *Codex, applied before you merged*. The one-line sentences under each name are rewritten to say what the agent may do (`levels.ts`). The stored values (`suggest`, `stepByStep`, `signOff`, `unattended`), `RepoRules.highest` and `neverUnattended` are unchanged, so saved settings keep working, and the Rust `Level::label` carries the same names. The per-agent access choices added by pull request #59 already use *Full access* (Codex) and *Auto-approve tools* (agy). Farm's own autonomy scale (*Manual … Unattended*) is a different setting and is not changed.
