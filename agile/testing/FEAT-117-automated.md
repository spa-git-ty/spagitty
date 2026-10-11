<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-117 — Automated test record

**Item:** [FEAT-117](../items/FEAT-117-every-merge-happens-in-merger.md)

| Suite | Cases |
| --- | --- |
| `src/lib/merger/origin.test.ts` | A pull request is merged through the host with the method, title and message; a rebase sends neither; a refusal shows the host's sentence; resolving here fetches, checks out and presets the branch and the fetched target; pushing sends `branch:source`; a Farm task is told to the Farm after landing, and the merge is kept if the Farm cannot be told; a merge not made for the Farm does not tell it. |
| `src/lib/merger/MergerOrigin.test.ts` | The pull request band offers the three methods, merges with the one chosen and says it is merged, shows a refusal, offers resolving here when the host says it conflicts, and says what is being resolved; the Farm band names the task and branches. |
| `src/lib/requests/workspace.test.ts` | Pull requests offers *Open in Merger* and no merge button or dialog; a merge made in Merger takes the workspace back to the list, not to another pull request (BUG-062). |
| `src/routes/farm/page.test.ts` | *Merge it in Merger* opens Merger on the task; a task with no branch says so. |
| `tools/merges.test.ts` | No frontend file outside Merger asks a host to merge, merges a branch or a task, or integrates. |
| `crates/spagitty-farm/tests/pipeline.rs` | A task is refused as merged until its work is in the destination, then finished with its worktree removed, and telling it again changes nothing (run in WSL: it uses `/bin/sh`). |

## Results — 2026-10-11

- `bunx vitest run`: all passed. `bun run check`: zero errors, zero warnings.
- `cargo test -p spagitty-farm --test pipeline a_task_merged` in WSL: 1 passed.
