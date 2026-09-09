// SPDX-License-Identifier: GPL-3.0-or-later

import { describe, expect, it } from 'vitest';
import { DIFF_ROUTE, isActive, NAV_ITEMS, navRows, OFF_RAIL } from './nav';

describe('isActive', () => {
	it('matches the graph only on the root path', () => {
		expect(isActive('/', '/')).toBe(true);
		expect(isActive('/', '')).toBe(true);
		expect(isActive('/', '/branches')).toBe(false);
	});

	it('does not treat every route as a child of the root', () => {
		// The naive `startsWith` implementation would light up the Graph item on
		// every screen, which would make the rail useless as an answer to
		// "where am I".
		for (const item of NAV_ITEMS) {
			if (item.href === '/') continue;
			expect(isActive('/', item.href)).toBe(false);
		}
	});

	it('matches an exact route', () => {
		expect(isActive('/branches', '/branches')).toBe(true);
	});

	it('matches a child route', () => {
		expect(isActive('/branches', '/branches/detail')).toBe(true);
	});

	it('does not match a sibling that merely shares a prefix', () => {
		expect(isActive('/branch', '/branches')).toBe(false);
		expect(isActive('/repos', '/repositories')).toBe(false);
	});
});

describe('NAV_ITEMS', () => {
	it('has a unique code and a unique href per item', () => {
		const codes = NAV_ITEMS.map((item) => item.code);
		const hrefs = NAV_ITEMS.map((item) => item.href);
		expect(new Set(codes).size).toBe(codes.length);
		expect(new Set(hrefs).size).toBe(hrefs.length);
	});

	it('names no keyboard shortcut, in any notation (FEAT-041)', () => {
		// The rail's right-hand column is counts. A shortcut printed there was
		// the only non-number in it, and it was written in one platform's
		// notation on every platform — the defect FEAT-021 took out of the title
		// bar. The palette lists shortcuts, per platform, in one place.
		// Serialised whole, so a shortcut smuggled back in under any property
		// name fails this rather than only the one that was removed.
		expect(JSON.stringify(NAV_ITEMS)).not.toMatch(/ctrl|cmd|⌘|⌃/i);
	});

	it('keeps the Diff screen off the rail', () => {
		expect(NAV_ITEMS.some((item) => item.href === DIFF_ROUTE)).toBe(false);
		expect(OFF_RAIL[DIFF_ROUTE].code).toBe('1B');
	});

	/**
	 * The farm is the product's own subject, and the rail's top slot is the one
	 * place in the window a person does not have to look for. The Graph follows
	 * it, and still owns `/`.
	 */
	it('starts at the farm, with the graph immediately after it', () => {
		expect(NAV_ITEMS[0].href).toBe('/farm');
		expect(NAV_ITEMS[0].code).toBe('1Q');
		expect(NAV_ITEMS[1].href).toBe('/');
		expect(NAV_ITEMS[1].code).toBe('1A');
	});

	/**
	 * FEAT-030 put Log after Rebase. The order is the screens roughly as they
	 * are worked through — what changed, what conflicts, what branches — and Log
	 * is where you go to look something up rather than a step in that sequence.
	 */
	it('runs the screens in the order they are worked through', () => {
		expect(NAV_ITEMS.map((item) => item.href)).toEqual([
			// FEAT-073. First, because supervising the farm is the day's work
			// and everything below it is where the farm's output is read.
			'/farm',
			'/',
			'/changes',
			'/conflicts',
			'/branches',
			// FEAT-051. Beside Branches because they are the same kind of
			// thing — named positions in history — and the rail already
			// counted tags with nowhere to send anyone.
			'/tags',
			'/stash',
			'/requests',
			'/rebase',
			'/search',
			// FEAT-050. After Log because they answer neighbouring questions —
			// what is in history, and what was just done to it — and before the
			// divider because both are about the open repository.
			'/reflog',
			// FEAT-072. Last of the repository screens, because it is the only
			// one that is not about the repository's state — it is about what
			// has been done in it, which is a question people ask after the
			// ones above rather than instead of them.
			'/badges',
			'/repos',
			'/settings'
		]);
	});

	it('puts Log immediately after Rebase', () => {
		const hrefs = NAV_ITEMS.map((item) => item.href);
		expect(hrefs.indexOf('/search')).toBe(hrefs.indexOf('/rebase') + 1);
	});

	it('keeps the divider before All repositories', () => {
		const repos = NAV_ITEMS.find((item) => item.href === '/repos');
		expect(repos?.dividerBefore).toBe(true);
	});
});

/**
 * The rail is four groups, not fourteen equal rows (TASK-041).
 *
 * Supervising a farm, doing routine git work, reaching for an occasional tool
 * and going somewhere that is not about this repository at all are four
 * different activities, and the rail gave the eye no way to tell them apart
 * without reading every label.
 */
describe('grouping', () => {
	const rows = navRows();

	it('covers every item exactly once, in the same order', () => {
		expect(rows.map((row) => row.item.href)).toEqual(NAV_ITEMS.map((item) => item.href));
	});

	/** Grouping must not move anything. Whatever a hand has learned still holds. */
	it('leaves the Farm first and the Graph second', () => {
		expect(rows[0].item.href).toBe('/farm');
		expect(rows[1].item.href).toBe('/');
	});

	it('gives the Farm a group of its own', () => {
		expect(NAV_ITEMS.filter((item) => item.group === 'farm')).toHaveLength(1);
		// And no heading: a heading over a single row is a label for nothing.
		expect(rows[0].heading).toBeNull();
	});

	it('starts a group exactly at each boundary', () => {
		const starts = rows.filter((row) => row.startsGroup).map((row) => row.item.href);
		expect(starts).toEqual(['/farm', '/', '/rebase', '/repos']);
	});

	it('heads every group but the first', () => {
		expect(rows.filter((row) => row.heading !== null).map((row) => row.heading)).toEqual([
			'Repository',
			'Tools',
			'Spagitty'
		]);
	});

	/**
	 * Each group is one contiguous run.
	 *
	 * The rail draws a heading wherever the group changes, so a group whose
	 * items were interleaved with another's would get two headings saying the
	 * same word — which is not a grouping, it is a list with labels sprinkled
	 * through it.
	 */
	it('keeps each group in one run', () => {
		const seen = new Set<string>();
		let previous: string | null = null;

		for (const row of rows) {
			if (row.item.group === previous) continue;
			expect(seen.has(row.item.group), `${row.item.group} appears twice`).toBe(false);
			seen.add(row.item.group);
			previous = row.item.group;
		}
	});

	it('puts the two screens that are not about this repository together', () => {
		expect(NAV_ITEMS.filter((item) => item.group === 'app').map((item) => item.href)).toEqual([
			'/repos',
			'/settings'
		]);
	});
});
