// SPDX-License-Identifier: GPL-3.0-or-later

//! The pull requests worth telling somebody about (FEAT-114).
//!
//! One read per account, on a timer, while Spagitty is open. It answers a
//! narrower question than the Review inbox: not "what is open", but "what
//! could have just happened to me" — so it reads the person's own pull
//! requests *including* merged and closed ones, the ones they were asked to
//! review, and the open ones they are otherwise in.
//!
//! What changed since the last read is decided by the screen, which keeps the
//! last answer. This module only reports the present, in a shape where a
//! change is a comparison of two fields.

use serde::Serialize;
use serde_json::Value;

use crate::forge::{github, gitlab, http, status_error, CheckState, Kind};
use crate::{Error, Result};

/// How many of each kind to read. The newest first, so a pull request that
/// fell off the end has been quiet longer than anything worth announcing.
const LIMIT: usize = 30;

/// Where a pull request stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum WatchState {
    Open,
    Merged,
    Closed,
}

/// One pull request, as the watcher compares it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Watched {
    /// `host/owner/name#number`: the same pull request on every read.
    pub key: String,
    pub host: String,
    /// `owner/name`, or GitLab's full project path.
    pub repository: String,
    pub number: u64,
    pub title: String,
    /// The pull request's own page.
    pub url: String,
    pub state: WatchState,
    /// The person opened it.
    pub mine: bool,
    /// The person is a requested reviewer.
    pub review_requested: bool,
    /// Comments and reviews so far. Only ever compared with itself.
    pub activity: u32,
    /// Who spoke last, when the host says.
    pub last_actor: Option<String>,
    /// The person spoke last. Their own comment is not news to them.
    pub last_actor_is_me: bool,
    /// Null when the host runs no checks, or does not say in a list.
    pub checks: Option<CheckState>,
}

/// The pull requests on `host` that could have news for `me`.
pub fn watch(kind: Kind, host: &str, token: &str, me: &str) -> Result<Vec<Watched>> {
    if me.is_empty() {
        return Err(Error::Forge {
            host: host.to_string(),
            detail: "no account is connected for this host".into(),
        });
    }
    match kind {
        Kind::GitHub => watch_github(host, token, me),
        Kind::GitLab => watch_gitlab(host, token, me),
        // No search across repositories to ask. Nothing to report is not an
        // error: the other accounts still get read.
        Kind::Bitbucket => Ok(Vec::new()),
    }
}

// --- GitHub ----------------------------------------------------------------

const GITHUB_QUERY: &str = r#"
query($mine: String!, $requested: String!, $involved: String!, $limit: Int!) {
  mine: search(query: $mine, type: ISSUE, first: $limit) { nodes { ...Watch } }
  requested: search(query: $requested, type: ISSUE, first: $limit) { nodes { ...Watch } }
  involved: search(query: $involved, type: ISSUE, first: $limit) { nodes { ...Watch } }
}

fragment Watch on PullRequest {
  number
  title
  url
  state
  author { login }
  repository { nameWithOwner }
  reviewRequests(first: 20) {
    nodes { requestedReviewer { ... on User { login } } }
  }
  commits(last: 1) {
    nodes { commit { statusCheckRollup { state } } }
  }
  comments(last: 1) { totalCount nodes { author { login } createdAt } }
  reviews(last: 1) { totalCount nodes { author { login } submittedAt } }
}
"#;

/// What every search adds: live repositories, newest first.
const RECENT: &str = "archived:false sort:updated-desc";

fn watch_github(host: &str, token: &str, me: &str) -> Result<Vec<Watched>> {
    // Merged and closed included for the person's own: that is the news.
    let mine = format!("is:pr author:{me} {RECENT}");
    let requested = format!("is:pr is:open review-requested:{me} {RECENT}");
    let involved = format!("is:pr is:open involves:{me} -author:{me} {RECENT}");
    let body = serde_json::json!({
        "query": GITHUB_QUERY,
        "variables": {
            "mine": mine,
            "requested": requested,
            "involved": involved,
            "limit": LIMIT,
        },
    })
    .to_string();

    let response = http::post_json(&github::graphql_url(host), token, host, &body)?;
    if response.status < 200 || response.status >= 300 {
        return Err(status_error(
            host,
            response.status,
            &response.body,
            response.retry_after.as_deref(),
        ));
    }
    read_github(&response.body, me, host)
}

/// Turn the three searches into one list, each pull request once. Makes no
/// request.
pub fn read_github(body: &str, me: &str, host: &str) -> Result<Vec<Watched>> {
    let json: Value = serde_json::from_str(body).map_err(|_| Error::Forge {
        host: host.to_string(),
        detail: "sent something that is not JSON".into(),
    })?;
    if let Some(message) = github::graphql_error(&json) {
        return Err(Error::Forge {
            host: host.to_string(),
            detail: message,
        });
    }

    let mut found: Vec<Watched> = Vec::new();
    for search in ["mine", "requested", "involved"] {
        let nodes = json["data"][search]["nodes"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        for node in &nodes {
            if let Some(watched) = github_row(node, me, host) {
                merge(&mut found, watched);
            }
        }
    }
    Ok(found)
}

fn github_row(node: &Value, me: &str, host: &str) -> Option<Watched> {
    // A search answers with any issue-shaped node, and one that is not a pull
    // request comes back without the fragment's fields.
    let number = node["number"].as_u64()?;
    let repository = node["repository"]["nameWithOwner"].as_str()?.to_string();
    let is_me = |login: Option<&str>| login.is_some_and(|login| login.eq_ignore_ascii_case(me));

    let state = match node["state"].as_str()? {
        "MERGED" => WatchState::Merged,
        "CLOSED" => WatchState::Closed,
        _ => WatchState::Open,
    };

    let review_requested = node["reviewRequests"]["nodes"]
        .as_array()
        .into_iter()
        .flatten()
        .any(|n| is_me(n["requestedReviewer"]["login"].as_str()));

    let comments = &node["comments"];
    let reviews = &node["reviews"];
    let count = |of: &Value| of["totalCount"].as_u64().unwrap_or(0) as u32;
    let last = |of: &Value, at: &str| {
        let node = of["nodes"].as_array()?.last()?;
        Some((
            node[at].as_str().unwrap_or_default().to_string(),
            node["author"]["login"].as_str()?.to_string(),
        ))
    };
    // ISO 8601 in UTC compares as text. A pending review has no time yet and
    // sorts first, which is right: nobody else can see it.
    let last_actor = match (last(comments, "createdAt"), last(reviews, "submittedAt")) {
        (Some(comment), Some(review)) if review.0 > comment.0 => Some(review.1),
        (Some((_, who)), _) | (None, Some((_, who))) => Some(who),
        (None, None) => None,
    };

    Some(Watched {
        key: format!("{host}/{repository}#{number}"),
        host: host.to_string(),
        number,
        title: node["title"].as_str().unwrap_or_default().to_string(),
        url: node["url"].as_str().unwrap_or_default().to_string(),
        state,
        mine: is_me(node["author"]["login"].as_str()),
        review_requested,
        activity: count(comments) + count(reviews),
        last_actor_is_me: is_me(last_actor.as_deref()),
        last_actor,
        checks: github::checks_of(node),
        repository,
    })
}

// --- GitLab ----------------------------------------------------------------

fn watch_gitlab(host: &str, token: &str, me: &str) -> Result<Vec<Watched>> {
    let list = format!("{}/merge_requests", Kind::GitLab.api_base(host));
    let order = format!("order_by=updated_at&sort=desc&per_page={LIMIT}");
    let reviewer = crate::forge::encode_segment(me);

    // Every state for the person's own: merged and closed are the news.
    let mine_url = format!("{list}?scope=created_by_me&{order}");
    let asked_url = format!("{list}?scope=all&state=opened&reviewer_username={reviewer}&{order}");

    let mine = gitlab::get(&mine_url, token, host)?;
    let requested = gitlab::get(&asked_url, token, host)?;
    Ok(read_gitlab(&mine, &requested, me, host))
}

/// The two lists as one, each merge request once. Makes no request.
///
/// GitLab's list says how many notes there are but not who wrote the last, so
/// [`Watched::last_actor`] is unknown here and a person's own comment counts
/// as activity.
pub fn read_gitlab(mine: &Value, requested: &Value, me: &str, host: &str) -> Vec<Watched> {
    let mut found = Vec::new();
    for (list, asked) in [(mine, false), (requested, true)] {
        for item in list.as_array().into_iter().flatten() {
            if let Some(watched) = gitlab_row(item, me, host, asked) {
                merge(&mut found, watched);
            }
        }
    }
    found
}

fn gitlab_row(item: &Value, me: &str, host: &str, asked: bool) -> Option<Watched> {
    let number = item["iid"].as_u64()?;
    // `references.full` is `group/project!12`; the project is everything
    // before the bang.
    let repository = item["references"]["full"]
        .as_str()
        .and_then(|full| full.rsplit_once('!'))
        .map(|(project, _)| project.to_string())
        .or_else(|| item["project_id"].as_u64().map(|id| id.to_string()))?;
    let is_me = |login: Option<&str>| login.is_some_and(|login| login.eq_ignore_ascii_case(me));

    let state = match item["state"].as_str()? {
        "merged" => WatchState::Merged,
        "closed" => WatchState::Closed,
        _ => WatchState::Open,
    };
    let reviewing = item["reviewers"]
        .as_array()
        .into_iter()
        .flatten()
        .any(|r| is_me(r["username"].as_str()));

    Some(Watched {
        key: format!("{host}/{repository}#{number}"),
        host: host.to_string(),
        number,
        title: item["title"].as_str().unwrap_or_default().to_string(),
        url: item["web_url"].as_str().unwrap_or_default().to_string(),
        state,
        mine: is_me(item["author"]["username"].as_str()),
        review_requested: asked || reviewing,
        activity: item["user_notes_count"].as_u64().unwrap_or(0) as u32,
        last_actor: None,
        last_actor_is_me: false,
        checks: None,
        repository,
    })
}

/// Add a row, or fold it into the one already there for the same pull
/// request: the searches overlap, and each flag is true if any said so.
fn merge(found: &mut Vec<Watched>, watched: Watched) {
    match found
        .iter_mut()
        .find(|existing| existing.key == watched.key)
    {
        Some(existing) => {
            existing.mine |= watched.mine;
            existing.review_requested |= watched.review_requested;
        }
        None => found.push(watched),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn node(number: u64, state: &str, author: &str) -> Value {
        json!({
            "number": number,
            "title": format!("PR {number}"),
            "url": format!("https://github.com/o/r/pull/{number}"),
            "state": state,
            "author": { "login": author },
            "repository": { "nameWithOwner": "o/r" },
            "reviewRequests": { "nodes": [] },
            "commits": { "nodes": [{ "commit": { "statusCheckRollup": { "state": "FAILURE" } } }] },
            "comments": {
                "totalCount": 2,
                "nodes": [{ "author": { "login": "ada" }, "createdAt": "2026-10-08T10:00:00Z" }]
            },
            "reviews": {
                "totalCount": 1,
                "nodes": [{ "author": { "login": "Me" }, "submittedAt": "2026-10-08T11:00:00Z" }]
            }
        })
    }

    fn answer(mine: Vec<Value>, requested: Vec<Value>, involved: Vec<Value>) -> String {
        json!({ "data": {
            "mine": { "nodes": mine },
            "requested": { "nodes": requested },
            "involved": { "nodes": involved }
        }})
        .to_string()
    }

    /// The three searches, read as `me` on github.com.
    fn github(mine: Vec<Value>, requested: Vec<Value>, involved: Vec<Value>) -> Vec<Watched> {
        read_github(&answer(mine, requested, involved), "me", "github.com").unwrap()
    }

    #[test]
    fn a_merged_pull_request_of_mine_reads_as_merged_and_mine() {
        let found = github(vec![node(7, "MERGED", "me")], vec![], vec![]);

        assert_eq!(found.len(), 1);
        assert_eq!(found[0].key, "github.com/o/r#7");
        assert_eq!(found[0].state, WatchState::Merged);
        assert!(found[0].mine);
        assert_eq!(found[0].checks, Some(CheckState::Failing));
    }

    #[test]
    fn activity_counts_comments_and_reviews_and_the_latest_speaker_wins() {
        let found = github(vec![node(7, "OPEN", "me")], vec![], vec![]);

        assert_eq!(found[0].activity, 3);
        // The review came after the comment, and the login matches in any case.
        assert_eq!(found[0].last_actor.as_deref(), Some("Me"));
        assert!(found[0].last_actor_is_me);
    }

    #[test]
    fn the_same_pull_request_in_two_searches_is_one_row_with_both_flags() {
        let mut asked = node(9, "OPEN", "ada");
        asked["reviewRequests"] = json!({ "nodes": [{ "requestedReviewer": { "login": "me" } }] });
        let found = github(vec![], vec![asked], vec![node(9, "OPEN", "ada")]);

        assert_eq!(found.len(), 1);
        assert!(found[0].review_requested);
        assert!(!found[0].mine);
    }

    #[test]
    fn an_issue_in_the_search_is_skipped() {
        let found = github(vec![json!({})], vec![], vec![]);

        assert!(found.is_empty());
    }

    #[test]
    fn a_graphql_error_is_reported() {
        let body = json!({ "errors": [{ "message": "rate limited" }] }).to_string();

        assert!(read_github(&body, "me", "github.com").is_err());
    }

    #[test]
    fn gitlab_merge_requests_read_their_project_state_and_notes() {
        let mine = json!([{
            "iid": 12, "title": "Fix", "web_url": "https://gitlab.com/g/p/-/merge_requests/12",
            "state": "merged", "user_notes_count": 4,
            "references": { "full": "g/p!12" }, "author": { "username": "me" }, "reviewers": []
        }]);
        let found = read_gitlab(&mine, &json!([]), "me", "gitlab.com");

        assert_eq!(found[0].key, "gitlab.com/g/p#12");
        assert_eq!(found[0].state, WatchState::Merged);
        assert_eq!(found[0].activity, 4);
        assert!(found[0].mine);
        assert!(found[0].last_actor.is_none());
    }

    #[test]
    fn a_gitlab_review_request_is_flagged() {
        let requested = json!([{
            "iid": 3, "title": "Add", "web_url": "u", "state": "opened", "user_notes_count": 0,
            "references": { "full": "g/p!3" }, "author": { "username": "ada" }
        }]);
        let found = read_gitlab(&json!([]), &requested, "me", "gitlab.com");

        assert!(found[0].review_requested);
        assert!(!found[0].mine);
    }
}
