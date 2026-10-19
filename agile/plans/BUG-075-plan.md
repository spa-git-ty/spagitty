<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-075 — Plan

**Item:** [BUG-075](../items/BUG-075-a-reply-that-cannot-be-read-is-taken-as-empty.md)

1. Replace `read_to_string().unwrap_or_default()` in `finish` with a `read_body` that takes any reader and a limit.
2. Read one byte past the limit so a reply that is too large is told apart from one that fits exactly; report a read error as a reply cut off, and bytes that are not UTF-8 as not text.
3. For a success status, a body that cannot be read is `Error::Model` with that reason, passed through `redact`. For an error status, keep returning the status with an empty body, so the caller still reports it.
4. Unit-test `read_body` over a reader that fails part-way, one over the limit, one exactly at it, and bytes that are not text.
