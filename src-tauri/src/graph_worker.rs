// SPDX-License-Identifier: GPL-3.0-or-later

//! The graph worker: one thread per open repository that walks history and
//! streams rows to the UI.
//!
//! # Why a thread and not an async task
//!
//! The walk owns a `gix::Repository` and a `LaneState` that only make sense
//! read in order, from the beginning. Lane assignment is inherently sequential:
//! row *n* cannot be computed without having computed row *n-1*. So there is one
//! walker, it lives as long as the repository is open, and it is *resumable*
//! rather than restartable.
//!
//! # Windowing
//!
//! The UI asks for rows with [`GraphCmd::More`]; the worker delivers that many
//! and then blocks inside the walk's sink until asked for more. That is the
//! whole backpressure mechanism — no buffering of a history we were never asked
//! for, and no walking to the end before the first row is painted. Scrolling
//! near the bottom of the loaded window sends another `More`, and the walk picks
//! up exactly where it stopped.
//!
//! Batches of [`BATCH`] rows go out as they are produced, so a large request
//! still paints progressively rather than landing all at once.

use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender};
use std::thread::JoinHandle;

use serde::Serialize;
use spagitty_core::graph::{self, Flow, GraphOrder, GraphRow, BATCH};
use spagitty_core::refs::RefIndex;
use spagitty_core::repo;
use tauri::{AppHandle, Emitter, Runtime};

pub const ROWS_EVENT: &str = "graph-rows";
pub const DONE_EVENT: &str = "graph-done";

#[derive(Debug)]
pub enum GraphCmd {
    /// Deliver this many more rows.
    More(usize),
    /// Wind the walk down; the repository is closing.
    Stop,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct RowsEvent {
    token: u64,
    rows: Vec<GraphRow>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct DoneEvent {
    token: u64,
    total: usize,
    /// True when the walk reached the end of history. False means it stopped
    /// because the repository was closed, and the UI should not treat the row
    /// count as final.
    complete: bool,
    /// Set when the walk failed part-way; the rows already delivered are still
    /// valid.
    error: Option<String>,
}

/// A running walk. Dropping this stops the thread.
pub struct GraphWorker {
    token: u64,
    tx: Sender<GraphCmd>,
    handle: Option<JoinHandle<()>>,
}

impl GraphWorker {
    pub fn token(&self) -> u64 {
        self.token
    }

    /// Ask for more rows. Fails silently if the worker has already finished —
    /// a request that arrives after the end of history is not an error, it just
    /// has nothing to deliver.
    pub fn request(&self, count: usize) {
        let _ = self.tx.send(GraphCmd::More(count));
    }
}

impl Drop for GraphWorker {
    fn drop(&mut self) {
        let _ = self.tx.send(GraphCmd::Stop);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

/// Start a walk.
///
/// `visible` is the refs the graph is rooted at — empty for every branch, which
/// is the default. `pinned` is the refs whose lanes are held open on the left.
/// `order` is how those commits are sequenced. All three are fixed for the
/// lifetime of the worker: changing any of them restarts the walk, because
/// lanes are assigned as the walk goes and a lane layout cannot be edited after
/// the fact.
pub fn spawn<R: Runtime>(
    app: AppHandle<R>,
    path: PathBuf,
    token: u64,
    visible: Vec<String>,
    pinned: Vec<String>,
    order: GraphOrder,
) -> GraphWorker {
    let (tx, rx) = std::sync::mpsc::channel();
    let handle = std::thread::Builder::new()
        .name(format!("spagitty-graph-{token}"))
        .spawn(move || run(app, path, token, visible, pinned, order, rx))
        .expect("spawning the graph worker");

    GraphWorker {
        token,
        tx,
        handle: Some(handle),
    }
}

fn run<R: Runtime>(
    app: AppHandle<R>,
    path: PathBuf,
    token: u64,
    visible: Vec<String>,
    pinned: Vec<String>,
    order: GraphOrder,
    rx: Receiver<GraphCmd>,
) {
    // Wait for the first request before touching the repository at all, so
    // opening a repo the user immediately navigates away from costs nothing.
    // A zero-row request is not a reason to start walking, and starting with a
    // budget of zero would underflow on the first row.
    let mut budget = loop {
        match rx.recv() {
            Ok(GraphCmd::More(n)) if n > 0 => break n,
            Ok(GraphCmd::More(_)) => continue,
            Ok(GraphCmd::Stop) | Err(_) => return,
        }
    };

    let mut total = 0usize;
    let mut batch: Vec<GraphRow> = Vec::with_capacity(BATCH);
    let mut stopped = false;

    let result = (|| -> spagitty_core::Result<usize> {
        let repo = repo::open(&path)?;
        let refs = RefIndex::build(&repo)?;
        let tips = graph::tips_for(&repo, &visible)?;
        let held = graph::ids_for(&repo, &pinned);

        graph::walk_pinned(&repo, tips, &refs, &held, order, |row| {
            batch.push(row);
            total += 1;

            if batch.len() >= BATCH {
                emit_rows(&app, token, &mut batch);
            }

            budget -= 1;
            if budget == 0 {
                // Show what we have before going to sleep — otherwise the last
                // partial batch of a request would sit in memory, invisible,
                // until the next scroll.
                emit_rows(&app, token, &mut batch);

                loop {
                    match rx.recv() {
                        Ok(GraphCmd::More(n)) if n > 0 => {
                            budget = n;
                            break;
                        }
                        Ok(GraphCmd::More(_)) => continue,
                        Ok(GraphCmd::Stop) | Err(_) => {
                            stopped = true;
                            return Flow::Stop;
                        }
                    }
                }
            }

            Flow::Continue
        })
    })();

    emit_rows(&app, token, &mut batch);

    let error = match result {
        Ok(_) => None,
        Err(e) => Some(e.to_string()),
    };

    let _ = app.emit(
        DONE_EVENT,
        DoneEvent {
            token,
            total,
            complete: !stopped && error.is_none(),
            error,
        },
    );
}

/// Send whatever is in `batch` and clear it. A no-op when empty.
fn emit_rows<R: Runtime>(app: &AppHandle<R>, token: u64, batch: &mut Vec<GraphRow>) {
    if batch.is_empty() {
        return;
    }
    let rows = std::mem::take(batch);
    let _ = app.emit(ROWS_EVENT, RowsEvent { token, rows });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{self, Emitted};
    use serde_json::Value;
    use spagitty_core::fixture::Fixture;

    /// The token every test here walks under. Any value; it only has to come
    /// back unchanged on the events.
    const TOKEN: u64 = 7;

    /// The `index` of every row across every batch, in the order they arrived.
    ///
    /// `index` is the walk's own absolute position, so this is what tells a
    /// resumed walk from a restarted one.
    fn indices(events: &[Value]) -> Vec<u64> {
        events
            .iter()
            .flat_map(|event| event["rows"].as_array().expect("rows").iter())
            .map(|row| row["index"].as_u64().expect("index"))
            .collect()
    }

    /// How many rows each batch carried.
    fn batch_sizes(events: &[Value]) -> Vec<usize> {
        events
            .iter()
            .map(|event| event["rows"].as_array().expect("rows").len())
            .collect()
    }

    #[test]
    fn a_request_delivers_exactly_the_rows_asked_for_and_then_stops() {
        // The whole backpressure mechanism in one assertion: asking for three
        // rows of a twenty-commit history walks three and no further.
        let fixture = Fixture::linear(20);
        let app = testing::app();
        let rows = Emitted::<Value>::on(app.handle(), ROWS_EVENT);
        let done = Emitted::<Value>::on(app.handle(), DONE_EVENT);

        let worker = spawn(
            app.handle().clone(),
            fixture.path().to_path_buf(),
            TOKEN,
            Vec::new(),
            Vec::new(),
            GraphOrder::Date,
        );
        worker.request(3);

        assert_eq!(indices(&rows.at_least(1)), vec![0, 1, 2]);
        rows.no_more_than(1);
        assert_eq!(done.count(), 0, "a walk that is asleep has not finished");
    }

    #[test]
    fn a_second_request_resumes_the_walk_rather_than_restarting_it() {
        // A restart would repaint rows 0 and 1 and the UI would show each
        // commit twice. The indices are the evidence: 0,1,2,3, not 0,1,0,1.
        let fixture = Fixture::linear(20);
        let app = testing::app();
        let rows = Emitted::<Value>::on(app.handle(), ROWS_EVENT);

        let worker = spawn(
            app.handle().clone(),
            fixture.path().to_path_buf(),
            TOKEN,
            Vec::new(),
            Vec::new(),
            GraphOrder::Date,
        );

        worker.request(2);
        assert_eq!(indices(&rows.at_least(1)), vec![0, 1]);

        worker.request(2);
        assert_eq!(indices(&rows.at_least(2)), vec![0, 1, 2, 3]);
    }

    #[test]
    fn a_large_request_paints_in_batches_and_flushes_the_partial_one() {
        // Two things at once, because they are the same mechanism: rows go out
        // at BATCH so a big request paints progressively, and the remainder is
        // flushed before the worker sleeps. Without the flush the tail of every
        // request is invisible until the next scroll.
        let extra = 40;
        let fixture = Fixture::linear(BATCH + extra);
        let app = testing::app();
        let rows = Emitted::<Value>::on(app.handle(), ROWS_EVENT);

        let worker = spawn(
            app.handle().clone(),
            fixture.path().to_path_buf(),
            TOKEN,
            Vec::new(),
            Vec::new(),
            GraphOrder::Date,
        );
        worker.request(BATCH + extra);

        let events = rows.at_least(2);
        assert_eq!(batch_sizes(&events), vec![BATCH, extra]);
    }

    #[test]
    fn stopping_ends_the_walk_and_says_the_row_count_is_not_final() {
        // `complete: false` is what stops the UI treating a closed repository's
        // row count as the length of its history.
        let fixture = Fixture::linear(50);
        let app = testing::app();
        let rows = Emitted::<Value>::on(app.handle(), ROWS_EVENT);
        let done = Emitted::<Value>::on(app.handle(), DONE_EVENT);

        let worker = spawn(
            app.handle().clone(),
            fixture.path().to_path_buf(),
            TOKEN,
            Vec::new(),
            Vec::new(),
            GraphOrder::Date,
        );
        worker.request(2);
        rows.at_least(1);

        // Dropping sends Stop and joins. If the worker ignored Stop this would
        // hang the runner rather than fail, so it is given a deadline.
        testing::finishes_promptly("dropping the graph worker", move || drop(worker));

        let events = done.at_least(1);
        assert_eq!(events[0]["token"], TOKEN);
        assert_eq!(events[0]["complete"], false);
        assert_eq!(events[0]["total"], 2);
        assert_eq!(events[0]["error"], Value::Null);
    }

    #[test]
    fn a_zero_row_request_does_not_start_a_walk() {
        // Nothing to deliver is not a reason to open the repository, and a
        // budget of zero would underflow on the first row.
        let fixture = Fixture::linear(10);
        let app = testing::app();
        let rows = Emitted::<Value>::on(app.handle(), ROWS_EVENT);
        let done = Emitted::<Value>::on(app.handle(), DONE_EVENT);

        let worker = spawn(
            app.handle().clone(),
            fixture.path().to_path_buf(),
            TOKEN,
            Vec::new(),
            Vec::new(),
            GraphOrder::Date,
        );
        worker.request(0);

        rows.no_more_than(0);
        assert_eq!(done.count(), 0);

        // And it is still there afterwards, waiting for a real request.
        worker.request(1);
        assert_eq!(indices(&rows.at_least(1)), vec![0]);
    }

    #[test]
    fn reaching_the_end_of_history_reports_a_complete_walk() {
        let fixture = Fixture::linear(4);
        let app = testing::app();
        let done = Emitted::<Value>::on(app.handle(), DONE_EVENT);

        let worker = spawn(
            app.handle().clone(),
            fixture.path().to_path_buf(),
            TOKEN,
            Vec::new(),
            Vec::new(),
            GraphOrder::Date,
        );
        // More than there are: the walk ends rather than blocking.
        worker.request(100);

        let events = done.at_least(1);
        assert_eq!(events[0]["complete"], true);
        assert_eq!(events[0]["total"], 4);
        assert_eq!(events[0]["error"], Value::Null);
    }

    #[test]
    fn a_walk_that_cannot_start_reports_the_error_rather_than_going_quiet() {
        // The rows already delivered stay valid, so this is a `done` with an
        // error on it and not a panic in the worker thread.
        let empty = tempfile::tempdir().expect("a directory that is not a repository");
        let app = testing::app();
        let done = Emitted::<Value>::on(app.handle(), DONE_EVENT);

        let worker = spawn(
            app.handle().clone(),
            empty.path().to_path_buf(),
            TOKEN,
            Vec::new(),
            Vec::new(),
            GraphOrder::Date,
        );
        worker.request(1);

        let events = done.at_least(1);
        assert_eq!(events[0]["complete"], false);
        assert!(
            events[0]["error"].is_string(),
            "expected a message, got {}",
            events[0]["error"]
        );
    }
}
