<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-067 — Review says the token is invalid until Pull requests has been opened

**Status:** Fixed — shipped in 2.1.0. The cold-start fault was never reproduced; the fix removes what could be read from the code and adds a retry and a clearer message; the manual sweep is still owed.
**Screens:** 1R.
**Raised by:** the author, 2026-10-11, on 2.0.1.

## What happens

1. Open a repository and go straight to Review.
2. Review says the token was refused.
3. Open Pull requests, then go back to Review: it works, with the same token.

The token is fine. The message sends the reader to Settings to fix something that is not broken.

## Suspected cause

Not reproduced yet. Both halves of *All my repos* pick an account when the repository's host is not known yet:

- `src/lib/review/store.svelte.ts`, `loadInvolved()`: the host is `requests.repo?.host`, which is only set once the Pull requests store has loaded. Until then it falls back to `accounts[0]`.
- `src-tauri/src/commands.rs`, `involved_pull_requests`: the account is the open repository's host when `forge_repo` resolves it, and otherwise `connected.first()`.

With two accounts connected (for example GitHub and the work GitLab), the first one can be asked instead of the repository's, and its refusal is shown as this repository's. Opening Pull requests sets the host, which matches *it works after I open Pull requests*.

## Acceptance criteria

- On a cold start, Review asks with the account for the open repository's host, and gets the same answer Pull requests would.
- A refusal names the host and the account that was refused, so a refusal of the wrong account would be visible as such.
- A test covers Review opened before Pull requests with two accounts connected.

## For the agent who picks this up

**Who:** unassigned. Any agent working on this repository can start cold.

- **Reproduce first:** confirm which host and account the failing request went to (log it), and whether `requests.repo` is null at that moment. If the room rather than the inbox shows the error, follow `review.open()` instead.
- **Where:** `src/lib/review/store.svelte.ts` (`prime`, `loadInvolved`), `src-tauri/src/commands.rs` (`involved_pull_requests`), `crates/spagitty-core/src/forge.rs` (`status_error`).
- **Branch:** `bugfix/BUG-067-review-says-the-token-is-invalid`, with plan, testing documents and a changelog entry when the work starts.

## What was done, and what was not

Reading the code did not confirm the suspected cause: the backend's `involved_pull_requests` and `pull_requests` both ask with the account for the open repository's host, and Review's own list is the Pull requests list (`requests.load()`), so Review and Pull requests run the same request. What was found and fixed:

- `requests.load()` took *you* from the first connected account, not from the repository's host, so with two accounts the inbox grouped pull requests for the wrong person. It now picks the account for the repository's host.
- A refusal now says which account was refused (`… (signed in as <login>)`), after the host that the sentence already named.
- A first read refused, with nothing changed, is read once more after 700 ms before the reader is told to fix their account; a repeated refusal is shown as before. Both lists do this, the repository's own and *All my repos* (`askAgainAfterRefusal`); the second was added on 2026-10-12.

The cause of *refused on first open, fine after Pull requests* is still unknown. The retry is a mitigation, not a diagnosis. When it happens again the message now names the host and account; if the second read is what succeeds, that points at the first read racing something at start-up (the keychain is the first suspect).
