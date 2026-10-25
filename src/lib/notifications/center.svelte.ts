// SPDX-License-Identifier: GPL-3.0-or-later

import { invoke } from '@tauri-apps/api/core';
import { repo } from '$lib/repo.svelte';
import { unreadCount, type Entry, type NewEntry } from './center';

/**
 * The notification center's state (FEAT-142): every entry, whether the drawer
 * is open, and the edits. The entries are kept by `src-tauri/src/inbox.rs`;
 * this holds the copy the drawer draws and writes each change through.
 *
 * The only module that calls the inbox's commands, so a rename is one file.
 */

/** Whether there is a backend to keep anything in. Read here rather than from
 * `$lib/api`, which suites that mock it leave without one. */
const inTauri = () => typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

let entries = $state<Entry[]>([]);
let open = $state(false);
let loaded = false;

/** What `add` is given when nothing says which repository: the open one. */
function here(): Pick<NewEntry, 'repository' | 'repositoryName'> {
	return { repository: repo.info?.path ?? '', repositoryName: repo.info?.name ?? '' };
}

export const center = {
	get entries(): Entry[] {
		return entries;
	},
	get open(): boolean {
		return open;
	},
	/** The bell's number: unread, everywhere. */
	get unread(): number {
		return unreadCount(entries);
	},

	async load(): Promise<void> {
		if (!inTauri()) return;
		try {
			entries = await invoke<Entry[]>('inbox_list');
			loaded = true;
		} catch {
			// The record could not be read. The drawer opens empty rather than
			// not at all, and what is said from now on is still kept.
		}
	},

	/**
	 * Keep one thing that was said.
	 *
	 * Optimistic: the row is drawn at once and replaced by the stored one, so a
	 * notice that is said and then looked for in the drawer is already there.
	 */
	async add(entry: Omit<NewEntry, 'repository' | 'repositoryName'> & Partial<NewEntry>): Promise<void> {
		const full: NewEntry = { ...here(), ...entry };
		const now = Date.now();
		const provisional: Entry = { ...full, id: `local-${now}-${Math.random()}`, createdMs: now, readMs: null };
		entries = [provisional, ...entries];
		if (!inTauri()) return;
		try {
			if (!loaded) await center.load();
			const stored = await invoke<Entry>('inbox_add', { entry: full });
			entries = [stored, ...entries.filter((e) => e.id !== provisional.id && e.id !== stored.id)];
		} catch {
			// Kept for this session only.
		}
	},

	async read(ids: string[]): Promise<void> {
		const now = Date.now();
		entries = entries.map((entry) =>
			ids.includes(entry.id) && entry.readMs === null ? { ...entry, readMs: now } : entry
		);
		if (inTauri()) await invoke('inbox_read', { ids }).catch(() => {});
	},

	async readAll(repository: string | null = null): Promise<void> {
		const now = Date.now();
		entries = entries.map((entry) =>
			entry.readMs === null && (repository === null || entry.repository === repository)
				? { ...entry, readMs: now }
				: entry
		);
		if (inTauri()) await invoke('inbox_read_all', { repository }).catch(() => {});
	},

	/**
	 * Forget what was said before `beforeMs`, about one repository or every
	 * one. Resolves with how many went.
	 */
	async clear(beforeMs: number, repository: string | null = null): Promise<number> {
		const was = entries.length;
		entries = entries.filter(
			(entry) => entry.createdMs >= beforeMs || (repository !== null && entry.repository !== repository)
		);
		if (!inTauri()) return was - entries.length;
		return invoke<number>('inbox_clear', { beforeMs, repository }).catch(() => was - entries.length);
	},

	/** Forget everything said so far (FEAT-159), about one repository or every one. */
	clearAll(repository: string | null = null): Promise<number> {
		return center.clear(Date.now() + 1, repository);
	},

	show(): void {
		open = true;
		if (!loaded) void center.load();
	},
	hide(): void {
		open = false;
	},
	toggle(): void {
		if (open) center.hide();
		else center.show();
	},

	/** For tests. */
	reset(next: Entry[] = []): void {
		entries = next;
		open = false;
		loaded = false;
	}
};
