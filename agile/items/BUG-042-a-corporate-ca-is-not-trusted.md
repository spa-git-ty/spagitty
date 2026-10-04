<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-042 — A corporate CA is not trusted

**Status:** Fixed.
**Branch:** `bugfix/BUG-042-a-corporate-ca-is-not-trusted`
**Screens:** Settings (adding a forge token), Pull requests.
**Raised by:** the author, adding a token for a self-hosted GitLab: "could not
reach gitlab.apps.ocp-nonprod-01.hodomain.local: native-tls: unable to find
any user-specified roots in the final cert chain".

## Problem

The GitLab's certificate is issued by a private corporate CA that Windows
trusts: the browser opens the site, and a `SslStream` handshake from the same
machine reports no policy errors. Spagitty refused it.

`forge/http.rs` promised the platform's certificate store, and FEAT-017 chose
`native-tls` for exactly this case. But ureq's `TlsConfig` defaults
`root_certs` to `RootCerts::WebPki`, the bundled Mozilla list, and under
native-tls that also calls `disable_built_in_roots(true)`. The platform store
was switched off, so any host signed by a CA outside the public list failed.
BUG-011 named the provider and left the roots at their default.

## Scope

- The agent trusts the platform's certificate store.
- Verification stays on. No option to accept an untrusted certificate.

## Acceptance criteria

- A host whose certificate chains to a CA in the operating system's store is
  reached, and a token can be added for it.
- The agent's configured roots are `PlatformVerifier`, asserted in a test.
