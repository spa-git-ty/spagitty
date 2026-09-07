<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-079 — Automated tests

**Item:** [`agile/items/FEAT-079-a-node-that-says-who.md`](../items/FEAT-079-a-node-that-says-who.md)

## What was tested

Thirty-three tests over three files. The weight is deliberately on the two
things that are expensive to get wrong — **what leaves the machine**, and **how
often** — rather than on the drawing, which is the part a person can see.

### `crates/spagitty-core/src/avatars.rs` — 14 tests

| Test | Asserts |
| --- | --- |
| `sha256_matches_the_published_vectors` | The empty string and `abc`. A hand-written hash is checked against the vectors every implementation is checked against. |
| `sha256_handles_a_message_that_needs_a_second_block` | 56 bytes — the length at which padding no longer fits in the block the message ends in, which is where a hand-written SHA-256 breaks. |
| `base64_pads_every_remainder` | All three remainders, both pad lengths. |
| `a_github_noreply_address_resolves_by_id_and_carries_its_handle` | The id form gives the permanent URL and the handle, with no lookup. |
| `an_older_noreply_address_resolves_by_login` | The form with no id falls back to the login rather than producing a URL with an empty id in it. |
| `an_ordinary_address_goes_to_gravatar_as_a_hash_and_nothing_else` | **The disclosure test.** Lowercased before hashing; the address itself does not appear in the URL; `d=404` is set. If this feature ever starts sending more than it should, this is the test that says so. |
| `an_address_that_names_no_public_host_is_never_looked_up` | Five local forms — no `@`, `localhost`, a dotless host, no user, empty — resolve to nowhere. |
| `an_answer_is_read_back_from_the_cache_rather_than_asked_for_again` | A cached picture comes back as a `data:` URL with no network reachable. |
| `a_remembered_miss_is_answered_without_a_request` | The negative cache. |
| `finding_a_picture_clears_the_miss_that_was_remembered_for_it` | The two files are exclusive, so a newly-uploaded picture is not shadowed by the miss recorded before it. |
| `the_cache_names_no_address_it_holds` | A directory listing is not a list of everybody's email address. |
| `an_address_is_cached_under_one_name_however_it_is_written` | Case and whitespace fold, so two spellings are one person and one request. |
| `a_body_that_is_not_an_image_is_not_offered_as_one` | An HTML error page served with a 200 is a miss, not a `data:` URL that renders as nothing. |
| `forgetting_removes_every_cached_answer_and_survives_there_being_none` | What the preference does when it is turned off, including on a machine that never fetched anything. |

### `src/lib/graph/avatars.test.ts` — 14 tests

| Test | Asserts |
| --- | --- |
| asks for nothing before the preference has been read | The window between the first paint and the preference landing. A request made in it is one nobody agreed to. |
| still answers, with nothing | And does not throw while disabled. |
| asks once for an address, however many rows carry it | Fifty rows by one author: one call. |
| asks again for a different address | The deduplication is per address, not global. |
| treats two spellings of one address as one person | Matches the Rust side's folding. |
| does not ask about a commit with no author at all | git allows it. |
| never asks twice, even when the answer was nothing | The expensive case, in memory this time. |
| keeps going after one address fails | A rejection settles that entry and does not wedge the queue. |
| makes the picture drawable, and only once it has decoded | The `onload` ordering. |
| bumps the version, so a canvas that had painted repaints | The whole mechanism by which a late picture reaches the screen. |
| carries a handle even when there is no picture | A no-reply address is worth something on its own — it is what the hover says. |
| leaves the picture undrawable when it will not decode | `onerror` is a miss, not a broken node. |
| drops what was already fetched when the preference goes off | A picture still on screen after opting out is a preference that did not take effect. |
| stops asking when the preference goes off | |

### `src/lib/graph/lanes.test.ts` — 5 added

| Test | Asserts |
| --- | --- |
| draws the picture it is handed, at the node | The exact `drawImage` rectangle, composed from the same `laneX`/`laneNodeRadius`/`rowCenterY` the hover target uses. |
| asks for a picture once per node and no more | The canvas repaints every frame; a lookup called twice per node doubles whatever the caller does in it. |
| never asks for one for a merge | A merge is a plain dot, and giving it a face would claim its author drew the branch it swallowed. |
| draws the node without one when there is none | The fallback still draws a circle. A missing picture is the ordinary case. |
| draws the node without one when no lookup was given | The option is optional. |

## What the tests do not reach, and why

- **No test makes a real request.** `get_bytes` is exercised by the callers'
  cache paths and by the `https`-only guard; pointing a test at Gravatar would
  make the suite depend on a third party's uptime and would send an address from
  a CI runner every time it ran. `SWEEP-002` and `SWEEP-003` are where a real
  request is watched, with a packet capture.
- **The hover target's position is not asserted in the DOM.** It is computed
  from the same three functions the canvas draws with, and the canvas side of
  that arithmetic *is* asserted, exactly. What is not covered is the two drifting
  apart because somebody changed one call site — `SWEEP-005` is a person putting
  a pointer on a circle.
- **Coverage.** `src/lib/graph/avatars.svelte.ts` is covered by its own file;
  `CommitRows.svelte` is not directly covered and was not before.

## Run against the broken state first

Each Rust test was run against a stubbed `source` returning `Source::default()`
and against a `write_cache` that wrote only the picture file — the first fails
every resolution test, the second fails
`finding_a_picture_clears_the_miss_that_was_remembered_for_it` alone, which is
the test that exists for it. The frontend tests were written against the store
before the queue existed: `asks once for an address` failed with 50 calls.
