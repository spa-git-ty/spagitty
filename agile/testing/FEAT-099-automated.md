<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-099 — automated verification

2026-10-06.

- Passed: core snapshot tests preserve actors/apps, separate discussion from inline endpoints, paginate to bounded limits, retain partial/error data, reject unsupported forges, make exactly one POST and re-check cancellation after metadata.
- Passed: worker mapping tests cover human mentions, numeric service identities, app attribution, exact/stale revisions, incomplete data and oversized content.
- Passed: real host/worker PR integration covers incremental/full discussion requests, requested versus completed versus stale, optional write grants, uncertain delivery after partial/complete refresh, and malformed receipts preventing duplicate writes.
- All tests use the backend Services seam; no token enters the worker or UI.
- Final counts and whole-suite evidence are in extensions-continuation-review.md.

An authenticated live GitHub PR smoke test is unverified.
