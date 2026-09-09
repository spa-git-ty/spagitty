// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * How much room the graph column asks for (TASK-041).
 *
 * Two things are worth asserting and they are different questions. The **store**
 * is a preference that persists and refuses nonsense. The **geometry** is what
 * the two settings actually produce, and that is where the feature either works
 * or is a label on a control that changes nothing.
 *
 * The second one matters most: at the design's own metrics a three-lane
 * repository is still given five lanes' worth of column, and on a 1280 window
 * that column is competing with the commit subject — which is what the screen
 * is for.
 */

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { density, DENSITIES } from './density.svelte';
import {
	COMFORTABLE,
	COMPACT,
	LANE_COLUMNS_MAX,
	LANE_PITCH_MIN,
	laneColumnWidth,
	laneColumns,
	laneNodeRadius,
	lanePitch,
	laneSpanFor,
	laneSpanOf,
	laneX,
	MERGE_R
} from '../metrics';

const KEY = 'spagitty.graph.density';

function stubStorage(initial: Record<string, string> = {}) {
	const store = new Map(Object.entries(initial));
	vi.stubGlobal('localStorage', {
		getItem: (key: string) => store.get(key) ?? null,
		setItem: (key: string, value: string) => void store.set(key, value)
	});
	return store;
}

beforeEach(() => {
	stubStorage();
	density.set('comfortable');
});

afterEach(() => {
	vi.unstubAllGlobals();
});

describe('the preference', () => {
	it('starts comfortable, which is what every release before this drew', () => {
		stubStorage();
		density.init();

		expect(density.id).toBe('comfortable');
		expect(density.current).toEqual(COMFORTABLE);
	});

	it('persists and restores', () => {
		const store = stubStorage();
		density.set('compact');
		expect(store.get(KEY)).toBe('compact');

		// A fresh launch: the store survives, the module state does not.
		stubStorage({ [KEY]: 'compact' });
		density.init();
		expect(density.id).toBe('compact');
	});

	it('ignores a stored value that is not a density', () => {
		stubStorage({ [KEY]: 'tiny' });
		density.init();

		expect(density.id).toBe('comfortable');
	});

	it('survives storage being unreadable', () => {
		vi.stubGlobal('localStorage', {
			getItem: () => {
				throw new Error('denied');
			},
			setItem: () => {
				throw new Error('denied');
			}
		});

		expect(() => density.init()).not.toThrow();
		expect(() => density.set('compact')).not.toThrow();
	});

	/**
	 * The preference is a name, not the numbers. Storing `{pitch: 16}` would
	 * freeze one release's idea of compact into every installation that ever
	 * chose it, and the numbers are exactly what gets retuned after somebody
	 * looks at a screenshot.
	 */
	it('stores a name rather than a geometry', () => {
		const store = stubStorage();
		density.set('compact');

		expect(store.get(KEY)).toBe('compact');
		expect(store.get(KEY)).not.toContain('pitch');
	});

	/** Portraits are a property of the choice, not of a radius somebody
	 *  compared against a magic number in a painter. */
	it('says whether a node is big enough to be a face', () => {
		density.set('comfortable');
		expect(density.portraits).toBe(true);

		density.set('compact');
		expect(density.portraits).toBe(false);
	});

	it('offers every density it can be set to', () => {
		expect(DENSITIES.map((option) => option.id)).toEqual(['comfortable', 'compact']);
		for (const option of DENSITIES) {
			density.set(option.id);
			expect(density.id).toBe(option.id);
		}
	});
});

/**
 * What the choice is actually worth, in pixels.
 *
 * The numbers below are the point of the feature, so they are asserted rather
 * than described: a "compact" that gave back twenty pixels would be a control
 * that does nothing anybody can see.
 */
describe('what compact buys', () => {
	/**
	 * The case the design's metrics are worst at. Most repositories are two or
	 * three lanes deep, and the column is sized for five regardless.
	 */
	it('gives an ordinary three-lane history most of its column back', () => {
		const comfortable = laneColumnWidth(3, 1, COMFORTABLE);
		const compact = laneColumnWidth(3, 1, COMPACT);

		// 16 + 2 x 26 + 11 + 18, against 16 + 2 x 16 + 6 + 18.
		expect(comfortable).toBe(149);
		expect(compact).toBe(72);
		// Seventy-seven pixels, straight into the commit subject on a screen
		// whose whole job is reading commit subjects.
		expect(comfortable - compact).toBe(77);
	});

	it('still holds five lanes in less room than comfortable holds three', () => {
		expect(laneColumnWidth(5, 1, COMPACT)).toBeLessThan(laneColumnWidth(3, 1, COMFORTABLE));
	});

	it('is narrower at every depth, not only the shallow ones', () => {
		for (const lanes of [1, 3, 5, 8, 12, 20]) {
			expect(
				laneColumnWidth(lanes, 1, COMPACT),
				`${lanes} lanes`
			).toBeLessThan(laneColumnWidth(lanes, 1, COMFORTABLE));
		}
	});

	it('scales with zoom like every other metric here', () => {
		expect(laneColumnWidth(5, 2, COMPACT)).toBe(laneColumnWidth(5, 1, COMPACT) * 2);
	});
});

describe('the geometry stays consistent with itself', () => {
	/**
	 * The whole argument for the comfortable pitch is that a lane closer than a
	 * node is wide draws lines through faces. That has to hold for the compact
	 * pair too, or compact is a column of overlapping marks.
	 */
	it.each([
		['comfortable', COMFORTABLE],
		['compact', COMPACT]
	])('%s keeps a node inside its own pitch', (_label, chosen) => {
		expect(chosen.node * 2).toBeLessThan(chosen.pitch);
	});

	/**
	 * A resting pitch at or below the compression floor would leave a deep
	 * history in compact mode with nothing left to give.
	 */
	it('leaves compact room to compress', () => {
		expect(COMPACT.pitch).toBeGreaterThan(LANE_PITCH_MIN);
	});

	it('floors the column at its own minimum, not the other one', () => {
		expect(laneColumns(1, COMPACT)).toBe(COMPACT.columnsMin);
		expect(laneColumns(1, COMFORTABLE)).toBe(COMFORTABLE.columnsMin);
		// The cap is shared: it is a cap on width, and both densities have one.
		expect(laneColumns(99, COMPACT)).toBe(LANE_COLUMNS_MAX);
	});

	it('rests at its own pitch and compresses from there', () => {
		const span = laneSpanOf(COMPACT);

		expect(lanePitch(2, span, COMPACT)).toBe(COMPACT.pitch);
		expect(lanePitch(LANE_COLUMNS_MAX, span, COMPACT)).toBe(COMPACT.pitch);
		expect(lanePitch(40, span, COMPACT)).toBeLessThan(COMPACT.pitch);
		expect(lanePitch(40, span, COMPACT)).toBeGreaterThanOrEqual(LANE_PITCH_MIN);
	});

	it('rests at its own node size and shrinks from there', () => {
		const span = laneSpanOf(COMPACT);

		expect(laneNodeRadius(3, span, COMPACT)).toBe(COMPACT.node);
		expect(laneNodeRadius(40, span, COMPACT)).toBeLessThan(COMPACT.node);
		expect(laneNodeRadius(400, span, COMPACT)).toBeGreaterThanOrEqual(MERGE_R);
	});

	/**
	 * BUG-003's invariant, at both densities: a node drawn past the column's own
	 * edge is the defect that started the graph's geometry being written down
	 * at all.
	 */
	it.each([
		['comfortable', COMFORTABLE],
		['compact', COMPACT]
	])('%s keeps every node inside the column it was given', (_label, chosen) => {
		for (const width of [400, 300, 200, 120, 80]) {
			const span = laneSpanFor(width, 1, chosen);
			for (const lanes of [1, 3, 5, 12, 30]) {
				const deepest = laneX(lanes - 1, lanes, 1, span, chosen) + laneNodeRadius(lanes, span, chosen);
				expect(deepest, `${lanes} lanes in ${width}px`).toBeLessThanOrEqual(width);
			}
		}
	});

	/** Lane 0 sits at the same x either way: the column's left edge does not
	 *  move, so the rows and the canvas agree without knowing the density. */
	it('starts both densities at the same first lane', () => {
		expect(laneX(0, 5, 1, laneSpanOf(COMPACT), COMPACT)).toBe(
			laneX(0, 5, 1, laneSpanOf(COMFORTABLE), COMFORTABLE)
		);
	});
});
