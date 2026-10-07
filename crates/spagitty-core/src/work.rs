// SPDX-License-Identifier: GPL-3.0-or-later

//! Changing the working copy: staging, unstaging and committing.
//!
//! Reading the working copy lives in [`crate::status`]. This module is the
//! writing half, and every operation in it goes through [`crate::shell`] — the
//! index and the commit are exactly the state the rest of the git ecosystem
//! reads, and hooks and signing have to run. The header of that module has the
//! full argument.
//!
//! Everything here but [`discard`] and [`discard_hunk`] is reversible: it moves
//! something between the working tree and the index, or turns the index into a
//! commit, and a mistake costs an unstage. Those two are not, and they are the
//! only operations in Spagitty that destroy work git cannot get back — there is
//! no reflog for the working tree. They were left out until FEAT-048 for that
//! reason, and the reason turned out to be the wrong one: a client that cannot
//! throw away a mistake sends people to the terminal for the most ordinary
//! thing in a working day. What guards them is a confirmation on the way in and
//! a narrow definition of what they touch, both written down below.

use std::path::Path;

use crate::diff::{self, FileStatus, Side};
use crate::error::{Error, Result};
use crate::shell;
use crate::signing;

/// Stage whole paths.
pub fn stage(repo: &gix::Repository, paths: &[String]) -> Result<()> {
    shell::stage(workdir(repo)?, paths)
}

/// Unstage whole paths. The working tree is untouched.
pub fn unstage(repo: &gix::Repository, paths: &[String]) -> Result<()> {
    shell::unstage(workdir(repo)?, paths)
}

/// Stage one hunk of one file.
///
/// `index` and `header` identify the hunk in the *unstaged* diff of `path`.
/// The patch is rebuilt from the bytes as they are now and refused if the
/// header no longer matches, so a file that changed under an open screen
/// cannot be half-staged from a stale view.
pub fn stage_hunk(repo: &gix::Repository, path: &str, index: usize, header: &str) -> Result<()> {
    let patch = diff::working_hunk_patch(repo, path, Side::Unstaged, index, header)?;
    shell::apply_to_index(workdir(repo)?, &patch, false)
}

/// Unstage one hunk of one file.
///
/// The mirror image: `index` and `header` identify the hunk in the *staged*
/// diff, and the patch is applied to the index in reverse.
pub fn unstage_hunk(repo: &gix::Repository, path: &str, index: usize, header: &str) -> Result<()> {
    let patch = diff::working_hunk_patch(repo, path, Side::Staged, index, header)?;
    shell::apply_to_index(workdir(repo)?, &patch, true)
}

/// Throw away unstaged changes to whole paths.
///
/// "Unstaged" is the whole definition, and it is what makes this safe enough to
/// offer: a tracked path goes back to what the index holds, so anything already
/// staged survives, and an untracked path is deleted, because there is no
/// earlier version of it to go back to. A path that is both staged and changed
/// again keeps its staged part.
///
/// The split is read from the status rather than taken from the caller. The
/// screen knows which column a row came from, but a row can be stale, and
/// telling `git restore` about a file it does not track fails the whole call —
/// so the classification is done here, against the repository as it is now.
///
/// Nothing here can be undone. The caller confirms first.
pub fn discard(repo: &gix::Repository, paths: &[String]) -> Result<()> {
    let workdir = workdir(repo)?;
    let work = crate::status::working_copy(repo)?;

    let untracked: Vec<String> = work
        .unstaged
        .iter()
        .filter(|entry| entry.status == FileStatus::Untracked)
        .map(|entry| entry.path.clone())
        .collect();

    let (remove, restore): (Vec<String>, Vec<String>) = paths
        .iter()
        .cloned()
        .partition(|path| untracked.iter().any(|known| known == path));

    // Restore first. If the clean fails — a permission, a file that vanished —
    // the tracked half has still happened, and the error names what is left.
    shell::discard(workdir, &restore)?;
    shell::remove_untracked(workdir, &remove)
}

/// Throw away one unstaged hunk of one file.
///
/// `index` and `header` identify the hunk in the *unstaged* diff of `path`, the
/// same way [`stage_hunk`] does, and the patch is refused if the header no
/// longer matches — a file that changed under an open screen cannot have the
/// wrong part of it discarded from a stale view. That check matters more here
/// than it does for staging: staging the wrong hunk is undone by unstaging it.
pub fn discard_hunk(repo: &gix::Repository, path: &str, index: usize, header: &str) -> Result<()> {
    let patch = diff::working_hunk_patch(repo, path, Side::Unstaged, index, header)?;
    shell::apply_to_worktree(workdir(repo)?, &patch, true)
}

/// Commit what is staged. Returns the new commit's id.
///
/// The subject is required: a commit with no subject line is one nobody can
/// read in a log, and git would open an editor we have no terminal for.
pub fn commit(repo: &gix::Repository, subject: &str, body: &str, amend: bool) -> Result<String> {
    commit_with(repo, subject, body, amend, false, &mut |_| {})
}

/// [`commit`], with a say over the hooks and their output as it is printed
/// (FEAT-107).
///
/// Hooks are skipped when `skip_hooks` asks for this one commit, or when this
/// repository's hooks are switched off ([`crate::hooks::enabled`]) — read
/// here, at the moment of committing, so a screen that read the switch a
/// minute ago cannot run hooks the person has since turned off.
pub fn commit_with(
    repo: &gix::Repository,
    subject: &str,
    body: &str,
    amend: bool,
    skip_hooks: bool,
    line: &mut dyn FnMut(&str),
) -> Result<String> {
    let subject = subject.trim();
    if subject.is_empty() {
        return Err(Error::EmptyMessage);
    }

    // Read rather than passed in: `commit.gpgsign` is the authority, and a
    // caller that carried its own answer could disagree with the configuration
    // between the screen reading it and the commit happening.
    let signing = signing::read(repo);

    let dir = workdir(repo)?;
    let skip = skip_hooks || !crate::hooks::enabled(dir)?;
    shell::commit_with(dir, subject, body, amend, signing.enabled, skip, line)
        .map_err(|error| signing::as_signing_failure(error, &signing.program))
}

/// The message of the commit `HEAD` points at, for pre-filling an amend.
///
/// An unborn `HEAD` has no message to offer, and amending is not possible
/// there either — the screen asks first and gets an empty string.
pub fn head_message(repo: &gix::Repository) -> Result<String> {
    if repo.head_id().is_err() {
        return Ok(String::new());
    }
    Ok(shell::head_message(workdir(repo)?)?.trim().to_string())
}

/// The working directory to run `git` in.
///
/// A bare repository has none, and none of these operations means anything
/// without one.
fn workdir(repo: &gix::Repository) -> Result<&Path> {
    repo.workdir()
        .ok_or_else(|| Error::NotStageable("a bare repository has no working copy".into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture::Fixture;
    use crate::status::working_copy;

    fn paths(entries: &[crate::status::StatusEntry]) -> Vec<String> {
        entries.iter().map(|e| e.path.clone()).collect()
    }

    #[test]
    fn staging_a_path_moves_it_from_unstaged_to_staged() {
        let fixture = Fixture::woven();
        fixture.write("notes.md", "changed\n");

        stage(&fixture.open(), &["notes.md".to_string()]).expect("stage");

        let work = working_copy(&fixture.open()).expect("status");
        assert_eq!(paths(&work.staged), vec!["notes.md"]);
        assert!(paths(&work.unstaged).is_empty());
    }

    #[test]
    fn staging_is_visible_to_git_immediately() {
        let fixture = Fixture::woven();
        fixture.write("notes.md", "changed\n");

        stage(&fixture.open(), &["notes.md".to_string()]).expect("stage");

        // `M ` in the first column is staged, ` M` is not.
        let porcelain = fixture.git(&["status", "--porcelain", "notes.md"]);
        assert!(porcelain.starts_with("M "), "unexpected: {porcelain:?}");
    }

    #[test]
    fn staging_an_untracked_file_adds_it() {
        let fixture = Fixture::woven();
        fixture.write("brand-new.txt", "hello\n");

        stage(&fixture.open(), &["brand-new.txt".to_string()]).expect("stage");

        let work = working_copy(&fixture.open()).expect("status");
        assert!(paths(&work.staged).contains(&"brand-new.txt".to_string()));
    }

    #[test]
    fn staging_a_deletion_stages_the_deletion() {
        let fixture = Fixture::woven();
        fixture.remove("notes.md");

        stage(&fixture.open(), &["notes.md".to_string()]).expect("stage");

        let work = working_copy(&fixture.open()).expect("status");
        assert!(paths(&work.staged).contains(&"notes.md".to_string()));
        assert!(!paths(&work.unstaged).contains(&"notes.md".to_string()));
    }

    #[test]
    fn stage_then_unstage_leaves_the_index_byte_for_byte_as_it_was() {
        let fixture = Fixture::woven();
        let before = fixture.git(&["ls-files", "--stage"]);
        fixture.write("notes.md", "changed\n");

        let repo = fixture.open();
        stage(&repo, &["notes.md".to_string()]).expect("stage");
        unstage(&fixture.open(), &["notes.md".to_string()]).expect("unstage");

        assert_eq!(fixture.git(&["ls-files", "--stage"]), before);
    }

    #[test]
    fn unstaging_never_touches_the_working_tree() {
        let fixture = Fixture::woven();
        fixture.write("notes.md", "my work\n");
        stage(&fixture.open(), &["notes.md".to_string()]).expect("stage");

        unstage(&fixture.open(), &["notes.md".to_string()]).expect("unstage");

        let on_disk = std::fs::read_to_string(fixture.path().join("notes.md")).expect("read");
        assert_eq!(on_disk, "my work\n", "the file on disk must be untouched");
    }

    #[test]
    fn unstaging_in_a_repository_with_no_commits_removes_the_entry() {
        // There is no HEAD to restore from, and the only way to be staged here
        // is to be new — so removing the entry is the same operation.
        let fixture = Fixture::empty();
        fixture.write("first.txt", "hello\n");
        fixture.git(&["add", "first.txt"]);

        unstage(&fixture.open(), &["first.txt".to_string()]).expect("unstage");

        let work = working_copy(&fixture.open()).expect("status");
        assert!(work.staged.is_empty());
        assert!(fixture.path().join("first.txt").exists(), "the file stays");
    }

    #[test]
    fn staging_nothing_is_not_an_error_and_does_nothing() {
        let fixture = Fixture::dirty();
        let before = fixture.git(&["ls-files", "--stage"]);

        stage(&fixture.open(), &[]).expect("stage");
        unstage(&fixture.open(), &[]).expect("unstage");

        assert_eq!(fixture.git(&["ls-files", "--stage"]), before);
    }

    #[test]
    fn a_path_that_looks_like_an_option_is_still_a_path() {
        let fixture = Fixture::woven();
        fixture.write("--weird-name", "hello\n");

        stage(&fixture.open(), &["--weird-name".to_string()]).expect("stage");

        let work = working_copy(&fixture.open()).expect("status");
        assert!(paths(&work.staged).contains(&"--weird-name".to_string()));
    }

    // --- Hunk staging ------------------------------------------------------

    /// A file with two changes far enough apart to be separate hunks.
    fn two_hunks(fixture: &Fixture) -> String {
        let lines: String = (1..=40).map(|n| format!("line {n}\n")).collect();
        fixture.write("wide.txt", &lines);
        fixture.git(&["add", "wide.txt"]);
        fixture.commit("Add a wide file");

        let changed = lines
            .replace("line 2\n", "LINE TWO\n")
            .replace("line 38\n", "LINE THIRTY-EIGHT\n");
        fixture.write("wide.txt", &changed);
        changed
    }

    fn unstaged_hunks(fixture: &Fixture, path: &str) -> Vec<crate::diff::Hunk> {
        diff::working_file_diff(&fixture.open(), path, Side::Unstaged)
            .expect("diff")
            .hunks
    }

    /// A repository that checks files out with CRLF, as Git for Windows does
    /// by default, with `wide.txt` committed and two lines changed on disk.
    ///
    /// The repository stores LF; the file on disk has CRLF on *every* line,
    /// changed or not. Read raw, every line differs by its `\r` (BUG-033).
    fn two_hunks_with_crlf(fixture: &Fixture) {
        fixture.git(&["config", "core.autocrlf", "true"]);
        two_hunks(fixture);
        let on_disk = fixture.read("wide.txt").replace('\n', "\r\n");
        fixture.write("wide.txt", &on_disk);
    }

    fn changed_lines(hunks: &[crate::diff::Hunk]) -> Vec<(char, String)> {
        hunks
            .iter()
            .flat_map(|hunk| &hunk.lines)
            .filter_map(|line| match line.origin {
                crate::diff::LineOrigin::Added => Some(('+', line.text.clone())),
                crate::diff::LineOrigin::Removed => Some(('-', line.text.clone())),
                crate::diff::LineOrigin::Context => None,
            })
            .collect()
    }

    #[test]
    fn a_crlf_file_diffs_only_the_lines_that_changed() {
        let fixture = Fixture::woven();
        two_hunks_with_crlf(&fixture);

        let hunks = unstaged_hunks(&fixture, "wide.txt");

        assert_eq!(hunks.len(), 2, "two changes, not one whole-file rewrite");
        assert_eq!(
            changed_lines(&hunks),
            vec![
                ('-', "line 2".to_string()),
                ('+', "LINE TWO".to_string()),
                ('-', "line 38".to_string()),
                ('+', "LINE THIRTY-EIGHT".to_string()),
            ]
        );
    }

    #[test]
    fn staging_one_hunk_of_a_crlf_file_stages_only_that_hunk() {
        let fixture = Fixture::woven();
        two_hunks_with_crlf(&fixture);
        let hunks = unstaged_hunks(&fixture, "wide.txt");

        stage_hunk(&fixture.open(), "wide.txt", 0, &hunks[0].header).expect("stage hunk");

        let staged = fixture.git(&["show", ":wide.txt"]);
        assert!(staged.contains("LINE TWO\n"));
        assert!(
            staged.contains("line 38\n"),
            "the other hunk stays unstaged"
        );
        assert!(
            !staged.contains('\r'),
            "the index keeps the repository's endings"
        );
    }

    #[test]
    fn discarding_one_hunk_of_a_crlf_file_keeps_its_endings() {
        let fixture = Fixture::woven();
        two_hunks_with_crlf(&fixture);
        let hunks = unstaged_hunks(&fixture, "wide.txt");

        discard_hunk(&fixture.open(), "wide.txt", 0, &hunks[0].header).expect("discard hunk");

        let now = fixture.read("wide.txt");
        assert!(now.contains("line 2\r\n"), "the first hunk is gone");
        assert!(
            now.contains("LINE THIRTY-EIGHT\r\n"),
            "the second is still there"
        );
        assert!(
            !now.replace("\r\n", "").contains('\n'),
            "every line keeps its CRLF"
        );
    }

    #[test]
    fn staging_one_hunk_stages_only_that_hunk() {
        let fixture = Fixture::woven();
        two_hunks(&fixture);

        let hunks = unstaged_hunks(&fixture, "wide.txt");
        assert_eq!(hunks.len(), 2);

        stage_hunk(&fixture.open(), "wide.txt", 0, &hunks[0].header).expect("stage hunk");

        let staged = diff::working_file_diff(&fixture.open(), "wide.txt", Side::Staged)
            .expect("staged diff");
        assert_eq!(staged.hunks.len(), 1);
        assert!(staged.hunks[0].lines.iter().any(|l| l.text == "LINE TWO"));

        let left = unstaged_hunks(&fixture, "wide.txt");
        assert_eq!(left.len(), 1, "the other hunk is still unstaged");
        assert!(left[0].lines.iter().any(|l| l.text == "LINE THIRTY-EIGHT"));
    }

    #[test]
    fn a_partly_staged_file_appears_in_both_lists() {
        let fixture = Fixture::woven();
        two_hunks(&fixture);
        let hunks = unstaged_hunks(&fixture, "wide.txt");

        stage_hunk(&fixture.open(), "wide.txt", 0, &hunks[0].header).expect("stage hunk");

        let work = working_copy(&fixture.open()).expect("status");
        assert!(paths(&work.staged).contains(&"wide.txt".to_string()));
        assert!(paths(&work.unstaged).contains(&"wide.txt".to_string()));
    }

    #[test]
    fn staging_a_hunk_never_touches_the_file_on_disk() {
        let fixture = Fixture::woven();
        let expected = two_hunks(&fixture);
        let hunks = unstaged_hunks(&fixture, "wide.txt");

        stage_hunk(&fixture.open(), "wide.txt", 0, &hunks[0].header).expect("stage hunk");

        let on_disk = std::fs::read_to_string(fixture.path().join("wide.txt")).expect("read");
        assert_eq!(
            on_disk, expected,
            "staging part of a file must not lose the rest"
        );
    }

    #[test]
    fn staging_a_hunk_and_unstaging_it_again_is_a_round_trip() {
        let fixture = Fixture::woven();
        two_hunks(&fixture);
        let before = fixture.git(&["ls-files", "--stage"]);

        let hunks = unstaged_hunks(&fixture, "wide.txt");
        stage_hunk(&fixture.open(), "wide.txt", 0, &hunks[0].header).expect("stage hunk");

        let staged = diff::working_file_diff(&fixture.open(), "wide.txt", Side::Staged)
            .expect("staged diff");
        unstage_hunk(&fixture.open(), "wide.txt", 0, &staged.hunks[0].header).expect("unstage");

        assert_eq!(fixture.git(&["ls-files", "--stage"]), before);
    }

    #[test]
    fn a_hunk_that_has_moved_is_refused_rather_than_half_applied() {
        let fixture = Fixture::woven();
        two_hunks(&fixture);
        let hunks = unstaged_hunks(&fixture, "wide.txt");

        // The screen's view is now stale: the file changed underneath it.
        fixture.write("wide.txt", "something else entirely\n");

        let error = stage_hunk(&fixture.open(), "wide.txt", 0, &hunks[0].header).unwrap_err();

        assert!(matches!(error, Error::Stale(p) if p == "wide.txt"));
    }

    #[test]
    fn a_hunk_index_past_the_end_is_refused() {
        let fixture = Fixture::woven();
        two_hunks(&fixture);
        let hunks = unstaged_hunks(&fixture, "wide.txt");

        let error = stage_hunk(&fixture.open(), "wide.txt", 9, &hunks[0].header).unwrap_err();

        assert!(matches!(error, Error::Stale(_)));
    }

    #[test]
    fn a_file_without_a_trailing_newline_does_not_gain_one() {
        // Applying a patch that silently adds a terminator is a content change
        // nobody asked for.
        let fixture = Fixture::woven();
        fixture.write("tail.txt", "one\ntwo\nthree");
        fixture.git(&["add", "tail.txt"]);
        fixture.commit("Add a file with no trailing newline");

        fixture.write("tail.txt", "one\ntwo\nTHREE");
        let hunks = unstaged_hunks(&fixture, "tail.txt");
        stage_hunk(&fixture.open(), "tail.txt", 0, &hunks[0].header).expect("stage hunk");

        let staged = fixture.git(&["show", ":tail.txt"]);
        assert_eq!(staged, "one\ntwo\nTHREE");
    }

    #[test]
    fn a_binary_file_has_no_hunks_to_stage() {
        let fixture = Fixture::woven();
        fixture.write_bytes("logo.bin", &[0x00, 0xff, 0x00]);

        let error = stage_hunk(&fixture.open(), "logo.bin", 0, "@@ -1 +1 @@").unwrap_err();

        assert!(matches!(error, Error::NotStageable(_)));
    }

    // --- Committing --------------------------------------------------------

    #[test]
    fn committing_writes_what_is_staged_and_nothing_else() {
        let fixture = Fixture::woven();
        fixture.write("notes.md", "staged\n");
        fixture.write("core.txt", "not staged\n");
        stage(&fixture.open(), &["notes.md".to_string()]).expect("stage");

        let id = commit(&fixture.open(), "A subject", "", false).expect("commit");

        assert_eq!(id, fixture.head());
        let touched = fixture.git(&["show", "--name-only", "--format=", &id]);
        assert_eq!(touched.trim(), "notes.md");

        let work = working_copy(&fixture.open()).expect("status");
        assert!(paths(&work.staged).is_empty());
        assert!(paths(&work.unstaged).contains(&"core.txt".to_string()));
    }

    #[test]
    fn a_subject_and_a_body_become_a_message_with_a_blank_line_between() {
        let fixture = Fixture::woven();
        fixture.write("notes.md", "staged\n");
        stage(&fixture.open(), &["notes.md".to_string()]).expect("stage");

        commit(&fixture.open(), "A subject", "A body paragraph.", false).expect("commit");

        let message = fixture.git(&["log", "-1", "--pretty=%B"]);
        assert_eq!(message.trim(), "A subject\n\nA body paragraph.");
    }

    #[test]
    fn an_empty_subject_is_refused_before_anything_runs() {
        let fixture = Fixture::woven();
        fixture.write("notes.md", "staged\n");
        stage(&fixture.open(), &["notes.md".to_string()]).expect("stage");
        let before = fixture.head();

        let error = commit(&fixture.open(), "   ", "a body", false).unwrap_err();

        assert!(matches!(error, Error::EmptyMessage));
        assert_eq!(fixture.head(), before, "nothing was committed");
    }

    #[test]
    fn committing_nothing_is_refused_by_git_and_leaves_head_alone() {
        let fixture = Fixture::woven();
        let before = fixture.head();

        let error = commit(&fixture.open(), "Nothing to say", "", false).unwrap_err();

        assert!(matches!(error, Error::Git { .. }));
        assert_eq!(fixture.head(), before);
    }

    #[test]
    fn a_failing_pre_commit_hook_stops_the_commit_and_says_why() {
        let fixture = Fixture::woven();
        let hook = fixture.path().join(".git/hooks/pre-commit");
        std::fs::write(&hook, "#!/bin/sh\necho 'the hook says no' >&2\nexit 1\n").expect("hook");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&hook, std::fs::Permissions::from_mode(0o755)).expect("chmod");
        }

        fixture.write("notes.md", "staged\n");
        stage(&fixture.open(), &["notes.md".to_string()]).expect("stage");
        let before = fixture.head();

        let error = commit(&fixture.open(), "Blocked", "", false).unwrap_err();

        match error {
            Error::Git { stderr, .. } => assert!(stderr.contains("the hook says no")),
            other => panic!("expected git's own message, got {other:?}"),
        }
        assert_eq!(fixture.head(), before);
    }

    #[test]
    fn amending_replaces_the_previous_commit_rather_than_adding_one() {
        let fixture = Fixture::woven();
        let before = fixture.git(&["rev-list", "--count", "HEAD"]);

        fixture.write("notes.md", "amended\n");
        stage(&fixture.open(), &["notes.md".to_string()]).expect("stage");
        commit(&fixture.open(), "Reworded", "", true).expect("amend");

        assert_eq!(fixture.git(&["rev-list", "--count", "HEAD"]), before);
        assert_eq!(
            fixture.git(&["log", "-1", "--pretty=%s"]).trim(),
            "Reworded"
        );
    }

    #[test]
    fn discarding_a_path_puts_the_file_back_the_way_the_index_has_it() {
        let fixture = Fixture::woven();
        let before = fixture.read("notes.md");
        fixture.write("notes.md", "wrong\n");

        discard(&fixture.open(), &["notes.md".to_string()]).expect("discard");

        assert_eq!(fixture.read("notes.md"), before);
        let work = working_copy(&fixture.open()).expect("status");
        assert!(paths(&work.unstaged).is_empty());
    }

    #[test]
    fn discarding_keeps_what_was_already_staged() {
        // The whole reason `--staged` is not passed: a decision already made
        // must survive throwing away the change made after it.
        let fixture = Fixture::woven();
        fixture.write("notes.md", "staged\n");
        stage(&fixture.open(), &["notes.md".to_string()]).expect("stage");
        fixture.write("notes.md", "staged\nand then more\n");

        discard(&fixture.open(), &["notes.md".to_string()]).expect("discard");

        assert_eq!(fixture.read("notes.md"), "staged\n");
        let work = working_copy(&fixture.open()).expect("status");
        assert_eq!(paths(&work.staged), vec!["notes.md"]);
        assert!(paths(&work.unstaged).is_empty());
    }

    #[test]
    fn discarding_an_untracked_file_deletes_it() {
        // There is no earlier version to restore, so the only meaning discard
        // can have here is removal.
        let fixture = Fixture::woven();
        fixture.write("scratch.txt", "notes to self\n");

        discard(&fixture.open(), &["scratch.txt".to_string()]).expect("discard");

        let work = working_copy(&fixture.open()).expect("status");
        assert!(paths(&work.unstaged).is_empty());
        assert!(!fixture.at("scratch.txt").exists());
    }

    #[test]
    fn discarding_a_deletion_brings_the_file_back() {
        let fixture = Fixture::woven();
        let before = fixture.read("notes.md");
        fixture.remove("notes.md");

        discard(&fixture.open(), &["notes.md".to_string()]).expect("discard");

        assert_eq!(fixture.read("notes.md"), before);
    }

    #[test]
    fn discarding_a_mixed_selection_handles_each_path_by_what_it_is() {
        let fixture = Fixture::woven();
        fixture.write("notes.md", "wrong\n");
        fixture.write("scratch.txt", "notes to self\n");

        discard(
            &fixture.open(),
            &["notes.md".to_string(), "scratch.txt".to_string()],
        )
        .expect("discard");

        let work = working_copy(&fixture.open()).expect("status");
        assert!(paths(&work.unstaged).is_empty());
        assert!(!fixture.at("scratch.txt").exists());
    }

    #[test]
    fn discarding_never_touches_an_ignored_file() {
        // `git clean` without `-x`. An ignored file is one the screen never
        // showed, and deleting it would be the surprise this operation cannot
        // afford to spring.
        let fixture = Fixture::woven();
        fixture.write(".cache/build.log", "noise\n");

        discard(&fixture.open(), &["notes.md".to_string()]).expect("discard");

        assert!(fixture.at(".cache/build.log").exists());
    }

    #[test]
    fn discarding_nothing_is_not_an_error_and_does_nothing() {
        let fixture = Fixture::woven();
        fixture.write("notes.md", "changed\n");

        discard(&fixture.open(), &[]).expect("discard");

        assert_eq!(fixture.read("notes.md"), "changed\n");
    }

    #[test]
    fn discarding_one_hunk_discards_only_that_hunk() {
        let fixture = Fixture::woven();
        let changed = two_hunks(&fixture);
        let hunks = unstaged_hunks(&fixture, "wide.txt");
        assert_eq!(hunks.len(), 2);

        discard_hunk(&fixture.open(), "wide.txt", 0, &hunks[0].header).expect("discard hunk");

        let now = fixture.read("wide.txt");
        assert!(now.contains("line 2\n"), "the first hunk should be gone");
        assert!(
            now.contains("LINE THIRTY-EIGHT\n"),
            "the second hunk should still be there"
        );
        assert_ne!(now, changed);
    }

    #[test]
    fn discarding_a_hunk_never_touches_the_index() {
        let fixture = Fixture::woven();
        two_hunks(&fixture);
        let before = fixture.git(&["ls-files", "--stage"]);
        let hunks = unstaged_hunks(&fixture, "wide.txt");

        discard_hunk(&fixture.open(), "wide.txt", 0, &hunks[0].header).expect("discard hunk");

        assert_eq!(fixture.git(&["ls-files", "--stage"]), before);
    }

    #[test]
    fn a_hunk_that_has_moved_is_refused_rather_than_discarded_from_a_stale_view() {
        // Staging the wrong hunk costs an unstage. Discarding the wrong one
        // costs the work, so the staleness check matters more here.
        let fixture = Fixture::woven();
        two_hunks(&fixture);
        let hunks = unstaged_hunks(&fixture, "wide.txt");

        fixture.write("wide.txt", "something else entirely\n");

        let error = discard_hunk(&fixture.open(), "wide.txt", 0, &hunks[0].header).unwrap_err();

        assert!(matches!(error, Error::Stale(p) if p == "wide.txt"));
    }

    #[test]
    fn the_head_message_is_offered_for_an_amend() {
        let fixture = Fixture::woven();
        assert_eq!(
            head_message(&fixture.open()).expect("message"),
            "Merge feature/split-view"
        );
    }

    #[test]
    fn an_unborn_head_offers_no_message_to_amend() {
        let fixture = Fixture::empty();
        assert_eq!(head_message(&fixture.open()).expect("message"), "");
    }

    #[test]
    fn a_bare_repository_has_no_working_copy_to_change() {
        let dir = tempfile::tempdir().expect("temp dir");
        std::process::Command::new("git")
            .args(["init", "-q", "--bare"])
            .arg(dir.path())
            .output()
            .expect("git init --bare");

        let repo = crate::repo::open(dir.path()).expect("open");
        let error = stage(&repo, &["anything".to_string()]).unwrap_err();

        assert!(matches!(error, Error::NotStageable(_)));
    }
}
