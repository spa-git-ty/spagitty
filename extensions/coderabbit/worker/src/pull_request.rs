// SPDX-License-Identifier: GPL-3.0-or-later

//! CodeRabbit's mapping of public forge snapshots. No forge token or HTTP client.
use super::rpc::Rpc;
use serde_json::{json, Value};

fn actor_matches(actor: &Value, settings: &Value) -> bool {
    let login = settings["githubBotLogin"]
        .as_str()
        .unwrap_or("coderabbitai[bot]");
    if settings["githubBotId"]
        .as_str()
        .is_some_and(|s| !s.is_empty() && s.parse::<u64>().is_err())
    {
        return false;
    }
    let id = settings["githubBotId"]
        .as_str()
        .filter(|s| !s.is_empty())
        .and_then(|s| s.parse::<u64>().ok());
    // A configured service account requires a numeric identity. Default bot
    // attribution requires GitHub's Bot type as well as its platform login.
    actor["id"].as_u64().is_some()
        && actor["login"] == login
        && match id {
            Some(id) => actor["id"] == id,
            None => login == "coderabbitai[bot]" && actor["kind"] == "Bot",
        }
}
fn check_matches(item: &Value, settings: &Value) -> bool {
    let slug = settings["githubAppSlug"].as_str().unwrap_or("coderabbitai");
    if settings["githubAppId"]
        .as_str()
        .is_some_and(|s| !s.is_empty() && s.parse::<u64>().is_err())
    {
        return false;
    }
    let id = settings["githubAppId"]
        .as_str()
        .filter(|s| !s.is_empty())
        .and_then(|s| s.parse::<u64>().ok());
    item["appSlug"] == slug
        && item["appId"].as_u64().is_some()
        && id
            .map(|id| item["appId"] == id)
            .unwrap_or(slug == "coderabbitai")
}
fn values<'a>(snapshot: &'a Value, key: &str) -> Vec<&'a Value> {
    snapshot[key]["items"]
        .as_array()
        .map(|a| a.iter().collect())
        .unwrap_or_default()
}
pub fn panel_data(snapshot: &Value, settings: &Value, pending: &Value) -> Value {
    let head = snapshot["headSha"].as_str().unwrap_or("");
    let complete = ["discussion", "findings", "reviews", "checks"]
        .iter()
        .all(|k| snapshot[*k]["complete"] == true)
        && snapshot["revisionCurrent"] == true;
    let comments: Vec<_> = values(snapshot, "discussion")
        .into_iter()
        .filter(|i| actor_matches(&i["author"], settings))
        .collect();
    let findings: Vec<_> = values(snapshot, "findings")
        .into_iter()
        .filter(|i| actor_matches(&i["author"], settings))
        .collect();
    let reviews: Vec<_> = values(snapshot, "reviews")
        .into_iter()
        .filter(|i| actor_matches(&i["author"], settings))
        .collect();
    let checks: Vec<_> = values(snapshot, "checks")
        .into_iter()
        .filter(|i| check_matches(i, settings))
        .collect();
    let current_checks: Vec<_> = checks
        .iter()
        .copied()
        .filter(|i| i["commitSha"] == head)
        .collect();
    let current_reviews: Vec<_> = reviews
        .iter()
        .copied()
        .filter(|i| i["commitSha"] == head)
        .collect();
    let latest = current_checks
        .iter()
        .max_by_key(|i| i["id"].as_u64().unwrap_or(0));
    let running = latest.is_some_and(|i| i["state"] == "queued" || i["state"] == "in_progress");
    let finished = latest.is_some_and(|i| i["state"] == "completed") || !current_reviews.is_empty();
    let old = !reviews.is_empty()
        || !checks.is_empty()
        || findings
            .iter()
            .any(|i| i["commitSha"].as_str().is_some_and(|c| c != head));
    let requested = pending["headSha"] == head
        && (pending["status"] == "posted" || pending["status"] == "uncertain");
    let state = if snapshot["revisionCurrent"] != true {
        "stale"
    } else if running {
        "running"
    } else if finished {
        "completed"
    } else if requested {
        "requested"
    } else if old {
        "stale"
    } else if !complete {
        "unavailable"
    } else {
        "notObserved"
    };
    let mut items = Vec::new();
    let mut links = Vec::new();
    for (kind, list) in [
        ("comment", &comments),
        ("finding", &findings),
        ("comment", &reviews),
        ("check", &checks),
    ] {
        for i in list.iter().rev().take(10) {
            let body = i["body"]
                .as_str()
                .unwrap_or("")
                .chars()
                .take(2_000)
                .collect::<String>();
            let state = i["conclusion"]
                .as_str()
                .or_else(|| i["state"].as_str())
                .unwrap_or("");
            items.push(json!({"kind":kind,"author":i["author"]["login"],"authorType":if i["author"]["kind"]=="Bot"{"bot"}else{"user"},"body":body,"title":i["title"],"path":i["path"],"line":i["line"],"state":state,"url":i["url"]}));
            if let Some(url) = i["url"].as_str().filter(|u| u.starts_with("https://")) {
                links.push(json!({"title":format!("Open {kind} {}",i["id"]),"url":url}));
            }
        }
    }
    let summary = comments
        .last()
        .and_then(|i| i["body"].as_str())
        .unwrap_or("");
    let truncated = comments.len() > 10
        || findings.len() > 10
        || reviews.len() > 10
        || checks.len() > 10
        || comments
            .iter()
            .chain(findings.iter())
            .chain(reviews.iter())
            .chain(checks.iter())
            .any(|i| {
                i["body"]
                    .as_str()
                    .is_some_and(|b| b.chars().count() > 2_000)
            });
    let headline = if truncated {
        "Long review data was shortened. Open the source links for the full text."
    } else if !complete {
        "Some review data could not be read. Revision coverage and thread resolution may be unknown."
    } else if pending["status"] == "uncertain" {
        "Request delivery is uncertain. Refresh before choosing to send again."
    } else if state == "notObserved" && !summary.is_empty() {
        "A bot summary is visible; no exact-revision review has been observed."
    } else {
        "CodeRabbit status on this pull request. A completed check does not resolve its findings."
    };
    json!({"state":state,"headline":headline,"summary":summary.chars().take(4_000).collect::<String>(),"revision":head,"items":items,"links":links,"complete":complete})
}
fn context(params: &Value) -> Result<(&str, u64), String> {
    let context = &params["context"];
    let repo = context["repository"].as_str().ok_or("Open a repository.")?;
    let number = context["pullRequest"]["number"]
        .as_u64()
        .filter(|n| *n > 0)
        .ok_or("Select a pull request.")?;
    Ok((repo, number))
}
fn stored(rpc: &Rpc, repo: &str, number: u64) -> Value {
    rpc.call(
        "storage.get",
        json!({"repository":repo,"key":format!("pr-request-{number}")}),
    )
    .ok()
    .map(|v| v["value"].clone())
    .unwrap_or(Value::Null)
}
fn save(rpc: &Rpc, repo: &str, number: u64, value: Value) -> Result<(), String> {
    rpc.call(
        "storage.set",
        json!({"repository":repo,"key":format!("pr-request-{number}"),"value":value}),
    )
    .map(|_| ())
    .map_err(|e| e.message)
}
pub fn panel(rpc: &Rpc, params: &Value) -> Result<Value, String> {
    let (repo, number) = context(params)?;
    let snapshot = rpc
        .call(
            "forge.pullRequest.snapshot",
            json!({"repository":repo,"number":number}),
        )
        .map_err(|e| e.message)?;
    let mut pending = stored(rpc, repo, number);
    if pending["status"] == "uncertain"
        && snapshot["discussion"]["complete"] == true
        && snapshot["revisionCurrent"] == true
    {
        pending["refreshed"] = json!(true);
        save(rpc, repo, number, pending.clone())?;
    }
    Ok(panel_data(&snapshot, &rpc.settings(), &pending))
}
pub fn request(rpc: &Rpc, params: &Value, full: bool) -> Result<String, String> {
    let (repo, number) = context(params)?;
    let operation = params["operationId"].as_str().ok_or("No operation.")?;
    let pending = stored(rpc, repo, number);
    if pending["status"] == "uncertain" && pending["refreshed"] != true {
        return Err("The previous request may already have been delivered. Refresh the CodeRabbit panel before deliberately sending again.".into());
    }
    if rpc
        .cancel_flag(operation)
        .load(std::sync::atomic::Ordering::Acquire)
    {
        return Err("Cancelled.".into());
    }
    let settings = rpc.settings();
    let login = settings["githubBotLogin"]
        .as_str()
        .unwrap_or("coderabbitai[bot]")
        .trim_end_matches("[bot]");
    if login.is_empty()
        || login.len() > 39
        || !login.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
    {
        return Err("Configure a valid GitHub bot login before requesting a review.".into());
    }
    let body = format!("@{login} {}", if full { "full review" } else { "review" });
    // Record uncertainty before the write. A crash or storage failure after
    // delivery must never make the next click silently duplicate the request.
    save(
        rpc,
        repo,
        number,
        json!({"status":"uncertain","headSha":params["context"]["pullRequest"]["headSha"],"refreshed":false,"message":"Delivery is not confirmed; refresh before sending again."}),
    )?;
    let receipt = rpc
        .call(
            "forge.pullRequest.comment",
            json!({"repository":repo,"number":number,"body":body,"operationId":operation}),
        )
        .map_err(|e| e.message)?;
    if receipt["status"] != "posted" && receipt["status"] != "uncertain" {
        // Preserve the pre-write uncertain marker rather than enabling a resend.
        return Err(
            "The host returned an unrecognized delivery receipt. Refresh before requesting again."
                .into(),
        );
    }
    save(rpc, repo, number, receipt.clone())?;
    if receipt["status"] == "uncertain" {
        Ok(receipt["message"]
            .as_str()
            .unwrap_or("Delivery uncertain; refresh before sending again.")
            .into())
    } else {
        Ok("CodeRabbit review requested. Refresh its panel to see the result; requesting is not completion.".into())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn malformed_identity_settings_never_fall_back_to_the_default() {
        assert!(!actor_matches(
            &json!({"login":"coderabbitai[bot]","kind":"Bot","id":9}),
            &json!({"githubBotId":"typo"})
        ));
        assert!(!check_matches(
            &json!({"appSlug":"coderabbitai","appId":9}),
            &json!({"githubAppId":"typo"})
        ));
    }
    #[test]
    fn large_provider_content_stays_within_the_host_message_limit() {
        let mut s = snapshot();
        let text = "😀".repeat(20_000);
        s["discussion"]["items"] = json!((0..100).map(|id| json!({"id":id,"author":{"id":9,"login":"coderabbitai[bot]","kind":"Bot"},"body":text})).collect::<Vec<_>>());
        let p = panel_data(&s, &json!({}), &Value::Null);
        assert!(serde_json::to_vec(&p).unwrap().len() < 512 * 1024);
        assert_eq!(p["items"].as_array().unwrap().len(), 10);
        assert!(p["headline"].as_str().unwrap().contains("shortened"));
    }
    fn snapshot() -> Value {
        json!({"headSha":"new","revisionCurrent":true,"discussion":{"complete":true,"items":[]},"findings":{"complete":true,"items":[]},"reviews":{"complete":true,"items":[]},"checks":{"complete":true,"items":[]}})
    }
    #[test]
    fn silence_or_a_human_mention_is_never_approval() {
        let mut s = snapshot();
        assert_eq!(
            panel_data(&s, &json!({}), &Value::Null)["state"],
            "notObserved"
        );
        s["discussion"]["items"] = json!([{"id":1,"author":{"login":"coderabbitai[bot]","kind":"User"},"body":"@coderabbitai approved"}]);
        let p = panel_data(&s, &json!({}), &Value::Null);
        assert_eq!(p["state"], "notObserved");
        assert_eq!(p["summary"], "");
    }
    #[test]
    fn a_summary_does_not_prove_revision_coverage() {
        let mut s = snapshot();
        s["discussion"]["items"] = json!([{"id":1,"author":{"id":9,"login":"coderabbitai[bot]","kind":"Bot"},"body":"summary"}]);
        let p = panel_data(&s, &json!({}), &Value::Null);
        assert_eq!(p["summary"], "summary");
        assert_eq!(p["state"], "notObserved");
    }
    #[test]
    fn checks_are_attributed_to_an_app_and_exact_head() {
        let mut s = snapshot();
        s["checks"]["items"] = json!([{"id":1,"appId":9,"appSlug":"coderabbitai","title":"review","commitSha":"new","state":"in_progress"}]);
        assert_eq!(panel_data(&s, &json!({}), &Value::Null)["state"], "running");
        s["checks"]["items"][0]["state"] = json!("completed");
        assert_eq!(
            panel_data(&s, &json!({}), &Value::Null)["state"],
            "completed"
        );
        s["checks"]["items"][0]["commitSha"] = json!("old");
        assert_eq!(panel_data(&s, &json!({}), &Value::Null)["state"], "stale");
        s["checks"]["items"][0]["appSlug"] = json!("impostor");
        assert_eq!(
            panel_data(&s, &json!({}), &Value::Null)["state"],
            "notObserved"
        );
    }
    #[test]
    fn incomplete_snapshots_and_requests_are_explicit() {
        let mut s = snapshot();
        s["findings"]["complete"] = json!(false);
        let p = panel_data(&s, &json!({}), &json!({"headSha":"new","status":"posted"}));
        assert_eq!(p["complete"], false);
        assert_eq!(p["state"], "requested");
        s["revisionCurrent"] = json!(false);
        assert_eq!(panel_data(&s, &json!({}), &Value::Null)["state"], "stale");
    }
    #[test]
    fn a_custom_service_account_requires_its_numeric_identity() {
        assert!(!actor_matches(
            &json!({"login":"review-service","kind":"User","id":7}),
            &json!({"githubBotLogin":"review-service"})
        ));
        assert!(actor_matches(
            &json!({"login":"review-service","kind":"User","id":7}),
            &json!({"githubBotLogin":"review-service","githubBotId":"7"})
        ));
    }
}
