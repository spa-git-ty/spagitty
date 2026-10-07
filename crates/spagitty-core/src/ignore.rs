// SPDX-License-Identifier: GPL-3.0-or-later

//! Whether a changed file in the working tree is one git would notice
//! (BUG-055).
//!
//! The file watcher now watches the working tree as well as `.git`, because an
//! edit in an editor touches nothing in `.git` and the working-copy count stayed
//! stale until something else did. A build or an `npm install` writes thousands
//! of ignored files, though, and refreshing the status for each burst of those
//! would be work for nothing — so a burst only counts when at least one of its
//! paths is not ignored.

use std::path::{Component, Path, PathBuf};

/// A working tree's ignore rules, opened once and asked many times — the file
/// watcher keeps one on its own thread.
pub struct Rules {
    repo: gix::Repository,
}

impl Rules {
    pub fn open(workdir: &Path) -> Option<Self> {
        gix::open(workdir).ok().map(|repo| Rules { repo })
    }

    /// See [`any_not_ignored`].
    pub fn any_not_ignored(&self, paths: &[PathBuf]) -> bool {
        any_not_ignored(&self.repo, paths)
    }
}

/// True when any of `paths` (absolute, inside `repo`'s working tree) is not
/// ignored. Unknown is a yes: a refresh too many is cheaper than a stale count.
pub fn any_not_ignored(repo: &gix::Repository, paths: &[PathBuf]) -> bool {
    let Some(workdir) = repo.workdir() else {
        return false;
    };
    let workdir = workdir
        .canonicalize()
        .unwrap_or_else(|_| workdir.to_path_buf());
    let Ok(index) = repo.index_or_empty() else {
        return true;
    };
    let Ok(mut stack) = repo.excludes(
        &index,
        None,
        gix::worktree::stack::state::ignore::Source::WorktreeThenIdMappingIfNotSkipped,
    ) else {
        return true;
    };

    paths.iter().any(|path| {
        let full = path.canonicalize().unwrap_or_else(|_| path.clone());
        let Ok(relative) = full.strip_prefix(&workdir) else {
            // Outside the working tree: nothing git tracks.
            return false;
        };
        // A path inside an ignored directory is ignored, whatever its own name
        // matches, so every leading directory is asked first.
        let mut prefix = PathBuf::new();
        let parts: Vec<_> = relative
            .components()
            .filter_map(|part| match part {
                Component::Normal(name) => Some(name.to_os_string()),
                _ => None,
            })
            .collect();
        if parts.is_empty() {
            return false;
        }
        for (at, part) in parts.iter().enumerate() {
            prefix.push(part);
            let last = at + 1 == parts.len();
            let mode = if last && !full.is_dir() {
                None
            } else {
                Some(gix::index::entry::Mode::DIR)
            };
            match stack.at_path(&prefix, mode) {
                Ok(platform) if platform.is_excluded() => return false,
                Ok(_) => {}
                Err(_) => return true,
            }
        }
        true
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture::Fixture;

    #[test]
    fn an_edited_tracked_file_counts_and_ignored_output_does_not() {
        let fixture = Fixture::linear(1);
        std::fs::write(fixture.path().join(".gitignore"), "target/\n*.log\n").expect("ignore");
        std::fs::create_dir_all(fixture.path().join("target/debug")).expect("dir");
        std::fs::write(fixture.path().join("target/debug/out.bin"), "x").expect("out");
        std::fs::write(fixture.path().join("build.log"), "x").expect("log");
        std::fs::write(fixture.path().join("notes.md"), "x").expect("notes");
        let repo = fixture.open();

        let ignored = [
            fixture.path().join("target/debug/out.bin"),
            fixture.path().join("build.log"),
        ];
        assert!(
            !any_not_ignored(&repo, &ignored),
            "only ignored output changed"
        );

        let mixed = [ignored[0].clone(), fixture.path().join("notes.md")];
        assert!(
            any_not_ignored(&repo, &mixed),
            "a file git would notice changed"
        );
    }
}
