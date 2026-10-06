// SPDX-License-Identifier: GPL-3.0-or-later

//! Whether CodeRabbit can be used here, and if not, what to do about it.
//!
//! Six states, kept apart because each has a different remedy:
//!
//! | State | Means | Remedy |
//! | --- | --- | --- |
//! | missing tool | no `coderabbit` or `cr` found | install it, or choose where it is |
//! | unsupported version | older than the minimum | update it |
//! | signed out | `auth status --agent` says `authenticated: false` | sign in |
//! | ready | found, recent enough, signed in | — |
//! | denied capability | the user has not let the extension run tools | turn it on in Settings |
//! | failed diagnostics | a check could not be run or read | see Diagnostics |
//!
//! Looking for the binary and asking its version is local. Asking whether it
//! is signed in reads the CLI's own credential store, which the CLI may refresh
//! over the network, so it runs before a review and when asked — never on
//! activation. `doctor`, which contacts CodeRabbit's servers, runs only when
//! the person asks for it.

use serde_json::{json, Value};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum State {
    MissingTool(String),
    UnsupportedVersion(String),
    SignedOut,
    Ready,
    DeniedCapability,
    FailedDiagnostics(String),
}

impl State {
    /// One sentence for a person.
    pub fn sentence(&self) -> String {
        match self {
            State::MissingTool(why) => why.clone(),
            State::UnsupportedVersion(why) => why.clone(),
            State::SignedOut => "Not signed in to CodeRabbit. Use “Sign in to CodeRabbit”.".into(),
            State::Ready => "Ready.".into(),
            State::DeniedCapability => {
                "CodeRabbit may not run its CLI here. Turn on “Run the external tools it declares” in Settings › Extensions."
                    .into()
            }
            State::FailedDiagnostics(why) => why.clone(),
        }
    }
}

/// The tool's state from the host's `tools.detect` answer.
pub fn from_detection(detected: &Value) -> Option<State> {
    let reason = detected
        .get("reason")
        .and_then(Value::as_str)
        .map(str::to_string);
    if detected.get("found").and_then(Value::as_bool) != Some(true) {
        return Some(State::MissingTool(reason.unwrap_or_else(|| {
            "The CodeRabbit CLI was not found. Install it from https://docs.coderabbit.ai/cli, or choose where it is.".into()
        })));
    }
    if detected.get("compatible").and_then(Value::as_bool) == Some(false) {
        return Some(State::UnsupportedVersion(reason.unwrap_or_else(|| {
            "This CodeRabbit CLI is too old. Run `coderabbit update`.".into()
        })));
    }
    if detected.get("version").is_none() && reason.is_some() {
        return Some(State::FailedDiagnostics(reason.unwrap_or_default()));
    }
    None
}

/// What `auth status --agent` said, read from its output lines.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Auth {
    pub authenticated: Option<bool>,
    pub region: Option<String>,
    pub account: Option<String>,
    pub problem: Option<String>,
}

/// Read `auth status --agent` output. The documented field is
/// `authenticated`; `region` and an account name are shown when present.
/// `credentials_unavailable` and `callback_listener_unavailable` mean the
/// credential store could not be reached — not that the person is signed out.
pub fn parse_auth(lines: &[String]) -> Auth {
    let mut auth = Auth {
        authenticated: None,
        region: None,
        account: None,
        problem: None,
    };
    for line in lines {
        let Ok(value) = serde_json::from_str::<Value>(line.trim()) else {
            continue;
        };
        let candidates = [
            value.clone(),
            value.get("data").cloned().unwrap_or(Value::Null),
            value.get("status").cloned().unwrap_or(Value::Null),
        ];
        for candidate in candidates.iter().filter(|v| v.is_object()) {
            if let Some(flag) = candidate.get("authenticated").and_then(Value::as_bool) {
                auth.authenticated = Some(flag);
            }
            if let Some(region) = candidate.get("region").and_then(Value::as_str) {
                auth.region = Some(region.to_string());
            }
            for key in ["user", "username", "login", "email", "account"] {
                if let Some(name) = candidate.get(key).and_then(Value::as_str) {
                    auth.account = Some(name.to_string());
                }
            }
            for key in ["error", "code", "errorCode"] {
                if let Some(code) = candidate.get(key).and_then(Value::as_str) {
                    if code.contains("unavailable") {
                        auth.problem = Some(match code {
                            "credentials_unavailable" => {
                                "The CodeRabbit CLI could not read its stored credentials.".into()
                            }
                            "callback_listener_unavailable" => {
                                "The CodeRabbit CLI could not start its sign-in listener.".into()
                            }
                            other => format!("The CodeRabbit CLI reported {other}."),
                        });
                    }
                }
            }
        }
    }
    auth
}

/// The state after a sign-in check.
pub fn from_auth(auth: &Auth, exit_code: Option<i64>) -> State {
    if let Some(problem) = &auth.problem {
        return State::FailedDiagnostics(problem.clone());
    }
    match auth.authenticated {
        Some(true) => State::Ready,
        Some(false) => State::SignedOut,
        None => State::FailedDiagnostics(format!(
            "CodeRabbit's sign-in check did not answer in a form Spagitty reads (exit {}).",
            exit_code
                .map(|c| c.to_string())
                .unwrap_or_else(|| "none".into())
        )),
    }
}

/// The rows of the setup panel.
pub fn panel(state: &State, version: Option<&str>, auth: Option<&Auth>, region: &str) -> Value {
    let mut rows = vec![json!({"label": "Status", "value": state.sentence()})];
    rows.push(json!({"label": "CLI", "value": version.map(|v| format!("CodeRabbit CLI {v}")).unwrap_or_else(|| "Not found".into())}));
    match auth {
        Some(auth) => {
            rows.push(json!({"label": "Signed in", "value": match auth.authenticated {
                Some(true) => auth.account.clone().map(|a| format!("Yes, as {a}")).unwrap_or_else(|| "Yes".into()),
                Some(false) => "No".into(),
                None => "Unknown".into(),
            }}));
            rows.push(json!({"label": "Region", "value": auth.region.clone().unwrap_or_else(|| region.to_uppercase())}));
        }
        None => {
            rows.push(json!({"label": "Signed in", "value": "Not checked — use “Check CodeRabbit sign-in”"}));
            rows.push(json!({"label": "Region", "value": region.to_uppercase()}));
        }
    }
    json!({
        "title": "CodeRabbit",
        "rows": rows,
        "text": "Reviews send the code you choose, and its context, to CodeRabbit under your own account. \
                 The CLI keeps its own credentials; Spagitty never sees them. [CodeRabbit CLI](https://docs.coderabbit.ai/cli)"
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(text: &[&str]) -> Vec<String> {
        text.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn detection_tells_missing_old_and_found_apart() {
        assert!(matches!(
            from_detection(&json!({"found": false})),
            Some(State::MissingTool(_))
        ));
        assert!(matches!(
            from_detection(&json!({"found": true, "version": "0.6.0", "compatible": false, "reason": "too old"})),
            Some(State::UnsupportedVersion(r)) if r == "too old"
        ));
        assert_eq!(
            from_detection(&json!({"found": true, "version": "0.8.1", "compatible": true})),
            None
        );
        assert!(matches!(
            from_detection(&json!({"found": true, "reason": "did not answer"})),
            Some(State::FailedDiagnostics(_))
        ));
    }

    #[test]
    fn sign_in_is_read_from_the_documented_field() {
        let signed_in = parse_auth(&lines(&[
            r#"{"authenticated":true,"region":"eu","user":"ada"}"#,
        ]));
        assert_eq!(from_auth(&signed_in, Some(0)), State::Ready);
        assert_eq!(signed_in.region.as_deref(), Some("eu"));
        let out = parse_auth(&lines(&[
            "some banner",
            r#"{"type":"auth_status","data":{"authenticated":false}}"#,
        ]));
        assert_eq!(from_auth(&out, Some(0)), State::SignedOut);
    }

    #[test]
    fn an_unreachable_credential_store_is_not_being_signed_out() {
        let auth = parse_auth(&lines(&[
            r#"{"authenticated":false,"error":"credentials_unavailable"}"#,
        ]));
        assert!(matches!(
            from_auth(&auth, Some(1)),
            State::FailedDiagnostics(_)
        ));
    }

    #[test]
    fn an_answer_spagitty_cannot_read_is_a_diagnostic_failure() {
        let auth = parse_auth(&lines(&["Logged in as ada (plain text)"]));
        assert!(
            matches!(from_auth(&auth, Some(0)), State::FailedDiagnostics(m) if m.contains("exit 0"))
        );
    }

    #[test]
    fn the_panel_never_shows_a_credential_and_says_what_is_sent_where() {
        let auth = parse_auth(&lines(&[
            r#"{"authenticated":true,"user":"ada","token":"secret-token"}"#,
        ]));
        let panel = panel(&State::Ready, Some("0.8.1"), Some(&auth), "us");
        let text = panel.to_string();
        assert!(!text.contains("secret-token"));
        assert!(text.contains("Yes, as ada"));
        assert!(text.contains("under your own account"));
        let unchecked = super::panel(&State::Ready, Some("0.8.1"), None, "eu");
        assert!(unchecked.to_string().contains("Not checked"));
    }

    #[test]
    fn every_state_has_a_sentence() {
        for state in [
            State::MissingTool("m".into()),
            State::UnsupportedVersion("u".into()),
            State::SignedOut,
            State::Ready,
            State::DeniedCapability,
            State::FailedDiagnostics("f".into()),
        ] {
            assert!(!state.sentence().is_empty());
        }
    }
}
