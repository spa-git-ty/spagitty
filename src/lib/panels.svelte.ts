// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * Resizable panel widths.
 *
 * The handoff's 186px rail and 270px detail panel are the *defaults*, not
 * fixed values — people work at different window sizes and care about different
 * columns. Widths are clamped so a panel can never be dragged to uselessness,
 * and they persist across restarts.
 *
 * These are published as the same CSS custom properties `applyMetrics` sets, so
 * every component keeps reading `var(--rail-w)` and needs no knowledge of the
 * drag.
 */

import {
	CHANGES_FILES_W,
	DETAIL_W,
	DIFF_FILES_W,
	FARM_LOG_H,
	RAIL_W,
	REQUESTS_DETAIL_W,
	STASH_ENTRIES_W
} from './metrics';

const STORAGE_KEY = 'spagitty.panels';

export const RAIL_MIN = 140;
export const RAIL_MAX = 340;
export const DETAIL_MIN = 200;
export const DETAIL_MAX = 520;

/**
 * Every resizable panel, by key (FEAT-037).
 *
 * Until this existed only the rail and the graph's detail panel could be
 * dragged. Every other screen with a side panel — Stash, Working copy, Diff,
 * Pull requests — published a width as a CSS variable and then gave nobody a
 * way to change it, which reads as an oversight rather than a decision.
 *
 * `side` is which edge the panel is anchored to, and it is the whole difference
 * between the two drag directions: a left panel widens as the pointer moves
 * right, a right panel widens as it moves left.
 */
export interface PanelSpec {
	/** The CSS custom property the size is published as. */
	variable: string;
	/**
	 * Which edge the panel is anchored to.
	 *
	 * `bottom` is a *height* rather than a width, and it is the only difference
	 * a drawer needs: the drag reads the pointer's Y, and everything else —
	 * clamping, persistence, hiding — is the same problem (FEAT-074).
	 */
	side: 'left' | 'right' | 'bottom';
	initial: number;
	min: number;
	max: number;
}

export const PANELS = {
	rail: { variable: 'rail-w', side: 'left', initial: RAIL_W, min: RAIL_MIN, max: RAIL_MAX },
	detail: {
		variable: 'detail-w',
		side: 'right',
		initial: DETAIL_W,
		min: DETAIL_MIN,
		max: DETAIL_MAX
	},
	requestsDetail: {
		variable: 'requests-detail-w',
		side: 'right',
		initial: REQUESTS_DETAIL_W,
		min: 220,
		max: 560
	},
	changesFiles: {
		variable: 'changes-files-w',
		side: 'left',
		initial: CHANGES_FILES_W,
		min: 160,
		max: 480
	},
	diffFiles: {
		variable: 'diff-files-w',
		side: 'left',
		initial: DIFF_FILES_W,
		min: 140,
		max: 440
	},
	stashEntries: {
		variable: 'stash-entries-w',
		side: 'left',
		initial: STASH_ENTRIES_W,
		min: 200,
		max: 520
	},
	farmLog: {
		variable: 'farm-log-h',
		side: 'bottom',
		initial: FARM_LOG_H,
		// Three lines at the low end; half a tall window at the high end. A
		// drawer that can be dragged to one line is a drawer that can be closed
		// by accident, and closing it has its own control.
		min: 88,
		max: 640
	}
} as const satisfies Record<string, PanelSpec>;

export type PanelKey = keyof typeof PANELS;

/**
 * Width of the rail while it is collapsed.
 *
 * Wide enough for a glyph and its focus ring and nothing else, which is the
 * point: a collapsed rail that still fits a short label is a narrow rail, not a
 * collapsed one.
 */
export const RAIL_COLLAPSED_W = 48;

let rail = $state(RAIL_W);
let detail = $state(DETAIL_W);
/** The panels added by FEAT-037, which have no reason to be named individually. */
let extra = $state<Record<string, number>>({
	requestsDetail: REQUESTS_DETAIL_W,
	changesFiles: CHANGES_FILES_W,
	diffFiles: DIFF_FILES_W,
	stashEntries: STASH_ENTRIES_W,
	farmLog: FARM_LOG_H
});
/**
 * Collapsed to icons.
 *
 * Kept beside the widths rather than in the rail component because `--rail-w`
 * is what every other component lays itself out against — collapsing has to
 * change that one number, or the graph would keep a rail-shaped hole beside a
 * rail that is no longer there.
 */
let railCollapsed = $state(false);

/**
 * Panels that are hidden outright (FEAT-054).
 *
 * Separate from the widths, and deliberately: a hidden panel keeps the width it
 * had, so bringing it back gives you the panel you dragged rather than the one
 * the design ships. Same reasoning as the rail's collapse.
 *
 * Only the right-hand detail panels are ever hidden. The rail collapses to
 * icons instead — it is the only way between screens, and a window with no way
 * out of the screen it is on is a worse place to be than one with a panel you
 * did not want.
 */
let hidden = $state<Partial<Record<PanelKey, boolean>>>({});

function clamp(value: number, min: number, max: number): number {
	return Math.min(Math.max(Math.round(value), min), max);
}

/**
 * The window the defaults were chosen for, and the one below which they stop
 * making sense (TASK-041).
 *
 * The handoff's numbers are a 1440-wide window's numbers. On the 1280 the
 * application actually opens at, the Graph screen reserves the rail at 186, the
 * refs gutter at 186, five lanes' worth of column at 149 and the detail panel
 * at 270 — 791 pixels spoken for before a single divider or a word of a commit
 * message. That leaves 489 for the subject line and its metadata, on a screen
 * whose entire job is reading commit subjects.
 *
 * So the defaults scale with the window they first open in, between these two
 * marks. Above 1440 they are the handoff's; at 1100 and below they are the
 * narrow end of each panel's own range; between, they interpolate.
 */
const WIDE_WINDOW = 1440;
const NARROW_WINDOW = 1100;

/**
 * How much of the way from narrow to wide this window is, 0 to 1.
 *
 * One number for every panel, so the chrome scales as a set rather than each
 * panel making its own decision and the proportions drifting apart at some
 * width nobody tested.
 */
function windowFactor(width: number): number {
	if (!Number.isFinite(width) || width <= 0) return 1;
	const span = WIDE_WINDOW - NARROW_WINDOW;
	return Math.min(1, Math.max(0, (width - NARROW_WINDOW) / span));
}

/**
 * A panel's starting width in a window of `width`.
 *
 * Interpolated between a floor and the design's own value. The floor is
 * deliberately **not** the panel's `min` — that is the width below which a
 * panel stops being useful at all, and starting there would mean a narrow
 * window opens with every panel already at the edge of usefulness and no room
 * to be dragged narrower. Two-thirds of the way from the minimum to the design
 * value is a panel that is visibly tighter and still has somewhere to go.
 *
 * Pure, and exported, because "what does a 1280 window open at" is a question
 * worth answering in a test rather than by opening one.
 */
export function initialWidth(spec: PanelSpec, width: number): number {
	// A drawer's height has nothing to do with how wide the window is.
	if (spec.side === 'bottom') return spec.initial;

	const floor = spec.min + (spec.initial - spec.min) * (2 / 3);
	return clamp(floor + (spec.initial - floor) * windowFactor(width), spec.min, spec.max);
}

function publish() {
	if (typeof document === 'undefined') return;
	const root = document.documentElement;
	root.style.setProperty('--rail-w', `${railCollapsed ? RAIL_COLLAPSED_W : rail}px`);
	root.style.setProperty('--detail-w', `${hidden.detail ? 0 : detail}px`);
	for (const [key, value] of Object.entries(extra)) {
		const spec = PANELS[key as PanelKey];
		if (!spec) continue;
		const width = hidden[key as PanelKey] ? 0 : value;
		root.style.setProperty(`--${spec.variable}`, `${width}px`);
	}
}

function save() {
	try {
		localStorage.setItem(
			STORAGE_KEY,
			JSON.stringify({ rail, detail, railCollapsed, hidden, ...extra })
		);
	} catch {
		// Storage unavailable; widths just won't survive a restart.
	}
}

export const panels = {
	get rail(): number {
		return rail;
	},
	get detail(): number {
		return detail;
	},

	/** True while the rail is a strip of icons. */
	get railCollapsed(): boolean {
		return railCollapsed;
	},

	/**
	 * Collapse or expand the rail.
	 *
	 * The dragged width is kept rather than reset: expanding returns the rail
	 * the user had, which is the difference between a collapse and a reset.
	 */
	toggleRail() {
		railCollapsed = !railCollapsed;
		publish();
		save();
	},

	/** True when this panel is hidden rather than merely narrow. */
	isHidden(key: PanelKey): boolean {
		return hidden[key] === true;
	},

	/**
	 * Hide a panel, or bring it back at the width it had.
	 *
	 * The width is untouched, so this is a toggle rather than a resize — and
	 * the panel's own splitter goes with it, because a divider with nothing on
	 * one side of it is a handle that resizes nothing.
	 */
	toggleHidden(key: PanelKey) {
		hidden = { ...hidden, [key]: !hidden[key] };
		publish();
		save();
	},

	setRail(width: number) {
		rail = clamp(width, RAIL_MIN, RAIL_MAX);
		publish();
	},

	setDetail(width: number) {
		detail = clamp(width, DETAIL_MIN, DETAIL_MAX);
		publish();
	},

	/** The current width of any panel, by key. */
	size(key: PanelKey): number {
		if (key === 'rail') return rail;
		if (key === 'detail') return detail;
		return extra[key] ?? PANELS[key].initial;
	},

	/** Set any panel's width, clamped to what it can still be useful at. */
	set(key: PanelKey, width: number) {
		const spec = PANELS[key];
		if (key === 'rail') rail = clamp(width, spec.min, spec.max);
		else if (key === 'detail') detail = clamp(width, spec.min, spec.max);
		else extra = { ...extra, [key]: clamp(width, spec.min, spec.max) };
		publish();
	},

	/** Back to the widths the design specifies. */
	reset() {
		rail = RAIL_W;
		detail = DETAIL_W;
		extra = {
			requestsDetail: REQUESTS_DETAIL_W,
			changesFiles: CHANGES_FILES_W,
			diffFiles: DIFF_FILES_W,
			stashEntries: STASH_ENTRIES_W,
			farmLog: FARM_LOG_H
		};
		railCollapsed = false;
		hidden = {};
		publish();
		save();
	},

	/** Called once a drag ends, so we write storage once instead of per pixel. */
	commit() {
		save();
	},

	/**
	 * The widths a first run opens at, for the window it opens in.
	 *
	 * Only ever applied where nothing is stored — a width somebody dragged is a
	 * decision and outranks any arithmetic here, including after they move the
	 * window to a different screen. This runs before the stored values are
	 * read, so each stored one simply overwrites its adaptive default.
	 */
	adapt(width: number) {
		rail = initialWidth(PANELS.rail, width);
		detail = initialWidth(PANELS.detail, width);
		const adapted = { ...extra };
		for (const key of Object.keys(adapted) as PanelKey[]) {
			adapted[key] = initialWidth(PANELS[key], width);
		}
		extra = adapted;
	},

	init() {
		// Before the stored widths, never after: a dragged width outranks this.
		this.adapt(typeof window === 'undefined' ? WIDE_WINDOW : window.innerWidth);

		try {
			const stored = localStorage.getItem(STORAGE_KEY);
			if (stored) {
				const parsed = JSON.parse(stored) as Record<string, unknown>;
				if (typeof parsed.railCollapsed === 'boolean') railCollapsed = parsed.railCollapsed;
				if (parsed.hidden && typeof parsed.hidden === 'object') {
					const stored = parsed.hidden as Record<string, unknown>;
					const restored: Partial<Record<PanelKey, boolean>> = {};
					for (const key of Object.keys(PANELS) as PanelKey[]) {
						if (stored[key] === true) restored[key] = true;
					}
					hidden = restored;
				}
				if (typeof parsed.rail === 'number') rail = clamp(parsed.rail, RAIL_MIN, RAIL_MAX);
				if (typeof parsed.detail === 'number') {
					detail = clamp(parsed.detail, DETAIL_MIN, DETAIL_MAX);
				}
				// A width stored before this panel existed is simply absent, and
				// the default stands — no migration, and no version to bump.
				const restored = { ...extra };
				for (const key of Object.keys(restored) as PanelKey[]) {
					const value = parsed[key];
					if (typeof value === 'number') {
						restored[key] = clamp(value, PANELS[key].min, PANELS[key].max);
					}
				}
				extra = restored;
			}
		} catch {
			// Corrupt or unreadable; the defaults are fine.
		}
		publish();
	}
};
