<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-107 — Plan

**Item:** [`agile/items/FEAT-107-hooks-you-can-see-and-skip.md`](../items/FEAT-107-hooks-you-can-see-and-skip.md)

- Core `hooks.rs`: `list` (hooks dir from `git rev-parse --git-path hooks`; Husky v9 stubs resolved to `.husky/<name>`, older Husky, lefthook and pre-commit detected; their config file read), `enabled` / `set_enabled` on `spagitty.hooks`.
- `shell::commit_with` (skip via `-c core.hooksPath=<nowhere>` and `--no-verify`; output streamed through `run_streaming`, which reads both pipes on threads), `hooks_dir`; `work::commit_with` reads the switch at commit time.
- Tauri: `commit` takes `skipHooks` and `token` and emits `hook-output`; it releases the session before git runs. New `hooks`, `set_hooks_enabled`.
- Frontend: `hooks/store.svelte.ts` (list, switch, the run and its lines), `dialog.choose` and a third button in `DialogHost`, `ui/Sheet.svelte` (one glass window frame for both new windows), `HookRunWindow` mounted by the layout, `HooksView` in `settings/HooksSection` and `hooks/HooksDialog`; `changes.commit` asks, skips or watches; `MessageBox` chips.
