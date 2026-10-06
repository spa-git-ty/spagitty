<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-098 — implementation plan

Implemented together on the existing `feature/FEAT-097-coderabbit-reviews-local-changes` checkout to preserve Claude’s work.

Declare the provider contract in the farm and implement it in desktop composition.
Keep Off as default; persist policy, record person-attributed revisions, retain exact
public evidence and bounded repair counts. Run after verification only under
existing review autonomy and consent. Enforce Required at the common manual/auto
merge boundary, including fresh readiness and a post-read task check. Preserve
independent AgentId and verification evidence. Selected findings use existing
change requests, retain IDs and never stage or modify the original checkout.
Test real worktrees, restart, stale commits/policy, missing providers, cancellation,
repair budget and independent-review authority.
