<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-075 — A model's reply that cannot be read is taken as an empty one

**Status:** Fixed — on `release/3.4.0`; unmerged. Covered by unit tests; not tried against a real provider.
**Screens:** 1K, 1R, 1S.
**Raised by:** the audit; found by reading `crates/spagitty-core/src/models/http.rs`.

## What happens

`finish` reads a provider's answer with `.read_to_string().unwrap_or_default()`. A reply larger than the 8 MiB limit, a connection cut in the middle, or a body that is not UTF-8 all become an empty string with a good status. The caller then reports a reply with nothing in it, or a parse failure on `""`, and the person is told the model said nothing when the transport failed. It sits beside BUG-071 and BUG-072 in what a person sees when an API agent does not work.

## Acceptance criteria

- A body that cannot be read is an error that says why: too large, cut off, or not text. It does not come back as an answer.
- A provider's error body that cannot be read still reports the status.
- A unit test over a reader that fails part-way and one over the size limit.

## For the agent who picks this up

**Who:** unassigned. Any agent working on this repository can start cold.

- **Where:** `crates/spagitty-core/src/models/http.rs`, `finish`. Keep the key out of the new message (`redact`).
