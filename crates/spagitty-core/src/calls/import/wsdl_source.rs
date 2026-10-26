// SPDX-License-Identifier: GPL-3.0-or-later

//! A WSDL as Import's endpoints (FEAT-163, proposal §7.1): one folder per
//! service and one call per operation, on the port of the version Settings
//! prefers when the service has both. The address's scheme and host become
//! `{{baseUrl}}`, and the envelope is made from the schemas unless Settings
//! says to start empty.

use std::collections::HashMap;

use super::super::model::{new_id, Call, Kind, WsMode, WsSecurity};
use super::super::soap::wsdl::{self, Wsdl};
use super::super::soap::{envelope, Version};
use super::{key_of, Endpoint, Status};

/// What Settings › Calls says a SOAP call starts with.
#[derive(Debug, Clone)]
pub struct SoapChoices {
    pub version: Version,
    pub envelope_from_wsdl: bool,
    /// `none` or `username`.
    pub ws_security: String,
}

impl Default for SoapChoices {
    fn default() -> Self {
        SoapChoices {
            version: Version::V11,
            envelope_from_wsdl: true,
            ws_security: "username".into(),
        }
    }
}

/// `https://billing.example.com/ws/x` as its server and the rest.
pub fn split_address(address: &str) -> (Option<String>, String) {
    match address.split_once("://") {
        Some((scheme, rest)) => {
            let at = rest.find('/').unwrap_or(rest.len());
            (
                Some(format!("{scheme}://{}", &rest[..at])),
                rest[at..].to_string(),
            )
        }
        None => (None, address.to_string()),
    }
}

/// The calls for `wsdl`, read from `file`, and the server its first
/// address names.
pub fn endpoints(
    file: &str,
    wsdl: &Wsdl,
    choices: &SoapChoices,
) -> (Vec<Endpoint>, Option<String>) {
    let mut out = Vec::new();
    let mut server = None;
    for service in &wsdl.services {
        let Some(port) = service
            .ports
            .iter()
            .find(|p| p.version == choices.version)
            .or_else(|| service.ports.first())
        else {
            continue;
        };
        let (host, path) = split_address(&port.address);
        if server.is_none() {
            server = host;
        }
        for operation in &port.operations {
            let text = if choices.envelope_from_wsdl {
                wsdl.envelope(operation, port.version, HashMap::new())
            } else {
                envelope(port.version, "", &[])
            };
            let ws_security = (choices.ws_security == "username").then(|| WsSecurity {
                mode: WsMode::Username,
                user: Some("{{user}}".into()),
                password: Some("{{password}}".into()),
                digest: false,
            });
            let call = Call {
                id: new_id(),
                name: operation.name.clone(),
                kind: Kind::Soap,
                url: format!("{{{{baseUrl}}}}{path}"),
                service: Some(service.name.clone()),
                port: Some(port.name.clone()),
                operation: Some(operation.name.clone()),
                version: Some(port.version.as_str().into()),
                action: (!operation.action.is_empty()).then(|| operation.action.clone()),
                envelope: Some(text),
                ws_security,
                ..Call::default()
            };
            out.push(Endpoint {
                id: format!("{file}#{}", out.len()),
                key: key_of(&call),
                source: format!("file:{file}"),
                group: service.name.clone(),
                file: Some(file.to_string()),
                line: None,
                method: "SOAP".into(),
                path: format!("{} › {}", service.name, operation.name),
                hint: format!(
                    "SOAP {}{}",
                    port.version.as_str(),
                    if operation.rpc { " · rpc" } else { "" }
                ),
                call,
                status: Status::New,
                declared: None,
                folder: Some(service.name.clone()),
            });
        }
    }
    (out, server)
}

/// Read a WSDL at `file` with `load` for what it imports, as endpoints.
pub fn read(
    file: &str,
    text: &str,
    load: &dyn Fn(&str) -> Option<String>,
    choices: &SoapChoices,
) -> Result<(Vec<Endpoint>, Option<String>), String> {
    let parsed = wsdl::parse(text, file, load)?;
    Ok(endpoints(file, &parsed, choices))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::calls::soap::wsdl::tests::{load, INVOICES};

    #[test]
    fn a_wsdl_is_a_call_per_operation_on_the_preferred_port() {
        let (found, server) = read(
            "wsdl/invoices.wsdl",
            INVOICES,
            &load,
            &SoapChoices::default(),
        )
        .unwrap();
        assert_eq!(server.as_deref(), Some("https://billing.example.com"));
        assert_eq!(found.len(), 1);
        let call = &found[0].call;
        assert_eq!(found[0].key, "SOAP BillingService#GetInvoice");
        assert_eq!(found[0].folder.as_deref(), Some("BillingService"));
        assert_eq!(call.url, "{{baseUrl}}/ws/invoices");
        assert_eq!(call.version.as_deref(), Some("1.1"));
        assert_eq!(call.action.as_deref(), Some("urn:billing:GetInvoice"));
        assert!(call
            .envelope
            .as_deref()
            .unwrap()
            .contains("<inv:invoiceId>invoiceId</inv:invoiceId>"));
        assert_eq!(call.ws_security.as_ref().unwrap().mode, WsMode::Username);

        let twelve = SoapChoices {
            version: Version::V12,
            envelope_from_wsdl: false,
            ws_security: "none".into(),
        };
        let (found, _) = read("wsdl/invoices.wsdl", INVOICES, &load, &twelve).unwrap();
        let call = &found[0].call;
        assert_eq!(call.url, "{{baseUrl}}/ws12/invoices");
        assert_eq!(call.port.as_deref(), Some("Invoices12Port"));
        assert!(!call.envelope.as_deref().unwrap().contains("GetInvoice"));
        assert!(call.ws_security.is_none());
    }
}
