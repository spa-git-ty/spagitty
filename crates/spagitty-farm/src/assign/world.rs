// SPDX-License-Identifier: GPL-3.0-or-later

//! The repository, as an assignment reaches it.
//!
//! Never the person's working copy. A review runs in a scratch worktree at
//! the pull request's head; a merge in a scratch worktree holding the merge
//! with its conflict markers, as Merger's dry run makes it. Both live under the
//! git directory, are named by the assignment, and are removed when it ends.

use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

use spagitty_core::diff::{self, DiffLine};
use spagitty_core::merger::Scratch;
use spagitty_core::{repo, shell};

use super::engine::{Changed, World};
use super::record::{CheckRun, Target};
use crate::persistence::store;
use crate::verification::command;

/// How much a search answers with: enough to find a caller, not a dump of
/// the tree into a prompt.
const SEARCH_BYTES: usize = 16 * 1024;

pub struct Repository {
    /// The person's repository: read, never written.
    repo: PathBuf,
    scratch: Scratch,
    target: Target,
    name: String,
}

impl Repository {
    /// The scratch worktree for an assignment: at the pull request's head, or
    /// at the receiving branch with the merge done in it.
    pub fn open(repo: &Path, target: &Target, name: &str) -> Result<Repository, String> {
        let handle = repo::open(repo).map_err(|error| error.to_string())?;
        let scratch = match target {
            Target::Review { head, .. } => Scratch::for_agent(&handle, head, name),
            Target::Merge { a_tip, b_tip, .. } => Scratch::for_agent(&handle, a_tip, name)
                .and_then(|scratch| {
                    shell::scratch_merge(scratch.path(), b_tip)?;
                    Ok(scratch)
                }),
        }
        .map_err(|error| error.to_string())?;
        Ok(Repository {
            repo: repo.to_path_buf(),
            scratch,
            target: target.clone(),
            name: name.to_string(),
        })
    }

    fn range(&self) -> (&str, &str) {
        match &self.target {
            Target::Review { base, head, .. } => (base, head),
            Target::Merge { base, a_tip, .. } => (base, a_tip),
        }
    }
}

impl World for Repository {
    fn workdir(&self) -> PathBuf {
        self.scratch.path().to_path_buf()
    }

    fn policy(&self) -> String {
        crate::policy::read(&self.repo).text
    }

    fn changed_files(&self) -> Result<Vec<Changed>, String> {
        let handle = repo::open(&self.repo).map_err(|e| e.to_string())?;
        let (from, to) = self.range();
        let files = diff::changes_between(&handle, from, to).map_err(|e| e.to_string())?;
        Ok(files
            .into_iter()
            .map(|file| Changed {
                path: file.path,
                added: file.added,
                removed: file.removed,
                binary: file.binary || file.too_large,
            })
            .collect())
    }

    fn diff(&self, path: &str) -> Result<Vec<DiffLine>, String> {
        let handle = repo::open(&self.repo).map_err(|e| e.to_string())?;
        let (from, to) = self.range();
        diff::full_file_between(&handle, from, to, path, None)
            .map(|file| file.lines)
            .map_err(|e| e.to_string())
    }

    fn read(&self, path: &str, head: bool) -> Result<String, String> {
        let (from, to) = self.range();
        let revision = if head { to } else { from };
        match shell::show_text(&self.repo, revision, path) {
            Ok(Some(text)) => Ok(text),
            Ok(None) => Err(format!("{path} is not there")),
            Err(error) => Err(error.to_string()),
        }
    }

    fn search(&self, pattern: &str) -> Result<String, String> {
        let mut out = shell::grep_tree(self.scratch.path(), pattern).map_err(|e| e.to_string())?;
        if out.len() > SEARCH_BYTES {
            let mut cut = SEARCH_BYTES;
            while !out.is_char_boundary(cut) {
                cut -= 1;
            }
            out.truncate(cut);
            out.push_str("\n… more matches left out\n");
        }
        Ok(out)
    }

    fn moved(&self) -> bool {
        match &self.target {
            // The head is the host's to move; the webview sees it and says so.
            Target::Review { .. } => false,
            Target::Merge {
                a, b, a_tip, b_tip, ..
            } => {
                let now = |name: &str| shell::commit_id(&self.repo, name).ok();
                now(a).is_some_and(|tip| &tip != a_tip) || now(b).is_some_and(|tip| &tip != b_tip)
            }
        }
    }

    fn checks(&self) -> Vec<String> {
        store::load_farm(&self.repo)
            .map(|farm| farm.verification)
            .unwrap_or_default()
            .into_iter()
            .filter(|line| !line.trim().is_empty())
            .collect()
    }

    fn run_checks(
        &self,
        resolved: &[(String, String)],
        cancel: &AtomicBool,
        hear: &mut dyn FnMut(&str),
    ) -> Result<Vec<CheckRun>, String> {
        let Target::Merge { a_tip, b_tip, .. } = &self.target else {
            return Ok(Vec::new());
        };
        let handle = repo::open(&self.repo).map_err(|e| e.to_string())?;
        let scratch = Scratch::for_agent(&handle, a_tip, &format!("{}-checks", self.name))
            .map_err(|e| e.to_string())?;
        shell::scratch_merge(scratch.path(), b_tip).map_err(|e| e.to_string())?;
        for (path, text) in resolved {
            let file = scratch.path().join(path);
            if let Some(parent) = file.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            std::fs::write(&file, text).map_err(|e| format!("{path}: {e}"))?;
        }
        let mut runs = Vec::new();
        for line in self.checks() {
            hear(&format!("$ {line}"));
            let result = command::run_cancellable(scratch.path(), &line, cancel, command::TIMEOUT);
            for tail in result
                .output
                .lines()
                .rev()
                .take(12)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
            {
                hear(tail);
            }
            hear(if result.passed { "passed" } else { "failed" });
            let passed = result.passed;
            runs.push(CheckRun {
                command: result.command,
                passed,
                output: result.output,
                duration_ms: result.duration_ms,
            });
            if !passed {
                break;
            }
        }
        Ok(runs)
    }
}

/// Did `agent` write commits in `from..to`? Named by a `Co-authored-by`
/// trailer, as Spagitty attributes agent work on the graph. An agent never
/// reviews its own work — the farm's rule, carried over to Review.
pub fn wrote_commits(repo: &Path, from: &str, to: &str, names: &[&str]) -> bool {
    let Ok(messages) = shell::messages_between(repo, from, to) else {
        return false;
    };
    authored_in(&messages, names)
}

pub fn authored_in(messages: &str, names: &[&str]) -> bool {
    messages.lines().any(|line| {
        let line = line.trim();
        let Some(rest) = line
            .get(..15)
            .filter(|head| head.eq_ignore_ascii_case("co-authored-by:"))
            .map(|_| line[15..].trim().to_lowercase())
        else {
            return false;
        };
        names
            .iter()
            .filter(|name| !name.trim().is_empty())
            .any(|name| rest.contains(&name.to_lowercase()))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_co_author_trailer_names_the_agent() {
        let messages = "Fix the cache\n\nCo-authored-by: Claude Code <noreply@example.com>\n\0";
        assert!(authored_in(messages, &["Claude Code"]));
        assert!(!authored_in(messages, &["Codex"]));
        assert!(authored_in(
            "x\n\nco-authored-by: codex <c@x>\n",
            &["Codex"]
        ));
    }

    #[test]
    fn a_mention_in_prose_is_not_authorship() {
        assert!(!authored_in("Asked Codex about this\n", &["Codex"]));
        assert!(!authored_in("Co-authored-by: Someone\n", &[""]));
    }
}
