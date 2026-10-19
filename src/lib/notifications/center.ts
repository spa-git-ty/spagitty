// SPDX-License-Identifier: GPL-3.0-or-later

import { styleOf, type Category } from './kinds';

/**
 * The notification center's rows, and how the drawer slices them (FEAT-142).
 *
 * Mirrors `src-tauri/src/inbox.rs`. Pure, so grouping by day and the filters
 * are tested without a drawer.
 */

export type Target =
	| { kind: 'route'; route: string }
	| { kind: 'url'; url: string }
	| { kind: 'none' };

export interface Entry {
	id: string;
	/** The repository's path, or empty when it is about none. */
	repository: string;
	repositoryName: string;
	category: Exclude<Category, 'issues'>;
	kind: string;
	title: string;
	body: string;
	target: Target;
	createdMs: number;
	readMs: number | null;
}

export type NewEntry = Omit<Entry, 'id' | 'createdMs' | 'readMs'>;

/** The chips, in the drawer's order. */
export type Filter = 'all' | 'unread' | Entry['category'];

export const FILTERS: { id: Filter; label: string }[] = [
	{ id: 'all', label: 'All' },
	{ id: 'unread', label: 'Unread' },
	{ id: 'pullRequests', label: 'Pull requests' },
	{ id: 'farm', label: 'Farm' },
	{ id: 'checks', label: 'Checks' },
	{ id: 'git', label: 'Git' }
];

export interface View {
	filter: Filter;
	/** Only this repository's, when set. */
	repository: string | null;
	search: string;
}

export function unread(entry: Entry): boolean {
	return entry.readMs === null;
}

export function visible(entries: Entry[], view: View): Entry[] {
	const words = view.search.trim().toLowerCase();
	return entries.filter((entry) => {
		if (view.repository !== null && entry.repository !== view.repository) return false;
		if (view.filter === 'unread' && !unread(entry)) return false;
		if (view.filter !== 'all' && view.filter !== 'unread' && entry.category !== view.filter) {
			return false;
		}
		if (!words) return true;
		const text = `${entry.title} ${entry.body} ${entry.repositoryName}`.toLowerCase();
		return words.split(/\s+/).every((word) => text.includes(word));
	});
}

const DAY = 24 * 60 * 60 * 1000;

function startOfDay(ms: number): number {
	const day = new Date(ms);
	day.setHours(0, 0, 0, 0);
	return day.getTime();
}

/** Today, Yesterday, then `Wednesday, Oct 14`. */
export function dayLabel(ms: number, now: number): string {
	const today = startOfDay(now);
	const day = startOfDay(ms);
	if (day >= today) return 'Today';
	if (day >= today - DAY) return 'Yesterday';
	return new Date(ms).toLocaleDateString('en-US', {
		weekday: 'long',
		month: 'short',
		day: 'numeric'
	});
}

/** The time on a row: minutes and hours today, the clock before that. */
export function timeLabel(ms: number, now: number): string {
	const ago = Math.max(0, now - ms);
	if (startOfDay(ms) >= startOfDay(now)) {
		if (ago < 60_000) return 'now';
		if (ago < 60 * 60_000) return `${Math.floor(ago / 60_000)} min`;
		return `${Math.floor(ago / (60 * 60_000))} h`;
	}
	return new Date(ms).toLocaleTimeString('en-GB', { hour: '2-digit', minute: '2-digit' });
}

export interface Group {
	label: string;
	entries: Entry[];
}

/** Newest first, one group per day, in the order the days come. */
export function groups(entries: Entry[], now: number): Group[] {
	const sorted = [...entries].sort((a, b) => b.createdMs - a.createdMs);
	const out: Group[] = [];
	for (const entry of sorted) {
		const label = dayLabel(entry.createdMs, now);
		const last = out[out.length - 1];
		if (last?.label === label) last.entries.push(entry);
		else out.push({ label, entries: [entry] });
	}
	return out;
}

/** How many a chip would show, for the Unread chip. */
export function unreadCount(entries: Entry[], repository: string | null = null): number {
	return entries.filter(
		(entry) => unread(entry) && (repository === null || entry.repository === repository)
	).length;
}

/** The button a row offers on hover: the same one Home uses for that kind. */
export function actionOf(entry: Entry): string {
	return styleOf(entry.kind).action;
}
