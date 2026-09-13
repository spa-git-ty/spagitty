// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * The graph's columns, and in particular the one that fills.
 *
 * FEAT-025: the Commit Message column takes the leftover width, which is the
 * right default and used to mean it had no drag handle at all — the one column
 * a person could not size. It now fills *until dragged*, and a stored width is
 * what both gives it a size and stops it filling, so there is no second flag
 * that could disagree with the width.
 */

import { readFileSync } from 'node:fs';
import { beforeEach, describe, expect, it } from 'vitest';
import { columns } from './columns.svelte';

beforeEach(() => {
	columns.reset();
});

function shownById(id: string) {
	const column = columns.shown.find((c) => c.id === id);
	if (!column) throw new Error(`${id} is not shown`);
	return column;
}

describe('the filling column', () => {
	it('fills by default, so the messages get what is left', () => {
		expect(shownById('message').fills).toBe(true);
		expect(columns.totalWidth).toBeNull();
	});

	it('stops filling once it is dragged, and keeps the width it was given', () => {
		columns.resize('message', 420);

		const message = shownById('message');
		expect(message.fills).toBeFalsy();
		expect(message.width).toBe(420);
	});

	it('cannot be dragged below what a message needs to say anything', () => {
		columns.resize('message', 20);
		expect(shownById('message').width).toBe(160);
	});

	it('fills again when its width is cleared', () => {
		columns.resize('message', 420);
		columns.unsize('message');

		expect(shownById('message').fills).toBe(true);
		expect(columns.totalWidth).toBeNull();
	});
});

describe('the total width', () => {
	it('is null while anything fills, because the table is exactly its viewport', () => {
		expect(columns.totalWidth).toBeNull();
	});

	it('adds up every column once none of them fills', () => {
		columns.resize('message', 400);

		const total = columns.shown.reduce((sum, column) => sum + column.width, 0);
		expect(columns.totalWidth).toBe(total);
		expect(columns.totalWidth).toBeGreaterThan(400);
	});

	it('grows when another column is added', () => {
		columns.resize('message', 400);
		const before = columns.totalWidth ?? 0;

		columns.toggle('author');

		expect(columns.totalWidth).toBeGreaterThan(before);
	});
});

/**
 * FEAT-039 — the graph column sizes itself until someone sizes it.
 *
 * It used to refuse a drag outright, on the grounds that its width is the lanes
 * on screen. That was true and still left the author unable to reclaim the empty
 * half of a wide graph column: no matter how much the rest of the table was
 * narrowed, the graph stayed exactly as wide as its lane count said.
 *
 * The lanes compress into whatever width they are given — the machinery FEAT-035
 * built for a history deeper than the cap, now reachable by hand — so the column
 * can be dragged like any other, and `unsize` hands it back to the lanes.
 */
describe('the graph column', () => {
	it('sizes itself until it is dragged', () => {
		columns.reset();
		// Zero is the "size yourself" convention, shared with the filling column.
		expect(columns.width('graph')).toBe(0);
	});

	it('takes a width when it is dragged', () => {
		columns.reset();
		columns.resize('graph', 220);
		expect(columns.width('graph')).toBe(220);
	});

	it('clamps to a width lanes can still be drawn in', () => {
		columns.reset();
		columns.resize('graph', 5);
		expect(columns.width('graph')).toBe(48);
	});

	it('goes back to sizing itself on unsize', () => {
		columns.reset();
		columns.resize('graph', 220);
		columns.unsize('graph');
		expect(columns.width('graph')).toBe(0);
	});

	it('is still required, so it cannot be hidden away', () => {
		columns.reset();
		columns.toggle('graph');
		expect(columns.shown.some((column) => column.id === 'graph')).toBe(true);
	});
});

describe('resetting', () => {
	it('puts the order, the widths and the fill back', () => {
		columns.resize('refs', 120);
		columns.resize('message', 300);
		columns.toggle('sha');

		columns.reset();

		expect(columns.shown.map((c) => c.id)).toEqual(['refs', 'graph', 'message']);
		expect(shownById('refs').width).toBe(186);
		expect(shownById('message').fills).toBe(true);
	});
});

/**
 * BUG-009 — the message column could not be resized.
 *
 * The store always allowed it: `message` fills but is not `computed`, so
 * `resize` accepts it. What stopped it was the handle. Every divider sits at
 * `right: -3px`, straddling the boundary between two columns — but the last
 * column has nothing on its right except the window edge, so a third of its
 * grab area was off-screen and the rest sat against the frame. The message
 * column is last by default, so in practice it had no handle at all.
 *
 * These read the stylesheet rather than a rendered header, for the reason
 * `src/lib/ui/btn.test.ts` sets out: the test environment applies no CSS, so a
 * geometry assertion here would pass whatever the rules said.
 */
describe('BUG-009 — the last column has a grabbable handle', () => {
	const header = readFileSync('src/lib/graph/GraphHeader.svelte', 'utf8');

	function rule(selector: string): string {
		const found = new RegExp(
			`${selector.replace(/[.]/g, '\\.')}\\s*\\{([^}]*)\\}`
		).exec(header);
		if (!found) throw new Error(`no rule for ${selector}`);
		return found[1];
	}

	it('pulls the last divider fully inside the column', () => {
		expect(rule('.divider.last')).toMatch(/right:\s*0/);
	});

	it('leaves every other divider straddling its boundary', () => {
		expect(rule('.divider')).toMatch(/right:\s*-3px/);
	});

	it('marks the last column in the markup, or the rule can never apply', () => {
		expect(header).toMatch(/class:last=\{index === shown\.length - 1\}/);
	});

	/** The store's half was never the problem, and must stay that way. */
	it('still lets the store resize the filling column', () => {
		columns.reset();
		const before = columns.width('message');
		columns.resize('message', 420);

		expect(columns.width('message')).toBe(420);
		expect(columns.width('message')).not.toBe(before);
	});

	it('resizes the graph column too, now that it has a width to take', () => {
		columns.reset();
		columns.resize('graph', 200);

		expect(columns.width('graph')).toBe(200);
	});
});

/**
 * BUG-009b — a divider sizes the column on its left.
 *
 * The commit message column sits between the graph column, whose divider is
 * fixed because its width is computed from the lanes, and the detail panel's own
 * splitter. Both of its boundaries belonged to something else, so it had no
 * handle anyone would find.
 *
 * The first attempt sent the graph's divider to the column *after* it. That was
 * wrong in a way only a person dragging it could see: it changed the message
 * column's width while its left edge stayed pinned by everything before it, so
 * the column shrank from its **right** edge and left a growing gap before the
 * detail panel. The boundary did not move; only the far side of the column did.
 *
 * A divider now sizes the nearest resizable column to its **left**, skipping
 * back over the computed graph column. Everything left of the boundary grows,
 * everything right of it shifts along, and the filling column takes what is
 * left — so the boundary goes where the pointer goes and no gap can open.
 */
describe('BUG-009b — a divider sizes the column on its left', () => {
	const header = readFileSync('src/lib/graph/GraphHeader.svelte', 'utf8');

	it('sizes the column the divider sits on', () => {
		expect(header).toMatch(/return shown\[index\]\.id/);
	});

	/** The gap in the report came from inverting; nothing may invert again. */
	it('never inverts the drag', () => {
		expect(header).toMatch(/resizing\.startWidth \+ \(event\.clientX - resizing\.startX\)/);
		expect(header).not.toMatch(/invert/);
	});

	it('measures the column it is about to size, by id', () => {
		expect(header).toMatch(/querySelector<HTMLElement>\(`\[data-column="\$\{id\}"\]`\)/);
		expect(header).toMatch(/data-column=\{column\.id\}/);
	});

	/** Every column is sizable now, so no divider is ever inert. */
	it('leaves no divider without a job', () => {
		expect(header).not.toMatch(/class:fixed/);
	});

	it('names the column it will actually size in its title', () => {
		expect(header).toMatch(/Resize \$\{sized\}/);
	});

	/**
	 * The message column must keep filling. A gap between it and the detail panel
	 * is the defect this item exists to remove, and it can only appear if the
	 * message column is given a width of its own.
	 */
	it('leaves the filling column filling', () => {
		columns.reset();
		const message = columns.catalogue.find((entry) => entry.column.id === 'message');

		expect(message?.column.fills).toBe(true);
		expect(columns.width('message')).toBe(0);
	});
});

/**
 * BUG-003's structural guard, written before FEAT-047 moved this store.
 *
 * The lane canvas does not know where the graph column starts. It is placed by
 * a layer that mirrors the **panning** half of the row — one spacer per column
 * ahead of the graph, at the widths the cells use — so the browser does the
 * arithmetic and the two cannot disagree. The commit-list pane is not part of
 * that layer: the graph slides under it rather than being padded out to it.
 *
 * BUG-003 was that arrangement not existing: the canvas sat at a constant
 * offset, and dragging, reordering or hiding the Branch/Tag column left the
 * lanes drawn over the messages.
 *
 * Asserted against the source because happy-dom does no layout, so there are no
 * real widths to measure. What can be checked is that the mirror is still built
 * the way the fix built it, which is what makes the regression structurally
 * unable to come back.
 */
describe('the lane layer still mirrors the row (BUG-003)', () => {
	const rows = readFileSync('src/lib/graph/CommitRows.svelte', 'utf8');

	function laneLayer(): string {
		const start = rows.indexOf('class="lane-layer"');
		const end = rows.indexOf('</div>', rows.indexOf('{/each}', start));
		if (start < 0 || end < 0) throw new Error('no lane layer in CommitRows');
		return rows.slice(start, end);
	}

	it('draws one spacer per panning column, in the order they are drawn', () => {
		expect(laneLayer()).toMatch(/\{#each shown\.slice\(0, freezeIndex\) as column \(column\.id\)\}/);
	});

	it('gives every fixed column a spacer of that column’s own width', () => {
		expect(laneLayer()).toMatch(/class="lane-gap" style="width: \{column\.width\}px"/);
	});

	it('does not pad the layer out to the message column', () => {
		// A fill gap here would keep the canvas in step with a table that
		// translated as one piece. The list pane is pinned now, so that gap
		// would be a hole the graph slid into without the pane covering it.
		expect(laneLayer()).not.toMatch(/lane-gap fill/);
	});

	it('puts the canvas in the graph column’s own slot', () => {
		const layer = laneLayer();
		expect(layer).toMatch(/\{#if column\.id === 'graph'\}/);
		expect(layer).toMatch(/class="lane-slot" style="width: \{laneWidth\}px"/);
	});

	it('never places the canvas from a constant', () => {
		// The defect itself: `--refs-gutter-w` was the offset, and it was right
		// only while Branch/Tag was first and at its default width.
		expect(laneLayer()).not.toMatch(/refs-gutter/);
	});

	it('is the only thing that positions the canvas', () => {
		// One layer, one canvas. A second placement is a second thing to keep in
		// step with the cells, which is the shape of the original bug.
		expect(rows.match(/<LaneCanvas/g)?.length).toBe(1);
	});
});
