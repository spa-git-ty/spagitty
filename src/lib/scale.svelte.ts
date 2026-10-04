// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * How big everything is: two independent dials.
 *
 * **Text size** (90–130%) scales the type tokens and the commit-row pitch, and
 * nothing else. It is the "the font is too small" dial: chrome, gutters and
 * lane geometry keep their proportions, so the window still holds the same
 * shape and only the words get bigger. The row pitch has to follow the type or
 * a 17px line would be clipped by a 26px row.
 *
 * **Interface zoom** (100–200%) scales everything — type, structural metrics,
 * radii, lane geometry. It is the "this display is too dense" dial, and it is
 * what GitKraken's status-bar zoom does.
 *
 * They compose: effective type is `base × zoom × text`, effective structure is
 * `base × zoom`, effective row pitch is `base × zoom × text`.
 *
 * Both live in `localStorage` alongside the theme rather than with the settings
 * the backend owns, and for the same reason: they have to be applied on the
 * first frame, and the boot path cannot wait on a Tauri command to find out how
 * big the window's text is.
 */

import { applyMetrics, ROW_PITCH } from './metrics';

/**
 * The type scale's base values, in CSS pixels.
 *
 * These are `src/app.css`'s `--fs-*` declarations. The stylesheet keeps them so
 * that the first paint has a size before any JavaScript runs; this table is
 * what every subsequent paint is computed from. The two must agree, and this is
 * the one place to change them.
 */
export const TYPE_BASE: Record<string, number> = {
	'fs-ui': 15.6,
	'fs-secondary': 13.2,
	'fs-mono': 12,
	'fs-title': 19.2,
	'fs-code': 14.4
};

export const TEXT_MIN = 0.9;
export const TEXT_MAX = 1.3;
export const TEXT_STEP = 0.05;

export const ZOOM_MIN = 1;
export const ZOOM_MAX = 2;
export const ZOOM_STEP = 0.1;

const TEXT_KEY = 'spagitty.scale.text';
const ZOOM_KEY = 'spagitty.scale.zoom';

let text = $state(1);
let zoom = $state(1);

function clamp(value: number, low: number, high: number): number {
	return Math.min(Math.max(value, low), high);
}

/**
 * Round to the nearest step.
 *
 * Repeated `+= 0.1` on a float walks off the grid — 1.0999999999999999 is not a
 * zoom level anybody chose, and it is what a label would then show.
 */
function snap(value: number, step: number): number {
	return Math.round(value / step) * step;
}

function stored(key: string): number | null {
	try {
		const raw = localStorage.getItem(key);
		if (raw === null) return null;
		const value = Number(raw);
		return Number.isFinite(value) ? value : null;
	} catch {
		// Private mode or a locked-down webview.
		return null;
	}
}

function remember(key: string, value: number): void {
	try {
		localStorage.setItem(key, String(value));
	} catch {
		// It just won't persist. Not worth failing a paint over.
	}
}

/** Write the type tokens and the structural metrics onto the document. */
function apply(): void {
	if (typeof document === 'undefined') return;
	const root = document.documentElement;

	for (const [name, base] of Object.entries(TYPE_BASE)) {
		root.style.setProperty(`--${name}`, `${round(base * zoom * text)}px`);
	}

	// Structural metrics take the zoom only; the row pitch also takes the text,
	// because it is the box the text sits in.
	applyMetrics(root, zoom, zoom * text);
}

/** Type and pitch are whole pixels: a half-pixel row makes lanes shimmer. */
function round(value: number): number {
	return Math.round(value * 100) / 100;
}

function commitText(next: number): void {
	text = clamp(snap(next, TEXT_STEP), TEXT_MIN, TEXT_MAX);
	apply();
	remember(TEXT_KEY, text);
}

function commitZoom(next: number): void {
	zoom = clamp(snap(next, ZOOM_STEP), ZOOM_MIN, ZOOM_MAX);
	apply();
	remember(ZOOM_KEY, zoom);
}

export const scale = {
	/** Text size, 0.9 to 1.3. */
	get text(): number {
		return text;
	},
	/** Interface zoom, 1 to 2. */
	get zoom(): number {
		return zoom;
	},

	/**
	 * The commit-row pitch in effect, in whole CSS pixels.
	 *
	 * Everything that positions a row reads this rather than `ROW_PITCH`: the
	 * virtualized list, the lane canvas, and the scroll arithmetic. Reading it
	 * is also what subscribes a component to a zoom change, which is why it is
	 * a getter on a `$state`-backed store rather than a plain function.
	 */
	get pitch(): number {
		return Math.max(1, Math.round(ROW_PITCH * zoom * text));
	},

	setText(next: number): void {
		commitText(next);
	},
	setZoom(next: number): void {
		commitZoom(next);
	},

	zoomIn(): void {
		commitZoom(zoom + ZOOM_STEP);
	},
	zoomOut(): void {
		commitZoom(zoom - ZOOM_STEP);
	},
	/** Back to 100%. Both dials, because "reset the zoom" means all of it. */
	reset(): void {
		commitText(1);
		commitZoom(1);
	},

	/**
	 * The code size before zoom and text scale, in CSS pixels (FEAT-090).
	 *
	 * Settings › Reading chooses it; it still composes with both dials, so a
	 * zoomed window keeps code in proportion with everything else.
	 */
	setCodeSize(px: number): void {
		TYPE_BASE['fs-code'] = px;
		apply();
	},

	/** Restore what was chosen last. Called once, at boot, before the first paint. */
	init(): void {
		text = clamp(stored(TEXT_KEY) ?? 1, TEXT_MIN, TEXT_MAX);
		zoom = clamp(stored(ZOOM_KEY) ?? 1, ZOOM_MIN, ZOOM_MAX);
		apply();
	}
};
