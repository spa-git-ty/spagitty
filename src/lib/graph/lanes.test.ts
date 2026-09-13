// SPDX-License-Identifier: GPL-3.0-or-later

import { describe, expect, it, vi } from 'vitest';
import { drawLanes, lanesNeeded, visibleRange } from './lanes';
import {
	LANE_COLUMNS_MAX,
	LANE_COLUMNS_MIN,
	LANE_PITCH,
	LANE_SPAN,
	LANE_X0,
	MERGE_R,
	NODE_R,
	ROW_PITCH,
	NODE_HALO,
	laneNodeRadius,
	laneSpanFor,
	laneX,
	rowCenterY
} from '../metrics';
import type { GraphRow, LaneEdge } from '../types';

function row(index: number, lane = 0, edges: LaneEdge[] = []): GraphRow {
	return {
		index,
		id: `${index}`.padStart(40, '0'),
		short: `${index}`.padStart(7, '0'),
		summary: `commit ${index}`,
		authorName: 'Ada Lovelace',
		authorEmail: 'ada@example.com',
		initials: 'AL',
		time: 1_700_000_000 - index * 60,
		lane,
		color: lane,
		signed: false,
		parents: [],
		refs: [],
		edges
	};
}

/** Look rows up out of an array, returning undefined past either end. */
function lookup(rows: GraphRow[]) {
	return (index: number) => rows[index];
}

describe('visibleRange', () => {
	it('reports an empty range for an empty list', () => {
		// `last` below `first` is what makes the caller's `for` loop skip.
		expect(visibleRange(0, 800, 0)).toEqual({ first: 0, last: -1 });
	});

	it('never starts above the first row', () => {
		expect(visibleRange(0, 800, 1000, 4).first).toBe(0);
	});

	it('never runs past the last row', () => {
		expect(visibleRange(0, 800, 10, 4).last).toBe(9);
	});

	it('covers the viewport plus overscan on both sides', () => {
		const { first, last } = visibleRange(100 * ROW_PITCH, 10 * ROW_PITCH, 1000, 4);
		expect(first).toBe(96);
		// 100 + 10 rows of viewport + 4 of overscan.
		expect(last).toBe(114);
	});

	it('covers every row the viewport actually shows', () => {
		const viewport = 7 * ROW_PITCH + 3; // deliberately not a whole number of rows
		const scrollTop = 40;
		const { first, last } = visibleRange(scrollTop, viewport, 1000, 0);

		// Every pixel of the viewport belongs to a row inside [first, last].
		const topRow = Math.floor(scrollTop / ROW_PITCH);
		const bottomRow = Math.floor((scrollTop + viewport - 1) / ROW_PITCH);
		expect(first).toBeLessThanOrEqual(topRow);
		expect(last).toBeGreaterThanOrEqual(bottomRow);
	});

	it('honours a zero overscan', () => {
		const { first } = visibleRange(10 * ROW_PITCH, 100, 1000, 0);
		expect(first).toBe(10);
	});
});

describe('lanesNeeded', () => {
	it('needs one column for a single lane-0 row', () => {
		expect(lanesNeeded(0, 0, lookup([row(0)]))).toBe(1);
	});

	it('counts the deepest node lane', () => {
		const rows = [row(0, 0), row(1, 3)];
		expect(lanesNeeded(0, 1, lookup(rows))).toBe(4);
	});

	it('counts lanes that only ever appear as edges', () => {
		// A lane can pass straight through the window without a commit sitting
		// in it. Counting nodes alone would clip it out of the canvas.
		const rows = [row(0, 0), row(1, 0, [{ from: 6, to: 6, color: 1 }])];
		expect(lanesNeeded(0, 1, lookup(rows))).toBe(7);
	});

	it('reaches one row past the fold, matching drawLanes', () => {
		const rows = [row(0, 0), row(1, 0), row(2, 0, [{ from: 5, to: 0, color: 2 }])];
		// Asking for rows 0..1 still has to account for the band arriving at 2.
		expect(lanesNeeded(0, 1, lookup(rows))).toBe(6);
	});

	it('ignores indices with no row', () => {
		expect(lanesNeeded(0, 50, lookup([row(0)]))).toBe(1);
	});
});

/** Minimal 2D context that records the calls the drawing code makes. */
function fakeContext() {
	const calls: Array<{ op: string; args: number[] }> = [];
	const record =
		(op: string) =>
		(...args: number[]) =>
			void calls.push({ op, args });

	const ctx = {
		canvas: {},
		clearRect: record('clearRect'),
		beginPath: record('beginPath'),
		moveTo: record('moveTo'),
		lineTo: record('lineTo'),
		arcTo: record('arcTo'),
		arc: record('arc'),
		fill: record('fill'),
		stroke: record('stroke'),
		fillText: vi.fn(),
		// The node clips a face into its circle (FEAT-079). Recorded rather
		// than stubbed away, because which image reached `drawImage` is the
		// whole of what the picture tests assert.
		save: record('save'),
		restore: record('restore'),
		clip: record('clip'),
		// Recorded so a test can prove the lanes are never clipped (FEAT-081).
		rect: record('rect'),
		drawImage: vi.fn(),
		lineWidth: 0,
		lineCap: '',
		strokeStyle: '',
		fillStyle: '',
		font: '',
		textAlign: '',
		textBaseline: ''
	};
	return { ctx, calls };
}

function draw(
	rows: GraphRow[],
	first: number,
	last: number,
	overrides: {
		columns?: number;
		picture?: (row: GraphRow) => CanvasImageSource | null;
	} = {}
) {
	const { ctx, calls } = fakeContext();
	drawLanes({
		ctx: ctx as unknown as CanvasRenderingContext2D,
		width: 150,
		height: 400,
		scrollTop: 0,
		first,
		last,
		row: lookup(rows),
		colors: ['red', 'green', 'blue', 'orange', 'purple'],
		nodeRing: '#101010',
		columns: LANE_COLUMNS_MIN,
		...overrides
	});
	return { ctx, calls };
}

/**
 * The author's real picture on the node (FEAT-079).
 *
 * `lanes.ts` knows nothing about where a picture comes from — it takes a
 * lookup and draws what it returns — so these are about the geometry and the
 * fallback, which is all this module is responsible for.
 */
describe('the face on a node', () => {
	const style = { getPropertyValue: () => 'monospace' };
	vi.stubGlobal('getComputedStyle', () => style);

	/** Stands in for a decoded image; `drawLanes` only ever passes it along. */
	const face = { width: 96, height: 96 } as unknown as CanvasImageSource;

	it('draws the picture it is handed, at the node', () => {
		const { ctx } = draw([row(0)], 0, 0, { picture: () => face });

		const radius = laneNodeRadius();
		expect(ctx.drawImage).toHaveBeenCalledWith(
			face,
			laneX(0, LANE_COLUMNS_MIN) - radius,
			rowCenterY(0, ROW_PITCH) - radius,
			radius * 2,
			radius * 2
		);
	});

	it('asks for a picture once per node and no more', () => {
		// The canvas repaints on every scroll frame. A lookup that were called
		// twice per node would double whatever the caller does in it.
		const picture = vi.fn(() => face);
		draw([row(0), row(1), row(2)], 0, 2, { picture });

		expect(picture).toHaveBeenCalledTimes(3);
	});

	it('never asks for one for a merge, which is nobody\'s own work', () => {
		const merge = { ...row(0), parents: ['a', 'b'] };
		const picture = vi.fn(() => face);
		draw([merge], 0, 0, { picture });

		expect(picture).not.toHaveBeenCalled();
	});

	it('draws the node without one when there is none', () => {
		// The generated face, or a plain disc where even that cannot be made.
		// Either way the graph keeps its shape — a missing picture is the
		// ordinary case, not a failure.
		const { ctx, calls } = draw([row(0)], 0, 0, { picture: () => null });

		expect(ctx.drawImage).not.toHaveBeenCalled();
		expect(calls.some((call) => call.op === 'arc')).toBe(true);
	});

	it('draws the node without one when no lookup was given at all', () => {
		const { ctx } = draw([row(0)], 0, 0);
		expect(ctx.drawImage).not.toHaveBeenCalled();
	});
});

describe('drawLanes', () => {
	// `getComputedStyle` is read for the node font. In node there is no DOM, so
	// the three tests below supply the one property the code asks for.
	const style = { getPropertyValue: () => 'monospace' };
	vi.stubGlobal('getComputedStyle', () => style);

	it('clears the canvas before drawing', () => {
		const { calls } = draw([row(0)], 0, 0);
		expect(calls[0].op).toBe('clearRect');
	});

	it('draws a straight segment for a lane that does not move', () => {
		const rows = [row(0), row(1, 0, [{ from: 0, to: 0, color: 0 }])];
		const { calls } = draw(rows, 0, 1);

		const line = calls.find((c) => c.op === 'lineTo');
		expect(line).toBeDefined();
		expect(calls.some((c) => c.op === 'arcTo')).toBe(false);
		expect(line?.args).toEqual([laneX(0), rowCenterY(1)]);
	});

	/**
	 * A rounded right angle, not a curve (FEAT-053): down its own lane, turn,
	 * straight across, turn, down the new lane. The straight runs are what the
	 * eye follows when many lanes share one band, and a full-row S leaves none.
	 */
	it('turns square, with two rounded corners, for a lane that changes column', () => {
		const rows = [row(0), row(1, 0, [{ from: 0, to: 1, color: 1 }])];
		const { calls } = draw(rows, 0, 1);

		const corners = calls.filter((c) => c.op === 'arcTo');
		expect(corners).toHaveLength(2);

		const middle = (rowCenterY(0) + rowCenterY(1)) / 2;
		// Out of the old lane and into the crossing, at the band's middle.
		expect(corners[0]?.args.slice(0, 4)).toEqual([laneX(0), middle, laneX(1), middle]);
		// Out of the crossing and down into the new lane.
		expect(corners[1]?.args.slice(0, 4)).toEqual([laneX(1), middle, laneX(1), rowCenterY(1)]);

		// And it finishes vertically in the destination lane, at that row's centre.
		const line = calls.filter((c) => c.op === 'lineTo').at(-1);
		expect(line?.args).toEqual([laneX(1), rowCenterY(1)]);
	});

	it('turns tighter rather than bulging when the lanes are close together', () => {
		// The radius is clamped against half the crossing, so neighbouring lanes
		// at a squeezed pitch cannot round past their own corner.
		const rows = [row(0), row(1, 0, [{ from: 0, to: 1, color: 1 }])];
		const { calls } = draw(rows, 0, 1, { columns: 40 });

		const corners = calls.filter((c) => c.op === 'arcTo');
		const radius = corners[0]?.args[4] as number;
		const crossing = Math.abs(laneX(1, 40) - laneX(0, 40));

		expect(radius).toBeLessThanOrEqual(crossing / 2);
		expect(radius).toBeGreaterThan(0);
	});

	it('draws the band arriving at the first row below the fold', () => {
		// Otherwise lanes appear to stop short at the bottom edge of the window.
		const rows = [row(0), row(1, 0, [{ from: 0, to: 0, color: 0 }])];
		const { calls } = draw(rows, 0, 0);
		expect(calls.some((c) => c.op === 'lineTo')).toBe(true);
	});

	it('draws nodes on top of the edges that reach them', () => {
		// Every lane segment is a `lineTo` or an `arcTo`; a node is `arc`.
		// Asserting against the path commands rather than against `stroke`
		// keeps this true now that a node strokes its own ring.
		const rows = [row(0), row(1, 0, [{ from: 0, to: 0, color: 0 }])];
		const { calls } = draw(rows, 0, 1);

		const ops = calls.map((c) => c.op);
		const lastEdge = Math.max(ops.lastIndexOf('lineTo'), ops.lastIndexOf('arcTo'));
		expect(ops.indexOf('arc')).toBeGreaterThan(lastEdge);
	});

	it('draws one node per visible row, at its lane and row centre', () => {
		const rows = [row(0, 0), row(1, 2), row(2, 1)];
		const { calls } = draw(rows, 0, 2);

		// A head is several arcs at one centre — the ring behind it, the clip,
		// and the outline — so the node is identified by its radius.
		const heads = calls.filter((c) => c.op === 'arc' && c.args[2] === NODE_R);
		const centres = new Set(heads.map((c) => `${c.args[0]},${c.args[1]}`));

		expect(centres.size).toBe(3);
		expect(centres.has(`${laneX(2)},${rowCenterY(1)}`)).toBe(true);
	});

	it('draws a merge as a plain dot rather than a face', () => {
		// A merge is the moment two lines join, not a person's work — giving it
		// the merge author's portrait would claim they wrote the branch it
		// swallowed.
		const merge = { ...row(0, 0), parents: ['a'.repeat(40), 'b'.repeat(40)] };
		const { calls } = draw([merge, row(1, 0)], 0, 1);

		const radii = calls.filter((c) => c.op === 'arc').map((c) => c.args[2]);
		expect(radii).toContain(MERGE_R);
		expect(radii.filter((r) => r === NODE_R).length).toBeGreaterThan(0);
	});

	it('skips a node scrolled out of the canvas', () => {
		const rows = Array.from({ length: 200 }, (_, i) => row(i));
		const { ctx, calls } = fakeContext();
		drawLanes({
			ctx: ctx as unknown as CanvasRenderingContext2D,
			width: 150,
			height: 100,
			// Rows 0..3 are above the canvas at this scroll position.
			scrollTop: 100 * ROW_PITCH,
			first: 0,
			last: 199,
			row: lookup(rows),
			colors: ['red'],
			nodeRing: '#101010',
			columns: LANE_COLUMNS_MIN
		});
		const arcs = calls.filter((c) => c.op === 'arc');
		expect(arcs.length).toBeLessThan(rows.length);
		for (const arc of arcs) {
			expect(arc.args[1]).toBeGreaterThanOrEqual(-NODE_R);
			expect(arc.args[1]).toBeLessThanOrEqual(100 + NODE_R);
		}
	});

	it('cycles colours rather than running off the end of the list', () => {
		const rows = [row(0), { ...row(1, 0, [{ from: 0, to: 0, color: 12 }]), color: 12 }];
		expect(() => draw(rows, 0, 1)).not.toThrow();
	});
});

/**
 * FEAT-046. Dragging the graph column narrower is a person saying how much of
 * the window the graph gets. It is not a request for smaller faces, and the
 * reference is unambiguous about it: the avatar diameter is identical at the
 * widest and the narrowest frame, and the lanes fold behind them.
 */
describe('squeezing the column keeps the portraits', () => {
	const style = { getPropertyValue: () => 'monospace' };
	vi.stubGlobal('getComputedStyle', () => style);

	/** Every radius `drawLanes` actually asked the canvas for, deduplicated. */
	function radii(rows: GraphRow[], columns: number, width: number): number[] {
		const { ctx, calls } = fakeContext();
		drawLanes({
			ctx: ctx as unknown as CanvasRenderingContext2D,
			width,
			height: 400,
			scrollTop: 0,
			first: 0,
			last: rows.length - 1,
			row: lookup(rows),
			colors: ['red', 'green', 'blue', 'orange', 'purple'],
			nodeRing: '#101010',
			columns,
			span: laneSpanFor(width)
		});
		return [...new Set(calls.filter((c) => c.op === 'arc').map((c) => c.args[2]))];
	}

	/** Three lanes, which every ordinary repository stays under. */
	const ordinary = [row(0, 0), row(1, 1), row(2, 2)];

	it('draws the same size node at every column width it can be dragged to', () => {
		for (const width of [331, 220, 150, 110, 80, 60]) {
			expect(radii(ordinary, 3, width), `${width}px column`).toContain(NODE_R);
		}
	});

	it('keeps the full portrait however deep the history in view is', () => {
		// FEAT-081: depth is counted over the rows on screen, so a radius that
		// followed it changed size as history scrolled past. Fourteen lanes and
		// thirty draw the same head as three; only merges are smaller.
		for (const lanes of [3, 14, 30]) {
			const deep = Array.from({ length: lanes }, (_, i) => row(i, i));
			for (const width of [331, 200, 60, 40]) {
				expect(radii(deep, lanes, width), `${lanes} lanes, ${width}px`).toEqual([NODE_R + NODE_HALO, NODE_R]);
			}
		}
	});

	it('keeps the node inside the column it was given', () => {
		// BUG-003's invariant, with the lanes folding onto the edge (FEAT-081).
		for (const width of [40, 60, 90, 150, 220, 331]) {
			const span = laneSpanFor(width);
			for (const lanes of [1, 2, 3, 5, 8, LANE_COLUMNS_MAX]) {
				const deepest = laneX(lanes - 1, lanes, 1, span) + laneNodeRadius() + NODE_HALO;
				expect(deepest, `${lanes} lanes in a ${width}px column`).toBeLessThanOrEqual(width);
			}
		}
	});

	/** Draws three lanes' worth of history — a node on each, a track through each. */
	function folded(width: number) {
		const rows = [
			row(0, 0),
			row(1, 1, [
				{ from: 0, to: 0, color: 0 },
				{ from: 1, to: 1, color: 1 },
				{ from: 0, to: 2, color: 2 }
			]),
			row(2, 2, [
				{ from: 0, to: 0, color: 0 },
				{ from: 1, to: 1, color: 1 },
				{ from: 2, to: 2, color: 2 }
			])
		];
		const { ctx, calls } = fakeContext();
		drawLanes({
			ctx: ctx as unknown as CanvasRenderingContext2D,
			width,
			height: 400,
			scrollTop: 0,
			first: 0,
			last: 2,
			row: lookup(rows),
			colors: ['red', 'green', 'blue'],
			nodeRing: '#101010',
			columns: 3,
			span: laneSpanFor(width)
		});
		const strokeXs = new Set(
			calls.filter((c) => c.op === 'moveTo' || c.op === 'lineTo').map((c) => c.args[0])
		);
		const nodeXs = new Set(calls.filter((c) => c.op === 'arc').map((c) => c.args[0]));
		return { calls, strokeXs, nodeXs };
	}

	it('draws a folded lane’s track and node at the same x — no node without its lane', () => {
		// 80px leaves 35px of span: lane 1 (26) fits, lane 2 (52) folds onto 51.
		for (const width of [331, 120, 80, 60, 40]) {
			const { calls, strokeXs, nodeXs } = folded(width);
			for (const x of nodeXs) {
				expect(strokeXs.has(x), `a node at ${x} has no lane under it at ${width}px`).toBe(true);
			}
			// Nothing is clipped away: the fold is the whole mechanism.
			// Nothing clips the lanes: the fold is the whole mechanism. (A face is
			// clipped into its own circle, which only happens once nodes are drawn.)
			const firstNode = calls.findIndex((c) => c.op === 'arc');
			expect(calls.some((c) => c.op === 'rect')).toBe(false);
			expect(calls.slice(0, firstNode).some((c) => c.op === 'clip')).toBe(false);
		}
	});

	it('leaves a fitting lane still and carries the reached one onto the edge', () => {
		const { strokeXs, nodeXs } = folded(80);
		const edge = LANE_X0 + laneSpanFor(80);
		expect([...nodeXs].sort((p, q) => p - q)).toEqual([LANE_X0, LANE_X0 + LANE_PITCH, edge]);
		expect(strokeXs.has(LANE_X0 + 2 * LANE_PITCH)).toBe(false);
	});

	it('merges the whole graph into lane 0 at the minimum width', () => {
		const { strokeXs, nodeXs } = folded(40);
		expect([...nodeXs]).toEqual([LANE_X0]);
		expect([...strokeXs]).toEqual([LANE_X0]);
	});

	it('does not squeeze a two-lane repository as though five lanes had to fit', () => {
		// The phantom floor (FEAT-046), and now no squeeze at all: lanes that fit
		// keep the design pitch at every lane count up to the cap.
		expect(laneX(1, 2) - laneX(0, 2)).toBe(LANE_PITCH);
		expect(laneX(1, 5) - laneX(0, 5)).toBe(LANE_PITCH);
		expect(radii([row(0, 0), row(1, 1)], 2, 80)).toContain(NODE_R);
	});

	it('leaves the design span drawing what it always drew', () => {
		for (const columns of [1, 2, 3, LANE_COLUMNS_MIN, LANE_COLUMNS_MAX]) {
			for (let lane = 0; lane < columns; lane++) {
				expect(laneX(lane, columns, 1, LANE_SPAN)).toBe(LANE_X0 + lane * LANE_PITCH);
			}
		}
	});
});
