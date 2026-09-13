// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * The graph's columns, and the author filter that sits beside them.
 *
 * The general machinery — order, widths, minimums, the filling column, the
 * per-repository `localStorage` — moved to `$lib/ui/columns.svelte` when the
 * Branches screen needed the same thing (FEAT-047). What is left here is what
 * was always about the graph: its catalogue, its defaults, the Graph column's
 * computed width, and the author filter.
 *
 * Three columns are on by default — Branch/Tag, Graph, Commit Message — and
 * Author, Date and SHA are available. That split is not arbitrary: the first
 * three answer *what happened*, and the other three answer *who and when*,
 * which most people want on some repositories and not on others. Hence per
 * repository rather than per install.
 *
 * # Why the Graph column is different
 *
 * Every other column has a width the user sets. The Graph column's width is
 * computed from how many lanes are actually on screen (see `laneColumnWidth`),
 * because a lane column narrower than its lanes does not truncate gracefully —
 * it draws commits on top of each other. It is listed here so that it can be
 * reordered and so that the header lines up, but until someone drags it its
 * width is read from the canvas, not from this store.
 */

import { createColumns, type Column } from '$lib/ui/columns.svelte';

export type ColumnId = 'refs' | 'graph' | 'message' | 'author' | 'time' | 'sha';

export type { Column };

/** The design's three, then the three the header's menu can add. */
const CATALOGUE: Column<ColumnId>[] = [
	{ id: 'refs', label: 'Branch / Tag', width: 186, min: 90 },
	// `width: 0` means "sizes itself" — the same convention the filling message
	// column uses. The graph follows the lanes on screen until someone drags it,
	// and a stored width is what says they did (FEAT-039).
	//
	// Forty is the narrowest the column can be and still merge the whole graph
	// into lane 0 (FEAT-081): the first lane's offset, a node and the tail add
	// up to 45 comfortable and 40 compact, so at 40 no room is left for any lane
	// to sit beside lane 0 at either density, and a full-size node on lane 0
	// still ends well inside the column. At 48 the lanes stopped 3px short of
	// merging and a folded graph read as two stacks a hair apart.
	{ id: 'graph', label: 'Graph', width: 0, min: 40, computed: true, required: true },
	{ id: 'message', label: 'Commit Message', width: 0, min: 160, fills: true, required: true },
	{ id: 'author', label: 'Author', width: 150, min: 80 },
	{ id: 'time', label: 'Date / Time', width: 150, min: 90 },
	{ id: 'sha', label: 'SHA', width: 84, min: 60 }
];

const DEFAULT_ORDER: ColumnId[] = ['refs', 'graph', 'message'];

const store = createColumns<ColumnId>({
	catalogue: CATALOGUE,
	defaultOrder: DEFAULT_ORDER,
	storageKey: 'spagitty.graph.columns:'
});

let author = $state('');

export const columns = {
	/** The shown columns, in draw order, with their current widths. */
	get shown() {
		return store.shown;
	},

	/** Total width of the shown columns, or null while one of them is filling. */
	get totalWidth() {
		return store.totalWidth;
	},

	/** Everything that could be shown, for the header's menu. */
	get catalogue() {
		return store.catalogue;
	},

	/**
	 * The author being filtered on, or the empty string.
	 *
	 * A filter here **dims** rather than removes. Removing rows would leave the
	 * lanes drawing edges between commits that are no longer parent and child,
	 * which is the same reason the Log search screen shows no lanes at all — and
	 * on the graph, the shape is the thing being looked at. Dimming answers
	 * "which of these are mine" without lying about the history.
	 */
	get author(): string {
		return author;
	},

	setAuthor(next: string): void {
		author = next;
	},

	/** Does a row's author match the current filter? True when there is none. */
	matches(name: string): boolean {
		const needle = author.trim().toLowerCase();
		return needle === '' || name.toLowerCase().includes(needle);
	},

	isShown: (id: ColumnId) => store.isShown(id),
	width: (id: ColumnId) => store.width(id),
	toggle: (id: ColumnId) => store.toggle(id),
	reorder: (from: number, to: number) => store.reorder(from, to),
	resize: (id: ColumnId, next: number) => store.resize(id, next),
	drag: (id: ColumnId, next: number) => store.drag(id, next),
	settle: () => store.settle(),
	unsize: (id: ColumnId) => store.unsize(id),
	reset: () => store.reset(),
	open: (path: string | null) => store.open(path)
};
