// SPDX-License-Identifier: GPL-3.0-or-later

//! GitLab merge requests and API integration (FEAT-070, FEAT-088).
//!
//! # Written against the spec
//!
//! `docs/reference/gitlab-merge-requests-api.pdf` is the author's reference
//! for GitLab's REST API v4, and every request here is one of its endpoints:
//!
//! | What | Endpoint |
//! | --- | --- |
//! | the list | `GET /projects/:id/merge_requests`, `GET /merge_requests` |
//! | files | `GET …/merge_requests/:iid/diffs` (`/changes` on servers older than 15.7) |
//! | commits | `GET …/merge_requests/:iid/commits` |
//! | threads | `GET …/merge_requests/:iid/discussions` |
//! | checks | `GET …/merge_requests/:iid/pipelines` |
//! | one commit's files | `GET /projects/:id/repository/commits/:sha/diff` |
//!
//! `:id` is the project's whole path URL-encoded ([`Repo::encoded_slug`]), so
//! a project in a nested group is addressed correctly. The token goes as
//! `Authorization: Bearer`, which the spec lists beside `PRIVATE-TOKEN`.
//!
//! # Nothing is invented
//!
//! A merge request in GitLab's list carries no pipeline status, no line
//! counts and no thread counts. The row says *none* for each rather than
//! guessing; the Review inbox asks for the checks and the threads separately
//! ([`review_summaries`]), a few merge requests at a time.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use serde_json::Value;

use crate::diff::{FileDiff, FileStatus, LineOrigin};
use crate::error::{Error, Result};
use crate::forge::github::timestamp;
use crate::forge::review::{
    parse_patch, DraftComment, LineKind, LinePlace, PullRequestComment, PullRequestCommit,
    ReviewVerdict,
};
use crate::forge::{http, status_error, CheckState, PullRequest, Repo, ReviewState, ReviewSummary};

/// Items per page, and the most GitLab answers with.
const PER_PAGE: usize = 100;

/// Pages read before a list is called long enough — a ceiling so a host
/// answering strangely cannot spin a loop, not an expected size.
const MAX_PAGES: usize = 10;

/// How many merge requests are asked about at once by [`review_summaries`].
const WORKERS: usize = 6;

pub fn whoami(host: &str, token: &str) -> Result<String> {
    let url = format!("{}/user", crate::forge::Kind::GitLab.api_base(host));
    let json = get(&url, token, host)?;
    json.get("username")
        .and_then(Value::as_str)
        .map(String::from)
        .ok_or_else(|| Error::Forge {
            host: host.to_string(),
            detail: "GitLab response had no username".into(),
        })
}

/// `{api}/projects/{encoded path}`, which every project call hangs off.
fn project_url(repo: &Repo) -> String {
    format!(
        "{}/projects/{}",
        repo.kind.api_base(&repo.host),
        repo.encoded_slug()
    )
}

/// `{project}/merge_requests/{iid}`.
fn merge_request_url(repo: &Repo, number: u64) -> String {
    format!("{}/merge_requests/{number}", project_url(repo))
}

/// `GET url` as JSON, a refused status as the error that says which.
fn get(url: &str, token: &str, host: &str) -> Result<Value> {
    let response = http::get_json(url, token, host)?;
    if response.status < 200 || response.status >= 300 {
        return Err(status_error(
            host,
            response.status,
            &response.body,
            response.retry_after.as_deref(),
        ));
    }
    serde_json::from_str(&response.body).map_err(|e| json_err(host, e))
}

/// Every page of a list endpoint, until a short page.
fn get_all(url: &str, token: &str, host: &str) -> Result<Vec<Value>> {
    get_all_or_missing(url, token, host)?.ok_or_else(|| status_error(host, 404, "", None))
}

/// [`get_all`], or `None` when the endpoint itself answers 404 — how an
/// older server says it does not have it.
fn get_all_or_missing(url: &str, token: &str, host: &str) -> Result<Option<Vec<Value>>> {
    let join = if url.contains('?') { '&' } else { '?' };
    let mut items = Vec::new();
    for page in 1..=MAX_PAGES {
        let response = http::get_json(
            &format!("{url}{join}per_page={PER_PAGE}&page={page}"),
            token,
            host,
        )?;
        if response.status == 404 && page == 1 {
            return Ok(None);
        }
        if response.status < 200 || response.status >= 300 {
            return Err(status_error(
                host,
                response.status,
                &response.body,
                response.retry_after.as_deref(),
            ));
        }
        let json: Value = serde_json::from_str(&response.body).map_err(|e| json_err(host, e))?;
        let Some(batch) = json.as_array() else {
            return Err(Error::Forge {
                host: host.to_string(),
                detail: "did not send a list".into(),
            });
        };
        let full = batch.len() == PER_PAGE;
        items.extend(batch.iter().cloned());
        if !full {
            break;
        }
    }
    Ok(Some(items))
}

pub fn pull_requests(repo: &Repo, token: &str, me: &str) -> Result<Vec<PullRequest>> {
    let url = format!(
        "{}/merge_requests?state=opened&per_page=50&order_by=updated_at&sort=desc",
        project_url(repo)
    );
    Ok(parse_merge_requests(&get(&url, token, &repo.host)?, me))
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
        "{}/merge_requests?scope=all&state=opened&per_page=50&order_by=updated_at&sort=desc&reviewer_username={}",
        crate::forge::Kind::GitLab.api_base(host),
        crate::forge::encode_segment(me),
    );
    Ok(parse_merge_requests(&get(&url, token, host)?, me))
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
    let url = format!("{}/merge_requests", project_url(repo));

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

/// `changes_count` is a string — `"5"`, or `"1000+"` past GitLab's ceiling —
/// and a number on some older servers. Either way, the digits it starts with.
fn count_of(value: &Value) -> u64 {
    match value {
        Value::Number(number) => number.as_u64().unwrap_or(0),
        Value::String(text) => text
            .chars()
            .take_while(char::is_ascii_digit)
            .collect::<String>()
            .parse()
            .unwrap_or(0),
        _ => 0,
    }
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
        .get("draft")
        .or_else(|| node.get("work_in_progress"))
        .and_then(Value::as_bool)
        .unwrap_or(false)
        || title.starts_with("Draft:")
        || title.starts_with("WIP:");

    let mergeable = node
        .get("has_conflicts")
        .and_then(Value::as_bool)
        .map(|conflicts| !conflicts);

    // Usernames are case-insensitive on GitLab.
    let review_requested = !me.is_empty()
        && node
            .get("reviewers")
            .and_then(Value::as_array)
            .is_some_and(|reviewers| {
                reviewers.iter().any(|reviewer| {
                    reviewer["username"]
                        .as_str()
                        .is_some_and(|name| name.eq_ignore_ascii_case(me))
                })
            });

    // It needs you when you are one of its reviewers (FEAT-088). It used to
    // need you whenever it was not yours, which put every merge request in
    // the project at the top of the list.
    let needs_you = review_requested;

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
        review: if node["reviewers"].as_array().is_some_and(|r| !r.is_empty()) {
            ReviewState::AwaitingReview
        } else {
            ReviewState::NoReviewers
        },
        // The list carries no pipeline. None, not "passing" (FEAT-088): a
        // merge request whose pipeline failed read as green.
        checks: None,
        needs_you,
        needs_you_because: needs_you.then(|| "your review was requested".to_string()),
        changed_files: node.get("changes_count").map(count_of).unwrap_or(0),
        added: 0,
        removed: 0,
        mergeable,
        head_sha: node
            .get("sha")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        review_requested,
        // `references.full` is `group/project!42`; the part before the `!`
        // names the project. Only a search across projects needs it.
        repository: node["references"]["full"]
            .as_str()
            .and_then(|full| full.rsplit_once('!'))
            .map(|(project, _)| project.to_string()),
        ..PullRequest::default()
    })
}

// --- A merge request's contents (FEAT-088) ---------------------------------

/// The files a merge request changes, with GitLab's diff of each.
///
/// `/diffs` is paged and is the spec's endpoint; a server older than 15.7
/// answers it 404, and `/changes` — one object with a `changes` list — is
/// what it has instead.
pub fn merge_request_files(repo: &Repo, token: &str, number: u64) -> Result<Vec<FileDiff>> {
    let base = merge_request_url(repo, number);
    if let Some(entries) = get_all_or_missing(&format!("{base}/diffs"), token, &repo.host)? {
        return Ok(entries.iter().filter_map(file_of).collect());
    }
    let json = get(&format!("{base}/changes"), token, &repo.host)?;
    Ok(json["changes"]
        .as_array()
        .map(|entries| entries.iter().filter_map(file_of).collect())
        .unwrap_or_default())
}

/// One file of a GitLab diff list, in the shape the screens render.
pub fn file_of(entry: &Value) -> Option<FileDiff> {
    let deleted = entry["deleted_file"].as_bool().unwrap_or(false);
    let path = if deleted {
        entry["old_path"].as_str()?
    } else {
        entry["new_path"].as_str()?
    };
    let diff = entry["diff"].as_str().unwrap_or("");
    let too_large = entry["too_large"].as_bool().unwrap_or(false)
        || entry["collapsed"].as_bool().unwrap_or(false) && diff.is_empty();
    let binary = diff.starts_with("Binary files") || diff.contains("\nBinary files");
    let hunks = if binary {
        Vec::new()
    } else {
        parse_patch(diff)
    };

    let count = |origin: LineOrigin| {
        hunks
            .iter()
            .flat_map(|hunk| &hunk.lines)
            .filter(|line| line.origin == origin)
            .count() as u32
    };

    Some(FileDiff {
        path: path.to_string(),
        status: if entry["new_file"].as_bool().unwrap_or(false) {
            FileStatus::Added
        } else if deleted {
            FileStatus::Deleted
        } else if entry["renamed_file"].as_bool().unwrap_or(false) {
            FileStatus::Renamed
        } else {
            FileStatus::Modified
        },
        binary,
        too_large,
        added: count(LineOrigin::Added),
        removed: count(LineOrigin::Removed),
        hunks,
    })
}

/// The commits in one merge request.
pub fn merge_request_commits(
    repo: &Repo,
    token: &str,
    number: u64,
) -> Result<Vec<PullRequestCommit>> {
    let url = format!("{}/commits", merge_request_url(repo, number));
    Ok(get_all(&url, token, &repo.host)?
        .iter()
        .filter_map(commit_of)
        .collect())
}

fn commit_of(entry: &Value) -> Option<PullRequestCommit> {
    let sha = entry["id"].as_str()?.to_string();
    Some(PullRequestCommit {
        short: entry["short_id"]
            .as_str()
            .map(str::to_string)
            .unwrap_or_else(|| sha.chars().take(7).collect()),
        summary: entry["title"].as_str().unwrap_or("").to_string(),
        author_name: entry["author_name"].as_str().unwrap_or("").to_string(),
        author_email: entry["author_email"].as_str().unwrap_or("").to_string(),
        time: timestamp(entry["authored_date"].as_str()),
        sha,
    })
}

/// The files one commit changed.
pub fn commit_files(repo: &Repo, token: &str, sha: &str) -> Result<Vec<FileDiff>> {
    let url = format!(
        "{}/repository/commits/{}/diff",
        project_url(repo),
        crate::forge::encode_segment(sha)
    );
    Ok(get_all(&url, token, &repo.host)?
        .iter()
        .filter_map(file_of)
        .collect())
}

/// A merge request's discussions, as GitLab sends them.
pub fn discussions(repo: &Repo, token: &str, number: u64) -> Result<Vec<Value>> {
    let url = format!("{}/discussions", merge_request_url(repo, number));
    get_all(&url, token, &repo.host)
}

/// The line comments of a merge request, flattened the way the Pull requests
/// screen reads them: each reply points at the first note of its thread.
pub fn merge_request_comments(
    repo: &Repo,
    token: &str,
    number: u64,
) -> Result<Vec<PullRequestComment>> {
    Ok(comments_of(&discussions(repo, token, number)?))
}

/// Every comment the Review room shows (FEAT-093): the line comments and the
/// notes on the merge request as a whole, each with its discussion's id.
pub fn review_comments(repo: &Repo, token: &str, number: u64) -> Result<Vec<PullRequestComment>> {
    Ok(notes_of(&discussions(repo, token, number)?, true))
}

/// Discussions as line comments. Makes no request.
pub fn comments_of(discussions: &[Value]) -> Vec<PullRequestComment> {
    notes_of(discussions, false)
}

/// Discussions as comments: those on a line, and with `general` those on the
/// merge request as a whole, with an empty path. System notes — "added 2
/// commits" — are left out. Makes no request.
fn notes_of(discussions: &[Value], general: bool) -> Vec<PullRequestComment> {
    let mut comments = Vec::new();
    for discussion in discussions {
        let Some(notes) = discussion["notes"].as_array() else {
            continue;
        };
        let notes: Vec<&Value> = notes
            .iter()
            .filter(|note| !note["system"].as_bool().unwrap_or(false))
            .collect();
        let Some(first) = notes.first() else { continue };
        let position = &first["position"];
        let path = match position["new_path"]
            .as_str()
            .or_else(|| position["old_path"].as_str())
        {
            Some(path) => path,
            None if general => "",
            None => continue,
        };
        let thread_id = discussion["id"].as_str().map(str::to_string);
        let (line, side) = match (position["new_line"].as_u64(), position["old_line"].as_u64()) {
            (Some(new), _) => (Some(new as u32), "RIGHT"),
            (None, Some(old)) => (Some(old as u32), "LEFT"),
            (None, None) => (None, "RIGHT"),
        };
        let root = first["id"].as_u64();
        let resolved = thread_resolved(&notes);

        for note in &notes {
            let Some(id) = note["id"].as_u64() else {
                continue;
            };
            comments.push(PullRequestComment {
                id,
                in_reply_to: if Some(id) == root { None } else { root },
                path: path.to_string(),
                line,
                side: side.to_string(),
                body: note["body"].as_str().unwrap_or("").to_string(),
                author: note["author"]["username"]
                    .as_str()
                    .unwrap_or("")
                    .to_string(),
                created_at: timestamp(note["created_at"].as_str()),
                resolved,
                thread_id: thread_id.clone(),
            });
        }
    }
    comments
}

/// Resolve a discussion, or open it again (FEAT-093): the spec's
/// `PUT …/discussions/:discussion_id` with `resolved`.
pub fn resolve_discussion(
    repo: &Repo,
    token: &str,
    number: u64,
    discussion: &str,
    resolved: bool,
) -> Result<()> {
    let url = format!(
        "{}/discussions/{}",
        merge_request_url(repo, number),
        crate::forge::encode_segment(discussion)
    );
    let body = serde_json::json!({ "resolved": resolved }).to_string();
    let response = http::put_json(&url, token, &repo.host, &body)?;
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

/// A thread is resolved when every note that can be resolved is.
fn thread_resolved(notes: &[&Value]) -> bool {
    let resolvable: Vec<&&Value> = notes
        .iter()
        .filter(|note| note["resolvable"].as_bool().unwrap_or(false))
        .collect();
    !resolvable.is_empty()
        && resolvable
            .iter()
            .all(|note| note["resolved"].as_bool().unwrap_or(false))
}

// --- What the Review inbox asks after the list (FEAT-088) ------------------

/// Checks and threads for each merge request in `numbers`.
///
/// Two requests each — the latest pipeline and the discussions — run a few
/// merge requests at a time. A merge request whose answers fail is left out
/// rather than failing the rest: the inbox shows what it learnt.
pub fn review_summaries(repo: &Repo, token: &str, me: &str, numbers: &[u64]) -> Vec<ReviewSummary> {
    let next = AtomicUsize::new(0);
    let found = Mutex::new(Vec::new());

    std::thread::scope(|scope| {
        for _ in 0..WORKERS.min(numbers.len()) {
            scope.spawn(|| loop {
                let index = next.fetch_add(1, Ordering::Relaxed);
                let Some(&number) = numbers.get(index) else {
                    break;
                };
                if let Some(summary) = summary_of(repo, token, me, number) {
                    found.lock().expect("summaries").push(summary);
                }
            });
        }
    });

    let mut summaries = found.into_inner().expect("summaries");
    summaries.sort_by_key(|summary| summary.number);
    summaries
}

fn summary_of(repo: &Repo, token: &str, me: &str, number: u64) -> Option<ReviewSummary> {
    let threads = discussions(repo, token, number).ok()?;
    // A project without CI, or a token that cannot read pipelines, answers
    // with nothing or a refusal; either way there are no checks to show.
    let pipelines = get(
        &format!("{}/pipelines", merge_request_url(repo, number)),
        token,
        &repo.host,
    )
    .ok();

    let mut summary = summarise(&threads, me);
    summary.number = number;
    summary.checks = pipelines.as_ref().and_then(checks_of);
    Some(summary)
}

/// Threads counted the way GitHub's are (FEAT-087). Makes no request.
///
/// A thread is a discussion that is not a lone comment and has something to
/// resolve. One you started where somebody else spoke last, still open, is a
/// reply to you.
pub fn summarise(discussions: &[Value], me: &str) -> ReviewSummary {
    let mut summary = ReviewSummary::default();
    for discussion in discussions {
        if discussion["individual_note"].as_bool().unwrap_or(false) {
            continue;
        }
        let Some(notes) = discussion["notes"].as_array() else {
            continue;
        };
        let notes: Vec<&Value> = notes
            .iter()
            .filter(|note| !note["system"].as_bool().unwrap_or(false))
            .collect();
        if !notes
            .iter()
            .any(|note| note["resolvable"].as_bool().unwrap_or(false))
        {
            continue;
        }
        if thread_resolved(&notes) {
            summary.resolved_threads += 1;
            continue;
        }
        summary.open_threads += 1;
        let author = |note: Option<&&Value>| {
            note.and_then(|note| note["author"]["username"].as_str())
                .unwrap_or("")
                .to_string()
        };
        let (first, last) = (author(notes.first()), author(notes.last()));
        if !me.is_empty() && first.eq_ignore_ascii_case(me) && !last.eq_ignore_ascii_case(me) {
            summary.replies_to_you += 1;
        }
    }
    summary
}

/// The latest pipeline's status, in the screen's words. Newest first is how
/// GitLab lists them.
pub fn checks_of(pipelines: &Value) -> Option<CheckState> {
    match pipelines.as_array()?.first()?["status"].as_str()? {
        "success" => Some(CheckState::Passing),
        "failed" => Some(CheckState::Failing),
        "created"
        | "waiting_for_resource"
        | "preparing"
        | "pending"
        | "running"
        | "scheduled"
        | "manual" => Some(CheckState::Running),
        // Canceled or skipped: no verdict either way.
        _ => None,
    }
}

// --- Answering a merge request (FEAT-088) -----------------------------------

/// `POST url` with a JSON body, a refused status as the error that says which.
fn post(url: &str, token: &str, host: &str, body: &Value) -> Result<Value> {
    let response = http::post_json(url, token, host, &body.to_string())?;
    if response.status < 200 || response.status >= 300 {
        return Err(status_error(
            host,
            response.status,
            &response.body,
            response.retry_after.as_deref(),
        ));
    }
    Ok(serde_json::from_str(&response.body).unwrap_or(Value::Null))
}

/// Reply to the thread a note belongs to.
///
/// The Pull requests screen knows a comment by its note id; GitLab replies to
/// a discussion, so the discussion holding that note is found first.
pub fn reply(
    repo: &Repo,
    token: &str,
    number: u64,
    note_id: u64,
    body: &str,
) -> Result<PullRequestComment> {
    let threads = discussions(repo, token, number)?;
    let thread = threads
        .iter()
        .find(|discussion| {
            discussion["notes"].as_array().is_some_and(|notes| {
                notes
                    .iter()
                    .any(|note| note["id"].as_u64() == Some(note_id))
            })
        })
        .ok_or_else(|| Error::Forge {
            host: repo.host.clone(),
            detail: "that thread is no longer on the merge request".into(),
        })?;
    let id = thread["id"].as_str().unwrap_or_default();

    let note = post(
        &format!(
            "{}/discussions/{}/notes",
            merge_request_url(repo, number),
            crate::forge::encode_segment(id)
        ),
        token,
        &repo.host,
        &serde_json::json!({ "body": body }),
    )?;

    // The answer is the note alone; its place is the thread's first note's.
    let first = thread["notes"][0].clone();
    notes_of(&[serde_json::json!({ "id": thread["id"], "notes": [first, note] })], true)
        .pop()
        .ok_or_else(|| Error::Forge {
            host: repo.host.clone(),
            detail: "failed to parse comment from response".into(),
        })
}

/// The three commits a comment's position is pinned to: the latest diff
/// version's base, start and head (`GET …/versions`, newest first).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffRefs {
    pub base: String,
    pub start: String,
    pub head: String,
}

pub fn diff_refs(repo: &Repo, token: &str, number: u64) -> Result<DiffRefs> {
    let json = get(
        &format!("{}/versions", merge_request_url(repo, number)),
        token,
        &repo.host,
    )?;
    refs_of(&json).ok_or_else(|| Error::Forge {
        host: repo.host.clone(),
        detail: "the merge request has no diff version to comment on".into(),
    })
}

fn refs_of(versions: &Value) -> Option<DiffRefs> {
    let latest = versions.as_array()?.first()?;
    Some(DiffRefs {
        base: latest["base_commit_sha"].as_str()?.to_string(),
        start: latest["start_commit_sha"].as_str()?.to_string(),
        head: latest["head_commit_sha"].as_str()?.to_string(),
    })
}

/// GitLab's `line_code`: the path's SHA-1 and the line's two counters.
fn line_code(path: &str, place: LinePlace) -> String {
    let mut hasher = gix::hash::hasher(gix::hash::Kind::Sha1);
    hasher.update(path.as_bytes());
    let digest = hasher
        .try_finalize()
        .map(|id| id.to_hex().to_string())
        .unwrap_or_default();
    format!("{digest}_{}_{}", place.old, place.new)
}

/// Where a pending comment goes: the position object of a draft note.
///
/// An added line is placed by its new number alone, a removed one by its old
/// number alone, and an unchanged line by both — GitLab reads a position with
/// both as an unchanged line. A comment over several lines also carries its
/// `line_range`, each end by its line code.
pub fn position(refs: &DiffRefs, draft: &DraftComment) -> Value {
    let path = draft.path.as_str();
    let mut position = serde_json::json!({
        "position_type": "text",
        "base_sha": refs.base,
        "start_sha": refs.start,
        "head_sha": refs.head,
        "new_path": path,
        "old_path": draft.old_path.as_deref().unwrap_or(path),
    });

    match draft.place {
        Some(place) => {
            if place.kind != LineKind::Removed {
                position["new_line"] = serde_json::json!(place.new);
            }
            if place.kind != LineKind::Added {
                position["old_line"] = serde_json::json!(place.old);
            }
        }
        // An older caller said only which side. Right enough for an added or
        // removed line.
        None if draft.side == "LEFT" => position["old_line"] = serde_json::json!(draft.line),
        None => position["new_line"] = serde_json::json!(draft.line),
    }

    if let (Some(start), Some(end)) = (draft.start_place, draft.place) {
        let end_of = |place: LinePlace| {
            serde_json::json!({
                "line_code": line_code(path, place),
                "type": match place.kind {
                    LineKind::Added => Value::from("new"),
                    LineKind::Removed => Value::from("old"),
                    LineKind::Context => Value::Null,
                },
                "old_line": (place.kind != LineKind::Added).then_some(place.old),
                "new_line": (place.kind != LineKind::Removed).then_some(place.new),
            })
        };
        position["line_range"] = serde_json::json!({ "start": end_of(start), "end": end_of(end) });
    }

    position
}

/// Leave a review: every pending comment as a draft note, the summary as one
/// more, all of them published at once, then the verdict (FEAT-088).
///
/// The spec's batch review: `POST …/draft_notes` each, then
/// `POST …/draft_notes/bulk_publish`. Approving is `POST …/approve` with the
/// head `sha`, so an approval cannot land on a version nobody read. GitLab has
/// no "request changes" over this API; asking for changes publishes the
/// comments and withdraws your approval if you had given one.
///
/// A failure before publishing deletes the drafts this call made, so trying
/// again does not publish them twice — `bulk_publish` sends every draft you
/// hold on the merge request.
pub fn submit_review(
    repo: &Repo,
    token: &str,
    number: u64,
    verdict: ReviewVerdict,
    comment: &str,
    drafts: &[DraftComment],
) -> Result<()> {
    let base = merge_request_url(repo, number);
    let refs = diff_refs(repo, token, number)?;
    let mut made: Vec<u64> = Vec::new();

    let sent = (|| -> Result<()> {
        for draft in drafts {
            let note = post(
                &format!("{base}/draft_notes"),
                token,
                &repo.host,
                &serde_json::json!({ "note": draft.body, "position": position(&refs, draft) }),
            )?;
            made.extend(note["id"].as_u64());
        }
        if !comment.trim().is_empty() {
            let note = post(
                &format!("{base}/draft_notes"),
                token,
                &repo.host,
                &serde_json::json!({ "note": comment }),
            )?;
            made.extend(note["id"].as_u64());
        }
        if !made.is_empty() {
            post(
                &format!("{base}/draft_notes/bulk_publish"),
                token,
                &repo.host,
                &serde_json::json!({}),
            )?;
        }
        Ok(())
    })();

    if let Err(error) = sent {
        for id in made {
            let _ = http::delete(&format!("{base}/draft_notes/{id}"), token, &repo.host);
        }
        return Err(error);
    }

    match verdict {
        ReviewVerdict::Approve => {
            post(
                &format!("{base}/approve"),
                token,
                &repo.host,
                &serde_json::json!({ "sha": refs.head }),
            )?;
        }
        ReviewVerdict::RequestChanges => {
            // Not having approved is not an error here.
            let _ = post(
                &format!("{base}/unapprove"),
                token,
                &repo.host,
                &serde_json::json!({}),
            );
        }
        ReviewVerdict::Comment => {}
    }
    Ok(())
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
    use crate::forge::Kind;

    fn repo() -> Repo {
        Repo {
            kind: Kind::GitLab,
            host: "gitlab.example.com".into(),
            owner: "team/backend".into(),
            name: "payments".into(),
        }
    }

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
                "changes_count": "5",
                "reviewers": [{ "username": "Reviewer1" }]
            }
        ]);

        let list = parse_merge_requests(&raw, "reviewer1");
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].number, 42);
        assert_eq!(list[0].author_name, "developer1");
        assert!(list[0].draft);
        assert!(list[0].needs_you);
        assert_eq!(list[0].source_branch, "feature/x");
        assert_eq!(list[0].changed_files, 5);
        assert_eq!(list[0].review, ReviewState::AwaitingReview);
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

    // FEAT-088 — as the spec says.

    #[test]
    fn a_merge_request_needs_you_only_when_you_are_a_reviewer() {
        // It used to need you whenever it was not yours.
        let raw = serde_json::json!([{
            "id": 1, "iid": 7, "title": "Someone else's",
            "author": { "username": "nour" }, "reviewers": []
        }]);
        let row = &parse_merge_requests(&raw, "me")[0];
        assert!(!row.needs_you);
        assert_eq!(row.needs_you_because, None);
        assert_eq!(row.review, ReviewState::NoReviewers);
    }

    #[test]
    fn the_list_claims_no_checks_it_was_not_sent() {
        let raw = serde_json::json!([{ "id": 1, "iid": 7, "title": "x" }]);
        assert_eq!(parse_merge_requests(&raw, "me")[0].checks, None);
    }

    #[test]
    fn a_change_count_past_gitlabs_ceiling_keeps_its_digits() {
        assert_eq!(count_of(&serde_json::json!("1000+")), 1000);
        assert_eq!(count_of(&serde_json::json!(12)), 12);
        assert_eq!(count_of(&serde_json::json!(null)), 0);
    }

    #[test]
    fn every_project_call_addresses_the_whole_path_encoded() {
        assert_eq!(
            merge_request_url(&repo(), 42),
            "https://gitlab.example.com/api/v4/projects/team%2Fbackend%2Fpayments/merge_requests/42"
        );
    }

    #[test]
    fn a_diff_entry_becomes_a_file_with_its_counts() {
        let entry = serde_json::json!({
            "old_path": "src/a.rs", "new_path": "src/a.rs",
            "new_file": false, "renamed_file": false, "deleted_file": false,
            "diff": "@@ -1,2 +1,3 @@\n a\n-b\n+c\n+d\n"
        });
        let file = file_of(&entry).expect("a file");
        assert_eq!(file.path, "src/a.rs");
        assert_eq!(file.status, FileStatus::Modified);
        assert_eq!((file.added, file.removed), (2, 1));
        assert_eq!(file.hunks.len(), 1);
    }

    #[test]
    fn added_deleted_renamed_and_binary_files_say_so() {
        let of = |json: Value| file_of(&json).expect("a file");

        let added = of(
            serde_json::json!({"old_path": "n", "new_path": "n", "new_file": true, "diff": "@@ -0,0 +1 @@\n+x\n"}),
        );
        assert_eq!(added.status, FileStatus::Added);

        let deleted = of(
            serde_json::json!({"old_path": "gone", "new_path": "gone", "deleted_file": true, "diff": "@@ -1 +0,0 @@\n-x\n"}),
        );
        assert_eq!(deleted.status, FileStatus::Deleted);
        assert_eq!(deleted.path, "gone");

        let renamed = of(
            serde_json::json!({"old_path": "a", "new_path": "b", "renamed_file": true, "diff": ""}),
        );
        assert_eq!(renamed.status, FileStatus::Renamed);
        assert_eq!(renamed.path, "b");

        let binary = of(
            serde_json::json!({"old_path": "i.png", "new_path": "i.png", "diff": "Binary files a/i.png and b/i.png differ\n"}),
        );
        assert!(binary.binary);
        assert!(binary.hunks.is_empty());

        let large = of(
            serde_json::json!({"old_path": "x", "new_path": "x", "too_large": true, "diff": ""}),
        );
        assert!(large.too_large);
    }

    #[test]
    fn a_commit_reads_its_short_id_title_and_author() {
        let commit = commit_of(&serde_json::json!({
            "id": "abcdef1234", "short_id": "abcdef1", "title": "Fix the lock",
            "author_name": "Nour", "author_email": "n@example.com",
            "authored_date": "2026-08-25T09:30:00.000Z"
        }))
        .expect("a commit");
        assert_eq!(commit.short, "abcdef1");
        assert_eq!(commit.summary, "Fix the lock");
        assert_eq!(commit.time, 1_787_650_200);
    }

    fn note(id: u64, who: &str, resolvable: bool, resolved: bool) -> Value {
        serde_json::json!({
            "id": id, "body": format!("note {id}"), "author": { "username": who },
            "created_at": "2026-08-25T09:30:00.000Z", "system": false,
            "resolvable": resolvable, "resolved": resolved,
            "position": { "new_path": "src/a.rs", "old_path": "src/a.rs", "new_line": 20, "old_line": null }
        })
    }

    fn discussion(individual: bool, notes: Vec<Value>) -> Value {
        serde_json::json!({ "id": "d1", "individual_note": individual, "notes": notes })
    }

    #[test]
    fn discussions_become_line_comments_that_point_at_their_first_note() {
        let comments = comments_of(&[discussion(
            false,
            vec![note(1, "me", true, false), note(2, "nour", true, false)],
        )]);
        assert_eq!(comments.len(), 2);
        assert_eq!(comments[0].in_reply_to, None);
        assert_eq!(comments[1].in_reply_to, Some(1));
        assert_eq!(comments[0].path, "src/a.rs");
        assert_eq!(comments[0].line, Some(20));
        assert_eq!(comments[0].side, "RIGHT");
        assert!(!comments[0].resolved);
    }

    #[test]
    fn threads_are_counted_and_a_reply_to_you_found_without_system_notes() {
        let mut system = note(9, "nour", false, false);
        system["system"] = serde_json::json!(true);
        let summary = summarise(
            &[
                // Yours, answered, open: a reply to you.
                discussion(
                    false,
                    vec![
                        note(1, "Me", true, false),
                        note(2, "nour", true, false),
                        system,
                    ],
                ),
                // Yours, waiting on the author.
                discussion(false, vec![note(3, "me", true, false)]),
                // Settled.
                discussion(
                    false,
                    vec![note(4, "nour", true, true), note(5, "me", true, true)],
                ),
                // A plain comment is not a thread.
                discussion(true, vec![note(6, "nour", false, false)]),
            ],
            "me",
        );
        assert_eq!(summary.open_threads, 2);
        assert_eq!(summary.resolved_threads, 1);
        assert_eq!(summary.replies_to_you, 1);
    }

    #[test]
    fn the_latest_pipeline_is_the_merge_requests_checks() {
        let of = |status: &str| {
            checks_of(&serde_json::json!([{ "status": status }, { "status": "success" }]))
        };
        assert_eq!(of("failed"), Some(CheckState::Failing));
        assert_eq!(of("success"), Some(CheckState::Passing));
        assert_eq!(of("running"), Some(CheckState::Running));
        assert_eq!(of("manual"), Some(CheckState::Running));
        assert_eq!(of("canceled"), None);
        assert_eq!(checks_of(&serde_json::json!([])), None);
    }

    fn refs() -> DiffRefs {
        DiffRefs {
            base: "b".into(),
            start: "s".into(),
            head: "h".into(),
        }
    }

    fn draft(place: Option<LinePlace>, start: Option<LinePlace>) -> DraftComment {
        DraftComment {
            path: "src/a.rs".into(),
            line: 20,
            side: "RIGHT".into(),
            body: "Write to a temp file?".into(),
            place,
            start_place: start,
            ..DraftComment::default()
        }
    }

    #[test]
    fn the_latest_version_pins_a_comments_position() {
        let versions = serde_json::json!([
            { "base_commit_sha": "b2", "start_commit_sha": "s2", "head_commit_sha": "h2" },
            { "base_commit_sha": "b1", "start_commit_sha": "s1", "head_commit_sha": "h1" }
        ]);
        assert_eq!(
            refs_of(&versions),
            Some(DiffRefs {
                base: "b2".into(),
                start: "s2".into(),
                head: "h2".into()
            })
        );
        assert_eq!(refs_of(&serde_json::json!([])), None);
    }

    #[test]
    fn an_added_line_is_placed_by_its_new_number_alone() {
        let place = LinePlace {
            kind: LineKind::Added,
            old: 18,
            new: 20,
        };
        let position = position(&refs(), &draft(Some(place), None));
        assert_eq!(position["position_type"], "text");
        assert_eq!(
            (position["base_sha"].as_str(), position["head_sha"].as_str()),
            (Some("b"), Some("h"))
        );
        assert_eq!(position["new_line"], 20);
        assert!(position.get("old_line").is_none());
        assert_eq!(position["new_path"], "src/a.rs");
        assert_eq!(position["old_path"], "src/a.rs");
    }

    #[test]
    fn a_removed_line_by_its_old_number_and_an_unchanged_one_by_both() {
        let removed = position(
            &refs(),
            &draft(
                Some(LinePlace {
                    kind: LineKind::Removed,
                    old: 15,
                    new: 15,
                }),
                None,
            ),
        );
        assert_eq!(removed["old_line"], 15);
        assert!(removed.get("new_line").is_none());

        let context = position(
            &refs(),
            &draft(
                Some(LinePlace {
                    kind: LineKind::Context,
                    old: 17,
                    new: 18,
                }),
                None,
            ),
        );
        assert_eq!(
            (context["old_line"].as_u64(), context["new_line"].as_u64()),
            (Some(17), Some(18))
        );
    }

    #[test]
    fn a_comment_over_several_lines_carries_its_range_by_line_code() {
        let start = LinePlace {
            kind: LineKind::Context,
            old: 17,
            new: 18,
        };
        let end = LinePlace {
            kind: LineKind::Added,
            old: 19,
            new: 20,
        };
        let position = position(&refs(), &draft(Some(end), Some(start)));
        let range = &position["line_range"];

        // `printf %s src/a.rs | sha1sum`, then the two counters.
        assert_eq!(
            range["start"]["line_code"],
            "c46b6f3386ba91fc8fcb5f960fd24012ff3c1460_17_18"
        );
        assert_eq!(range["start"]["type"], Value::Null);
        assert_eq!(range["end"]["type"], "new");
        assert!(range["end"]["line_code"]
            .as_str()
            .unwrap()
            .ends_with("_19_20"));
        assert_eq!(range["end"]["old_line"], Value::Null);
        assert_eq!(range["end"]["new_line"], 20);
    }

    #[test]
    fn a_line_code_is_the_paths_sha1_and_both_counters() {
        // `printf %s src/a.rs | sha1sum`
        let place = LinePlace {
            kind: LineKind::Context,
            old: 1,
            new: 2,
        };
        assert_eq!(
            line_code("src/a.rs", place),
            "c46b6f3386ba91fc8fcb5f960fd24012ff3c1460_1_2"
        );
    }

    #[test]
    fn an_older_caller_that_says_only_a_side_still_places_its_comment() {
        let mut left = draft(None, None);
        left.side = "LEFT".into();
        assert_eq!(position(&refs(), &left)["old_line"], 20);
        assert_eq!(position(&refs(), &draft(None, None))["new_line"], 20);
    }

    #[test]
    fn no_merge_requests_asks_nothing() {
        assert!(review_summaries(&repo(), "token", "me", &[]).is_empty());
    }

    #[test]
    fn the_review_room_reads_notes_on_the_whole_merge_request_too() {
        let discussions: Vec<Value> = serde_json::from_str(
            r#"[
              {"id":"d1","notes":[{"id":1,"body":"On a line","author":{"username":"nour"},"created_at":"2026-10-04T10:00:00Z",
                "resolvable":true,"resolved":true,
                "position":{"new_path":"src/a.rs","old_path":"src/a.rs","new_line":20,"old_line":null}}]},
              {"id":"d2","individual_note":true,"notes":[{"id":2,"body":"Should the cache have a cap?","author":{"username":"nour"},
                "created_at":"2026-10-04T10:05:00Z","resolvable":false}]},
              {"id":"d3","individual_note":true,"notes":[{"id":3,"system":true,"body":"added 2 commits"}]}
            ]"#,
        )
        .unwrap();

        let all = notes_of(&discussions, true);
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].thread_id.as_deref(), Some("d1"));
        assert!(all[0].resolved);
        assert_eq!((all[1].path.as_str(), all[1].line), ("", None));
        assert_eq!(all[1].thread_id.as_deref(), Some("d2"));

        // The Pull requests screen still reads the line comments alone.
        let lines = comments_of(&discussions);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].path, "src/a.rs");
    }
}
