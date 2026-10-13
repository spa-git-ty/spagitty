// SPDX-License-Identifier: GPL-3.0-or-later

import { describe, expect, it } from 'vitest';
import {
	DIFF_ROUTE,
	hiddenItems,
	inOrder,
	isActive,
	isHidden,
	isItemActive,
	isShown,
	moveBy,
	moveOnto,
	NAV_ITEMS,
	navRows,
	OFF_RAIL,
	RAIL_DEFAULT,
	railChoice,
	railRows,
	REF_SCREENS,
	type NavContext,
	type RailSettings,
	type ScreenCode
} from './nav';

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
			// FEAT-131. Reading any file at any revision.
			'/files',
			// FEAT-100. Where a merge starts, beside where one that stopped is finished.
			'/merge',
			'/branches',
			'/requests',
			// FEAT-087. Reading a pull request comes after browsing them.
			'/review',
			// TASK-056. The tools are always on the rail again, and Stash, Tags
			// and Reflog are rows of their own, as the author asked (reversing
			// TASK-045, which had made them tabs of Branches).
			'/rebase',
			// FEAT-132. What was File history, on the rail.
			'/investigate',
			'/search',
			'/stash',
			'/tags',
			'/reflog',
			'/badges',
			'/repos',
			'/settings'
		]);
	});

	/** FEAT-030 put Log after Rebase; FEAT-132 put Investigate between them. */
	it('puts Investigate after Rebase, and Log after that', () => {
		const hrefs = NAV_ITEMS.map((item) => item.href);
		expect(hrefs.indexOf('/investigate')).toBe(hrefs.indexOf('/rebase') + 1);
		expect(hrefs.indexOf('/search')).toBe(hrefs.indexOf('/rebase') + 2);
	});

	it('has no File history off the rail: it is Investigate, and on it (FEAT-132)', () => {
		expect(OFF_RAIL['/history']).toBeUndefined();
		expect(NAV_ITEMS.find((item) => item.code === '1O')?.href).toBe('/investigate');
	});

	it('keeps a divider before Settings, the one row not about this repository', () => {
		const settings = NAV_ITEMS.find((item) => item.href === '/settings');
		expect(settings?.dividerBefore).toBe(true);
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
		expect(starts).toEqual(['/farm', '/', '/rebase', '/settings']);
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

	it('ends with Settings, on its own', () => {
		expect(NAV_ITEMS.filter((item) => item.group === 'app').map((item) => item.href)).toEqual([
			'/settings'
		]);
	});
});

/** Which rows are on the rail right now (TASK-045). */
describe('isShown', () => {
	const quiet: NavContext = { pathname: '/', conflicts: 0, delight: false };
	const shown = (context: NavContext) =>
		NAV_ITEMS.filter((item) => isShown(item, context)).map((item) => item.href);

	it('shows the everyday rows and Settings on a quiet day', () => {
		expect(shown(quiet)).toEqual([
			'/farm',
			'/',
			'/changes',
			'/files',
			'/merge',
			'/branches',
			'/requests',
			'/review',
			'/rebase',
			'/investigate',
			'/search',
			'/stash',
			'/tags',
			'/reflog',
			'/repos',
			'/settings'
		]);
	});

	it('adds Conflicts while there is something to resolve, and only then', () => {
		expect(shown({ ...quiet, conflicts: 1 })).toContain('/conflicts');
		expect(shown({ ...quiet, conflicts: null })).not.toContain('/conflicts');
	});

	it('keeps Conflicts while it is open, even once it is empty', () => {
		expect(shown({ ...quiet, pathname: '/conflicts' })).toContain('/conflicts');
	});


	it('never offers Badges while the delight layer is off, even on its own screen', () => {
		expect(shown({ ...quiet, pathname: '/badges' })).not.toContain('/badges');
		expect(shown({ ...quiet, pathname: '/badges', delight: true })).toContain('/badges');
	});

	it('filters the rows the rail draws', () => {
		expect(navRows(NAV_ITEMS, quiet).map((row) => row.item.href)).toEqual(shown(quiet));
	});
});

describe('isItemActive', () => {
	const branches = NAV_ITEMS.find((item) => item.href === '/branches')!;

	it('Stash, Tags and Reflog are where you are on their own screens, not Branches (TASK-056)', () => {
		for (const href of REF_SCREENS.slice(1)) {
			expect(isItemActive(branches, href)).toBe(false);
			expect(NAV_ITEMS.filter((item) => isItemActive(item, href)).map((item) => item.href)).toEqual([href]);
		}
	});

	it('does not put Branches where you are anywhere else', () => {
		expect(isItemActive(branches, '/')).toBe(false);
		expect(isItemActive(branches, '/requests')).toBe(false);
	});
});

/**
 * Pin or hide what is on the rail (FEAT-134).
 *
 * 3.0 adds four rows to a rail TASK-056 had already filled. Which of them a
 * person never touches is something only they know, so each screen can be
 * taken off the rail, and the rows of a group put in another order. Nothing
 * changes for anybody who has not chosen.
 */
describe('the rail a person chose', () => {
	const quiet: NavContext = { pathname: '/', conflicts: 0, delight: false };
	const rail = (overrides: Partial<RailSettings> = {}): RailSettings => ({
		...RAIL_DEFAULT,
		...overrides
	});
	const hrefs = (context: NavContext) => navRows(NAV_ITEMS, context).map((row) => row.item.href);
	const code = (href: string): ScreenCode => NAV_ITEMS.find((item) => item.href === href)!.code;

	it('is every screen pinned, in the built-in order, until somebody chooses otherwise', () => {
		expect(RAIL_DEFAULT).toEqual({ hidden: [], order: [], where: 'more' });
		expect(hrefs({ ...quiet, rail: rail() })).toEqual(hrefs(quiet));
		expect(
			railRows(NAV_ITEMS, { ...quiet, rail: rail() }).every((row) => row.kind === 'screen')
		).toBe(true);
	});

	describe('hiding', () => {
		const chosen = rail({ hidden: [code('/rebase'), code('/stash')] });

		it('takes a hidden screen off the rail and leaves the rest where they were', () => {
			const shown = hrefs({ ...quiet, rail: chosen });

			expect(shown).not.toContain('/rebase');
			expect(shown).not.toContain('/stash');
			expect(shown).toEqual(hrefs(quiet).filter((href) => href !== '/rebase' && href !== '/stash'));
		});

		/** "Where am I" always has an answer: the rule `open` rows already had. */
		it('keeps the screen you are on, hidden or not, for as long as you are on it', () => {
			expect(hrefs({ ...quiet, pathname: '/stash', rail: chosen })).toContain('/stash');
			expect(hrefs({ ...quiet, pathname: '/tags', rail: chosen })).not.toContain('/stash');
		});

		/** Settings has to stay reachable to undo a hide, and the farm is the product. */
		it('cannot hide Farm or Settings, whatever the stored list says', () => {
			const all = rail({ hidden: NAV_ITEMS.map((item) => item.code) });
			const shown = hrefs({ ...quiet, rail: all });

			expect(shown).toContain('/farm');
			expect(shown).toContain('/settings');
			for (const href of ['/farm', '/settings']) {
				const item = NAV_ITEMS.find((candidate) => candidate.href === href)!;
				expect(railChoice(item)).toBe('always');
				expect(isHidden(item, all)).toBe(false);
			}
		});

		it('leaves Conflicts and Badges to the rules they already keep', () => {
			const conflicts = NAV_ITEMS.find((item) => item.href === '/conflicts')!;
			const badges = NAV_ITEMS.find((item) => item.href === '/badges')!;
			const all = rail({ hidden: NAV_ITEMS.map((item) => item.code) });

			expect(railChoice(conflicts)).toBe('rule');
			expect(railChoice(badges)).toBe('rule');
			// Hidden in the list, and still there the moment there is a conflict.
			expect(isShown(conflicts, { ...quiet, conflicts: 2, rail: all })).toBe(true);
			expect(isShown(conflicts, { ...quiet, rail: all })).toBe(false);
		});

		it('offers the choice on every other row', () => {
			const free = NAV_ITEMS.filter((item) => railChoice(item) === 'free').map((item) => item.href);

			expect(free).toEqual(
				NAV_ITEMS.map((item) => item.href).filter(
					(href) => !['/farm', '/settings', '/conflicts', '/badges'].includes(href)
				)
			);
		});

		it('ignores a code this build has no screen for', () => {
			const odd = rail({ hidden: ['9Z' as ScreenCode], order: ['9Z' as ScreenCode] });

			expect(hrefs({ ...quiet, rail: odd })).toEqual(hrefs(quiet));
		});
	});

	describe('the More row', () => {
		const chosen = rail({ hidden: [code('/rebase'), code('/stash'), code('/repos')] });
		const more = (context: NavContext) =>
			railRows(NAV_ITEMS, context).find((row) => row.kind === 'more');

		it('is not there with nothing hidden', () => {
			expect(more({ ...quiet, rail: rail() })).toBeUndefined();
			expect(more(quiet)).toBeUndefined();
		});

		it('holds the hidden screens, in rail order', () => {
			const row = more({ ...quiet, rail: chosen });

			expect(row?.kind === 'more' && row.items.map((item) => item.href)).toEqual([
				'/rebase',
				'/stash',
				'/repos'
			]);
		});

		it('sits at the end of Tools, immediately before Settings', () => {
			const rows = railRows(NAV_ITEMS, { ...quiet, rail: chosen });
			const at = rows.findIndex((row) => row.kind === 'more');
			const before = rows[at - 1];
			const after = rows[at + 1];

			expect(before.kind === 'screen' && before.item.group).toBe('tools');
			expect(after.kind === 'screen' && after.item.href).toBe('/settings');
			expect(rows[at].startsGroup).toBe(false);
		});

		it('starts the group itself when every tool is hidden', () => {
			const tools = NAV_ITEMS.filter((item) => item.group === 'tools').map((item) => item.code);
			const rows = railRows(NAV_ITEMS, { ...quiet, rail: rail({ hidden: tools }) });
			const at = rows.findIndex((row) => row.kind === 'more');

			expect(rows[at].startsGroup).toBe(true);
			expect(rows[at + 1].kind === 'screen' && rows[at + 1].startsGroup).toBe(true);
		});

		it('goes when hidden screens are for the palette only', () => {
			const palette = rail({ ...chosen, where: 'palette' });

			expect(more({ ...quiet, rail: palette })).toBeUndefined();
			// Hidden all the same.
			expect(hrefs({ ...quiet, rail: palette })).not.toContain('/rebase');
		});

		/** A menu whose contents change with where you are is one nobody can learn. */
		it('still lists the hidden screen that is open', () => {
			const here: NavContext = { ...quiet, pathname: '/stash', rail: chosen };

			expect(hiddenItems(NAV_ITEMS, here).map((item) => item.href)).toContain('/stash');
			expect(hrefs(here)).toContain('/stash');
		});

		it('never offers Badges while the delight layer is off', () => {
			const all = rail({ hidden: NAV_ITEMS.map((item) => item.code) });

			expect(
				hiddenItems(NAV_ITEMS, { ...quiet, rail: all }).map((item) => item.href)
			).not.toContain('/badges');
		});
	});

	describe('reordering', () => {
		const groupOf = (href: string) => NAV_ITEMS.find((item) => item.href === href)!.group;
		const arranged = (order: ScreenCode[]) => inOrder(NAV_ITEMS, order).map((item) => item.href);

		it('moves a row down and up within its group', () => {
			const down = moveBy(code('/rebase'), 1, []);
			expect(arranged(down).indexOf('/rebase')).toBe(arranged([]).indexOf('/rebase') + 1);

			// And back: the built-in order is stored as nothing at all, so Reset
			// and "moved it back by hand" are the same state.
			expect(moveBy(code('/rebase'), -1, down)).toEqual([]);
		});

		it('refuses a move that would leave the group', () => {
			const firstTool = NAV_ITEMS.find((item) => item.group === 'tools')!;
			const lastWork = NAV_ITEMS.filter((item) => item.group === 'work').at(-1)!;

			expect(moveBy(firstTool.code, -1, [])).toEqual([]);
			expect(moveBy(lastWork.code, 1, [])).toEqual([]);
			expect(moveBy(code('/farm'), 1, [])).toEqual([]);
			expect(moveBy(code('/settings'), -1, [])).toEqual([]);
		});

		it('drops a row onto another of its own group, and not onto another group', () => {
			const order = moveOnto(code('/reflog'), code('/rebase'), []);
			const tools = arranged(order).filter((href) => groupOf(href) === 'tools');

			expect(tools[0]).toBe('/reflog');
			expect(tools[1]).toBe('/rebase');
			expect(moveOnto(code('/reflog'), code('/changes'), [])).toEqual([]);
		});

		it('keeps every group one run, in the same place, whatever the order says', () => {
			// A stored order that interleaves the groups, as a hand-edit might.
			const scrambled = [...NAV_ITEMS].reverse().map((item) => item.code);
			const groups = inOrder(NAV_ITEMS, scrambled).map((item) => item.group);

			expect(groups.filter((group, index) => group !== groups[index - 1])).toEqual([
				'farm',
				'work',
				'tools',
				'app'
			]);
			expect(inOrder(NAV_ITEMS, scrambled)).toHaveLength(NAV_ITEMS.length);
		});

		/** A screen a later build adds arrives where it was designed to be. */
		it('puts a screen the order does not name after the row it follows', () => {
			const order = moveBy(code('/rebase'), 1, []);
			const without = order.filter((other) => other !== code('/stash'));
			const built = NAV_ITEMS.map((item) => item.href);
			const before = built[built.indexOf('/stash') - 1];
			const now = arranged(without);

			expect(now).toHaveLength(NAV_ITEMS.length);
			expect(now.indexOf('/stash')).toBe(now.indexOf(before) + 1);
		});

		it('draws the rail in the chosen order', () => {
			const order = moveOnto(code('/reflog'), code('/rebase'), []);
			const shown = hrefs({ ...quiet, rail: rail({ order }) });

			expect(shown.indexOf('/reflog')).toBeLessThan(shown.indexOf('/rebase'));
			expect([...shown].sort()).toEqual([...hrefs(quiet)].sort());
		});
	});
});
