// SPDX-License-Identifier: GPL-3.0-or-later

//! TASK-052's baseline: how long the work behind each screen takes on a large
//! repository, measured under the application rather than through it.
//!
//! ```text
//! cargo run -p spagitty-core --release --example baseline -- \
//!     <repository> <a long file to blame> <a commit with a large diff> <its large file>
//! ```
//!
//! Each operation runs once cold and then `RUNS` times; the table gives the
//! first run and the median of the rest. Under the session lock as it stands,
//! each of these is also how long every other command waits behind it — which
//! is what God mode's Timings panel shows in the application.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use spagitty_core::graph::{self, Flow};
use spagitty_core::refs::RefIndex;
use spagitty_core::{blame, branches, diff, repo, status};

const RUNS: usize = 5;

fn ms(duration: Duration) -> String {
    format!("{:.1}", duration.as_secs_f64() * 1000.0)
}

/// Time `work`: once cold, then `RUNS` more; print a table row; return the
/// last answer and the median.
fn time<T>(label: &str, mut work: impl FnMut() -> T) -> (T, Duration) {
    let started = Instant::now();
    let mut answer = work();
    let first = started.elapsed();
    let mut runs = Vec::with_capacity(RUNS);
    for _ in 0..RUNS {
        let started = Instant::now();
        answer = work();
        runs.push(started.elapsed());
    }
    runs.sort();
    let median = runs[RUNS / 2];
    println!("| {label} | {} | {} |", ms(first), ms(median));
    (answer, median)
}

fn main() {
    let mut args = std::env::args().skip(1);
    let usage = "baseline <repository> <blame-path> <big-commit> <big-path>";
    let path = PathBuf::from(args.next().expect(usage));
    let blamed = args.next().expect(usage);
    let big_commit = args.next().expect(usage);
    let big_path = args.next().expect(usage);

    println!("| Operation | First run (ms) | Median of {RUNS} (ms) |");
    println!("| --- | --- | --- |");

    let (opened, _) = time("Open the repository", || repo::open(&path).expect("open"));
    let repository = opened;
    time("Repository info (header, status strip)", || repo::info(&repository).expect("info"));
    let (refs, _) = time("Index every ref", || RefIndex::build(&repository).expect("refs"));
    time("Counts (status strip)", || status::counts(&repository, &refs).expect("counts"));
    time("Working copy status", || status::working_copy(&repository).expect("status"));
    time("Branches list", || branches::list(&repository).expect("branches"));

    let tips = graph::all_tips(&repository).expect("tips");
    time("Graph: first screen (120 rows)", || {
        let mut rows = 0;
        graph::walk(&repository, tips.clone(), &refs, |_| {
            rows += 1;
            if rows >= 120 { Flow::Stop } else { Flow::Continue }
        })
        .expect("walk")
    });
    let (walked, _) = time("Graph: the whole history", || {
        graph::walk(&repository, tips.clone(), &refs, |_| Flow::Continue).expect("walk")
    });
    println!("|   (commits walked: {walked}) | | |");

    let head = repository.head_id().expect("head").to_string();
    time("Select a commit (detail)", || diff::commit_detail(&repository, &head).expect("detail"));
    let (listed, _) = time("Large commit: file list", || {
        diff::commit_diff(&repository, &big_commit).expect("commit diff")
    });
    println!("|   (files in it: {}) | | |", listed.files.len());
    let (file, _) = time("Large commit: the large file's hunks", || {
        diff::file_diff(&repository, &big_commit, &big_path).expect("file diff")
    });
    let lines: usize = file.hunks.iter().map(|hunk| hunk.lines.len()).sum();
    println!("|   (diff lines: {lines}) | | |");
    let (blamed_file, _) = time("Blame a long file", || blame::file(&repository, &blamed, "").expect("blame"));
    println!("|   (lines blamed: {}) | | |", blamed_file.lines.len());
    time("File history (200 entries)", || blame::history(&repository, &blamed, 200).expect("history"));

    let base = repository
        .rev_parse_single("HEAD~500")
        .expect("500 commits back")
        .to_string();
    let (between, _) = time("Review room: files over 500 commits", || {
        diff::changes_between(&repository, &base, &head).expect("between")
    });
    println!("|   (files changed: {}) | | |", between.len());
}
