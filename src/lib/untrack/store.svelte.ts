// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * The Untrack screen's state (FEAT-155).
 *
 * A branch, the files and folders chosen on it, whether each gets a
 * `.gitignore` line, the commit's message, and whether the commit is pushed.
 * The tree is read from git's objects at the branch, as Files reads it, so a
 * branch that is not checked out shows its own files and not the ones on
 * disk.
 */

import * as api from '../api';
import { network } from '../network/store.svelte';
import type { FilesEntry, Untracked, UntrackTarget } from '../types';

/** A folder as the tree holds it: its entries, or why there are none yet. */
export type Folder = FilesEntry[] | 'loading' | { error: string };

let branch = $state('');
/** The commit the branch was at when its tree was read. */
let revision = $state<string | null>(null);
let folders = $state<Record<string, Folder>>({});
let open = $state<string[]>([]);
let finding = $state('');
let found = $state<string[] | null>(null);

let chosen = $state<UntrackTarget[]>([]);
let ignore = $state(true);
let push = $state(true);
/** What the person typed; null while the message is the one suggested. */
let typed = $state<string | null>(null);

let busy = $state(false);
let error = $state<string | null>(null);
let done = $state<Untracked | null>(null);
/** Whether the push after the commit was started. */
let pushed = $state(false);

let ticket = 0;
let findTicket = 0;

/** Is `path` inside the folder `dir`? */
function inside(path: string, dir: string): boolean {
	return path.startsWith(`${dir}/`);
}

async function readFolder(dir: string): Promise<void> {
	const at = revision;
	if (!at) return;
	const held = folders[dir];
	if (Array.isArray(held) || held === 'loading') return;
	folders[dir] = 'loading';
	try {
		const entries = await api.filesTree(at, dir);
		if (revision === at) folders[dir] = entries;
	} catch (error) {
		if (revision === at) folders[dir] = { error: String(error) };
	}
}

/** The message suggested for what is chosen. */
export function suggested(targets: readonly UntrackTarget[]): string {
	const named = targets.map((target) => (target.folder ? `${target.path}/` : target.path));
	if (named.length === 0) return '';
	if (named.length === 1) return `Stop tracking ${named[0]}`;
	return `Stop tracking ${named.length} paths\n\n${named.map((name) => `- ${name}`).join('\n')}`;
}

/** The `.gitignore` line that ignores exactly this path, as `untrack.rs` writes it. */
export function ignoreLine(target: UntrackTarget): string {
	const name = target.path.replace(/^\/+|\/+$/g, '');
	const escaped = name.replace(/[*?[\]\\]/g, (c) => `\\${c}`).replace(/ +$/, (spaces) => spaces.replace(/ /g, '\\ '));
	return `/${escaped}${target.folder ? '/' : ''}`;
}

export const untrack = {
	get branch(): string {
		return branch;
	},
	get revision(): string | null {
		return revision;
	},
	folder(dir: string): Folder | undefined {
		return folders[dir];
	},
	get open(): string[] {
		return open;
	},
	get finding(): string {
		return finding;
	},
	get found(): string[] | null {
		return found;
	},
	get chosen(): UntrackTarget[] {
		return chosen;
	},
	get ignore(): boolean {
		return ignore;
	},
	get push(): boolean {
		return push;
	},
	get message(): string {
		return typed ?? suggested(chosen);
	},
	get busy(): boolean {
		return busy;
	},
	get error(): string | null {
		return error;
	},
	get done(): Untracked | null {
		return done;
	},
	get pushed(): boolean {
		return pushed;
	},

	/** Whether `path` is chosen, itself or as part of a chosen folder. */
	isChosen(path: string): boolean {
		return chosen.some((target) => target.path === path || (target.folder && inside(path, target.path)));
	},

	/** Whether `path` is chosen only because a folder around it is. */
	isCovered(path: string): boolean {
		return chosen.some((target) => target.folder && inside(path, target.path));
	},

	/** Look at another branch. What was chosen on the last one is let go. */
	async choose(next: string): Promise<void> {
		branch = next;
		revision = null;
		folders = {};
		open = [];
		chosen = [];
		typed = null;
		finding = '';
		found = null;
		error = null;
		const mine = ++ticket;
		if (next === '') return;
		try {
			const resolved = await api.filesResolve(next);
			if (mine !== ticket) return;
			revision = resolved.id;
			await readFolder('');
		} catch (failure) {
			if (mine === ticket) error = String(failure);
		}
	},

	toggleFolder(dir: string): void {
		if (open.includes(dir)) {
			open = open.filter((path) => path !== dir);
		} else {
			open = [...open, dir];
			void readFolder(dir);
		}
	},

	/**
	 * Choose a path, or let it go. A folder chosen takes in what was chosen
	 * inside it: it is untracked with everything in it.
	 */
	toggle(target: UntrackTarget): void {
		if (chosen.some((held) => held.path === target.path)) {
			chosen = chosen.filter((held) => held.path !== target.path);
			return;
		}
		if (this.isCovered(target.path)) return;
		const kept = target.folder ? chosen.filter((held) => !inside(held.path, target.path)) : chosen;
		chosen = [...kept, target];
		done = null;
	},

	remove(path: string): void {
		chosen = chosen.filter((held) => held.path !== path);
	},

	setIgnore(next: boolean): void {
		ignore = next;
	},

	setPush(next: boolean): void {
		push = next;
	},

	setMessage(text: string): void {
		typed = text;
	},

	/** Look for files by name at the branch. An empty word goes back to the tree. */
	async find(text: string): Promise<void> {
		finding = text;
		const mine = ++findTicket;
		if (text.trim() === '' || !revision) {
			found = null;
			return;
		}
		try {
			const answer = await api.filesFind(revision, text);
			if (mine === findTicket) found = answer.paths;
		} catch {
			if (mine === findTicket) found = [];
		}
	},

	/**
	 * Untrack what is chosen and commit it; then push it, when asked and
	 * there is a remote to push to. Resolves once the commit is made and the
	 * push has started.
	 */
	async run(): Promise<boolean> {
		if (busy || branch === '' || chosen.length === 0) return false;
		busy = true;
		error = null;
		done = null;
		pushed = false;
		try {
			const made = await api.untrack(branch, chosen, ignore, this.message);
			done = made;
			if (push && made.remote) pushed = await network.push(made.remote, made.branch);
			chosen = [];
			typed = null;
			// The branch has moved: its tree is read again where it is now.
			const at = branch;
			await this.choose(at);
			done = made;
			return true;
		} catch (failure) {
			error = String(failure);
			return false;
		} finally {
			busy = false;
		}
	},

	clearState(): void {
		ticket++;
		findTicket++;
		branch = '';
		revision = null;
		folders = {};
		open = [];
		finding = '';
		found = null;
		chosen = [];
		ignore = true;
		push = true;
		typed = null;
		busy = false;
		error = null;
		done = null;
		pushed = false;
	}
};
