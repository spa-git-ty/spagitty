<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-064 — Plan

**Item:** [TASK-064](../items/TASK-064-refuse-agent-attribution-in-ci.md)

One small module, `tools/attribution.mjs`, in the shape of `tools/release-notes.mjs`:
an exported `findAttribution(text)` and a CLI that reads either `git log
<range>` or a text file or stdin.

What counts: a `Claude-Session:` line; a *Generated with/by* or *Created with
Claude Code* line; and a `Co-Authored-By:` trailer whose value names an agent
or a tool, or an address at a vendor's `noreply`. A person as co-author does not
count, and neither does prose that names an agent. The application writes its
own `Co-authored-by` trailer into a user's commit when an agent resolved a
merge (FEAT-115); that commit is made on the user's machine and never passes
through CI.

Gate 1 runs it on `pull_request` events only, over `base.sha..head.sha`, and
over the description passed through `env`, never interpolated into the script.
History is not rewritten or scanned: the author decides that (TASK-063).
