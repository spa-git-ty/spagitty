// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * The streamed graph.
 *
 * Rows live in a plain array, not in `$state`. A repository with 200k commits
 * would mean 200k deeply-proxied objects, and every batch would pay to re-wrap
 * them. Instead the array is untracked and a `version` counter is the reactive
 * signal: appending a batch is O(batch), and consumers re-read what they need.
 * Nothing here holds a DOM node — the row list is virtualized and the lanes are
 * drawn on canvas.
 */

import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import * as api from '../api';
import { repo } from '../repo.svelte';
import {
	GRAPH_DONE_EVENT,
	GRAPH_ROWS_EVENT,
	type CommitDetail,
	type GraphDoneEvent,
	type GraphRow,
	type GraphRowsEvent
} from '../types';

/** Rows asked for on first paint. Enough to fill any window several times over. */
const FIRST_WINDOW = 600;

/** Rows asked for on each subsequent request. */
const NEXT_WINDOW = 2000;

/** Start fetching when the viewport gets this close to the last loaded row. */
const PREFETCH = 300;

/** Untracked row storage, indexed by absolute row index. */
let buffer: GraphRow[] = [];

let version = $state(0);
let count = $state(0);
/**
 * Lanes the loaded history needs, over every row rather than the rows on screen.
 *
 * The lane pitch and the undragged column width follow this figure. Measured
 * over the viewport, scrolling from a shallow stretch of history into a deep one
 * re-spaced every lane and resized the column under the reader's hand; measured
 * over the history it only grows while the walk streams, and holds still after.
 */
let lanes = $state(1);
let requested = $state(0);
let complete = $state(false);
/**
 * When the walk last finished, in unix seconds — not when the process started,
 * and not when the last row arrived (FEAT-040). The graph's footer says how old
 * what is on screen is, and a walk still running has refreshed nothing yet, so
 * this moves only on completion.
 */
let refreshedAt = $state<number | null>(null);
let error = $state<string | null>(null);
let selectedIndex = $state<number | null>(null);
let detail = $state<CommitDetail | null>(null);
let detailError = $state<string | null>(null);

/** Guards against a slow detail fetch landing after a newer selection. */
let detailSeq = 0;
let unlisteners: UnlistenFn[] = [];

/**
 * Set when a re-walk is in flight. The existing rows stay on screen until the
 * first batch of the new walk arrives, so a ref moving does not blank the graph
 * and flash it back in.
 */
let pendingReset = false;

/** The selected commit, tracked by id so it survives a re-walk. */
let selectedId: string | null = null;
/** True while we are re-walking and have not seen the selected commit again. */
let selectionUnverified = false;

function reset() {
	buffer = [];
	version += 1;
	count = 0;
	lanes = 1;
	requested = 0;
	complete = false;
	refreshedAt = null;
	error = null;
	selectedIndex = null;
	selectedId = null;
	selectionUnverified = false;
	pendingReset = false;
	detail = null;
	detailError = null;
}

export const graph = {
	/** Reactive signal; changes whenever rows were added or cleared. */
	get version(): number {
		return version;
	},
	/** Number of rows loaded so far. Grows as the walk streams. */
	get count(): number {
		return count;
	},
	/** Lanes the whole loaded history needs. See `lanes` above. */
	get lanes(): number {
		return lanes;
	},
	/** True once the walk reached the end of history. */
	get complete(): boolean {
		return complete;
	},
	/** Unix seconds when the walk last completed, or null before it has. */
	get refreshedAt(): number | null {
		return refreshedAt;
	},
	get error(): string | null {
		return error;
	},
	get selectedIndex(): number | null {
		return selectedIndex;
	},
	get selected(): GraphRow | null {
		return selectedIndex === null ? null : (buffer[selectedIndex] ?? null);
	},
	get detail(): CommitDetail | null {
		return detail;
	},
	get detailError(): string | null {
		return detailError;
	},

	/** One row, or undefined if the walk has not reached it yet. */
	row(index: number): GraphRow | undefined {
		return buffer[index];
	},

	/**
	 * Subscribe to the walk. Call once; the returned function detaches.
	 * Events carrying a stale token are dropped, which is what makes restarting
	 * the walk after a ref change safe.
	 */
	async attach(): Promise<() => void> {
		unlisteners.push(
			await listen<GraphRowsEvent>(GRAPH_ROWS_EVENT, (event) => {
				const payload = event.payload;
				if (payload.token !== repo.token) return;

				// The new walk has produced something, so the old rows can go.
				if (pendingReset) {
					buffer = [];
					count = 0;
					lanes = 1;
					complete = false;
					pendingReset = false;
				}

				let deepest = lanes;

				for (const row of payload.rows) {
					buffer[row.index] = row;
					if (row.index + 1 > count) count = row.index + 1;
					deepest = Math.max(deepest, row.lane + 1);
					for (const edge of row.edges) {
						deepest = Math.max(deepest, edge.from + 1, edge.to + 1);
					}
					// Follow the selected commit to wherever it landed.
					if (selectionUnverified && row.id === selectedId) {
						selectedIndex = row.index;
						selectionUnverified = false;
					}
				}
				if (deepest !== lanes) lanes = deepest;
				version += 1;
				repo.setCommitCount(count);
			})
		);

		unlisteners.push(
			await listen<GraphDoneEvent>(GRAPH_DONE_EVENT, (event) => {
				const payload = event.payload;
				if (payload.token !== repo.token) return;

				// A walk that ended without producing a row still has to clear
				// the old ones — the repository may now be empty.
				if (pendingReset) {
					buffer = [];
					count = 0;
					lanes = 1;
					pendingReset = false;
					version += 1;
				}

				complete = payload.complete;
				if (payload.complete) refreshedAt = Math.floor(Date.now() / 1000);
				if (payload.error) error = payload.error;

				// The selected commit never reappeared: history was rewritten
				// under it, so the selection is gone rather than merely unseen.
				if (selectionUnverified && payload.complete) {
					selectionUnverified = false;
					selectedIndex = null;
					selectedId = null;
					detail = null;
				}

				repo.setCommitCount(count);
			})
		);

		return () => {
			for (const off of unlisteners) off();
			unlisteners = [];
		};
	},

	/** Clear everything and ask for the first window of the current walk. */
	async restart(): Promise<void> {
		reset();
		const token = repo.token;
		if (token === null) return;
		requested = FIRST_WINDOW;
		try {
			await api.graphRequest(token, FIRST_WINDOW);
		} catch (e) {
			error = String(e);
		}
	},

	/**
	 * Restart against a *new* walk, after refs moved. The old token's events
	 * stop being accepted the moment the repo store takes the new one.
	 */
	async reload(): Promise<void> {
		try {
			await this.adopt(await api.graphRestart());
		} catch (e) {
			pendingReset = false;
			error = String(e);
		}
	},

	/**
	 * Take over a walk that was started elsewhere, by its token.
	 *
	 * `reload` starts one and adopts it; changing which branches are shown
	 * starts one as a side effect of setting the visibility, and adopts the
	 * token that came back. Both need the same handover, and it is the handover
	 * that is easy to get wrong — the old token's events must stop being
	 * accepted at the same moment the rows they would have appended stop being
	 * wanted.
	 */
	async adopt(token: number): Promise<void> {
		// Hold the current rows until the new walk delivers; only then do they
		// get replaced. Clearing first is what makes a refresh flash.
		pendingReset = true;
		selectionUnverified = selectedId !== null;
		requested = FIRST_WINDOW;
		error = null;
		repo.setToken(token);
		await api.graphRequest(token, FIRST_WINDOW);
	},

	/**
	 * Tell the store the viewport reaches `endIndex`. Requests more rows when
	 * the walk is running out ahead of the scroll.
	 */
	ensure(endIndex: number): void {
		const token = repo.token;
		if (token === null || complete) return;
		if (endIndex + PREFETCH < requested) return;

		requested += NEXT_WINDOW;
		api.graphRequest(token, NEXT_WINDOW).catch((e) => {
			error = String(e);
		});
	},

	/** The selected commit's id, which survives a walk the row index does not. */
	get selectedId(): string | null {
		return selectedId;
	},

	/**
	 * Ask for a commit to be selected once the walk reaches it.
	 *
	 * Used when a repository is reopened from its tab: the row index a selection
	 * had last time means nothing after a fresh walk, but the id does. The same
	 * machinery a ref-move already uses — `selectionUnverified` — carries it
	 * until the row arrives, and gives up honestly if the walk completes without
	 * it, which is what a rewritten history looks like.
	 */
	want(id: string): void {
		selectedId = id;
		selectionUnverified = true;
		selectedIndex = null;
		detail = null;
		detailError = null;
	},

	/** Select a row and load its detail panel. */
	select(index: number): void {
		const row = buffer[index];
		if (!row) return;
		selectedIndex = index;
		selectedId = row.id;
		selectionUnverified = false;
		detail = null;
		detailError = null;

		const seq = ++detailSeq;
		api
			.commitDetail(row.id)
			.then((result) => {
				if (seq === detailSeq) detail = result;
			})
			.catch((e) => {
				if (seq === detailSeq) detailError = String(e);
			});
	},

	clear(): void {
		reset();
	}
};
