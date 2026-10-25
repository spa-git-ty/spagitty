// SPDX-License-Identifier: GPL-3.0-or-later

//! The third place Spagitty makes a network request: a call the person sent
//! (FEAT-160), or a WSDL or OpenAPI address they typed into Import.
//!
//! [`crate::forge::http`] and [`crate::models::http`] are the other two, and
//! the reasons each is one file hold here: "what does this send, and where"
//! has an answer somebody can read. `src/lib/requests/requests.test.ts` names
//! the three files and nothing else may reach the client.
//!
//! # What it does, and what it does not
//!
//! - **Only what it is handed.** The request arrives resolved, from
//!   [`crate::calls::build`]; this file adds no header but the ones the
//!   client always sends (`Accept-Encoding`), and reaches no address on its
//!   own: no telemetry, no update check.
//! - **Unlike the other two**, plain `http` is allowed to any host and
//!   redirects are followed when the setting says so. A service under
//!   development is reached on `http://localhost` and a staging server often
//!   on plain `http`; the author accepted this on 2026-10-25.
//! - **Cancellable.** The body is read in chunks and the flag is looked at
//!   between them. A connection that stalls before answering cannot be
//!   interrupted by a flag, so the caller stops waiting at once and lets this
//!   finish on its own thread.
//! - **Capped.** At most [`LIMIT`] bytes of a body are read; the rest is left
//!   unread and the response says it was cut.

use std::io::Read;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

/// The most of a body that is read: 50 MB.
pub const LIMIT: u64 = 50 * 1024 * 1024;
/// Waiting for a connection, separately from the setting: an address that
/// does not answer is reported quickly.
const CONNECT: Duration = Duration::from_secs(15);
const CHUNK: usize = 64 * 1024;

/// A request, resolved: no `{{name}}` left in it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Outgoing {
    pub method: String,
    pub url: String,
    pub headers: Vec<(String, String)>,
    /// `None` sends no body at all.
    pub body: Option<Vec<u8>>,
}

/// Where requests go through.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind", content = "url")]
pub enum Proxy {
    /// `HTTPS_PROXY`, `HTTP_PROXY`, `ALL_PROXY` and `NO_PROXY`, as the
    /// system sets them.
    #[default]
    System,
    None,
    Custom(String),
}

/// A certificate and its key, for a server that asks who is calling.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientCert {
    pub cert_pem: String,
    pub key_pem: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Options {
    /// Waiting for the response to start. The body itself is not timed, so
    /// an event stream can run until it is cancelled.
    pub timeout: Duration,
    pub follow_redirects: bool,
    pub check_tls: bool,
    pub proxy: Proxy,
    pub client_cert: Option<ClientCert>,
    pub limit: u64,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            timeout: Duration::from_secs(30),
            follow_redirects: true,
            check_tls: true,
            proxy: Proxy::System,
            client_cert: None,
            limit: LIMIT,
        }
    }
}

/// What came back.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Incoming {
    pub status: u16,
    pub reason: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
    /// More arrived than [`Options::limit`], and was not read.
    pub truncated: bool,
    /// From the request leaving to the last byte read.
    pub millis: u64,
    /// Where the response came from, after any redirect.
    pub url: String,
}

impl Incoming {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }
}

/// Why nothing came back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Failure {
    Cancelled,
    TimedOut,
    /// Anything else, in a sentence.
    Failed(String),
}

impl std::fmt::Display for Failure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Failure::Cancelled => f.write_str("cancelled"),
            Failure::TimedOut => f.write_str("no answer before the time-out"),
            Failure::Failed(why) => f.write_str(why),
        }
    }
}

/// Send `out` and read the answer. `on_chunk` is handed the response's
/// headers and each piece of the body as it arrives, for a screen showing an
/// event stream.
pub fn send(
    out: &Outgoing,
    options: &Options,
    cancel: &AtomicBool,
    mut on_chunk: impl FnMut(&[(String, String)], &[u8]),
) -> Result<Incoming, Failure> {
    let agent = agent(options)?;
    let mut builder = ureq::http::Request::builder()
        .method(out.method.as_str())
        .uri(out.url.as_str());
    for (name, value) in &out.headers {
        builder = builder.header(name.as_str(), value.as_str());
    }
    let started = Instant::now();
    let answer = match &out.body {
        Some(bytes) => builder
            .body(bytes.clone())
            .map_err(|e| Failure::Failed(format!("the request could not be made: {e}")))
            .and_then(|request| agent.run(request).map_err(failure)),
        None => builder
            .body(())
            .map_err(|e| Failure::Failed(format!("the request could not be made: {e}")))
            .and_then(|request| agent.run(request).map_err(failure)),
    };
    if cancel.load(Ordering::Relaxed) {
        return Err(Failure::Cancelled);
    }
    let mut response = answer?;

    use ureq::ResponseExt;
    let url = response.get_uri().to_string();
    let status = response.status();
    let headers: Vec<(String, String)> = response
        .headers()
        .iter()
        .map(|(name, value)| {
            (
                name.as_str().to_string(),
                String::from_utf8_lossy(value.as_bytes()).into_owned(),
            )
        })
        .collect();

    let mut reader = response.body_mut().with_config().limit(u64::MAX).reader();
    let mut body = Vec::new();
    let mut buffer = vec![0u8; CHUNK];
    let mut truncated = false;
    loop {
        if cancel.load(Ordering::Relaxed) {
            return Err(Failure::Cancelled);
        }
        let read = match reader.read(&mut buffer) {
            Ok(0) => break,
            Ok(read) => read,
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(error) if error.kind() == std::io::ErrorKind::TimedOut => {
                return Err(Failure::TimedOut)
            }
            Err(error) => {
                return Err(Failure::Failed(format!(
                    "the response was cut off before it ended ({error})"
                )))
            }
        };
        let room = options.limit.saturating_sub(body.len() as u64) as usize;
        let taken = read.min(room);
        on_chunk(&headers, &buffer[..taken]);
        body.extend_from_slice(&buffer[..taken]);
        if taken < read {
            truncated = true;
            break;
        }
    }

    Ok(Incoming {
        status: status.as_u16(),
        reason: status.canonical_reason().unwrap_or("").to_string(),
        headers,
        body,
        truncated,
        millis: started.elapsed().as_millis() as u64,
        url,
    })
}

fn failure(error: ureq::Error) -> Failure {
    match error {
        ureq::Error::Timeout(_) => Failure::TimedOut,
        ureq::Error::HostNotFound => Failure::Failed("the host was not found".into()),
        ureq::Error::ConnectionFailed => Failure::Failed("the connection was refused".into()),
        ureq::Error::Io(error) if error.kind() == std::io::ErrorKind::TimedOut => Failure::TimedOut,
        ureq::Error::Io(error) if error.kind() == std::io::ErrorKind::ConnectionRefused => {
            Failure::Failed("the connection was refused".into())
        }
        other => Failure::Failed(other.to_string()),
    }
}

fn agent(options: &Options) -> Result<ureq::Agent, Failure> {
    use ureq::tls::{RootCerts, TlsConfig, TlsProvider};

    let mut tls = TlsConfig::builder()
        .provider(TlsProvider::NativeTls)
        .root_certs(RootCerts::PlatformVerifier)
        .disable_verification(!options.check_tls);
    if let Some(cert) = &options.client_cert {
        tls = tls.client_cert(Some(client_cert(cert)?));
    }

    let proxy = match &options.proxy {
        Proxy::System => ureq::Proxy::try_from_env(),
        Proxy::None => None,
        Proxy::Custom(url) => Some(
            ureq::Proxy::new(url)
                .map_err(|e| Failure::Failed(format!("the proxy {url} could not be used: {e}")))?,
        ),
    };

    Ok(ureq::Agent::config_builder()
        .http_status_as_error(false)
        .allow_non_standard_methods(true)
        .max_redirects(if options.follow_redirects { 10 } else { 0 })
        .max_redirects_will_error(false)
        .timeout_global(None)
        .timeout_connect(Some(CONNECT.min(options.timeout)))
        .timeout_send_request(Some(options.timeout))
        .timeout_recv_response(Some(options.timeout))
        .timeout_recv_body(None)
        // The caller sends `User-Agent` from Settings › Calls, or none.
        .user_agent(ureq::config::AutoHeaderValue::None)
        .proxy(proxy)
        .tls_config(tls.build())
        .build()
        .new_agent())
}

fn client_cert(cert: &ClientCert) -> Result<ureq::tls::ClientCert, Failure> {
    use ureq::tls::{parse_pem, PemItem, PrivateKey};
    let certs: Vec<_> = parse_pem(cert.cert_pem.as_bytes())
        .filter_map(|item| match item {
            Ok(PemItem::Certificate(found)) => Some(found),
            _ => None,
        })
        .collect();
    if certs.is_empty() {
        return Err(Failure::Failed(
            "the client certificate has no certificate in it".into(),
        ));
    }
    let key = PrivateKey::from_pem(cert.key_pem.as_bytes()).map_err(|e| {
        Failure::Failed(format!(
            "the client certificate's key could not be read: {e}"
        ))
    })?;
    Ok(ureq::tls::ClientCert::new_with_certs(&certs, key))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::io::{BufRead, BufReader, Write};
    use std::net::{TcpListener, TcpStream};
    use std::sync::mpsc::{channel, Receiver};
    use std::thread;

    /// What a test server was sent.
    #[derive(Debug)]
    pub struct Seen {
        pub request_line: String,
        pub headers: Vec<(String, String)>,
        pub body: Vec<u8>,
    }

    /// A server on a free local port that answers each connection with the
    /// next of `answers`, and reports what it was sent.
    pub fn serve(answers: Vec<Vec<u8>>) -> (String, Receiver<Seen>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = format!("http://{}", listener.local_addr().unwrap());
        let (tx, rx) = channel();
        thread::spawn(move || {
            for answer in answers {
                let Ok((stream, _)) = listener.accept() else {
                    return;
                };
                let seen = read_request(&stream);
                let _ = tx.send(seen);
                let mut stream = stream;
                let _ = stream.write_all(&answer);
                let _ = stream.flush();
            }
        });
        (address, rx)
    }

    fn read_request(stream: &TcpStream) -> Seen {
        let mut reader = BufReader::new(stream.try_clone().unwrap());
        let mut request_line = String::new();
        reader.read_line(&mut request_line).unwrap();
        let mut headers = Vec::new();
        loop {
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            let line = line.trim_end();
            if line.is_empty() {
                break;
            }
            if let Some((name, value)) = line.split_once(':') {
                headers.push((name.trim().to_ascii_lowercase(), value.trim().to_string()));
            }
        }
        let length = headers
            .iter()
            .find(|(name, _)| name == "content-length")
            .and_then(|(_, value)| value.parse::<usize>().ok())
            .unwrap_or(0);
        let mut body = vec![0u8; length];
        reader.read_exact(&mut body).unwrap();
        Seen {
            request_line: request_line.trim_end().to_string(),
            headers,
            body,
        }
    }

    pub fn ok(content_type: &str, body: &str) -> Vec<u8> {
        format!(
            "HTTP/1.1 200 OK\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        )
        .into_bytes()
    }

    fn outgoing(method: &str, url: String) -> Outgoing {
        Outgoing {
            method: method.into(),
            url,
            headers: vec![("Accept".into(), "application/json".into())],
            body: None,
        }
    }

    fn quick() -> Options {
        Options {
            timeout: Duration::from_secs(5),
            proxy: Proxy::None,
            ..Options::default()
        }
    }

    #[test]
    fn a_get_comes_back_with_its_status_headers_and_body() {
        let (base, seen) = serve(vec![ok("application/json", "{\"a\":1}")]);
        let got = send(
            &outgoing("GET", format!("{base}/ping?x=1")),
            &quick(),
            &AtomicBool::new(false),
            |_, _| {},
        )
        .unwrap();
        assert_eq!(got.status, 200);
        assert_eq!(got.reason, "OK");
        assert_eq!(got.header("content-type"), Some("application/json"));
        assert_eq!(got.body, b"{\"a\":1}");
        assert!(!got.truncated);
        let seen = seen.recv().unwrap();
        assert_eq!(seen.request_line, "GET /ping?x=1 HTTP/1.1");
        assert!(seen
            .headers
            .iter()
            .any(|(n, v)| n == "accept" && v == "application/json"));
        assert!(!seen.headers.iter().any(|(n, _)| n == "user-agent"));
    }

    #[test]
    fn every_method_carries_its_body() {
        for method in ["POST", "PUT", "PATCH", "DELETE", "GET", "PROPFIND"] {
            let (base, seen) = serve(vec![ok("text/plain", "done")]);
            let mut out = outgoing(method, format!("{base}/x"));
            out.body = Some(b"{\"n\":2}".to_vec());
            out.headers
                .push(("Content-Type".into(), "application/json".into()));
            let got = send(&out, &quick(), &AtomicBool::new(false), |_, _| {}).unwrap();
            assert_eq!(got.status, 200, "{method}");
            let seen = seen.recv().unwrap();
            assert!(seen.request_line.starts_with(method), "{method}");
            assert_eq!(seen.body, b"{\"n\":2}", "{method}");
        }
    }

    #[test]
    fn redirects_are_followed_or_shown_as_the_setting_says() {
        let redirect = |to: &str| {
            format!("HTTP/1.1 302 Found\r\nLocation: {to}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
                .into_bytes()
        };
        let (base, _seen) = serve(vec![redirect("/there"), ok("text/plain", "arrived")]);
        let got = send(
            &outgoing("GET", format!("{base}/here")),
            &quick(),
            &AtomicBool::new(false),
            |_, _| {},
        )
        .unwrap();
        assert_eq!(got.status, 200);
        assert_eq!(got.body, b"arrived");
        assert!(got.url.ends_with("/there"), "{}", got.url);

        let (base, _seen) = serve(vec![redirect("/there")]);
        let options = Options {
            follow_redirects: false,
            ..quick()
        };
        let got = send(
            &outgoing("GET", format!("{base}/here")),
            &options,
            &AtomicBool::new(false),
            |_, _| {},
        )
        .unwrap();
        assert_eq!(got.status, 302);
        assert_eq!(got.header("location"), Some("/there"));
    }

    #[test]
    fn a_server_that_does_not_answer_times_out() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        thread::spawn(move || {
            let held = listener.accept();
            thread::sleep(Duration::from_secs(3));
            drop(held);
        });
        let options = Options {
            timeout: Duration::from_millis(400),
            ..quick()
        };
        let got = send(
            &outgoing("GET", base),
            &options,
            &AtomicBool::new(false),
            |_, _| {},
        );
        assert_eq!(got.unwrap_err(), Failure::TimedOut);
    }

    #[test]
    fn a_stream_is_handed_out_as_it_arrives_and_stops_when_cancelled() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            read_request(&stream);
            let _ = stream.write_all(
                b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nTransfer-Encoding: chunked\r\n\r\n",
            );
            for n in 0..50 {
                let event = format!("data: {n}\n\n");
                let chunk = format!("{:x}\r\n{event}\r\n", event.len());
                if stream.write_all(chunk.as_bytes()).is_err() {
                    return;
                }
                let _ = stream.flush();
                thread::sleep(Duration::from_millis(40));
            }
        });
        let cancel = AtomicBool::new(false);
        let mut chunks = 0;
        let got = send(&outgoing("GET", base), &quick(), &cancel, |_, piece| {
            if !piece.is_empty() {
                chunks += 1;
            }
            if chunks == 3 {
                cancel.store(true, Ordering::Relaxed);
            }
        });
        assert_eq!(got.unwrap_err(), Failure::Cancelled);
        assert_eq!(chunks, 3);
    }

    #[test]
    fn a_body_past_the_limit_is_cut() {
        let big = "x".repeat(10_000);
        let (base, _seen) = serve(vec![ok("text/plain", &big)]);
        let options = Options {
            limit: 1000,
            ..quick()
        };
        let got = send(
            &outgoing("GET", base),
            &options,
            &AtomicBool::new(false),
            |_, _| {},
        )
        .unwrap();
        assert!(got.truncated);
        assert_eq!(got.body.len(), 1000);
    }

    #[test]
    fn nothing_listening_is_said_plainly() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        drop(listener);
        let got = send(
            &outgoing("GET", base),
            &quick(),
            &AtomicBool::new(false),
            |_, _| {},
        );
        assert!(matches!(got, Err(Failure::Failed(_))), "{got:?}");
    }

    /// The client tunnels through a proxy with `CONNECT`, then sends the
    /// request inside the tunnel.
    #[test]
    fn a_custom_proxy_is_used() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let proxy = format!("http://{}", listener.local_addr().unwrap());
        let (tx, seen) = channel();
        thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let connect = read_request(&stream);
            let _ = stream.write_all(
                b"HTTP/1.1 200 Connection established

",
            );
            let inner = read_request(&stream);
            let _ = tx.send((connect, inner));
            let _ = stream.write_all(&ok("text/plain", "via proxy"));
        });
        let options = Options {
            proxy: Proxy::Custom(proxy),
            ..quick()
        };
        let got = send(
            &outgoing("GET", "http://service.invalid/thing".into()),
            &options,
            &AtomicBool::new(false),
            |_, _| {},
        )
        .unwrap();
        assert_eq!(got.body, b"via proxy");
        let (connect, inner) = seen.recv().unwrap();
        assert!(
            connect.request_line.starts_with("CONNECT service.invalid"),
            "{}",
            connect.request_line
        );
        assert!(
            inner.request_line.starts_with("GET /thing"),
            "{}",
            inner.request_line
        );
    }
}
