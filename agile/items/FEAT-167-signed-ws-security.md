<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-167 — Signed WS-Security for SOAP calls

**Status:** Backlog — split out of FEAT-163.
**Screens:** 2C (a SOAP call's *WS-Security* tab), 1K (Settings › Calls).
**Raised by:** the author, 2026-10-25, answering whether *Signed* belonged in 4.0.0: "Username token now, Signed later". In FEAT-163, *Signed* is drawn and disabled.

## Change

*Signed* WS-Security: an X.509 certificate and private key from the keychain, a `wsu:Timestamp`, and an XML-DSig signature over the body and the timestamp.

## Acceptance criteria

- The key and certificate are chosen once per environment and kept in the keychain, never in `.spagitty/`.
- Exclusive canonicalisation (`exc-c14n`), RSA-SHA256 and SHA-256 digests. A `BinarySecurityToken` carries the certificate.
- A response's signature is not checked.
- **Tests**: signatures checked against a reference verifier over fixture envelopes. A test that the key never reaches a file.

## Not in this item

- Encryption (`xenc`), SAML tokens, WS-SecureConversation.
