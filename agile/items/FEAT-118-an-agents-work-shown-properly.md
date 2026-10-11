<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-118 — What an agent was sent and what it said, shown properly

**Status:** Done — on `feature/FEAT-118-an-agents-work-shown-properly`; unmerged. Not yet looked at in the running app.
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

## What was done

**Where the kind is set.** `crates/spagitty-farm/src/execution/narrate.rs` gains `EventKind` (*sent, command, tool, read, reply, result, error*), `StepEvent` and `classify`, which reads the grammar the narrators already write: a `·` line is Spagitty's (`Read a.rs`, `Bash cargo test`, `read the diff of b.rs`, `failed: …`, a refusal), a plain line is the agent's reply unless it has the shape a command-line agent prints for a command or its outcome, `You · …` is what the person told it, `$ …` is a check's command. A failed tool now says `failed:` so it is an error by its words. Every step keeps `record: Vec<StepEvent>` beside `events`, and `prompt`, what Spagitty sent for the step, cut at 60,000 characters (`record.rs`, `engine.rs`); the remote agent's tool calls come through the same narration. Records saved before this have neither and say so.

**What is shown.** The agent card no longer lists a step's lines. A step line selects the step; **Details** opens its record, hidden until asked for: *Sent* (the prompt, folded), then each part labelled as what it is: replies as Markdown, commands and tool input as code, a file read as a link into the diff (in Review, it selects the file), results quiet, errors in red, the files sent to a remote model and the checks. *Raw output* stays, inside Details.
