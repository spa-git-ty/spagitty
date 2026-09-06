// SPDX-License-Identifier: GPL-3.0-or-later

//! # The `git` binary boundary
//!
//! This module is the **only** place in Spagitty that spawns a process. Nothing
//! else in `spagitty-core` shells out; if you need to, add it here.
//!
//! Everything Spagitty *reads* — log walking, refs, diffing, status — is done
//! in-process with `gix`. But a specific set of operations is deliberately
//! **not** reimplemented in Rust:
//!
//! | Operation | Why we shell out |
//! |---|---|
//! | **Interactive rebase execution** | The rebase machinery is a stateful protocol (`rebase-merge/`, `done`, `git-rebase-todo`, `ORIG_HEAD`, the reflog trail) that other tools and the user's own `git` expect to find intact. A reimplementation that got the on-disk state subtly wrong would strand people mid-rebase with a repo their CLI can't finish. We *plan* the rebase in Rust (that is the Rebase screen's preview) and hand the todo list to `git rebase -i` to execute. |
//! | **Hooks** | `pre-commit`, `commit-msg`, `pre-push` and friends are arbitrary executables with an environment contract, and users expect the same behavior they get on the CLI. `git` already runs them correctly. |
//! | **LFS** | LFS is a filter/smudge protocol implemented by a separate binary that git invokes via config. Bypassing git means bypassing LFS, and checkouts silently produce pointer files. |
//! | **Submodule recursion** | Recursive init/update/sync spans nested repositories with their own config, URLs and credentials. `git submodule` is the reference implementation. |
//! | **Cloning** | The first operation that needs credentials, and therefore the one credential helpers exist for. It is also transport — the smart protocol over HTTPS or SSH — which is the largest thing in git that is not worth a second implementation. |
//! | **Credential helpers** | Helpers are external programs resolved through config with a documented stdin/stdout protocol, and they are where OS keychain integration already lives. Reimplementing the protocol would mean reimplementing every helper's quirks. |
//! | **Committing** | `pre-commit` and `commit-msg` hooks have to run, and a signed commit goes through the user's configured GPG or SSH program. Writing the commit object ourselves would silently skip both, so a commit made in Spagitty would differ from the same commit made on the command line. |
//! | **Staging** | The index is the most-read piece of shared state there is: the user's own `git status`, their prompt, their editor's gutter and their hooks all read it. Writing it ourselves — including through `git apply --cached` for a single hunk — keeps the on-disk format and its locking in git's hands. |
//!
//! The rule of thumb: if the operation *mutates* state that the wider git
//! ecosystem also reads, or if it delegates to something outside the repository,
//! it belongs here. Read-only history questions belong in `gix`.
//!
//! **One read breaks that rule, and it is written down rather than quietly
//! done.** [`blame`] shells out, because `gix::blame` 0.16 — the newest there
//! is — panics on an ordinary history shape rather than returning an error. The
//! rule stands; blame is an exception with a reason and an end condition, and
//! it moves back in-process when the upstream defect is fixed. See [`blame`]
//! itself for what exactly goes wrong.
//!
//! Nothing in this module is called by the Graph screen. It exists now so the
//! boundary is drawn before the screens that need it are built.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex, OnceLock, Weak};
use std::time::Instant;

use crate::error::{Error, Result};
use crate::record::{self, Outcome};

/// Build the `git` command every function here spawns.
///
/// One place, so the invariants hold for all of them: the working directory is
/// the repository, and `GIT_TERMINAL_PROMPT=0` stops git blocking on a tty
/// prompt we have no terminal for — credential requests must come back as a
/// failure the UI can act on, rather than hanging the app forever.
fn command(repo: &Path, args: &[&str]) -> Command {
    let mut command = Command::new("git");
    command
        .current_dir(repo)
        .args(args)
        .env("GIT_TERMINAL_PROMPT", "0");
    command
}

/// Run a prepared command to completion, record what happened, and turn a
/// non-zero exit into [`Error::Git`].
///
/// Every spawn in this module ends here or in [`record_spawn`], which is what
/// makes [`crate::record`] a record of what ran rather than of what someone
/// remembered to log. Adding a spawn that bypasses both is the one mistake this
/// shape is meant to make obvious.
fn finish(mut command: Command, args: &[&str]) -> Result<String> {
    let started = Instant::now();
    let output = command.output();
    let elapsed = started.elapsed().as_millis() as u64;

    let output = match output {
        Ok(output) => output,
        Err(error) => {
            // git missing from PATH, or the repository gone from under us. It
            // never ran, and that is worth showing in the log too — an empty
            // log next to a failed operation reads as "Spagitty did nothing".
            record::push(
                args,
                Outcome::Failed {
                    code: None,
                    stderr: error.to_string(),
                },
                elapsed,
            );
            return Err(error.into());
        }
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        record::push(
            args,
            Outcome::Failed {
                code: output.status.code(),
                stderr: stderr.clone(),
            },
            elapsed,
        );
        return Err(Error::Git {
            command: args.join(" "),
            stderr,
        });
    }

    record::push(args, Outcome::Ok, elapsed);
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Spawn a prepared command without waiting, and record it as started.
///
/// [`clone_start`], [`rebase_interactive_spawn`], [`fetch_spawn`] and
/// [`push_spawn`] use this: each runs for long enough that waiting for the
/// outcome before recording would hide the one command the user is most likely
/// to be asking about while it runs.
fn record_spawn(mut command: Command, args: &[&str]) -> Result<std::process::Child> {
    match command.spawn() {
        Ok(child) => {
            record::push(args, Outcome::Started, 0);
            Ok(child)
        }
        Err(error) => {
            record::push(
                args,
                Outcome::Failed {
                    code: None,
                    stderr: error.to_string(),
                },
                0,
            );
            Err(error.into())
        }
    }
}

/// Run `git` in `repo` and return stdout on success.
fn run(repo: &Path, args: &[&str]) -> Result<String> {
    let lock = operation_lock(repo);
    let _guard = lock.lock().expect("git operation lock");
    finish(command(repo, args), args)
}

/// Serialize synchronous Git operations for the same checkout inside Spagitty.
/// External Git processes still own their own coordination and Git lockfiles.
fn operation_lock(repo: &Path) -> Arc<Mutex<()>> {
    static LOCKS: OnceLock<Mutex<HashMap<PathBuf, Weak<Mutex<()>>>>> = OnceLock::new();
    let key = repo.canonicalize().unwrap_or_else(|_| repo.to_path_buf());
    let mut locks = LOCKS.get_or_init(Mutex::default).lock().expect("git locks");
    locks.retain(|_, lock| lock.strong_count() > 0);
    if let Some(lock) = locks.get(&key).and_then(Weak::upgrade) {
        return lock;
    }
    let lock = Arc::new(Mutex::new(()));
    locks.insert(key, Arc::downgrade(&lock));
    lock
}

/// The `git` version on PATH, or an error if there is no usable `git`.
///
/// Called at startup: the operations above have no in-process fallback, so a
/// missing `git` is worth telling the user about before they hit one.
pub fn version(repo: &Path) -> Result<String> {
    Ok(run(repo, &["--version"])?.trim().to_string())
}

/// Run `git` with `input` on its stdin. Used for `git apply`, which is how a
/// single hunk reaches the index.
fn run_with_stdin(repo: &Path, args: &[&str], input: &str) -> Result<String> {
    use std::io::Write;
    use std::process::Stdio;

    let started = Instant::now();
    let mut child = command(repo, args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    // Dropped at the end of the block, closing stdin; without that, `git apply`
    // waits for more patch forever.
    {
        let mut stdin = child.stdin.take().expect("piped stdin");
        stdin.write_all(input.as_bytes())?;
    }

    let output = child.wait_with_output()?;
    let elapsed = started.elapsed().as_millis() as u64;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        record::push(
            args,
            Outcome::Failed {
                code: output.status.code(),
                stderr: stderr.clone(),
            },
            elapsed,
        );
        return Err(Error::Git {
            command: args.join(" "),
            stderr,
        });
    }

    record::push(args, Outcome::Ok, elapsed);
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// `--` before the paths, always: a path that happens to look like an option,
/// or like a revision, is still a path.
fn with_paths<'a>(args: &[&'a str], paths: &'a [String]) -> Vec<&'a str> {
    let mut all: Vec<&str> = args.to_vec();
    all.push("--");
    all.extend(paths.iter().map(String::as_str));
    all
}

/// Stage whole paths. Works for a modification, an addition and a deletion
/// alike, which is why it is `git add` rather than three commands.
pub fn stage(repo: &Path, paths: &[String]) -> Result<()> {
    if paths.is_empty() {
        return Ok(());
    }
    run(repo, &with_paths(&["add"], paths))?;
    Ok(())
}

/// Unstage whole paths: put the index back to what `HEAD` has.
///
/// In a repository with no commits there is no `HEAD` to restore from, and the
/// only way for a path to be staged is for it to be new — so removing the entry
/// is the same operation expressed differently, not a special case bolted on.
///
/// Neither form touches the working tree. The file on disk is left exactly as
/// it is; only the index moves.
pub fn unstage(repo: &Path, paths: &[String]) -> Result<()> {
    if paths.is_empty() {
        return Ok(());
    }

    if run(repo, &["rev-parse", "--verify", "--quiet", "HEAD"]).is_ok() {
        run(repo, &with_paths(&["restore", "--staged"], paths))?;
    } else {
        run(repo, &with_paths(&["rm", "--cached", "--quiet"], paths))?;
    }
    Ok(())
}

/// Apply one hunk's patch to the index, forwards to stage it or in reverse to
/// unstage it.
///
/// `--cached` is what keeps this off the working tree: the file on disk is
/// never touched, so staging part of a file cannot lose the rest of it.
pub fn apply_to_index(repo: &Path, patch: &str, reverse: bool) -> Result<()> {
    let mut args = vec!["apply", "--cached", "--whitespace=nowarn"];
    if reverse {
        args.push("--reverse");
    }
    args.push("-");

    run_with_stdin(repo, &args, patch)?;
    Ok(())
}

/// Apply one hunk's patch to the working tree, in reverse, to discard it.
///
/// The mirror of [`apply_to_index`] with the `--cached` taken off: this one
/// writes the file on disk and nothing else, so a discarded hunk leaves both
/// the index and the rest of the file exactly as they were.
pub fn apply_to_worktree(repo: &Path, patch: &str, reverse: bool) -> Result<()> {
    let mut args = vec!["apply", "--whitespace=nowarn"];
    if reverse {
        args.push("--reverse");
    }
    args.push("-");

    run_with_stdin(repo, &args, patch)?;
    Ok(())
}

/// Throw away working-tree changes to tracked paths, back to the index.
///
/// Not `--staged`: what is staged is a decision the user has already made, and
/// discarding an unstaged change must not quietly undo it as well. A file
/// staged in part keeps its staged part and loses the rest, which is exactly
/// what the Unstaged column is showing.
///
/// This is the first operation in Spagitty that destroys work with no way back
/// — there is no reflog for the working tree — so the caller confirms first.
pub fn discard(repo: &Path, paths: &[String]) -> Result<()> {
    if paths.is_empty() {
        return Ok(());
    }
    run(repo, &with_paths(&["restore", "--worktree"], paths))?;
    Ok(())
}

/// Delete untracked paths.
///
/// `git clean` rather than removing the files directly, so that the same rules
/// the user's own `git clean` follows apply here — in particular no `-x`, so an
/// ignored file is never deleted by a screen that never showed it. `-d` covers
/// an untracked directory, which status reports as one entry.
pub fn remove_untracked(repo: &Path, paths: &[String]) -> Result<()> {
    if paths.is_empty() {
        return Ok(());
    }
    run(repo, &with_paths(&["clean", "-f", "-d", "-q"], paths))?;
    Ok(())
}

/// Commit what is staged.
///
/// Through `git` so that `pre-commit` and `commit-msg` hooks run and a
/// configured signing program is used — a commit made here is the same commit
/// the command line would have made.
///
/// `sign` adds `--gpg-sign`, and only when it is true (FEAT-019). git would
/// sign anyway when `commit.gpgsign` is set — the flag is what makes the
/// command say so, both in the log the Settings panel shows and in the gap
/// between what the Commit screen promised and what ran.
pub fn commit(repo: &Path, subject: &str, body: &str, amend: bool, sign: bool) -> Result<String> {
    let mut args = vec!["commit", "-m", subject];
    if !body.trim().is_empty() {
        args.push("-m");
        args.push(body);
    }
    if amend {
        args.push("--amend");
    }
    if sign {
        args.push("--gpg-sign");
    }

    run(repo, &args)?;
    Ok(run(repo, &["rev-parse", "HEAD"])?.trim().to_string())
}

/// Check out a branch.
///
/// Through `git` so that checkout filters, LFS smudging and post-checkout hooks
/// all run, and so that a checkout which would overwrite uncommitted work is
/// refused by git with its own message.
/// `git switch`, not `git checkout`: `checkout` is overloaded — with a `--` it
/// restores paths, without one it guesses between a branch and a revision.
/// `switch` only ever changes branch, so a branch whose name looks like a path
/// cannot be misread as one.
pub fn checkout(repo: &Path, name: &str) -> Result<()> {
    run(repo, &["switch", name])?;
    Ok(())
}

/// Create a branch, optionally checking it out.
///
/// `start` may be empty, which means `HEAD` — the same default `git branch`
/// has. Name validation is git's: it already knows every rule
/// `git check-ref-format` enforces.
pub fn create_branch(repo: &Path, name: &str, start: &str, checkout_it: bool) -> Result<()> {
    let mut args: Vec<&str> = if checkout_it {
        vec!["switch", "--create", name]
    } else {
        vec!["branch", name]
    };
    if !start.is_empty() {
        args.push(start);
    }

    run(repo, &args)?;
    Ok(())
}

/// Stash the working copy.
///
/// Through `git` because a stash is three writes at once — a commit, a ref and
/// the working tree — and its on-disk shape is what every other tool reads as
/// "a stash". An empty message means git writes its own default.
pub fn stash_push(repo: &Path, message: &str, include_untracked: bool) -> Result<()> {
    let mut args = vec!["stash", "push"];
    if include_untracked {
        args.push("--include-untracked");
    }
    if !message.is_empty() {
        args.push("--message");
        args.push(message);
    }

    run(repo, &args)?;
    Ok(())
}

/// The message of the commit `HEAD` points at, for pre-filling an amend.
pub fn head_message(repo: &Path) -> Result<String> {
    run(repo, &["log", "-1", "--pretty=%B"])
}

/// Set a config key in one scope.
///
/// Config is state the whole git ecosystem reads — the user's own `git`, their
/// prompt, their editor, their hooks — so it is written by `git config` rather
/// than by us rewriting an INI file. `scope` is `--global` or `--local`, and it
/// is always passed explicitly: inferring it is how a repository-local identity
/// silently becomes a global one.
pub fn set_config(repo: &Path, scope: &str, key: &str, value: &str) -> Result<()> {
    run(repo, &["config", scope, key, value])?;
    Ok(())
}

/// Remove a config key from one scope.
///
/// `--unset`, not an empty value. An empty `user.email` is a *configured* empty
/// email, which git will commit with; an unset one falls back to the next
/// scope, which is what clearing a field means.
///
/// Unsetting a key that is not there is not an error here: `git config --unset`
/// exits 5 in that case, and "it is already gone" is the outcome the caller
/// asked for.
pub fn unset_config(repo: &Path, scope: &str, key: &str) -> Result<()> {
    match run(repo, &["config", scope, "--unset", key]) {
        Ok(_) => Ok(()),
        Err(Error::Git { stderr, .. }) if stderr.is_empty() => Ok(()),
        Err(other) => Err(other),
    }
}

/// Read a git config key (FEAT-068).
pub fn get_config(repo: &Path, key: &str) -> Result<Option<String>> {
    match run(repo, &["config", "--get", key]) {
        Ok(val) => {
            let trimmed = val.trim().to_string();
            if trimmed.is_empty() {
                Ok(None)
            } else {
                Ok(Some(trimmed))
            }
        }
        Err(Error::Git { .. }) => Ok(None),
        Err(other) => Err(other),
    }
}

/// Launch git difftool for a file or revision without blocking (FEAT-068).
pub fn launch_difftool(
    repo: &Path,
    path: &str,
    tool: Option<&str>,
    commit: Option<&str>,
) -> Result<()> {
    let mut args = vec!["difftool", "--no-prompt", "-y"];
    if let Some(t) = tool {
        if !t.is_empty() {
            args.push("-t");
            args.push(t);
        }
    }
    if let Some(c) = commit {
        if !c.is_empty() {
            args.push(c);
        }
    }
    args.push("--");
    args.push(path);

    let mut cmd = command(repo, &args);
    cmd.stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    let _ = cmd.spawn().map_err(|e| Error::Git {
        command: format!("git {}", args.join(" ")),
        stderr: e.to_string(),
    })?;
    Ok(())
}

/// Launch git mergetool for a conflicted path without blocking (FEAT-068).
pub fn launch_mergetool(repo: &Path, path: &str, tool: Option<&str>) -> Result<()> {
    let mut args = vec!["mergetool", "--no-prompt", "-y"];
    if let Some(t) = tool {
        if !t.is_empty() {
            args.push("-t");
            args.push(t);
        }
    }
    args.push("--");
    args.push(path);

    let mut cmd = command(repo, &args);
    cmd.stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    let _ = cmd.spawn().map_err(|e| Error::Git {
        command: format!("git {}", args.join(" ")),
        stderr: e.to_string(),
    })?;
    Ok(())
}

/// Start a clone and hand the caller the running process.
///
/// The one function here that does not wait for git to finish. A clone can take
/// minutes, has to be cancellable, and reports progress as it goes — so the
/// caller owns the child, reads its stderr, and kills it if the user changes
/// their mind. Everything else in this module is a command that ends.
///
/// Through `git` because this is the operation credential helpers exist for:
/// they are external programs resolved through config, and they are where OS
/// keychain integration already lives. `GIT_TERMINAL_PROMPT=0` still holds, so
/// a repository whose credentials no helper can supply fails with git's message
/// rather than hanging on a prompt there is no terminal for.
///
/// `--progress` because git only reports progress when stderr is a terminal,
/// and here it is a pipe.
pub fn clone_start(url: &str, destination: &Path) -> Result<std::process::Child> {
    use std::process::Stdio;

    // The one command with no repository to run in — it is what creates one —
    // so it is built by hand rather than through `command`, and the working
    // directory is left as the process's own.
    let into = destination.to_string_lossy().into_owned();
    let args = [
        "clone",
        "--progress",
        "--recurse-submodules",
        "--",
        url,
        &into,
    ];

    let mut command = Command::new("git");
    command
        .args(args)
        .env("GIT_TERMINAL_PROMPT", "0")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    record_spawn(command, &args)
}

/// Who last touched each line, as `--line-porcelain`.
///
/// The one **read** in this module, and the exception noted in the header.
/// `gix::blame` panics — a failed internal assertion, not an error we could
/// catch and report — when blaming a file at a merge commit whose history has
/// an intervening commit that left the file alone. That is not an exotic shape;
/// it is most files in most repositories. Every diff algorithm and both rename
/// settings do it, and 0.16 is the newest published version.
///
/// So blame comes from `git` until that is fixed upstream, and it is worth
/// being plain about the trade: this call spawns a process for a question the
/// rest of the crate answers in memory. `--line-porcelain` repeats every header
/// on every line, which makes the parser a loop rather than a state machine.
pub fn blame(repo: &Path, revision: &str, path: &str) -> Result<String> {
    run(repo, &["blame", "--line-porcelain", revision, "--", path])
}

// --- History rewriting and moving ------------------------------------------
//
// Everything below mutates refs, the index or the working tree, which is the
// rule at the top of this file: it goes through `git`. Two further reasons
// apply to this group in particular. Every one of them writes `ORIG_HEAD` and a
// reflog entry, which is the *only* thing that makes these operations
// recoverable — a reimplementation that forgot either would turn "undo" into
// "restore from backup". And each of them can stop half-way with conflicts, in
// which case the repository is left in a documented state (`MERGE_HEAD`,
// `rebase-merge/`, `CHERRY_PICK_HEAD`) that the user's own `git` and Spagitty's
// Conflicts screen both already know how to read.

/// How far back a reset takes the index and the working tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ResetMode {
    /// Move the branch. Index and working tree untouched — the changes become
    /// staged.
    Soft,
    /// Move the branch and reset the index. Working tree untouched.
    Mixed,
    /// Move everything. **Uncommitted work is gone.**
    Hard,
}

impl ResetMode {
    fn flag(self) -> &'static str {
        match self {
            ResetMode::Soft => "--soft",
            ResetMode::Mixed => "--mixed",
            ResetMode::Hard => "--hard",
        }
    }
}

/// Move the current branch to `commit`.
pub fn reset(repo: &Path, commit: &str, mode: ResetMode) -> Result<()> {
    run(repo, &["reset", mode.flag(), commit])?;
    Ok(())
}

/// Create a commit that undoes `commit`.
///
/// `--no-edit` so the message git writes ("Revert \"…\"") is used rather than
/// an editor being opened against a terminal that does not exist. Reverting a
/// merge needs `-m 1`, which git otherwise refuses to guess; the first parent is
/// the mainline in every case Spagitty can show, because that is what the graph's
/// first parent *is*.
pub fn revert(repo: &Path, commit: &str, is_merge: bool) -> Result<()> {
    let mut args = vec!["revert", "--no-edit"];
    if is_merge {
        args.push("-m");
        args.push("1");
    }
    args.push(commit);

    run(repo, &args)?;
    Ok(())
}

/// Replay `commits` onto the current branch, oldest first.
///
/// One `git cherry-pick` with every commit on it rather than a loop: git then
/// owns the sequencer state, so a conflict half-way through leaves a repository
/// that `git cherry-pick --continue` can finish. A loop of single picks would
/// stop with no record that more were meant to follow.
pub fn cherry_pick(repo: &Path, commits: &[String]) -> Result<()> {
    if commits.is_empty() {
        return Ok(());
    }
    let mut args = vec!["cherry-pick"];
    args.extend(commits.iter().map(String::as_str));

    run(repo, &args)?;
    Ok(())
}

/// Merge `source` into the current branch.
///
/// `--no-edit` for the same reason as [`revert`]. `ff_only` is the graph's
/// "Fast-forward" action, which is a *different request* from a merge that
/// happens to fast-forward: it asks git to refuse rather than to create a merge
/// commit, and that refusal is the answer the user wanted.
pub fn merge(repo: &Path, source: &str, ff_only: bool, no_ff: bool) -> Result<()> {
    let mut args = vec!["merge", "--no-edit"];
    if ff_only {
        args.push("--ff-only");
    } else if no_ff {
        args.push("--no-ff");
    }
    args.push(source);

    run(repo, &args)?;
    Ok(())
}

/// Merge a pinned commit only while the intended branch remains checked out.
/// The same lock is used by checkout and other synchronous Git mutations, so
/// an in-app branch switch cannot slip between validation and the merge.
pub fn merge_into(repo: &Path, commit: &str, destination: &str) -> Result<()> {
    let lock = operation_lock(repo);
    let _guard = lock.lock().expect("git operation lock");
    let opened = crate::repo::open(repo)?;
    if crate::repo::head(&opened).branch.as_deref() != Some(destination) {
        return Err(Error::Stale(format!("destination branch {destination}")));
    }
    let args = ["merge", "--no-edit", "--no-ff", commit];
    finish(command(repo, &args), &args)?;
    Ok(())
}

/// Replay commits onto `onto`.
///
/// The three-argument form covers every case the graph offers:
///
/// - *Rebase this branch onto that one* — `upstream` is the target, `onto` is
///   empty, `branch` is empty (meaning HEAD).
/// - *Rebase onto this commit* — the same, with a commit id instead of a name.
/// - *Rebase these N commits onto that branch* — `onto` is the target,
///   `upstream` is the parent of the oldest selected commit, and the range
///   between them is what moves.
pub fn rebase(repo: &Path, onto: &str, upstream: &str, branch: &str) -> Result<()> {
    let mut args = vec!["rebase"];
    if !onto.is_empty() {
        args.push("--onto");
        args.push(onto);
    }
    args.push(upstream);
    if !branch.is_empty() {
        args.push(branch);
    }

    run(repo, &args)?;
    Ok(())
}

/// Execute a planned interactive rebase. The todo list is produced in Rust by
/// the Rebase screen; `git` performs it.
///
/// The todo reaches git through `GIT_SEQUENCE_EDITOR`, which git runs with the
/// path of the todo file it just wrote. Ours does not edit it — it overwrites
/// it with the plan the user built on screen. That is the documented way to
/// drive `rebase -i` without a terminal, and it means git is still the thing
/// executing the rebase, with its own state directory, reflog and conflict
/// handling intact.
///
/// `GIT_EDITOR` is pointed at an accept-as-is script for the same reason: a
/// `squash` opens an editor on the combined message, and there is no terminal
/// for it to open on. Accepting git's prepared message is what the Rebase
/// screen's preview already shows will happen.
pub fn rebase_interactive(repo: &Path, upstream: &str, todo: &str) -> Result<()> {
    let scripts = SequenceScripts::write(repo, todo)?;
    let args = ["rebase", "--interactive", upstream];

    let mut command = command(repo, &args);
    command
        .env("GIT_SEQUENCE_EDITOR", scripts.sequence_editor())
        .env("GIT_EDITOR", scripts.message_editor());

    finish(command, &args)?;
    Ok(())
}

/// Start a planned interactive rebase, without waiting for it.
///
/// The same invocation as [`rebase_interactive`], handed back as a child so the
/// caller can watch it. A rebase of a hundred commits takes long enough that a
/// blocking call would hold whatever lock the caller has for the duration, and
/// the progress the screen shows is read from git's own state directory while
/// this child is running.
///
/// The caller owns the child and must reap it. Nothing here kills it: a rebase
/// stopped by a signal leaves state on disk that only `git rebase --abort`
/// knows how to unwind.
pub fn rebase_interactive_spawn(
    repo: &Path,
    upstream: &str,
    todo: &str,
) -> Result<std::process::Child> {
    let scripts = SequenceScripts::write(repo, todo)?;
    let args = ["rebase", "--interactive", upstream];

    let mut command = command(repo, &args);
    command
        .env("GIT_SEQUENCE_EDITOR", scripts.sequence_editor())
        .env("GIT_EDITOR", scripts.message_editor());

    record_spawn(command, &args)
}

/// Carry on with a rebase that stopped, once its conflicts are resolved.
///
/// `GIT_EDITOR` is pointed at the accept-as-is script for the same reason it is
/// during the rebase itself: continuing past a `squash` or a `reword` opens an
/// editor, and there is no terminal for it to open on.
pub fn rebase_continue(repo: &Path) -> Result<()> {
    let scripts = SequenceScripts::write(repo, "")?;
    let args = ["rebase", "--continue"];

    let mut command = command(repo, &args);
    command.env("GIT_EDITOR", scripts.message_editor());

    finish(command, &args)?;
    Ok(())
}

/// Drop the commit a rebase stopped on and carry on with the rest.
pub fn rebase_skip(repo: &Path) -> Result<()> {
    run(repo, &["rebase", "--skip"])?;
    Ok(())
}

/// Unwind a rebase and put the branch back where it started.
///
/// git's own undo, and the reason nothing here reimplements the state
/// directory: `--abort` knows what to unwind because git wrote it.
pub fn rebase_abort(repo: &Path) -> Result<()> {
    run(repo, &["rebase", "--abort"])?;
    Ok(())
}

/// The two little programs a non-interactive `rebase -i` needs, on disk.
///
/// They live under the git directory rather than in the system temp directory
/// so that they are on the same filesystem as the repository, cannot be swept
/// by a temp cleaner mid-rebase, and are obvious to anyone looking at why a
/// rebase behaved the way it did.
struct SequenceScripts {
    sequence: PathBuf,
    message: PathBuf,
}

impl SequenceScripts {
    fn write(repo: &Path, todo: &str) -> Result<Self> {
        // Asked for rather than assumed to be `.git`: in a linked worktree it
        // is a file pointing elsewhere, and in a submodule it is somewhere else
        // again. `--absolute-git-dir` because the scripts are invoked with
        // whatever working directory git happens to have at the time.
        let git_dir = run(repo, &["rev-parse", "--absolute-git-dir"])?;
        let dir = PathBuf::from(git_dir.trim()).join("spagitty");
        std::fs::create_dir_all(&dir)?;

        let plan = dir.join("rebase-todo");
        std::fs::write(&plan, todo)?;

        let (sequence, message) = if cfg!(windows) {
            let sequence = dir.join("sequence-editor.bat");
            std::fs::write(
                &sequence,
                format!("@echo off\r\ncopy /Y \"{}\" %1 >nul\r\n", plan.display()),
            )?;
            let message = dir.join("message-editor.bat");
            std::fs::write(&message, "@echo off\r\nexit /b 0\r\n")?;
            (sequence, message)
        } else {
            let sequence = dir.join("sequence-editor.sh");
            std::fs::write(
                &sequence,
                format!("#!/bin/sh\ncat '{}' > \"$1\"\n", plan.display()),
            )?;
            let message = dir.join("message-editor.sh");
            std::fs::write(&message, "#!/bin/sh\nexit 0\n")?;
            make_executable(&sequence)?;
            make_executable(&message)?;
            (sequence, message)
        };

        Ok(Self { sequence, message })
    }

    fn sequence_editor(&self) -> String {
        quoted(&self.sequence)
    }

    fn message_editor(&self) -> String {
        quoted(&self.message)
    }
}

/// git runs the editor through a shell, so a path with a space in it — which is
/// every path under a Windows user profile, and plenty on macOS — has to arrive
/// already quoted.
fn quoted(path: &Path) -> String {
    format!("\"{}\"", path.display())
}

#[cfg(unix)]
fn make_executable(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut permissions = std::fs::metadata(path)?.permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(path, permissions)?;
    Ok(())
}

#[cfg(not(unix))]
fn make_executable(_path: &Path) -> Result<()> {
    Ok(())
}

// --- Refs -------------------------------------------------------------------

/// Check out a commit without a branch — a detached HEAD.
///
/// `switch --detach`, not `checkout`, and for the reason in [`checkout`]: a
/// revision that looks like a path cannot be misread as one.
pub fn checkout_detached(repo: &Path, revision: &str) -> Result<()> {
    run(repo, &["switch", "--detach", revision])?;
    Ok(())
}

/// Take one whole side of a conflicted path into the working tree.
///
/// `--ours` and `--theirs` read the index's stage 2 and stage 3, which is why
/// this is a git call rather than us writing the blob: git's answer is the one
/// the rest of the ecosystem will see, and a reconstruction could differ.
pub fn checkout_side(repo: &Path, path: &str, side: crate::conflicts::Side) -> Result<()> {
    let which = match side {
        crate::conflicts::Side::Ours => "--ours",
        crate::conflicts::Side::Theirs => "--theirs",
    };
    run(repo, &["checkout", which, "--", path])?;
    Ok(())
}

/// `git <command> --continue` or `--abort`, for whatever is in progress.
///
/// One function rather than eight, because the difference between continuing a
/// merge and continuing a cherry-pick is the word, and eight near-identical
/// wrappers is eight places for that word to be wrong.
///
/// `GIT_EDITOR` is the accept-as-is script for the same reason it is during a
/// rebase: continuing a merge or a cherry-pick opens an editor on the message,
/// and there is no terminal for it to open on.
pub fn sequencer(repo: &Path, command: &str, action: &str) -> Result<()> {
    let scripts = SequenceScripts::write(repo, "")?;
    let args = [command, action];

    let mut prepared = self::command(repo, &args);
    prepared.env("GIT_EDITOR", scripts.message_editor());

    finish(prepared, &args)?;
    Ok(())
}

/// Rename a branch.
///
/// `-m`, never `-M`: forcing would clobber an existing branch of the target
/// name, which is a destroyed branch presented as a rename.
pub fn rename_branch(repo: &Path, from: &str, to: &str) -> Result<()> {
    run(repo, &["branch", "-m", from, to])?;
    Ok(())
}

/// Delete a branch.
///
/// `force` is `-D`, which deletes a branch whose commits are not merged
/// anywhere — the one that actually loses work. It is never the default and
/// never inferred: the caller has to have been told what it means and asked.
pub fn delete_branch(repo: &Path, name: &str, force: bool) -> Result<()> {
    run(repo, &["branch", if force { "-D" } else { "-d" }, name])?;
    Ok(())
}

/// Create a tag. An empty `message` makes a lightweight tag, a message makes an
/// annotated one — which is the distinction git itself draws.
pub fn create_tag(repo: &Path, name: &str, target: &str, message: &str) -> Result<()> {
    let mut args = vec!["tag"];
    if !message.is_empty() {
        args.push("--annotate");
        args.push("--message");
        args.push(message);
    }
    args.push(name);
    if !target.is_empty() {
        args.push(target);
    }

    run(repo, &args)?;
    Ok(())
}

/// Delete a tag, locally.
pub fn delete_tag(repo: &Path, name: &str) -> Result<()> {
    run(repo, &["tag", "--delete", name])?;
    Ok(())
}

// --- Stash ------------------------------------------------------------------

/// Restore a stash and remove it. Through `git` for the same reason as
/// [`stash_push`]: a pop is a working-tree write and a ref delete at once.
pub fn stash_pop(repo: &Path, index: usize) -> Result<()> {
    run(repo, &["stash", "pop", &format!("stash@{{{index}}}")])?;
    Ok(())
}

/// Restore a stash and keep it.
pub fn stash_apply(repo: &Path, index: usize) -> Result<()> {
    run(repo, &["stash", "apply", &format!("stash@{{{index}}}")])?;
    Ok(())
}

/// Forget a stash.
///
/// The one operation here that only removes. The commit survives in the reflog
/// for git's own expiry window, and the caller is told the id before it goes —
/// see the Stash screen, which shows it in the confirmation.
pub fn stash_drop(repo: &Path, index: usize) -> Result<()> {
    run(repo, &["stash", "drop", &format!("stash@{{{index}}}")])?;
    Ok(())
}

// --- Network ----------------------------------------------------------------

/// Fetch. Goes through `git` so credential helpers and the OS keychain work.
///
/// `prune` deletes remote-tracking refs the remote no longer has. It used to be
/// passed unconditionally, which meant a destructive operation ran on every
/// fetch without anybody choosing it — the opposite of what FEAT-018 asked for
/// and of what Amendment 6 means. It is a parameter now, and the choice is the
/// caller's.
///
/// An empty `remote` means every remote.
pub fn fetch(repo: &Path, remote: &str, prune: bool) -> Result<String> {
    run(repo, &fetch_args(remote, prune))
}

/// Start a fetch without waiting for it, so its progress can be watched.
pub fn fetch_spawn(repo: &Path, remote: &str, prune: bool) -> Result<std::process::Child> {
    let args = fetch_args(remote, prune);
    record_spawn(command(repo, &args), &args)
}

fn fetch_args(remote: &str, prune: bool) -> Vec<&str> {
    let mut args = vec!["fetch", "--progress"];
    if prune {
        args.push("--prune");
    }
    if remote.is_empty() {
        args.push("--all");
    } else {
        args.push(remote);
    }
    args
}

/// How a pull should bring the remote's commits in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub enum PullMode {
    /// Refuse unless the local branch can simply move forward. The safe one:
    /// it can never write a merge commit or leave a conflict behind.
    FastForwardOnly,
    /// Fast-forward where possible, merge where not.
    Merge,
    /// Replay local commits on top of the remote's. Rewrites them, so anything
    /// already pushed needs a force push afterwards.
    Rebase,
}

/// Pull: fetch, then bring the upstream's commits into the current branch.
///
/// One `git pull` rather than [`fetch`] followed by [`merge`], because git
/// resolves which upstream the current branch tracks and that resolution is
/// exactly the part not worth reimplementing — it reads `branch.<name>.remote`,
/// `branch.<name>.merge`, and the push/pull defaults, any of which a user may
/// have configured per branch.
///
/// `--no-edit` because a merge commit's default message is the right one here
/// and there is no editor to open; `GIT_TERMINAL_PROMPT=0` from [`command`]
/// already stops it blocking on credentials.
pub fn pull(repo: &Path, remote: &str, mode: PullMode) -> Result<String> {
    let mut args = vec!["pull", "--progress", "--no-edit"];
    match mode {
        PullMode::FastForwardOnly => args.push("--ff-only"),
        PullMode::Merge => args.push("--no-rebase"),
        PullMode::Rebase => args.push("--rebase"),
    }
    if !remote.is_empty() {
        args.push(remote);
    }
    run(repo, &args)
}

// --- Remotes (FEAT-049) -----------------------------------------------------

/// Add a remote. Configuration only — nothing is fetched.
///
/// Through `git` rather than a config write because `remote add` also writes
/// the fetch refspec, and a remote without one looks configured and fetches
/// nothing.
pub fn remote_add(repo: &Path, name: &str, url: &str) -> Result<()> {
    run(repo, &["remote", "add", name, url])?;
    Ok(())
}

/// Rename a remote, its tracking refs, and every upstream pointing at it.
///
/// The three-things-at-once is the reason this is not a config edit: a rename
/// that moved only the config would leave `refs/remotes/old/` behind and every
/// branch tracking a remote that no longer exists.
pub fn remote_rename(repo: &Path, from: &str, to: &str) -> Result<()> {
    run(repo, &["remote", "rename", from, to])?;
    Ok(())
}

/// Remove a remote, its tracking refs, and the upstreams pointing at it.
pub fn remote_remove(repo: &Path, name: &str) -> Result<()> {
    run(repo, &["remote", "remove", name])?;
    Ok(())
}

/// Change where a remote points, leaving everything else as it is.
pub fn remote_set_url(repo: &Path, name: &str, url: &str) -> Result<()> {
    run(repo, &["remote", "set-url", name, url])?;
    Ok(())
}

/// Push. Same reason as [`fetch`], plus `pre-push` hooks.
///
/// `--force-with-lease` rather than `--force` when a force is asked for: it
/// refuses if the remote moved since the last fetch, which is the case where a
/// plain force destroys somebody else's work.
///
/// `--set-upstream` whenever a remote is named (FEAT-049). Without it, the
/// first push of a new branch sends the commits and leaves the branch tracking
/// nothing — so the Branches screen shows no upstream, the divergence bar has
/// nothing to compare against, and the next plain `push` or `pull` fails with a
/// message about upstreams that reads as though something is broken. Setting it
/// is what the user meant by pushing to that remote, and git does nothing when
/// the upstream is already what it would set.
pub fn push(repo: &Path, remote: &str, refspec: &str, force: bool) -> Result<String> {
    run(repo, &push_args(remote, refspec, force))
}

/// Start a push without waiting for it, so its progress can be watched.
pub fn push_spawn(
    repo: &Path,
    remote: &str,
    refspec: &str,
    force: bool,
) -> Result<std::process::Child> {
    let args = push_args(remote, refspec, force);
    record_spawn(command(repo, &args), &args)
}

fn push_args<'a>(remote: &'a str, refspec: &'a str, force: bool) -> Vec<&'a str> {
    let mut args = vec!["push", "--progress"];
    if force {
        args.push("--force-with-lease");
    }
    if !remote.is_empty() {
        args.push("--set-upstream");
        args.push(remote);
        if !refspec.is_empty() {
            args.push(refspec);
        }
    }
    args
}

/// List linked worktrees in porcelain format (FEAT-062).
pub fn worktree_list(repo: &Path) -> Result<String> {
    run(repo, &["worktree", "list", "--porcelain"])
}

/// Add a worktree pointing to a branch or path (FEAT-062).
pub fn worktree_add(
    repo: &Path,
    target: &Path,
    branch: Option<&str>,
    new_branch: Option<&str>,
    detach: bool,
) -> Result<()> {
    let target_str = target.to_str().ok_or_else(|| Error::Git {
        command: "worktree add".into(),
        stderr: "target path is not valid utf-8".into(),
    })?;
    let mut args = vec!["worktree", "add"];
    if detach {
        args.push("--detach");
    }
    if let Some(nb) = new_branch {
        if !nb.is_empty() {
            args.push("-b");
            args.push(nb);
        }
    }
    args.push(target_str);
    if let Some(b) = branch {
        if !b.is_empty() {
            args.push(b);
        }
    }
    run(repo, &args)?;
    Ok(())
}

/// Remove a worktree (FEAT-062).
pub fn worktree_remove(repo: &Path, target: &Path, force: bool) -> Result<()> {
    let target_str = target.to_str().ok_or_else(|| Error::Git {
        command: "worktree remove".into(),
        stderr: "target path is not valid utf-8".into(),
    })?;
    let mut args = vec!["worktree", "remove"];
    if force {
        args.push("--force");
    }
    args.push(target_str);
    run(repo, &args)?;
    Ok(())
}

/// Lock a worktree against pruning (FEAT-062).
pub fn worktree_lock(repo: &Path, target: &Path, reason: Option<&str>) -> Result<()> {
    let target_str = target.to_str().ok_or_else(|| Error::Git {
        command: "worktree lock".into(),
        stderr: "target path is not valid utf-8".into(),
    })?;
    let mut args = vec!["worktree", "lock"];
    if let Some(r) = reason {
        if !r.is_empty() {
            args.push("--reason");
            args.push(r);
        }
    }
    args.push(target_str);
    run(repo, &args)?;
    Ok(())
}

/// Unlock a locked worktree (FEAT-062).
pub fn worktree_unlock(repo: &Path, target: &Path) -> Result<()> {
    let target_str = target.to_str().ok_or_else(|| Error::Git {
        command: "worktree unlock".into(),
        stderr: "target path is not valid utf-8".into(),
    })?;
    run(repo, &["worktree", "unlock", target_str])?;
    Ok(())
}

/// Prune stale worktree administrative metadata (FEAT-062).
pub fn worktree_prune(repo: &Path) -> Result<()> {
    run(repo, &["worktree", "prune"])?;
    Ok(())
}

/// Read commit history touching a single path, following renames (FEAT-063).
pub fn file_history(repo: &Path, path: &str, limit: usize) -> Result<String> {
    let limit_str = format!("-{}", limit.max(1));
    run(
        repo,
        &[
            "log",
            "--follow",
            &limit_str,
            "--format=%H%x00%h%x00%an%x00%ae%x00%at%x00%s",
            "--",
            path,
        ],
    )
}

/// Read submodule status (FEAT-067).
pub fn submodule_status(repo: &Path) -> Result<String> {
    run(repo, &["submodule", "status"])
}

/// Update submodules recursively (FEAT-067).
pub fn submodule_update(
    repo: &Path,
    paths: &[String],
    init: bool,
    recursive: bool,
) -> Result<String> {
    let mut args = vec!["submodule", "update"];
    if init {
        args.push("--init");
    }
    if recursive {
        args.push("--recursive");
    }
    for p in paths {
        args.push(p.as_str());
    }
    run(repo, &args)
}

/// Sync submodule URLs from .gitmodules (FEAT-067).
pub fn submodule_sync(repo: &Path, recursive: bool) -> Result<String> {
    let mut args = vec!["submodule", "sync"];
    if recursive {
        args.push("--recursive");
    }
    run(repo, &args)
}

/// De-initialize a submodule (FEAT-067).
pub fn submodule_deinit(repo: &Path, path: &str, force: bool) -> Result<String> {
    let mut args = vec!["submodule", "deinit"];
    if force {
        args.push("-f");
    }
    args.push("--");
    args.push(path);
    run(repo, &args)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture::Fixture;

    #[test]
    fn the_git_version_is_reported_from_the_binary_on_path() {
        let fixture = Fixture::empty();
        let version = version(fixture.path()).expect("a git version");

        assert!(version.starts_with("git version"), "unexpected: {version}");
        assert_eq!(version.trim(), version, "the caller gets a clean string");
    }

    #[test]
    fn a_clone_produces_the_same_repository_git_clone_would() {
        // Criterion 1, over a local path so the test needs no network. The
        // clone is a real `git clone`, which is the whole point of the boundary.
        //
        // The gate is taken because this writes to the process-wide record, and
        // a test that reads the record must not have another test's clone
        // appear in the middle of it (BUG-023).
        let _gate = crate::record::test_gate();
        let source = Fixture::woven();
        let into = tempfile::tempdir().expect("temp dir");
        let destination = into.path().join("project");

        let status = clone_start(&source.path().to_string_lossy(), &destination)
            .expect("starting the clone")
            .wait()
            .expect("waiting for the clone");

        assert!(status.success(), "the clone failed");
        assert!(
            destination.join(".git").is_dir(),
            "a repository was created"
        );

        let cloned = crate::repo::open(&destination).expect("opening the clone");
        assert_eq!(
            crate::repo::head(&cloned).id.as_deref(),
            Some(source.head().as_str()),
            "the clone is at the same commit"
        );
    }

    #[test]
    fn a_clone_reports_progress_on_stderr_rather_than_running_silently() {
        // Criterion 4's data half: without `--progress`, git writes nothing to
        // a pipe and the screen would sit frozen for the whole clone.
        use std::io::Read;

        // Writes to the record; see the note in the test above (BUG-023).
        let _gate = crate::record::test_gate();

        let source = Fixture::woven();
        let into = tempfile::tempdir().expect("temp dir");

        let mut child = clone_start(
            &source.path().to_string_lossy(),
            &into.path().join("project"),
        )
        .expect("starting the clone");

        let mut noise = String::new();
        child
            .stderr
            .take()
            .expect("piped stderr")
            .read_to_string(&mut noise)
            .expect("reading stderr");
        child.wait().expect("waiting for the clone");

        assert!(
            crate::clone::progress(noise.lines().next().unwrap_or_default()).is_some(),
            "git said nothing: {noise:?}"
        );
    }

    #[test]
    fn every_execution_reaches_the_record_with_the_flags_this_module_added() {
        let _gate = crate::record::test_gate();
        // The point of recording here rather than in a screen: `--prune` and
        // `--all` are added on the way down, and a screen reporting "git fetch"
        // would be describing a command nobody ran.
        let fixture = Fixture::woven();
        let before = crate::record::recent(0).last().map_or(0, |entry| entry.seq);

        let _ = fetch(fixture.path(), "", true);

        let entry = crate::record::recent(before)
            .into_iter()
            .find(|entry| entry.argv.get(1).map(String::as_str) == Some("fetch"))
            .expect("the fetch was recorded");

        assert_eq!(entry.line(), "git fetch --progress --prune --all");
    }

    #[test]
    fn a_fetch_that_is_not_pruning_says_so_by_omission() {
        // FEAT-018. `--prune` used to be unconditional, so the log could not
        // tell a fetch that deleted refs from one that did not. Now it can.
        let _gate = crate::record::test_gate();
        let fixture = Fixture::woven();
        let before = crate::record::recent(0).last().map_or(0, |entry| entry.seq);

        let _ = fetch(fixture.path(), "origin", false);

        let entry = crate::record::recent(before)
            .into_iter()
            .find(|entry| entry.argv.get(1).map(String::as_str) == Some("fetch"))
            .expect("the fetch was recorded");

        assert_eq!(entry.line(), "git fetch --progress origin");
    }

    #[test]
    fn a_failing_command_is_recorded_with_its_exit_code_and_gits_words() {
        let _gate = crate::record::test_gate();
        let fixture = Fixture::empty();
        let before = crate::record::recent(0).last().map_or(0, |entry| entry.seq);

        let _ = run(
            fixture.path(),
            &["rev-parse", "--verify", "refs/heads/nope"],
        );

        let entry = crate::record::recent(before)
            .into_iter()
            .find(|entry| entry.argv.contains(&"refs/heads/nope".to_string()))
            .expect("the failure was recorded");

        match entry.outcome {
            crate::record::Outcome::Failed { code, stderr } => {
                assert_eq!(code, Some(128), "git's own exit code is kept");
                assert!(!stderr.is_empty(), "git's message is kept");
            }
            other => panic!("expected a failure, got {other:?}"),
        }
    }

    #[test]
    fn a_clone_is_recorded_at_spawn_with_its_credentials_removed() {
        let _gate = crate::record::test_gate();
        // Nothing waits for a clone, so the record cannot wait either — and the
        // URL is the one argument in Spagitty that can carry a live secret.
        let source = Fixture::woven();
        let into = tempfile::tempdir().expect("temp dir");
        let before = crate::record::recent(0).last().map_or(0, |entry| entry.seq);

        // A URL git will fail on, which is fine: the record is written at spawn
        // and the failure arrives later, on the child nobody is waiting for.
        let url = format!(
            "https://maxmya:ghp_secret@example.invalid{}",
            source.path().display()
        );
        let mut child = clone_start(&url, &into.path().join("project")).expect("git was spawned");

        // *This* clone, not whichever clone happened to be recorded next: the
        // record is process-wide and two other tests in this module clone as
        // well. Matching on the host is what makes the assertion about the
        // entry this test produced (BUG-023).
        let entry = crate::record::recent(before)
            .into_iter()
            .find(|entry| {
                entry.argv.get(1).map(String::as_str) == Some("clone")
                    && entry.line().contains("example.invalid")
            })
            .expect("the clone was recorded");

        assert_eq!(entry.outcome, crate::record::Outcome::Started);
        assert!(
            !entry.line().contains("ghp_secret"),
            "a credential reached the log: {}",
            entry.line()
        );
        assert!(
            entry.line().contains("maxmya:***@example.invalid"),
            "the URL stopped being recognisable: {}",
            entry.line()
        );

        let _ = child.wait();
    }

    #[test]
    fn a_failing_command_carries_gits_own_stderr() {
        // git's message is almost always more useful than anything we would
        // write in its place, so it is what the error holds.
        let fixture = Fixture::empty();

        let error = run(
            fixture.path(),
            &["rev-parse", "--verify", "refs/heads/nope"],
        )
        .unwrap_err();

        match error {
            Error::Git { command, stderr } => {
                assert_eq!(command, "rev-parse --verify refs/heads/nope");
                assert!(
                    !stderr.is_empty(),
                    "git said nothing, which cannot be right"
                );
            }
            other => panic!("expected a Git error, got {other:?}"),
        }
    }
}

#[cfg(test)]
mod farm_merge_tests {
    use super::*;
    use crate::fixture::Fixture;

    #[test]
    fn a_destination_change_is_rejected_before_merge() {
        let fixture = Fixture::woven();
        let before = fixture.git(&["rev-parse", "HEAD"]);
        let source = fixture.git(&["rev-parse", "HEAD~1"]);
        assert!(merge_into(fixture.path(), source.trim(), "unrelated").is_err());
        assert_eq!(fixture.git(&["rev-parse", "HEAD"]), before);
    }

    #[test]
    fn concurrent_merges_into_one_checkout_preserve_both_changes() {
        let fixture = Arc::new(Fixture::woven());
        let mut commits = Vec::new();
        for name in ["merge-one", "merge-two"] {
            fixture.git(&["switch", "-c", name, "main"]);
            std::fs::write(fixture.path().join(name), name).unwrap();
            fixture.git(&["add", name]);
            fixture.git(&["commit", "-qm", name]);
            commits.push(fixture.git(&["rev-parse", "HEAD"]).trim().to_string());
        }
        fixture.git(&["switch", "main"]);
        let barrier = Arc::new(std::sync::Barrier::new(2));
        let workers: Vec<_> = commits
            .into_iter()
            .map(|commit| {
                let fixture = fixture.clone();
                let barrier = barrier.clone();
                std::thread::spawn(move || {
                    barrier.wait();
                    merge_into(fixture.path(), &commit, "main")
                })
            })
            .collect();
        for worker in workers {
            worker.join().unwrap().unwrap();
        }
        assert!(fixture.path().join("merge-one").exists());
        assert!(fixture.path().join("merge-two").exists());
        assert_eq!(fixture.git(&["branch", "--show-current"]).trim(), "main");
    }
}
