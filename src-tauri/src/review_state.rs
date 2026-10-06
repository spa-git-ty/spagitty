// SPDX-License-Identifier: GPL-3.0-or-later

//! What a reviewer has done on one pull request, kept between sessions
//! (FEAT-087).
//!
//! Which files were ticked as viewed, and at which version; comments written
//! but not yet sent. It is the reviewer's own working state, so it lives with
//! Spagitty's application data rather than in the repository — nothing here is
//! the project's, and a clone somewhere else is the same pull request.
//!
//! One small JSON file per pull request, under the host, owner and name it
//! belongs to:
//!
//! ```text
//! <app data>/reviews/github.com/owner/name/214.json
//! ```
//!
//! The screen owns the shape; this module stores a JSON value and refuses one
//! that is not an object or that is larger than any review needs. The path is
//! built from parts that are checked one by one, because they arrive from the
//! webview and a `..` in any of them would be a write anywhere on the disk.

use std::path::{Path, PathBuf};

use tauri::{AppHandle, Manager, Runtime};

/// More than a review with a thousand pending comments would take. A value
/// past this is a bug in the screen, not a review.
const MAX_BYTES: usize = 4 * 1024 * 1024;

/// Where one pull request's state is kept, under `root`.
///
/// `None` when any part is not a plain name: empty, a dot or two, or carrying
/// a separator. GitLab's nested groups (`team/sub`) arrive as one owner with a
/// slash, which is split into names that are each checked.
pub fn path_in(root: &Path, host: &str, owner: &str, name: &str, number: u64) -> Option<PathBuf> {
    let mut path = root.join("reviews");
    path.push(plain(host)?);
    for part in owner.split('/') {
        path.push(plain(part)?);
    }
    path.push(plain(name)?);
    path.push(format!("{number}.json"));
    Some(path)
}

/// `part`, when it is a single ordinary file name.
fn plain(part: &str) -> Option<&str> {
    let ok = !part.is_empty()
        && part != "."
        && part != ".."
        && !part.contains(['/', '\\', ':', '\0'])
        && !part.starts_with('.');
    ok.then_some(part)
}

/// The stored state, or `None` when there is none or it cannot be read.
///
/// An unreadable file is the same as no file: a review that starts over is a
/// nuisance, and a screen that will not open is worse.
pub fn read(path: &Path) -> Option<serde_json::Value> {
    let text = std::fs::read_to_string(path).ok()?;
    let value: serde_json::Value = serde_json::from_str(&text).ok()?;
    value.is_object().then_some(value)
}

/// Store `value`, replacing what was there. Written to a temporary file and
/// renamed, so a crash mid-write leaves the old state rather than half of one.
pub fn write(path: &Path, value: &serde_json::Value) -> Result<(), String> {
    if !value.is_object() {
        return Err("a review's state is an object".into());
    }
    let text = serde_json::to_string(value).map_err(|e| e.to_string())?;
    if text.len() > MAX_BYTES {
        return Err("this review's state is too large to keep".into());
    }

    let dir = path.parent().ok_or("no folder to keep the review in")?;
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let partial = path.with_extension("json.partial");
    std::fs::write(&partial, text).map_err(|e| e.to_string())?;
    std::fs::rename(&partial, path).map_err(|e| e.to_string())
}

/// Forget one pull request's state. Gone already is not an error.
pub fn forget(path: &Path) -> Result<(), String> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.to_string()),
    }
}

/// The application's data folder, where reviews are kept.
pub fn root<R: Runtime>(app: &AppHandle<R>) -> Option<PathBuf> {
    app.path().app_data_dir().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_pull_request_is_kept_under_its_host_owner_and_name() {
        let path = path_in(Path::new("/data"), "github.com", "team", "spagitty", 214).unwrap();
        assert_eq!(
            path,
            Path::new("/data/reviews/github.com/team/spagitty/214.json")
        );
    }

    #[test]
    fn a_nested_gitlab_group_becomes_nested_folders() {
        let path = path_in(Path::new("/data"), "gitlab.com", "team/sub", "app", 7).unwrap();
        assert_eq!(
            path,
            Path::new("/data/reviews/gitlab.com/team/sub/app/7.json")
        );
    }

    #[test]
    fn a_part_that_could_climb_out_of_the_folder_is_refused() {
        for (host, owner, name) in [
            ("..", "team", "app"),
            ("github.com", "../..", "app"),
            ("github.com", "team", "a/b"),
            ("github.com", "team", "a\\b"),
            ("github.com", "", "app"),
            ("github.com", "team", ".hidden"),
            ("c:", "team", "app"),
        ] {
            assert!(
                path_in(Path::new("/data"), host, owner, name, 1).is_none(),
                "{host} {owner} {name}"
            );
        }
    }

    #[test]
    fn state_reads_back_as_it_was_written_and_forgetting_it_twice_is_fine() {
        let dir = std::env::temp_dir().join(format!("spagitty-review-{}", std::process::id()));
        let path = path_in(&dir, "github.com", "team", "app", 9).unwrap();
        let value = serde_json::json!({ "headSha": "abc", "viewed": { "a.rs": "blob" } });

        assert_eq!(read(&path), None);
        write(&path, &value).unwrap();
        assert_eq!(read(&path), Some(value));
        forget(&path).unwrap();
        forget(&path).unwrap();
        assert_eq!(read(&path), None);

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn something_that_is_not_an_object_is_neither_written_nor_read() {
        let dir = std::env::temp_dir().join(format!("spagitty-review-x-{}", std::process::id()));
        let path = path_in(&dir, "github.com", "team", "app", 9).unwrap();

        assert!(write(&path, &serde_json::json!([1, 2])).is_err());
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, "[1,2]").unwrap();
        assert_eq!(read(&path), None);
        std::fs::write(&path, "not json").unwrap();
        assert_eq!(read(&path), None);

        let _ = std::fs::remove_dir_all(dir);
    }
}
