// SPDX-License-Identifier: GPL-3.0-or-later

//! GitLab merge requests and API integration (FEAT-070).

use serde_json::Value;

use crate::error::{Error, Result};
use crate::forge::github::timestamp;
use crate::forge::{http, status_error, CheckState, PullRequest, Repo, ReviewState};

pub fn whoami(host: &str, token: &str) -> Result<String> {
    let url = format!("{}/user", crate::forge::Kind::GitLab.api_base(host));
    let response = http::get_json(&url, token, host)?;

    if response.status < 200 || response.status >= 300 {
        return Err(status_error(
            host,
            response.status,
            &response.body,
            response.retry_after.as_deref(),
        ));
    }

    let json: Value = serde_json::from_str(&response.body).map_err(|e| json_err(host, e))?;
    json.get("username")
        .and_then(Value::as_str)
        .map(String::from)
        .ok_or_else(|| Error::Forge {
            host: host.to_string(),
            detail: "GitLab response had no username".into(),
        })
}

pub fn pull_requests(repo: &Repo, token: &str, me: &str) -> Result<Vec<PullRequest>> {
    let project_path = format!("{}%2F{}", repo.owner, repo.name);
    let url = format!(
        "{}/projects/{}/merge_requests?state=opened&per_page=50&order_by=updated_at&sort=desc",
        repo.kind.api_base(&repo.host),
        project_path
    );

    let response = http::get_json(&url, token, &repo.host)?;
    if response.status < 200 || response.status >= 300 {
        return Err(status_error(
            &repo.host,
            response.status,
            &response.body,
            response.retry_after.as_deref(),
        ));
    }

    let json: Value = serde_json::from_str(&response.body).map_err(|e| json_err(&repo.host, e))?;
    Ok(parse_merge_requests(&json, me))
}

/// Open merge requests anywhere on the host that `me` is a reviewer of
/// (FEAT-087).
///
/// GitLab has no "involves" search, so this is the review requests alone.
pub fn involved_merge_requests(host: &str, token: &str, me: &str) -> Result<Vec<PullRequest>> {
    if me.is_empty() {
        return Err(Error::Forge {
            host: host.to_string(),
            detail: "no account is connected for this host".into(),
        });
    }

    let url = format!(
        "{}/merge_requests?scope=all&state=opened&per_page=50&order_by=updated_at&sort=desc&reviewer_username={me}",
        crate::forge::Kind::GitLab.api_base(host),
    );

    let response = http::get_json(&url, token, host)?;
    if response.status < 200 || response.status >= 300 {
        return Err(status_error(
            host,
            response.status,
            &response.body,
            response.retry_after.as_deref(),
        ));
    }

    let json: Value = serde_json::from_str(&response.body).map_err(|e| json_err(host, e))?;
    Ok(parse_merge_requests(&json, me))
}

pub fn create_merge_request(
    repo: &Repo,
    token: &str,
    title: &str,
    body: &str,
    head: &str,
    base: &str,
    draft: bool,
) -> Result<PullRequest> {
    let project_path = format!("{}%2F{}", repo.owner, repo.name);
    let url = format!(
        "{}/projects/{}/merge_requests",
        repo.kind.api_base(&repo.host),
        project_path
    );

    let title_formatted = if draft && !title.starts_with("Draft:") && !title.starts_with("WIP:") {
        format!("Draft: {title}")
    } else {
        title.to_string()
    };

    let payload = serde_json::json!({
        "source_branch": head,
        "target_branch": base,
        "title": title_formatted,
        "description": body,
    });

    let response = http::post_json(&url, token, &repo.host, &payload.to_string())?;
    if response.status < 200 || response.status >= 300 {
        return Err(status_error(
            &repo.host,
            response.status,
            &response.body,
            response.retry_after.as_deref(),
        ));
    }

    let json: Value = serde_json::from_str(&response.body).map_err(|e| json_err(&repo.host, e))?;
    parse_single_mr(&json, "").ok_or_else(|| Error::Forge {
        host: repo.host.clone(),
        detail: "Could not parse created merge request".into(),
    })
}

pub fn parse_merge_requests(json: &Value, me: &str) -> Vec<PullRequest> {
    let Some(array) = json.as_array() else {
        return Vec::new();
    };
    array
        .iter()
        .filter_map(|node| parse_single_mr(node, me))
        .collect()
}

fn parse_single_mr(node: &Value, me: &str) -> Option<PullRequest> {
    let iid = node.get("iid")?.as_u64()?;
    let id = node.get("id")?.to_string();
    let title = node.get("title")?.as_str()?.to_string();
    let body = node
        .get("description")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let author_name = node
        .get("author")
        .and_then(|a| a.get("username"))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();

    let updated_str = node.get("updated_at").and_then(Value::as_str);
    let updated = timestamp(updated_str);

    let source_branch = node
        .get("source_branch")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let target_branch = node
        .get("target_branch")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let draft = node
        .get("work_in_progress")
        .and_then(Value::as_bool)
        .unwrap_or(false)
        || title.starts_with("Draft:")
        || title.starts_with("WIP:");

    let mergeable = node
        .get("has_conflicts")
        .and_then(Value::as_bool)
        .map(|conflicts| !conflicts);

    let needs_you = !me.is_empty() && author_name != me;

    Some(PullRequest {
        id,
        number: iid,
        title,
        body,
        author_name,
        updated,
        source_branch,
        target_branch,
        draft,
        review: ReviewState::NoReviewers,
        checks: Some(CheckState::Passing),
        needs_you,
        needs_you_because: if needs_you {
            Some("Merge request awaiting review".into())
        } else {
            None
        },
        changed_files: node
            .get("changes_count")
            .and_then(Value::as_u64)
            .unwrap_or(0),
        added: 0,
        removed: 0,
        mergeable,
        head_sha: node
            .get("sha")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        review_requested: !me.is_empty()
            && node
                .get("reviewers")
                .and_then(Value::as_array)
                .is_some_and(|reviewers| {
                    reviewers
                        .iter()
                        .any(|reviewer| reviewer["username"].as_str() == Some(me))
                }),
        // `references.full` is `group/project!42`; the part before the `!`
        // names the project. Only a search across projects needs it.
        repository: node["references"]["full"]
            .as_str()
            .and_then(|full| full.rsplit_once('!'))
            .map(|(project, _)| project.to_string()),
        ..PullRequest::default()
    })
}

fn json_err(host: &str, e: impl std::fmt::Display) -> Error {
    Error::Forge {
        host: host.to_string(),
        detail: format!("JSON parsing error: {e}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_gitlab_merge_request_json() {
        let raw = serde_json::json!([
            {
                "id": 12345,
                "iid": 42,
                "title": "Draft: Feature implementation",
                "description": "MR description here",
                "state": "opened",
                "author": { "username": "developer1" },
                "updated_at": "2024-01-01T12:00:00Z",
                "source_branch": "feature/x",
                "target_branch": "main",
                "work_in_progress": true,
                "has_conflicts": false,
                "changes_count": 5
            }
        ]);

        let list = parse_merge_requests(&raw, "reviewer1");
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].number, 42);
        assert_eq!(list[0].author_name, "developer1");
        assert!(list[0].draft);
        assert!(list[0].needs_you);
        assert_eq!(list[0].source_branch, "feature/x");
    }

    #[test]
    fn the_head_a_review_request_and_the_project_are_read_off_a_merge_request() {
        // FEAT-087: what the Review screen's inbox needs from a row.
        let raw = serde_json::json!([{
            "id": 1, "iid": 7, "title": "Retry the lock",
            "author": { "username": "nour" },
            "sha": "c0ffee",
            "reviewers": [{ "username": "someone" }, { "username": "reviewer1" }],
            "references": { "full": "team/sub/project!7" }
        }]);

        let row = &parse_merge_requests(&raw, "reviewer1")[0];
        assert_eq!(row.head_sha, "c0ffee");
        assert!(row.review_requested);
        assert_eq!(row.repository.as_deref(), Some("team/sub/project"));

        let other = &parse_merge_requests(&raw, "nobody")[0];
        assert!(!other.review_requested);
    }
}
