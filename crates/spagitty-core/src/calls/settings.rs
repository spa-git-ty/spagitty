// SPDX-License-Identifier: GPL-3.0-or-later

//! Settings › Calls (FEAT-165), and how a repository overrides them.
//!
//! The settings themselves are Spagitty's, stored with its other preferences.
//! A repository may override any of them in its calls folder's
//! `settings.json`, with only the keys it changes; [`effective`] lays that
//! over the person's and says which keys it took.

use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::calls::send::{Options, Proxy};
use crate::calls::store::DEFAULT_FOLDER;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ScanOnOpen {
    #[default]
    Ask,
    Always,
    Never,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ShareAs {
    File,
    #[default]
    Zip,
    Commit,
}

/// A certificate to show a host that asks for one: two PEM files on this
/// computer, named by path. The files stay where they are.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientCertFor {
    pub host: String,
    pub cert: String,
    pub key: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CallsSettings {
    // Where calls live.
    pub folder: String,
    pub open_found: bool,
    /// `postman`, `insomnia`, `bruno`, `openapi`, `http`, `wsdl`, `har`.
    pub look_for: Vec<String>,
    // Scanning code.
    pub scan_on_open: ScanOnOpen,
    /// `springMvc`, `springWebflux`, `jaxRs`, `jaxWs`, `springWs`.
    pub frameworks: Vec<String>,
    /// Globs, separated by commas.
    pub skip_paths: String,
    // Kept values.
    pub click_to_keep: bool,
    /// A new kept value updates on every call; off is once.
    pub keep_every: bool,
    pub suggest_names: bool,
    // Sending.
    pub timeout_seconds: u32,
    pub follow_redirects: bool,
    pub check_tls: bool,
    pub proxy: Proxy,
    pub client_certs: Vec<ClientCertFor>,
    pub beautify: bool,
    /// `Name: value` lines. `None` is `User-Agent: Spagitty/<version>`.
    pub every_header: Option<String>,
    // SOAP.
    pub soap_version: String,
    pub envelope_from_wsdl: bool,
    /// `none` or `username`. `signed` is FEAT-167.
    pub ws_security: String,
    pub fault_as_error: bool,
    // Safety and sharing.
    pub history: u32,
    pub share_as: ShareAs,
}

impl Default for CallsSettings {
    fn default() -> Self {
        CallsSettings {
            folder: DEFAULT_FOLDER.to_string(),
            open_found: true,
            look_for: ["postman", "insomnia", "bruno", "openapi", "http", "wsdl"]
                .map(String::from)
                .to_vec(),
            scan_on_open: ScanOnOpen::Ask,
            frameworks: ["springMvc", "springWebflux", "jaxRs", "jaxWs", "springWs"]
                .map(String::from)
                .to_vec(),
            skip_paths: "**/test/**, **/generated/**".to_string(),
            click_to_keep: true,
            keep_every: true,
            suggest_names: true,
            timeout_seconds: 30,
            follow_redirects: true,
            check_tls: true,
            proxy: Proxy::System,
            client_certs: Vec::new(),
            beautify: true,
            every_header: None,
            soap_version: "1.1".to_string(),
            envelope_from_wsdl: true,
            ws_security: "username".to_string(),
            fault_as_error: true,
            history: 500,
            share_as: ShareAs::Zip,
        }
    }
}

impl CallsSettings {
    /// The headers every call is sent with, unless it sets its own.
    pub fn every_headers(&self, version: &str) -> Vec<(String, String)> {
        let text = self
            .every_header
            .clone()
            .unwrap_or_else(|| format!("User-Agent: Spagitty/{version}"));
        text.lines()
            .filter_map(|line| line.split_once(':'))
            .map(|(name, value)| (name.trim().to_string(), value.trim().to_string()))
            .filter(|(name, _)| !name.is_empty())
            .collect()
    }

    /// What [`crate::calls::send`] needs. `check_tls` is off when either
    /// these settings or the environment turn it off.
    pub fn options(&self, environment_checks_tls: bool) -> Options {
        Options {
            timeout: Duration::from_secs(self.timeout_seconds.clamp(1, 3600) as u64),
            follow_redirects: self.follow_redirects,
            check_tls: self.check_tls && environment_checks_tls,
            proxy: self.proxy.clone(),
            client_cert: None,
            ..Options::default()
        }
    }

    /// The certificate for `host`, when one is set.
    pub fn cert_for(&self, host: &str) -> Option<&ClientCertFor> {
        self.client_certs
            .iter()
            .find(|cert| cert.host.eq_ignore_ascii_case(host))
    }

    /// The skip globs, one by one.
    pub fn skip_globs(&self) -> Vec<String> {
        self.skip_paths
            .split([',', '\n'])
            .map(|glob| glob.trim().to_string())
            .filter(|glob| !glob.is_empty())
            .collect()
    }
}

/// The settings with a repository's `overrides` laid over `mine`, and the
/// keys it overrode. A key that is not a setting, or a value of the wrong
/// kind, is ignored rather than costing every other override.
pub fn effective(
    mine: &CallsSettings,
    overrides: Option<&serde_json::Value>,
) -> (CallsSettings, Vec<String>) {
    let Some(serde_json::Value::Object(overrides)) = overrides else {
        return (mine.clone(), Vec::new());
    };
    let mut current = mine.clone();
    let mut taken = Vec::new();
    for (key, value) in overrides {
        let Ok(serde_json::Value::Object(mut whole)) = serde_json::to_value(&current) else {
            break;
        };
        if !whole.contains_key(key) {
            continue;
        }
        whole.insert(key.clone(), value.clone());
        if let Ok(next) = serde_json::from_value::<CallsSettings>(serde_json::Value::Object(whole))
        {
            current = next;
            taken.push(key.clone());
        }
    }
    taken.sort();
    (current, taken)
}

/// The host of a URL, lower case, without a port.
pub fn host_of(url: &str) -> String {
    let rest = url.split_once("://").map(|(_, rest)| rest).unwrap_or(url);
    let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
    let authority = authority.rsplit('@').next().unwrap_or(authority);
    let host = if let Some(bracketed) = authority.strip_prefix('[') {
        bracketed.split(']').next().unwrap_or("")
    } else {
        authority.split(':').next().unwrap_or("")
    };
    host.to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn the_defaults_are_the_proposal_s() {
        let settings = CallsSettings::default();
        assert_eq!(settings.folder, ".spagitty/calls");
        assert!(!settings.look_for.contains(&"har".to_string()));
        assert_eq!(settings.timeout_seconds, 30);
        assert_eq!(settings.history, 500);
        assert_eq!(settings.share_as, ShareAs::Zip);
        assert_eq!(
            settings.every_headers("4.0.0"),
            vec![("User-Agent".to_string(), "Spagitty/4.0.0".to_string())]
        );
        assert_eq!(settings.skip_globs(), vec!["**/test/**", "**/generated/**"]);
    }

    #[test]
    fn a_repository_overrides_only_what_it_names() {
        let overrides = json!({ "timeoutSeconds": 5, "followRedirects": false, "nonsense": 1, "beautify": "loud" });
        let (settings, taken) = effective(&CallsSettings::default(), Some(&overrides));
        assert_eq!(settings.timeout_seconds, 5);
        assert!(!settings.follow_redirects);
        assert!(settings.beautify);
        assert_eq!(taken, vec!["followRedirects", "timeoutSeconds"]);
    }

    #[test]
    fn an_empty_header_setting_sends_none() {
        let settings = CallsSettings {
            every_header: Some(String::new()),
            ..CallsSettings::default()
        };
        assert!(settings.every_headers("4").is_empty());
    }

    #[test]
    fn tls_is_checked_unless_either_says_not() {
        let settings = CallsSettings::default();
        assert!(settings.options(true).check_tls);
        assert!(!settings.options(false).check_tls);
    }

    #[test]
    fn hosts_are_read_from_urls() {
        assert_eq!(
            host_of("https://User:pw@API.example.com:8443/x?y"),
            "api.example.com"
        );
        assert_eq!(host_of("http://[::1]:8080/"), "::1");
        assert_eq!(host_of("localhost:8080/x"), "localhost");
    }
}
