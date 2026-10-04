<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-042 — Plan

**Item:** [`agile/items/BUG-042-a-corporate-ca-is-not-trusted.md`](../items/BUG-042-a-corporate-ca-is-not-trusted.md)

## Approach

Set `root_certs(RootCerts::PlatformVerifier)` on the agent's `TlsConfig`, next
to the provider BUG-011 named. With native-tls that leaves the built-in roots
enabled, which is the platform store: SChannel on Windows, Security.framework
on macOS, OpenSSL's system store on Linux.

Diagnosed from ureq 3.4.0's `tls/native_tls.rs`: `WebPki` disables the built-in
roots and adds the bundled list; `PlatformVerifier` keeps them.

## Files

| File | Change |
| --- | --- |
| `crates/spagitty-core/src/forge/http.rs` | The roots, chosen; a test that reads them back. |

## Risks and rollback

- A host signed only by a CA in Mozilla's list but missing from the platform
  store would now be refused. Every mainstream OS carries those roots, so this
  is the same trust the browser on that machine already uses. Rollback is a
  revert.
