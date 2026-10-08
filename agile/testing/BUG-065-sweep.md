<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-065 — Manual and live checks

1. On Windows, detect Codex, `omp` and `agy` with their normal installations.
2. Settings → Agents → Codex: test with the default sandbox; a startup failure
   must show its error. Select Full Access, test again, restart and check persistence.
3. Assign a pull request to Codex. Complete a file gate: its viewed tick should
   appear. Stop during another file: it must stay unticked. Untick a completed
   file and reload: it must stay unticked. Move the PR head: stale work must not tick it.
4. Test and assign OMP and agy. Confirm headless print mode exits after each step
   and cancellation stops the child process.

## Live results — Windows, 2026-10-08

The opt-in tests in `crates/spagitty-farm/tests/live_cli_access.rs` exercise
normal detection, adapter arguments, PATH and process launching. Each creates
a temporary file with an unpredictable marker absent from the prompt, and
requires the CLI to read that marker without editing the workspace.

- Codex 0.162.0: reproduced the untrusted-directory rejection, and the Windows
  sandbox's `setup refresh had errors` with an exit-zero failure verdict.
  The adapter with explicit Full Access read the temporary file successfully.
- agy 1.3.1: detected in its normal per-user install directory and successfully
  read the temporary file in print/plan mode. The print prompt stays adjacent
  to `--print`, since that flag consumes a value.
- OMP 18.8.6: detected as `omp` from `.bun/bin`; its native npm-installed Bun
  runtime is available for both probing and launch. Print mode starts, but the
  local configuration reports `No default model selected`. The authenticated
  file-read check remains incomplete until a default model is configured.

```text
cargo test -p spagitty-farm --test live_cli_access -- --ignored --nocapture
```

Mounted tests cover the Settings controls and assignment-event viewed ticks.
The complete desktop UI sweep and a hosted review/merge sweep were not performed.
The new settings take effect in a build containing this change; changing another
application's Codex permissions does not configure Spagitty's process.
