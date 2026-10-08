// SPDX-License-Identifier: GPL-3.0-or-later

//! The second place Spagitty makes a network request: to a model provider,
//! with the person's own key (2.0, agents).
//!
//! [`crate::forge::http`] is the first, and the reasons it is one place hold
//! here: "what does this send, and where" has an answer somebody can read in
//! an afternoon. `src/lib/requests/requests.test.ts` names the two files and
//! nothing else may reach the client.
//!
//! # What it will not do
//!
//! - **No plaintext off this machine.** `https` only — except to an endpoint
//!   on this machine, `localhost` or a loopback address, where a local model
//!   server such as Ollama answers in plain `http` and nothing leaves the
//!   computer to be read on the way.
//! - **No redirects.** A redirect would carry the key to wherever it pointed.
//! - **No key in an error.** What a provider answered is reported by status and
//!   by its own error sentence; headers are never quoted, and the key is cut
//!   out of anything that is.

use std::time::Duration;

use crate::{Error, Result};

/// A model can take minutes on a long step. Long enough for that, short enough
/// that a provider that stopped answering is reported rather than waited on.
const TIMEOUT: Duration = Duration::from_secs(300);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
/// How much of an answer is read. A model's reply is text; anything bigger
/// than this is not one.
const LIMIT: u64 = 8 * 1024 * 1024;

/// What a provider answered.
pub struct Answered {
    pub status: u16,
    pub body: String,
}

impl std::fmt::Debug for Answered {
    /// Never prints the body, which can echo what was sent.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Answered")
            .field("status", &self.status)
            .field("body_bytes", &self.body.len())
            .finish()
    }
}

/// Is `url` on this machine?
///
/// The host is read the way a client connects to it: an authority with
/// userinfo (`localhost@api.example.com`) is refused outright, since it is the
/// part after `@` that is reached, and only `localhost` or an address that
/// parses as loopback counts. A name that merely starts with `127.` does not.
pub fn is_local(url: &str) -> bool {
    let rest = url
        .strip_prefix("http://")
        .or_else(|| url.strip_prefix("https://"))
        .unwrap_or(url);
    let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
    if authority.contains('@') {
        return false;
    }
    let host = if let Some(bracketed) = authority.strip_prefix('[') {
        bracketed.split(']').next().unwrap_or("")
    } else {
        authority.split(':').next().unwrap_or("")
    };
    host.eq_ignore_ascii_case("localhost")
        || host
            .parse::<std::net::IpAddr>()
            .is_ok_and(|ip| ip.is_loopback())
}

fn allowed(url: &str, provider: &str) -> Result<()> {
    if url.starts_with("https://") || (url.starts_with("http://") && is_local(url)) {
        return Ok(());
    }
    Err(Error::Model {
        provider: provider.to_string(),
        detail: "refusing to send a key over an unencrypted connection; use https, or an \
                 endpoint on this machine"
            .into(),
    })
}

/// `GET url` with `headers`.
pub fn get(url: &str, headers: &[(&str, &str)], provider: &str) -> Result<Answered> {
    allowed(url, provider)?;
    let mut request = agent().get(url);
    for (name, value) in headers {
        request = request.header(*name, *value);
    }
    finish(request.call(), provider, headers)
}

/// `POST url` with `headers` and a JSON body.
pub fn post(url: &str, headers: &[(&str, &str)], body: &str, provider: &str) -> Result<Answered> {
    allowed(url, provider)?;
    let mut request = agent().post(url).header("Content-Type", "application/json");
    for (name, value) in headers {
        request = request.header(*name, *value);
    }
    finish(request.send(body), provider, headers)
}

fn finish(
    sent: std::result::Result<ureq::http::Response<ureq::Body>, ureq::Error>,
    provider: &str,
    headers: &[(&str, &str)],
) -> Result<Answered> {
    match sent {
        Ok(mut response) => {
            let status = response.status().as_u16();
            let body = response
                .body_mut()
                .with_config()
                .limit(LIMIT)
                .read_to_string()
                .unwrap_or_default();
            Ok(Answered { status, body })
        }
        Err(ureq::Error::StatusCode(status)) => Ok(Answered {
            status,
            body: String::new(),
        }),
        Err(error) => Err(Error::Model {
            provider: provider.to_string(),
            detail: redact(&error.to_string(), headers),
        }),
    }
}

/// Cut every header value out of `text`. A transport error should never
/// quote one, and this makes sure.
pub fn redact(text: &str, headers: &[(&str, &str)]) -> String {
    let mut out = text.to_string();
    for (_, value) in headers {
        let secret = value.trim_start_matches("Bearer ").trim();
        if secret.len() >= 8 {
            out = out.replace(secret, "••••");
        }
    }
    out
}

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(TIMEOUT))
        .timeout_connect(Some(CONNECT_TIMEOUT))
        .max_redirects(0)
        .http_status_as_error(false)
        .user_agent(concat!("spagitty/", env!("CARGO_PKG_VERSION")))
        // Chosen, as in `forge::http`: the platform's TLS and its own roots,
        // so a corporate CA the machine trusts is trusted here too.
        .tls_config(
            ureq::tls::TlsConfig::builder()
                .provider(ureq::tls::TlsProvider::NativeTls)
                .root_certs(ureq::tls::RootCerts::PlatformVerifier)
                .build(),
        )
        .build()
        .new_agent()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_this_machine_counts_as_local() {
        assert!(is_local("http://localhost:11434/v1"));
        assert!(is_local("http://127.0.0.1:1234/v1"));
        assert!(is_local("http://[::1]:8080"));
        assert!(!is_local("http://localhost.example.com/v1"));
        assert!(!is_local("http://127.0.0.1.example.com"));
        assert!(!is_local("https://api.anthropic.com"));
        assert!(is_local("http://127.8.9.10/v1"));
        assert!(is_local("http://LOCALHOST:8080"));
        // The client connects to what follows `@`, not to what precedes it.
        assert!(!is_local("http://localhost:80@api.example.com/v1"));
        assert!(!is_local("http://127.0.0.1@api.example.com/v1"));
        // A name that starts like a loopback address is a name.
        assert!(!is_local("http://127.foo.example.com/v1"));
        assert!(!is_local("http://127.0.0.256/v1"));
    }

    #[test]
    fn plain_http_is_refused_off_this_machine() {
        assert!(allowed("http://example.com/v1", "OpenAI-compatible").is_err());
        assert!(allowed("http://localhost:11434/v1", "Ollama").is_ok());
        assert!(allowed("https://api.openai.com/v1", "OpenAI").is_ok());
        assert!(allowed("ftp://example.com", "x").is_err());
    }

    #[test]
    fn a_key_is_cut_out_of_an_error() {
        let headers = [
            ("x-api-key", "sk-ant-secret-1234"),
            ("Authorization", "Bearer sk-openai-5678"),
        ];
        let text = "failed sending sk-ant-secret-1234 and sk-openai-5678";
        assert_eq!(redact(text, &headers), "failed sending •••• and ••••");
    }
}
