// SPDX-License-Identifier: GPL-3.0-or-later

//! Forge-neutral PR evidence with explicit attribution and completeness.
//! Discussion comments use issues endpoints; findings use review endpoints.
//! Thread resolution is unknown unless an API actually supplied it.
use super::{http, status_error, Kind, Repo};
use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

const PER_PAGE: usize = 100;
const MAX_PAGES: usize = 10;
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Actor {
    pub id: Option<u64>,
    pub login: String,
    pub kind: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Item {
    pub id: u64,
    pub author: Actor,
    pub body: String,
    pub url: String,
    pub commit_sha: Option<String>,
    pub path: Option<String>,
    pub line: Option<u32>,
    pub original_line: Option<u32>,
    pub side: Option<String>,
    pub resolved: Option<bool>,
    pub state: Option<String>,
    pub conclusion: Option<String>,
    pub created_at: Option<String>,
    pub app_id: Option<u64>,
    pub app_slug: Option<String>,
    pub title: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Collection {
    pub items: Vec<Item>,
    pub complete: bool,
    pub error: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub forge: Kind,
    pub host: String,
    pub number: u64,
    pub url: String,
    pub head_sha: String,
    pub base_sha: String,
    pub discussion: Collection,
    pub findings: Collection,
    pub reviews: Collection,
    pub checks: Collection,
    pub revision_current: bool,
}
fn failure(repo: &Repo, detail: impl Into<String>) -> Error {
    Error::Forge {
        host: repo.host.clone(),
        detail: detail.into(),
    }
}
fn supported(repo: &Repo) -> Result<()> {
    if repo.kind == Kind::GitHub {
        Ok(())
    } else {
        Err(failure(
            repo,
            "Pull request review snapshots are currently available only on GitHub.",
        ))
    }
}
fn endpoint(repo: &Repo, path: &str) -> String {
    format!(
        "{}/repos/{}/{}/{}",
        repo.kind.api_base(&repo.host),
        repo.owner,
        repo.name,
        path
    )
}
fn get(repo: &Repo, token: &str, path: &str) -> Result<Value> {
    let response = http::get_json(&endpoint(repo, path), token, &repo.host)?;
    if !(200..300).contains(&response.status) {
        return Err(status_error(
            &repo.host,
            response.status,
            "Read pull request review data",
            response.retry_after.as_deref(),
        ));
    }
    serde_json::from_str(&response.body)
        .map_err(|e| failure(repo, format!("The host sent unreadable review data: {e}")))
}
fn optional(v: &Value, key: &str) -> Option<String> {
    v[key].as_str().map(str::to_string)
}
fn actor(v: &Value) -> Actor {
    Actor {
        id: v["id"].as_u64(),
        login: v["login"].as_str().unwrap_or("").into(),
        kind: v["type"].as_str().unwrap_or("Unknown").into(),
    }
}
fn item(v: &Value, check: bool) -> Option<Item> {
    Some(Item {
        id: v["id"].as_u64()?,
        author: actor(&v["user"]),
        body: v["body"]
            .as_str()
            .or_else(|| v["output"]["summary"].as_str())
            .unwrap_or("")
            .into(),
        url: v["html_url"]
            .as_str()
            .or_else(|| v["details_url"].as_str())
            .unwrap_or("")
            .into(),
        commit_sha: optional(v, if check { "head_sha" } else { "commit_id" }),
        path: optional(v, "path"),
        line: v["line"].as_u64().and_then(|n| n.try_into().ok()),
        original_line: v["original_line"].as_u64().and_then(|n| n.try_into().ok()),
        side: optional(v, "side"),
        resolved: None,
        state: optional(v, if check { "status" } else { "state" }),
        conclusion: optional(v, "conclusion"),
        created_at: optional(v, "created_at").or_else(|| optional(v, "started_at")),
        app_id: v["app"]["id"].as_u64(),
        app_slug: optional(&v["app"], "slug"),
        title: optional(v, "name"),
    })
}
fn read_collection(repo: &Repo, token: &str, path: &str, check: bool) -> Collection {
    collect_pages(path, check, |path| get(repo, token, path))
}
fn collect_pages(
    path: &str,
    check: bool,
    mut fetch: impl FnMut(&str) -> Result<Value>,
) -> Collection {
    let mut result = Collection {
        items: Vec::new(),
        complete: false,
        error: None,
    };
    for page in 1..=MAX_PAGES {
        let value = match fetch(&format!("{path}?per_page={PER_PAGE}&page={page}")) {
            Ok(v) => v,
            Err(e) => {
                result.error = Some(e.to_string());
                return result;
            }
        };
        let values = if check {
            value["check_runs"].as_array()
        } else {
            value.as_array()
        };
        let Some(values) = values else {
            result.error = Some("The host sent an unreadable review list.".into());
            return result;
        };
        for value in values {
            match item(value, check) {
                Some(i) => result.items.push(i),
                None => {
                    result.error = Some("An item lacked its platform identity.".into());
                    return result;
                }
            }
        }
        if values.len() < PER_PAGE {
            result.complete = true;
            return result;
        }
    }
    result.error = Some("The review list exceeded its bounded page limit.".into());
    result
}
pub fn pull_request(repo: &Repo, token: &str, number: u64) -> Result<Snapshot> {
    supported(repo)?;
    if number == 0 {
        return Err(failure(repo, "Choose a pull request."));
    }
    let metadata = get(repo, token, &format!("pulls/{number}"))?;
    let head = metadata["head"]["sha"]
        .as_str()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| failure(repo, "The pull request has no head revision."))?
        .to_string();
    let base = metadata["base"]["sha"]
        .as_str()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| failure(repo, "The pull request has no base revision."))?
        .to_string();
    let discussion = read_collection(repo, token, &format!("issues/{number}/comments"), false);
    let findings = read_collection(repo, token, &format!("pulls/{number}/comments"), false);
    let reviews = read_collection(repo, token, &format!("pulls/{number}/reviews"), false);
    let checks = read_collection(repo, token, &format!("commits/{head}/check-runs"), true);
    let final_metadata = get(repo, token, &format!("pulls/{number}"))?;
    Ok(Snapshot {
        forge: repo.kind,
        host: repo.host.clone(),
        number,
        url: metadata["html_url"].as_str().unwrap_or("").into(),
        head_sha: head.clone(),
        base_sha: base.clone(),
        discussion,
        findings,
        reviews,
        checks,
        revision_current: final_metadata["head"]["sha"] == head
            && final_metadata["base"]["sha"] == base,
    })
}
/// No retry: a failed transport may already have delivered the comment.
pub fn post_comment(repo: &Repo, token: &str, number: u64, body: &str) -> Result<Value> {
    post_comment_with_stop(
        repo,
        token,
        number,
        body,
        &std::sync::atomic::AtomicBool::new(false),
    )
}
pub fn post_comment_with_stop(
    repo: &Repo,
    token: &str,
    number: u64,
    body: &str,
    stop: &std::sync::atomic::AtomicBool,
) -> Result<Value> {
    post_comment_using(
        repo,
        number,
        body,
        stop,
        |path| get(repo, token, path),
        |path, body| http::post_json(&endpoint(repo, path), token, &repo.host, body),
    )
}
fn post_comment_using(
    repo: &Repo,
    number: u64,
    body: &str,
    stop: &std::sync::atomic::AtomicBool,
    mut fetch: impl FnMut(&str) -> Result<Value>,
    mut post: impl FnMut(&str, &str) -> Result<http::Response>,
) -> Result<Value> {
    supported(repo)?;
    if number == 0 || body.trim().is_empty() || body.len() > 60_000 {
        return Err(failure(
            repo,
            "Choose a pull request and a comment shorter than 60 KiB.",
        ));
    }
    let cancelled = || failure(repo, "The comment request was cancelled before posting.");
    if stop.load(std::sync::atomic::Ordering::Acquire) {
        return Err(cancelled());
    }
    let metadata = fetch(&format!("pulls/{number}"))?;
    let head = metadata["head"]["sha"]
        .as_str()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| failure(repo, "The pull request has no head revision."))?;
    if stop.load(std::sync::atomic::Ordering::Acquire) {
        return Err(cancelled());
    }
    let response = match post(
        &format!("issues/{number}/comments"),
        &json!({"body":body}).to_string(),
    ) {
        Ok(r) => r,
        Err(Error::ForgeOffline { .. }) => {
            return Ok(
                json!({"status":"uncertain","headSha":head,"message":"The host did not confirm delivery. Refresh the pull request before deciding to send again."}),
            )
        }
        Err(e) => return Err(e),
    };
    if !(200..300).contains(&response.status) {
        return Err(status_error(
            &repo.host,
            response.status,
            "Post pull request discussion comment",
            response.retry_after.as_deref(),
        ));
    }
    match serde_json::from_str::<Value>(&response.body) {
        Ok(v) if v["id"].as_u64().is_some() => {
            Ok(json!({"status":"posted","headSha":head,"commentId":v["id"],"url":v["html_url"]}))
        }
        _ => Ok(
            json!({"status":"uncertain","headSha":head,"message":"Delivery may have succeeded, but its receipt was unreadable. Refresh before sending again."}),
        ),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn platform_identity_and_revision_survive_mapping() {
        let raw = json!({"id":7,"user":{"id":9,"login":"bot[bot]","type":"Bot"},"body":"untrusted content","commit_id":"abc","path":"a.rs","line":8,"original_line":6,"side":"RIGHT","html_url":"https://github.com/a","resolved":false});
        let mapped = item(&raw, false).unwrap();
        assert_eq!(mapped.author.id, Some(9));
        assert_eq!(mapped.author.kind, "Bot");
        assert_eq!(mapped.commit_sha.as_deref(), Some("abc"));
        assert_eq!(mapped.line, Some(8));
        assert_eq!(mapped.resolved, None);
        assert!(item(&json!({"body":"anonymous"}), false).is_none());
    }
    #[test]
    fn check_identity_is_separate_from_a_user_name() {
        let raw = json!({"id":1,"name":"CodeRabbit","head_sha":"abc","status":"completed","conclusion":"success","app":{"id":2,"slug":"coderabbitai"},"output":{"summary":"reviewed"}});
        let mapped = item(&raw, true).unwrap();
        assert_eq!(mapped.app_id, Some(2));
        assert_eq!(mapped.author.kind, "Unknown");
        assert_eq!(mapped.commit_sha.as_deref(), Some("abc"));
        assert_eq!(mapped.body, "reviewed");
    }
    #[test]
    fn pagination_uses_the_correct_endpoint_and_preserves_partial_evidence() {
        let mut paths = Vec::new();
        let result = collect_pages("issues/7/comments", false, |path| {
            paths.push(path.to_string());
            Ok(if paths.len() == 1 {
                json!((0..100).map(|id| json!({"id":id})).collect::<Vec<_>>())
            } else {
                json!([{"id":100}])
            })
        });
        assert!(result.complete);
        assert_eq!(result.items.len(), 101);
        assert_eq!(
            paths,
            [
                "issues/7/comments?per_page=100&page=1",
                "issues/7/comments?per_page=100&page=2"
            ]
        );
        let mut calls = 0;
        let partial = collect_pages("pulls/7/comments", false, |_| {
            calls += 1;
            if calls == 1 {
                Ok(json!((0..100)
                    .map(|id| json!({"id":id}))
                    .collect::<Vec<_>>()))
            } else {
                Err(Error::ForgeOffline {
                    host: "github.com".into(),
                    detail: "disconnected".into(),
                })
            }
        });
        assert!(!partial.complete);
        assert_eq!(partial.items.len(), 100);
        assert!(partial.error.is_some());
    }
    #[test]
    fn pagination_never_treats_a_limit_or_malformed_item_as_complete() {
        let mut calls = 0;
        let result = collect_pages("pulls/7/reviews", false, |_| {
            calls += 1;
            Ok(json!((0..100)
                .map(|id| json!({"id":id}))
                .collect::<Vec<_>>()))
        });
        assert_eq!(calls, 10);
        assert!(!result.complete);
        assert!(result.error.unwrap().contains("limit"));
        let result = collect_pages("commits/head/check-runs", true, |_| {
            Ok(
                json!({"check_runs":[{"id":7,"head_sha":"head","app":{"id":9,"slug":"coderabbitai"}}]}),
            )
        });
        assert!(result.complete);
        assert_eq!(result.items[0].commit_sha.as_deref(), Some("head"));
        let malformed = collect_pages("pulls/7/comments", false, |_| {
            Ok(json!([{"body":"no identity"}]))
        });
        assert!(!malformed.complete);
        assert!(malformed.error.is_some());
    }
    #[test]
    fn comment_delivery_is_single_attempt_and_uncertain_when_no_receipt_arrives() {
        let repo = Repo {
            kind: Kind::GitHub,
            host: "github.com".into(),
            owner: "o".into(),
            name: "r".into(),
        };
        let mut calls = 0;
        let receipt = post_comment_using(
            &repo,
            7,
            "@coderabbitai review",
            &std::sync::atomic::AtomicBool::new(false),
            |path| {
                assert_eq!(path, "pulls/7");
                Ok(json!({"head":{"sha":"head"}}))
            },
            |path, body| {
                calls += 1;
                assert_eq!(path, "issues/7/comments");
                assert_eq!(
                    serde_json::from_str::<Value>(body).unwrap()["body"],
                    "@coderabbitai review"
                );
                Err(Error::ForgeOffline {
                    host: "github.com".into(),
                    detail: "timeout after possible delivery".into(),
                })
            },
        )
        .unwrap();
        assert_eq!(calls, 1);
        assert_eq!(receipt["status"], "uncertain");
        assert_eq!(receipt["headSha"], "head");
        let unreadable = post_comment_using(
            &repo,
            7,
            "review",
            &std::sync::atomic::AtomicBool::new(false),
            |_| Ok(json!({"head":{"sha":"head"}})),
            |_, _| {
                Ok(http::Response {
                    status: 201,
                    body: "broken receipt".into(),
                    retry_after: None,
                })
            },
        )
        .unwrap();
        assert_eq!(unreadable["status"], "uncertain");
    }
    #[test]
    fn cancellation_during_metadata_read_never_posts() {
        let repo = Repo {
            kind: Kind::GitHub,
            host: "github.com".into(),
            owner: "o".into(),
            name: "r".into(),
        };
        let stop = std::sync::atomic::AtomicBool::new(false);
        let result = post_comment_using(
            &repo,
            7,
            "review",
            &stop,
            |_| {
                stop.store(true, std::sync::atomic::Ordering::Release);
                Ok(json!({"head":{"sha":"head"}}))
            },
            |_, _| panic!("a cancelled request must not POST"),
        );
        assert!(result.unwrap_err().to_string().contains("cancelled"));
    }
    #[test]
    fn unsupported_forges_fail_before_network_or_credentials() {
        for kind in [Kind::GitLab, Kind::Bitbucket] {
            let repo = Repo {
                kind,
                host: "example.org".into(),
                owner: "owner".into(),
                name: "name".into(),
            };
            assert!(pull_request(&repo, "", 1).is_err());
            assert!(post_comment(&repo, "", 1, "review").is_err());
        }
    }
}
