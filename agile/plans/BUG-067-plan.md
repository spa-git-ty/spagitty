<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-067 — Plan

**Item:** [BUG-067](../items/BUG-067-review-says-the-token-is-invalid.md)

Reading both halves of *All my repos* showed they agree on the account when a repository is open; what differs is the *current user*, taken from the first account. Fix that, name the account in a refusal, and ask once more after a refusal on the first read, since the same token is accepted a moment later. Test the host's account against two connected accounts, the retry and the repeated refusal, and Review opened with two accounts.
