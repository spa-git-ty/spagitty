<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-163 — SOAP calls from a WSDL

**Status:** Backlog — after FEAT-162.
**Screens:** 2C (Calls, with a SOAP call open; Import's SOAP sources).
**Raised by:** the author, 2026-10-25: Calls supports REST and SOAP.
**Proposal:** [`docs/proposals/calls.md`](../../docs/proposals/calls.md) §7. **Designs:** [`design_handoff_calls/`](../../design_handoff_calls/) — `SoapCall.dc.html`, `Scan.dc.html`.

## Change

A SOAP collection is made from a WSDL, with one call per operation. Spagitty builds the envelope and sets the SOAP headers, adds WS-Security, shows faults as errors, and keeps values by XPath, using the same environments as REST.

## Acceptance criteria

- **From a WSDL** (a file in the repository, a file from outside, or a URL the person types): one folder per service, one call per operation, and the WSDL path kept on the collection. WSDL 1.1 with SOAP 1.1 and 1.2 bindings; imported XSDs within the repository are followed.
- **From code**: JAX-WS `@WebService` and `@WebMethod`, and Spring-WS `@Endpoint` and `@PayloadRoot`. A WSDL in the repository is used when one exists. Otherwise the envelope is built from the request class.
- **The request**:
  - The version shown (from the WSDL where it says), the operation picker and the endpoint URL with its host as a variable.
  - Tabs *Envelope*, *Headers*, *WS-Security*, *Keep*.
  - *Rebuild from WSDL* keeps typed values where elements still exist.
- **The envelope** has every element, a sample value, and a comment with its XSD type and whether it is required. Variables work as in REST.
- **Headers**: `SOAPAction` (1.1) or the `action` parameter of `Content-Type` (1.2), set from the WSDL and shown.
- **WS-Security**: *None*; *Username token* (text or digest, nonce, created); *Signed* (an X.509 key from the keychain, a timestamp, a signed body).
- **Faults**: shown as an error with code, string and detail, even on HTTP 200. The status line says *no fault* otherwise.
- **Keeping**: XPath with the response's own prefixes, with the namespaces stored on the rule.
- **Export to Postman** (with FEAT-164) writes a SOAP call as a raw POST.
- **Tests**:
  - Rust tests over fixture WSDLs: document/literal, rpc/literal, 1.1 and 1.2.
  - Envelope generation, headers, both WS-Security modes, fault parsing, XPath keeping.
  - Component tests for the request view.

## Not in this item

- WSDL 2.0, MTOM and attachments, WS-Addressing, WS-ReliableMessaging.
