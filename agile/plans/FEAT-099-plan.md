<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-099 — implementation plan

Implemented together on the existing `feature/FEAT-097-coderabbit-reviews-local-changes` checkout to preserve Claude’s work.

Introduce typed GitHub snapshots in the core through the existing HTTP seam.
Distinguish issue discussion from inline review endpoints; retain actor/app IDs,
revision fields, partial pagination and unknown resolution. Re-read metadata to
notice pushes. Keep forge credentials in the backend. Map public data in the
normal worker and generic panel. Request incremental/full discussion comments
only after host confirmation; store uncertainty before writing, make one POST,
and require complete discussion refresh before deliberate resend. Test pure
mapping and the real worker/host, including grants, stale revisions and ambiguity.
