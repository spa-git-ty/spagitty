// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * A table's columns: which are shown, in what order, how wide — per repository.
 *
 * Written for the graph and extracted here when the Branches screen needed the
 * same thing (FEAT-047). The graph's version had all of it — a catalogue,
 * order, per-column widths with minimums, one column that fills, `localStorage`
 * keyed by repository — and none of it was about commits. What *was* about
 * commits stayed behind: the author filter, and the graph column whose width
 * comes from the lanes on screen rather than from a drag.
 *
 * # Where it is kept
 *
 * `localStorage`, keyed by repository path under a prefix each table chooses.
 * This is view state for one person on one machine: putting it in the
 * repository would mean a layout choice arriving in someone else's checkout,
 * and putting it in Spagitty's config directory would mean a backend round
 * trip before the first paint of a screen that has to be instant.
 */

export interface Column<Id extends string = string> {
	id: Id;
	/** What the header says, and what a header menu would call it. */
	label: string;
	/** Current width in CSS pixels. Ignored while a column fills or computes. */
	width: number;
	/** Below this the column cannot say anything useful. */
	min: number;
	/**
	 * True for the column that takes the leftover width. At most one is, and it
	 * should be the column people read.
	 */
	fills?: boolean;
	/** True when the width is computed rather than dragged. */
	computed?: boolean;
	/** True when hiding it would leave a table that is not the table. */
	required?: boolean;
}

export interface ColumnsOptions<Id extends string> {
	/** Everything that could be shown, in the order a menu should list it. */
	catalogue: Column<Id>[];
	/** What is shown before anyone has an opinion. */
	defaultOrder: Id[];
	/** `localStorage` prefix. The repository path is appended to it. */
	storageKey: string;
}

interface Layout<Id extends string> {
	/** Shown columns, in the order they are drawn. */
	order: Id[];
	/** Widths for the ones that have a settable width. */
	widths: Partial<Record<Id, number>>;
}

export interface ColumnStore<Id extends string> {
	readonly shown: Column<Id>[];
	readonly totalWidth: number | null;
	readonly catalogue: { column: Column<Id>; shown: boolean }[];
	isShown(id: Id): boolean;
	width(id: Id): number;
	toggle(id: Id): void;
	reorder(from: number, to: number): void;
	resize(id: Id, next: number): void;
	drag(id: Id, next: number): void;
	settle(): void;
	unsize(id: Id): void;
	reset(): void;
	open(path: string | null): void;
}

export function createColumns<Id extends string>(options: ColumnsOptions<Id>): ColumnStore<Id> {
	const { catalogue, defaultOrder, storageKey } = options;

	let repoKey = $state<string | null>(null);
	let order = $state<Id[]>([...defaultOrder]);
	let widths = $state<Partial<Record<Id, number>>>({});

	function definition(id: Id): Column<Id> {
		// The catalogue is exhaustive and the id type is closed, so this cannot
		// miss — but a stored layout from a future version could name one that
		// has since been removed, and falling back is better than rendering
		// `undefined`.
		return catalogue.find((column) => column.id === id) ?? catalogue[0];
	}

	function isColumnId(value: string): value is Id {
		return catalogue.some((column) => column.id === value);
	}

	function persist(): void {
		if (repoKey === null) return;
		const layout: Layout<Id> = { order, widths };
		try {
			localStorage.setItem(storageKey + repoKey, JSON.stringify(layout));
		} catch {
			// The layout just won't be remembered. Not worth failing a paint over.
		}
	}

	/**
	 * Read a stored layout, discarding anything that no longer makes sense.
	 *
	 * A layout that has lost a required column would render as the wrong table,
	 * so the required ones are put back rather than the whole layout being
	 * thrown away — the user's other choices are still theirs.
	 */
	function restore(key: string): void {
		let layout: Layout<Id> | null = null;
		try {
			const raw = localStorage.getItem(storageKey + key);
			layout = raw === null ? null : (JSON.parse(raw) as Layout<Id>);
		} catch {
			layout = null;
		}

		if (!layout || !Array.isArray(layout.order)) {
			order = [...defaultOrder];
			widths = {};
			return;
		}

		const kept = layout.order.filter(
			(id, index): id is Id =>
				typeof id === 'string' && isColumnId(id) && layout.order.indexOf(id) === index
		);

		for (const column of catalogue) {
			if (column.required && !kept.includes(column.id)) {
				// Back where the default puts it, which keeps the required
				// columns in their intended places however the rest was arranged.
				kept.splice(Math.min(defaultOrder.indexOf(column.id), kept.length), 0, column.id);
			}
		}

		order = kept.length > 0 ? kept : [...defaultOrder];
		widths = typeof layout.widths === 'object' && layout.widths !== null ? layout.widths : {};
	}

	function shownColumns(): Column<Id>[] {
		return order.map((id) => {
			const column = definition(id);
			const stored = widths[id];

			// A filling column fills until it is dragged. A stored width is
			// exactly what "the user has an opinion about this one" means, so it
			// is also what stops it filling — no second flag to keep in step,
			// and clearing the width is what hands the fill back.
			if (column.fills) {
				return stored === undefined
					? column
					: { ...column, fills: false, width: Math.max(column.min, stored) };
			}

			return { ...column, width: stored ?? column.width };
		});
	}

	/**
	 * Set a width for the length of a drag, without writing it down (FEAT-081).
	 *
	 * `resize` used to be what a divider called on every pointer move, and
	 * every call was a synchronous `localStorage.setItem` of the whole layout
	 * — sixty serialisations a second on the one frame budget the drag has.
	 * The width is the same clamped, rounded number either way; only when it
	 * is saved changed. The drag calls this as it moves and `settle` once
	 * when it lets go, so a person who drags across the screen and back
	 * writes one layout rather than a few hundred.
	 */
	function drag(id: Id, next: number): void {
		const column = definition(id);
		const width = Math.max(column.min, Math.round(next));
		// Assigning an equal width would still notify every reader of `widths`,
		// and a pointer that moved less than half a pixel should not repaint the
		// table.
		if (widths[id] === width) return;
		widths = { ...widths, [id]: width };
	}

	return {
		/** The shown columns, in draw order, with their current widths. */
		get shown(): Column<Id>[] {
			return shownColumns();
		},

		/**
		 * Total width of the shown columns, or null while one of them is filling.
		 *
		 * Null means the table is exactly as wide as its viewport and nothing
		 * scrolls sideways. A number means the columns have been sized past it
		 * and every layer of the table has to agree on how far.
		 */
		get totalWidth(): number | null {
			let total = 0;
			for (const column of shownColumns()) {
				if (column.fills) return null;
				total += column.width;
			}
			return total;
		},

		/** Everything that could be shown, for a header's menu. */
		get catalogue(): { column: Column<Id>; shown: boolean }[] {
			return catalogue.map((column) => ({ column, shown: order.includes(column.id) }));
		},

		isShown(id: Id): boolean {
			return order.includes(id);
		},

		width(id: Id): number {
			return widths[id] ?? definition(id).width;
		},

		/** Show or hide a column. A required column cannot be hidden. */
		toggle(id: Id): void {
			if (order.includes(id)) {
				if (definition(id).required) return;
				order = order.filter((current) => current !== id);
			} else {
				order = [...order, id];
			}
			persist();
		},

		/** Move the column at `from` so that it sits at `to`. Header drag-and-drop. */
		reorder(from: number, to: number): void {
			if (from === to || from < 0 || from >= order.length) return;
			const next = [...order];
			const [moved] = next.splice(from, 1);
			next.splice(Math.max(0, Math.min(to, next.length)), 0, moved);
			order = next;
			persist();
		},

		/**
		 * Set a width, clamped to what the column can still say something in.
		 *
		 * Every column can be dragged, including one that sizes itself.
		 * `computed` and `fills` describe what a column does when it has *not*
		 * been given a width, not whether it may be given one. Both go back to
		 * sizing themselves through `unsize`.
		 */
		resize(id: Id, next: number): void {
			drag(id, next);
			persist();
		},

		drag,

		/** Write the layout as it now stands. What a drag does when it ends. */
		settle(): void {
			persist();
		},

		/**
		 * Give a column its default back.
		 *
		 * For a filling column that means filling again, which is the only way
		 * back once it has been dragged — double-clicking its divider is the
		 * gesture, because a menu item for it would be a menu item nobody finds.
		 */
		unsize(id: Id): void {
			const { [id]: _dropped, ...rest } = widths;
			widths = rest as Partial<Record<Id, number>>;
			persist();
		},

		/** Back to the default order at the design widths. */
		reset(): void {
			order = [...defaultOrder];
			widths = {};
			persist();
		},

		/**
		 * Point the store at a repository, loading its layout.
		 *
		 * Called whenever the open repository changes. A null path — no
		 * repository — keeps the last layout on screen rather than flashing back
		 * to the default while the next one opens.
		 */
		open(path: string | null): void {
			if (path === null || path === repoKey) return;
			repoKey = path;
			restore(path);
		}
	};
}
