<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-166 — Copy a call as cURL, and paste one in

**Status:** Backlog — after FEAT-161.
**Screens:** 2C (Calls: the cURL window).
**Raised by:** the author, 2026-10-25: "a popup to export or import a curl". The proposal had *Paste cURL* only inside Import (FEAT-162) and nothing to copy a call out.

## Change

A *cURL* window on Calls, opened from the request bar, has two sides:

- **Copy.** The open call as one `curl` command, with variables resolved in the active environment. Secrets and kept values stay as `{{name}}` unless *Fill in secrets* is ticked; it is off every time the window opens.
- **Paste.** A `curl` command typed or pasted in becomes a call in the chosen collection and folder, or replaces the open call's request. The window says which flags it read and which it ignored.

## Acceptance criteria

- **Copy** quotes for the platform's shell (POSIX on Linux and macOS, `cmd`-safe double quotes on Windows), with a choice to switch. It writes the method only when it is not implied, the headers, the body (`--data-raw`, `--form` or `--data-urlencode`), `-u` for basic auth, `-L` when redirects are followed, `-k` when the environment skips TLS checks, and `--compressed`.
- **Paste** reads `-X`/`--request`, `-H`/`--header`, `-d`/`--data`/`--data-raw`/`--data-binary`/`--data-urlencode`, `-F`/`--form`, `-u`/`--user`, `-b`/`--cookie`, `-A`/`--user-agent`, `-e`/`--referer`, `-G`/`--get`, `-I`/`--head`, `-L`, `-k`, `--url`, and line continuations (`\`, `^` and the backtick). The body type is guessed from `Content-Type`, or from the text when there is none.
- **SOAP**: a SOAP call copies as a POST with its envelope and SOAP headers.
- **Tests**: Rust round trips (a call copied, pasted back and compared) for each body type and both quoting styles, the flags above, and a test that no secret appears in a copy unless asked. A component test for the window.

## Not in this item

- Other formats to copy as (HTTPie, fetch, a language's client).
