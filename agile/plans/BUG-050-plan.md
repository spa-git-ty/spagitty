<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-050 — Plan

**Item:** [`agile/items/BUG-050-the-mac-check-waits-for-a-licence-agreement.md`](../items/BUG-050-the-mac-check-waits-for-a-licence-agreement.md)

## Approach

Feed `Y` to `hdiutil attach` on stdin with a here-string rather than a pipe. The
step runs under `set -o pipefail`, and a pipe from `yes` would fail the step
when `yes` is killed by the closed pipe. The attach output goes to a file under
`RUNNER_TEMP`, printed with an error if the mount fails, so the log keeps the
reason without 35 KB of GPL on every green run.

Taking the licence out of the bundle would also work, and would change what
every person downloading the DMG is shown. That is not this fix's call.

## Files

| File | Change |
| --- | --- |
| `.github/actions/macos-verify/action.yml` | The mount agrees to the licence; its output is kept for a failure. |
| `tools/release-macos.test.ts` | A licensed bundle's mount answers the prompt. |

## Risks and rollback

- Nothing but a Mac runner can show the prompt is answered. The draft release
  run is the test; see the sweep.
- Rollback is a revert.
