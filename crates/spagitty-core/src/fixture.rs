// SPDX-License-Identifier: GPL-3.0-or-later

//! Real repositories for tests.
//!
//! The pure functions in this crate — hunk construction, the lane fold — are
//! tested directly. Everything else answers questions about a repository, and
//! the only honest way to test that is against one.
//!
//! Fixtures are built with the `git` binary rather than with `gix`, on purpose:
//! a fixture built by the same library under test would agree with it by
//! construction, and a bug in how we write refs would produce a repository that
//! only our own reader understands.
//!
//! Every fixture lives in a temporary directory that is removed when the
//! `Fixture` is dropped. Nothing here writes outside it.

use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::TempDir;

pub struct Fixture {
    dir: TempDir,
}

impl Fixture {
    /// An initialised repository with a fixed identity and no commits.
    pub fn empty() -> Self {
        let dir = tempfile::tempdir().expect("temp dir");
        let fixture = Fixture { dir };

        fixture.git(&["init", "-q", "-b", "main"]);
        fixture.git(&["config", "user.name", "Ada Lovelace"]);
        fixture.git(&["config", "user.email", "ada@example.com"]);
        // Signing would prompt, and a fixture must never wait for a human.
        fixture.git(&["config", "commit.gpgsign", "false"]);
        fixture.git(&["config", "gc.auto", "0"]);
        // The machine's line-ending policy stays out of it. Git for Windows
        // ships `core.autocrlf=true` in its system config, which checks every
        // file out with CRLF and makes a fixture mean something different on
        // Windows than everywhere else (BUG-037). A repository's own config
        // outranks the system's, for `git` and `gix` alike; a test about line
        // endings sets its own.
        fixture.git(&["config", "core.autocrlf", "false"]);
        fixture
    }

    /// The repository the screens are exercised against.
    ///
    /// ```text
    /// * Merge feature/split-view   (main, v0.2.0)
    /// |\
    /// | * Start the split view     (feature/split-view)
    /// | * Rewrite line 3
    /// * | Rewrite line 38
    /// |/
    /// * Add notes                  (v0.1.0, annotated)
    /// * Initial import
    /// ```
    ///
    /// Plus a binary file, a dotfile, a deeply nested path and one stash entry.
    ///
    /// The working copy is left **clean**, so tests that check out or switch
    /// branches are not fighting uncommitted changes. [`Fixture::dirty`] is the
    /// same repository with work in progress on top.
    pub fn woven() -> Self {
        let fixture = Self::empty();

        let lines: String = (1..=40).map(|n| format!("line {n}\n")).collect();
        fixture.write("core.txt", &lines);
        fixture.write("notes.md", "alpha\nbeta\n");
        fixture.write(".gitignore", ".cache/\n");
        fixture.write("src/deep/nested/main.rs", "fn main() {}\n");
        fixture.write_bytes("logo.bin", &[0x00, 0x01, 0x02, b'b', b'i', b'n', 0x00]);
        fixture.git(&["add", "-A"]);
        fixture.commit("Initial import");

        fixture.write("notes.md", "alpha\nbeta\nentry 1\n");
        fixture.commit_all("Add notes");
        fixture.git(&["tag", "-a", "v0.1.0", "-m", "First tag"]);

        fixture.git(&["switch", "-q", "-c", "feature/split-view"]);
        fixture.write("core.txt", &lines.replace("line 3\n", "LINE THREE\n"));
        fixture.commit_all("Rewrite line 3");
        fixture.write("split.txt", "split view work\n");
        fixture.git(&["add", "split.txt"]);
        fixture.commit("Start the split view");

        fixture.git(&["switch", "-q", "main"]);
        fixture.write(
            "core.txt",
            &lines.replace("line 38\n", "LINE THIRTY-EIGHT\n"),
        );
        fixture.commit_all("Rewrite line 38");
        fixture.git(&[
            "merge",
            "-q",
            "--no-ff",
            "feature/split-view",
            "-m",
            "Merge feature/split-view",
        ]);
        fixture.git(&["tag", "v0.2.0"]);

        // A branch that is fully contained in main, for "merged" filters.
        fixture.git(&["branch", "merged/already-in-main", "main~1"]);

        fixture.write("notes.md", "alpha\nbeta\nentry 1\nstashed\n");
        fixture.git(&["stash", "push", "-q", "-m", "wip on notes"]);

        fixture
    }

    /// [`Fixture::woven`] with work in progress: one staged change, one
    /// unstaged change and one untracked file.
    pub fn dirty() -> Self {
        let fixture = Self::woven();

        fixture.write("notes.md", "alpha\nbeta\nentry 1\nstaged\n");
        fixture.git(&["add", "notes.md"]);
        fixture.write("core.txt", "unstaged\n");
        fixture.write("untracked.txt", "brand new\n");

        fixture
    }

    /// A repository stopped mid-merge, with one conflicted file.
    ///
    /// Kept separate from [`Fixture::woven`]: a repository in the middle of a
    /// merge is a state the other screens should not be tested against.
    pub fn conflicted() -> Self {
        let fixture = Self::empty();

        fixture.write("shared.txt", "one\ntwo\nthree\n");
        fixture.write("untouched.txt", "calm\n");
        fixture.git(&["add", "-A"]);
        fixture.commit("Base");

        fixture.git(&["switch", "-q", "-c", "theirs"]);
        fixture.write("shared.txt", "one\nTHEIRS\nthree\n");
        fixture.commit_all("Their change");

        fixture.git(&["switch", "-q", "main"]);
        fixture.write("shared.txt", "one\nOURS\nthree\n");
        fixture.commit_all("Our change");

        // Expected to fail: stopping mid-merge is the point of this fixture.
        fixture.merge_expecting_conflict("theirs");
        fixture
    }

    /// A merge stopped by a file both sides *added*, so there is no stage 1.
    pub fn added_on_both_sides() -> Self {
        let fixture = Self::empty();

        fixture.write("untouched.txt", "calm\n");
        fixture.git(&["add", "-A"]);
        fixture.commit("Base");

        fixture.git(&["switch", "-q", "-c", "theirs"]);
        fixture.write("both.txt", "their new file\n");
        fixture.git(&["add", "both.txt"]);
        fixture.commit("They add it");

        fixture.git(&["switch", "-q", "main"]);
        fixture.write("both.txt", "our new file\n");
        fixture.git(&["add", "both.txt"]);
        fixture.commit("We add it");

        fixture.merge_expecting_conflict("theirs");
        fixture
    }

    /// A merge stopped by a file one side deleted and the other changed, so one
    /// of stages 2 and 3 is missing.
    pub fn deleted_on_one_side() -> Self {
        let fixture = Self::empty();

        fixture.write("gone.txt", "one\ntwo\nthree\n");
        fixture.git(&["add", "-A"]);
        fixture.commit("Base");

        fixture.git(&["switch", "-q", "-c", "theirs"]);
        fixture.remove("gone.txt");
        fixture.git(&["add", "-A"]);
        fixture.commit("They delete it");

        fixture.git(&["switch", "-q", "main"]);
        fixture.write("gone.txt", "one\nOURS\nthree\n");
        fixture.commit_all("We change it");

        fixture.merge_expecting_conflict("theirs");
        fixture
    }

    /// A merge stopped by a binary file both sides changed.
    pub fn binary_conflict() -> Self {
        let fixture = Self::empty();

        fixture.write_bytes("logo.bin", &[0x00, 0x01, 0x02, b'b', b'i', b'n', 0x00]);
        fixture.git(&["add", "-A"]);
        fixture.commit("Base");

        fixture.git(&["switch", "-q", "-c", "theirs"]);
        fixture.write_bytes("logo.bin", &[0x00, 0x09, 0x09, b't', b'h', b'x', 0x00]);
        fixture.commit_all("Their bytes");

        fixture.git(&["switch", "-q", "main"]);
        fixture.write_bytes("logo.bin", &[0x00, 0x07, 0x07, b'o', b'u', b'r', 0x00]);
        fixture.commit_all("Our bytes");

        fixture.merge_expecting_conflict("theirs");
        fixture
    }

    /// A cherry-pick stopped by a conflict.
    ///
    /// The same conflicted index as [`Fixture::conflicted`], reached by a
    /// different command — which is the point: the screen has to name what is
    /// actually in progress rather than assume a merge.
    pub fn cherry_pick_conflict() -> Self {
        let fixture = Self::empty();

        fixture.write("shared.txt", "one\ntwo\nthree\n");
        fixture.git(&["add", "-A"]);
        fixture.commit("Base");

        fixture.git(&["switch", "-q", "-c", "theirs"]);
        fixture.write("shared.txt", "one\nTHEIRS\nthree\n");
        let picked = fixture.commit_all("Their change");

        fixture.git(&["switch", "-q", "main"]);
        fixture.write("shared.txt", "one\nOURS\nthree\n");
        fixture.commit_all("Our change");

        // Expected to fail: stopping mid-pick is the point of this fixture.
        fixture.try_git(&["cherry-pick", &picked]);
        fixture
    }

    /// A linear history of `count` commits, oldest last.
    ///
    /// For the windowing tests: everything the graph worker does in batches
    /// needs a history longer than one batch, and the content of the commits is
    /// beside the point. Empty commits keep it fast — the walk reads the commit
    /// graph, not the trees.
    pub fn linear(count: usize) -> Self {
        let fixture = Self::empty();

        for n in 0..count {
            fixture.git(&[
                "commit",
                "-q",
                "--allow-empty",
                "-m",
                &format!("commit {n}"),
            ]);
        }

        fixture
    }

    pub fn path(&self) -> &Path {
        self.dir.path()
    }

    /// Merge `branch`, expecting git to stop on a conflict.
    ///
    /// Asserts that it *did* conflict: a fixture that merged cleanly would
    /// produce tests that pass by testing nothing.
    fn merge_expecting_conflict(&self, branch: &str) {
        let status = self.try_git(&["merge", branch]);
        assert!(
            !status,
            "merging {branch} was meant to conflict but succeeded"
        );
    }

    /// Run `git` and report whether it succeeded, rather than panicking. For
    /// the commands a fixture *wants* to fail.
    fn try_git(&self, args: &[&str]) -> bool {
        Command::new("git")
            .current_dir(self.dir.path())
            .args(args)
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("HOME", self.dir.path())
            .output()
            .unwrap_or_else(|e| panic!("running git {args:?}: {e}"))
            .status
            .success()
    }

    /// Run `git` in the fixture, returning stdout. Panics on failure, since a
    /// fixture that did not build is not a test result worth reporting.
    pub fn git(&self, args: &[&str]) -> String {
        let output = Command::new("git")
            .current_dir(self.dir.path())
            .args(args)
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("HOME", self.dir.path())
            .output()
            .unwrap_or_else(|e| panic!("running git {args:?}: {e}"));

        assert!(
            output.status.success(),
            "git {args:?} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8_lossy(&output.stdout).into_owned()
    }

    pub fn write(&self, path: &str, contents: &str) {
        self.write_bytes(path, contents.as_bytes());
    }

    pub fn write_bytes(&self, path: &str, contents: &[u8]) {
        let full = self.dir.path().join(path);
        if let Some(parent) = full.parent() {
            std::fs::create_dir_all(parent).expect("fixture directory");
        }
        std::fs::write(full, contents).expect("fixture file");
    }

    /// One path inside the fixture, for asserting a file exists or does not.
    pub fn at(&self, path: &str) -> PathBuf {
        self.dir.path().join(path)
    }

    /// Read a fixture file back as text. Panics if it is not there, which is
    /// the assertion a test wanted to make anyway.
    pub fn read(&self, path: &str) -> String {
        std::fs::read_to_string(self.at(path))
            .unwrap_or_else(|e| panic!("reading fixture file {path}: {e}"))
    }

    pub fn remove(&self, path: &str) {
        std::fs::remove_file(self.dir.path().join(path)).expect("removing a fixture file");
    }

    /// Commit what is staged. Returns the new commit's full id.
    pub fn commit(&self, message: &str) -> String {
        self.git(&["commit", "-q", "-m", message]);
        self.head()
    }

    /// Stage every tracked change and commit. Returns the new commit's full id.
    pub fn commit_all(&self, message: &str) -> String {
        self.git(&["commit", "-q", "-am", message]);
        self.head()
    }

    pub fn head(&self) -> String {
        self.git(&["rev-parse", "HEAD"]).trim().to_string()
    }

    /// Resolve any revision the way `git rev-parse` would.
    pub fn rev(&self, revision: &str) -> String {
        self.git(&["rev-parse", revision]).trim().to_string()
    }

    pub fn open(&self) -> gix::Repository {
        crate::repo::open(self.dir.path()).expect("opening the fixture")
    }
}
