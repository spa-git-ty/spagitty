<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-042 — Automated test record

**Item:** [`agile/items/BUG-042-a-corporate-ca-is-not-trusted.md`](../items/BUG-042-a-corporate-ca-is-not-trusted.md)

## What was tested

`forge::http::tests::the_agent_trusts_the_platform_certificate_store_not_a_bundled_one`:
the agent's TLS config reads back `RootCerts::PlatformVerifier`.

## Test command and output

On Windows 11: `cargo test -p spagitty-core --lib forge::http` — 9 passed.

## What is not covered automatically

A real handshake with a private CA, which needs that CA and a host. Checked on
the author's machine with a throwaway test, removed afterwards: `get_json` on
`https://gitlab.apps.ocp-nonprod-01.hodomain.local/api/v4/version` with no
token returned `Response { status: 401 }`. The handshake completed, and 401
is the host's answer to a request without a token.
