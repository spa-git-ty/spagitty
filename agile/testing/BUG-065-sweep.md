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
  A headless command permission denial was reproduced in the desktop Settings
  test. With explicit tool auto-approval, the live print/plan test ran a read-only
  shell command and returned the unpredictable temporary-file marker successfully.
- OMP 18.8.6: detected as `omp` from `.bun/bin`; its native npm-installed Bun
  runtime is available for both probing and launch. Print mode starts, but the
  local configuration reports `No default model selected`. The authenticated
  file-read check remains incomplete until the selected model/profile is supplied.
  Settings now saves those choices and passes them to Test, Review and Merger;
  no global OMP configuration is changed.

The ignored `agents::tests::live_settings_tests_use_saved_codex_and_agy_permissions`
test also passed through the actual Settings save and Test handlers. It uses an
isolated application identifier, preserves the normal CLI authentication environment,
and requires an unpredictable file marker absent from the prompt. Codex passed
with saved Full Access in 11,118 ms; agy passed with saved tool approval in 12,915 ms.

```text
cargo test -p spagitty --lib agents::tests::live_settings_tests_use_saved_codex_and_agy_permissions -- --ignored --nocapture
```

```text
cargo test -p spagitty-farm --test live_cli_access -- --ignored --nocapture
```

To use the same model/profile as an interactive launch in the OMP smoke test,
set `SPAGITTY_TEST_OMP_MODEL` and, if used, `SPAGITTY_TEST_OMP_PROFILE` before
running it. These contain model/profile names, not credentials.

Mounted tests cover the Settings controls and assignment-event viewed ticks.
The dev app was launched with `bun run tauri dev` from the fix worktree; the
native window opened and its Settings dev route served successfully. The complete
desktop UI sweep and a hosted review/merge sweep were not performed.
The new settings take effect in a build containing this change; changing another
application's Codex permissions does not configure Spagitty's process.
