// SPDX-License-Identifier: GPL-3.0-or-later

//! GitHub, mapped onto the shape the screen already renders.
//!
//! # One request, not 1 + 3N
//!
//! The REST API would need a list call, then a call per pull request for the
//! file and line counts, then another per pull request for the review decision,
//! then another for the checks. Thirty open pull requests is ninety-one
//! requests, against a budget of five thousand an hour shared with everything
//! else the token does.
//!
//! GraphQL asks for all of it once. That is the whole reason it is used here —
//! not preference, and not novelty. It also means the screen either has
//! everything or has nothing, which is a simpler thing to render than a list
//! where some rows know their line counts and others are still waiting.
//!
//! # The mapping is a pure function
//!
//! [`read_pull_requests`] takes JSON and returns rows. It makes no request, so
//! every shape a host can send — a missing author, a repository with no checks,
//! a review decision nobody has made yet — is a test with a fixture rather than
//! something discovered in production against somebody's real account.

use serde_json::Value;

use crate::forge::{http, status_error, CheckState, PullRequest, Repo, ReviewState};
use crate::{Error, Result};

/// How many pull requests are asked for.
///
/// A screen, not an archive. Someone with more than this open is not reading
/// the hundredth row, and the ordering is by last update so the ones that fell
/// off are the ones nobody has touched.
const LIMIT: usize = 50;

/// The API endpoint for GraphQL.
///
/// `github.com` answers at `api.github.com/graphql`; an Enterprise installation
/// answers at `<host>/api/graphql` — note **not** `/api/v3/graphql`, which is
/// the REST root and a different path.
fn graphql_url(host: &str) -> String {
    if host == "github.com" {
        "https://api.github.com/graphql".into()
    } else {
        format!("https://{host}/api/graphql")
    }
}

/// One pull request's fields, shared by the list and the search (FEAT-087).
///
/// `reviewThreads` is what the Review screen's inbox counts: threads open and
/// resolved, and which open ones somebody answered after you started them. The
/// first and the last comment of each is enough to tell — the whole thread is
/// read only when the pull request is opened.
const FIELDS: &str = r#"
fragment Row on PullRequest {
  id
  number
  title
  body
  isDraft
  updatedAt
  mergeable
  changedFiles
  additions
  deletions
  headRefName
  headRefOid
  baseRefName
  reviewDecision
  author { login }
  repository { nameWithOwner }
  reviewRequests(first: 20) {
    nodes { requestedReviewer { ... on User { login } } }
  }
  commits(last: 1) {
    nodes { commit { statusCheckRollup { state } } }
  }
  reviewThreads(first: 50) {
    nodes {
      isResolved
      first: comments(first: 1) { nodes { author { login } } }
      last: comments(last: 1) { nodes { author { login } } }
    }
  }
}
"#;

/// Everything the Pull requests screen shows, in one query.
const QUERY: &str = r#"
query($owner: String!, $name: String!, $limit: Int!) {
  repository(owner: $owner, name: $name) {
    pullRequests(states: OPEN, first: $limit, orderBy: {field: UPDATED_AT, direction: DESC}) {
      nodes { ...Row }
    }
  }
}
"#;

/// Open pull requests anywhere on the host that involve the token's owner
/// (FEAT-087). Two searches in one request: asked to review, and involved
/// some other way — commented, mentioned, reviewed before.
const INVOLVED: &str = r#"
query($requested: String!, $involved: String!, $limit: Int!) {
  requested: search(query: $requested, type: ISSUE, first: $limit) {
    nodes { ...Row }
  }
  involved: search(query: $involved, type: ISSUE, first: $limit) {
    nodes { ...Row }
  }
}
"#;

/// The open pull requests for `repo`, as `me` sees them.
pub fn pull_requests(repo: &Repo, token: &str, me: &str) -> Result<Vec<PullRequest>> {
    let body = serde_json::json!({
        "query": format!("{QUERY}{FIELDS}"),
        "variables": { "owner": repo.owner, "name": repo.name, "limit": LIMIT },
    })
    .to_string();

    let response = http::post_json(&graphql_url(&repo.host), token, &repo.host, &body)?;

    if response.status < 200 || response.status >= 300 {
        return Err(status_error(
            &repo.host,
            response.status,
            &response.body,
            response.retry_after.as_deref(),
        ));
    }

    read_pull_requests(&response.body, me, &repo.host)
}

/// The open pull requests on `host` that involve `me`, in any repository
/// (FEAT-087).
pub fn involved_pull_requests(host: &str, token: &str, me: &str) -> Result<Vec<PullRequest>> {
    if me.is_empty() {
        return Err(Error::Forge {
            host: host.to_string(),
            detail: "no account is connected for this host".into(),
        });
    }

    let body = serde_json::json!({
        "query": format!("{INVOLVED}{FIELDS}"),
        "variables": {
            "requested": format!("is:pr is:open archived:false review-requested:{me}"),
            "involved": format!("is:pr is:open archived:false involves:{me} -author:{me}"),
            "limit": LIMIT,
        },
    })
    .to_string();

    let response = http::post_json(&graphql_url(host), token, host, &body)?;

    if response.status < 200 || response.status >= 300 {
        return Err(status_error(
            host,
            response.status,
            &response.body,
            response.retry_after.as_deref(),
        ));
    }

    read_involved(&response.body, me, host)
}

/// Turn the two searches into one list, each pull request once. Makes no
/// request.
///
/// A search answers with any issue-shaped node; one that is not a pull request
/// carries none of these fields and is skipped by [`row`].
pub fn read_involved(body: &str, me: &str, host: &str) -> Result<Vec<PullRequest>> {
    let json: Value = serde_json::from_str(body).map_err(|_| Error::Forge {
        host: host.to_string(),
        detail: "sent something that is not JSON".into(),
    })?;

    if let Some(message) = graphql_error(&json) {
        return Err(Error::Forge {
            host: host.to_string(),
            detail: message,
        });
    }

    let mut rows: Vec<PullRequest> = Vec::new();
    for search in ["requested", "involved"] {
        let nodes = json["data"][search]["nodes"]
            .as_array()
            .ok_or_else(|| Error::Forge {
                host: host.to_string(),
                detail: "answered without a list of pull requests".into(),
            })?;
        for node in nodes {
            let Some(mut found) = row(node, me) else {
                continue;
            };
            found.repository = node["repository"]["nameWithOwner"]
                .as_str()
                .map(str::to_string);
            if !rows.iter().any(|held| held.id == found.id) {
                rows.push(found);
            }
        }
    }

    rows.sort_by_key(|row| std::cmp::Reverse(row.updated));
    Ok(rows)
}

/// Who a token belongs to.
///
/// Asked when an account is connected: it proves the token works and gets the
/// login in one request, rather than asking a person to type a name that would
/// then be theirs to get wrong.
pub fn whoami(host: &str, token: &str) -> Result<String> {
    let body = serde_json::json!({ "query": "query { viewer { login } }" }).to_string();
    let response = http::post_json(&graphql_url(host), token, host, &body)?;

    if response.status < 200 || response.status >= 300 {
        return Err(status_error(
            host,
            response.status,
            &response.body,
            response.retry_after.as_deref(),
        ));
    }

    let json: Value = serde_json::from_str(&response.body).map_err(|_| Error::Forge {
        host: host.to_string(),
        detail: "sent something that is not JSON".into(),
    })?;

    if let Some(message) = graphql_error(&json) {
        return Err(Error::Forge {
            host: host.to_string(),
            detail: message,
        });
    }

    json["data"]["viewer"]["login"]
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| Error::Forge {
            host: host.to_string(),
            detail: "did not say who the token belongs to".into(),
        })
}

/// Move a pull request in or out of draft (FEAT-071).
///
/// GitHub's REST update accepts a `draft` field and silently does nothing with
/// it — converting is a mutation, and it is addressed by the pull request's
/// node id rather than by its number. The id is already on every row (see
/// [`row`]), so nothing extra has to be fetched to send this.
pub fn set_draft(host: &str, token: &str, id: &str, draft: bool) -> Result<()> {
    if id.is_empty() {
        return Err(Error::Forge {
            host: host.to_string(),
            detail: "no identifier for that pull request".into(),
        });
    }

    let response = http::post_json(&graphql_url(host), token, host, &draft_mutation(id, draft))?;

    if response.status < 200 || response.status >= 300 {
        return Err(status_error(
            host,
            response.status,
            &response.body,
            response.retry_after.as_deref(),
        ));
    }

    let json: Value = serde_json::from_str(&response.body).map_err(|_| Error::Forge {
        host: host.to_string(),
        detail: "sent something that is not JSON".into(),
    })?;

    // A refused mutation comes back as a 200 with an `errors` array, exactly
    // as a refused query does.
    if let Some(message) = graphql_error(&json) {
        return Err(Error::Forge {
            host: host.to_string(),
            detail: message,
        });
    }

    Ok(())
}

/// The mutation body that converts a pull request, one way or the other.
///
/// Two mutations rather than one with a flag, because that is what GitHub
/// offers. Built separately from the sending so the choice is testable.
fn draft_mutation(id: &str, draft: bool) -> String {
    let mutation = if draft {
        "mutation($id: ID!) { convertPullRequestToDraft(input: { pullRequestId: $id }) { clientMutationId } }"
    } else {
        "mutation($id: ID!) { markPullRequestReadyForReview(input: { pullRequestId: $id }) { clientMutationId } }"
    };

    serde_json::json!({ "query": mutation, "variables": { "id": id } }).to_string()
}

/// Turn a GraphQL answer into rows. Makes no request.
pub fn read_pull_requests(body: &str, me: &str, host: &str) -> Result<Vec<PullRequest>> {
    let json: Value = serde_json::from_str(body).map_err(|_| Error::Forge {
        host: host.to_string(),
        detail: "sent something that is not JSON".into(),
    })?;

    // GraphQL reports its own failures with a 200 and an `errors` array, so a
    // successful status is not a successful answer and this has to be looked
    // at before the data is.
    if let Some(message) = graphql_error(&json) {
        return Err(Error::Forge {
            host: host.to_string(),
            detail: message,
        });
    }

    let nodes = json["data"]["repository"]["pullRequests"]["nodes"]
        .as_array()
        .ok_or_else(|| Error::Forge {
            host: host.to_string(),
            detail: "answered without a list of pull requests".into(),
        })?;

    Ok(nodes.iter().filter_map(|node| row(node, me)).collect())
}

/// The first GraphQL error message, if the answer carries any.
fn graphql_error(json: &Value) -> Option<String> {
    let errors = json["errors"].as_array()?;
    let first = errors.first()?;
    Some(
        first["message"]
            .as_str()
            .unwrap_or("refused the query")
            .to_string(),
    )
}

/// One node, or `None` when it is missing something a row cannot be built
/// without.
///
/// Skipped rather than defaulted. A pull request with no number is not a pull
/// request, and inventing a zero would put a row on screen that no link opens.
fn row(node: &Value, me: &str) -> Option<PullRequest> {
    let number = node["number"].as_u64()?;

    // `author` is null for a deleted account, which is a real thing to find in
    // a long-lived repository. The row is still worth showing.
    let author = node["author"]["login"].as_str().unwrap_or("");
    let review = review_of(node["reviewDecision"].as_str());
    let reviewers = requested_reviewers(node);
    let needs = needs_you(me, author, review, &reviewers);
    let threads = threads_of(node, me);

    Some(PullRequest {
        head_sha: node["headRefOid"].as_str().unwrap_or("").to_string(),
        review_requested: !me.is_empty() && reviewers.iter().any(|login| login == me),
        open_threads: threads.open,
        resolved_threads: threads.resolved,
        replies_to_you: threads.replies_to_you,
        repository: None,
        id: node["id"].as_str().unwrap_or("").to_string(),
        number,
        title: node["title"].as_str().unwrap_or("").to_string(),
        body: node["body"].as_str().unwrap_or("").to_string(),
        author_name: if author.is_empty() {
            "a deleted account".into()
        } else {
            author.to_string()
        },
        updated: timestamp(node["updatedAt"].as_str()),
        source_branch: node["headRefName"].as_str().unwrap_or("").to_string(),
        target_branch: node["baseRefName"].as_str().unwrap_or("").to_string(),
        draft: node["isDraft"].as_bool().unwrap_or(false),
        review,
        checks: checks_of(node),
        needs_you: needs.is_some(),
        needs_you_because: needs,
        changed_files: node["changedFiles"].as_u64().unwrap_or(0),
        added: node["additions"].as_u64().unwrap_or(0),
        removed: node["deletions"].as_u64().unwrap_or(0),
        mergeable: mergeable_of(node["mergeable"].as_str()),
    })
}

/// What one pull request's review threads add up to, for `me`.
#[derive(Debug, Default, PartialEq, Eq)]
struct Threads {
    open: u32,
    resolved: u32,
    replies_to_you: u32,
}

/// Count the threads, and the open ones `me` started where somebody else has
/// spoken since (FEAT-087).
///
/// Only the first and the last comment are read, which is enough: a thread
/// you started whose last word is yours is waiting on the author, and one
/// whose last word is theirs is waiting on you.
fn threads_of(node: &Value, me: &str) -> Threads {
    let mut threads = Threads::default();
    let Some(nodes) = node["reviewThreads"]["nodes"].as_array() else {
        return threads;
    };

    let login = |thread: &Value, which: &str| {
        thread[which]["nodes"][0]["author"]["login"]
            .as_str()
            .unwrap_or("")
            .to_string()
    };

    for thread in nodes {
        if thread["isResolved"].as_bool().unwrap_or(false) {
            threads.resolved += 1;
            continue;
        }
        threads.open += 1;
        if !me.is_empty() && login(thread, "first") == me && login(thread, "last") != me {
            threads.replies_to_you += 1;
        }
    }

    threads
}

/// GitHub's review decision, in the screen's words.
///
/// `null` means nobody has been asked and nobody has ruled — which the screen
/// calls "no reviewers", and which is a different state from "waiting for a
/// review that was requested".
fn review_of(decision: Option<&str>) -> ReviewState {
    match decision {
        Some("APPROVED") => ReviewState::Approved,
        Some("CHANGES_REQUESTED") => ReviewState::ChangesRequested,
        Some("REVIEW_REQUIRED") => ReviewState::AwaitingReview,
        _ => ReviewState::NoReviewers,
    }
}

/// `mergeable` is a tri-state: GitHub answers `UNKNOWN` while it is still
/// working the merge out, and that is *not* the same as "no".
fn mergeable_of(value: Option<&str>) -> Option<bool> {
    match value {
        Some("MERGEABLE") => Some(true),
        Some("CONFLICTING") => Some(false),
        _ => None,
    }
}

/// The rolled-up state of the checks on the last commit.
///
/// `None` when the repository runs none — which the screen shows as nothing at
/// all, rather than as a check that has not passed.
fn checks_of(node: &Value) -> Option<CheckState> {
    let state = node["commits"]["nodes"]
        .as_array()?
        .first()?
        .get("commit")?
        .get("statusCheckRollup")?
        .get("state")?
        .as_str()?;

    match state {
        "SUCCESS" => Some(CheckState::Passing),
        // `ERROR` is a check that fell over rather than one that failed, and
        // both mean the same thing to somebody deciding whether to merge.
        "FAILURE" | "ERROR" => Some(CheckState::Failing),
        "PENDING" | "EXPECTED" => Some(CheckState::Running),
        _ => None,
    }
}

fn requested_reviewers(node: &Value) -> Vec<String> {
    node["reviewRequests"]["nodes"]
        .as_array()
        .map(|nodes| {
            nodes
                .iter()
                .filter_map(|request| request["requestedReviewer"]["login"].as_str())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

/// Is this one waiting on the person using Spagitty, and why?
///
/// The ordering the whole screen is built around, so the reason is carried
/// rather than recomputed: the row says *why* it is at the top, and a row that
/// could not say why should not be at the top.
///
/// Two ways it can be yours. Somebody asked you to review it — the plainest
/// case. Or it is yours and a reviewer has asked for changes, which is a queue
/// people genuinely lose things in.
fn needs_you(me: &str, author: &str, review: ReviewState, reviewers: &[String]) -> Option<String> {
    if me.is_empty() {
        return None;
    }

    if reviewers.iter().any(|login| login == me) {
        return Some("your review was requested".into());
    }

    if author == me && review == ReviewState::ChangesRequested {
        return Some("changes were requested on your pull request".into());
    }

    None
}

/// An ISO-8601 instant as unix seconds.
///
/// Hand-parsed rather than through a date library: the only format GitHub sends
/// is `YYYY-MM-DDTHH:MM:SSZ`, always UTC, and this crate has no date dependency
/// to spend on reading one field. Anything that does not match is 0, which the
/// screen renders as an unknown time rather than as a wrong one.
pub(crate) fn timestamp(iso: Option<&str>) -> i64 {
    let Some(text) = iso else { return 0 };
    let bytes = text.as_bytes();
    if bytes.len() < 20 || bytes[4] != b'-' || bytes[10] != b'T' {
        return 0;
    }

    let number = |from: usize, to: usize| text[from..to].parse::<i64>().ok();
    let (Some(year), Some(month), Some(day)) = (number(0, 4), number(5, 7), number(8, 10)) else {
        return 0;
    };
    let (Some(hour), Some(minute), Some(second)) = (number(11, 13), number(14, 16), number(17, 19))
    else {
        return 0;
    };

    days_from_civil(year, month, day) * 86_400 + hour * 3_600 + minute * 60 + second
}

/// Days since 1970-01-01 for a proleptic Gregorian date.
///
/// Howard Hinnant's `days_from_civil`, which is the reference algorithm for
/// this and is exact for every date a repository can carry. Written out rather
/// than depended on: one function against a whole date crate, for one field.
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let year_of_era = year - era * 400;
    let day_of_year = (153 * (if month > 2 { month - 3 } else { month + 9 }) + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

/// Create a pull request on GitHub via REST API (FEAT-070).
pub fn create_pull_request(
    repo: &Repo,
    token: &str,
    title: &str,
    body: &str,
    head: &str,
    base: &str,
    draft: bool,
) -> Result<PullRequest> {
    let url = format!(
        "{}/repos/{}/{}/pulls",
        repo.kind.api_base(&repo.host),
        repo.owner,
        repo.name
    );
    let payload = serde_json::json!({
        "title": title,
        "body": body,
        "head": head,
        "base": base,
        "draft": draft,
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

    let json: Value = serde_json::from_str(&response.body).map_err(|e| Error::Forge {
        host: repo.host.clone(),
        detail: format!("JSON parsing error: {e}"),
    })?;
    let number = json.get("number").and_then(Value::as_u64).unwrap_or(0);
    let id = json
        .get("node_id")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let title = json
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let body = json
        .get("body")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let author_name = json
        .get("user")
        .and_then(|u| u.get("login"))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let updated = timestamp(json.get("updated_at").and_then(Value::as_str));
    let source_branch = json
        .get("head")
        .and_then(|h| h.get("ref"))
        .and_then(Value::as_str)
        .unwrap_or(head)
        .to_string();
    let target_branch = json
        .get("base")
        .and_then(|b| b.get("ref"))
        .and_then(Value::as_str)
        .unwrap_or(base)
        .to_string();
    let draft = json.get("draft").and_then(Value::as_bool).unwrap_or(draft);

    Ok(PullRequest {
        id,
        number,
        title,
        body,
        author_name,
        updated,
        source_branch,
        target_branch,
        draft,
        review: ReviewState::NoReviewers,
        checks: None,
        needs_you: false,
        needs_you_because: None,
        changed_files: json
            .get("changed_files")
            .and_then(Value::as_u64)
            .unwrap_or(0),
        added: json.get("additions").and_then(Value::as_u64).unwrap_or(0),
        removed: json.get("deletions").and_then(Value::as_u64).unwrap_or(0),
        mergeable: json.get("mergeable").and_then(Value::as_bool),
        head_sha: json["head"]["sha"].as_str().unwrap_or_default().to_string(),
        ..PullRequest::default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One node with everything present, which each test then bends.
    fn node() -> Value {
        serde_json::json!({
            "id": "PR_kwDO",
            "number": 412,
            "title": "Give the graph a footer",
            "body": "PR description markdown",
            "isDraft": false,
            "updatedAt": "2026-08-25T09:30:00Z",
            "mergeable": "MERGEABLE",
            "changedFiles": 7,
            "additions": 120,
            "deletions": 34,
            "headRefName": "feature/footer",
            "baseRefName": "main",
            "reviewDecision": "REVIEW_REQUIRED",
            "author": { "login": "grace" },
            "reviewRequests": { "nodes": [{ "requestedReviewer": { "login": "ada" } }] },
            "commits": { "nodes": [{ "commit": { "statusCheckRollup": { "state": "SUCCESS" } } }] }
        })
    }

    fn answer(nodes: Value) -> String {
        serde_json::json!({ "data": { "repository": { "pullRequests": { "nodes": nodes } } } })
            .to_string()
    }

    fn one(node: Value, me: &str) -> PullRequest {
        let rows = read_pull_requests(&answer(serde_json::json!([node])), me, "github.com")
            .expect("a list");
        rows.into_iter().next().expect("a row")
    }

    #[test]
    fn a_pull_request_arrives_in_the_shape_the_screen_renders() {
        let row = one(node(), "ada");

        assert_eq!(row.number, 412);
        assert_eq!(row.title, "Give the graph a footer");
        assert_eq!(row.author_name, "grace");
        assert_eq!(row.source_branch, "feature/footer");
        assert_eq!(row.target_branch, "main");
        assert_eq!(row.changed_files, 7);
        assert_eq!(row.added, 120);
        assert_eq!(row.removed, 34);
        assert!(!row.draft);
        assert_eq!(row.mergeable, Some(true));
    }

    // FEAT-087 — what the Review screen's inbox reads off the same row.

    fn thread(resolved: bool, first: &str, last: &str) -> Value {
        serde_json::json!({
            "isResolved": resolved,
            "first": { "nodes": [{ "author": { "login": first } }] },
            "last": { "nodes": [{ "author": { "login": last } }] }
        })
    }

    #[test]
    fn the_head_commit_and_a_review_request_are_read_off_the_row() {
        let mut bent = node();
        bent["headRefOid"] = serde_json::json!("a1b2c3");

        let mine = one(bent.clone(), "ada");
        assert_eq!(mine.head_sha, "a1b2c3");
        assert!(mine.review_requested);

        let theirs = one(bent, "grace");
        assert!(!theirs.review_requested);
    }

    #[test]
    fn threads_are_counted_open_and_resolved_and_answered_ones_are_yours() {
        let mut bent = node();
        bent["reviewThreads"] = serde_json::json!({ "nodes": [
            // You asked, the author answered: back with you.
            thread(false, "ada", "grace"),
            // You asked and nobody has answered yet.
            thread(false, "ada", "ada"),
            // Somebody else's thread.
            thread(false, "linus", "grace"),
            // Answered, and settled.
            thread(true, "ada", "grace"),
        ]});

        let row = one(bent, "ada");
        assert_eq!(row.open_threads, 3);
        assert_eq!(row.resolved_threads, 1);
        assert_eq!(row.replies_to_you, 1);
    }

    #[test]
    fn a_row_with_no_threads_says_none_rather_than_failing() {
        let row = one(node(), "ada");
        assert_eq!((row.open_threads, row.resolved_threads), (0, 0));
        assert_eq!(row.replies_to_you, 0);
        assert_eq!(row.repository, None);
    }

    #[test]
    fn the_two_searches_become_one_list_with_each_pull_request_once() {
        let mut first = node();
        first["repository"] = serde_json::json!({ "nameWithOwner": "team/one" });
        first["updatedAt"] = serde_json::json!("2026-08-25T09:30:00Z");
        let mut second = node();
        second["id"] = serde_json::json!("PR_other");
        second["number"] = serde_json::json!(7);
        second["repository"] = serde_json::json!({ "nameWithOwner": "team/two" });
        second["updatedAt"] = serde_json::json!("2026-08-26T09:30:00Z");

        let body = serde_json::json!({ "data": {
            "requested": { "nodes": [first.clone()] },
            // The same pull request again, and an issue the search also found.
            "involved": { "nodes": [first, second, { "title": "an issue" }] }
        }})
        .to_string();

        let rows = read_involved(&body, "ada", "github.com").expect("a list");
        assert_eq!(rows.len(), 2);
        // Newest first, whichever search found it.
        assert_eq!(rows[0].repository.as_deref(), Some("team/two"));
        assert_eq!(rows[1].repository.as_deref(), Some("team/one"));
    }

    #[test]
    fn searching_with_no_login_is_refused_before_anything_is_sent() {
        assert!(involved_pull_requests("github.com", "token", "").is_err());
    }

    #[test]
    fn every_review_decision_github_can_send_has_a_word_the_screen_knows() {
        assert_eq!(review_of(Some("APPROVED")), ReviewState::Approved);
        assert_eq!(
            review_of(Some("CHANGES_REQUESTED")),
            ReviewState::ChangesRequested
        );
        assert_eq!(
            review_of(Some("REVIEW_REQUIRED")),
            ReviewState::AwaitingReview
        );
        // Nobody asked and nobody ruled, which is not the same as waiting.
        assert_eq!(review_of(None), ReviewState::NoReviewers);
        assert_eq!(review_of(Some("SOMETHING_NEW")), ReviewState::NoReviewers);
    }

    #[test]
    fn a_repository_that_runs_no_checks_shows_no_checks() {
        // Null, not failing. "This has not passed" about a repository with no
        // CI would be an accusation the host never made.
        let mut node = node();
        node["commits"]["nodes"][0]["commit"]["statusCheckRollup"] = Value::Null;

        assert_eq!(one(node, "ada").checks, None);
    }

    #[test]
    fn a_check_that_fell_over_reads_the_same_as_one_that_failed() {
        // ERROR and FAILURE are different to GitHub and the same to somebody
        // deciding whether to merge.
        for state in ["FAILURE", "ERROR"] {
            let mut node = node();
            node["commits"]["nodes"][0]["commit"]["statusCheckRollup"]["state"] =
                Value::String(state.into());
            assert_eq!(
                one(node, "ada").checks,
                Some(CheckState::Failing),
                "for {state}"
            );
        }
    }

    #[test]
    fn a_check_still_running_says_so() {
        for state in ["PENDING", "EXPECTED"] {
            let mut node = node();
            node["commits"]["nodes"][0]["commit"]["statusCheckRollup"]["state"] =
                Value::String(state.into());
            assert_eq!(
                one(node, "ada").checks,
                Some(CheckState::Running),
                "for {state}"
            );
        }
    }

    #[test]
    fn a_merge_github_has_not_worked_out_yet_is_not_a_merge_it_refused() {
        // UNKNOWN while it computes. Rendering that as "cannot merge" would be
        // wrong for a few seconds every time somebody pushes.
        assert_eq!(mergeable_of(Some("UNKNOWN")), None);
        assert_eq!(mergeable_of(None), None);
        assert_eq!(mergeable_of(Some("CONFLICTING")), Some(false));
        assert_eq!(mergeable_of(Some("MERGEABLE")), Some(true));
    }

    #[test]
    fn a_review_requested_from_you_puts_it_at_the_top_and_says_why() {
        let row = one(node(), "ada");

        assert!(row.needs_you);
        assert_eq!(
            row.needs_you_because.as_deref(),
            Some("your review was requested")
        );
    }

    #[test]
    fn changes_requested_on_your_own_pull_request_is_yours_too() {
        // The queue people actually lose things in.
        let mut node = node();
        node["author"]["login"] = Value::String("ada".into());
        node["reviewDecision"] = Value::String("CHANGES_REQUESTED".into());
        node["reviewRequests"]["nodes"] = serde_json::json!([]);

        let row = one(node, "ada");

        assert!(row.needs_you);
        assert!(row
            .needs_you_because
            .unwrap()
            .contains("changes were requested"));
    }

    #[test]
    fn your_own_pull_request_waiting_on_somebody_else_is_not_yours() {
        let mut node = node();
        node["author"]["login"] = Value::String("ada".into());
        node["reviewRequests"]["nodes"] = serde_json::json!([]);

        let row = one(node, "ada");

        assert!(!row.needs_you);
        assert_eq!(row.needs_you_because, None);
    }

    #[test]
    fn nothing_needs_you_when_nobody_knows_who_you_are() {
        // No login means no way to tell, and a screen that put everything at
        // the top would be a screen with no ordering at all.
        let row = one(node(), "");

        assert!(!row.needs_you);
    }

    #[test]
    fn a_deleted_author_still_gets_a_row() {
        // A real thing to find in a long-lived repository, and the pull request
        // is still open and still worth showing.
        let mut node = node();
        node["author"] = Value::Null;

        let row = one(node, "ada");

        assert_eq!(row.author_name, "a deleted account");
    }

    #[test]
    fn a_node_with_no_number_is_skipped_rather_than_invented() {
        // A row with a zero for a number is a row no link opens.
        let mut node = node();
        node["number"] = Value::Null;

        let rows =
            read_pull_requests(&answer(serde_json::json!([node])), "ada", "github.com").unwrap();

        assert!(rows.is_empty());
    }

    #[test]
    fn an_empty_list_is_an_empty_list_and_not_an_error() {
        let rows = read_pull_requests(&answer(serde_json::json!([])), "ada", "github.com").unwrap();

        assert!(rows.is_empty());
    }

    #[test]
    fn a_graphql_error_is_reported_even_though_it_arrived_with_a_200() {
        // GraphQL reports its own failures in the body with a successful
        // status, so a 200 is not an answer until the errors are looked at.
        let body = serde_json::json!({
            "data": null,
            "errors": [{ "message": "Could not resolve to a Repository with the name 'x/y'." }]
        })
        .to_string();

        match read_pull_requests(&body, "ada", "github.com") {
            Err(Error::Forge { detail, .. }) => assert!(detail.contains("Could not resolve")),
            other => panic!("expected a forge error, got {other:?}"),
        }
    }

    #[test]
    fn a_body_that_is_not_json_is_reported_rather_than_panicked_on() {
        let html = read_pull_requests("<html>maintenance</html>", "ada", "github.com");

        assert!(matches!(html, Err(Error::Forge { .. })));
    }

    #[test]
    fn an_answer_with_no_list_in_it_says_so() {
        let empty = read_pull_requests(r#"{"data":{}}"#, "ada", "github.com");

        assert!(matches!(empty, Err(Error::Forge { .. })));
    }

    #[test]
    fn the_time_is_read_without_a_date_dependency() {
        // 2026-08-25T09:30:00Z. Checked against a value computed elsewhere,
        // because a hand-written calendar is exactly the thing to get wrong.
        assert_eq!(timestamp(Some("2026-08-25T09:30:00Z")), 1_787_650_200);
        assert_eq!(timestamp(Some("1970-01-01T00:00:00Z")), 0);
        // A leap day, and the year 2000, which is the leap-century case.
        assert_eq!(timestamp(Some("2000-02-29T00:00:00Z")), 951_782_400);
    }

    #[test]
    fn a_time_that_does_not_parse_is_unknown_rather_than_wrong() {
        for bad in [None, Some(""), Some("yesterday"), Some("2026-08-25")] {
            assert_eq!(timestamp(bad), 0, "for {bad:?}");
        }
    }

    #[test]
    fn an_enterprise_installation_is_asked_at_its_own_graphql_path() {
        // `/api/graphql`, not `/api/v3/graphql` — v3 is the REST root.
        assert_eq!(graphql_url("github.com"), "https://api.github.com/graphql");
        assert_eq!(
            graphql_url("github.example.com"),
            "https://github.example.com/api/graphql"
        );
    }

    #[test]
    fn the_query_asks_for_everything_the_row_needs_and_nothing_else() {
        // The argument for GraphQL is that one request fills the whole row. If
        // a field stops being asked for, the row silently defaults instead.
        for field in [
            "number",
            "title",
            "isDraft",
            "updatedAt",
            "mergeable",
            "changedFiles",
            "additions",
            "deletions",
            "headRefName",
            "baseRefName",
            "reviewDecision",
            "statusCheckRollup",
            "headRefOid",
            "reviewThreads",
        ] {
            assert!(
                FIELDS.contains(field),
                "the query no longer asks for {field}"
            );
        }

        // Both reads spell the fields once, through the fragment.
        assert!(QUERY.contains("...Row"));
        assert!(INVOLVED.contains("...Row"));

        // Read-only, by decision. A mutation here would be a write nobody
        // reviewed.
        for query in [QUERY, INVOLVED, FIELDS] {
            assert!(!query.contains("mutation"));
        }
    }

    #[test]
    fn converting_to_draft_and_back_are_two_different_mutations() {
        // GitHub offers no flag: draft and ready are separate mutations, and
        // sending the wrong one moves the pull request the wrong way.
        let to_draft = draft_mutation("PR_kwDO", true);
        let to_ready = draft_mutation("PR_kwDO", false);

        assert!(to_draft.contains("convertPullRequestToDraft"));
        assert!(!to_draft.contains("markPullRequestReadyForReview"));
        assert!(to_ready.contains("markPullRequestReadyForReview"));
        assert!(!to_ready.contains("convertPullRequestToDraft"));
    }

    #[test]
    fn the_draft_mutation_addresses_the_pull_request_by_node_id() {
        // Its number is not an `ID!`. A mutation given one is refused with a
        // type error rather than doing anything.
        let body: Value = serde_json::from_str(&draft_mutation("PR_kwDO", true))
            .expect("the mutation body is JSON");

        assert_eq!(body["variables"]["id"], "PR_kwDO");
        assert!(body["query"]
            .as_str()
            .unwrap()
            .contains("pullRequestId: $id"));
    }

    #[test]
    fn a_pull_request_with_no_node_id_is_refused_before_anything_is_sent() {
        // `row` defaults a missing id to an empty string rather than dropping
        // the row, so this is reachable — and an empty `ID!` is a 422 that
        // says nothing useful.
        let refused = set_draft("github.com", "token", "", true);

        match refused {
            Err(Error::Forge { detail, .. }) => assert!(detail.contains("no identifier")),
            other => panic!("expected a refusal, got {other:?}"),
        }
    }
}
