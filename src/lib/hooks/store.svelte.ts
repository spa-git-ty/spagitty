// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * The open repository's git hooks, and the run a commit is watching
 * (FEAT-107).
 *
 * Two things, kept together because they answer one question — what is about
 * to run when I commit, and what did it say. `info` is the list, read fresh
 * before every commit (a hook added a minute ago in a terminal still asks);
 * `run` is the log window's state, fed a line at a time by `hook-output`
 * events carrying the token this store handed out.
 */

import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import * as api from '../api';
import { HOOK_OUTPUT_EVENT, type HookOutputEvent, type Hooks } from '../types';

/** The hooks a commit runs, in the order it runs them. */
export const ON_COMMIT = ['pre-commit', 'prepare-commit-msg', 'commit-msg', 'post-commit'];

/** Past this many lines the oldest go; a runaway hook must not eat the window. */
const MAX_LINES = 5000;

export interface HookRun {
	token: number;
	/** The hooks it was asked to run. */
	names: string[];
	lines: string[];
	status: 'running' | 'passed' | 'failed';
	error: string | null;
}

let info = $state<Hooks | null>(null);
let loading = $state(false);
let error = $state<string | null>(null);
let run = $state<HookRun | null>(null);
let next = 1;
let unlisten: UnlistenFn | null = null;

/** The hooks a commit with `from` would run, by name. */
export function onCommit(from: Hooks | null): string[] {
	if (!from || !from.enabled) return [];
	return ON_COMMIT.filter((name) => from.hooks.some((hook) => hook.name === name));
}

export const MANAGER_LABEL: Record<Hooks['manager'], string> = {
	git: 'git',
	husky: 'Husky',
	lefthook: 'lefthook',
	preCommit: 'pre-commit'
};

export const hooks = {
	get info(): Hooks | null {
		return info;
	},
	get loading(): boolean {
		return loading;
	},
	get error(): string | null {
		return error;
	},
	/** What a commit would run now, by name. */
	get onCommit(): string[] {
		return onCommit(info);
	},
	get run(): HookRun | null {
		return run;
	},

	/** Read the repository's hooks. Never throws; a failure is `error`. */
	async load(): Promise<Hooks | null> {
		loading = true;
		try {
			if (!api.inTauri()) return null;
			info = (await api.hooks()) ?? null;
			error = null;
		} catch (e) {
			error = String(e);
			info = null;
		} finally {
			loading = false;
		}
		return info;
	},

	/** Switch this repository's hooks on or off for commits made here. */
	async setEnabled(on: boolean): Promise<void> {
		try {
			await api.setHooksEnabled(on);
			error = null;
		} catch (e) {
			error = String(e);
		}
		await this.load();
	},

	/**
	 * Open the log window for a commit about to run `names`, listening before
	 * the commit starts so its first line is not missed. Returns the token the
	 * commit carries.
	 */
	async begin(names: string[]): Promise<number> {
		unlisten?.();
		unlisten = null;
		const token = next++;
		run = { token, names, lines: [], status: 'running', error: null };
		if (api.inTauri()) {
			unlisten = await listen<HookOutputEvent>(HOOK_OUTPUT_EVENT, (event) => {
				if (!run || event.payload.token !== run.token) return;
				this.print(event.payload.line);
			});
		}
		return token;
	},

	/** One line of output. */
	print(line: string): void {
		if (!run) return;
		const lines = [...run.lines, line];
		run.lines = lines.length > MAX_LINES ? lines.slice(lines.length - MAX_LINES) : lines;
	},

	/** The commit ended: null when it landed, else what stopped it. */
	finish(failure: string | null): void {
		unlisten?.();
		unlisten = null;
		if (!run) return;
		run.status = failure === null ? 'passed' : 'failed';
		run.error = failure;
	},

	/** Close the log window. Only once the run has ended. */
	close(): void {
		if (run?.status === 'running') return;
		run = null;
	},

	/** Forget everything. Called when the open repository changes. */
	clear(): void {
		unlisten?.();
		unlisten = null;
		info = null;
		error = null;
		run = null;
	}
};
