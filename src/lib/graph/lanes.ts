// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * Lane drawing.
 *
 * One canvas covers the lane column for the *visible* window only. A repository
 * with 200k commits draws the same number of paths as one with 40 — whatever
 * fits on screen — so scrolling cost is flat.
 *
 * All geometry derives from `metrics.ts`. There is no pixel constant in this
 * file.
 */

import {
	ELBOW_RADIUS,
	LANE_SPAN,
	LANE_STROKE,
	MERGE_R,
	ROW_PITCH,
	laneNodeRadius,
	laneX,
	rowCenterY
} from '../metrics';
import { portraitTile, seedOf } from './portrait';
import type { GraphRow } from '../types';

const TAU = Math.PI * 2;

export interface LaneDrawOptions {
	ctx: CanvasRenderingContext2D;
	width: number;
	height: number;
	scrollTop: number;
	/** Inclusive row range to draw. */
	first: number;
	last: number;
	row: (index: number) => GraphRow | undefined;
	/** Resolved lane colors, in cycle order. */
	colors: string[];
	/**
	 * Colour of the ring around a portrait, and of the graph column behind it.
	 *
	 * The ring is painted in the *background* colour rather than the lane's, so
	 * a head sitting on its own lane line reads as a bead on a thread rather
	 * than as a blob the line runs into.
	 */
	nodeRing: string;
	/** Lane columns the canvas is currently sized for. */
	columns: number;
	/**
	 * Horizontal room the lanes share.
	 *
	 * `LANE_SPAN` until the graph column is dragged; after that it is whatever
	 * the chosen width leaves, and the lanes compress into it (FEAT-039).
	 */
	span?: number;
	/** Row height in effect. `scale.pitch`, not the design constant. */
	pitch?: number;
	/** Interface zoom, which scales the horizontal geometry and the node. */
	zoom?: number;
	/**
	 * Rows to paint at full strength while everything else fades. Null means
	 * nothing is dimmed, which is not the same as an empty set — an empty set
	 * fades every row.
	 *
	 * Only the author filter uses this now. Hovering a branch used to dim
	 * everything outside it, and that came out in FEAT-023: a hover is a
	 * pointer resting somewhere, and answering it by draining the colour out of
	 * most of the screen makes the graph flicker as the mouse crosses it.
	 */
	highlight?: Set<number> | null;
	/**
	 * Rows carrying a stash, and how many. A stash is a commit hanging off the
	 * row it was made on, so it is drawn beside that row's node rather than
	 * being given a row of its own — see `overlay.svelte.ts`.
	 */
	stashes?: Map<number, number>;
	/**
	 * The author's real picture for a row, when one has already been fetched
	 * and decoded (FEAT-079).
	 *
	 * A lookup passed in rather than a store imported, so this module stays
	 * what it is: geometry and paint, with no idea that a network exists. It is
	 * also what makes the node testable — a test hands in a function.
	 *
	 * It must never start a request. The canvas runs inside a scroll frame and
	 * a fling across a large repository would otherwise be a request per author
	 * per frame; the rows do the asking, and this draws what asking produced.
	 */
	picture?: (row: GraphRow) => CanvasImageSource | null;
}

/** How much of its colour a row keeps when another branch is being hovered. */
const FADED = 0.22;

export function drawLanes(options: LaneDrawOptions): void {
	const {
		ctx,
		width,
		height,
		scrollTop,
		first,
		last,
		row,
		colors,
		nodeRing,
		columns,
		span = LANE_SPAN,
		pitch = ROW_PITCH,
		zoom = 1,
		highlight = null,
		stashes,
		picture
	} = options;

	ctx.clearRect(0, 0, width, height);
	ctx.lineWidth = LANE_STROKE * zoom;
	ctx.lineCap = 'round';

	/** Full strength unless a highlight is running and this row is outside it. */
	const alphaFor = (index: number): number =>
		highlight === null || highlight.has(index) ? 1 : FADED;

	// Edges first, so nodes sit on top of the lines that reach them.
	//
	// A row's edges describe the band *above* it, so the range runs one past
	// `last` — otherwise the segment arriving at the first row below the fold
	// would be missing and lanes would appear to stop short at the bottom edge.
	// The corner radius follows the pitch, so a squeezed row turns proportionally
	// tighter rather than keeping a corner too big for the space.
	const corner = (ELBOW_RADIUS / ROW_PITCH) * pitch;

	for (let i = first; i <= last + 1; i++) {
		const commit = row(i);
		if (!commit) continue;

		const bottom = rowCenterY(i, pitch) - scrollTop;
		const top = rowCenterY(i - 1, pitch) - scrollTop;
		// A band belongs to both rows it joins; it stays bright if either end is
		// in the highlight, so a branch's line is not cut off at its own tip.
		ctx.globalAlpha = Math.max(alphaFor(i), alphaFor(i - 1));

		for (const edge of commit.edges) {
			const x0 = laneX(edge.from, columns, zoom, span);
			const x1 = laneX(edge.to, columns, zoom, span);

			ctx.strokeStyle = colors[edge.color % colors.length];
			ctx.beginPath();
			ctx.moveTo(x0, top);
			if (x0 === x1) {
				ctx.lineTo(x1, bottom);
			} else {
				// A rounded right angle, not a curve: straight down its own lane,
				// turn, straight across, turn, straight down the new one. The
				// straight runs are the point — they are what the eye follows
				// when thirty lanes share a band.
				//
				// The radius is clamped against half the crossing and half the
				// row so a jump between neighbouring lanes at a squeezed pitch
				// turns tighter instead of bulging past its own corner.
				const middle = (top + bottom) / 2;
				const radius = Math.min(
					corner,
					Math.abs(x1 - x0) / 2,
					Math.abs(bottom - top) / 2
				);

				// `arcTo` rounds the corner between the line into the point and
				// the line out of it, which is exactly a quarter turn here.
				ctx.arcTo(x0, middle, x1, middle, radius);
				ctx.arcTo(x1, middle, x1, bottom, radius);
				ctx.lineTo(x1, bottom);
			}
			ctx.stroke();
		}
	}
	ctx.globalAlpha = 1;

	// The node follows the *depth*, not the drag (FEAT-046).
	//
	// Measured against the design span rather than the one in effect, so a
	// column someone dragged narrower keeps full-size portraits and the lanes
	// fold behind them — which is what the reference does, and what dragging a
	// column is asking for: less of the window for the graph, not smaller
	// faces. A history deeper than the design span can hold still shrinks, and
	// there the shrink is what keeps the column readable rather than something
	// anyone chose.
	//
	// This reverses FEAT-035's decision in the case the user caused. Its
	// argument — that portraits at full size redraw over the compression they
	// were meant to make room for — is true, and is now the intended picture.
	const radius = laneNodeRadius(columns) * zoom;
	const ratio = devicePixelRatio();
	const tileSize = Math.max(8, Math.round(radius * 2 * ratio));

	for (let i = first; i <= last; i++) {
		const commit = row(i);
		if (!commit) continue;

		const x = laneX(commit.lane, columns, zoom, span);
		const y = rowCenterY(i, pitch) - scrollTop;
		if (y < -radius || y > height + radius) continue;

		ctx.globalAlpha = alphaFor(i);
		const lane = colors[commit.color % colors.length];

		// A merge is the moment two lines join rather than one person's work, so
		// it is a plain dot in the lane's colour. Giving it a face would claim
		// the merge commit's author drew the branch it swallowed.
		if (commit.parents.length > 1) {
			ctx.fillStyle = lane;
			ctx.beginPath();
			ctx.arc(x, y, MERGE_R * zoom, 0, TAU);
			ctx.fill();
		} else {
			drawHead(
				ctx,
				commit,
				x,
				y,
				radius,
				tileSize,
				colors,
				lane,
				nodeRing,
				picture?.(commit) ?? null
			);
		}

		// A stash sits to the right of the commit it was made on, joined by a
		// short stub: a diamond, so it is not mistaken for a commit at a glance.
		const count = stashes?.get(i) ?? 0;
		if (count > 0) {
			const at = x + radius * 1.7;
			ctx.strokeStyle = lane;
			ctx.lineWidth = LANE_STROKE * zoom;
			ctx.beginPath();
			ctx.moveTo(x + radius, y);
			ctx.lineTo(at - radius * 0.55, y);
			ctx.stroke();

			ctx.fillStyle = lane;
			diamond(ctx, at, y, radius * 0.5);
			ctx.fill();
		}
	}
	ctx.globalAlpha = 1;
}

/**
 * One commit's node: the author's face, clipped to a circle, ringed in the
 * column's own background colour and outlined in the lane's.
 *
 * Three faces, in order of how much they say about who the author is:
 *
 * 1. the **real picture**, when one has been fetched for this address
 *    (FEAT-079). It identifies;
 * 2. the **generated portrait** from `portrait.ts`, pre-rendered at device
 *    resolution and cached because this runs for every visible node on every
 *    scroll frame. It disambiguates — the same person is the same face — which
 *    is most of the value and needs no network;
 * 3. a **filled disc** in the lane colour, when no portrait can be produced —
 *    no 2d context, which happens in tests and in a webview that has run out of
 *    canvases. The graph never loses its shape over a decoration.
 */
function drawHead(
	ctx: CanvasRenderingContext2D,
	commit: GraphRow,
	x: number,
	y: number,
	radius: number,
	tileSize: number,
	colors: string[],
	lane: string,
	ring: string,
	real: CanvasImageSource | null
): void {
	const tile =
		real ?? portraitTile(seedOf(commit.authorEmail ?? '', commit.authorName), tileSize, colors);

	// The gap that separates a head from the line running behind it. Two pixels,
	// not the lane's own stroke width — a thicker halo eats the daylight between
	// vertically adjacent heads.
	ctx.fillStyle = ring;
	ctx.beginPath();
	ctx.arc(x, y, radius + 2, 0, TAU);
	ctx.fill();

	if (tile) {
		ctx.save();
		ctx.beginPath();
		ctx.arc(x, y, radius, 0, TAU);
		ctx.clip();
		ctx.drawImage(tile as CanvasImageSource, x - radius, y - radius, radius * 2, radius * 2);
		ctx.restore();
	} else {
		ctx.fillStyle = lane;
		ctx.beginPath();
		ctx.arc(x, y, radius, 0, TAU);
		ctx.fill();
	}

	ctx.strokeStyle = lane;
	ctx.lineWidth = 2;
	ctx.beginPath();
	ctx.arc(x, y, radius, 0, TAU);
	ctx.stroke();
}

/** The device pixel ratio, guarded for the environments that have no window. */
function devicePixelRatio(): number {
	return typeof window === 'undefined' ? 1 : Math.min(3, Math.max(1, window.devicePixelRatio || 1));
}

/**
 * The ghost branch: a dashed line from a bare commit up to the nearest row that
 * carries a reference.
 *
 * Dashed and drawn last, over the lanes, because it is not part of the history
 * — it is the graph answering "where does this one live" for as long as the
 * pointer is on it.
 */
function drawGhost(
	ctx: CanvasRenderingContext2D,
	path: number[],
	row: (index: number) => GraphRow | undefined,
	columns: number,
	pitch: number,
	zoom: number,
	scrollTop: number,
	colors: string[],
	span: number
): void {
	if (path.length < 2) return;

	ctx.save();
	ctx.setLineDash([3 * zoom, 3 * zoom]);
	ctx.lineWidth = LANE_STROKE * zoom;
	ctx.strokeStyle = colors[0];
	ctx.globalAlpha = 0.85;

	ctx.beginPath();
	let started = false;
	for (const index of path) {
		const commit = row(index);
		if (!commit) continue;
		const x = laneX(commit.lane, columns, zoom, span);
		const y = rowCenterY(index, pitch) - scrollTop;
		if (started) ctx.lineTo(x, y);
		else {
			ctx.moveTo(x, y);
			started = true;
		}
	}
	ctx.stroke();
	ctx.restore();
}

/** A diamond, centred, with `r` from centre to point. */
function diamond(ctx: CanvasRenderingContext2D, x: number, y: number, r: number): void {
	ctx.beginPath();
	ctx.moveTo(x, y - r);
	ctx.lineTo(x + r, y);
	ctx.lineTo(x, y + r);
	ctx.lineTo(x - r, y);
	ctx.closePath();
}

/**
 * A square with rounded corners, centred on `x, y` and sized so that it reads
 * as the same weight as a circle of radius `r` beside it.
 *
 * `roundRect` would be shorter, but it is not in every webview Spagitty ships
 * against, and a merge node that silently stops being drawn is worse than four
 * arcs written out.
 */
function roundedSquare(
	ctx: CanvasRenderingContext2D,
	x: number,
	y: number,
	r: number,
	corner: number
): void {
	// 0.88 keeps the square's area close to the circle's; at 1.0 it reads as
	// noticeably bigger, because a square of side 2r has more of it.
	const half = r * 0.88;
	const left = x - half;
	const right = x + half;
	const top = y - half;
	const bottom = y + half;

	ctx.beginPath();
	ctx.moveTo(left + corner, top);
	ctx.arcTo(right, top, right, bottom, corner);
	ctx.arcTo(right, bottom, left, bottom, corner);
	ctx.arcTo(left, bottom, left, top, corner);
	ctx.arcTo(left, top, right, top, corner);
	ctx.closePath();
}

/**
 * How many lane columns the rows in view actually need.
 *
 * Counts edges as well as nodes: a lane can pass straight through the whole
 * window without a single commit sitting in it, and it still has to be drawn.
 * The `last + 1` matches `drawLanes`, which reaches one row past the fold for
 * the band arriving there.
 */
export function lanesNeeded(
	first: number,
	last: number,
	row: (index: number) => GraphRow | undefined
): number {
	let deepest = 0;
	for (let i = first; i <= last + 1; i++) {
		const commit = row(i);
		if (!commit) continue;
		if (commit.lane > deepest) deepest = commit.lane;
		for (const edge of commit.edges) {
			if (edge.from > deepest) deepest = edge.from;
			if (edge.to > deepest) deepest = edge.to;
		}
	}
	return deepest + 1;
}

/**
 * Which rows are in view. Overscan keeps a row of slack above and below so a
 * fast scroll does not show a blank strip before the next frame.
 */
export function visibleRange(
	scrollTop: number,
	viewportHeight: number,
	count: number,
	overscan = 4,
	pitch: number = ROW_PITCH
): { first: number; last: number } {
	if (count === 0) return { first: 0, last: -1 };
	const first = Math.max(0, Math.floor(scrollTop / pitch) - overscan);
	const last = Math.min(count - 1, Math.ceil((scrollTop + viewportHeight) / pitch) + overscan);
	return { first, last };
}
