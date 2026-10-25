// SPDX-License-Identifier: GPL-3.0-or-later

//! The notification center's record (FEAT-142).
//!
//! The watcher, the farm, the checks and the network all say things once, in a
//! toast or the operating system's centre, and before this they were gone the
//! moment they had been said. Every one of them is also written here, and the
//! drawer reads it back.
//!
//! Application state, like `settings.json` and `repositories.json`, so it lives
//! in the configuration directory beside them and not in any repository: a
//! notification is about a repository, but it is the person's, and one drawer
//! spans every repository they have open.
//!
//! # Written whole
//!
//! A JSON array, rewritten atomically after each change. A few thousand entries
//! are a few hundred kilobytes, and an append-only log would need compacting to
//! record that something was read. Kept until the person clears it.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, Runtime, State};

const FILE: &str = "notifications.json";

/// Which chip of the drawer an entry is under.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Category {
    PullRequests,
    Farm,
    Checks,
    Git,
}

/// Where opening an entry goes: a screen of the application, or a page.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Target {
    Route { route: String },
    Url { url: String },
    None,
}

/// One notification, as it was said.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    pub id: String,
    /// The repository's path, when it is about one. Empty for an account-wide
    /// notification, such as a pull request on a host with no checkout here.
    #[serde(default)]
    pub repository: String,
    /// What a person reads: the folder's name, or the host's `owner/name`.
    #[serde(default)]
    pub repository_name: String,
    pub category: Category,
    /// The kind within the category — `merged`, `taskStuck`, `fetchFailed` —
    /// which chooses the icon, the colour and the action.
    pub kind: String,
    pub title: String,
    #[serde(default)]
    pub body: String,
    #[serde(default = "no_target")]
    pub target: Target,
    pub created_ms: u64,
    #[serde(default)]
    pub read_ms: Option<u64>,
}

fn no_target() -> Target {
    Target::None
}

/// What the screen sends: an entry without the parts the record assigns.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewEntry {
    #[serde(default)]
    pub repository: String,
    #[serde(default)]
    pub repository_name: String,
    pub category: Category,
    pub kind: String,
    pub title: String,
    #[serde(default)]
    pub body: String,
    #[serde(default = "no_target")]
    pub target: Target,
}

/// The record, in memory, and where it is written.
pub struct Inbox {
    path: Option<PathBuf>,
    entries: Vec<Entry>,
    sequence: u64,
}

impl Inbox {
    /// Read the record at `path`. A missing or unreadable file is an empty
    /// record: losing the drawer must never stop the application starting.
    pub fn open(path: Option<PathBuf>) -> Self {
        let entries = path
            .as_deref()
            .and_then(|path| std::fs::read_to_string(path).ok())
            .and_then(|text| serde_json::from_str::<Vec<Entry>>(&text).ok())
            .unwrap_or_default();
        Inbox {
            path,
            entries,
            sequence: 0,
        }
    }

    /// Every entry, newest first.
    pub fn list(&self) -> Vec<Entry> {
        let mut entries = self.entries.clone();
        entries.sort_by(|a, b| b.created_ms.cmp(&a.created_ms).then(b.id.cmp(&a.id)));
        entries
    }

    pub fn add(&mut self, new: NewEntry, now: u64) -> Entry {
        self.sequence += 1;
        let entry = Entry {
            id: format!("n-{now}-{}", self.sequence),
            repository: new.repository,
            repository_name: new.repository_name,
            category: new.category,
            kind: new.kind,
            title: new.title,
            body: new.body,
            target: new.target,
            created_ms: now,
            read_ms: None,
        };
        self.entries.push(entry.clone());
        self.save();
        entry
    }

    /// Mark these read. Already read ones keep the time they were read.
    pub fn read(&mut self, ids: &[String], now: u64) {
        let mut changed = false;
        for entry in &mut self.entries {
            if entry.read_ms.is_none() && ids.contains(&entry.id) {
                entry.read_ms = Some(now);
                changed = true;
            }
        }
        if changed {
            self.save();
        }
    }

    /// Mark everything read, or everything about one repository.
    pub fn read_all(&mut self, repository: Option<&str>, now: u64) {
        let mut changed = false;
        for entry in &mut self.entries {
            let about = match repository {
                Some(repository) => entry.repository == repository,
                None => true,
            };
            if about && entry.read_ms.is_none() {
                entry.read_ms = Some(now);
                changed = true;
            }
        }
        if changed {
            self.save();
        }
    }

    /// Forget everything said before `before`, about one repository or all
    /// of them. Returns how many went.
    pub fn clear(&mut self, before: u64, repository: Option<&str>) -> usize {
        let was = self.entries.len();
        self.entries.retain(|entry| {
            let about = match repository {
                Some(repository) => entry.repository == repository,
                None => true,
            };
            !about || entry.created_ms >= before
        });
        let gone = was - self.entries.len();
        if gone > 0 {
            self.save();
        }
        gone
    }

    fn save(&self) {
        let Some(path) = &self.path else {
            return;
        };
        if let Ok(text) = serde_json::to_string(&self.entries) {
            // A failed write costs the record of this change, not the session.
            let _ = write_atomically(path, &text);
        }
    }
}

/// Through a temporary file in the same directory and a rename, so a crash
/// mid-write leaves the last good record rather than half of one.
fn write_atomically(path: &Path, text: &str) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let temporary = path.with_extension("json.tmp");
    std::fs::write(&temporary, text)?;
    std::fs::rename(&temporary, path)
}

/// The record, opened the first time a command asks for it.
#[derive(Default)]
pub struct InboxState(Mutex<Option<Inbox>>);

impl InboxState {
    fn with<R: Runtime, T>(&self, app: &AppHandle<R>, run: impl FnOnce(&mut Inbox) -> T) -> T {
        let mut guard = self.0.lock().expect("inbox lock");
        let inbox = guard.get_or_insert_with(|| {
            Inbox::open(app.path().app_config_dir().ok().map(|dir| dir.join(FILE)))
        });
        run(inbox)
    }
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_millis() as u64)
        .unwrap_or(0)
}

#[tauri::command(async)]
pub fn inbox_list<R: Runtime>(app: AppHandle<R>, state: State<'_, InboxState>) -> Vec<Entry> {
    state.with(&app, |inbox| inbox.list())
}

#[tauri::command(async)]
pub fn inbox_add<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, InboxState>,
    entry: NewEntry,
) -> Entry {
    state.with(&app, |inbox| inbox.add(entry, now_ms()))
}

#[tauri::command(async)]
pub fn inbox_read<R: Runtime>(app: AppHandle<R>, state: State<'_, InboxState>, ids: Vec<String>) {
    state.with(&app, |inbox| inbox.read(&ids, now_ms()));
}

#[tauri::command(async)]
pub fn inbox_read_all<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, InboxState>,
    repository: Option<String>,
) {
    state.with(&app, |inbox| {
        inbox.read_all(repository.as_deref(), now_ms())
    });
}

#[tauri::command(async)]
pub fn inbox_clear<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, InboxState>,
    before_ms: u64,
    repository: Option<String>,
) -> usize {
    state.with(&app, |inbox| inbox.clear(before_ms, repository.as_deref()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn said(title: &str, repository: &str) -> NewEntry {
        NewEntry {
            repository: repository.into(),
            repository_name: "spagitty".into(),
            category: Category::Farm,
            kind: "taskStuck".into(),
            title: title.into(),
            body: String::new(),
            target: Target::Route {
                route: "/farm".into(),
            },
        }
    }

    fn scratch() -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().expect("temporary directory");
        let path = dir.path().join(FILE);
        (dir, path)
    }

    #[test]
    fn what_was_said_is_there_after_a_restart() {
        let (_dir, path) = scratch();
        let mut inbox = Inbox::open(Some(path.clone()));
        let first = inbox.add(said("T-02 is ready to merge", "/r"), 10);
        inbox.add(said("T-09 is stuck after 3 tries", "/r"), 20);
        inbox.read(std::slice::from_ref(&first.id), 30);

        let again = Inbox::open(Some(path));
        let titles: Vec<_> = again
            .list()
            .into_iter()
            .map(|e| (e.title, e.read_ms))
            .collect();
        assert_eq!(
            titles,
            vec![
                ("T-09 is stuck after 3 tries".to_string(), None),
                ("T-02 is ready to merge".to_string(), Some(30)),
            ]
        );
    }

    #[test]
    fn reading_twice_keeps_the_first_time() {
        let mut inbox = Inbox::open(None);
        let entry = inbox.add(said("a", "/r"), 1);
        inbox.read(std::slice::from_ref(&entry.id), 5);
        inbox.read(std::slice::from_ref(&entry.id), 9);
        assert_eq!(inbox.list()[0].read_ms, Some(5));
    }

    #[test]
    fn mark_all_read_can_stay_inside_one_repository() {
        let mut inbox = Inbox::open(None);
        inbox.add(said("here", "/here"), 1);
        inbox.add(said("there", "/there"), 2);
        inbox.read_all(Some("/here"), 3);
        let unread: Vec<_> = inbox
            .list()
            .into_iter()
            .filter(|e| e.read_ms.is_none())
            .map(|e| e.title)
            .collect();
        assert_eq!(unread, vec!["there".to_string()]);
    }

    #[test]
    fn clearing_forgets_only_what_is_older() {
        let mut inbox = Inbox::open(None);
        inbox.add(said("old", "/r"), 100);
        inbox.add(said("new", "/r"), 200);
        assert_eq!(inbox.clear(150, None), 1);
        assert_eq!(inbox.list()[0].title, "new");
    }

    #[test]
    fn clearing_one_repository_leaves_the_others() {
        let mut inbox = Inbox::open(None);
        inbox.add(said("here", "/r"), 100);
        inbox.add(said("there", "/s"), 100);
        assert_eq!(inbox.clear(u64::MAX, Some("/r")), 1);
        assert_eq!(inbox.list().len(), 1);
        assert_eq!(inbox.list()[0].title, "there");
    }

    #[test]
    fn a_file_that_is_not_a_record_reads_as_an_empty_one() {
        let (_dir, path) = scratch();
        std::fs::write(&path, "{ not a list").unwrap();
        assert!(Inbox::open(Some(path)).list().is_empty());
    }

    #[test]
    fn the_stored_shape_is_the_one_the_screen_reads() {
        let mut inbox = Inbox::open(None);
        let entry = inbox.add(said("x", "/r"), 7);
        let json = serde_json::to_value(&entry).unwrap();
        assert_eq!(json["category"], "farm");
        assert_eq!(json["target"]["kind"], "route");
        assert_eq!(json["target"]["route"], "/farm");
        assert_eq!(json["createdMs"], 7);
        assert!(json["readMs"].is_null());
    }
}
