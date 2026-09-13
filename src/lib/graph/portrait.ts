// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * Author avatars when no fetched picture has arrived.
 *
 * The graph used to draw Boring Avatars' *marble* — three soft blobs on a
 * disc. That disambiguates ("these four rows are one person") and does not
 * identify: a marble is not a face, and it is nothing like the picture a
 * person already has. The generated stand-in is therefore the ordinary
 * avatar: initials on a stable colour, the same mark the rest of the
 * application already knew how to make from a name.
 *
 * The real picture still wins when FEAT-079 has fetched one. This module is
 * only the fallback, and it has to work offline, on every machine, forever.
 *
 * Colour comes from the email — git keys a person by address, so "Ada",
 * "ada l" and "Ada Lovelace" stay one disc. Letters come from the name, which
 * is what a person recognises.
 *
 * # One geometry, two renderers
 *
 * [`portrait`] answers *what* the disc is. The graph draws it to a canvas
 * ([`portraitTile`], via `lanes.ts`) and the author column paints the same
 * colour with CSS ([`portraitBackground`]) and writes the letters into the
 * element. Both read the same description, so a head on a lane and the same
 * author's mark beside the message cannot drift apart.
 */

import { hex, isDark, parse } from '$lib/colour';
import { LANE_COLOR_COUNT, laneColorVar } from '$lib/metrics';
import { initials } from './avatar';

export interface Portrait {
	/** Index into the lane colour cycle. */
	color: number;
}

/**
 * FNV-1a, the same hash [`./avatar`] uses, so an author's colour and letters
 * come from one family of numbers rather than two that disagree about who is
 * similar to whom.
 */
function hash(text: string): number {
	let value = 0x811c9dc5;
	for (let i = 0; i < text.length; i++) {
		value ^= text.charCodeAt(i);
		value = (value + (value << 1) + (value << 4) + (value << 7) + (value << 8) + (value << 24)) >>> 0;
	}
	return value;
}

/**
 * The identity a portrait is generated from.
 *
 * The email, because it is what git actually keys a person by — the same person
 * commits as "Ada", "ada l" and "Ada Lovelace" over a career, and all three are
 * one colour if the address is the same. The name is the fallback for the
 * commits that carry no email, which git allows.
 */
export function seedOf(email: string, name = ''): string {
	const address = email.trim().toLowerCase();
	return address || name.trim().toLowerCase();
}

/** The letters drawn on the disc. The name, falling back to the address. */
export function lettersOf(name: string, email = ''): string {
	return initials(name.trim() || email.trim());
}

/** The portrait for a seed: which lane colour the disc is. */
export function portrait(seed: string): Portrait {
	return { color: hash(seed) % LANE_COLOR_COUNT };
}

/**
 * The disc as a CSS `background` value, for the DOM half of the app.
 *
 * A solid lane colour. The letters live in the element, not in the
 * background — a gradient cannot carry text.
 */
export function portraitBackground(seed: string): string {
	return `var(${laneColorVar(portrait(seed).color)})`;
}

/**
 * Ink that reads on a filled lane colour.
 *
 * White on a dark disc, near-black on a light one. Parsed from the resolved
 * colour the canvas was given; if that cannot be read, white, which is the
 * safer miss on the dark graph.
 */
export function letterColor(fill: string): string {
	const rgb = parse(fill);
	if (!rgb) return '#ffffff';
	return isDark(rgb) ? '#ffffff' : hex({ r: 26, g: 26, b: 26 });
}

/**
 * Draw a portrait into a square canvas context of `size` pixels.
 *
 * Given resolved colours rather than variable names because a canvas cannot
 * read a custom property — `lanes.ts` already resolves the lane palette once
 * per paint for exactly this reason, and hands the same array here.
 *
 * The caller owns clipping. This fills the whole square; the graph clips it to
 * a circle, and the author column's element is already round.
 */
export function drawPortrait(
	ctx: CanvasRenderingContext2D,
	seed: string,
	size: number,
	colors: string[],
	letters: string
): void {
	const fill = colors[portrait(seed).color % colors.length] || '#888';
	ctx.fillStyle = fill;
	ctx.fillRect(0, 0, size, size);

	const mark = letters.trim() || '?';
	const scale = mark.length <= 1 ? 0.48 : 0.38;
	ctx.fillStyle = letterColor(fill);
	ctx.font = `600 ${Math.max(8, Math.round(size * scale))}px system-ui, sans-serif`;
	ctx.textAlign = 'center';
	ctx.textBaseline = 'middle';
	ctx.fillText(mark, size / 2, size / 2 + size * 0.02);
}

/**
 * Portraits already drawn, keyed by seed, letters, size and palette.
 *
 * The graph repaints every visible node on every scroll frame; laying type
 * per node per frame is the cost this cache exists to remove. The palette is
 * part of the key so a theme change produces new discs rather than stale
 * ones, and the old entries fall out with [`forgetPortraits`].
 */
const drawn = new Map<string, HTMLCanvasElement | OffscreenCanvas>();

/** Bounded so a repository with thousands of authors cannot grow it forever. */
const CACHE_LIMIT = 512;

function surface(size: number): HTMLCanvasElement | OffscreenCanvas {
	if (typeof OffscreenCanvas !== 'undefined') return new OffscreenCanvas(size, size);
	const canvas = document.createElement('canvas');
	canvas.width = size;
	canvas.height = size;
	return canvas;
}

/**
 * A portrait ready to be drawn, at device resolution.
 *
 * `size` is in device pixels — the caller has already multiplied by the device
 * pixel ratio for its canvas, and a face rendered at CSS size and scaled up is
 * the one thing that would make these look cheap.
 */
export function portraitTile(
	seed: string,
	size: number,
	colors: string[],
	letters: string
): HTMLCanvasElement | OffscreenCanvas | null {
	const key = `${seed}|${letters}|${size}|${colors.join()}`;
	const cached = drawn.get(key);
	if (cached) return cached;

	const tile = surface(size);
	const ctx = tile.getContext('2d') as CanvasRenderingContext2D | null;
	if (!ctx) return null;

	drawPortrait(ctx, seed, size, colors, letters);

	if (drawn.size >= CACHE_LIMIT) {
		// Oldest first: insertion order is scroll order, so the entries that go
		// are the ones furthest from where the user is looking.
		const oldest = drawn.keys().next().value;
		if (oldest !== undefined) drawn.delete(oldest);
	}
	drawn.set(key, tile);
	return tile;
}

/** Drop every rendered portrait. Called when the theme changes. */
export function forgetPortraits(): void {
	drawn.clear();
}

/** How many portraits are held. Exposed for the cache's own test. */
export function portraitCacheSize(): number {
	return drawn.size;
}
