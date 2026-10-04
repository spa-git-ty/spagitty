<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-052 — Baseline

**Item:** [`agile/items/TASK-052-nothing-waits-in-line.md`](../items/TASK-052-nothing-waits-in-line.md)

**Not measured yet.** The author asked to do the measuring themselves; this is
how, and where the numbers go.

## How to measure

1. A release build (`bunx tauri build --no-bundle`); a debug build is far
   slower at startup and says nothing about performance.
2. Settings › God mode (the delight layer on) › **Timings**, then **Clear**.
3. On the reference repository, do each action below and read the row it
   adds: *Holding the repository* (the work), *Waiting for it* (time spent
   behind another command), *Calls, round trip* (what the screen waited for),
   and *Screens and diffs, until painted*.
4. For the work alone, without the application around it:

   ```
   cargo run -p spagitty-core --release --example baseline -- \
       <repository> <a long file to blame> <a commit with a large diff> <its large file>
   ```

Reference repositories, as the item suggests: `rust-lang/rust` or the Linux
kernel for history depth, and a commit touching a generated file of 10k+ lines
(a lockfile) for diff size.

## Before (step 1)

| Action | Held | Waited | Round trip | Painted |
| --- | --- | --- | --- | --- |
| Open the repository | | | | |
| Switch to each screen | | | | |
| Select a commit | | | | |
| Select a commit while a long file is being blamed | | | | |
| Open a 10k-line diff | | | | |
| Scroll it | | | | |
| Blame a long file | | | | |

## After (step 5)

The same table, once steps 2 to 4 are done.
