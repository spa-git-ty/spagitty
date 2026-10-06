// SPDX-License-Identifier: GPL-3.0-or-later

//! CodeRabbit's `--agent` stream, read into Spagitty's review model.
//!
//! The stream is one JSON object per line, dispatched by `type` (CodeRabbit's
//! CLI reference, "`--agent` review output", read 2026-10-06): `finding`,
//! `review_context`, `status`, `heartbeat`, `complete`, `error`, and the
//! billing `action_required` result. Pure: lines in, events and an outcome
//! out, so every shape is a test rather than a discovery.
//!
//! **Nothing here reads silence as approval.** An empty stream, exit code
//! zero without a `complete` event, a second `complete`, a line that is not
//! JSON, a failure outcome, unreviewed files or a finding count that disagrees
//! with the completion all end as something other than a clean `completed`.

use std::collections::HashSet;

use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};

/// What a line meant, for the worker to pass on as it happens.
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    Progress(String),
    Heartbeat,
    Finding(Value),
    /// Something the stream said that the adapter did not recognise.
    Unknown(String),
}

/// How the tool process ended, as the host reported it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Exit {
    pub code: Option<i64>,
    pub cancelled: bool,
    pub timed_out: bool,
}

#[derive(Debug, Default)]
pub struct Adapter {
    ids: HashSet<String>,
    findings: usize,
    completes: Vec<Value>,
    errors: Vec<String>,
    action: Option<Value>,
    malformed: usize,
    files: Option<u64>,
}

/// CodeRabbit's severities onto Spagitty's, conservatively. Anything else is
/// `unknown`, which blocks a required gate until a person looks.
pub fn severity(provider: &str) -> &'static str {
    match provider.trim().to_ascii_lowercase().as_str() {
        "critical" => "critical",
        "major" => "high",
        "minor" => "medium",
        "trivial" => "low",
        "info" | "none" => "info",
        _ => "unknown",
    }
}

/// A clean relative path, or nothing.
fn clean_path(path: &str) -> Option<String> {
    let path = path.trim().trim_start_matches("./");
    let ok = !path.is_empty()
        && path.len() <= 1024
        && !path.starts_with('/')
        && !path.contains('\\')
        && !path.contains(':')
        && path
            .split('/')
            .all(|s| !s.is_empty() && s != "." && s != "..");
    ok.then(|| path.to_string())
}

fn positive(value: Option<&Value>) -> Option<u64> {
    value.and_then(Value::as_u64).filter(|n| *n > 0)
}

fn text(value: Option<&Value>) -> Option<String> {
    value
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

/// `suggestions` as plain text: strings, or objects with something readable.
fn suggestions(value: Option<&Value>) -> Option<String> {
    let list = match value? {
        Value::Array(items) => items.clone(),
        Value::String(s) => vec![Value::String(s.clone())],
        _ => return None,
    };
    let parts: Vec<String> = list
        .iter()
        .filter_map(|item| match item {
            Value::String(s) => Some(s.clone()),
            Value::Object(map) => ["suggestion", "command", "code", "text", "description"]
                .iter()
                .find_map(|key| map.get(*key).and_then(Value::as_str).map(str::to_string)),
            _ => None,
        })
        .filter(|s| !s.trim().is_empty())
        .collect();
    (!parts.is_empty()).then(|| parts.join("\n\n"))
}

fn first_line(text: &str) -> String {
    let line = text
        .lines()
        .map(|l| {
            l.trim()
                .trim_start_matches('#')
                .trim()
                .trim_matches('*')
                .trim()
        })
        .find(|l| !l.is_empty())
        .unwrap_or("CodeRabbit finding");
    let mut title: String = line.chars().take(120).collect();
    if line.chars().count() > 120 {
        title.push('…');
    }
    title
}

impl Adapter {
    pub fn new() -> Adapter {
        Adapter::default()
    }

    /// Read one line of the stream.
    pub fn feed(&mut self, line: &str) -> Vec<Event> {
        let line = line.trim();
        if line.is_empty() {
            return Vec::new();
        }
        let Ok(Value::Object(event)) = serde_json::from_str::<Value>(line) else {
            self.malformed += 1;
            return vec![Event::Unknown("a line that is not a JSON object".into())];
        };
        if event.get("status").and_then(Value::as_str) == Some("awaiting_confirmation")
            || event.get("type").and_then(Value::as_str) == Some("action_required")
        {
            self.action = Some(Value::Object(event));
            return vec![Event::Progress(
                "CodeRabbit is waiting for a decision".into(),
            )];
        }
        match event.get("type").and_then(Value::as_str).unwrap_or("") {
            "finding" => self.finding(&event).into_iter().collect(),
            "heartbeat" => vec![Event::Heartbeat],
            "status" => {
                let words = text(event.get("message"))
                    .or_else(|| text(event.get("status")).map(|s| s.replace('_', " ")))
                    .unwrap_or_else(|| "Reviewing".into());
                vec![Event::Progress(words)]
            }
            "review_context" => {
                self.files = positive(event.get("fileCount"))
                    .or_else(|| positive(event.get("filesCount")))
                    .or_else(|| {
                        event
                            .get("files")
                            .and_then(Value::as_array)
                            .map(|a| a.len() as u64)
                    });
                vec![Event::Progress(match self.files {
                    Some(n) => format!("Reviewing {n} file{}", if n == 1 { "" } else { "s" }),
                    None => "Reviewing".into(),
                })]
            }
            "complete" => {
                self.completes.push(Value::Object(event));
                vec![Event::Progress("Finishing".into())]
            }
            "error" => {
                let mut message = text(event.get("message"))
                    .or_else(|| text(event.get("error")))
                    .unwrap_or_else(|| "CodeRabbit reported an error.".into());
                if let Some(candidates) = event.get("candidates").and_then(Value::as_array) {
                    let options: Vec<String> = candidates
                        .iter()
                        .filter_map(|c| {
                            text(c.get("command"))
                                .or_else(|| text(c.get("description")))
                                .or_else(|| c.as_str().map(str::to_string))
                        })
                        .take(5)
                        .collect();
                    if !options.is_empty() {
                        message.push_str(&format!(
                            " Narrower scopes CodeRabbit suggested: {}.",
                            options.join("; ")
                        ));
                    }
                }
                self.errors.push(message);
                Vec::new()
            }
            other => vec![Event::Unknown(format!("an event of type {other:?}"))],
        }
    }

    fn finding(&mut self, event: &Map<String, Value>) -> Option<Event> {
        let provider = text(event.get("severity")).unwrap_or_else(|| "unspecified".into());
        let instructions = text(event.get("codegenInstructions"));
        let comment = text(event.get("comment"));
        let message = comment
            .clone()
            .or_else(|| instructions.clone())
            .unwrap_or_else(|| "CodeRabbit reported a finding without text.".into());
        let path = event
            .get("fileName")
            .and_then(Value::as_str)
            .and_then(clean_path);
        let start = positive(event.get("startLine"))
            .or_else(|| positive(event.get("lineStart")))
            .or_else(|| positive(event.get("line")));
        let end = positive(event.get("endLine"))
            .or_else(|| positive(event.get("lineEnd")))
            .filter(|end| start.is_some_and(|start| *end >= start));

        let mut hash = Sha256::new();
        hash.update(path.as_deref().unwrap_or(""));
        hash.update([0]);
        hash.update(&provider);
        hash.update([0]);
        hash.update(&message);
        let digest: String = hash
            .finalize()
            .iter()
            .take(8)
            .map(|b| format!("{b:02x}"))
            .collect();
        let mut id = format!("cr-{digest}");
        let mut n = 2;
        while self.ids.contains(&id) {
            id = format!("cr-{digest}-{n}");
            n += 1;
        }
        self.ids.insert(id.clone());
        self.findings += 1;

        let mut finding = json!({
            "id": id,
            "severity": severity(&provider),
            "providerSeverity": provider,
            "title": first_line(comment.as_deref().or(instructions.as_deref()).unwrap_or("")),
            "message": message,
        });
        if let Some(path) = path {
            finding["path"] = json!(path);
            if let Some(start) = start {
                finding["startLine"] = json!(start);
                if let Some(end) = end {
                    finding["endLine"] = json!(end);
                }
            }
        }
        // The agent-oriented instructions, when the human comment was the
        // message; never run, only shown.
        let mut extra = Vec::new();
        if comment.is_some() {
            if let Some(instructions) = &instructions {
                extra.push(instructions.clone());
            }
        }
        if let Some(text) = suggestions(event.get("suggestions")) {
            extra.push(text);
        }
        if !extra.is_empty() {
            finding["suggestion"] = json!(extra.join("\n\n"));
        }
        Some(Event::Finding(finding))
    }

    /// The outcome, once the tool has ended. `version` is the CLI's version.
    pub fn finish(&self, exit: Exit, version: &str) -> Value {
        let mut outcome = json!({ "providerVersion": version });
        let count = self.findings;
        let plural = |n: usize| if n == 1 { "finding" } else { "findings" };

        if exit.cancelled {
            outcome["status"] = json!("cancelled");
            outcome["completeness"] = json!("unknown");
            outcome["summary"] = json!("Cancelled.");
            return outcome;
        }

        if let Some(action) = &self.action {
            let files = action
                .get("billableFileCount")
                .or_else(|| action.get("billableFiles"))
                .and_then(Value::as_u64);
            let price = action
                .get("maxPrice")
                .or_else(|| action.get("maximumPrice"))
                .map(|p| match p {
                    Value::String(s) => s.clone(),
                    other => other.to_string(),
                });
            let mut message = String::from(
                "CodeRabbit needs your confirmation to use paid usage credits for this review",
            );
            if let Some(files) = files {
                message.push_str(&format!(" ({files} billable files"));
                if let Some(price) = &price {
                    message.push_str(&format!(", up to {price}"));
                }
                message.push(')');
            }
            message.push_str(". Spagitty never confirms this for you; nothing was charged and nothing was reviewed.");
            outcome["status"] = json!("actionRequired");
            outcome["completeness"] = json!("unknown");
            outcome["summary"] = json!(message);
            outcome["actionRequired"] = json!({
                "kind": "billing",
                "message": message,
                "detail": {
                    "billableFileCount": files,
                    "maxPrice": price,
                    "confirmationHeadCommitId": action.get("confirmationHeadCommitId"),
                }
            });
            return outcome;
        }

        let failed_exit =
            exit.timed_out || exit.code.is_some_and(|c| c != 0) || exit.code.is_none();
        if !self.errors.is_empty() || failed_exit {
            let mut summary =
                self.errors
                    .first()
                    .cloned()
                    .unwrap_or_else(|| match (exit.timed_out, exit.code) {
                        (true, _) => "The review ran past its time limit.".into(),
                        (_, Some(code)) => format!("CodeRabbit exited with code {code}."),
                        (_, None) => "CodeRabbit was stopped.".into(),
                    });
            if count > 0 {
                summary.push_str(&format!(
                    " {count} {} arrived before it stopped.",
                    plural(count)
                ));
            }
            outcome["status"] = json!("failed");
            outcome["completeness"] = json!(if count > 0 { "partial" } else { "unknown" });
            outcome["summary"] = json!(summary);
            return outcome;
        }

        let Some(complete) = self.completes.first() else {
            outcome["status"] = json!("incomplete");
            outcome["completeness"] = json!("unknown");
            outcome["summary"] = json!(format!(
                "CodeRabbit ended without saying the review finished ({count} {} received).",
                plural(count)
            ));
            return outcome;
        };

        let status = complete.get("status").and_then(Value::as_str).unwrap_or("");
        let provider_outcome = complete
            .get("outcome")
            .and_then(Value::as_str)
            .unwrap_or("");
        let unreviewed = complete
            .get("unreviewedFileCount")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        let reported = complete.get("findings").and_then(Value::as_u64);
        let message = text(complete.get("message"));

        let incomplete = |why: String| {
            let mut out = json!({"providerVersion": version, "status": "incomplete", "completeness": "partial"});
            out["summary"] = json!(why);
            out
        };
        if self.completes.len() > 1 {
            return incomplete(
                "CodeRabbit reported finishing more than once; the result cannot be trusted."
                    .into(),
            );
        }
        if self.malformed > 0 {
            return incomplete(format!(
                "{} line{} of CodeRabbit's output could not be read.",
                self.malformed,
                if self.malformed == 1 { "" } else { "s" }
            ));
        }
        if status == "review_skipped" {
            outcome["status"] = json!("skipped");
            outcome["completeness"] = json!("complete");
            outcome["summary"] = json!(message.unwrap_or_else(|| "No changes to review.".into()));
            return outcome;
        }
        if status != "review_completed" {
            return incomplete(format!(
                "CodeRabbit finished with an unrecognised status {status:?}."
            ));
        }
        if provider_outcome == "failed" {
            return incomplete(
                message.unwrap_or_else(|| "CodeRabbit reported the review as failed.".into()),
            );
        }
        if unreviewed > 0 {
            return incomplete(format!(
                "{unreviewed} file{} were not reviewed.",
                if unreviewed == 1 { "" } else { "s" }
            ));
        }
        if let Some(reported) = reported {
            if reported as usize != count {
                return incomplete(format!(
                    "CodeRabbit reported {reported} {} but sent {count}.",
                    plural(reported as usize)
                ));
            }
        }
        outcome["status"] = json!("completed");
        outcome["completeness"] = json!("complete");
        let mut summary = if count == 0 {
            "No findings.".to_string()
        } else {
            format!("{count} {}.", plural(count))
        };
        if provider_outcome == "completed_with_warnings" {
            summary.push_str(" CodeRabbit finished with warnings.");
        }
        if let Some(message) = message {
            summary = format!("{summary} {message}");
        }
        outcome["summary"] = json!(summary);
        outcome
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const OK: Exit = Exit {
        code: Some(0),
        cancelled: false,
        timed_out: false,
    };
    const FAIL: Exit = Exit {
        code: Some(1),
        cancelled: false,
        timed_out: false,
    };

    fn run(lines: &[&str], exit: Exit) -> (Vec<Value>, Value) {
        let mut adapter = Adapter::new();
        let mut findings = Vec::new();
        for line in lines {
            for event in adapter.feed(line) {
                if let Event::Finding(f) = event {
                    findings.push(f);
                }
            }
        }
        (findings, adapter.finish(exit, "0.8.1"))
    }

    const CONTEXT: &str = r#"{"type":"review_context","workingDirectory":"/repo","currentBranch":"work","baseBranch":"main"}"#;
    const FINDING: &str = r#"{"type":"finding","severity":"major","fileName":"src/lib.rs","comment":"Unchecked index\n\nThis panics on an empty list.","codegenInstructions":"Use first()","suggestions":["items.first()"]}"#;
    const DONE_1: &str = r#"{"type":"complete","status":"review_completed","findings":1}"#;

    #[test]
    fn severities_map_conservatively_and_keep_the_providers_word() {
        for (from, to) in [
            ("critical", "critical"),
            ("major", "high"),
            ("minor", "medium"),
            ("trivial", "low"),
            ("info", "info"),
            ("none", "info"),
            ("blocker", "unknown"),
            ("", "unknown"),
        ] {
            assert_eq!(severity(from), to, "{from}");
        }
        let (findings, _) = run(&[FINDING], OK);
        assert_eq!(findings[0]["providerSeverity"], "major");
    }

    #[test]
    fn a_clean_review_with_findings_is_completed_and_complete() {
        let (findings, outcome) = run(&[CONTEXT, FINDING, DONE_1], OK);
        assert_eq!(outcome["status"], "completed");
        assert_eq!(outcome["completeness"], "complete");
        assert_eq!(outcome["providerVersion"], "0.8.1");
        let f = &findings[0];
        assert_eq!(f["severity"], "high");
        assert_eq!(f["path"], "src/lib.rs");
        assert_eq!(f["title"], "Unchecked index");
        assert!(f["message"].as_str().unwrap().contains("panics"));
        assert!(f["suggestion"].as_str().unwrap().contains("items.first()"));
        assert!(f["suggestion"].as_str().unwrap().contains("Use first()"));
        assert!(
            f.get("startLine").is_none(),
            "no line was sent, so none is invented"
        );
    }

    #[test]
    fn a_clean_review_without_findings_says_so_and_is_not_a_skip() {
        let (_, outcome) = run(
            &[
                CONTEXT,
                r#"{"type":"complete","status":"review_completed","findings":0}"#,
            ],
            OK,
        );
        assert_eq!(outcome["status"], "completed");
        assert_eq!(outcome["summary"], "No findings.");
    }

    #[test]
    fn an_empty_scope_is_a_skip_not_an_approval() {
        let (_, outcome) = run(
            &[
                CONTEXT,
                r#"{"type":"status","status":"review_skipped"}"#,
                r#"{"type":"complete","status":"review_skipped","findings":0,"message":"No changes detected"}"#,
            ],
            OK,
        );
        assert_eq!(outcome["status"], "skipped");
        assert_eq!(outcome["summary"], "No changes detected");
    }

    #[test]
    fn exit_zero_with_nothing_said_is_not_a_review() {
        let (_, outcome) = run(&[], OK);
        assert_eq!(outcome["status"], "incomplete");
        assert_eq!(outcome["completeness"], "unknown");
        let (_, outcome) = run(&[CONTEXT, FINDING], OK);
        assert_eq!(outcome["status"], "incomplete");
    }

    #[test]
    fn a_failure_after_findings_keeps_them_and_says_partial() {
        let (findings, outcome) = run(
            &[
                CONTEXT,
                FINDING,
                r#"{"type":"error","message":"Connection lost"}"#,
            ],
            FAIL,
        );
        assert_eq!(findings.len(), 1);
        assert_eq!(outcome["status"], "failed");
        assert_eq!(outcome["completeness"], "partial");
        assert!(outcome["summary"]
            .as_str()
            .unwrap()
            .starts_with("Connection lost"));
    }

    #[test]
    fn a_non_zero_exit_fails_even_after_a_completion_label() {
        let (_, outcome) = run(&[CONTEXT, FINDING, DONE_1], FAIL);
        assert_eq!(outcome["status"], "failed");
        assert!(outcome["summary"].as_str().unwrap().contains("code 1"));
    }

    #[test]
    fn a_completed_label_with_a_failed_outcome_or_unreviewed_files_is_incomplete() {
        let (_, outcome) = run(
            &[
                r#"{"type":"complete","status":"review_completed","findings":0,"outcome":"failed","message":"Model unavailable"}"#,
            ],
            OK,
        );
        assert_eq!(
            (outcome["status"].as_str(), outcome["completeness"].as_str()),
            (Some("incomplete"), Some("partial"))
        );
        assert_eq!(outcome["summary"], "Model unavailable");
        let (_, outcome) = run(
            &[
                r#"{"type":"complete","status":"review_completed","findings":0,"unreviewedFileCount":3}"#,
            ],
            OK,
        );
        assert_eq!(outcome["status"], "incomplete");
        assert!(outcome["summary"].as_str().unwrap().contains("3 files"));
    }

    #[test]
    fn warnings_alone_are_not_failure() {
        let (_, outcome) = run(
            &[
                r#"{"type":"complete","status":"review_completed","findings":0,"outcome":"completed_with_warnings"}"#,
            ],
            OK,
        );
        assert_eq!(outcome["status"], "completed");
        assert!(outcome["summary"].as_str().unwrap().contains("warnings"));
    }

    #[test]
    fn conflicting_completions_malformed_lines_and_miscounts_are_incomplete() {
        let (_, outcome) = run(&[DONE_1, DONE_1], OK);
        assert_eq!(outcome["status"], "incomplete");
        let (_, outcome) = run(&[FINDING, "{not json", DONE_1], OK);
        assert_eq!(outcome["status"], "incomplete");
        assert!(outcome["summary"]
            .as_str()
            .unwrap()
            .contains("could not be read"));
        let (_, outcome) = run(
            &[r#"{"type":"complete","status":"review_completed","findings":4}"#],
            OK,
        );
        assert!(outcome["summary"].as_str().unwrap().contains("reported 4"));
    }

    #[test]
    fn a_billing_decision_is_an_action_never_a_success() {
        let line = r#"{"type":"action_required","status":"awaiting_confirmation","billableFileCount":12,"maxPrice":"$1.20","confirmationHeadCommitId":"abc","command":"coderabbit review --use-credits"}"#;
        let (_, outcome) = run(&[CONTEXT, line], OK);
        assert_eq!(outcome["status"], "actionRequired");
        assert_eq!(outcome["actionRequired"]["kind"], "billing");
        assert_eq!(outcome["actionRequired"]["detail"]["billableFileCount"], 12);
        let message = outcome["actionRequired"]["message"].as_str().unwrap();
        assert!(message.contains("12 billable files, up to $1.20"));
        assert!(message.contains("never confirms"));
        let (_, other) = run(
            &[r#"{"type":"complete","status":"awaiting_confirmation"}"#],
            OK,
        );
        assert_eq!(other["status"], "actionRequired");
    }

    #[test]
    fn a_too_large_scope_reports_the_narrower_choices() {
        let line = r#"{"type":"error","message":"Too many files","candidates":[{"command":"coderabbit review --committed","estimatedFiles":40},{"command":"coderabbit review --dir src"}],"candidatesNote":"Choose one"}"#;
        let (_, outcome) = run(&[line], FAIL);
        let summary = outcome["summary"].as_str().unwrap();
        assert!(
            summary.contains("--committed") && summary.contains("--dir src"),
            "{summary}"
        );
    }

    #[test]
    fn cancellation_and_timeouts_are_their_own_outcomes() {
        let (_, outcome) = run(
            &[FINDING],
            Exit {
                code: None,
                cancelled: true,
                timed_out: false,
            },
        );
        assert_eq!(outcome["status"], "cancelled");
        let (_, outcome) = run(
            &[FINDING],
            Exit {
                code: None,
                cancelled: false,
                timed_out: true,
            },
        );
        assert_eq!(outcome["status"], "failed");
        assert!(outcome["summary"].as_str().unwrap().contains("time limit"));
    }

    #[test]
    fn paths_are_kept_only_when_clean_and_lines_only_when_sent() {
        let mut adapter = Adapter::new();
        let bad = adapter.feed(
            r#"{"type":"finding","severity":"minor","fileName":"../../etc/passwd","comment":"x"}"#,
        );
        let Event::Finding(bad) = &bad[0] else {
            panic!()
        };
        assert!(bad.get("path").is_none());
        let good = adapter.feed(r#"{"type":"finding","severity":"minor","fileName":"./src/a.rs","comment":"y","startLine":12,"endLine":14}"#);
        let Event::Finding(good) = &good[0] else {
            panic!()
        };
        assert_eq!(good["path"], "src/a.rs");
        assert_eq!(
            (good["startLine"].as_u64(), good["endLine"].as_u64()),
            (Some(12), Some(14))
        );
        let backwards = adapter.feed(r#"{"type":"finding","severity":"minor","fileName":"src/a.rs","comment":"z","startLine":12,"endLine":3}"#);
        let Event::Finding(backwards) = &backwards[0] else {
            panic!()
        };
        assert!(backwards.get("endLine").is_none());
    }

    #[test]
    fn identical_findings_get_distinct_stable_ids() {
        let (first, _) = run(&[FINDING, FINDING], OK);
        let (again, _) = run(&[FINDING, FINDING], OK);
        assert_ne!(first[0]["id"], first[1]["id"]);
        assert_eq!(first[0]["id"], again[0]["id"]);
        assert_eq!(first[1]["id"], again[1]["id"]);
    }

    #[test]
    fn heartbeats_status_and_unknown_events_are_passed_on_not_dropped() {
        let mut adapter = Adapter::new();
        assert_eq!(
            adapter.feed(r#"{"type":"heartbeat"}"#),
            vec![Event::Heartbeat]
        );
        assert_eq!(
            adapter.feed(r#"{"type":"status","status":"reviewing_files"}"#),
            vec![Event::Progress("reviewing files".into())]
        );
        assert!(matches!(
            adapter.feed(r#"{"type":"something_new","x":1}"#)[0],
            Event::Unknown(_)
        ));
        assert!(adapter.feed("   ").is_empty());
    }

    #[test]
    fn a_finding_with_only_instructions_still_has_a_title_and_message() {
        let (findings, _) = run(
            &[
                r#"{"type":"finding","severity":"trivial","fileName":"a.md","codegenInstructions":"Fix the typo in the heading"}"#,
            ],
            OK,
        );
        assert_eq!(findings[0]["title"], "Fix the typo in the heading");
        assert_eq!(findings[0]["message"], "Fix the typo in the heading");
        assert!(findings[0].get("suggestion").is_none());
    }
}
