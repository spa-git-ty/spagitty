// SPDX-License-Identifier: GPL-3.0-or-later

//! Bitbucket Cloud pull requests and API integration (FEAT-070).

use serde_json::Value;

use crate::error::{Error, Result};
use crate::forge::github::timestamp;
use crate::forge::{http, status_error, CheckState, PullRequest, Repo, ReviewState};

pub fn whoami(host: &str, token: &str) -> Result<String> {
    let url = format!("{}/user", crate::forge::Kind::Bitbucket.api_base(host));
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
        .or_else(|| json.get("display_name"))
        .and_then(Value::as_str)
        .map(String::from)
        .ok_or_else(|| Error::Forge {
            host: host.to_string(),
            detail: "Bitbucket response had no username".into(),
        })
}

pub fn pull_requests(repo: &Repo, token: &str, me: &str) -> Result<Vec<PullRequest>> {
    let url = format!(
        "{}/repositories/{}/{}/pullrequests?state=OPEN&pagelen=50",
        repo.kind.api_base(&repo.host),
        repo.owner,
        repo.name
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
    Ok(parse_pull_requests(&json, me))
}

pub fn create_pull_request(
    repo: &Repo,
    token: &str,
    title: &str,
    body: &str,
    head: &str,
    base: &str,
) -> Result<PullRequest> {
    let url = format!(
        "{}/repositories/{}/{}/pullrequests",
        repo.kind.api_base(&repo.host),
        repo.owner,
        repo.name
    );

    let payload = serde_json::json!({
        "title": title,
        "description": body,
        "source": { "branch": { "name": head } },
        "destination": { "branch": { "name": base } },
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
    parse_single_pr(&json, "").ok_or_else(|| Error::Forge {
        host: repo.host.clone(),
        detail: "Could not parse created Bitbucket pull request".into(),
    })
}

pub fn parse_pull_requests(json: &Value, me: &str) -> Vec<PullRequest> {
    let Some(array) = json.get("values").and_then(Value::as_array) else {
        return Vec::new();
    };
    array
        .iter()
        .filter_map(|node| parse_single_pr(node, me))
        .collect()
}

fn parse_single_pr(node: &Value, me: &str) -> Option<PullRequest> {
    let number = node.get("id")?.as_u64()?;
    let title = node.get("title")?.as_str()?.to_string();
    let body = node
        .get("description")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let author_name = node
        .get("author")
        .and_then(|a| a.get("display_name").or_else(|| a.get("username")))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();

    let updated_str = node.get("updated_on").and_then(Value::as_str);
    let updated = timestamp(updated_str);

    let source_branch = node
        .get("source")
        .and_then(|s| s.get("branch"))
        .and_then(|b| b.get("name"))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();

    let target_branch = node
        .get("destination")
        .and_then(|s| s.get("branch"))
        .and_then(|b| b.get("name"))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();

    let needs_you = !me.is_empty() && author_name != me;

    Some(PullRequest {
        id: number.to_string(),
        number,
        title,
        body,
        author_name,
        updated,
        source_branch,
        target_branch,
        draft: false,
        review: ReviewState::NoReviewers,
        checks: Some(CheckState::Passing),
        needs_you,
        needs_you_because: if needs_you {
            Some("Pull request awaiting review".into())
        } else {
            None
        },
        changed_files: 0,
        added: 0,
        removed: 0,
        mergeable: None,
        your_review: None,
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
    fn parses_bitbucket_pull_requests_json() {
        let raw = serde_json::json!({
            "values": [
                {
                    "id": 101,
                    "title": "Fix memory safety issue",
                    "description": "Details here",
                    "author": { "display_name": "Dev User", "username": "devuser" },
                    "updated_on": "2024-01-01T12:00:00Z",
                    "source": { "branch": { "name": "bugfix/101" } },
                    "destination": { "branch": { "name": "main" } }
                }
            ]
        });

        let list = parse_pull_requests(&raw, "reviewer");
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].number, 101);
        assert_eq!(list[0].author_name, "Dev User");
        assert_eq!(list[0].source_branch, "bugfix/101");
        assert!(list[0].needs_you);
    }
}
