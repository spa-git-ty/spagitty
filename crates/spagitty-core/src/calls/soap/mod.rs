// SPDX-License-Identifier: GPL-3.0-or-later

//! SOAP calls (FEAT-163, proposal §7): a WSDL read into services and
//! operations (`wsdl`), envelopes made from its schemas (`xsd`), the
//! username token of WS-Security (`security`), and a fault read out of a
//! response ([`fault`]).

pub mod security;
pub mod wsdl;
pub mod xsd;

use serde::{Deserialize, Serialize};

use super::xml;

pub const ENVELOPE_11: &str = "http://schemas.xmlsoap.org/soap/envelope/";
pub const ENVELOPE_12: &str = "http://www.w3.org/2003/05/soap-envelope";

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Version {
    #[default]
    #[serde(rename = "1.1")]
    V11,
    #[serde(rename = "1.2")]
    V12,
}

impl Version {
    pub fn from_text(text: Option<&str>) -> Version {
        if text == Some("1.2") {
            Version::V12
        } else {
            Version::V11
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Version::V11 => "1.1",
            Version::V12 => "1.2",
        }
    }
}

/// `body` (already indented under `Body`) in an envelope of `version`,
/// declaring `prefixes` (namespace, prefix) on it.
pub fn envelope(version: Version, body: &str, prefixes: &[(String, String)]) -> String {
    let (p, ns) = match version {
        Version::V11 => ("soapenv", ENVELOPE_11),
        Version::V12 => ("soap", ENVELOPE_12),
    };
    let mut declared = format!("xmlns:{p}=\"{ns}\"");
    for (uri, prefix) in prefixes {
        declared.push_str(&format!(" xmlns:{prefix}=\"{}\"", xml::escape(uri)));
    }
    format!("<{p}:Envelope {declared}>\n  <{p}:Header/>\n  <{p}:Body>\n{body}  </{p}:Body>\n</{p}:Envelope>")
}

/// A SOAP fault, as the response view shows it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Fault {
    pub code: String,
    pub reason: String,
    /// The detail as it was sent, XML and all.
    pub detail: Option<String>,
}

/// The fault in a SOAP 1.1 or 1.2 response, or `None` when it has none.
pub fn fault(body: &str) -> Option<Fault> {
    let root = xml::parse(body)?;
    if root.local() != "Envelope" {
        return None;
    }
    let fault = root.first("Body")?.first("Fault")?;
    let text = |e: Option<&xml::Element>| {
        e.map(|e| e.all_text().trim().to_string())
            .unwrap_or_default()
    };
    // 1.1: faultcode, faultstring, detail. 1.2: Code/Value, Reason/Text, Detail.
    let code = match fault.first("faultcode") {
        Some(code) => code.text.trim().to_string(),
        None => {
            let code = fault.first("Code");
            let mut value = text(code.and_then(|c| c.first("Value")));
            if let Some(sub) = code
                .and_then(|c| c.first("Subcode"))
                .and_then(|s| s.first("Value"))
            {
                value.push_str(&format!(" / {}", sub.all_text().trim()));
            }
            value
        }
    };
    let reason = match fault.first("faultstring") {
        Some(reason) => reason.all_text().trim().to_string(),
        None => text(fault.first("Reason").and_then(|r| r.first("Text"))),
    };
    let detail = fault
        .first("detail")
        .or_else(|| fault.first("Detail"))
        .map(|d| inner_xml(body, d))
        .filter(|d| !d.trim().is_empty());
    Some(Fault {
        code,
        reason,
        detail,
    })
}

/// What is between a detail element's tags in the text as sent, or its
/// text when that cannot be found.
fn inner_xml(body: &str, element: &xml::Element) -> String {
    let open = format!("<{}", element.name);
    let close = format!("</{}>", element.name);
    if let Some(start) = body.find(&open) {
        if let Some(after) = body[start..].find('>') {
            let from = start + after + 1;
            if let Some(end) = body[from..].find(&close) {
                return body[from..from + end].trim().to_string();
            }
        }
    }
    element.all_text().trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn faults_are_read_in_both_versions() {
        let v11 = r#"<soapenv:Envelope xmlns:soapenv="http://schemas.xmlsoap.org/soap/envelope/"><soapenv:Body><soapenv:Fault><faultcode>soapenv:Client</faultcode><faultstring>Invoice INV-9 not found</faultstring><detail><inv:NotFound xmlns:inv="urn:inv"><inv:id>INV-9</inv:id></inv:NotFound></detail></soapenv:Fault></soapenv:Body></soapenv:Envelope>"#;
        assert_eq!(
            fault(v11),
            Some(Fault {
                code: "soapenv:Client".into(),
                reason: "Invoice INV-9 not found".into(),
                detail: Some(
                    r#"<inv:NotFound xmlns:inv="urn:inv"><inv:id>INV-9</inv:id></inv:NotFound>"#
                        .into()
                )
            })
        );
        let v12 = r#"<env:Envelope xmlns:env="http://www.w3.org/2003/05/soap-envelope"><env:Body><env:Fault><env:Code><env:Value>env:Sender</env:Value><env:Subcode><env:Value>m:Invalid</env:Value></env:Subcode></env:Code><env:Reason><env:Text xml:lang="en">Bad id</env:Text></env:Reason></env:Fault></env:Body></env:Envelope>"#;
        let read = fault(v12).unwrap();
        assert_eq!(read.code, "env:Sender / m:Invalid");
        assert_eq!(read.reason, "Bad id");
        assert_eq!(read.detail, None);
        assert_eq!(fault("<soapenv:Envelope xmlns:soapenv=\"x\"><soapenv:Body><ok/></soapenv:Body></soapenv:Envelope>"), None);
        assert_eq!(fault("{}"), None);
    }

    #[test]
    fn an_envelope_declares_its_prefixes() {
        assert_eq!(
            envelope(Version::V12, "    <a:x/>\n", &[("urn:a".into(), "a".into())]),
            "<soap:Envelope xmlns:soap=\"http://www.w3.org/2003/05/soap-envelope\" xmlns:a=\"urn:a\">\n  <soap:Header/>\n  <soap:Body>\n    <a:x/>\n  </soap:Body>\n</soap:Envelope>"
        );
    }
}
