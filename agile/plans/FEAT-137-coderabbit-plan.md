# FEAT-137 — CodeRabbit review fixes

**Scope:** Address the six actionable CodeRabbit findings on PR #73.

1. `crates/spagitty-core/src/hooks/import.rs` — classify an import as `Format` only when `{staged_files}` is present; keep preset as label.
2. `crates/spagitty-core/src/hooks/runner.sh` — retention sees `done` anywhere in a record; pass pushed refs to each block on stdin.
3. `src-tauri/src/hooks.rs` — restrict `Try message` to commit-msg `Message` blocks.
4. `design_handoff_forge_views/source/HooksEdit.dc.html` — use a box-shadow selection ring so keyboard focus remains visible.
5. Item records — clarify the push/merge restriction text.

No new dependencies. No file reformatting.
