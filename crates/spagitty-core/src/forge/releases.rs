// SPDX-License-Identifier: GPL-3.0-or-later

//! Releases share one screen shape. Mutations make a single request and never
//! retry an uncertain delivery. GitLab drafts belong to the application layer.

use super::{encode_segment as enc, http, issues::root, status_error, Kind, Repo};
use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Asset {
    pub id: String,
    pub name: String,
    pub url: String,
    pub size: Option<u64>,
    pub downloads: Option<u64>,
    pub removable: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Release {
    pub id: String,
    pub tag: String,
    pub target: String,
    pub title: String,
    pub body: String,
    pub draft: bool,
    pub prerelease: bool,
    pub latest: bool,
    pub author: String,
    pub created: String,
    pub url: String,
    pub assets: Vec<Asset>,
    pub local: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Page {
    pub releases: Vec<Release>,
    pub page: u32,
    pub more: bool,
    pub etag: Option<String>,
}

fn text(value: &Value, key: &str) -> String {
    value[key].as_str().unwrap_or("").into()
}
fn failure(repo: &Repo, detail: &str) -> Error {
    Error::Forge {
        host: repo.host.clone(),
        detail: detail.into(),
    }
}
pub fn checked(repo: &Repo, response: http::Response) -> Result<Value> {
    if !(200..300).contains(&response.status) {
        return Err(status_error(
            &repo.host,
            response.status,
            &response.body,
            response.retry_after.as_deref(),
        ));
    }
    if response.body.trim().is_empty() {
        return Ok(Value::Null);
    }
    serde_json::from_str(&response.body)
        .map_err(|_| failure(repo, "The host sent an unreadable release answer."))
}
fn get(repo: &Repo, token: &str, path: &str) -> Result<Value> {
    checked(
        repo,
        http::get_json(&format!("{}/{path}", root(repo)?), token, &repo.host)?,
    )
}

pub fn read_release(raw: &Value, kind: Kind) -> Release {
    let github = kind == Kind::GitHub;
    let mut assets: Vec<Asset> = raw["assets"][if github { "unused" } else { "links" }]
        .as_array()
        .into_iter()
        .flatten()
        .map(|asset| Asset {
            id: asset["id"].to_string(),
            name: text(asset, "name"),
            url: asset["direct_asset_url"]
                .as_str()
                .filter(|url| !url.is_empty())
                .or_else(|| asset["url"].as_str())
                .unwrap_or("")
                .into(),
            size: None,
            downloads: None,
            removable: true,
        })
        .collect();
    if github {
        assets = raw["assets"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|asset| Asset {
                id: asset["id"].to_string(),
                name: text(asset, "name"),
                url: text(asset, "browser_download_url"),
                size: asset["size"].as_u64(),
                downloads: asset["download_count"].as_u64(),
                removable: true,
            })
            .collect();
        for (field, name) in [
            ("zipball_url", "Source code (zip)"),
            ("tarball_url", "Source code (tar.gz)"),
        ] {
            if let Some(url) = raw[field].as_str() {
                assets.push(Asset {
                    id: field.into(),
                    name: name.into(),
                    url: url.into(),
                    ..Asset::default()
                });
            }
        }
    } else {
        assets.extend(
            raw["assets"]["sources"]
                .as_array()
                .into_iter()
                .flatten()
                .map(|source| Asset {
                    id: format!("source:{}", text(source, "format")),
                    name: format!("Source code ({})", text(source, "format")),
                    url: text(source, "url"),
                    ..Asset::default()
                }),
        );
    }
    Release {
        id: if github {
            raw["id"].to_string()
        } else {
            text(raw, "tag_name")
        },
        tag: text(raw, "tag_name"),
        target: if github {
            text(raw, "target_commitish")
        } else {
            text(&raw["commit"], "id")
        },
        title: text(raw, "name"),
        body: text(raw, if github { "body" } else { "description" }),
        draft: github && raw["draft"].as_bool().unwrap_or(false),
        prerelease: github && raw["prerelease"].as_bool().unwrap_or(false),
        latest: false,
        author: text(&raw["author"], if github { "login" } else { "username" }),
        created: text(
            raw,
            if github {
                "published_at"
            } else {
                "released_at"
            },
        ),
        url: if github {
            text(raw, "html_url")
        } else {
            text(&raw["_links"], "self")
        },
        assets,
        local: false,
    }
}

pub fn page(repo: &Repo, token: &str, number: u32, old: Option<&Page>) -> Result<Page> {
    if !(1..=10000).contains(&number) {
        return Err(failure(repo, "Choose a release page."));
    }
    let response = http::get_conditional_json(
        &format!("{}/releases?per_page=50&page={number}", root(repo)?),
        token,
        &repo.host,
        old.and_then(|p| p.etag.as_deref()),
    )?;
    if response.response.status == 304 {
        let mut saved = old
            .cloned()
            .ok_or_else(|| failure(repo, "The unchanged release page has no saved answer."))?;
        if number == 1 && repo.kind == Kind::GitHub {
            mark_latest(repo, token, &mut saved.releases);
        }
        return Ok(saved);
    }
    let raw = checked(repo, response.response)?;
    let rows = raw
        .as_array()
        .ok_or_else(|| failure(repo, "The host did not send a release list."))?;
    let mut releases: Vec<_> = rows
        .iter()
        .map(|row| read_release(row, repo.kind))
        .collect();
    if number == 1 {
        if repo.kind == Kind::GitHub {
            mark_latest(repo, token, &mut releases);
        } else {
            // GitLab lists by release date, newest first, and its latest is
            // the newest one already released. Its permalink endpoint answers
            // with a redirect, which the transport never follows (BUG-089).
            if let Some(release) = gitlab_latest(rows).and_then(|i| releases.get_mut(i)) {
                release.latest = true;
            }
        }
    }
    Ok(Page {
        releases,
        page: number,
        more: response.more || rows.len() == 50,
        etag: response.etag,
    })
}

/// The first release GitLab lists that is already out: its list is newest
/// first by release date, and an upcoming release is never the latest.
fn gitlab_latest(rows: &[Value]) -> Option<usize> {
    rows.iter()
        .position(|row| !row["upcoming_release"].as_bool().unwrap_or(false))
}

/// GitHub only. A *Latest* mark is a detail: failing to read it leaves the
/// list unmarked rather than refusing the whole list.
fn mark_latest(repo: &Repo, token: &str, releases: &mut [Release]) {
    let latest = root(repo)
        .and_then(|root| http::get_json(&format!("{root}/releases/latest"), token, &repo.host))
        .ok()
        .filter(|response| response.status != 404)
        .and_then(|response| checked(repo, response).ok())
        .unwrap_or(Value::Null);
    let id = latest["id"].to_string();
    for release in releases {
        release.latest = release.id == id;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum Write {
    Save {
        release: Release,
        publish: bool,
    },
    Delete {
        id: String,
        tag: String,
    },
    DeleteTag {
        tag: String,
    },
    RemoveAsset {
        release_id: String,
        tag: String,
        asset_id: String,
    },
    LinkAsset {
        tag: String,
        name: String,
        url: String,
    },
    GenerateNotes {
        tag: String,
        target: String,
        since: String,
    },
}

fn numeric(repo: &Repo, id: &str) -> Result<()> {
    if id.parse::<u64>().is_ok_and(|id| id > 0) {
        Ok(())
    } else {
        Err(failure(repo, "Choose a host release or file."))
    }
}
pub fn write(repo: &Repo, token: &str, action: &Write, confirmed: bool) -> Result<Value> {
    write_using(repo, action, confirmed, |method, url, body| match method {
        "PATCH" => http::patch_json(url, token, &repo.host, body),
        "PUT" => http::put_json(url, token, &repo.host, body),
        "DELETE" => http::delete(url, token, &repo.host),
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
            "Confirm this release action before sending it.",
        ));
    }
    let root = root(repo)?;
    let github = repo.kind == Kind::GitHub;
    let (method, path, body) = match action {
        Write::Save { release, publish } => {
            if release.tag.trim().is_empty()
                || release.tag.len() > 256
                || release.target.trim().is_empty()
                || release.body.len() > 125000
                || release.title.len() > 256
            {
                return Err(failure(
                    repo,
                    "Choose a tag, target and title within the host's size limits.",
                ));
            }
            if !github && !publish {
                return Err(failure(repo, "GitLab drafts must be saved locally."));
            }
            let existing = !release.id.is_empty() && !release.local;
            if github && existing {
                numeric(repo, &release.id)?;
            }
            let path = if existing {
                format!("releases/{}", enc(&release.id))
            } else {
                "releases".into()
            };
            let body = if github {
                json!({"tag_name":release.tag,"target_commitish":release.target,"name":release.title,"body":release.body,"draft":!publish,"prerelease":release.prerelease,"make_latest":if *publish && !release.prerelease && release.latest {"true"}else{"false"}})
            } else {
                let mut body = json!({"tag_name":release.tag,"ref":release.target,"name":release.title,"description":release.body});
                if !existing {
                    body["assets"] = json!({"links":release.assets.iter().filter(|a| a.removable).map(|a| json!({"name":a.name,"url":a.url})).collect::<Vec<_>>()});
                }
                body
            };
            (
                if existing {
                    if github {
                        "PATCH"
                    } else {
                        "PUT"
                    }
                } else {
                    "POST"
                },
                path,
                body,
            )
        }
        Write::Delete { id, tag } => {
            if github {
                numeric(repo, id)?;
            }
            (
                "DELETE",
                format!("releases/{}", enc(if github { id } else { tag })),
                Value::Null,
            )
        }
        Write::DeleteTag { tag } => {
            if tag.is_empty() {
                return Err(failure(repo, "Choose a tag to delete."));
            }
            (
                "DELETE",
                if github {
                    format!("git/refs/tags/{}", enc(tag))
                } else {
                    format!("repository/tags/{}", enc(tag))
                },
                Value::Null,
            )
        }
        Write::RemoveAsset {
            release_id: _,
            tag,
            asset_id,
        } => {
            numeric(repo, asset_id)?;
            (
                "DELETE",
                if github {
                    format!("releases/assets/{asset_id}")
                } else {
                    format!("releases/{}/assets/links/{asset_id}", enc(tag))
                },
                Value::Null,
            )
        }
        Write::LinkAsset { tag, name, url } => {
            if github || name.trim().is_empty() || !url.starts_with("https://") {
                return Err(failure(repo, "Choose a GitLab file with an HTTPS link."));
            }
            (
                "POST",
                format!("releases/{}/assets/links", enc(tag)),
                json!({"name":name,"url":url}),
            )
        }
        Write::GenerateNotes { tag, target, since } => {
            if !github {
                return Err(failure(
                    repo,
                    "Host-generated notes are available on GitHub.",
                ));
            }
            let mut body = json!({"tag_name":tag,"target_commitish":target});
            if !since.is_empty() {
                body["previous_tag_name"] = since.clone().into();
            }
            ("POST", "releases/generate-notes".into(), body)
        }
    };
    match send(method, &format!("{root}/{path}"), &body.to_string()) {
        Err(Error::ForgeOffline { .. }) => Err(failure(
            repo,
            "The host did not confirm delivery. Refresh before trying this action again.",
        )),
        Err(error) => Err(error),
        Ok(response) => checked(repo, response),
    }
}

/// The destination is derived from the connected repository, never a URL from
/// a release body or a frontend parameter. No token follows a redirect.
pub fn upload_url(repo: &Repo, release: &Release, name: &str) -> Result<(String, bool)> {
    if name.is_empty() || name.contains(['/', '\\', '\0']) {
        return Err(failure(repo, "Choose a file name without folders."));
    }
    if repo.kind == Kind::GitHub {
        numeric(repo, &release.id)?;
        let base = if repo.host == "github.com" {
            "https://uploads.github.com".into()
        } else {
            format!("https://{}/api/uploads", repo.host)
        };
        Ok((
            format!(
                "{base}/repos/{}/{}/releases/{}/assets?name={}",
                enc(&repo.owner),
                enc(&repo.name),
                release.id,
                enc(name)
            ),
            false,
        ))
    } else {
        Ok((
            format!(
                "{}/packages/generic/spagitty-release/{}/{}",
                root(repo)?,
                enc(&release.tag),
                enc(name)
            ),
            true,
        ))
    }
}

pub fn notes(records: &str) -> String {
    let mut groups: [Vec<String>; 4] = Default::default();
    for record in records.split('\u{1e}') {
        let fields: Vec<_> = record.trim().split('\u{1f}').collect();
        if fields.len() < 3 {
            continue;
        }
        let parsed = crate::hooks::blocks::conventional(fields[2]);
        let index = match parsed.as_ref().map(|p| p.kind) {
            Some("feat") => 0,
            Some("fix") => 1,
            Some("refactor" | "perf") => 2,
            _ => 3,
        };
        let subject = parsed.map(|p| p.subject).unwrap_or(fields[2]);
        groups[index].push(format!("- {subject} ({})", fields[1]));
    }
    groups
        .into_iter()
        .zip(["New", "Fixed", "Changed", "Other"])
        .filter(|(lines, _)| !lines.is_empty())
        .map(|(lines, heading)| format!("### {heading}\n\n{}", lines.join("\n")))
        .collect::<Vec<_>>()
        .join("\n\n")
}

pub fn target(repo: &Repo, token: &str, revision: &str) -> Result<String> {
    if revision.is_empty() {
        return Err(failure(repo, "Choose a target branch or commit."));
    }
    let raw = get(
        repo,
        token,
        &format!(
            "{}{}/{}",
            if repo.kind == Kind::GitHub {
                ""
            } else {
                "repository/"
            },
            "commits",
            enc(revision)
        ),
    )?;
    raw[if repo.kind == Kind::GitHub {
        "sha"
    } else {
        "id"
    }]
    .as_str()
    .filter(|id| id.len() >= 40)
    .map(str::to_string)
    .ok_or_else(|| failure(repo, "The host did not resolve the target commit."))
}

pub fn tag_target(repo: &Repo, token: &str, tag: &str) -> Result<Option<String>> {
    let path = if repo.kind == Kind::GitHub {
        format!("git/ref/tags/{}", enc(tag))
    } else {
        format!("repository/tags/{}", enc(tag))
    };
    let response = http::get_json(&format!("{}/{path}", root(repo)?), token, &repo.host)?;
    if response.status == 404 {
        return Ok(None);
    }
    let raw = checked(repo, response)?;
    if repo.kind == Kind::GitLab {
        return Ok(raw["commit"]["id"].as_str().map(str::to_string));
    }
    let mut object = raw["object"].clone();
    for _ in 0..8 {
        if object["type"] == "commit" {
            return Ok(object["sha"].as_str().map(str::to_string));
        }
        let id = object["sha"]
            .as_str()
            .filter(|id| id.len() >= 40 && id.bytes().all(|c| c.is_ascii_hexdigit()))
            .ok_or_else(|| failure(repo, "The host sent an unreadable tag target."))?;
        object = get(repo, token, &format!("git/tags/{id}"))?["object"].clone();
    }
    Err(failure(repo, "The annotated tag chain is too deep."))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Artifact {
    pub id: String,
    pub name: String,
    pub size: Option<u64>,
    pub run: String,
    pub commit: String,
}

pub fn artifacts(repo: &Repo, token: &str, commit: &str) -> Result<Vec<Artifact>> {
    if commit.len() < 40 || !commit.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(failure(
            repo,
            "Resolve the target commit before choosing artifacts.",
        ));
    }
    let github = repo.kind == Kind::GitHub;
    let raw = get(
        repo,
        token,
        &if github {
            format!("actions/runs?head_sha={commit}&status=completed&per_page=50")
        } else {
            format!("pipelines?sha={commit}&per_page=50")
        },
    )?;
    let runs = if github { &raw["workflow_runs"] } else { &raw };
    let mut result = Vec::new();
    for run in runs.as_array().into_iter().flatten() {
        if !github
            && !matches!(
                run["status"].as_str(),
                Some("success" | "failed" | "canceled" | "skipped")
            )
        {
            continue;
        }
        let Some(id) = run["id"].as_u64() else {
            continue;
        };
        for page in 1..=100 {
            let raw = get(
                repo,
                token,
                &if github {
                    format!("actions/runs/{id}/artifacts?per_page=100&page={page}")
                } else {
                    format!("pipelines/{id}/jobs?per_page=100&page={page}")
                },
            )?;
            let rows = if github { &raw["artifacts"] } else { &raw };
            let rows = rows
                .as_array()
                .ok_or_else(|| failure(repo, "The host did not send an artifact list."))?;
            for row in rows {
                if github && row["expired"].as_bool() == Some(true) {
                    continue;
                }
                if !github && row["artifacts_file"]["filename"].as_str().is_none() {
                    continue;
                }
                if let Some(artifact) = row["id"].as_u64() {
                    result.push(Artifact {
                        id: artifact.to_string(),
                        name: if github {
                            format!("{}.zip", text(row, "name"))
                        } else {
                            text(&row["artifacts_file"], "filename")
                        },
                        size: if github {
                            row["size_in_bytes"].as_u64()
                        } else {
                            row["artifacts_file"]["size"].as_u64()
                        },
                        run: format!("{} #{id}", text(run, if github { "name" } else { "ref" })),
                        commit: commit.into(),
                    });
                }
            }
            if rows.len() < 100 {
                break;
            }
        }
    }
    Ok(result)
}

pub fn artifact_url(repo: &Repo, id: &str) -> Result<String> {
    numeric(repo, id)?;
    Ok(format!(
        "{}/{}",
        root(repo)?,
        if repo.kind == Kind::GitHub {
            format!("actions/artifacts/{id}/zip")
        } else {
            format!("jobs/{id}/artifacts")
        }
    ))
}

/// Only a clean, checked-out branch may receive the ordinary changelog commit.
/// A hook failure leaves its edits and staged file visible for the person to fix.
pub fn commit_changelog(
    path: &std::path::Path,
    branch: &str,
    expected: &str,
    tag: &str,
    body: &str,
) -> Result<String> {
    let repo = crate::repo::open(path)?;
    let current = repo
        .head_name()
        .map_err(|e| Error::Config(e.to_string()))?
        .map(|name| name.as_bstr().to_string())
        .unwrap_or_default();
    if current != format!("refs/heads/{branch}")
        || crate::shell::commit_id(path, "HEAD")? != expected
    {
        return Err(Error::NotStageable(
            "Check out the target branch at the reviewed commit before writing the changelog."
                .into(),
        ));
    }
    if crate::status::working_copy(&repo)?.changed_paths() != 0 {
        return Err(Error::NotStageable(
            "Commit or set aside existing work before writing the release changelog.".into(),
        ));
    }
    if !crate::hooks::enabled(path)? {
        return Err(Error::NotStageable(
            "Enable repository hooks before committing the release changelog.".into(),
        ));
    }
    if tag.trim().is_empty() || tag.contains(['\n', '\r']) {
        return Err(Error::NotStageable(
            "Choose a release tag without line breaks.".into(),
        ));
    }
    let file = path.join("CHANGELOG.md");
    if std::fs::symlink_metadata(&file).is_ok_and(|metadata| metadata.file_type().is_symlink()) {
        return Err(Error::NotStageable(
            "The changelog must be a regular file inside this repository.".into(),
        ));
    }
    let previous = match std::fs::read_to_string(&file) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(error) => return Err(error.into()),
    };
    let section = format!("## [{tag}]\n\n{}\n\n", body.trim());
    let at = previous
        .match_indices("\n## [")
        .find(|(at, _)| !previous[at + 1..].starts_with("## [Unreleased]"))
        .map(|(at, _)| at);
    let next = if let Some(at) = at {
        format!(
            "{}\n\n{section}{}",
            previous[..at].trim_end(),
            &previous[at + 1..]
        )
    } else if previous.is_empty() {
        format!("# Changelog\n\n{section}")
    } else {
        format!("{}\n\n{section}", previous.trim_end())
    };
    std::fs::write(file, next)?;
    crate::work::stage(&repo, &["CHANGELOG.md".into()])?;
    crate::work::commit(&repo, &format!("docs: changelog for {tag}"), "", false)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn repo(kind: Kind) -> Repo {
        Repo {
            kind,
            host: "example.org".into(),
            owner: "team".into(),
            name: "app".into(),
        }
    }

    #[test]
    fn hosts_preserve_assets_and_only_github_has_download_counts() {
        let github = read_release(
            &json!({"id":12,"tag_name":"v1","draft":true,"assets":[{"id":8,"name":"app.zip","size":32,"download_count":9}]}),
            Kind::GitHub,
        );
        assert!(github.draft);
        assert_eq!(github.assets[0].downloads, Some(9));
        let gitlab = read_release(
            &json!({"tag_name":"v1","assets":{"links":[{"id":8,"name":"app.zip","url":"https://example.org/file"}],"sources":[{"format":"zip","url":"https://example.org/source"}]}}),
            Kind::GitLab,
        );
        assert!(!gitlab.draft);
        assert_eq!(gitlab.assets[0].downloads, None);
        assert!(!gitlab.assets[1].removable);
    }

    #[test]
    fn gitlab_latest_is_the_newest_release_already_out() {
        let rows = [
            json!({"tag_name":"v3","upcoming_release":true}),
            json!({"tag_name":"v2","upcoming_release":false}),
            json!({"tag_name":"v1"}),
        ];
        assert_eq!(gitlab_latest(&rows), Some(1));
        assert_eq!(gitlab_latest(&rows[2..]), Some(0));
        assert_eq!(gitlab_latest(&rows[..1]), None);
    }

    #[test]
    fn release_deletion_does_not_delete_a_tag_and_every_write_needs_consent() {
        for kind in [Kind::GitHub, Kind::GitLab] {
            let repo = repo(kind);
            let action = Write::Delete {
                id: "12".into(),
                tag: "v1".into(),
            };
            assert!(write_using(&repo, &action, false, |_, _, _| panic!("no consent")).is_err());
            let mut calls = 0;
            write_using(&repo, &action, true, |method, url, _| {
                calls += 1;
                assert_eq!(method, "DELETE");
                assert!(url.contains("/releases/"));
                Ok(http::Response {
                    status: 204,
                    body: String::new(),
                    retry_after: None,
                })
            })
            .unwrap();
            assert_eq!(calls, 1);
        }
    }

    #[test]
    fn publishing_uses_each_hosts_fields_and_uncertain_delivery_is_one_attempt() {
        for kind in [Kind::GitHub, Kind::GitLab] {
            let repo = repo(kind);
            let release = Release {
                tag: "v2".into(),
                target: "a".repeat(40),
                title: "Second".into(),
                body: "notes".into(),
                draft: true,
                latest: true,
                ..Release::default()
            };
            let mut calls = 0;
            let result = write_using(
                &repo,
                &Write::Save {
                    release,
                    publish: true,
                },
                true,
                |method, url, body| {
                    calls += 1;
                    assert_eq!(method, "POST");
                    assert!(url.ends_with("/releases"));
                    let body: Value = serde_json::from_str(body).unwrap();
                    assert_eq!(body["tag_name"], "v2");
                    if kind == Kind::GitHub {
                        assert_eq!(body["draft"], false);
                        assert_eq!(body["make_latest"], "true");
                    } else {
                        assert_eq!(body["ref"], "a".repeat(40));
                        assert!(body.get("draft").is_none());
                    }
                    Err(Error::ForgeOffline {
                        host: repo.host.clone(),
                        detail: "lost".into(),
                    })
                },
            );
            assert_eq!(calls, 1);
            assert!(result.unwrap_err().to_string().contains("Refresh"));
        }
    }

    #[test]
    fn gitlab_drafts_stay_local_and_uploads_cannot_choose_another_host() {
        let release = Release {
            id: "12".into(),
            tag: "v1".into(),
            target: "abc".into(),
            ..Release::default()
        };
        assert!(write_using(
            &repo(Kind::GitLab),
            &Write::Save {
                release: release.clone(),
                publish: false
            },
            true,
            |_, _, _| panic!("local draft")
        )
        .is_err());
        assert!(upload_url(&repo(Kind::GitHub), &release, "../secret").is_err());
        assert!(upload_url(&repo(Kind::GitLab), &release, "app.zip")
            .unwrap()
            .0
            .contains("example.org/api/v4/projects/"));
    }

    #[test]
    fn commit_notes_keep_request_numbers_and_group_the_shared_parser() {
        let body = notes("abcdef\u{1f}abc\u{1f}feat(api): add refunds (#12)\u{1f}1\u{1e}defghi\u{1f}def\u{1f}fix: stop duplicate (#13)\u{1f}2\u{1e}");
        assert!(body.contains("### New\n\n- add refunds (#12) (abc)"));
        assert!(body.contains("### Fixed"));
    }

    #[test]
    fn changelog_uses_the_commit_path_runs_hooks_and_refuses_unrelated_work() {
        let fixture = crate::fixture::Fixture::linear(1);
        let head = crate::shell::commit_id(fixture.path(), "HEAD").unwrap();
        fixture.write("personal.txt", "work");
        assert!(commit_changelog(fixture.path(), "main", &head, "v2", "notes").is_err());
        fixture.remove("personal.txt");
        let hook = fixture.path().join(".git/hooks/pre-commit");
        std::fs::write(&hook, "#!/bin/sh\nprintf ran > hook-ran\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&hook, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        let next = commit_changelog(
            fixture.path(),
            "main",
            &head,
            "v2",
            "### New\n\n- A feature",
        )
        .unwrap();
        assert_ne!(next, head);
        assert!(fixture.path().join("hook-ran").exists());
        assert!(
            crate::shell::show_text(fixture.path(), &next, "CHANGELOG.md")
                .unwrap()
                .unwrap()
                .contains("## [v2]")
        );
        assert!(commit_changelog(fixture.path(), "main", &head, "v3", "notes").is_err());
    }
}
