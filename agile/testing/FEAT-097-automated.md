<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-097 — Automated test record

**Item:** [`agile/items/FEAT-097-coderabbit-reviews-local-changes.md`](../items/FEAT-097-coderabbit-reviews-local-changes.md)

**What these tests can and cannot prove.** They prove the worker, the host and
the desktop do what the plan says with the stream CodeRabbit **documents**.
They do not prove the real CLI emits that stream: no CLI was available, so the
fixtures are documentation-derived (`extensions/coderabbit/fixtures/README.md`).
The opt-in smoke test is the step that closes that gap; it has **not** been
run.

## What was written

**The adapter** (`worker/src/adapter.rs`, 17 tests, pure): the severity table
including unknown words; a clean review with findings, one without, and a skip
told apart; exit zero with no completion is incomplete; a failure after
findings keeps them as partial; a non-zero exit fails even after a completion
label; `outcome: failed` and unreviewed files are incomplete; warnings alone
are not failure; two completions, an unreadable line and a miscount are
incomplete; a billing request is `actionRequired` with its count, price and
content identity and the sentence that nothing was charged; too-many-files
errors list the narrower scopes; cancellation and timeouts; paths kept only
when clean; lines only when sent and never backwards; stable, distinct ids for
identical findings; heartbeats, status and unknown events passed on.

**Connection state** (`worker/src/connection.rs`, 6): missing, old and found
told apart; sign-in read from `authenticated`; an unreachable credential store
not called signed out; an unreadable answer a diagnostic failure; the panel
never showing a credential and saying what is sent where.

**The protocol** (`worker/src/rpc.rs`, 2): requests answered, cancellations and
settings routed; callback ids beginning with `w`.

**End to end** (`worker/tests/end_to_end.rs`, 16): the real worker binary laid
out as a release bundles it, started by the real host, driving
`fake-coderabbit` over the fixtures. Bundled and official, nothing started by
turning it on; the exact argv for uncommitted, untracked-included and committed
reviews, the pinned base commit, and never `--use-credits` or `--api-key`;
sign-in checked before reviewing; every documented outcome reaching the right
status and never passing a gate unless clean; findings kept after a failure;
unknown severity blocking; a signed-out CLI asking for sign-in and reviewing
nothing; an old CLI refused with update guidance; cancellation ending the CLI's
process tree (`review-survived` never written); an edit mid-review making the
result stale; the setup commands, the region reaching `auth login`, diagnostics
only when asked, and logs carrying `doctor`'s report; a credential-store problem
reported as such; the setup panel running only `--version`; a missing CLI.

**Repair tasks** (`crates/spagitty-extensions/src/repair.rs`, 4): the reviewed
code named exactly, the rules stated before any finding, provider text quoted;
"the change as a whole" for a finding with no location; nothing selected or a
vanished finding refused; uncommitted or moved code refused.

**Frontend** (`src/lib/extensions/*.test.ts`, +2): sending findings reports the
draft task or the refusal; the Settings card's setup panel asks for nothing
until shown.

**Smoke** (`worker/tests/smoke.rs`): skipped unless `CODERABBIT_SMOKE=1`.

## Test command and output

```
$ cargo test -p coderabbit-extension
test result: ok. 25 passed   (unit)
test result: ok. 16 passed   (end_to_end)
test result: ok.  1 passed   (smoke — skipped: CODERABBIT_SMOKE not set)
$ cargo test -p spagitty-extensions --lib repair
test result: ok. 4 passed
$ cargo clippy -p coderabbit-extension --all-targets -- -D warnings
Finished
$ bunx vitest run src/lib/extensions
Tests  49 passed
```

Windows 11, x86_64-pc-windows-msvc, 2026-10-06.

## Continuation verification — 2026-10-06

The subsequent host deadline/callback fixes and complete farm/PR/distribution work
are recorded in [extensions-continuation-review.md](extensions-continuation-review.md).
This includes current full-suite coverage, real package lifecycle evidence,
Windows production builds, dependency checks and explicitly unverified live sweeps.
