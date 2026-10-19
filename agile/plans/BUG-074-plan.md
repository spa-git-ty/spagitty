<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-074 — Plan

**Item:** [BUG-074](../items/BUG-074-a-remote-agents-token-limit-is-read-only-between-steps.md)

1. Add a `Budget` to `Ask`: the tokens left of the run's limit (already capped by the day's in `assignment_start`) and the deadline of the time limit. The engine fills it from `Limits` when it asks each step.
2. Move the remote driver's loop into `converse`, which takes the exchange as a function, so a scripted provider can drive it without a network.
3. In `converse`, check the budget before every exchange after the first; once it is spent, stop the step and keep what it gathered.
4. Run each real exchange on its own thread and wait for it in slices, so Stop and the time limit end a step that is waiting on a provider.
5. In the engine, a step that comes back with no answer while a limit is reached stops with the limit's sentence instead of failing with *gave no answer*. One with an answer is used, and the next checkpoint stops the run as before.
6. Tests: a scripted provider whose answers cost 30,000 tokens against a 50,000 limit stops after the second exchange with both findings; no limit means none; the wait gives up at the deadline and at Stop; the engine hands each step what is left and stops a step cut off with nothing.
