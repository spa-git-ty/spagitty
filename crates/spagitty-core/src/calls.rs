// SPDX-License-Identifier: GPL-3.0-or-later

//! Calls (2C): REST and SOAP requests beside the code (docs/proposals/calls.md).
//!
//! A collection is plain JSON in the repository, under `.spagitty/calls/`, so
//! a call is reviewed and committed with the code it exercises. What belongs
//! to the person — the active environment, open calls, history — is in
//! application data, and what is secret is in the keychain.
//!
//! | Module | Holds |
//! | --- | --- |
//! | `model` | The file formats, and the rule that an environment file never holds a secret |
//! | `store` | The calls folder: list, read, write, rename, delete |
//! | `vars` | `{{name}}`: found, resolved, redacted; the built-ins |
//! | `build` | A saved call made into the request to send, and the copy history keeps |
//! | `send` | The only file here that reaches a network |
//! | `body` | What a response body is: JSON, XML, an event stream, binary |
//! | `history` | Each send, per repository, in application data |
//! | `soap` | SOAP: a WSDL and its schemas, envelopes, WS-Security, faults |
//! | `settings` | Settings › Calls, and a repository's overrides |
//! | `import` | Import: endpoints from Java, NestJS and OpenAPI, calls from collection files |
//! | `keep` | Values kept from responses: JSONPath, XPath, and where each came from |
//! | `vault` | Secret and kept values, in the keychain |
//! | `xml` | XML as a small tree with its namespaces in scope |

pub mod body;
pub mod build;
pub mod history;
pub mod import;
pub mod keep;
pub mod model;
pub mod send;
pub mod settings;
pub mod soap;
pub mod store;
pub mod vars;
pub mod vault;
pub mod xml;
