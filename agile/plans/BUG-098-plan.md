<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-098 — Plan

**Item:** [BUG-098](../items/BUG-098-pull-requests-says-only-500-internal-server-error.md)

1. `gitlab.rs`: `merge_request_list` for both lists, over a `Fetch` so a test can answer; on a 5xx, `one_by_one`: the simple view, then each merge request by its project and iid on six threads, kept in order, failures left out.
2. `forge.rs`: `status_error` says a 5xx is the server's failure.
