// SPDX-License-Identifier: GPL-3.0-or-later

//! Each send, kept per person and per repository in application data, never
//! in the repository (proposal §5.3).
//!
//! The request is kept as it was sent with secrets and kept values as their
//! `{{name}}`; the response's status, time, size and headers, and its body up
//! to [`BODY`]. An index lists the sends, newest first, and each body is its
//! own file beside it, so trimming the oldest is deleting files rather than
//! rewriting one large one.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::calls::build::Shown;
use crate::calls::store::write_text;
use crate::Result;

/// The most of a response body kept: 1 MB.
pub const BODY: usize = 1024 * 1024;
/// The most of a request body kept in the index.
const REQUEST_BODY: usize = 64 * 1024;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    pub id: String,
    /// Milliseconds since 1970.
    pub at: i64,
    pub call_id: Option<String>,
    /// The collection's file stem.
    pub collection: Option<String>,
    pub name: String,
    pub environment: Option<String>,
    pub request: Shown,
    /// Absent when nothing came back.
    pub status: Option<u16>,
    #[serde(default)]
    pub reason: String,
    #[serde(default)]
    pub millis: u64,
    /// The whole body's size, even where less was kept.
    #[serde(default)]
    pub size: u64,
    #[serde(default)]
    pub headers: Vec<(String, String)>,
    /// Why nothing came back.
    #[serde(default)]
    pub error: Option<String>,
    /// The body kept was cut at [`BODY`].
    #[serde(default)]
    pub cut: bool,
}

#[derive(Debug, Clone)]
pub struct History {
    dir: PathBuf,
}

impl History {
    pub fn new(dir: &Path) -> Self {
        History {
            dir: dir.to_path_buf(),
        }
    }

    fn index(&self) -> PathBuf {
        self.dir.join("index.json")
    }

    fn body_file(&self, id: &str) -> PathBuf {
        self.dir.join(format!("{}.body", safe(id)))
    }

    /// Every send, newest first. An index that cannot be read is no history.
    pub fn list(&self) -> Vec<Entry> {
        std::fs::read_to_string(self.index())
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default()
    }

    /// Record a send and drop the oldest past `cap`. Secrets are expected to
    /// have been taken out of `entry.request` already; `private` is cut out
    /// of the response body and headers as well, in case a server echoed one.
    pub fn add(
        &self,
        mut entry: Entry,
        body: &[u8],
        cap: usize,
        private: &[String],
    ) -> Result<Entry> {
        std::fs::create_dir_all(&self.dir)?;
        let mut kept = body[..body.len().min(BODY)].to_vec();
        entry.cut = body.len() > BODY;
        if let Some(text) = entry.request.body.as_mut() {
            if text.len() > REQUEST_BODY {
                let mut end = REQUEST_BODY;
                while !text.is_char_boundary(end) {
                    end -= 1;
                }
                text.truncate(end);
                text.push('…');
            }
        }
        if !private.is_empty() {
            if let Ok(text) = String::from_utf8(kept.clone()) {
                kept = redact(&text, private).into_bytes();
            }
            for (_, value) in entry.headers.iter_mut() {
                *value = redact(value, private);
            }
            if let Some(error) = entry.error.as_mut() {
                *error = redact(error, private);
            }
        }
        std::fs::write(self.body_file(&entry.id), &kept)?;

        let mut all = self.list();
        all.insert(0, entry.clone());
        for gone in all.iter().skip(cap.max(1)) {
            let _ = std::fs::remove_file(self.body_file(&gone.id));
        }
        all.truncate(cap.max(1));
        write_text(
            &self.index(),
            &serde_json::to_string(&all).unwrap_or_default(),
        )?;
        Ok(entry)
    }

    /// The body kept for a send.
    pub fn body(&self, id: &str) -> Option<Vec<u8>> {
        std::fs::read(self.body_file(id)).ok()
    }

    /// The newest send of a call, for *Last response* in Share (FEAT-164).
    pub fn last_of(&self, call_id: &str) -> Option<Entry> {
        self.list()
            .into_iter()
            .find(|entry| entry.call_id.as_deref() == Some(call_id) && entry.status.is_some())
    }

    pub fn clear(&self) -> Result<()> {
        match std::fs::remove_dir_all(&self.dir) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e.into()),
        }
    }
}

/// `text` with each of `private` replaced by dots.
pub fn redact(text: &str, private: &[String]) -> String {
    let mut out = text.to_string();
    for value in private.iter().filter(|value| value.len() >= 4) {
        out = out.replace(value.as_str(), "••••");
    }
    out
}

fn safe(id: &str) -> String {
    id.chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-')
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(id: &str, call: &str) -> Entry {
        Entry {
            id: id.into(),
            at: 1,
            call_id: Some(call.into()),
            name: "Log in".into(),
            status: Some(200),
            ..Entry::default()
        }
    }

    #[test]
    fn sends_are_kept_newest_first_and_trimmed_to_the_cap() {
        let dir = tempfile::tempdir().unwrap();
        let history = History::new(dir.path());
        for n in 0..5 {
            history
                .add(entry(&format!("h{n}"), "c_1"), b"{}", 3, &[])
                .unwrap();
        }
        let ids: Vec<_> = history.list().into_iter().map(|e| e.id).collect();
        assert_eq!(ids, vec!["h4", "h3", "h2"]);
        assert!(history.body("h4").is_some());
        assert!(history.body("h0").is_none());
        assert_eq!(history.last_of("c_1").unwrap().id, "h4");
        history.clear().unwrap();
        assert!(history.list().is_empty());
    }

    #[test]
    fn a_body_is_kept_to_a_megabyte_and_a_secret_echoed_back_is_cut() {
        let dir = tempfile::tempdir().unwrap();
        let history = History::new(dir.path());
        let big = vec![b'x'; BODY + 10];
        let kept = history.add(entry("h1", "c"), &big, 10, &[]).unwrap();
        assert!(kept.cut);
        assert_eq!(history.body("h1").unwrap().len(), BODY);

        let mut echoed = entry("h2", "c");
        echoed.headers.push(("X-Token".into(), "hunter22".into()));
        history
            .add(
                echoed,
                b"{\"password\":\"hunter22\"}",
                10,
                &["hunter22".to_string()],
            )
            .unwrap();
        let body = String::from_utf8(history.body("h2").unwrap()).unwrap();
        assert!(!body.contains("hunter22"));
        assert_eq!(history.list()[0].headers[0].1, "••••");
    }
}
