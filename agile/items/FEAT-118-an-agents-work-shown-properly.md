<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-118 — What an agent was sent and what it said, shown properly

**Status:** Backlog — reported 2026-10-11, not started.
**Screens:** 1R, 1S.
**Raised by:** the author, 2026-10-11, on 2.0.1.

## Problem

An agent's work is plain text in the agent card's column. In `src/lib/agents/AgentCard.svelte`:

- each step lists `step.events` as plain list items (around lines 192–210) and shows the last four while it runs;
- *Raw output* dumps the transcript into a `<pre>` (around lines 300–315).

What Spagitty sent the agent is not shown at all. The timeline shortens the command line, and the prompt is only in the transcript file. Its replies, tool calls, files read and results all look the same.

## Change

- Each event has a kind, set where the stream is read (`crates/spagitty-farm/src/execution/narrate.rs` and the remote driver), not guessed by the screen:
  - what Spagitty sent (the instructions);
  - the command;
  - a tool call and its input;
  - a file read;
  - the agent's reply;
  - a result;
  - an error.
- Each kind is drawn as itself. Replies are rendered as Markdown with the existing renderer. Commands and tool input are set as code. Files read are links into the diff.
- All of it is hidden by default. The card shows the step list and the current sentence, and *Details* opens the full record of the selected step.
- *Raw output* stays, inside *Details*, for when the structured view is not enough.

## Acceptance criteria

- For any step, the reader can see exactly what Spagitty sent and exactly what came back, each part labelled.
- Nothing of it shows until asked for.
- Claude Code, Codex and a remote agent are covered by tests that turn a recorded stream into typed events.

## For the agent who picks this up

**Who:** unassigned. Any agent working on this repository can start cold.

- **Where:** `src/lib/agents/AgentCard.svelte`, `src/lib/agents/types.ts` (`Step.events`), `crates/spagitty-farm/src/assign/` (engine, local, remote), `crates/spagitty-farm/src/execution/narrate.rs`, `src-tauri/src/agents.rs` (`assignment_transcript`).
- Do it together with FEAT-119: both change the card.
- **Branch:** `feature/FEAT-118-an-agents-work-shown-properly`, with plan, testing documents and a changelog entry when the work starts.
