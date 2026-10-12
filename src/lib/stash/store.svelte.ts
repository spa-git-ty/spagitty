// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * Stash entries, and what is in the selected one.
 *
 * The list is one call. What an entry contains is `commitDiff` on the entry's
 * own id — a stash is a commit, so the Diff screen's machinery answers the
 * question without a second implementation.
 *
 * One file of an entry is `fileDiff` on the same id (FEAT-034), loaded as that
 * file is selected and kept, exactly as the Diff screen does it. Walking back
 * and forth through an entry's files then costs one fetch each rather than one
 * per view, and the cache is thrown away when the entry changes — a
 * within-entry convenience, not a history of everything ever opened.
 */

import * as api from '../api';
import { activity } from '../activity/store.svelte';
import * as act from '../graph/actions';
import { repo } from '../repo.svelte';
import type { CommitDiff, FileDiff, StashAction, StashEntry } from '../types';

let entries = $state<StashEntry[]>([]);
let loaded = $state(false);
let loading = $state(false);
let error = $state<string | null>(null);

let selected = $state<string | null>(null);
let contents = $state<CommitDiff | null>(null);
let contentsError = $state<string | null>(null);
let contentsLoading = $state(false);

/** The file open in the pane, and its hunks. */
let path = $state<string | null>(null);
let file = $state<FileDiff | null>(null);
let fileError = $state<string | null>(null);
let fileLoading = $state(false);

/** Hunks already fetched for the open entry. */
let cache = new Map<string, FileDiff>();

/** The message a new stash would carry, and whether to take untracked files. */
let message = $state('');
let includeUntracked = $state(false);

let busy = $state(false);
let writeError = $state<string | null>(null);

let listSeq = 0;
let contentsSeq = 0;
let fileSeq = 0;

/** Forget the open file. Called whenever the entry under it changes. */
function forgetFile() {
	fileSeq += 1;
	path = null;
	file = null;
	fileError = null;
	fileLoading = false;
	cache = new Map();
}

export const stash = {
	get entries(): StashEntry[] {
		return entries;
	},
	get loaded(): boolean {
		return loaded;
	},
	get loading(): boolean {
		return loading;
	},
	get error(): string | null {
		return error;
	},
	get selected(): StashEntry | null {
		return entries.find((entry) => entry.id === selected) ?? null;
	},
	get contents(): CommitDiff | null {
		return contents;
	},
	get contentsError(): string | null {
		return contentsError;
	},
	get contentsLoading(): boolean {
		return contentsLoading;
	},
	get path(): string | null {
		return path;
	},
	get file(): FileDiff | null {
		return file;
	},
	get fileError(): string | null {
		return fileError;
	},
	get fileLoading(): boolean {
		return fileLoading;
	},

	/** Position of the selected file, 0-based. -1 when nothing is selected. */
	get fileIndex(): number {
		if (contents === null || path === null) return -1;
		return contents.files.findIndex((f) => f.path === path);
	},

	get fileCount(): number {
		return contents?.files.length ?? 0;
	},
	get message(): string {
		return message;
	},
	get includeUntracked(): boolean {
		return includeUntracked;
	},
	get busy(): boolean {
		return busy;
	},
	get writeError(): string | null {
		return writeError;
	},

	setMessage(next: string) {
		message = next;
	},
	setIncludeUntracked(next: boolean) {
		includeUntracked = next;
	},

	async load(): Promise<void> {
		loading = true;
		const seq = ++listSeq;
		try {
			const next = await api.stashes();
			if (seq !== listSeq) return;

			entries = next;
			loaded = true;
			error = null;

			// Keep the open entry if it survived; otherwise open the newest.
			const keep = next.find((entry) => entry.id === selected);
			if (keep) this.select(keep.id, true);
			else if (next.length > 0) this.select(next[0].id);
			else this.deselect();
		} catch (e) {
			if (seq === listSeq) {
				error = String(e);
				entries = [];
				this.deselect();
			}
		} finally {
			if (seq === listSeq) loading = false;
		}
	},

	/** Open an entry and read what is in it. */
	select(id: string, force = false): void {
		if (!force && selected === id) return;

		// A forced re-select is the same entry read again — after an apply, or
		// after a `repo-changed`. Keep the file that is open across it, or the
		// list would jump back to the first file every time the screen refreshed.
		const keepPath = force ? path : null;

		selected = id;
		contents = null;
		contentsError = null;
		contentsLoading = true;
		forgetFile();

		const seq = ++contentsSeq;
		api
			.commitDiff(id)
			.then((result) => {
				if (seq !== contentsSeq) return;
				contents = result;
				contentsLoading = false;
				// An entry always opens on its first file: an empty pane beside
				// a list of files is a click the screen can make for you.
				const survived = result.files.find((f) => f.path === keepPath);
				if (survived) this.selectFile(survived.path);
				else if (result.files.length > 0) this.selectFile(result.files[0].path);
			})
			.catch((e) => {
				if (seq !== contentsSeq) return;
				contentsError = String(e);
				contentsLoading = false;
			});
	},

	deselect(): void {
		contentsSeq += 1;
		selected = null;
		contents = null;
		contentsError = null;
		contentsLoading = false;
		forgetFile();
	},

	/** Open one file of the selected entry, from the cache where possible. */
	selectFile(next: string): void {
		const id = selected;
		if (id === null) return;

		path = next;
		fileError = null;

		const cached = cache.get(next);
		if (cached) {
			file = cached;
			fileLoading = false;
			return;
		}

		file = null;
		fileLoading = true;

		const seq = ++fileSeq;
		api
			.fileDiff(id, next)
			.then((result) => {
				cache.set(next, result);
				if (seq !== fileSeq) return;
				file = result;
				fileLoading = false;
			})
			.catch((e) => {
				if (seq !== fileSeq) return;
				fileError = String(e);
				fileLoading = false;
			});
	},

	/** Move `delta` files through the entry. Stops at either end. */
	stepFile(delta: number): void {
		const files = contents?.files ?? [];
		if (files.length === 0) return;

		const at = this.fileIndex;
		const next = Math.min(Math.max(at + delta, 0), files.length - 1);
		if (next !== at) this.selectFile(files[next].path);
	},

	/** Stash the working copy, then re-read the list and the rail. */
	async push(): Promise<boolean> {
		if (busy) return false;
		// Named while it runs, and not started over a pull or a checkout
		// (FEAT-121).
		const step = activity.begin('stash', 'Stashing your changes');
		if (!step) {
			writeError = activity.blocked('repository');
			return false;
		}
		busy = true;
		writeError = null;
		try {
			await api.stashPush(message, includeUntracked);
			message = '';
			return true;
		} catch (e) {
			writeError = String(e);
			return false;
		} finally {
			busy = false;
			step.end();
			await this.load();
			await repo.refresh();
		}
	},

	/**
	 * Pop, apply or drop the selected entry, then re-read the list (FEAT-014).
	 *
	 * The confirmation and the write itself live in `graph/actions.ts`, not
	 * here: that module's argument is that the sentence shown before a
	 * destructive operation is as much a part of it as the command, and a second
	 * screen offering the same operation must not write a second sentence. What
	 * this adds is the part `actions` cannot know about — the list on *this*
	 * screen is now stale, and `perform`'s own refresh reaches the graph and the
	 * rail but not here.
	 *
	 * Pop and drop remove the entry, so the selection is released before the
	 * re-read rather than pointing at something that no longer exists.
	 */
	async restore(action: StashAction): Promise<void> {
		const entry = this.selected;
		if (!entry || busy) return;

		busy = true;
		try {
			const changed = await act.stash(entry.index, entry.name, action);
			if (!changed) return;

			if (action !== 'apply') this.deselect();
			await this.load();
			await repo.refresh();
		} finally {
			busy = false;
		}
	},

	clear(): void {
		listSeq += 1;
		contentsSeq += 1;
		forgetFile();
		entries = [];
		loaded = false;
		loading = false;
		error = null;
		selected = null;
		contents = null;
		contentsError = null;
		contentsLoading = false;
		message = '';
		includeUntracked = false;
		busy = false;
		writeError = null;
	}
};
