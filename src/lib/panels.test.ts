// SPDX-License-Identifier: GPL-3.0-or-later

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import {
	PANELS,
	type PanelKey,
	DETAIL_MAX,
	DETAIL_MIN,
	panels,
	RAIL_COLLAPSED_W,
	RAIL_MAX,
	RAIL_MIN
} from './panels.svelte';
import {
	CHANGES_FILES_W,
	DETAIL_W,
	DIFF_FILES_W,
	FARM_LOG_H,
	RAIL_W,
	REQUESTS_DETAIL_W,
	STASH_ENTRIES_W
} from './metrics';

const KEY = 'spagitty.panels';

function stubStorage(initial: Record<string, string> = {}) {
	const store = new Map(Object.entries(initial));
	vi.stubGlobal('localStorage', {
		getItem: (k: string) => store.get(k) ?? null,
		setItem: (k: string, v: string) => void store.set(k, v),
		removeItem: (k: string) => void store.delete(k)
	});
	return store;
}

function cssVar(name: string): string {
	return document.documentElement.style.getPropertyValue(name);
}

/**
 * The window the adaptive defaults treat as "wide" (TASK-041).
 *
 * Set in `beforeEach`, because happy-dom's own `innerWidth` is 1024 and every
 * assertion in this file that names `RAIL_W` or `DETAIL_W` is about the design
 * value rather than about a narrow window's version of it. The adaptive
 * behaviour has a describe block of its own, which sets the width it is
 * testing.
 */
const WIDE = 1600;

function stubWindowWidth(width: number) {
	Object.defineProperty(window, 'innerWidth', { value: width, configurable: true });
}

beforeEach(() => {
	stubStorage();
	stubWindowWidth(WIDE);
	panels.reset();
});

afterEach(() => {
	vi.unstubAllGlobals();
});

describe('defaults', () => {
	it('starts at the widths the design specifies', () => {
		expect(panels.rail).toBe(RAIL_W);
		expect(panels.detail).toBe(DETAIL_W);
	});
});

describe('clamping', () => {
	it('will not let a panel be dragged to uselessness', () => {
		panels.setRail(0);
		expect(panels.rail).toBe(RAIL_MIN);

		panels.setDetail(-500);
		expect(panels.detail).toBe(DETAIL_MIN);
	});

	it('will not let a panel eat the window', () => {
		panels.setRail(10_000);
		expect(panels.rail).toBe(RAIL_MAX);

		panels.setDetail(10_000);
		expect(panels.detail).toBe(DETAIL_MAX);
	});

	it('passes a width inside the range through, rounded to a whole pixel', () => {
		panels.setRail(220.6);
		expect(panels.rail).toBe(221);

		panels.setDetail(300.2);
		expect(panels.detail).toBe(300);
	});
});

describe('publishing', () => {
	it('writes the widths as the same custom properties the stylesheet reads', () => {
		panels.setRail(200);
		panels.setDetail(300);

		expect(cssVar('--rail-w')).toBe('200px');
		expect(cssVar('--detail-w')).toBe('300px');
	});

	it('publishes the clamped value, not the requested one', () => {
		panels.setRail(10_000);
		expect(cssVar('--rail-w')).toBe(`${RAIL_MAX}px`);
	});
});

describe('persistence', () => {
	it('does not write storage on every pixel of a drag', () => {
		const store = stubStorage();
		panels.setRail(200);
		panels.setRail(201);
		expect(store.has(KEY)).toBe(false);

		panels.commit();
		expect(store.has(KEY)).toBe(true);
	});

	it('restores stored widths', () => {
		stubStorage({ [KEY]: JSON.stringify({ rail: 210, detail: 330 }) });

		panels.init();

		expect(panels.rail).toBe(210);
		expect(panels.detail).toBe(330);
		expect(cssVar('--rail-w')).toBe('210px');
	});

	it('clamps stored widths, so a file edited by hand cannot break the layout', () => {
		stubStorage({ [KEY]: JSON.stringify({ rail: 9000, detail: 1 }) });

		panels.init();

		expect(panels.rail).toBe(RAIL_MAX);
		expect(panels.detail).toBe(DETAIL_MIN);
	});

	it('ignores a partial record', () => {
		stubStorage({ [KEY]: JSON.stringify({ rail: 210 }) });

		panels.init();

		expect(panels.rail).toBe(210);
		expect(panels.detail).toBe(DETAIL_W);
	});

	it('ignores values of the wrong type', () => {
		stubStorage({ [KEY]: JSON.stringify({ rail: 'wide', detail: null }) });

		panels.init();

		expect(panels.rail).toBe(RAIL_W);
		expect(panels.detail).toBe(DETAIL_W);
	});

	it('survives corrupt storage', () => {
		stubStorage({ [KEY]: 'not json' });

		expect(() => panels.init()).not.toThrow();
		expect(panels.rail).toBe(RAIL_W);
	});

	it('publishes the defaults even when nothing is stored', () => {
		stubStorage();
		panels.init();
		expect(cssVar('--rail-w')).toBe(`${RAIL_W}px`);
	});
});

describe('reset', () => {
	it('returns to the design widths and stores that', () => {
		const store = stubStorage();
		panels.setRail(300);
		panels.reset();

		expect(panels.rail).toBe(RAIL_W);
		expect(cssVar('--rail-w')).toBe(`${RAIL_W}px`);
		expect(JSON.parse(store.get(KEY) as string)).toEqual({
			rail: RAIL_W,
			detail: DETAIL_W,
			railCollapsed: false,
			// Nothing is hidden after a reset (FEAT-056).
			hidden: {},
			// FEAT-037's panels round-trip through the same record.
			requestsDetail: REQUESTS_DETAIL_W,
			changesFiles: CHANGES_FILES_W,
			diffFiles: DIFF_FILES_W,
			stashEntries: STASH_ENTRIES_W,
			farmLog: FARM_LOG_H
		});
	});
});

describe('collapsing the rail', () => {
	it('narrows the variable every other panel lays itself out against', () => {
		stubStorage();
		panels.setRail(300);

		panels.toggleRail();

		expect(panels.railCollapsed).toBe(true);
		expect(cssVar('--rail-w')).toBe(`${RAIL_COLLAPSED_W}px`);
	});

	it('gives back the width that was dragged, not the default', () => {
		stubStorage();
		panels.setRail(300);
		panels.toggleRail();

		panels.toggleRail();

		expect(panels.railCollapsed).toBe(false);
		expect(panels.rail).toBe(300);
		expect(cssVar('--rail-w')).toBe('300px');
	});

	it('survives a restart', () => {
		const store = stubStorage();
		panels.toggleRail();

		// What a restart actually reads: the stored layout carries the flag…
		expect(JSON.parse(store.get(KEY) as string).railCollapsed).toBe(true);

		// …and `init` puts it back, which is the half a fresh process runs.
		panels.init();

		expect(panels.railCollapsed).toBe(true);
		expect(cssVar('--rail-w')).toBe(`${RAIL_COLLAPSED_W}px`);
	});

	it('is undone by a reset, since a collapsed rail is not a design width', () => {
		stubStorage();
		panels.toggleRail();

		panels.reset();

		expect(panels.railCollapsed).toBe(false);
		expect(cssVar('--rail-w')).toBe(`${RAIL_W}px`);
	});
});

/**
 * FEAT-037 — every panel resizes, not just the rail and the graph's detail.
 *
 * Stash, Working copy, Diff and Pull requests each published a width as a CSS
 * variable and then gave nobody a way to change it, which reads as an oversight
 * rather than a decision.
 */
describe('the panel registry', () => {
	it('publishes every panel as its own CSS variable', () => {
		stubStorage();
		panels.reset();

		for (const [key, spec] of Object.entries(PANELS)) {
			if (key === 'rail' || key === 'detail') continue;
			expect(cssVar(`--${spec.variable}`), key).toBe(`${spec.initial}px`);
		}
	});

	it('sets and reads any panel by key', () => {
		stubStorage();
		panels.set('diffFiles', 300);

		expect(panels.size('diffFiles')).toBe(300);
		expect(cssVar('--diff-files-w')).toBe('300px');
	});

	it('clamps every panel to its own range', () => {
		stubStorage();

		for (const [key, spec] of Object.entries(PANELS)) {
			panels.set(key as PanelKey, 10_000);
			expect(panels.size(key as PanelKey), key).toBe(spec.max);

			panels.set(key as PanelKey, -10_000);
			expect(panels.size(key as PanelKey), key).toBe(spec.min);
		}
	});

	it('keeps rail and detail reachable through both APIs', () => {
		stubStorage();
		panels.set('rail', 240);
		panels.setDetail(320);

		expect(panels.rail).toBe(240);
		expect(panels.size('rail')).toBe(240);
		expect(panels.detail).toBe(320);
		expect(panels.size('detail')).toBe(320);
	});

	it('restores a stored width for a new panel', () => {
		const store = stubStorage();
		store.set(
			KEY,
			JSON.stringify({ rail: RAIL_W, detail: DETAIL_W, railCollapsed: false, diffFiles: 320 })
		);
		panels.init();

		expect(panels.size('diffFiles')).toBe(320);
	});

	/** A layout stored before a panel existed simply lacks it; no migration. */
	it('falls back to the default when a stored layout predates a panel', () => {
		const store = stubStorage();
		store.set(KEY, JSON.stringify({ rail: 200, detail: 300, railCollapsed: false }));
		panels.init();

		expect(panels.size('rail')).toBe(200);
		expect(panels.size('changesFiles')).toBe(PANELS.changesFiles.initial);
	});

	it('clamps a stored width that is out of range', () => {
		const store = stubStorage();
		store.set(KEY, JSON.stringify({ requestsDetail: 9999 }));
		panels.init();

		expect(panels.size('requestsDetail')).toBe(PANELS.requestsDetail.max);
	});

	/** Which edge a panel is anchored to is what decides its drag direction. */
	it('declares a side for every panel', () => {
		for (const [key, spec] of Object.entries(PANELS)) {
			expect(['left', 'right', 'bottom'], key).toContain(spec.side);
			expect(spec.min, key).toBeLessThan(spec.max);
			expect(spec.initial, key).toBeGreaterThanOrEqual(spec.min);
			expect(spec.initial, key).toBeLessThanOrEqual(spec.max);
		}
	});
});

/**
 * The widths a first run opens at, in the window it opens in (TASK-041).
 *
 * The handoff's numbers are a 1440-wide window's numbers. On the 1280 the
 * application actually opens at, the Graph screen reserves 791 pixels — rail,
 * refs gutter, five lanes and the detail panel — before a divider or a word of
 * a commit message, leaving 489 for the subject line on a screen whose whole
 * job is reading subject lines.
 */
describe('adaptive defaults', () => {
	it('is the design width in a wide window', () => {
		stubStorage();
		stubWindowWidth(1600);
		panels.init();

		expect(panels.rail).toBe(RAIL_W);
		expect(panels.detail).toBe(DETAIL_W);
	});

	it('gives the work more room in a narrow one', () => {
		stubStorage();
		stubWindowWidth(1280);
		panels.init();

		expect(panels.rail).toBeLessThan(RAIL_W);
		expect(panels.detail).toBeLessThan(DETAIL_W);
	});

	/**
	 * The floor is deliberately not the panel's `min`. Starting there would
	 * open every panel at the edge of usefulness with nowhere left to drag.
	 */
	it('never starts a panel at the width it stops being useful at', () => {
		stubStorage();
		stubWindowWidth(600);
		panels.init();

		expect(panels.rail).toBeGreaterThan(RAIL_MIN);
		expect(panels.detail).toBeGreaterThan(DETAIL_MIN);
		expect(panels.rail).toBeLessThan(RAIL_W);
	});

	it('is monotonic in the window width', () => {
		const widths = [900, 1100, 1280, 1440, 1920].map((width) => {
			stubStorage();
			stubWindowWidth(width);
			panels.init();
			return panels.rail;
		});

		for (let index = 1; index < widths.length; index++) {
			expect(widths[index]).toBeGreaterThanOrEqual(widths[index - 1]);
		}
	});

	/**
	 * **A dragged width is a decision and outranks the arithmetic.** Including
	 * after the window is moved to a different screen: `adapt` runs before the
	 * stored values are read, so each stored one overwrites its default.
	 */
	it('never overrides a width somebody chose', () => {
		stubStorage({ [KEY]: JSON.stringify({ rail: 300, detail: 500 }) });
		stubWindowWidth(900);
		panels.init();

		expect(panels.rail).toBe(300);
		expect(panels.detail).toBe(500);
	});

	/** A drawer's height has nothing to do with how wide the window is. */
	it('leaves a bottom drawer alone', () => {
		stubStorage();
		stubWindowWidth(900);
		panels.init();

		expect(panels.size('farmLog')).toBe(FARM_LOG_H);
	});

	it('scales every side panel, not only the two named ones', () => {
		stubStorage();
		stubWindowWidth(900);
		panels.init();

		for (const key of ['changesFiles', 'diffFiles', 'stashEntries', 'requestsDetail'] as const) {
			expect(panels.size(key), key).toBeLessThan(PANELS[key].initial);
			expect(panels.size(key), key).toBeGreaterThan(PANELS[key].min);
		}
	});
});
