// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * What the screen waited for (TASK-052).
 *
 * Two kinds of number, kept here and read by God mode's Timings panel:
 *
 * - **Round trips**: every call to the backend, from asking to the answer,
 *   timed in `api.ts` — the one place that calls it. This is what a screen
 *   actually waits for: the work, any wait for the repository behind another
 *   command, and the trip across.
 * - **Measures**: navigating to a screen until its first frame is painted, and
 *   a diff from being handed its file until it is painted. Also left as
 *   `performance` marks and measures, so a profiler sees the same spans.
 *
 * The backend's own half — how long each command waited for the repository
 * and how long it held it — is read from the backend by the panel.
 *
 * Kept in a plain array, not reactive state: the graph alone makes many calls a
 * second, and a reactive array would copy itself on every one. A counter is
 * what the panel watches.
 */

export interface Trip {
	name: string;
	ms: number;
	ok: boolean;
	/** `performance.now()` at the answer. */
	at: number;
}

export type MeasureKind = 'navigate' | 'diff';

export interface Measure {
	kind: MeasureKind;
	/** The route navigated to, or the file drawn. */
	label: string;
	ms: number;
	at: number;
	/** Lines drawn, for a diff. */
	size?: number;
}

/** How many of each are kept: a minute or two of busy use. */
export const CAPACITY = 1000;

const trips: Trip[] = [];
const measures: Measure[] = [];
let version = $state(0);

function keep<T>(list: T[], item: T): void {
	list.push(item);
	if (list.length > CAPACITY) list.splice(0, list.length - CAPACITY);
	version += 1;
}

function now(): number {
	return typeof performance === 'undefined' ? Date.now() : performance.now();
}

/** Open spans, by kind: the start of what is being measured. */
const open = new Map<string, { label: string; started: number; size?: number }>();

export const timing = {
	/** Bumped on every number kept. */
	get version(): number {
		return version;
	},

	now,

	/** A backend call answered, or refused. */
	trip(name: string, started: number, ok: boolean): void {
		const at = now();
		keep(trips, { name, ms: at - started, ok, at });
	},

	/** Start measuring `kind` — a later start of the same kind replaces it. */
	start(kind: MeasureKind, label: string, size?: number): void {
		open.set(kind, { label, started: now(), size });
		try {
			performance.mark(`spagitty:${kind}:start`);
		} catch {
			// No Performance API: the number is kept all the same.
		}
	},

	/**
	 * End a measure once what it waits for is painted: two frames from now,
	 * the first being the one the change is laid out in.
	 */
	settle(kind: MeasureKind): void {
		const span = open.get(kind);
		if (!span) return;
		const done = () => {
			if (open.get(kind) !== span) return;
			open.delete(kind);
			const at = now();
			keep(measures, { kind, label: span.label, ms: at - span.started, at, size: span.size });
			try {
				performance.mark(`spagitty:${kind}:end`);
				performance.measure(`spagitty:${kind} ${span.label}`, `spagitty:${kind}:start`, `spagitty:${kind}:end`);
			} catch {
				// As above.
			}
		};
		if (typeof requestAnimationFrame === 'undefined') done();
		else requestAnimationFrame(() => requestAnimationFrame(done));
	},

	trips(): Trip[] {
		return trips.slice();
	},

	measures(): Measure[] {
		return measures.slice();
	},

	clear(): void {
		trips.length = 0;
		measures.length = 0;
		open.clear();
		version += 1;
	}
};

export interface Summary {
	name: string;
	count: number;
	p50: number;
	p95: number;
	max: number;
}

/** The middle, the slow end and the worst of some durations. */
export function summarise(name: string, values: number[]): Summary {
	const sorted = [...values].sort((a, b) => a - b);
	const at = (share: number) => sorted[Math.min(sorted.length - 1, Math.floor(share * sorted.length))] ?? 0;
	return { name, count: sorted.length, p50: at(0.5), p95: at(0.95), max: sorted[sorted.length - 1] ?? 0 };
}

/** Durations by name, summarised, slowest worst first. */
export function byName<T>(items: T[], name: (item: T) => string, value: (item: T) => number): Summary[] {
	const groups = new Map<string, number[]>();
	for (const item of items) {
		const key = name(item);
		const list = groups.get(key);
		if (list) list.push(value(item));
		else groups.set(key, [value(item)]);
	}
	return [...groups].map(([key, values]) => summarise(key, values)).sort((a, b) => b.max - a.max);
}
