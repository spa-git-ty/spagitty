<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-043 — The Linux renderer and the accessibility bridge

**Status:** Backlog.
**Screen:** — (the Linux host, before the first frame).
**Raised by:** a UI review: "profile the actual release build on representative
GPUs. Investigate a version/hardware-scoped workaround and an accessible path;
do not remove known stability fixes on appearance grounds."

## Problem

`src-tauri/src/platform.rs` makes two decisions for every Linux user, and both
were reached on one machine.

**The DMABuf renderer is disabled on Wayland.** FEAT-055 measured three
presentation paths on a Wayland session with an NVIDIA driver and found that
only the software path kept painting: native Wayland with DMABuf died with
`Error 71`, and XWayland lost its buffer and went transparent. The conclusion
was right for that machine and it is the default for everyone, which means every
blur, shadow, gradient and scrolled row on every Linux install is rasterized on
the CPU. That cost is real and it is the reason the interface is careful about
how many of those it asks for.

**`NO_AT_BRIDGE=1` is set unless overridden.** WebKitGTK has a known deadlock
with at-spi2: with an AT-SPI registry running, interaction in the webview
triggers synchronous ATK D-Bus queries that deadlock the GTK main loop. Turning
the bridge off avoids the hang — and turns off the accessibility bridge, so a
screen reader sees nothing. Every `aria-label` and every `role` in this
application is invisible on Linux, which makes several of the accessibility
assertions elsewhere in the suite true and inert.

Both are guarded: an explicit environment variable in either direction is left
alone, and BUG-015 already narrowed what counts as "explicit". Neither is a bug
in the code. What is missing is evidence from more than one machine.

## What it would take

- **Profiling the release build on representative GPUs.** Intel, AMD and NVIDIA,
  on Wayland and on X11, on a driver generation more recent than the one
  FEAT-055 measured. The output is the same table that file already carries,
  with more rows and a date.
- **A scoped workaround rather than a blanket one.** If the failure is a driver
  or WebKitGTK version, the check belongs on that version rather than on
  "Wayland". The current policy function is pure and table-driven precisely so
  that a narrower rule is an edit to a table.
- **An accessible path.** Whether a newer WebKitGTK still deadlocks; whether
  the bridge can be left on with a different at-spi configuration; and, failing
  both, whether the trade should be a preference rather than a default. A
  screen-reader user on Linux currently has no way to turn it back on except an
  environment variable nothing tells them about.

## Why it is not being done now

There is one machine, and it is the machine FEAT-055 already measured. Guessing
at a narrower rule would replace evidence with a hope, and removing the
workaround on appearance grounds is what the review explicitly says not to do.
The honest state is that this is known, argued and unmeasured.

## Dependencies

FEAT-055 measured the rendering path and wrote the table. BUG-004 and BUG-015
are the two defects that produced the current policy. TASK-042 records this as
its own non-scope.
