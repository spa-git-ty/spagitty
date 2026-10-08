// SPDX-License-Identifier: GPL-3.0-or-later

//! Checked after every step: what did the agent write where it was given?
//!
//! A review's worktree is for reading, and a merge's is for proposing — the
//! proposal arrives as text and Spagitty applies it. So anything a step leaves
//! on disk is something the agent was not asked to do. It is listed in the
//! timeline, by path, and put back. Nothing it wrote is ever kept: the limits
//! hold whatever the agent was persuaded to want, and they are enforced here,
//! not asked for in a prompt.

use std::collections::BTreeMap;
use std::path::Path;

use spagitty_core::shell;

/// The worktree's changes before a step: each changed path and its bytes.
#[derive(Debug, Default)]
pub struct Snapshot {
    files: BTreeMap<String, Option<Vec<u8>>>,
}

impl Snapshot {
    pub fn take(dir: &Path) -> Snapshot {
        let files = shell::changed_paths(dir)
            .unwrap_or_default()
            .into_iter()
            .map(|(_, path)| {
                let bytes = std::fs::read(dir.join(&path)).ok();
                (path, bytes)
            })
            .collect();
        Snapshot { files }
    }

    /// The paths a step changed since the snapshot was taken, put back as
    /// they were. Returns them sorted, for the timeline, or why one could not
    /// be put back: a path still on disk is never reported as not kept.
    pub fn restore(&self, dir: &Path) -> Result<Vec<String>, String> {
        let now = shell::changed_paths(dir).unwrap_or_default();
        let mut touched = Vec::new();
        let mut back_to_head = Vec::new();

        for (code, path) in &now {
            match self.files.get(path) {
                Some(before) => {
                    let after = std::fs::read(dir.join(path)).ok();
                    if &after != before {
                        touched.push(path.clone());
                        put_back(dir, path, before.as_deref())?;
                    }
                }
                None if code == "??" => {
                    touched.push(path.clone());
                    put_back(dir, path, None)?;
                }
                None => {
                    touched.push(path.clone());
                    back_to_head.push(path.clone());
                }
            }
        }
        // A path changed before the step and clean now: the agent undid
        // something that was there, which is also a write.
        for (path, before) in &self.files {
            if !now.iter().any(|(_, listed)| listed == path) {
                touched.push(path.clone());
                put_back(dir, path, before.as_deref())?;
            }
        }
        shell::restore_paths(dir, &back_to_head).map_err(|e| e.to_string())?;
        touched.sort();
        touched.dedup();
        Ok(touched)
    }
}

fn put_back(dir: &Path, path: &str, bytes: Option<&[u8]>) -> Result<(), String> {
    let file = dir.join(path);
    let done = match bytes {
        Some(bytes) => file
            .parent()
            .map_or(Ok(()), std::fs::create_dir_all)
            .and_then(|()| std::fs::write(&file, bytes)),
        None => match std::fs::remove_file(&file) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            other => other,
        },
    };
    done.map_err(|error| format!("{path}: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use spagitty_core::fixture::Fixture;

    fn committed() -> Fixture {
        let fixture = Fixture::empty();
        fixture.write("a.txt", "one\n");
        fixture.git(&["add", "a.txt"]);
        fixture.commit("first");
        fixture
    }

    #[test]
    fn a_clean_step_touches_nothing() {
        let fixture = committed();
        let snapshot = Snapshot::take(fixture.path());
        assert!(snapshot.restore(fixture.path()).unwrap().is_empty());
    }

    #[test]
    fn what_an_agent_wrote_is_listed_and_put_back() {
        let fixture = committed();
        let snapshot = Snapshot::take(fixture.path());

        std::fs::write(fixture.path().join("a.txt"), "two\n").unwrap();
        std::fs::write(fixture.path().join("new.txt"), "agent\n").unwrap();

        assert_eq!(
            snapshot.restore(fixture.path()).unwrap(),
            vec!["a.txt".to_string(), "new.txt".to_string()]
        );
        assert_eq!(
            std::fs::read_to_string(fixture.path().join("a.txt")).unwrap(),
            "one\n"
        );
        assert!(!fixture.path().join("new.txt").exists());
    }

    /// What an agent staged is put back too: a new file it added to the index
    /// and a rename it staged, whose old path is restored, without either one
    /// stopping the plain edit beside it from being put back.
    #[test]
    fn staged_additions_and_renames_are_put_back_with_the_rest() {
        let fixture = committed();
        fixture.write("b.txt", "bee\n");
        fixture.git(&["add", "b.txt"]);
        fixture.commit("second");
        let snapshot = Snapshot::take(fixture.path());

        std::fs::write(fixture.path().join("new.rs"), "agent\n").unwrap();
        fixture.git(&["add", "new.rs"]);
        fixture.git(&["mv", "b.txt", "moved.txt"]);
        std::fs::write(fixture.path().join("a.txt"), "two\n").unwrap();

        assert_eq!(
            snapshot.restore(fixture.path()).unwrap(),
            vec![
                "a.txt".to_string(),
                "b.txt".to_string(),
                "moved.txt".to_string(),
                "new.rs".to_string()
            ]
        );
        let read = |name: &str| std::fs::read_to_string(fixture.path().join(name)).ok();
        assert_eq!(read("a.txt").as_deref(), Some("one\n"));
        assert_eq!(read("b.txt").as_deref(), Some("bee\n"));
        assert_eq!(read("moved.txt"), None);
        assert_eq!(read("new.rs"), None);
        assert!(shell::changed_paths(fixture.path()).unwrap().is_empty());
    }

    /// A file named like a pattern is put back by its name alone: the edit
    /// that was already on `a.txt` is not taken back with `*.txt`.
    #[test]
    fn a_path_is_put_back_by_its_name_not_as_a_pattern() {
        let fixture = committed();
        fixture.write("*.txt", "star\n");
        fixture.git(&["add", "--", ":(literal)*.txt"]);
        fixture.commit("star");
        std::fs::write(fixture.path().join("a.txt"), "merged\n").unwrap();
        let snapshot = Snapshot::take(fixture.path());

        std::fs::write(fixture.path().join("*.txt"), "agent\n").unwrap();
        fixture.git(&["add", "--", ":(literal)*.txt"]);

        assert_eq!(
            snapshot.restore(fixture.path()).unwrap(),
            vec!["*.txt".to_string()]
        );
        let read = |name: &str| std::fs::read_to_string(fixture.path().join(name)).unwrap();
        assert_eq!(read("*.txt"), "star\n");
        assert_eq!(read("a.txt"), "merged\n");
    }

    #[test]
    fn a_change_that_was_there_before_the_step_is_kept_as_it_was() {
        let fixture = committed();
        std::fs::write(fixture.path().join("a.txt"), "merged\n").unwrap();
        let snapshot = Snapshot::take(fixture.path());

        std::fs::write(fixture.path().join("a.txt"), "agent\n").unwrap();
        assert_eq!(
            snapshot.restore(fixture.path()).unwrap(),
            vec!["a.txt".to_string()]
        );
        assert_eq!(
            std::fs::read_to_string(fixture.path().join("a.txt")).unwrap(),
            "merged\n"
        );
    }
}
