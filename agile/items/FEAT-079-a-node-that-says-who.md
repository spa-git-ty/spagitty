<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-079 — A node that says who

**Status:** Open — built and tested; awaiting the sweep and the merge.
**Branch:** `feature/FEAT-079-avatars-and-quieter-settings`
**Screens:** Graph.

## What and why

A commit node on the Graph screen carries a generated "marble" — a coloured
blob derived from the author's email (FEAT-023, `portrait.ts`). It is good at
one job and cannot do the other:

- **Disambiguating** — "these four rows are the same person, those two are
  not" — which it does perfectly, offline, identically on every machine.
- **Identifying** — "*which* person" — which it cannot do at all. Nobody knows
  their own blob.

Identifying is the question a node is actually looked at to answer, and the
screen had no answer to it anywhere: the node had no hover, so a graph could be
scrolled for an hour without ever saying whose commits they were unless the
Author column happened to be shown.

So two things: the node shows the author's real picture when there is one, and
hovering it says who they are.

## Scope and acceptance criteria

- A commit node draws the author's real picture when one can be found.
- It falls back to the generated marble when one cannot — offline, no picture
  for that address, the preference off. The fallback is not an error state and
  looks exactly as it did before this feature.
- Hovering a node names the author: their name, their address, and their handle
  where the address spells one out.
- A merge node keeps its plain dot and is not hoverable for an author. It is
  drawn as a dot precisely because it is not one person's work.
- The Author column uses the same picture, from the same resolution, so the two
  cannot disagree about who somebody is.
- One request per author, ever: cached to disk across sessions, deduplicated in
  memory within one.
- A preference, on by default, turns it off. Off stops every request **and**
  empties the cache.
- Nothing blocks a paint, and nothing is requested from inside a scroll frame.

## What leaves the machine, exactly

This feature is the second thing in Spagitty that makes a network request, and
the Settings screen says so. Named here because "the app fetches avatars" is not
a specific enough claim for somebody deciding whether to turn it off.

- **A GitHub no-reply address** — `1234567+octocat@users.noreply.github.com` —
  resolves with **no lookup at all**: the account is written in the address. The
  request is for a picture at `avatars.githubusercontent.com`, carrying an
  account id that is already in the repository's commits in plain text.
- **Any other address** goes to Gravatar as the SHA-256 of its lowercased form,
  with `d=404` so a missing picture is a status rather than a generated image.
  Gravatar cannot reverse the hash; it can confirm an address it already holds.
  That is a real disclosure, and it is the one the preference exists for.
- **An address with no public host** — no `@` at all, or a host with no dot,
  like `ada@localhost` or `ada@build-box` — never leaves. Asking the internet
  about a name on somebody's own network would be telling the internet about it.

No API is called, no token is sent, no repository, path, commit message or name
is sent, and no host is asked who an address belongs to.

## Non-scope

- Asking a forge who an address belongs to. GitHub's user-search endpoint would
  turn any address into a name and a picture, needs a token, is rate-limited per
  account, and is a question about a person rather than a request for a public
  image. The no-reply forms give a handle for free; every other address gets a
  picture or nothing.
- GitLab and Bitbucket no-reply forms. Neither has a stable published shape the
  way GitHub's does; both land on Gravatar, which is what GitLab serves anyway.
- Uploading, changing or caching a picture anywhere a person would find it.
- A tooltip component. The hover is a `title`, which is what the rest of this
  screen already uses for the same kind of answer.
