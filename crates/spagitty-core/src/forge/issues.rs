// SPDX-License-Identifier: GPL-3.0-or-later

//! Issues in the host-neutral shape of the screen (FEAT-136). Reads are
//! conditional; writes are explicit and never automatically retried.

use super::{encode_segment as enc, http, status_error, Kind, Repo};
use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Filter {
    pub state: String,
    pub assignee: String,
    pub labels: Vec<String>,
    pub milestone: String,
    pub sort: String,
    pub search: String,
}

pub fn filter(text: &str, me: &str) -> Result<Filter> {
    let mut result = Filter {
        state: "open".into(),
        sort: "updated".into(),
        ..Filter::default()
    };
    let mut tokens = Vec::new();
    let mut token = String::new();
    let mut quoted = false;
    for c in text.chars() {
        match c {
            '"' => quoted = !quoted,
            c if c.is_whitespace() && !quoted => {
                if !token.is_empty() {
                    tokens.push(std::mem::take(&mut token));
                }
            }
            c => token.push(c),
        }
    }
    if quoted {
        return Err(Error::Forge {
            host: String::new(),
            detail: "Close the quote in the issue filter.".into(),
        });
    }
    if !token.is_empty() {
        tokens.push(token);
    }
    let mut search = Vec::new();
    for token in tokens {
        match token.split_once(':') {
            Some(("is", state @ ("open" | "closed" | "all"))) => result.state = state.into(),
            Some(("assignee", value)) => {
                result.assignee = if value == "@me" {
                    me.into()
                } else {
                    value.into()
                }
            }
            Some(("label", value)) if !value.is_empty() => result.labels.push(value.into()),
            Some(("milestone", value)) => result.milestone = value.into(),
            Some(("sort", value @ ("created" | "updated" | "comments"))) => {
                result.sort = value.into()
            }
            _ => search.push(token),
        }
    }
    result.search = search.join(" ");
    Ok(result)
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Label {
    pub name: String,
    pub color: String,
}
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Milestone {
    pub id: String,
    pub title: String,
    pub open: Option<u64>,
    pub closed: Option<u64>,
}
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Issue {
    pub branch_suggestion: String,
    pub id: String,
    pub number: u64,
    pub title: String,
    pub body: String,
    pub state: String,
    pub author: String,
    pub created: String,
    pub updated: String,
    pub url: String,
    pub replies: u64,
    pub labels: Vec<Label>,
    pub assignees: Vec<String>,
    pub milestone: Option<Milestone>,
    pub locked: bool,
}
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IssueComment {
    pub id: String,
    pub author: String,
    pub body: String,
    pub at: String,
    pub url: String,
}
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IssueEvent {
    pub id: String,
    pub kind: String,
    pub author: String,
    pub at: String,
    pub title: String,
    pub url: String,
    pub branch: Option<String>,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Page {
    pub issues: Vec<Issue>,
    pub page: u32,
    pub more: bool,
    pub etag: Option<String>,
    pub can_write: Option<bool>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Thread {
    pub issue: Issue,
    pub comments: Vec<IssueComment>,
    pub events: Vec<IssueEvent>,
    pub comments_more: bool,
    pub events_more: bool,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Metadata {
    pub default_branch: String,
    pub open: u64,
    pub closed: u64,
    pub labels: Vec<Label>,
    pub milestones: Vec<Milestone>,
}

fn failure(repo: &Repo, detail: impl Into<String>) -> Error {
    Error::Forge {
        host: repo.host.clone(),
        detail: detail.into(),
    }
}
fn supported(repo: &Repo) -> Result<()> {
    if repo.kind == Kind::Bitbucket {
        Err(failure(repo, "Bitbucket issues are coming later."))
    } else {
        Ok(())
    }
}
pub fn root(repo: &Repo) -> Result<String> {
    supported(repo)?;
    Ok(match repo.kind {
        Kind::GitHub => format!(
            "{}/repos/{}/{}",
            repo.kind.api_base(&repo.host),
            enc(&repo.owner),
            enc(&repo.name)
        ),
        _ => format!(
            "{}/projects/{}",
            repo.kind.api_base(&repo.host),
            repo.encoded_slug()
        ),
    })
}
fn checked(repo: &Repo, response: http::Response) -> Result<Value> {
    if !(200..300).contains(&response.status) {
        return Err(status_error(
            &repo.host,
            response.status,
            &response.body,
            response.retry_after.as_deref(),
        ));
    }
    serde_json::from_str(&response.body)
        .map_err(|_| failure(repo, "The host sent an unreadable issue response."))
}
fn get(repo: &Repo, token: &str, path: &str) -> Result<Value> {
    checked(
        repo,
        http::get_json(&format!("{}/{path}", root(repo)?), token, &repo.host)?,
    )
}
fn string(value: &Value, field: &str) -> String {
    value[field].as_str().unwrap_or("").into()
}
fn labels(value: &Value) -> Vec<Label> {
    value
        .as_array()
        .into_iter()
        .flatten()
        .map(|label| Label {
            name: label
                .as_str()
                .map(str::to_string)
                .unwrap_or_else(|| string(label, "name")),
            color: label["color"]
                .as_str()
                .map(|color| color.trim_start_matches('#'))
                .filter(|c| c.len() == 6 && c.bytes().all(|b| b.is_ascii_hexdigit()))
                .unwrap_or("")
                .into(),
        })
        .collect()
}
fn milestone(value: &Value, github: bool) -> Option<Milestone> {
    if value.is_null() {
        return None;
    }
    Some(Milestone {
        id: value[if github { "number" } else { "id" }]
            .as_u64()?
            .to_string(),
        title: string(value, "title"),
        open: value["open_issues"].as_u64(),
        closed: value["closed_issues"].as_u64(),
    })
}
pub fn read_issue(value: &Value, kind: Kind) -> Option<Issue> {
    let github = kind == Kind::GitHub;
    if github && value.get("pull_request").is_some() {
        return None;
    }
    let mut issue = Issue {
        branch_suggestion: String::new(),
        id: value["id"].as_u64()?.to_string(),
        number: value[if github { "number" } else { "iid" }].as_u64()?,
        title: string(value, "title"),
        body: string(value, if github { "body" } else { "description" }),
        state: if value["state"] == "closed" {
            "closed"
        } else {
            "open"
        }
        .into(),
        author: string(
            &value[if github { "user" } else { "author" }],
            if github { "login" } else { "username" },
        ),
        created: string(value, "created_at"),
        updated: string(value, "updated_at"),
        url: string(value, if github { "html_url" } else { "web_url" }),
        replies: value[if github {
            "comments"
        } else {
            "user_notes_count"
        }]
        .as_u64()
        .unwrap_or(0),
        labels: labels(&value["labels"]),
        assignees: value["assignees"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|a| {
                a[if github { "login" } else { "username" }]
                    .as_str()
                    .map(str::to_string)
            })
            .collect(),
        milestone: milestone(&value["milestone"], github),
        locked: value["locked"].as_bool().unwrap_or(false)
            || value["discussion_locked"].as_bool().unwrap_or(false),
    };
    issue.branch_suggestion = branch_name(&issue);
    Some(issue)
}

pub fn list_url(repo: &Repo, filter: &Filter, page: u32) -> Result<String> {
    if page == 0 || page > 10000 {
        return Err(failure(repo, "Choose an issue page between 1 and 10000."));
    }
    let root = root(repo)?;
    let state = match filter.state.as_str() {
        "closed" => "closed",
        "all" => "all",
        _ if repo.kind == Kind::GitLab => "opened",
        _ => "open",
    };
    if repo.kind == Kind::GitHub && !filter.search.is_empty() {
        let mut query = format!(
            "repo:{} is:issue \"{}\"",
            repo.slug(),
            filter.search.replace('"', "\\\"")
        );
        if state != "all" {
            query.push_str(&format!(" is:{state}"));
        }
        if !filter.assignee.is_empty() {
            query.push_str(&format!(" assignee:{}", filter.assignee));
        }
        for label in &filter.labels {
            query.push_str(&format!(" label:\"{label}\""));
        }
        if !filter.milestone.is_empty() {
            query.push_str(&format!(" milestone:\"{}\"", filter.milestone));
        }
        return Ok(format!(
            "{}/search/issues?q={}&sort={}&order=desc&per_page=50&page={page}",
            repo.kind.api_base(&repo.host),
            enc(&query),
            enc(&filter.sort)
        ));
    }
    let mut url = format!("{root}/issues?state={state}&per_page=50&page={page}");
    if repo.kind == Kind::GitHub {
        url.push_str(&format!("&sort={}&direction=desc", enc(&filter.sort)));
        if !filter.assignee.is_empty() {
            url.push_str(&format!("&assignee={}", enc(&filter.assignee)));
        }
    } else {
        if filter.sort == "comments" {
            return Err(failure(
                repo,
                "GitLab does not support sorting issues by replies.",
            ));
        }
        let sort = if filter.sort == "created" {
            "created_at"
        } else {
            "updated_at"
        };
        url.push_str(&format!(
            "&scope=all&with_labels_details=true&order_by={sort}&sort=desc"
        ));
        if !filter.assignee.is_empty() {
            url.push_str(&format!("&assignee_username[]={}", enc(&filter.assignee)));
        }
        if !filter.search.is_empty() {
            url.push_str(&format!("&search={}", enc(&filter.search)));
        }
    }
    if !filter.labels.is_empty() {
        url.push_str(&format!("&labels={}", enc(&filter.labels.join(","))));
    }
    if !filter.milestone.is_empty() {
        url.push_str(&format!("&milestone={}", enc(&filter.milestone)));
    }
    Ok(url)
}

pub fn page(
    repo: &Repo,
    token: &str,
    filter: &Filter,
    number: u32,
    cached: Option<&Page>,
) -> Result<Page> {
    page_using(repo, filter, number, cached, |url, etag| {
        http::get_conditional_json(url, token, &repo.host, etag)
    })
}
fn page_using(
    repo: &Repo,
    filter: &Filter,
    number: u32,
    cached: Option<&Page>,
    mut fetch: impl FnMut(&str, Option<&str>) -> Result<http::Conditional>,
) -> Result<Page> {
    let response = fetch(
        &list_url(repo, filter, number)?,
        cached.and_then(|p| p.etag.as_deref()),
    )?;
    let write = response
        .scopes
        .as_ref()
        .map(|scopes| scopes.iter().any(|s| s == "repo" || s == "public_repo"));
    if response.response.status == 304 {
        let mut page = cached.cloned().ok_or_else(|| {
            failure(
                repo,
                "The host returned an unchanged page without a cached answer.",
            )
        })?;
        page.can_write = write;
        if response.etag.is_some() {
            page.etag = response.etag;
        }
        return Ok(page);
    }
    let raw = checked(repo, response.response)?;
    let rows = if raw.is_array() { &raw } else { &raw["items"] };
    let rows = rows
        .as_array()
        .ok_or_else(|| failure(repo, "The host did not send an issue list."))?;
    Ok(Page {
        issues: rows
            .iter()
            .filter_map(|row| read_issue(row, repo.kind))
            .collect(),
        page: number,
        more: response.more || rows.len() == 50,
        etag: response.etag,
        can_write: write,
    })
}

pub fn thread(
    repo: &Repo,
    token: &str,
    number: u64,
    comments_page: u32,
    events_page: u32,
) -> Result<Thread> {
    if number == 0 || comments_page == 0 || events_page == 0 {
        return Err(failure(repo, "Choose an issue and a page."));
    }
    let issue = read_issue(&get(repo, token, &format!("issues/{number}"))?, repo.kind)
        .ok_or_else(|| failure(repo, "This item is not an issue."))?;
    let github = repo.kind == Kind::GitHub;
    let comments = get(
        repo,
        token,
        &format!(
            "issues/{number}/{}?per_page=100&page={comments_page}{}",
            if github { "comments" } else { "notes" },
            if github {
                ""
            } else {
                "&sort=asc&order_by=created_at"
            }
        ),
    )?;
    let events = get(
        repo,
        token,
        &format!(
            "issues/{number}/{}?per_page=100&page={events_page}",
            if github {
                "timeline"
            } else {
                "related_merge_requests"
            }
        ),
    )?;
    read_thread(repo, issue, &comments, &events)
}
fn read_thread(repo: &Repo, issue: Issue, notes: &Value, timeline: &Value) -> Result<Thread> {
    let github = repo.kind == Kind::GitHub;
    let notes = notes
        .as_array()
        .ok_or_else(|| failure(repo, "The host sent an unreadable issue thread."))?;
    let timeline = timeline
        .as_array()
        .ok_or_else(|| failure(repo, "The host sent an unreadable issue timeline."))?;
    let mut comments = Vec::new();
    let mut events = Vec::new();
    for note in notes {
        let id = note["id"]
            .as_u64()
            .map(|id| id.to_string())
            .unwrap_or_default();
        let author = string(
            &note[if github { "user" } else { "author" }],
            if github { "login" } else { "username" },
        );
        if !github && note["system"].as_bool() == Some(true) {
            events.push(IssueEvent {
                id: format!("note:{id}"),
                kind: "system".into(),
                author,
                at: string(note, "created_at"),
                title: string(note, "body"),
                ..IssueEvent::default()
            });
        } else {
            comments.push(IssueComment {
                id,
                author,
                body: string(note, "body"),
                at: string(note, "created_at"),
                url: string(note, if github { "html_url" } else { "url" }),
            });
        }
    }
    for event in timeline {
        if github && event["event"] == "commented" {
            continue;
        }
        let source = if github {
            &event["source"]["issue"]
        } else {
            event
        };
        let kind = if github {
            string(event, "event")
        } else {
            "linked request".into()
        };
        let title = if !source["title"].is_null() {
            format!(
                "#{} {}",
                source[if github { "number" } else { "iid" }]
                    .as_u64()
                    .unwrap_or(0),
                string(source, "title")
            )
        } else if !event["label"].is_null() {
            format!("{kind} {}", string(&event["label"], "name"))
        } else {
            kind.clone()
        };
        events.push(IssueEvent {
            id: event["id"]
                .as_u64()
                .map(|id| format!("event:{id}"))
                .unwrap_or_else(|| format!("event-{}", events.len())),
            kind,
            author: string(
                &event[if github { "actor" } else { "author" }],
                if github { "login" } else { "username" },
            ),
            at: string(event, "created_at"),
            title,
            url: string(source, if github { "html_url" } else { "web_url" }),
            branch: source["source_branch"].as_str().map(str::to_string),
        });
    }
    events.sort_by(|a, b| a.at.cmp(&b.at));
    Ok(Thread {
        issue,
        comments,
        events,
        comments_more: notes.len() == 100,
        events_more: timeline.len() == 100,
    })
}

pub fn metadata(repo: &Repo, token: &str) -> Result<Metadata> {
    let raw = checked(repo, http::get_json(&root(repo)?, token, &repo.host)?)?;
    let label_rows = get(repo, token, "labels?per_page=100")?;
    let milestones = get(repo, token, "milestones?state=all&per_page=100")?;
    let (open, closed) = if repo.kind == Kind::GitHub {
        let count = |state: &str| -> Result<u64> {
            let query = enc(&format!("repo:{} is:issue is:{state}", repo.slug()));
            let raw = checked(
                repo,
                http::get_json(
                    &format!(
                        "{}/search/issues?q={query}&per_page=1",
                        repo.kind.api_base(&repo.host)
                    ),
                    token,
                    &repo.host,
                )?,
            )?;
            Ok(raw["total_count"].as_u64().unwrap_or(0))
        };
        (count("open")?, count("closed")?)
    } else {
        let counts = get(repo, token, "issues_statistics?scope=all")?;
        (
            counts["statistics"]["counts"]["opened"]
                .as_u64()
                .unwrap_or(0),
            counts["statistics"]["counts"]["closed"]
                .as_u64()
                .unwrap_or(0),
        )
    };
    Ok(Metadata {
        default_branch: string(&raw, "default_branch"),
        open,
        closed,
        labels: labels(&label_rows),
        milestones: milestones
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|m| milestone(m, repo.kind == Kind::GitHub))
            .collect(),
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Write {
    Create { title: String, body: String },
    Comment { number: u64, body: String },
    Close { number: u64 },
}
pub fn write(repo: &Repo, token: &str, action: &Write, confirmed: bool) -> Result<Value> {
    write_using(repo, action, confirmed, |method, url, body| match method {
        "PATCH" => http::patch_json(url, token, &repo.host, body),
        "PUT" => http::put_json(url, token, &repo.host, body),
        _ => http::post_json(url, token, &repo.host, body),
    })
}
fn write_using(
    repo: &Repo,
    action: &Write,
    confirmed: bool,
    mut send: impl FnMut(&str, &str, &str) -> Result<http::Response>,
) -> Result<Value> {
    if !confirmed {
        return Err(failure(
            repo,
            "Confirm this issue action before sending it.",
        ));
    }
    let root = root(repo)?;
    let github = repo.kind == Kind::GitHub;
    let (method, path, body) = match action {
        Write::Create { title, body }
            if !title.trim().is_empty() && title.len() <= 256 && body.len() <= 60000 =>
        {
            (
                "POST",
                "issues".to_string(),
                if github {
                    json!({"title":title,"body":body})
                } else {
                    json!({"title":title,"description":body})
                },
            )
        }
        Write::Comment { number, body }
            if *number > 0 && !body.trim().is_empty() && body.len() <= 60000 =>
        {
            (
                "POST",
                format!(
                    "issues/{number}/{}",
                    if github { "comments" } else { "notes" }
                ),
                json!({"body":body}),
            )
        }
        Write::Close { number } if *number > 0 => (
            if github { "PATCH" } else { "PUT" },
            format!("issues/{number}"),
            if github {
                json!({"state":"closed"})
            } else {
                json!({"state_event":"close"})
            },
        ),
        _ => {
            return Err(failure(
                repo,
                "Choose an issue, a title and nonempty content within the host's size limits.",
            ))
        }
    };
    match send(method, &format!("{root}/{path}"), &body.to_string()) {
        Err(Error::ForgeOffline { .. }) => Err(failure(
            repo,
            "The host did not confirm delivery. Refresh before deciding to send again.",
        )),
        other => checked(repo, other?),
    }
}

pub fn branch_name(issue: &Issue) -> String {
    let prefix = if issue
        .labels
        .iter()
        .any(|l| l.name.eq_ignore_ascii_case("bug"))
    {
        "fix"
    } else {
        "feat"
    };
    let mut slug = String::new();
    for c in issue.title.to_lowercase().chars() {
        if c.is_ascii_alphanumeric() {
            slug.push(c);
        } else if !slug.is_empty() && !slug.ends_with('-') {
            slug.push('-');
        }
        if slug.len() >= 60 {
            break;
        }
    }
    let slug = slug.trim_end_matches('-');
    format!(
        "{prefix}/{}-{}",
        issue.number,
        if slug.is_empty() { "issue" } else { slug }
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    fn repo(kind: Kind) -> Repo {
        Repo {
            kind,
            host: "example.org".into(),
            owner: "group/sub".into(),
            name: "app".into(),
        }
    }
    #[test]
    fn filters_keep_quoted_labels_and_map_each_hosts_parameters() {
        let mut filter = filter("is:closed assignee:@me label:\"good first issue\" milestone:12 sort:comments broken button","ada").unwrap();
        assert_eq!(filter.labels, ["good first issue"]);
        assert_eq!(filter.assignee, "ada");
        let github = list_url(&repo(Kind::GitHub), &filter, 2).unwrap();
        assert!(github.contains("is%3Aissue"));
        assert!(github.contains("page=2"));
        assert!(list_url(&repo(Kind::GitLab), &filter, 2).is_err());
        filter.sort = "updated".into();
        let gitlab = list_url(&repo(Kind::GitLab), &filter, 2).unwrap();
        assert!(gitlab.contains("group%2Fsub%2Fapp"));
        assert!(gitlab.contains("assignee_username[]=ada"));
        assert!(super::filter("label:\"unclosed", "ada").is_err());
    }
    #[test]
    fn pull_requests_are_removed_and_unchanged_pages_use_the_saved_answer() {
        let repo = repo(Kind::GitHub);
        let filter = Filter::default();
        let raw =
            json!([{"id":1,"number":1,"title":"issue"},{"id":2,"number":2,"pull_request":{}}]);
        let first = page_using(&repo, &filter, 1, None, |_, _| {
            Ok(http::Conditional {
                response: http::Response {
                    status: 200,
                    body: raw.to_string(),
                    retry_after: None,
                },
                etag: Some("tag".into()),
                more: true,
                scopes: Some(vec!["repo".into()]),
            })
        })
        .unwrap();
        assert_eq!(first.issues.len(), 1);
        assert!(first.more);
        assert_eq!(first.can_write, Some(true));
        let same = page_using(&repo, &filter, 1, Some(&first), |_, etag| {
            assert_eq!(etag, Some("tag"));
            Ok(http::Conditional {
                response: http::Response {
                    status: 304,
                    body: String::new(),
                    retry_after: None,
                },
                etag: None,
                more: false,
                scopes: None,
            })
        })
        .unwrap();
        assert_eq!(same.issues, first.issues);
        assert_eq!(same.can_write, None);
    }
    #[test]
    fn gitlab_system_notes_and_related_requests_are_events() {
        let thread = read_thread(&repo(Kind::GitLab),Issue::default(),&json!([{"id":1,"system":true,"body":"closed","author":{"username":"ada"}},{"id":2,"body":"please fix","author":{"username":"lin"}}]),&json!([{"id":1,"iid":8,"title":"Fix","source_branch":"fix/8"}])).unwrap();
        assert_eq!(thread.comments.len(), 1);
        assert_eq!(thread.events.len(), 2);
        assert_ne!(thread.events[0].id, thread.events[1].id);
        assert_eq!(thread.comments[0].author, "lin");
        assert!(thread
            .events
            .iter()
            .any(|e| e.branch.as_deref() == Some("fix/8")));
    }
    #[test]
    fn writes_require_consent_use_the_correct_verb_and_never_retry() {
        let action = Write::Close { number: 8 };
        let repo = repo(Kind::GitLab);
        assert!(write_using(&repo, &action, false, |_, _, _| panic!("no consent")).is_err());
        let mut calls = 0;
        let result = write_using(&repo, &action, true, |method, path, body| {
            calls += 1;
            assert_eq!(method, "PUT");
            assert!(path.ends_with("issues/8"));
            assert_eq!(
                serde_json::from_str::<Value>(body).unwrap()["state_event"],
                "close"
            );
            Err(Error::ForgeOffline {
                host: String::new(),
                detail: String::new(),
            })
        });
        assert_eq!(calls, 1);
        assert!(result.unwrap_err().to_string().contains("Refresh"));
    }
    #[test]
    fn branch_names_choose_bug_labels_and_are_editable_safe_suggestions() {
        let issue = Issue {
            number: 17,
            title: "Refund / button doesn't respond!".into(),
            labels: vec![Label {
                name: "bug".into(),
                color: String::new(),
            }],
            ..Issue::default()
        };
        assert_eq!(branch_name(&issue), "fix/17-refund-button-doesn-t-respond");
    }
}
