<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-073 — The sweeps, refreshed and run from a portal

**Status:** Open — the portal is built on `release/3.4.0`; the refresh of the sweep documents is under way.
**Branch:** `release/3.4.0`.
**Screens:** none in the application; `tools/sweep/`, `agile/testing/`.
**Raised by:** the author, 2026-10-19: "i tried to run sweep test from starting with first document but it felt outdated, so (don't delete them just update them if the sweep element is obsolete or deprecated) and create a document with index to all of them with html portal (maybe small server i can run with one command) to check sweep test done, or record behaviour or attach screen or video".

## Problem

There are 286 sweep documents and about 1,900 tickets, written over two months in four shapes. Many describe screens as they were: themes that were folded into one, a Requests workspace that became Review, merges made from the Pull requests screen that now happen in Merger, a rail that has moved twice. A person who starts at the first document is walked through an application that no longer exists. And there is nowhere to record a result except by editing a table cell by hand, and nowhere at all for a screenshot.

## Change

- **The portal.** `bun run sweep` starts a server on `127.0.0.1` and opens a page listing every sweep with its progress. Each ticket shows its preconditions, steps and expected result; a result is set with a note, and screenshots or a screen recording are attached by pasting, dropping, choosing a file or recording from the page. The result is written into the ticket's own cell or line in its sweep document, nothing else in the file changing.
- **The index.** `agile/testing/sweeps.md` lists every sweep with its item's title and status and its tallies. It is generated; the portal writes it after every result, and a test fails when it is stale.
- **The refresh.** Every ticket whose screen or behaviour has since been replaced is kept and marked *Obsolete*, naming what replaced it; wording that still describes a real path but names an old screen is brought up to date. Nothing is deleted.

## Acceptance criteria

- One command opens the portal; it binds to this machine only.
- A result set in the portal lands in the right ticket in every one of the four shapes, and the file is otherwise unchanged.
- Evidence files are stored outside git and linked from the result.
- `agile/testing/sweeps.md` exists and a test keeps it current.
- No sweep document is deleted. Tickets for what no longer exists say *Obsolete* and why.
