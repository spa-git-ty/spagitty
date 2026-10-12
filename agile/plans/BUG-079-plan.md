<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-079 — Plan

**Item:** [BUG-079](../items/BUG-079-all-resolved-is-drawn-as-a-button.md)

One element did two jobs: a way to the next unresolved conflict, and a statement that there is none. Split them. While something is left, the button; once nothing is, a `span` that says so, with `role="status"` so the change is announced.

Not done by disabling the button: a disabled button is still a button, dimmed, and the words would read as something that will be available later.

## What decides it is right

- With every conflict resolved there is no button named *All resolved* or *Next unresolved* in the pill, and one status that reads *All resolved*.
