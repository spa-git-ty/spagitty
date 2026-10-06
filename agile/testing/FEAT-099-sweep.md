<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-099 — manual verification

2026-10-06. Live GitHub and application UI checks are unverified.

Use a disposable GitHub PR connected through Spagitty to inspect bot summaries/inline locations, request one incremental and one full review through the preview, refresh, push a new head, and cancel/navigate while waiting. Confirm that an uncertain delivery requires refresh and explicit resend.
No external comments, approval, resolution, merge or branch-protection changes were made during deterministic verification.
GitLab and Bitbucket remain unsupported for the CodeRabbit PR integration, with an explicit backend refusal.
