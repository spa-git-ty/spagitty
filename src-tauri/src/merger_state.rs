// SPDX-License-Identifier: GPL-3.0-or-later

//! The choices made while resolving a merge in Merger, kept between visits
//! (FEAT-102).
//!
//! They are the person's working state, not the project's, so they live with
//! Spagitty's application data rather than in the repository — the same
//! reasoning as a review's state (`review_state.rs`), and the same writing:
//! a temporary file and a rename.
//!
//! One small JSON file per merge, named by a key the screen derives from the
//! repository, both branches and their merge base:
//!
//! ```text
//! <app data>/merges/<key>.json
//! ```
//!
//! The key arrives from the webview, so it is accepted only as what it is
//! meant to be — a short run of lowercase hex — and never as a path.

use std::path::{Path, PathBuf};

/// More than any merge's choices need: a thousand hand edits of a page each.
const MAX_BYTES: usize = 4 * 1024 * 1024;

/// Where one merge's choices are kept under `root`, or `None` for a key that
/// is not lowercase hex of a sensible length.
pub fn path_in(root: &Path, key: &str) -> Option<PathBuf> {
    let ok = (8..=64).contains(&key.len())
        && key
            .chars()
            .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c));
    ok.then(|| root.join("merges").join(format!("{key}.json")))
}

pub use crate::review_state::{forget, read, root};

/// Store `value`, replacing what was there.
pub fn write(path: &Path, value: &serde_json::Value) -> Result<(), String> {
    if !value.is_object() {
        return Err("a merge's choices are an object".into());
    }
    let text = serde_json::to_string(value).map_err(|e| e.to_string())?;
    if text.len() > MAX_BYTES {
        return Err("this merge's choices are too large to keep".into());
    }
    let dir = path.parent().ok_or("no folder to keep the choices in")?;
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let partial = path.with_extension("json.partial");
    std::fs::write(&partial, text).map_err(|e| e.to_string())?;
    std::fs::rename(&partial, path).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_key_is_a_file_under_merges() {
        assert_eq!(
            path_in(Path::new("/data"), "0123abcd").unwrap(),
            Path::new("/data/merges/0123abcd.json")
        );
    }

    #[test]
    fn anything_but_short_lowercase_hex_is_refused() {
        for key in [
            "",
            "abc",
            "../../etc",
            "ABCDEF12",
            "0123abcd/..",
            &"a".repeat(65),
        ] {
            assert!(path_in(Path::new("/data"), key).is_none(), "{key}");
        }
    }

    #[test]
    fn choices_read_back_and_a_non_object_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let path = path_in(dir.path(), "0123abcd").unwrap();
        let value = serde_json::json!({ "version": 1, "choices": {} });
        write(&path, &value).unwrap();
        assert_eq!(read(&path), Some(value));
        assert!(write(&path, &serde_json::json!([1])).is_err());
        forget(&path).unwrap();
        assert_eq!(read(&path), None);
    }
}
