// SPDX-License-Identifier: GPL-3.0-or-later

//! Explicit pipeline reads and single-attempt writes on configured hosts.
use super::{
    encode_segment as enc, github::timestamp, http, issues::root, status_error, Kind, Repo,
};
use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Run {
    pub id: u64,
    pub name: String,
    pub branch: String,
    pub commit: String,
    pub event: String,
    pub state: String,
    pub result: String,
    pub url: String,
    pub started: i64,
    pub ended: i64,
    pub duration: Option<u64>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Step {
    pub number: u64,
    pub name: String,
    pub state: String,
    pub result: String,
    pub started: i64,
    pub ended: i64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Job {
    pub id: u64,
    pub name: String,
    pub stage: String,
    pub state: String,
    pub result: String,
    pub runner: String,
    pub started: i64,
    pub ended: i64,
    pub duration: Option<u64>,
    pub url: String,
    pub steps: Vec<Step>,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Page {
    pub runs: Vec<Run>,
    pub etag: Option<String>,
    pub more: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Jobs {
    pub jobs: Vec<Job>,
    pub etag: Option<String>,
    pub more: bool,
    pub page: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Watches {
    pub default_branch: String,
    pub branches: Vec<String>,
    pub requests: Vec<String>,
}
fn string(v: &Value, k: &str) -> String {
    v[k].as_str().unwrap_or("").into()
}
fn failure(repo: &Repo, text: impl Into<String>) -> Error {
    Error::Forge {
        host: repo.host.clone(),
        detail: text.into(),
    }
}
fn checked(repo: &Repo, response: http::Response, write: bool) -> Result<Value> {
    if response.status == 401 || response.status == 403 {
        return Err(failure(repo,format!("The token cannot {} pipelines. {}",if write{"change"}else{"read"},match(repo.kind,write){(Kind::GitHub,true)=>"Grant Actions: write (or repo for a classic token).",(Kind::GitHub,false)=>"Grant Actions: read (or repo for a private repository with a classic token).",(_,true)=>"Grant the api scope.",(_,false)=>"Grant the read_api or api scope."})));
    }
    if !(200..300).contains(&response.status) {
        return Err(status_error(
            &repo.host,
            response.status,
            &response.body,
            response.retry_after.as_deref(),
        ));
    }
    if response.body.is_empty() {
        return Ok(Value::Null);
    }
    serde_json::from_str(&response.body)
        .map_err(|_| failure(repo, "The host sent an unreadable pipeline response."))
}
fn duration(v: &Value, start: i64, end: i64) -> Option<u64> {
    v["duration"]
        .as_f64()
        .filter(|d| d.is_finite() && *d >= 0.0)
        .map(|d| d as u64)
        .or_else(|| (start > 0 && end >= start).then_some((end - start).max(0) as u64))
}
pub fn run(v: &Value, github: bool) -> Run {
    let started = timestamp(
        v[if github {
            "run_started_at"
        } else {
            "started_at"
        }]
        .as_str()
        .or(v["created_at"].as_str()),
    );
    let ended = timestamp(v[if github { "updated_at" } else { "finished_at" }].as_str());
    let status = string(v, "status");
    let active = if github {
        status != "completed"
    } else {
        matches!(
            status.as_str(),
            "created"
                | "waiting_for_resource"
                | "preparing"
                | "pending"
                | "running"
                | "manual"
                | "scheduled"
        )
    };
    Run {
        id: v["id"].as_u64().unwrap_or(0),
        name: if github {
            string(v, "name")
        } else {
            format!("Pipeline #{}", v["id"])
        },
        branch: string(v, if github { "head_branch" } else { "ref" }),
        commit: string(v, if github { "head_sha" } else { "sha" }),
        event: string(v, if github { "event" } else { "source" }),
        state: if active {
            if status == "running" || status == "in_progress" {
                "running"
            } else {
                "queued"
            }
        } else {
            "completed"
        }
        .into(),
        result: if github {
            string(v, "conclusion")
        } else if active {
            String::new()
        } else {
            status
        },
        url: string(v, if github { "html_url" } else { "web_url" }),
        started,
        ended: if active { 0 } else { ended },
        duration: if active {
            None
        } else {
            duration(v, started, ended)
        },
    }
}
pub fn job(v: &Value, github: bool) -> Job {
    let started = timestamp(v["started_at"].as_str());
    let ended = timestamp(
        v[if github {
            "completed_at"
        } else {
            "finished_at"
        }]
        .as_str(),
    );
    let status = string(v, "status");
    let active = if github {
        status != "completed"
    } else {
        matches!(
            status.as_str(),
            "created"
                | "pending"
                | "preparing"
                | "running"
                | "waiting_for_resource"
                | "manual"
                | "scheduled"
        )
    };
    Job {
        id: v["id"].as_u64().unwrap_or(0),
        name: string(v, "name"),
        stage: string(v, "stage"),
        state: if active {
            if matches!(status.as_str(), "running" | "in_progress") {
                "running"
            } else {
                "queued"
            }
        } else {
            "completed"
        }
        .into(),
        result: if github {
            string(v, "conclusion")
        } else if active {
            String::new()
        } else {
            status
        },
        runner: if github {
            string(v, "runner_name")
        } else {
            string(&v["runner"], "description")
        },
        started,
        ended,
        duration: duration(v, started, ended),
        url: string(v, if github { "html_url" } else { "web_url" }),
        steps: v["steps"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|s| Step {
                number: s["number"].as_u64().unwrap_or(0),
                name: string(s, "name"),
                state: string(s, "status"),
                result: string(s, "conclusion"),
                started: timestamp(s["started_at"].as_str()),
                ended: timestamp(s["completed_at"].as_str()),
            })
            .collect(),
    }
}
pub fn page(repo: &Repo, token: &str, branch: &str, old: Option<&Page>) -> Result<Page> {
    let url = format!(
        "{}/{}",
        root(repo)?,
        if repo.kind == Kind::GitHub {
            format!("actions/runs?per_page=12&branch={}", enc(branch))
        } else {
            format!(
                "pipelines?per_page=12&ref={}&order_by=id&sort=desc",
                enc(branch)
            )
        }
    );
    let response =
        http::get_conditional_json(&url, token, &repo.host, old.and_then(|p| p.etag.as_deref()))?;
    if response.response.status == 304 {
        return old.cloned().ok_or_else(|| {
            failure(
                repo,
                "The host returned no pipeline list and no saved list exists.",
            )
        });
    }
    let value = checked(repo, response.response, false)?;
    let rows = if repo.kind == Kind::GitHub {
        &value["workflow_runs"]
    } else {
        &value
    };
    let mut runs: Vec<_> = rows
        .as_array()
        .into_iter()
        .flatten()
        .map(|v| run(v, repo.kind == Kind::GitHub))
        .collect();
    if repo.kind == Kind::GitLab {
        for row in &mut runs {
            let detail = checked(
                repo,
                http::get_json(
                    &format!("{}/pipelines/{}", root(repo)?, row.id),
                    token,
                    &repo.host,
                )?,
                false,
            )?;
            *row = run(&detail, false);
        }
    }
    Ok(Page {
        runs,
        etag: response.etag,
        more: response.more,
    })
}
pub fn jobs(repo: &Repo, token: &str, id: u64, page: u32, old: Option<&Jobs>) -> Result<Jobs> {
    if id == 0 || !(1..=100).contains(&page) {
        return Err(failure(
            repo,
            "Choose a pipeline and a job page between 1 and 100.",
        ));
    }
    let url = format!(
        "{}/{}",
        root(repo)?,
        if repo.kind == Kind::GitHub {
            format!("actions/runs/{id}/jobs?per_page=100&page={page}&filter=latest")
        } else {
            format!("pipelines/{id}/jobs?per_page=100&page={page}&include_retried=false")
        }
    );
    let response =
        http::get_conditional_json(&url, token, &repo.host, old.and_then(|p| p.etag.as_deref()))?;
    if response.response.status == 304 {
        return old
            .cloned()
            .ok_or_else(|| failure(repo, "The host returned no jobs and no saved jobs exist."));
    }
    let value = checked(repo, response.response, false)?;
    let rows = if repo.kind == Kind::GitHub {
        &value["jobs"]
    } else {
        &value
    };
    Ok(Jobs {
        jobs: rows
            .as_array()
            .into_iter()
            .flatten()
            .map(|v| job(v, repo.kind == Kind::GitHub))
            .collect(),
        etag: response.etag,
        more: response.more,
        page,
    })
}
pub fn watches(repo: &Repo, token: &str, me: &str) -> Result<Watches> {
    let metadata = checked(
        repo,
        http::get_json(&root(repo)?, token, &repo.host)?,
        false,
    )?;
    let pulls = super::pull_requests(repo, token, me)?;
    let requests = pulls
        .into_iter()
        .filter(|p| p.author_name == me)
        .map(|p| p.source_branch)
        .collect();
    Ok(Watches {
        default_branch: string(&metadata, "default_branch"),
        branches: vec![],
        requests,
    })
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Write {
    Cancel { id: u64 },
    RerunFailed { id: u64 },
    Lint { content: String },
}
pub fn write(repo: &Repo, token: &str, action: &Write, confirmed: bool) -> Result<Value> {
    if !confirmed {
        return Err(failure(
            repo,
            "Confirm this pipeline action before sending it.",
        ));
    }
    let (url, body) = match action {
        Write::Cancel { id } | Write::RerunFailed { id } => {
            if *id == 0 {
                return Err(failure(repo, "Choose a pipeline first."));
            }
            let cancel = matches!(action, Write::Cancel { .. });
            (
                format!(
                    "{}/{}",
                    root(repo)?,
                    if repo.kind == Kind::GitHub {
                        format!(
                            "actions/runs/{id}/{}",
                            if cancel {
                                "cancel"
                            } else {
                                "rerun-failed-jobs"
                            }
                        )
                    } else {
                        format!("pipelines/{id}/{}", if cancel { "cancel" } else { "retry" })
                    }
                ),
                "{}".into(),
            )
        }
        Write::Lint { content } => {
            if repo.kind != Kind::GitLab {
                return Err(failure(repo, "Host lint is available on GitLab."));
            }
            if content.len() > 1024 * 1024 {
                return Err(failure(repo, "The CI file exceeds 1 MiB."));
            }
            (
                format!("{}/ci/lint", root(repo)?),
                json!({"content":content,"include_merged_yaml":false}).to_string(),
            )
        }
    };
    checked(repo,http::post_json(&url,token,&repo.host,&body).map_err(|_|failure(repo,"The pipeline action received no answer. Read its status before trying again; it may have succeeded."))?,true)
}
pub fn log(repo: &Repo, token: &str, id: u64) -> Result<String> {
    if id == 0 {
        return Err(failure(repo, "Choose a job first."));
    }
    let url = if repo.kind == Kind::GitHub {
        let job = checked(
            repo,
            http::get_json(
                &format!("{}/actions/jobs/{id}", root(repo)?),
                token,
                &repo.host,
            )?,
            false,
        )?;
        if job["status"] != "completed" {
            return Err(failure(repo,"GitHub exposes this log after the job completes. Follow its step states while it runs."));
        }
        format!("{}/actions/jobs/{id}/logs", root(repo)?)
    } else {
        format!("{}/jobs/{id}/trace", root(repo)?)
    };
    http::get_log(&url, token, &repo.host, repo.kind == Kind::GitHub)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn both_hosts_map_step_states_times_and_manual_runs() {
        let gh = run(
            &json!({"id":3,"status":"completed","conclusion":"success","run_started_at":"2026-10-14T12:00:00Z","updated_at":"2026-10-14T12:02:00Z"}),
            true,
        );
        assert_eq!(gh.duration, Some(120));
        let gl = run(&json!({"id":7,"status":"manual","ref":"main"}), false);
        assert_eq!(gl.state, "queued");
        let j = job(
            &json!({"id":9,"status":"in_progress","steps":[{"number":1,"name":"build","status":"completed","conclusion":"success"},{"number":2,"name":"test","status":"in_progress"}]}),
            true,
        );
        assert_eq!(j.state, "running");
        assert_eq!(j.steps.len(), 2);
    }
    #[test]
    fn permission_errors_name_the_required_scope_and_writes_need_confirmation() {
        let repo = Repo {
            kind: Kind::GitLab,
            host: "gitlab.com".into(),
            owner: "group".into(),
            name: "app".into(),
        };
        assert!(checked(
            &repo,
            http::Response {
                status: 403,
                body: String::new(),
                retry_after: None
            },
            true
        )
        .unwrap_err()
        .to_string()
        .contains("api scope"));
        assert!(write(&repo, "secret", &Write::Cancel { id: 2 }, false)
            .unwrap_err()
            .to_string()
            .contains("Confirm"));
        let repo = Repo {
            kind: Kind::GitHub,
            ..repo
        };
        assert!(checked(
            &repo,
            http::Response {
                status: 403,
                body: String::new(),
                retry_after: None
            },
            false
        )
        .unwrap_err()
        .to_string()
        .contains("Actions: read"));
    }
}
