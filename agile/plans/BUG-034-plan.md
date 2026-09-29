<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-034 — Plan

**Item:** [`agile/items/BUG-034-a-transparent-band-round-the-window-on-windows.md`](../items/BUG-034-a-transparent-band-round-the-window-on-windows.md)

## Approach

`decoratesItself(userAgent)` already decides this for Linux and macOS, from the
agent string; the `flush` state it leads to already removes the margin, the
corner and the shadow. Adding `Windows NT` to the platforms that decorate is the
whole change in behaviour. The window manifest gains `"shadow": true`, which is
Tauri's default for an undecorated window, so the system's frame is a stated
decision rather than an inherited one; the Linux override repeats the window
object, as `tools/window.test.ts` requires, and has no effect there.

## Files

| File | Change |
| --- | --- |
| `src/lib/chrome/window.ts` | `Windows NT` decorates. |
| `src/lib/chrome/window.test.ts` | Windows is flush, maximized or not; only an unknown host draws the card. |
| `src-tauri/tauri.conf.json`, `src-tauri/tauri.linux.conf.json` | `"shadow": true`. |

## Risks and rollback

- Windows 10 has no rounded corner for an undecorated window. A square window
  with the system shadow is what its own applications have.
- Rollback is a revert.
