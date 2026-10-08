// SPDX-License-Identifier: GPL-3.0-or-later

//! What an agent hands back from a step, and how it is read.
//!
//! A command-line agent answers in prose, and somewhere in the prose is one
//! fenced block in a form Spagitty can read — the farm's handoff and review
//! verdict work the same way. A model behind an API proposes through typed
//! tools instead ([`super::remote`]); both arrive here as the same values, so
//! nothing downstream knows which kind of agent made a proposal.
//!
//! The parse is defensive in one direction only: a block that cannot be read
//! is *no answer*, never a guess. A step with no answer fails with the agent's
//! own last words, and what it made before that is kept.

use serde::{Deserialize, Serialize};

pub use crate::review::decision::Severity;

/// The fence an agent is asked to wrap its answer in.
pub const FENCE: &str = "spagitty-agent";

/// Which side of a diff a line number counts on.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Side {
    /// The pull request's base: a removed line.
    Old,
    /// The pull request's head: an added or unchanged line.
    #[default]
    New,
}

/// One file in a reading plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanItem {
    pub path: String,
    #[serde(default)]
    pub why: String,
}

/// The agent's reading plan for a review.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Plan {
    pub files: Vec<PlanItem>,
    #[serde(default)]
    pub look_for: String,
}

/// One finding: a line or a range, a severity, the comment, and how sure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Finding {
    #[serde(default)]
    pub path: String,
    pub line: u32,
    #[serde(default)]
    pub start_line: Option<u32>,
    #[serde(default)]
    pub side: Side,
    pub severity: Severity,
    pub body: String,
    #[serde(default = "yes")]
    pub sure: bool,
}

/// The verdict an agent suggests for a whole pull request.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Verdict {
    #[default]
    Comment,
    Approve,
    RequestChanges,
}

/// What a pull request does, what was found, and a suggested verdict.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Summary {
    pub summary: String,
    #[serde(default)]
    pub verdict: Verdict,
    #[serde(default = "yes")]
    pub sure: bool,
}

/// The text an agent proposes for one conflict region, with why.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Resolution {
    pub text: String,
    #[serde(default)]
    pub why: String,
    #[serde(default = "yes")]
    pub sure: bool,
}

fn yes() -> bool {
    true
}

/// Everything an answer block may hold. A step reads the part it asked for.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Answer {
    #[serde(default)]
    pub plan: Option<Vec<PlanItem>>,
    #[serde(default)]
    pub look_for: Option<String>,
    #[serde(default)]
    pub findings: Option<Vec<Finding>>,
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default)]
    pub verdict: Option<Verdict>,
    #[serde(default)]
    pub resolution: Option<Resolution>,
    #[serde(default)]
    pub sure: Option<bool>,
    /// The answer to *Ask why*.
    #[serde(default)]
    pub why: Option<String>,
}

impl Answer {
    /// The last readable block in `transcript`.
    ///
    /// Last wins, as with a farm handoff: an agent that corrects itself is
    /// believed the second time. The transcript may be the agent's prose, or
    /// a machine stream of JSON events — Claude Code's `stream-json` — in
    /// which case the block is inside a string value and is looked for there.
    pub fn find(transcript: &str) -> Option<Answer> {
        let mut found = None;
        for text in texts(transcript) {
            if let Some(answer) = last_block(&text) {
                found = Some(answer);
            }
        }
        found
    }

    pub fn plan(&self) -> Option<Plan> {
        self.plan.as_ref().map(|files| Plan {
            files: files.clone(),
            look_for: self.look_for.clone().unwrap_or_default(),
        })
    }

    pub fn summary(&self) -> Option<Summary> {
        self.summary.as_ref().map(|summary| Summary {
            summary: summary.clone(),
            verdict: self.verdict.unwrap_or_default(),
            sure: self.sure.unwrap_or(true),
        })
    }
}

fn last_block(text: &str) -> Option<Answer> {
    let opening = format!("```{FENCE}");
    let mut found = None;
    let mut rest = text;
    while let Some(start) = rest.find(&opening) {
        let after = &rest[start + opening.len()..];
        let Some(end) = after.find("```") else { break };
        if let Ok(answer) = serde_json::from_str::<Answer>(after[..end].trim()) {
            found = Some(answer);
        }
        rest = &after[end + 3..];
    }
    found
}

/// The places an answer block might be: the transcript as it is, and every
/// string inside every line of it that is a JSON value.
fn texts(transcript: &str) -> Vec<String> {
    let mut out = vec![transcript.to_string()];
    for line in transcript.lines() {
        let line = line.trim();
        if !line.starts_with('{') {
            continue;
        }
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(line) {
            strings(&value, &mut out);
        }
    }
    out
}

fn strings(value: &serde_json::Value, out: &mut Vec<String>) {
    match value {
        serde_json::Value::String(text) if text.contains(FENCE) => out.push(text.clone()),
        serde_json::Value::Array(items) => items.iter().for_each(|item| strings(item, out)),
        serde_json::Value::Object(map) => map.values().for_each(|item| strings(item, out)),
        _ => {}
    }
}

/// The instructions appended to a step's prompt: the one block to end with.
pub fn contract(shape: &str) -> String {
    format!(
        "End your reply with exactly one block in this form, and nothing after it:\n\
         \n\
         ```{FENCE}\n\
         {shape}\n\
         ```\n"
    )
}

pub const PLAN_SHAPE: &str = r#"{
  "plan": [{ "path": "src/file.rs", "why": "what to check here" }],
  "lookFor": "one sentence: what you will look for"
}"#;

pub const FINDINGS_SHAPE: &str = r#"{
  "findings": [
    {
      "line": 42,
      "startLine": null,
      "side": "new",
      "severity": "high",
      "body": "the comment, as you would write it to the author",
      "sure": true
    }
  ]
}"#;

pub const SUMMARY_SHAPE: &str = r#"{
  "summary": "what the pull request does and what you found",
  "verdict": "comment",
  "sure": true
}"#;

pub const RESOLUTION_SHAPE: &str = r#"{
  "resolution": {
    "text": "the exact lines that replace the conflict region",
    "why": "one sentence on why",
    "sure": true
  }
}"#;

pub const WHY_SHAPE: &str = r#"{ "why": "what you read and why you chose this" }"#;

#[cfg(test)]
mod tests {
    use super::*;

    fn block(json: &str) -> String {
        format!("Some prose.\n\n```{FENCE}\n{json}\n```\n")
    }

    #[test]
    fn a_plan_is_read() {
        let answer = Answer::find(&block(
            r#"{"plan":[{"path":"a.rs","why":"the cache"}],"lookFor":"races"}"#,
        ))
        .unwrap();
        let plan = answer.plan().unwrap();
        assert_eq!(plan.files[0].path, "a.rs");
        assert_eq!(plan.look_for, "races");
    }

    #[test]
    fn findings_default_to_sure_and_the_new_side() {
        let answer = Answer::find(&block(
            r#"{"findings":[{"line":3,"severity":"low","body":"nit"}]}"#,
        ))
        .unwrap();
        let finding = &answer.findings.unwrap()[0];
        assert!(finding.sure);
        assert_eq!(finding.side, Side::New);
        assert_eq!(finding.severity, Severity::Low);
    }

    #[test]
    fn the_last_block_wins() {
        let text = format!(
            "{}{}",
            block(r#"{"summary":"first","verdict":"approve"}"#),
            block(r#"{"summary":"second","verdict":"requestChanges","sure":false}"#)
        );
        let summary = Answer::find(&text).unwrap().summary().unwrap();
        assert_eq!(summary.summary, "second");
        assert_eq!(summary.verdict, Verdict::RequestChanges);
        assert!(!summary.sure);
    }

    #[test]
    fn an_unreadable_block_is_no_answer() {
        assert_eq!(Answer::find(&block("{ not json")), None);
        assert_eq!(Answer::find("no block at all"), None);
    }

    #[test]
    fn a_block_inside_a_stream_of_json_events_is_found() {
        let result = serde_json::json!({
            "type": "result",
            "result": block(r#"{"resolution":{"text":"a\nb","why":"both","sure":true}}"#)
        });
        let transcript = format!("{{\"type\":\"system\"}}\n{result}\n");
        let answer = Answer::find(&transcript).unwrap();
        assert_eq!(answer.resolution.unwrap().text, "a\nb");
    }

    #[test]
    fn the_contract_shows_the_block_the_parser_reads() {
        for shape in [
            PLAN_SHAPE,
            FINDINGS_SHAPE,
            SUMMARY_SHAPE,
            RESOLUTION_SHAPE,
            WHY_SHAPE,
        ] {
            let text = contract(shape);
            assert!(Answer::find(&text).is_some(), "{shape}");
        }
    }
}
