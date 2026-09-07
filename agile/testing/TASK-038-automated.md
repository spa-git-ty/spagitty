<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-038 — Automated tests

**Item:** [`agile/items/TASK-038-settings-that-stop-lecturing.md`](../items/TASK-038-settings-that-stop-lecturing.md)

## What was tested

No new test file. This task **changed** tests rather than adding them, and the
change is the interesting part.

Five tests failed on the trim, which is exactly right: they asserted sentences,
and the sentences went. Each was rewritten to assert the claim it was there to
protect, so that the copy can be improved again without a test having to be
edited to permit it.

| Test | Was | Is | Why the new one is better |
| --- | --- | --- | --- |
| `says a toggle is not honoured yet` | `'Persisted, not yet honoured'` | `'Not honoured yet'` | The claim is that a switch admits it does not work. Neither phrasing is the claim; the shorter one is less of a hostage. |
| `says what leaves the machine, beside the switch that stops it` | `'No account, no identifier'` | `'no account, no identifier'` | Same claim, now mid-sentence. The second assertion in this test — `Turning it off stops every request` — was kept verbatim **on purpose**: that one is a promise, and a promise is allowed to be pinned to its words. |
| `explains what GPG does with no key` | `'No user.signingkey is set'` | `'No user.signingkey'` + `'committer address'` | The second half was always the real assertion; the first was scaffolding around it. |
| `names the global file and what is in it` | `'Your global configuration holds'` | `'Global holds'` | |
| `says a scope that holds nothing holds nothing` | `'nothing, so the value above comes from somewhere else'` | `'This repository holds nothing'` | The clause it asserted was removed as redundant with the "In effect" line above it. The test now asserts the fact rather than the explanation of the fact. |

One more moved for an unrelated reason: `shows the stored state of every toggle`
went from three chips to four, because FEAT-079 added one on the same branch.

## What this task could not be tested for

The complaint was that the screen reads as too much. There is no assertion for
that. A word count would pass while the screen stayed unreadable, and would fail
the first time somebody wrote a genuinely necessary sentence.

What the suite *can* do is make sure the trim did not take a claim with it, and
that is what the five rewrites above are: each of them names something the
screen must still say. `SWEEP-001` and `SWEEP-002` are where somebody looks at
the result and says whether it worked.

## Run against the broken state first

Inverted, and the result is worth recording honestly.

These tests were passing; the copy changed; they failed. That is the evidence
that they were pinned to wording rather than to meaning, and it is why they were
rewritten.

The rewrites were then checked the other way round: the nine section components
were reverted to their pre-trim state and the **new** assertions run against the
**old** copy. If an assertion really named a claim rather than a sentence, it
would pass against both.

**Two of the five do. Three do not.**

| Assertion | Against the old copy | |
| --- | --- | --- |
| `'This repository holds nothing'` | passes | The old text was this clause plus an explanation after it |
| `'No user.signingkey'` + `'committer address'` | passes | The old sentence contained both |
| `'Not honoured yet'` | **fails** | Old: `Persisted, not yet honoured` |
| `'Global holds'` | **fails** | Old: `Your global configuration holds` |
| `'no account, no identifier'` | **fails** | Old: the same words, capitalised mid-sentence |

So the rewrite is an improvement and not a cure. Three of these assertions are
still coupled to a phrasing, because in rendered text a fact and its wording are
the same string — there is nothing else to assert against short of giving the
elements test ids, which would be markup added for the benefit of the test
rather than the reader.

Recording it rather than claiming otherwise: **changing this copy again will
break tests again**, and the tests are the right size for what they protect. The
one assertion deliberately left pinned to its exact words is `Turning it off
stops every request` — that one is a promise about behaviour, and it should cost
something to reword.

(A sixth failure appears in the same run — `shows the stored state of every
toggle`, which counts chips. That is FEAT-079 adding a fourth toggle on the same
branch, not this task.)
