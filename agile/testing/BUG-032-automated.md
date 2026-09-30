<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-032 — Automated test record

**Item:** [`agile/items/BUG-032-a-path-test-that-only-passes-on-unix.md`](../items/BUG-032-a-path-test-that-only-passes-on-unix.md)

## What was tested

No test was added. The defect is a test.

## Test command and output

On Windows 11, before:

```
$ bun run test
Tests  1 failed | 2912 passed (2913)
```

After:

```
$ bun run test
Test Files  137 passed (137)
     Tests  2917 passed (2917)
```

The count rises by four because the record test checks this item's own documents.

## What is not covered automatically

The pipeline still runs the frontend suite on Ubuntu only, so a new
separator-dependent test would again pass there and fail on Windows.
