<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-070 — Plan

**Item:** [BUG-070](../items/BUG-070-local-agents-do-not-get-their-instructions.md)

Build on pull request #59 (BUG-065), which is rebased first. Then make the
instructions reach the agent whole: a prompt of many lines cannot be an
argument to a Windows `.cmd` shim, so `prompt_needs_stdin(executable, prompt)`
decides and each adapter obeys it (Codex `-`, Claude Code and Cursor with no
prompt argument, Oh My Pi's stdin mode). agy is a native executable and keeps
the argument. Settings *Test* sends a two-line prompt so the path is tested
where the instructions are tested. Test the decision and each adapter's
command line without a process; run every agent for real in the sweep.
