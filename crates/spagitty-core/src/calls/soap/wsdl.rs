// SPDX-License-Identifier: GPL-3.0-or-later

//! WSDL 1.1 (FEAT-163, proposal §7.1): services, their SOAP 1.1 and 1.2
//! ports, and each operation's action, style and input.
//!
//! The schemas in `types` are read, and a `xs:import` or `xs:include` (or a
//! `wsdl:import`) whose location is a file beside the WSDL is followed
//! through `load`; a location that is a URL is not fetched.

use std::collections::{BTreeMap, HashMap};

use super::super::xml::{self, Element};
use super::xsd::{Schemas, Writer};
use super::{envelope, Version};

const SOAP11: &str = "http://schemas.xmlsoap.org/wsdl/soap/";
const SOAP12: &str = "http://schemas.xmlsoap.org/wsdl/soap12/";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Part {
    pub name: String,
    /// `element=`: a global element.
    pub element: Option<(Option<String>, String)>,
    /// `type=`: an element named as the part, of this type.
    pub ty: Option<(Option<String>, String)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Operation {
    pub name: String,
    pub action: String,
    pub rpc: bool,
    /// An rpc operation's wrapper namespace, from `soap:body`.
    pub namespace: Option<String>,
    pub input: Vec<Part>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Port {
    pub name: String,
    pub version: Version,
    pub address: String,
    pub operations: Vec<Operation>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Service {
    pub name: String,
    pub ports: Vec<Port>,
}

#[derive(Debug, Clone, Default)]
pub struct Wsdl {
    pub target: Option<String>,
    pub services: Vec<Service>,
    /// The prefixes the WSDL declares at its root, for the envelope to use.
    pub prefixes: BTreeMap<String, String>,
    pub schemas: Schemas,
}

/// `a/b/service.wsdl` and `../xsd/types.xsd` as `a/xsd/types.xsd`.
pub fn relative(base: &str, location: &str) -> String {
    let mut parts: Vec<&str> = base.split('/').collect();
    parts.pop();
    for piece in location.split('/') {
        match piece {
            "." | "" => {}
            ".." => {
                parts.pop();
            }
            other => parts.push(other),
        }
    }
    parts.join("/")
}

/// Every schema in `root`'s `types`, and the ones they import or include,
/// with `seen` keeping a file from being read twice.
fn schemas_of(
    root: &Element,
    location: &str,
    load: &dyn Fn(&str) -> Option<String>,
    out: &mut Schemas,
    seen: &mut Vec<String>,
) {
    let add = |schema: &Element, at: &str, out: &mut Schemas, seen: &mut Vec<String>| {
        out.add(schema);
        for child in &schema.children {
            if matches!(child.local(), "import" | "include") {
                let Some(file) = child.attr("schemaLocation") else {
                    continue;
                };
                if file.contains("://") {
                    continue;
                }
                let path = relative(at, file);
                if seen.contains(&path) {
                    continue;
                }
                seen.push(path.clone());
                if let Some(next) = load(&path).as_deref().and_then(xml::parse) {
                    schemas_of(&next, &path, load, out, seen);
                }
            }
        }
    };
    if root.local() == "schema" {
        add(root, location, out, seen);
        return;
    }
    for types in root.all("types") {
        for schema in types.all("schema") {
            add(schema, location, out, seen);
        }
    }
}

/// A binding's operations: action, style where it says, rpc namespace.
type BoundOperations = HashMap<String, (String, Option<bool>, Option<String>)>;
/// A binding: its version, whether it is rpc, its port type, operations.
type Binding = (Version, bool, String, BoundOperations);

/// Read a WSDL. `location` is its path, for its imports; `load` reads one
/// of them.
pub fn parse(
    text: &str,
    location: &str,
    load: &dyn Fn(&str) -> Option<String>,
) -> Result<Wsdl, String> {
    let root = xml::parse(text).ok_or("the WSDL is not well-formed XML")?;
    if root.local() != "definitions" {
        return Err(if root.local() == "description" {
            "this is WSDL 2.0, which Calls does not read; WSDL 1.1 is".into()
        } else {
            "this is not a WSDL: its root is not <definitions>".into()
        });
    }
    let mut seen = vec![location.to_string()];
    let mut documents = vec![(root.clone(), location.to_string())];
    // `wsdl:import` of another WSDL beside this one: its messages and
    // bindings count as this one's.
    for import in root.all("import") {
        if let Some(file) = import.attr("location").filter(|f| !f.contains("://")) {
            let path = relative(location, file);
            if !seen.contains(&path) {
                seen.push(path.clone());
                if let Some(next) = load(&path).as_deref().and_then(xml::parse) {
                    documents.push((next, path));
                }
            }
        }
    }
    let mut wsdl = Wsdl {
        target: root.attr("targetNamespace").map(str::to_string),
        prefixes: root.ns.clone(),
        ..Wsdl::default()
    };
    let mut messages: HashMap<String, Vec<Part>> = HashMap::new();
    let mut port_types: HashMap<String, Vec<(String, Option<String>)>> = HashMap::new();
    let mut bindings: HashMap<String, Binding> = HashMap::new();
    let mut services: Vec<&Element> = Vec::new();
    for (doc, at) in &documents {
        schemas_of(doc, at, load, &mut wsdl.schemas, &mut seen);
        for message in doc.all("message") {
            let parts = message
                .all("part")
                .map(|part| Part {
                    name: part.attr("name").unwrap_or("").to_string(),
                    element: part.attr("element").map(|e| part.qname(e)),
                    ty: part.attr("type").map(|t| part.qname(t)),
                })
                .collect();
            messages.insert(message.attr("name").unwrap_or("").to_string(), parts);
        }
        for port_type in doc.all("portType") {
            let operations = port_type
                .all("operation")
                .map(|op| {
                    let input = op
                        .first("input")
                        .and_then(|i| i.attr("message"))
                        .map(|m| xml::local(m).to_string());
                    (op.attr("name").unwrap_or("").to_string(), input)
                })
                .collect();
            port_types.insert(port_type.attr("name").unwrap_or("").to_string(), operations);
        }
        for binding in doc.all("binding") {
            let Some(soap) = binding.children.iter().find(|c| {
                c.local() == "binding" && matches!(c.uri.as_deref(), Some(SOAP11) | Some(SOAP12))
            }) else {
                continue;
            };
            let version = if soap.uri.as_deref() == Some(SOAP12) {
                Version::V12
            } else {
                Version::V11
            };
            let rpc = soap.attr("style") == Some("rpc");
            let ty = binding
                .attr("type")
                .map(|t| xml::local(t).to_string())
                .unwrap_or_default();
            let mut ops = HashMap::new();
            for op in binding.all("operation") {
                let soap_op = op.children.iter().find(|c| c.local() == "operation");
                let action = soap_op
                    .and_then(|s| s.attr("soapAction"))
                    .unwrap_or("")
                    .to_string();
                let style = soap_op.and_then(|s| s.attr("style")).map(|s| s == "rpc");
                let namespace = op
                    .first("input")
                    .and_then(|i| i.first("body"))
                    .and_then(|b| b.attr("namespace"))
                    .map(str::to_string);
                ops.insert(
                    op.attr("name").unwrap_or("").to_string(),
                    (action, style, namespace),
                );
            }
            bindings.insert(
                binding.attr("name").unwrap_or("").to_string(),
                (version, rpc, ty, ops),
            );
        }
        services.extend(doc.all("service"));
    }
    for service in services {
        let mut ports = Vec::new();
        for port in service.all("port") {
            let binding_name = port.attr("binding").map(xml::local).unwrap_or("");
            let Some((version, rpc, ty, ops)) = bindings.get(binding_name) else {
                continue;
            };
            let address = port
                .children
                .iter()
                .find(|c| c.local() == "address")
                .and_then(|a| a.attr("location"))
                .unwrap_or("")
                .to_string();
            let operations = port_types
                .get(ty)
                .map(|list| {
                    list.iter()
                        .map(|(name, input)| {
                            let (action, style, namespace) =
                                ops.get(name).cloned().unwrap_or_default();
                            Operation {
                                name: name.clone(),
                                action,
                                rpc: style.unwrap_or(*rpc),
                                namespace: namespace.or_else(|| wsdl.target.clone()),
                                input: input
                                    .as_ref()
                                    .and_then(|m| messages.get(m))
                                    .cloned()
                                    .unwrap_or_default(),
                            }
                        })
                        .collect()
                })
                .unwrap_or_default();
            ports.push(Port {
                name: port.attr("name").unwrap_or("").to_string(),
                version: *version,
                address,
                operations,
            });
        }
        wsdl.services.push(Service {
            name: service.attr("name").unwrap_or("").to_string(),
            ports,
        });
    }
    if wsdl.services.iter().all(|s| s.ports.is_empty()) {
        return Err("the WSDL has no SOAP 1.1 or 1.2 port".into());
    }
    Ok(wsdl)
}

impl Wsdl {
    pub fn find(
        &self,
        service: &str,
        port: Option<&str>,
        operation: &str,
    ) -> Option<(&Port, &Operation)> {
        let service = self.services.iter().find(|s| s.name == service)?;
        let port = match port {
            Some(name) => service.ports.iter().find(|p| p.name == name)?,
            None => service.ports.first()?,
        };
        port.operations
            .iter()
            .find(|o| o.name == operation)
            .map(|op| (port, op))
    }

    /// The envelope for `operation` in `version`. `typed` are the values of
    /// an envelope being rebuilt, kept where their element still is.
    pub fn envelope(
        &self,
        operation: &Operation,
        version: Version,
        typed: HashMap<String, String>,
    ) -> String {
        let mut writer = Writer::new(&self.schemas, &self.prefixes, typed);
        if operation.rpc {
            let ns = operation.namespace.clone().unwrap_or_default();
            let p = writer.prefix(&ns);
            writer
                .out
                .push_str(&format!("    <{p}:{}>\n", operation.name));
            for part in &operation.input {
                match (&part.element, &part.ty) {
                    (Some((ns, name)), _) => writer.global(ns, name, 3, &operation.name),
                    (None, Some(ty)) => writer.typed(&part.name, ty, 3, &operation.name),
                    _ => {}
                }
            }
            writer
                .out
                .push_str(&format!("    </{p}:{}>\n", operation.name));
        } else {
            for part in &operation.input {
                match (&part.element, &part.ty) {
                    (Some((ns, name)), _) => writer.global(ns, name, 2, ""),
                    (None, Some(ty)) => writer.typed(&part.name, ty, 2, ""),
                    _ => {}
                }
            }
        }
        envelope(version, &writer.out, &writer.prefixes)
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    pub const INVOICES: &str = r#"<?xml version="1.0"?>
<wsdl:definitions xmlns:wsdl="http://schemas.xmlsoap.org/wsdl/" xmlns:soap="http://schemas.xmlsoap.org/wsdl/soap/"
    xmlns:soap12="http://schemas.xmlsoap.org/wsdl/soap12/" xmlns:xs="http://www.w3.org/2001/XMLSchema"
    xmlns:inv="urn:billing:invoices" targetNamespace="urn:billing:invoices">
  <wsdl:types>
    <xs:schema targetNamespace="urn:billing:invoices" elementFormDefault="qualified">
      <xs:include schemaLocation="common.xsd"/>
      <xs:element name="GetInvoice">
        <xs:complexType>
          <xs:sequence>
            <xs:element name="invoiceId" type="xs:string"/>
            <xs:element name="lang" type="inv:Lang" minOccurs="0"/>
            <xs:element name="line" type="inv:Line" minOccurs="0" maxOccurs="unbounded"/>
          </xs:sequence>
          <xs:attribute name="version" type="xs:int"/>
        </xs:complexType>
      </xs:element>
      <xs:simpleType name="Lang"><xs:restriction base="xs:string"><xs:enumeration value="en"/><xs:enumeration value="ar"/></xs:restriction></xs:simpleType>
    </xs:schema>
  </wsdl:types>
  <wsdl:message name="GetInvoiceRequest"><wsdl:part name="parameters" element="inv:GetInvoice"/></wsdl:message>
  <wsdl:portType name="Invoices"><wsdl:operation name="GetInvoice"><wsdl:input message="inv:GetInvoiceRequest"/></wsdl:operation></wsdl:portType>
  <wsdl:binding name="Invoices11" type="inv:Invoices">
    <soap:binding style="document" transport="http://schemas.xmlsoap.org/soap/http"/>
    <wsdl:operation name="GetInvoice"><soap:operation soapAction="urn:billing:GetInvoice"/><wsdl:input><soap:body use="literal"/></wsdl:input></wsdl:operation>
  </wsdl:binding>
  <wsdl:binding name="Invoices12" type="inv:Invoices">
    <soap12:binding style="document" transport="http://schemas.xmlsoap.org/soap/http"/>
    <wsdl:operation name="GetInvoice"><soap12:operation soapAction="urn:billing:GetInvoice"/><wsdl:input><soap12:body use="literal"/></wsdl:input></wsdl:operation>
  </wsdl:binding>
  <wsdl:service name="BillingService">
    <wsdl:port name="Invoices11Port" binding="inv:Invoices11"><soap:address location="https://billing.example.com/ws/invoices"/></wsdl:port>
    <wsdl:port name="Invoices12Port" binding="inv:Invoices12"><soap12:address location="https://billing.example.com/ws12/invoices"/></wsdl:port>
  </wsdl:service>
</wsdl:definitions>"#;

    pub const COMMON: &str = r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" targetNamespace="urn:billing:invoices" elementFormDefault="qualified">
  <xs:complexType name="Line"><xs:sequence><xs:element name="sku" type="xs:string"/><xs:element name="qty" type="xs:int"/></xs:sequence></xs:complexType>
</xs:schema>"#;

    pub fn load(path: &str) -> Option<String> {
        (path == "wsdl/common.xsd").then(|| COMMON.to_string())
    }

    #[test]
    fn a_document_literal_wsdl_gives_its_services_ports_and_operations() {
        let wsdl = parse(INVOICES, "wsdl/invoices.wsdl", &load).unwrap();
        assert_eq!(wsdl.services.len(), 1);
        let ports = &wsdl.services[0].ports;
        assert_eq!(ports.len(), 2);
        assert_eq!(ports[0].version, Version::V11);
        assert_eq!(ports[1].version, Version::V12);
        assert_eq!(ports[0].address, "https://billing.example.com/ws/invoices");
        let op = &ports[0].operations[0];
        assert_eq!(op.name, "GetInvoice");
        assert_eq!(op.action, "urn:billing:GetInvoice");
        assert!(!op.rpc);

        let envelope = wsdl.envelope(op, Version::V11, HashMap::new());
        assert_eq!(
            envelope,
            r#"<soapenv:Envelope xmlns:soapenv="http://schemas.xmlsoap.org/soap/envelope/" xmlns:inv="urn:billing:invoices">
  <soapenv:Header/>
  <soapenv:Body>
    <!-- required -->
    <inv:GetInvoice version="0">
      <!-- xs:string, required -->
      <inv:invoiceId>invoiceId</inv:invoiceId>
      <!-- Lang, optional -->
      <inv:lang>en</inv:lang>
      <!-- Line, optional, repeats -->
      <inv:line>
        <!-- xs:string, required -->
        <inv:sku>sku</inv:sku>
        <!-- xs:int, required -->
        <inv:qty>0</inv:qty>
      </inv:line>
    </inv:GetInvoice>
  </soapenv:Body>
</soapenv:Envelope>"#
        );
    }

    #[test]
    fn rebuilding_keeps_what_was_typed_where_it_still_is() {
        let wsdl = parse(INVOICES, "wsdl/invoices.wsdl", &load).unwrap();
        let op = &wsdl.services[0].ports[1].operations[0];
        let old = r#"<soap:Envelope xmlns:soap="http://www.w3.org/2003/05/soap-envelope" xmlns:inv="urn:billing:invoices"><soap:Body><inv:GetInvoice><inv:invoiceId>{{invoiceId}}</inv:invoiceId><inv:gone>x</inv:gone><inv:line><inv:qty>3</inv:qty></inv:line></inv:GetInvoice></soap:Body></soap:Envelope>"#;
        let envelope = wsdl.envelope(op, Version::V12, super::super::xsd::typed_values(old));
        assert!(envelope
            .starts_with("<soap:Envelope xmlns:soap=\"http://www.w3.org/2003/05/soap-envelope\""));
        assert!(envelope.contains("<inv:invoiceId>{{invoiceId}}</inv:invoiceId>"));
        assert!(envelope.contains("<inv:qty>3</inv:qty>"));
        assert!(envelope.contains("<inv:sku>sku</inv:sku>"));
        assert!(!envelope.contains("gone"));
    }

    #[test]
    fn an_rpc_wsdl_wraps_its_parts_in_the_operation() {
        let rpc = r#"<definitions xmlns="http://schemas.xmlsoap.org/wsdl/" xmlns:soap="http://schemas.xmlsoap.org/wsdl/soap/" xmlns:xsd="http://www.w3.org/2001/XMLSchema" xmlns:tns="urn:calc" targetNamespace="urn:calc">
  <message name="AddIn"><part name="a" type="xsd:int"/><part name="b" type="xsd:int"/></message>
  <portType name="Calc"><operation name="Add"><input message="tns:AddIn"/></operation></portType>
  <binding name="CalcBinding" type="tns:Calc"><soap:binding style="rpc" transport="http://schemas.xmlsoap.org/soap/http"/>
    <operation name="Add"><soap:operation soapAction=""/><input><soap:body use="literal" namespace="urn:calc:ops"/></input></operation></binding>
  <service name="CalcService"><port name="CalcPort" binding="tns:CalcBinding"><soap:address location="http://localhost:8080/calc"/></port></service>
</definitions>"#;
        let wsdl = parse(rpc, "calc.wsdl", &|_| None).unwrap();
        let op = &wsdl.services[0].ports[0].operations[0];
        assert!(op.rpc);
        let envelope = wsdl.envelope(op, Version::V11, HashMap::new());
        assert!(envelope.contains("xmlns:ns1=\"urn:calc:ops\""));
        assert!(envelope.contains("    <ns1:Add>\n      <!-- xs:int -->\n      <a>0</a>\n"));
        assert!(parse("<description/>", "x", &|_| None)
            .unwrap_err()
            .contains("WSDL 2.0"));
    }

    #[test]
    fn imports_are_followed_relative_to_the_wsdl() {
        assert_eq!(relative("a/b/s.wsdl", "../xsd/t.xsd"), "a/xsd/t.xsd");
        assert_eq!(relative("s.wsdl", "./t.xsd"), "t.xsd");
    }
}
