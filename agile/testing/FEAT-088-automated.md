<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-088 — Automated test record

**Item:** [`agile/items/FEAT-088-gitlab-as-its-api-says.md`](../items/FEAT-088-gitlab-as-its-api-says.md)

Written after the branch was built, on the tip of the stack that contains it
(`feature/FEAT-093-threads-done-properly`); the suites below are this item's,
and the counts are the whole run at that tip.

## What was tested

| Suite | Cases |
| --- | --- |
| `crates/spagitty-core/src/forge.rs` | A GitLab project in a nested group identified, a GitLab page's URL not taken for a nested project, only GitLab nesting; a project path one segment with every slash escaped; a connected account vouching for a host under another name. |
| `crates/spagitty-core/src/forge/gitlab.rs` | A merge request needs you only when you are a reviewer; the list claims no checks it was not sent; a change count past GitLab's ceiling keeps its digits; every project call addresses the whole path, encoded; a diff entry becomes a file with its counts, and added, deleted, renamed and binary files say so; a commit's short id, title and author; discussions become line comments pointing at their first note; threads counted and a reply to you found without system notes; the latest pipeline is the checks; the latest version pins a comment's position; an added line placed by its new number, a removed one by its old, an unchanged one by both; a range carried by line code; a line code is the path's SHA-1 and both counters; an older caller that says only a side still places its comment; no merge requests asks nothing. |
| `src/routes/review/page.test.ts` | GitLab asked once for the checks and threads its list leaves out; GitHub asked nothing more. |

## Test command and output

On Windows 11, at the stack's tip:

```
$ bun run check
0 ERRORS 0 WARNINGS
$ bun run test
Tests  1 failed | 3153 passed (3154)
$ cargo test -p spagitty-core
test result: ok. 586 passed; 0 failed
```

The one failure is `tools/record.test.ts` on BUG-043, which reached `main`
without its plan and testing documents before these branches were cut.

In WSL (Arch), on this item's own branch when it was built and again at the
stack's tip:

```
$ cargo test -p spagitty --lib
test result: ok. 106 passed; 0 failed
```

## What is not covered automatically

Every call against the author's work GitLab: the sweep.
