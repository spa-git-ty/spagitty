<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-114 — Pull request notifications

**Status:** Done — merged 2026-10-08, ships in 1.3.0. The native sweep on macOS, Windows and Linux is still to run.
**Branch:** `main`.
**Screens:** Settings → Notifications, app shell.
**Raised by:** the author, 2026-10-08.

## Change

While Spagitty is open, look at every connected account's pull requests on a timer and say when something happens: in the corner, as an operating-system notification, and with Spagitty's own `notification` sound at the chosen Sound level. Hosts cannot push to a desktop app, so this polls.

## Acceptance criteria

- Settings → Notifications: one switch, off by default. Under it: system notifications on or off, the kinds (merged or closed, comments and reviews, review requests, failing checks), how often to look (1, 2, 5 or 15 minutes), Check now, and Send a test.
- Covers the person's own pull requests including merged and closed ones, the ones they are asked to review, and open ones they are otherwise in. GitHub and GitLab; Bitbucket reports nothing.
- The first read after switching on announces nothing. Switching off forgets the last read. A merge that happens while Spagitty is closed is announced on the next start.
- The person's own comment is not news on GitHub. GitLab's list does not say who spoke last, so there it is counted.
- The OS notification carries no system sound. Spagitty plays its own cue, and Sound Off means silence.
- One account failing does not hide the others; an error shows on the Settings section, never as a timed popup.
- New dependency: `tauri-plugin-notification`, called from Rust only, so the frontend lockfile is unchanged.
