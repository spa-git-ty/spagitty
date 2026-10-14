<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-139 — Plan

**Item:** [FEAT-139](../items/FEAT-139-releases.md)

Written on 2026-10-14, before release code.

## The pieces

1. **Host-neutral releases** in `forge/releases.rs`: paged releases and assets, explicit create/update/publish/delete and asset actions, host-generated notes and local Conventional Commit notes. Injected requests exercise both hosts without credentials.
2. **Transfers** stay in `forge/http.rs`: bounded downloads, binary uploads, credential-bound destinations, progress and one-file retries. GitHub uploads use the configured host's upload endpoint; GitLab files use the project generic package registry before a release link is created.
3. **Commands** resolve configured remotes through `forge_views.rs`. Application data holds hidden ids and GitLab drafts. Each mutation is separately confirmed. Release deletion and tag deletion are separate actions; tag deletion starts unchecked.
4. **Changelog commit** uses the existing ordinary commit path and hooks. Require the checked-out target branch, a clean index and no unrelated work. Compare the host branch tip before committing and again before pushing the resulting commit. Never move an existing tag. Keep partial success visible.
5. **Screen** follows the artboard's release list, central editor and files column. Filters, local hiding, safe Markdown preview, tag/target/since fields, note generation, file picker/drop, independent transfer state and completed run artifacts are functional controls.
6. **Records and checks** cover fixtures, commands, guarded stores and mounted controls. Update the item, index, sweep, automated record, screens and changelog before a checked commit and stacked pull request.

## Decisions

- Credentials stay in Rust. No frontend-supplied upload URL receives a token.
- Host writes get one attempt; uncertain delivery asks for a refresh before retry.
- GitLab has neither host drafts nor release link download counts. Drafts stay local and unavailable counts are absent. Its latest release follows host ordering; GitHub alone exposes the explicit Latest option.
- Publishing a new GitHub tag uses its resolved commit id. GitLab uses its release create call with a resolved ref. Existing tags are never retargeted.
- The rail group stays immediately above Settings.
- This branch stacks on Issues; no merge is authorized.

## Dependencies

No new package is planned. Existing HTTP, native file dialogs, message parser and Markdown renderer are reused.

## References

[GitHub releases](https://docs.github.com/en/rest/releases/releases), [GitHub assets](https://docs.github.com/en/rest/releases/assets), [GitLab releases](https://docs.gitlab.com/api/releases/), [GitLab generic packages](https://docs.gitlab.com/user/packages/generic_packages/).

## What decides it is right

- Both host fixtures map states, assets and missing counts correctly.
- Every write refuses missing confirmation; releases and tags have separate deletion requests.
- Notes preserve request numbers and use the same Conventional Commit parser as Hooks.
- Changelog fixture tests show hooks run, the tag target includes the new commit, and moved targets stop publication.
- State and mounted tests cover drafts, filtering, local hiding, stale replies, write errors and independent file retry.
- Format, clippy, Rust suites, frontend types, coverage and build pass before the feature is committed.
