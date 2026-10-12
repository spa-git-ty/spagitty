<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-066 — Plan

**Item:** [BUG-066](../items/BUG-066-extensions-refuse-spagitty-2.md)

Drop the upper bound from the four manifests, write the reason in the developer guide, and add a test that reads each shipped manifest and the application version and fails when one does not admit the other, so the next major release cannot ship the same fault. The bundle copy is generated from `extensions/coderabbit/extension.json` and is rebuilt, not hand-edited, in a release.
