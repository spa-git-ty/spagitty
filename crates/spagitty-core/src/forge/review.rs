// SPDX-License-Identifier: GPL-3.0-or-later

//! A pull request's files, and the review somebody leaves on it.
//!
//! # Why REST here, when the list is GraphQL
//!
//! [`super::github`] uses GraphQL because the list needs 1 + 3N requests over
//! REST and one over GraphQL. Neither reason applies here.
//!
//! A pull request's files are one endpoint answering with everything: the
//! path, the status, the counts, and the unified patch. GraphQL's `files`
//! connection carries the first three and **not** the patch — there is no field
//! for it — so a GraphQL route would list the files and then need a REST call
//! per file to show any of them. Submitting a review is a mutation with three
//! inputs, where REST is one POST and GraphQL needs the pull request's node id
//! fetched first.
//!
//! So: the list is GraphQL because REST would be N requests; this is REST
//! because GraphQL would be. Both times the count decides, not the fashion.
//!
//! # The patch parser is pure and separate
//!
//! [`parse_patch`] takes a string and returns hunks. Every shape a host can
//! send — a hunk header with no line counts, a file with no trailing newline, a
//! patch that is only additions — is a test with a fixture rather than
//! something discovered against somebody's real pull request.
//!
//! Spagitty does not re-derive the diff from the two blobs, because it does not
//! have them: the head branch of a pull request usually is not fetched, and
//! fetching it to render a file list would turn reading a review into a network
//! operation on the repository. The host already computed this diff; the patch
//! it sends is that computation.

use serde_json::Value;

use crate::diff::{DiffLine, FileDiff, FileStatus, Hunk, LineOrigin};
use crate::forge::{github, gitlab, http, status_error, Kind, Repo};
use crate::{Error, Result};

/// Merge strategies supported across forges (FEAT-071).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MergeMethod {
    Merge,
    Squash,
    Rebase,
}

impl MergeMethod {
    pub fn as_str(self) -> &'static str {
        match self {
            MergeMethod::Merge => "merge",
            MergeMethod::Squash => "squash",
            MergeMethod::Rebase => "rebase",
        }
    }
}

/// Files per page, and the most GitHub will answer with.
const PER_PAGE: usize = 100;

/// How many pages are read before the list is called long enough.
///
/// GitHub stops sending files past 3000 in a pull request, so this is a
/// ceiling on a ceiling — it exists so that a host answering strangely cannot
/// spin this loop, not because a thousand-file review is expected to be read.
const MAX_PAGES: usize = 10;

/// What a reviewer is saying.
///
/// Spagitty's own words, mapped onto the host's at the edge — the same
/// discipline [`super::ReviewState`] follows. GitHub calls these `APPROVE`,
/// `REQUEST_CHANGES` and `COMMENT`; another host calls them something else and
/// the screen says the same three things either way.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ReviewVerdict {
    Approve,
    RequestChanges,
    Comment,
}

impl ReviewVerdict {
    /// What GitHub calls it.
    fn event(self) -> &'static str {
        match self {
            ReviewVerdict::Approve => "APPROVE",
            ReviewVerdict::RequestChanges => "REQUEST_CHANGES",
            ReviewVerdict::Comment => "COMMENT",
        }
    }

    /// Whether the host refuses this verdict without a comment.
    ///
    /// GitHub rejects `REQUEST_CHANGES` and `COMMENT` with an empty body, with
    /// a 422 that does not say which field is wrong. Asking here means the
    /// person is told what to do instead of being shown the host's confusion.
    pub fn needs_a_comment(self) -> bool {
        !matches!(self, ReviewVerdict::Approve)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PullRequestCommit {
    pub sha: String,
    pub short: String,
    pub summary: String,
    pub author_name: String,
    pub author_email: String,
    pub time: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PullRequestComment {
    pub id: u64,
    pub in_reply_to: Option<u64>,
    pub path: String,
    pub line: Option<u32>,
    pub side: String,
    pub body: String,
    pub author: String,
    pub created_at: i64,
    pub resolved: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DraftComment {
    pub path: String,
    /// The line the comment is on — the last, when it covers a range.
    pub line: u32,
    /// `RIGHT` numbers `line` in the new version, `LEFT` in the old.
    pub side: String,
    pub body: String,
    /// The first line, when the comment covers a range, and its side.
    #[serde(default)]
    pub start_line: Option<u32>,
    #[serde(default)]
    pub start_side: Option<String>,
    /// Where `line` sits in the diff, and where `start_line` does: GitLab
    /// places a comment by both versions' counters rather than by one side's
    /// number (FEAT-088). Absent from an older caller, which gets one side.
    #[serde(default)]
    pub place: Option<LinePlace>,
    #[serde(default)]
    pub start_place: Option<LinePlace>,
    /// The file's path before the change, when it was renamed.
    #[serde(default)]
    pub old_path: Option<String>,
}

/// One line of a diff, as GitLab counts it (FEAT-088).
///
/// `old` and `new` are the two versions' counters at that line: the line's
/// own number on the side it is on, and on the other side the number of the
/// next line there. An unchanged line has its number on both.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LinePlace {
    pub kind: LineKind,
    pub old: u32,
    pub new: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LineKind {
    Added,
    Removed,
    Context,
}

/// `{api_base}/repos/{owner}/{name}/pulls/{number}` — everything here hangs
/// off it.
fn pull_url(repo: &Repo, number: u64) -> String {
    format!(
        "{}/repos/{}/{}/pulls/{}",
        repo.kind.api_base(&repo.host),
        repo.owner,
        repo.name,
        number
    )
}

/// The files a pull request changes, with their diffs.
pub fn pull_request_files(repo: &Repo, token: &str, number: u64) -> Result<Vec<FileDiff>> {
    if repo.kind == Kind::GitLab {
        return gitlab::merge_request_files(repo, token, number);
    }
    let mut files = Vec::new();

    for page in 1..=MAX_PAGES {
        let url = format!(
            "{}/files?per_page={PER_PAGE}&page={page}",
            pull_url(repo, number)
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

        let batch = read_files(&response.body, &repo.host)?;
        let full = batch.len() == PER_PAGE;
        files.extend(batch);

        if !full {
            break;
        }
    }

    Ok(files)
}

/// The commits in one pull request.
pub fn pull_request_commits(
    repo: &Repo,
    token: &str,
    number: u64,
) -> Result<Vec<PullRequestCommit>> {
    if repo.kind == Kind::GitLab {
        return gitlab::merge_request_commits(repo, token, number);
    }
    let mut commits = Vec::new();

    for page in 1..=MAX_PAGES {
        let url = format!(
            "{}/commits?per_page={PER_PAGE}&page={page}",
            pull_url(repo, number)
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

        let batch = read_commits(&response.body, &repo.host)?;
        let full = batch.len() == PER_PAGE;
        commits.extend(batch);

        if !full {
            break;
        }
    }

    Ok(commits)
}

/// Parse commits response from host.
pub fn read_commits(body: &str, host: &str) -> Result<Vec<PullRequestCommit>> {
    let json: Value = serde_json::from_str(body).map_err(|_| Error::Forge {
        host: host.to_string(),
        detail: "sent something that is not JSON".into(),
    })?;

    let Some(entries) = json.as_array() else {
        return Err(Error::Forge {
            host: host.to_string(),
            detail: match json["message"].as_str() {
                Some(msg) => msg.to_string(),
                None => "did not send a list of commits".into(),
            },
        });
    };

    Ok(entries
        .iter()
        .filter_map(|e| {
            let sha = e["sha"].as_str()?.to_string();
            let short = if sha.len() >= 7 {
                sha[..7].to_string()
            } else {
                sha.clone()
            };
            let commit = &e["commit"];
            let raw_msg = commit["message"].as_str().unwrap_or("");
            let summary = raw_msg.lines().next().unwrap_or("").to_string();
            let author_obj = &commit["author"];
            let author_name = author_obj["name"]
                .as_str()
                .or_else(|| e["author"]["login"].as_str())
                .unwrap_or("Unknown")
                .to_string();
            let author_email = author_obj["email"].as_str().unwrap_or("").to_string();
            let time = super::github::timestamp(author_obj["date"].as_str());

            Some(PullRequestCommit {
                sha,
                short,
                summary,
                author_name,
                author_email,
                time,
            })
        })
        .collect())
}

/// Read the files changed by one specific commit.
pub fn commit_files(repo: &Repo, token: &str, sha: &str) -> Result<Vec<FileDiff>> {
    if repo.kind == Kind::GitLab {
        return gitlab::commit_files(repo, token, sha);
    }
    let url = format!(
        "{}/repos/{}/{}/commits/{}",
        repo.kind.api_base(&repo.host),
        repo.owner,
        repo.name,
        sha
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

    read_commit_files(&response.body, &repo.host)
}

/// Parse commit details for its changed files.
pub fn read_commit_files(body: &str, host: &str) -> Result<Vec<FileDiff>> {
    let json: Value = serde_json::from_str(body).map_err(|_| Error::Forge {
        host: host.to_string(),
        detail: "sent something that is not JSON".into(),
    })?;

    if let Some(files) = json["files"].as_array() {
        return Ok(files.iter().filter_map(file).collect());
    }

    if let Some(msg) = json["message"].as_str() {
        return Err(Error::Forge {
            host: host.to_string(),
            detail: msg.to_string(),
        });
    }

    Ok(Vec::new())
}

/// Read inline review comments on a pull request.
pub fn pull_request_comments(
    repo: &Repo,
    token: &str,
    number: u64,
) -> Result<Vec<PullRequestComment>> {
    if repo.kind == Kind::GitLab {
        return gitlab::merge_request_comments(repo, token, number);
    }
    let mut comments = Vec::new();

    for page in 1..=MAX_PAGES {
        let url = format!(
            "{}/comments?per_page={PER_PAGE}&page={page}",
            pull_url(repo, number)
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

        let batch = read_comments(&response.body, &repo.host)?;
        let full = batch.len() == PER_PAGE;
        comments.extend(batch);

        if !full {
            break;
        }
    }

    Ok(comments)
}

/// Parse review comments from JSON body.
pub fn read_comments(body: &str, host: &str) -> Result<Vec<PullRequestComment>> {
    let json: Value = serde_json::from_str(body).map_err(|_| Error::Forge {
        host: host.to_string(),
        detail: "sent something that is not JSON".into(),
    })?;

    let Some(entries) = json.as_array() else {
        return Err(Error::Forge {
            host: host.to_string(),
            detail: match json["message"].as_str() {
                Some(msg) => msg.to_string(),
                None => "did not send a list of comments".into(),
            },
        });
    };

    Ok(entries.iter().filter_map(comment_item).collect())
}

fn comment_item(entry: &Value) -> Option<PullRequestComment> {
    let id = entry["id"].as_u64()?;
    let path = entry["path"].as_str()?.to_string();
    let body = entry["body"].as_str().unwrap_or("").to_string();
    let author = entry["user"]["login"].as_str().unwrap_or("").to_string();
    let in_reply_to = entry["in_reply_to_id"].as_u64();
    let line = entry["line"]
        .as_u64()
        .or_else(|| entry["original_line"].as_u64())
        .map(|l| l as u32);
    let side = entry["side"].as_str().unwrap_or("RIGHT").to_string();
    let created_at = super::github::timestamp(entry["created_at"].as_str());

    Some(PullRequestComment {
        id,
        in_reply_to,
        path,
        line,
        side,
        body,
        author,
        created_at,
        resolved: false,
    })
}

/// Reply to an existing review comment.
pub fn reply_comment(
    repo: &Repo,
    token: &str,
    number: u64,
    comment_id: u64,
    body: &str,
) -> Result<PullRequestComment> {
    if body.trim().is_empty() {
        return Err(Error::Forge {
            host: repo.host.clone(),
            detail: "reply comment cannot be empty".into(),
        });
    }
    if repo.kind == Kind::GitLab {
        return gitlab::reply(repo, token, number, comment_id, body);
    }

    let payload = serde_json::json!({ "body": body }).to_string();
    let response = http::post_json(
        &format!("{}/comments/{}/replies", pull_url(repo, number), comment_id),
        token,
        &repo.host,
        &payload,
    )?;

    if response.status < 200 || response.status >= 300 {
        return Err(status_error(
            &repo.host,
            response.status,
            &response.body,
            response.retry_after.as_deref(),
        ));
    }

    read_comment(&response.body, &repo.host)
}

/// Parse a single comment from JSON response.
pub fn read_comment(body: &str, host: &str) -> Result<PullRequestComment> {
    let json: Value = serde_json::from_str(body).map_err(|_| Error::Forge {
        host: host.to_string(),
        detail: "sent something that is not JSON".into(),
    })?;

    comment_item(&json).ok_or_else(|| Error::Forge {
        host: host.to_string(),
        detail: "failed to parse comment from response".into(),
    })
}

/// Leave a review on a pull request with optional draft comments.
pub fn submit_review_with_comments(
    repo: &Repo,
    token: &str,
    number: u64,
    verdict: ReviewVerdict,
    comment: &str,
    draft_comments: &[DraftComment],
) -> Result<()> {
    if verdict.needs_a_comment() && comment.trim().is_empty() && draft_comments.is_empty() {
        return Err(Error::Forge {
            host: repo.host.clone(),
            detail: "this review needs a comment or inline change request to go with it".into(),
        });
    }
    if repo.kind == Kind::GitLab {
        return gitlab::submit_review(repo, token, number, verdict, comment, draft_comments);
    }

    let raw_comments: Vec<Value> = draft_comments
        .iter()
        .map(|c| {
            let mut raw = serde_json::json!({
                "path": c.path,
                "line": c.line,
                "side": c.side,
                "body": c.body
            });
            // A comment over several lines names where it starts as well.
            if let Some(start) = c.start_line.filter(|start| *start < c.line) {
                raw["start_line"] = serde_json::json!(start);
                raw["start_side"] =
                    serde_json::json!(c.start_side.clone().unwrap_or_else(|| c.side.clone()));
            }
            raw
        })
        .collect();

    let mut map = serde_json::Map::new();
    map.insert("event".into(), Value::String(verdict.event().into()));
    if !comment.trim().is_empty() {
        map.insert("body".into(), Value::String(comment.to_string()));
    }
    if !raw_comments.is_empty() {
        map.insert("comments".into(), Value::Array(raw_comments));
    }

    let payload = Value::Object(map).to_string();
    let response = http::post_json(
        &format!("{}/reviews", pull_url(repo, number)),
        token,
        &repo.host,
        &payload,
    )?;

    if response.status < 200 || response.status >= 300 {
        return Err(status_error(
            &repo.host,
            response.status,
            &response.body,
            response.retry_after.as_deref(),
        ));
    }

    Ok(())
}

/// Leave a review on a pull request.
pub fn submit_review(
    repo: &Repo,
    token: &str,
    number: u64,
    verdict: ReviewVerdict,
    comment: &str,
) -> Result<()> {
    submit_review_with_comments(repo, token, number, verdict, comment, &[])
}

/// What a lifecycle call sends, worked out before anything is sent (FEAT-071).
///
/// Merging, closing, and un-drafting are three actions across three forges, and
/// what differs between them is only the verb, the path, and the payload — the
/// part that is easy to get wrong and impossible to check by reading. Building
/// that separately from sending it is what lets a test assert the exact request
/// each host would receive without a network.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Request {
    verb: Verb,
    url: String,
    body: String,
}

/// The verb a lifecycle request goes out with.
///
/// The forges are specific: GitHub merges with `PUT` and edits with `PATCH`,
/// GitLab does both with `PUT`, and only Bitbucket's two endpoints are `POST`.
/// A wrong verb is answered with a method error rather than the change, which
/// is why this is carried in the request rather than assumed at the call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Verb {
    Post,
    Put,
    Patch,
}

/// Send a built request and read its status as an outcome.
///
/// The status check is here once rather than at each of the nine places a
/// lifecycle request is built.
fn send(request: &Request, token: &str, host: &str) -> Result<()> {
    let response = match request.verb {
        Verb::Post => http::post_json(&request.url, token, host, &request.body)?,
        Verb::Put => http::put_json(&request.url, token, host, &request.body)?,
        Verb::Patch => http::patch_json(&request.url, token, host, &request.body)?,
    };

    if response.status < 200 || response.status >= 300 {
        return Err(status_error(
            host,
            response.status,
            &response.body,
            response.retry_after.as_deref(),
        ));
    }

    Ok(())
}

/// GitLab addresses a project by its URL-encoded path rather than by owner and
/// name, so the slash between them is escaped.
fn project_path(repo: &Repo) -> String {
    repo.encoded_slug()
}

/// The request that merges a pull request on `repo`'s host (FEAT-071).
fn merge_request(
    repo: &Repo,
    number: u64,
    method: MergeMethod,
    commit_title: Option<&str>,
    commit_message: Option<&str>,
) -> Request {
    let base = repo.kind.api_base(&repo.host);

    match repo.kind {
        Kind::GitHub => {
            let mut payload = serde_json::json!({ "merge_method": method.as_str() });
            // A blank title or message is the caller saying "let the host
            // decide", which is what omitting the field means. Sending an empty
            // string instead would set an empty commit subject.
            if let Some(title) = commit_title.map(str::trim).filter(|t| !t.is_empty()) {
                payload["commit_title"] = serde_json::json!(title);
            }
            if let Some(message) = commit_message.map(str::trim).filter(|m| !m.is_empty()) {
                payload["commit_message"] = serde_json::json!(message);
            }

            Request {
                verb: Verb::Put,
                url: format!(
                    "{base}/repos/{}/{}/pulls/{number}/merge",
                    repo.owner, repo.name
                ),
                body: payload.to_string(),
            }
        }
        Kind::GitLab => Request {
            verb: Verb::Put,
            url: format!(
                "{base}/projects/{}/merge_requests/{number}/merge",
                project_path(repo)
            ),
            // GitLab has no rebase-on-merge over the API, so a rebase request
            // merges without squashing rather than failing: the strategies it
            // offers are merge and squash.
            body: serde_json::json!({ "squash": method == MergeMethod::Squash }).to_string(),
        },
        Kind::Bitbucket => Request {
            verb: Verb::Post,
            url: format!(
                "{base}/repositories/{}/{}/pullrequests/{number}/merge",
                repo.owner, repo.name
            ),
            body: serde_json::json!({
                "merge_strategy": match method {
                    MergeMethod::Merge => "merge_commit",
                    MergeMethod::Squash => "squash",
                    MergeMethod::Rebase => "fast_forward",
                }
            })
            .to_string(),
        },
    }
}

/// The request that closes a pull request without merging it (FEAT-071).
fn close_request(repo: &Repo, number: u64) -> Request {
    let base = repo.kind.api_base(&repo.host);

    match repo.kind {
        Kind::GitHub => Request {
            verb: Verb::Patch,
            url: format!("{base}/repos/{}/{}/pulls/{number}", repo.owner, repo.name),
            body: serde_json::json!({ "state": "closed" }).to_string(),
        },
        Kind::GitLab => Request {
            verb: Verb::Put,
            url: format!(
                "{base}/projects/{}/merge_requests/{number}",
                project_path(repo)
            ),
            body: serde_json::json!({ "state_event": "close" }).to_string(),
        },
        Kind::Bitbucket => Request {
            verb: Verb::Post,
            url: format!(
                "{base}/repositories/{}/{}/pullrequests/{number}/decline",
                repo.owner, repo.name
            ),
            body: "{}".into(),
        },
    }
}

/// The title with any draft marker taken off the front.
///
/// GitLab has no draft flag: a merge request is a draft when its title starts
/// with `Draft:`, case-insensitively, and `WIP:` is the older spelling it still
/// honours. Stripping in a loop means a title that already carries one — or
/// two, from a title edited by hand — does not gain another.
fn without_draft_prefix(title: &str) -> &str {
    let mut text = title.trim();

    loop {
        let stripped = ["Draft:", "draft:", "WIP:", "wip:"]
            .iter()
            .find_map(|marker| text.strip_prefix(marker));

        match stripped {
            Some(rest) => text = rest.trim_start(),
            None => return text,
        }
    }
}

/// The request that moves a GitLab merge request in or out of draft (FEAT-071).
///
/// GitLab only — GitHub converts draft state over GraphQL, and Bitbucket has no
/// draft at all. Both are decided in [`set_draft_status`] rather than here.
fn gitlab_draft_request(repo: &Repo, number: u64, title: &str, draft: bool) -> Request {
    let cleaned = without_draft_prefix(title);
    let new_title = if draft {
        format!("Draft: {cleaned}")
    } else {
        cleaned.to_string()
    };

    Request {
        verb: Verb::Put,
        url: format!(
            "{}/projects/{}/merge_requests/{number}",
            repo.kind.api_base(&repo.host),
            project_path(repo)
        ),
        body: serde_json::json!({ "title": new_title }).to_string(),
    }
}

/// Merge a pull request (FEAT-071).
pub fn merge_pull_request(
    repo: &Repo,
    token: &str,
    number: u64,
    method: MergeMethod,
    commit_title: Option<&str>,
    commit_message: Option<&str>,
) -> Result<()> {
    let request = merge_request(repo, number, method, commit_title, commit_message);
    send(&request, token, &repo.host)
}

/// Close / reject a pull request without merging (FEAT-071).
pub fn close_pull_request(repo: &Repo, token: &str, number: u64) -> Result<()> {
    let request = close_request(repo, number);
    send(&request, token, &repo.host)
}

/// Set draft / ready-for-review status on a pull request (FEAT-071).
///
/// `id` is the host's own identifier for the pull request, which GitHub needs
/// and the others ignore; `title` is what GitLab rewrites and the others
/// ignore. Each host is given the one it can act on.
pub fn set_draft_status(
    repo: &Repo,
    token: &str,
    number: u64,
    id: &str,
    title: &str,
    draft: bool,
) -> Result<()> {
    match repo.kind {
        // GitHub's REST update accepts a `draft` field and does nothing with
        // it; converting is a GraphQL mutation on the pull request's node id.
        Kind::GitHub => github::set_draft(&repo.host, token, id, draft),
        Kind::GitLab => send(
            &gitlab_draft_request(repo, number, title, draft),
            token,
            &repo.host,
        ),
        // Bitbucket Cloud has no draft pull request. The workspace hides the
        // control, and this refuses rather than faking the state with a title
        // prefix that Bitbucket would treat as ordinary text.
        Kind::Bitbucket => Err(Error::Forge {
            host: repo.host.clone(),
            detail: "Bitbucket has no draft pull requests".into(),
        }),
    }
}

/// Turn the host's file list into the shape the Diff screen already renders.
///
/// Makes no request.
pub fn read_files(body: &str, host: &str) -> Result<Vec<FileDiff>> {
    let json: Value = serde_json::from_str(body).map_err(|_| Error::Forge {
        host: host.to_string(),
        detail: "sent something that is not JSON".into(),
    })?;

    let Some(entries) = json.as_array() else {
        // A message where a list should be is how GitHub reports "no such pull
        // request" with a 200 in front of a proxy. Saying so beats an empty
        // file list, which reads as a pull request that changes nothing.
        return Err(Error::Forge {
            host: host.to_string(),
            detail: match json["message"].as_str() {
                Some(message) => message.to_string(),
                None => "did not send a list of files".into(),
            },
        });
    };

    Ok(entries.iter().filter_map(file).collect())
}

/// One entry, or nothing when it carries no path to show it under.
fn file(entry: &Value) -> Option<FileDiff> {
    let path = entry["filename"].as_str()?.to_string();
    let patch = entry["patch"].as_str();

    Some(FileDiff {
        path,
        status: status_of(entry["status"].as_str()),
        // No patch and a changed file means the host would not send one:
        // binary, or past its own size ceiling. It does not say which, and
        // guessing between them would be inventing a fact.
        binary: patch.is_none(),
        too_large: false,
        added: entry["additions"].as_u64().unwrap_or(0) as u32,
        removed: entry["deletions"].as_u64().unwrap_or(0) as u32,
        hunks: patch.map(parse_patch).unwrap_or_default(),
    })
}

/// GitHub's word for what happened to a file, in Spagitty's vocabulary.
///
/// `copied` and `changed` are GitHub's own; both are a file that differs from
/// the base, which is what `Modified` means here. An unknown word is
/// `Modified` too — the file is in a list of changed files, so something
/// happened to it, and that is the honest floor.
fn status_of(status: Option<&str>) -> FileStatus {
    match status {
        Some("added") => FileStatus::Added,
        Some("removed") => FileStatus::Deleted,
        Some("renamed") => FileStatus::Renamed,
        _ => FileStatus::Modified,
    }
}

/// Parse a unified diff into hunks. Makes no request and reads no repository.
pub fn parse_patch(patch: &str) -> Vec<Hunk> {
    let mut hunks: Vec<Hunk> = Vec::new();
    let mut old = 0u32;
    let mut new = 0u32;

    // One trailing newline terminates the last line rather than starting an
    // empty one. Splitting without stripping it appends a phantom segment,
    // which the blank-context rule below would then take for a real line.
    let body = patch.strip_suffix('\n').unwrap_or(patch);

    for line in body.split('\n') {
        if line.starts_with("@@") {
            if let Some(header) = parse_hunk_header(line) {
                old = header.old_start;
                new = header.new_start;
                hunks.push(header);
            }
            continue;
        }

        // Anything before the first `@@` is preamble — a `diff --git` line, a
        // mode change — and belongs to no hunk.
        let Some(hunk) = hunks.last_mut() else {
            continue;
        };

        // "\ No newline at end of file" annotates the line above and is not a
        // line of either version.
        if line.starts_with('\\') {
            continue;
        }

        let (origin, text) = match line.chars().next() {
            Some('+') => (LineOrigin::Added, &line[1..]),
            Some('-') => (LineOrigin::Removed, &line[1..]),
            Some(' ') => (LineOrigin::Context, &line[1..]),
            // An empty line inside a hunk is a context line whose single space
            // was stripped somewhere between the host and here. Some hosts do
            // strip it; treating it as the end of the hunk would drop
            // everything after it.
            None => (LineOrigin::Context, line),
            // A line with any other prefix is not part of this hunk's body.
            Some(_) => continue,
        };

        let (at_old, at_new) = match origin {
            LineOrigin::Added => (None, Some(new)),
            LineOrigin::Removed => (Some(old), None),
            LineOrigin::Context => (Some(old), Some(new)),
        };

        if at_old.is_some() {
            old += 1;
        }
        if at_new.is_some() {
            new += 1;
        }

        hunk.lines.push(DiffLine {
            origin,
            old: at_old,
            new: at_new,
            text: text.to_string(),
        });
    }

    hunks
}

/// `@@ -12,7 +14,9 @@ fn thing()` — an empty hunk carrying its own numbers.
///
/// A count may be left out, which means one line: `@@ -1 +1 @@` is legal and
/// GitHub sends it for a single-line file.
fn parse_hunk_header(line: &str) -> Option<Hunk> {
    let inner = line.strip_prefix("@@")?;
    let (ranges, _) = inner.split_once("@@")?;

    let mut old_range = None;
    let mut new_range = None;
    for part in ranges.split_whitespace() {
        if let Some(rest) = part.strip_prefix('-') {
            old_range = parse_range(rest);
        } else if let Some(rest) = part.strip_prefix('+') {
            new_range = parse_range(rest);
        }
    }

    let (old_start, old_lines) = old_range?;
    let (new_start, new_lines) = new_range?;

    Some(Hunk {
        old_start,
        old_lines,
        new_start,
        new_lines,
        header: line.to_string(),
        lines: Vec::new(),
    })
}

/// `12,7` or `12`. The second form is one line.
fn parse_range(text: &str) -> Option<(u32, u32)> {
    match text.split_once(',') {
        Some((start, count)) => Some((start.parse().ok()?, count.parse().ok()?)),
        None => Some((text.parse().ok()?, 1)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forge::Kind;

    fn repo() -> Repo {
        Repo {
            kind: Kind::GitHub,
            host: "github.com".into(),
            owner: "spa-git-ty".into(),
            name: "spagitty".into(),
        }
    }

    #[test]
    fn a_patch_becomes_the_hunks_the_diff_screen_renders() {
        let hunks = parse_patch("@@ -1,3 +1,4 @@ fn main()\n one\n-two\n+three\n+four\n five\n");

        assert_eq!(hunks.len(), 1);
        let hunk = &hunks[0];
        assert_eq!((hunk.old_start, hunk.old_lines), (1, 3));
        assert_eq!((hunk.new_start, hunk.new_lines), (1, 4));
        assert_eq!(hunk.header, "@@ -1,3 +1,4 @@ fn main()");
        assert_eq!(hunk.lines.len(), 5);
    }

    #[test]
    fn every_line_carries_the_number_it_has_in_the_version_it_belongs_to() {
        let hunks = parse_patch("@@ -10,3 +20,3 @@\n a\n-b\n+c\n d\n");
        let lines = &hunks[0].lines;

        // Context: in both, and both advance.
        assert_eq!((lines[0].old, lines[0].new), (Some(10), Some(20)));
        // Removed: in the old only, and only the old advances.
        assert_eq!((lines[1].old, lines[1].new), (Some(11), None));
        // Added: in the new only.
        assert_eq!((lines[2].old, lines[2].new), (None, Some(21)));
        // The next context line resumes from where each side actually is.
        assert_eq!((lines[3].old, lines[3].new), (Some(12), Some(22)));
    }

    #[test]
    fn the_line_text_arrives_without_its_diff_prefix() {
        let hunks = parse_patch("@@ -1,1 +1,2 @@\n keep\n+    indented\n");

        assert_eq!(hunks[0].lines[0].text, "keep");
        // The prefix goes and the indentation stays — stripping a whole
        // leading run would eat the code's own shape.
        assert_eq!(hunks[0].lines[1].text, "    indented");
    }

    #[test]
    fn a_hunk_header_with_no_count_means_one_line() {
        // `@@ -1 +1 @@` is legal and GitHub sends it for a single-line file.
        let hunks = parse_patch("@@ -1 +1 @@\n-old\n+new\n");

        assert_eq!((hunks[0].old_start, hunks[0].old_lines), (1, 1));
        assert_eq!((hunks[0].new_start, hunks[0].new_lines), (1, 1));
    }

    #[test]
    fn several_hunks_in_one_patch_each_keep_their_own_numbering() {
        let hunks = parse_patch("@@ -1,2 +1,2 @@\n a\n-b\n+c\n@@ -50,2 +50,2 @@\n x\n-y\n+z\n");

        assert_eq!(hunks.len(), 2);
        assert_eq!(hunks[1].old_start, 50);
        // The second hunk counts from its own header, not from where the
        // first one left off.
        assert_eq!(hunks[1].lines[0].old, Some(50));
    }

    #[test]
    fn the_no_newline_marker_is_not_a_line_of_either_version() {
        let hunks = parse_patch("@@ -1,1 +1,1 @@\n-old\n\\ No newline at end of file\n+new\n");

        assert_eq!(hunks[0].lines.len(), 2);
        assert!(hunks[0]
            .lines
            .iter()
            .all(|line| !line.text.contains("No newline")));
    }

    #[test]
    fn an_empty_line_inside_a_hunk_is_context_rather_than_the_end_of_it() {
        // Some hosts strip the single leading space from a blank context line.
        // Treating that as the end of the hunk would drop everything after it.
        let hunks = parse_patch("@@ -1,3 +1,3 @@\n a\n\n+b\n");

        assert_eq!(hunks[0].lines.len(), 3);
        assert_eq!(hunks[0].lines[1].origin, LineOrigin::Context);
    }

    #[test]
    fn anything_before_the_first_hunk_header_belongs_to_no_hunk() {
        let hunks = parse_patch("diff --git a/x b/x\nindex 1..2 100644\n@@ -1,1 +1,1 @@\n a\n");

        assert_eq!(hunks.len(), 1);
        assert_eq!(hunks[0].lines.len(), 1);
    }

    #[test]
    fn a_patch_that_is_not_a_patch_is_no_hunks_rather_than_a_panic() {
        assert!(parse_patch("").is_empty());
        assert!(parse_patch("@@ this is not a header @@\n a\n").is_empty());
        assert!(parse_patch("@@ -notanumber,3 +1,2 @@\n a\n").is_empty());
    }

    #[test]
    fn a_file_list_arrives_in_the_shape_the_screen_renders() {
        let files = read_files(
            r#"[{"filename":"src/main.rs","status":"modified","additions":2,"deletions":1,
                 "patch":"@@ -1,2 +1,3 @@\n a\n-b\n+c\n+d\n"}]"#,
            "github.com",
        )
        .expect("a list");

        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path, "src/main.rs");
        assert_eq!(files[0].status, FileStatus::Modified);
        assert_eq!((files[0].added, files[0].removed), (2, 1));
        assert!(!files[0].binary);
        assert_eq!(files[0].hunks.len(), 1);
    }

    #[test]
    fn every_status_github_sends_has_a_word_the_screen_knows() {
        for (sent, expected) in [
            ("added", FileStatus::Added),
            ("removed", FileStatus::Deleted),
            ("renamed", FileStatus::Renamed),
            ("modified", FileStatus::Modified),
            // GitHub's own, and both mean a file that differs from the base.
            ("changed", FileStatus::Modified),
            ("copied", FileStatus::Modified),
            // A word nothing knows is still a file in a list of changed files.
            ("something-new", FileStatus::Modified),
        ] {
            let body = format!(r#"[{{"filename":"x","status":"{sent}"}}]"#);
            let files = read_files(&body, "github.com").expect("a list");
            assert_eq!(files[0].status, expected, "status {sent}");
        }
    }

    #[test]
    fn a_file_the_host_sends_no_patch_for_is_marked_rather_than_shown_empty() {
        // Binary, or past the host's own size ceiling. It does not say which,
        // and an empty hunk list would render as a file that changed nothing.
        let files = read_files(
            r#"[{"filename":"icon.png","status":"modified","additions":0,"deletions":0}]"#,
            "github.com",
        )
        .expect("a list");

        assert!(files[0].binary);
        assert!(files[0].hunks.is_empty());
    }

    #[test]
    fn an_entry_with_no_path_is_skipped_rather_than_invented() {
        let files = read_files(
            r#"[{"status":"modified"},{"filename":"real.rs"}]"#,
            "github.com",
        )
        .expect("a list");

        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path, "real.rs");
    }

    #[test]
    fn an_empty_list_is_an_empty_list_and_not_an_error() {
        assert!(read_files("[]", "github.com").expect("a list").is_empty());
    }

    #[test]
    fn a_message_where_a_list_should_be_is_reported_in_the_hosts_own_words() {
        // A 200 carrying an object is how this arrives through some proxies,
        // and an empty file list would read as a pull request changing nothing.
        let refused = read_files(r#"{"message":"Not Found"}"#, "github.com");

        match refused {
            Err(Error::Forge { detail, .. }) => assert_eq!(detail, "Not Found"),
            other => panic!("expected the host's words, got {other:?}"),
        }
    }

    #[test]
    fn something_that_is_not_json_is_an_error_rather_than_a_panic() {
        assert!(read_files("<html>502</html>", "github.com").is_err());
    }

    #[test]
    fn a_verdict_that_needs_a_comment_is_refused_before_anything_is_sent() {
        // GitHub answers an empty body with a 422 that does not name the field.
        // Refusing here tells the person what to do instead.
        for verdict in [ReviewVerdict::RequestChanges, ReviewVerdict::Comment] {
            let refused = submit_review(&repo(), "token", 1, verdict, "   ");

            match refused {
                Err(Error::Forge { detail, .. }) => assert!(detail.contains("needs a comment")),
                other => panic!("expected a refusal for {verdict:?}, got {other:?}"),
            }
        }
    }

    #[test]
    fn approving_needs_no_comment() {
        assert!(!ReviewVerdict::Approve.needs_a_comment());
        assert!(ReviewVerdict::RequestChanges.needs_a_comment());
        assert!(ReviewVerdict::Comment.needs_a_comment());
    }

    #[test]
    fn every_verdict_has_the_word_github_expects() {
        assert_eq!(ReviewVerdict::Approve.event(), "APPROVE");
        assert_eq!(ReviewVerdict::RequestChanges.event(), "REQUEST_CHANGES");
        assert_eq!(ReviewVerdict::Comment.event(), "COMMENT");
    }

    #[test]
    fn the_url_is_the_hosts_api_root_rather_than_github_com_for_an_enterprise_repo() {
        let enterprise = Repo {
            kind: Kind::GitHub,
            host: "git.example.com".into(),
            owner: "team".into(),
            name: "thing".into(),
        };

        assert_eq!(
            pull_url(&enterprise, 7),
            "https://git.example.com/api/v3/repos/team/thing/pulls/7"
        );
        assert_eq!(
            pull_url(&repo(), 7),
            "https://api.github.com/repos/spa-git-ty/spagitty/pulls/7"
        );
    }
    #[test]
    fn read_commits_parses_json_response() {
        let json = r#"[
            {
                "sha": "1234567890abcdef",
                "commit": {
                    "message": "fix: update parser\n\nMore details",
                    "author": { "name": "Grace", "email": "grace@example.com", "date": "2026-08-25T09:30:00Z" }
                }
            }
        ]"#;
        let commits = read_commits(json, "github.com").expect("commits");
        assert_eq!(commits.len(), 1);
        assert_eq!(commits[0].sha, "1234567890abcdef");
        assert_eq!(commits[0].short, "1234567");
        assert_eq!(commits[0].summary, "fix: update parser");
        assert_eq!(commits[0].author_name, "Grace");
        assert_eq!(commits[0].author_email, "grace@example.com");
        assert_eq!(commits[0].time, 1_787_650_200);
    }

    #[test]
    fn read_commit_files_parses_files_array() {
        let json = r#"{
            "sha": "1234567890abcdef",
            "files": [
                {
                    "filename": "src/main.rs",
                    "status": "modified",
                    "additions": 5,
                    "deletions": 2,
                    "patch": "@@ -1,2 +1,5 @@\n a\n+b\n+c\n+d\n"
                }
            ]
        }"#;
        let files = read_commit_files(json, "github.com").expect("commit files");
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path, "src/main.rs");
        assert_eq!(files[0].added, 5);
        assert_eq!(files[0].removed, 2);
    }

    #[test]
    fn read_comments_parses_inline_review_comments() {
        let json = r#"[
            {
                "id": 9988,
                "in_reply_to_id": null,
                "path": "src/lib.rs",
                "line": 42,
                "side": "RIGHT",
                "body": "Check this condition",
                "user": { "login": "reviewer1" },
                "created_at": "2026-08-25T09:30:00Z"
            },
            {
                "id": 9989,
                "in_reply_to_id": 9988,
                "path": "src/lib.rs",
                "line": 42,
                "side": "RIGHT",
                "body": "Addressed in latest commit",
                "user": { "login": "author1" },
                "created_at": "2026-08-25T10:00:00Z"
            }
        ]"#;
        let comments = read_comments(json, "github.com").expect("comments");
        assert_eq!(comments.len(), 2);
        assert_eq!(comments[0].id, 9988);
        assert_eq!(comments[0].in_reply_to, None);
        assert_eq!(comments[0].path, "src/lib.rs");
        assert_eq!(comments[0].line, Some(42));
        assert_eq!(comments[0].author, "reviewer1");
        assert_eq!(comments[1].in_reply_to, Some(9988));
        assert_eq!(comments[1].author, "author1");
    }

    #[test]
    fn read_comment_parses_single_reply() {
        let json = r#"{
            "id": 9990,
            "in_reply_to_id": 9988,
            "path": "src/lib.rs",
            "line": 42,
            "side": "RIGHT",
            "body": "Looks good now",
            "user": { "login": "reviewer1" },
            "created_at": "2026-08-25T10:30:00Z"
        }"#;
        let comment = read_comment(json, "github.com").expect("single comment");
        assert_eq!(comment.id, 9990);
        assert_eq!(comment.in_reply_to, Some(9988));
        assert_eq!(comment.body, "Looks good now");
    }

    #[test]
    fn submit_review_with_draft_comments_allows_empty_top_level_body() {
        // A review with inline comments doesn't strictly need a top-level body
        let draft = DraftComment {
            path: "src/main.rs".into(),
            line: 10,
            side: "RIGHT".into(),
            body: "Need change here".into(),
            ..DraftComment::default()
        };
        // Should not reject if draft_comments is non-empty even if comment is empty
        // (will hit network error in test without real server, but doesn't error on validation)
        let res = submit_review_with_comments(
            &repo(),
            "token",
            1,
            ReviewVerdict::RequestChanges,
            "",
            &[draft],
        );
        assert!(res.is_err());
        if let Err(Error::Forge { detail, .. }) = res {
            assert!(!detail.contains("needs a comment or inline change request"));
        }
    }

    // FEAT-071 — the lifecycle requests.
    //
    // Asserted on the built request rather than on a live call: the verb and
    // the path are exactly what a host rejects when they are wrong, and a
    // mocked command layer above this cannot see either of them.

    fn repo_on(kind: Kind, host: &str) -> Repo {
        Repo {
            kind,
            host: host.into(),
            owner: "spa-git-ty".into(),
            name: "spagitty".into(),
        }
    }

    fn payload(request: &Request) -> Value {
        serde_json::from_str(&request.body).expect("a request body is JSON")
    }

    #[test]
    fn github_merges_with_put_because_a_post_is_answered_with_a_method_error() {
        let request = merge_request(&repo(), 412, MergeMethod::Squash, None, None);

        assert_eq!(request.verb, Verb::Put);
        assert_eq!(
            request.url,
            "https://api.github.com/repos/spa-git-ty/spagitty/pulls/412/merge"
        );
        assert_eq!(
            payload(&request),
            serde_json::json!({ "merge_method": "squash" })
        );
    }

    #[test]
    fn a_blank_commit_title_is_omitted_rather_than_sent_empty() {
        // An empty `commit_title` is not "no title" to GitHub — it is a commit
        // with no subject. Omitting the field is how the host is left to pick.
        let request = merge_request(&repo(), 412, MergeMethod::Merge, Some("   "), Some(""));

        assert_eq!(
            payload(&request),
            serde_json::json!({ "merge_method": "merge" })
        );
    }

    #[test]
    fn a_commit_title_and_message_travel_trimmed() {
        let request = merge_request(
            &repo(),
            412,
            MergeMethod::Merge,
            Some("  Land the workspace  "),
            Some("  Closes #7  "),
        );

        assert_eq!(
            payload(&request),
            serde_json::json!({
                "merge_method": "merge",
                "commit_title": "Land the workspace",
                "commit_message": "Closes #7",
            })
        );
    }

    #[test]
    fn an_enterprise_host_is_addressed_at_its_own_api_path() {
        let request = merge_request(
            &repo_on(Kind::GitHub, "git.example.com"),
            412,
            MergeMethod::Merge,
            None,
            None,
        );

        assert_eq!(
            request.url,
            "https://git.example.com/api/v3/repos/spa-git-ty/spagitty/pulls/412/merge"
        );
    }

    #[test]
    fn gitlab_merges_with_put_on_a_url_encoded_project_path() {
        let request = merge_request(
            &repo_on(Kind::GitLab, "gitlab.com"),
            412,
            MergeMethod::Squash,
            None,
            None,
        );

        assert_eq!(request.verb, Verb::Put);
        assert_eq!(
            request.url,
            "https://gitlab.com/api/v4/projects/spa-git-ty%2Fspagitty/merge_requests/412/merge"
        );
        assert_eq!(payload(&request), serde_json::json!({ "squash": true }));
    }

    #[test]
    fn only_a_squash_sets_gitlabs_squash_flag() {
        for method in [MergeMethod::Merge, MergeMethod::Rebase] {
            let request = merge_request(
                &repo_on(Kind::GitLab, "gitlab.com"),
                412,
                method,
                None,
                None,
            );

            assert_eq!(payload(&request), serde_json::json!({ "squash": false }));
        }
    }

    #[test]
    fn bitbucket_merges_with_post_and_its_own_strategy_names() {
        let strategies = [
            (MergeMethod::Merge, "merge_commit"),
            (MergeMethod::Squash, "squash"),
            (MergeMethod::Rebase, "fast_forward"),
        ];

        for (method, expected) in strategies {
            let request = merge_request(
                &repo_on(Kind::Bitbucket, "bitbucket.org"),
                412,
                method,
                None,
                None,
            );

            assert_eq!(request.verb, Verb::Post);
            assert_eq!(
                request.url,
                "https://api.bitbucket.org/2.0/repositories/spa-git-ty/spagitty/pullrequests/412/merge"
            );
            assert_eq!(
                payload(&request),
                serde_json::json!({ "merge_strategy": expected })
            );
        }
    }

    #[test]
    fn github_closes_with_patch_on_the_pull_request_itself() {
        let request = close_request(&repo(), 412);

        assert_eq!(request.verb, Verb::Patch);
        assert_eq!(
            request.url,
            "https://api.github.com/repos/spa-git-ty/spagitty/pulls/412"
        );
        assert_eq!(payload(&request), serde_json::json!({ "state": "closed" }));
    }

    #[test]
    fn gitlab_closes_with_a_state_event_and_bitbucket_declines() {
        let gitlab = close_request(&repo_on(Kind::GitLab, "gitlab.com"), 412);
        assert_eq!(gitlab.verb, Verb::Put);
        assert_eq!(
            gitlab.url,
            "https://gitlab.com/api/v4/projects/spa-git-ty%2Fspagitty/merge_requests/412"
        );
        assert_eq!(
            payload(&gitlab),
            serde_json::json!({ "state_event": "close" })
        );

        let bitbucket = close_request(&repo_on(Kind::Bitbucket, "bitbucket.org"), 412);
        assert_eq!(bitbucket.verb, Verb::Post);
        assert_eq!(
            bitbucket.url,
            "https://api.bitbucket.org/2.0/repositories/spa-git-ty/spagitty/pullrequests/412/decline"
        );
    }

    #[test]
    fn gitlab_marks_a_draft_by_prefixing_the_title() {
        let request = gitlab_draft_request(
            &repo_on(Kind::GitLab, "gitlab.com"),
            412,
            "Workspace review overhaul",
            true,
        );

        assert_eq!(request.verb, Verb::Put);
        assert_eq!(
            request.url,
            "https://gitlab.com/api/v4/projects/spa-git-ty%2Fspagitty/merge_requests/412"
        );
        assert_eq!(
            payload(&request),
            serde_json::json!({ "title": "Draft: Workspace review overhaul" })
        );
    }

    #[test]
    fn a_title_that_is_already_a_draft_does_not_gain_a_second_prefix() {
        let request = gitlab_draft_request(
            &repo_on(Kind::GitLab, "gitlab.com"),
            412,
            "Draft: Workspace review overhaul",
            true,
        );

        assert_eq!(
            payload(&request),
            serde_json::json!({ "title": "Draft: Workspace review overhaul" })
        );
    }

    #[test]
    fn marking_ready_takes_every_spelling_of_the_marker_off() {
        // GitLab reads the marker case-insensitively and still honours `WIP:`,
        // so a title edited by hand can carry either, or both.
        let cases = [
            "Draft: Workspace review overhaul",
            "draft: Workspace review overhaul",
            "WIP: Workspace review overhaul",
            "Draft: WIP: Workspace review overhaul",
        ];

        for title in cases {
            let request =
                gitlab_draft_request(&repo_on(Kind::GitLab, "gitlab.com"), 412, title, false);

            assert_eq!(
                payload(&request),
                serde_json::json!({ "title": "Workspace review overhaul" }),
                "started from {title}"
            );
        }
    }

    #[test]
    fn bitbucket_refuses_a_draft_rather_than_faking_one_in_the_title() {
        // Bitbucket Cloud has no draft pull request. Writing `Draft:` into the
        // title would leave a state the host does not know it is in.
        let refused = set_draft_status(
            &repo_on(Kind::Bitbucket, "bitbucket.org"),
            "token",
            412,
            "PR_kwDO",
            "Workspace review overhaul",
            true,
        );

        match refused {
            Err(Error::Forge { detail, .. }) => assert!(detail.contains("no draft")),
            other => panic!("expected a refusal, got {other:?}"),
        }
    }
}
