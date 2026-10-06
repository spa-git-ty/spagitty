// SPDX-License-Identifier: GPL-3.0-or-later

//! How long each command waited for the open repository, and how long it held
//! it (TASK-052).
//!
//! Every command that reads or changes the open repository takes the session
//! lock, and until that is changed, holds it for the whole of its work: one
//! slow command makes every other wait. Before changing it, this measures it —
//! for each hold, the time spent waiting to get the lock and the time it was
//! held — so the change can be shown to help rather than believed to.
//!
//! The same shape as `spagitty_core::record`: a bounded ring buffer, read by
//! the frontend when God mode's Timings panel asks. Nothing leaves the
//! machine.
//!
//! No `tracing`: two numbers per hold, read by one panel, are a buffer and a
//! clock, and the crates a subscriber needs would be the first dependencies
//! added for something this small.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::Serialize;

/// How many holds are kept. Navigating a large repository makes dozens a
/// second; this is the last minute or so of it.
pub const CAPACITY: usize = 1000;

/// One hold of the session lock.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Timed {
    /// Monotonic within the process, so the panel asks only for what is new.
    pub seq: u64,
    /// Unix milliseconds, for display.
    pub at_ms: u64,
    /// The command that took the lock.
    pub command: &'static str,
    /// Microseconds spent waiting for the lock.
    pub wait_us: u64,
    /// Microseconds the lock was held: the command's work.
    pub held_us: u64,
}

fn buffer() -> &'static Mutex<VecDeque<Timed>> {
    static BUFFER: OnceLock<Mutex<VecDeque<Timed>>> = OnceLock::new();
    BUFFER.get_or_init(|| Mutex::new(VecDeque::with_capacity(CAPACITY)))
}

static SEQ: AtomicU64 = AtomicU64::new(0);

fn micros(duration: Duration) -> u64 {
    u64::try_from(duration.as_micros()).unwrap_or(u64::MAX)
}

/// Keep one hold.
pub fn push(command: &'static str, wait: Duration, held: Duration) {
    let entry = Timed {
        seq: SEQ.fetch_add(1, Ordering::Relaxed) + 1,
        at_ms: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|since| u64::try_from(since.as_millis()).unwrap_or(0))
            .unwrap_or(0),
        command,
        wait_us: micros(wait),
        held_us: micros(held),
    };
    let mut kept = buffer().lock().expect("timing buffer");
    if kept.len() == CAPACITY {
        kept.pop_front();
    }
    kept.push_back(entry);
}

/// Every hold kept after `after`, oldest first.
pub fn since(after: u64) -> Vec<Timed> {
    buffer()
        .lock()
        .expect("timing buffer")
        .iter()
        .filter(|entry| entry.seq > after)
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn holds_are_kept_in_order_and_read_from_where_the_reader_left_off() {
        push("first", Duration::from_micros(5), Duration::from_millis(2));
        let mark = since(0).last().expect("kept").seq;
        push("second", Duration::from_millis(1), Duration::from_micros(30));

        let after = since(mark);
        assert_eq!(after.len(), 1);
        assert_eq!(after[0].command, "second");
        assert_eq!((after[0].wait_us, after[0].held_us), (1000, 30));
        assert!(since(0).iter().any(|entry| entry.command == "first" && entry.held_us == 2000));
    }

    #[test]
    fn the_oldest_hold_leaves_when_the_buffer_is_full() {
        for _ in 0..CAPACITY + 5 {
            push("filler", Duration::ZERO, Duration::ZERO);
        }
        assert_eq!(since(0).len(), CAPACITY);
    }
}
