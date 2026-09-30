<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-039 — A graph test that reads the clock

**Status:** Fixed.
**Branch:** `bugfix/BUG-039-a-graph-test-that-reads-the-clock`
**Screens:** none. It is the test suite.
**Raised by:** BUG-038's test run on Windows, where
`graph::walk_tests::date_order_interleaves_parallel_histories` failed once and
passed on the run before it.

## Problem

`Fixture::woven` made its commits on the wall clock. The graph's date-order
test expects `Rewrite line 38`, on main, to sit between the feature branch's
two commits — which is only true when the commits' times tie and the walk's
tie-breaking puts it there. On a fast machine the whole fixture is made inside
one second and the test passes; on a slow one, where every `git` is a new
process, the commits spread over several seconds, `Rewrite line 38` is the
newest, and the test fails. Windows is the slow one, but a Linux runner that
crosses a second boundary fails it too.

Dating the fixture showed a second test resting on the same tie:
`rebase::tests::merges_are_left_out_because_a_rebase_does_not_replay_them`
compared the Rebase screen's todo with `git rev-list` in order. Where two lines
of history run side by side, the todo's order is not git's — `R3, Start, R38`
against git's `R38, R3, Start` — and the two agreed only while all three
commits shared a second.

## Scope

- `woven`'s own commits have fixed dates, from 2023-11-14, one minute apart,
  with `Rewrite line 38` dated between the feature's two commits: the
  interleaving the date-order test is about, stated rather than hoped for.
  `Fixture::commit_at` and `commit_all_at` do the dating.
- The todo test asserts what the todo promises: the commits git would replay,
  each after its parent. The doc comment on `rebase::todo`, which claimed git's
  order, says what is true.

## Non-scope

- The todo's order itself. It is a valid replay order, and it is the list git
  is handed, so what runs is what the screen showed.
- The farm crate's tests on Windows, which start `/bin/sh` and check process
  containment; TASK-033.

## Acceptance criteria

- The date-order test passes whatever the machine's speed.
- `spagitty-core` passes on native Windows, three runs in a row, and on Linux.
