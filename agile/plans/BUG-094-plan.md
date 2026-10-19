<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-094 — Plan

**Item:** [BUG-094](../items/BUG-094-a-resumed-agent-skips-files-that-changed-since.md)

1. `same_revisions(a, b)`: the same job (`same_job`) and, for a review, the same base and head; for a merge, the same two tips and base. The title and the strategy are not revisions.
2. In `assignment_start`, load the earlier run as before, then keep it only when `same_revisions` holds with the request's target.
3. When it does not, start fresh and add to the note: *This was asked to carry on an earlier run, but what it works on has moved since, so it starts over.*
4. Unit-test both kinds of target.
