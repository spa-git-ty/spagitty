// SPDX-License-Identifier: GPL-3.0-or-later

//! The repository's git hooks, read so they can be seen before they run
//! (FEAT-107).
//!
//! A hook is an executable git runs at a moment of its own choosing — before a
//! commit, on its message, after it. Spagitty commits through `git`, so they
//! have always run; what was missing was any way to know they exist, read what
//! they do, or say no to them. This module answers the first two; the third is
//! [`crate::shell::commit_with`]'s `skip_hooks` and the per-repository switch
//! here.
//!
//! # Where hooks come from
//!
//! `git rev-parse --git-path hooks` is the authority: `.git/hooks`, or
//! wherever `core.hooksPath` points. A hook manager usually owns that
//! directory, and each one keeps the part a person wrote somewhere else:
//!
//! - **Husky** points `core.hooksPath` at `.husky/_`, a directory of stubs for
//!   every hook name. The script that runs is `.husky/<name>`; a stub with no
//!   script beside it does nothing, so it is not listed.
//! - **lefthook** writes hooks that call `lefthook run <name>`; what runs is in
//!   `lefthook.yml`.
//! - **pre-commit** (the Python framework) writes hooks that call it; what runs
//!   is in `.pre-commit-config.yaml`.
//!
//! The configuration file is returned beside the hooks, because for those two
//! the hook itself says nothing about the steps.
//!
//! # The per-repository switch
//!
//! Kept in the repository's own `.git/config` as `spagitty.hooks = false`:
//! local to this clone and this person, never committed, and read at the
//! moment of committing rather than remembered by a screen.

use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::error::Result;
use crate::shell;

/// The hook names git knows. Anything else in the directory is not a hook.
const NAMES: &[&str] = &[
    "applypatch-msg",
    "pre-applypatch",
    "post-applypatch",
    "pre-commit",
    "pre-merge-commit",
    "prepare-commit-msg",
    "commit-msg",
    "post-commit",
    "pre-rebase",
    "post-checkout",
    "post-merge",
    "pre-push",
    "pre-receive",
    "update",
    "proc-receive",
    "post-receive",
    "post-update",
    "reference-transaction",
    "push-to-checkout",
    "pre-auto-gc",
    "post-rewrite",
    "sendemail-validate",
    "fsmonitor-watchman",
    "post-index-change",
];

/// The hooks a commit runs, in the order it runs them.
pub const ON_COMMIT: &[&str] = &[
    "pre-commit",
    "prepare-commit-msg",
    "commit-msg",
    "post-commit",
];

/// A script longer than this is cut, and says so. Hooks are short; a long one
/// is usually a vendored tool, and the first 64 KiB say what it is.
const MAX_SCRIPT: usize = 64 * 1024;

/// The config key the per-repository switch is kept under.
const KEY: &str = "spagitty.hooks";

/// Who manages the hooks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Manager {
    /// Plain files in the hooks directory.
    Git,
    Husky,
    Lefthook,
    PreCommit,
}

/// One hook that would run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Hook {
    pub name: String,
    /// The file that holds what runs, relative to the repository where it is
    /// inside it.
    pub path: String,
    /// What it runs.
    pub script: String,
    /// Cut at [`MAX_SCRIPT`].
    pub truncated: bool,
    /// One of [`ON_COMMIT`].
    pub on_commit: bool,
}

/// A configuration file a manager reads its steps from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub path: String,
    pub text: String,
}

/// Everything about this repository's hooks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Hooks {
    /// The directory git runs hooks from, as the repository names it.
    pub dir: String,
    pub manager: Manager,
    /// lefthook's or pre-commit's configuration, when that is where the steps are.
    pub config: Option<Config>,
    pub hooks: Vec<Hook>,
    /// False when this repository's hooks are switched off in Spagitty.
    pub enabled: bool,
}

impl Hooks {
    /// The hooks a commit here would run, by name — none when switched off.
    pub fn on_commit(&self) -> Vec<String> {
        if !self.enabled {
            return Vec::new();
        }
        ON_COMMIT
            .iter()
            .filter(|name| self.hooks.iter().any(|hook| hook.name == **name))
            .map(|name| name.to_string())
            .collect()
    }
}

/// Read the repository's hooks.
pub fn list(repo: &Path) -> Result<Hooks> {
    let dir = shell::hooks_dir(repo)?;
    let enabled = enabled(repo)?;
    let shown = shown_path(repo, &dir);

    let husky = is_husky(repo, &dir);
    let mut hooks = Vec::new();
    for name in NAMES {
        let file = dir.join(name);
        if !runnable(&file) {
            continue;
        }
        // Husky's stubs run `.husky/<name>`; without one there, nothing runs.
        let source = if husky {
            match husky_script(repo, &dir, name) {
                Some(script) => script,
                None => continue,
            }
        } else {
            file
        };
        let Ok(bytes) = std::fs::read(&source) else {
            continue;
        };
        let truncated = bytes.len() > MAX_SCRIPT;
        let script = String::from_utf8_lossy(&bytes[..bytes.len().min(MAX_SCRIPT)]).into_owned();
        hooks.push(Hook {
            name: name.to_string(),
            path: shown_path(repo, &source),
            script,
            truncated,
            on_commit: ON_COMMIT.contains(name),
        });
    }

    let manager = if husky {
        Manager::Husky
    } else if hooks.iter().any(|hook| hook.script.contains("lefthook")) {
        Manager::Lefthook
    } else if hooks.iter().any(|hook| {
        hook.script.contains("pre-commit")
            && (hook.script.contains("pre_commit")
                || hook.script.contains("pre-commit run")
                || hook.script.contains("INSTALL_PYTHON"))
    }) {
        Manager::PreCommit
    } else {
        Manager::Git
    };

    let config = match manager {
        Manager::Lefthook => first_config(
            repo,
            &[
                "lefthook.yml",
                ".lefthook.yml",
                "lefthook.yaml",
                ".lefthook.yaml",
                "lefthook.toml",
            ],
        ),
        Manager::PreCommit => {
            first_config(repo, &[".pre-commit-config.yaml", ".pre-commit-config.yml"])
        }
        _ => None,
    };

    Ok(Hooks {
        dir: shown,
        manager,
        config,
        hooks,
        enabled,
    })
}

/// Whether Spagitty runs this repository's hooks when it commits.
pub fn enabled(repo: &Path) -> Result<bool> {
    Ok(shell::get_config(repo, KEY)?.as_deref() != Some("false"))
}

/// Switch this repository's hooks on or off for commits made in Spagitty.
/// On is the absence of the key, so switching back leaves `.git/config` as it
/// was found.
pub fn set_enabled(repo: &Path, on: bool) -> Result<()> {
    if on {
        shell::unset_config(repo, "--local", KEY)
    } else {
        shell::set_config(repo, "--local", KEY, "false")
    }
}

/// A file git would run: present, a file, and on Unix executable — git skips
/// a hook without the bit, with a warning. Windows has no bit to check.
fn runnable(file: &Path) -> bool {
    let Ok(meta) = std::fs::metadata(file) else {
        return false;
    };
    if !meta.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        meta.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        true
    }
}

/// Husky's layout: `core.hooksPath` at `.husky/_` (v9), or at `.husky` with
/// the `_/husky.sh` helper beside it (v4 to v8).
fn is_husky(repo: &Path, dir: &Path) -> bool {
    let husky = repo.join(".husky");
    same(dir, &husky.join("_")) || (same(dir, &husky) && husky.join("_").is_dir())
}

/// What a Husky hook runs: `.husky/<name>` for v9's stubs, the hook itself for
/// older layouts where `.husky` is the hooks directory.
fn husky_script(repo: &Path, dir: &Path, name: &str) -> Option<PathBuf> {
    let husky = repo.join(".husky");
    if same(dir, &husky) {
        return Some(dir.join(name));
    }
    let script = husky.join(name);
    script.is_file().then_some(script)
}

fn same(a: &Path, b: &Path) -> bool {
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(a), Ok(b)) => a == b,
        _ => a == b,
    }
}

/// `path` relative to the repository when it is inside it, with `/`.
fn shown_path(repo: &Path, path: &Path) -> String {
    let repo = repo.canonicalize().unwrap_or_else(|_| repo.to_path_buf());
    let full = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let shown = full
        .strip_prefix(&repo)
        .map(Path::to_path_buf)
        .unwrap_or(full);
    let text = shown.to_string_lossy().replace('\\', "/");
    text.strip_prefix("//?/")
        .map(str::to_string)
        .unwrap_or(text)
}

fn first_config(repo: &Path, names: &[&str]) -> Option<Config> {
    names.iter().find_map(|name| {
        let text = std::fs::read_to_string(repo.join(name)).ok()?;
        Some(Config {
            path: name.to_string(),
            text,
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture::Fixture;

    fn write_hook(path: &Path, text: &str) {
        std::fs::create_dir_all(path.parent().expect("parent")).expect("dir");
        std::fs::write(path, text).expect("hook");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).expect("chmod");
        }
    }

    #[test]
    fn a_repository_with_only_samples_has_no_hooks() {
        let fixture = Fixture::linear(1);
        let hooks = list(fixture.path()).expect("hooks");
        assert!(hooks.hooks.is_empty());
        assert_eq!(hooks.manager, Manager::Git);
        assert!(hooks.on_commit().is_empty());
    }

    #[test]
    fn plain_hooks_are_listed_with_their_script_and_whether_a_commit_runs_them() {
        let fixture = Fixture::linear(1);
        let dir = fixture.path().join(".git/hooks");
        write_hook(&dir.join("pre-commit"), "#!/bin/sh\necho lint\n");
        write_hook(&dir.join("pre-push"), "#!/bin/sh\necho test\n");
        write_hook(&dir.join("not-a-hook"), "#!/bin/sh\n");

        let hooks = list(fixture.path()).expect("hooks");
        let names: Vec<_> = hooks.hooks.iter().map(|hook| hook.name.as_str()).collect();
        assert_eq!(names, ["pre-commit", "pre-push"]);
        assert_eq!(hooks.hooks[0].script, "#!/bin/sh\necho lint\n");
        assert_eq!(hooks.hooks[0].path, ".git/hooks/pre-commit");
        assert!(hooks.hooks[0].on_commit);
        assert!(!hooks.hooks[1].on_commit);
        assert_eq!(hooks.on_commit(), ["pre-commit"]);
    }

    #[test]
    fn husky_shows_the_script_a_person_wrote_not_its_stub() {
        let fixture = Fixture::linear(1);
        let stubs = fixture.path().join(".husky/_");
        for name in ["pre-commit", "commit-msg", "pre-push"] {
            write_hook(&stubs.join(name), ". \"$(dirname \"$0\")/h\"\n");
        }
        write_hook(
            &fixture.path().join(".husky/pre-commit"),
            "npx lint-staged\n",
        );
        fixture.git(&["config", "core.hooksPath", ".husky/_"]);

        let hooks = list(fixture.path()).expect("hooks");
        assert_eq!(hooks.manager, Manager::Husky);
        assert_eq!(
            hooks.hooks.len(),
            1,
            "a stub with nothing beside it does nothing"
        );
        assert_eq!(hooks.hooks[0].path, ".husky/pre-commit");
        assert_eq!(hooks.hooks[0].script, "npx lint-staged\n");
        assert_eq!(hooks.dir, ".husky/_");
    }

    #[test]
    fn lefthook_brings_its_configuration() {
        let fixture = Fixture::linear(1);
        write_hook(
            &fixture.path().join(".git/hooks/pre-commit"),
            "#!/bin/sh\nlefthook run \"pre-commit\" \"$@\"\n",
        );
        std::fs::write(
            fixture.path().join("lefthook.yml"),
            "pre-commit:\n  commands:\n    lint:\n      run: npm run lint\n",
        )
        .expect("config");

        let hooks = list(fixture.path()).expect("hooks");
        assert_eq!(hooks.manager, Manager::Lefthook);
        let config = hooks.config.expect("the steps");
        assert_eq!(config.path, "lefthook.yml");
        assert!(config.text.contains("npm run lint"));
    }

    #[test]
    fn switching_off_is_kept_in_the_repository_and_switching_on_leaves_no_trace() {
        let fixture = Fixture::linear(1);
        write_hook(
            &fixture.path().join(".git/hooks/pre-commit"),
            "#!/bin/sh\nexit 0\n",
        );

        set_enabled(fixture.path(), false).expect("off");
        let off = list(fixture.path()).expect("hooks");
        assert!(!off.enabled);
        assert!(off.on_commit().is_empty(), "nothing runs when off");
        assert_eq!(
            fixture.git(&["config", "--local", "spagitty.hooks"]).trim(),
            "false"
        );

        set_enabled(fixture.path(), true).expect("on");
        assert!(enabled(fixture.path()).expect("read"));
        assert!(shell::get_config(fixture.path(), KEY)
            .expect("read")
            .is_none());
    }

    #[test]
    fn a_skipped_commit_runs_no_hook_at_all_and_a_run_one_streams_what_it_says() {
        let fixture = Fixture::linear(1);
        std::fs::write(fixture.path().join("a.txt"), "a").expect("file");
        fixture.git(&["add", "a.txt"]);
        let dir = fixture.path().join(".git/hooks");
        write_hook(
            &dir.join("pre-commit"),
            "#!/bin/sh\necho 'checking the code'\nexit 1\n",
        );
        write_hook(
            &dir.join("post-commit"),
            "#!/bin/sh\ntouch post-commit-ran\n",
        );

        let mut lines = Vec::new();
        let refused = shell::commit_with(
            fixture.path(),
            "add a",
            "",
            false,
            false,
            false,
            &mut |line| lines.push(line.to_string()),
        );
        assert!(refused.is_err(), "the hook said no");
        assert!(
            lines.iter().any(|line| line == "checking the code"),
            "{lines:?}"
        );

        shell::commit_with(fixture.path(), "add a", "", false, false, true, &mut |_| {})
            .expect("skipping the hooks commits");
        assert!(
            !fixture.path().join("post-commit-ran").exists(),
            "post-commit is skipped too, which --no-verify alone does not do"
        );
    }
}
