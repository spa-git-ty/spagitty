// SPDX-License-Identifier: GPL-3.0-or-later

import type { IconName } from './ui/icons';
import type { RepoCounts } from './types';

/**
 * The nav rail is the only source of "where am I" — the active item and the
 * route are the same fact. Screen order and labels come from the handoff and
 * use standard git terminology verbatim.
 */

export type CountKey = keyof RepoCounts;

/**
 * Screen codes.
 *
 * Short handles so a screen can be named in one token in conversation and in
 * commit messages — "1A" rather than "the graph screen". The letters follow the
 * order the screens appear in the design handoff.
 *
 *   1A  Graph              1G  Stash
 *   1B  Diff               1H  Pull requests
 *   1C  Working copy       1I  Log search
 *   1D  Conflicts          1J  All repositories
 *   1E  Rebase             1K  Settings
 *   1F  Branches           1L  Clone modal
 *
 * 1M is the Reflog (FEAT-050) and 1N is Tags (FEAT-051): the first screens that
 * were not in the design handoff. Both came out of the GitKraken gap analysis,
 * and they are numbered after the handoff's run rather than inserted into it so
 * that a code still says where a screen came from.
 *
 * 1P is Badges (FEAT-072), and it came from neither: it is the first screen
 * that is not about the state of a repository but about what has been done in
 * one, by whom.
 *
 * 1Q is the Farm (FEAT-073), and it is the first screen about work that has not
 * happened yet: a goal, the tasks it was broken into, and the agents working
 * them. It takes the top slot: this is the gateway to a Git-managed agent farm,
 * and the screen a person supervising one is in all day belongs where the eye
 * lands rather than eleven rows down among the screens they visit to look
 * something up. The Graph keeps `/` — it is still what the window opens on —
 * and follows immediately, because what the farm produces is read there.
 *
 * 1R is Review (FEAT-087): a place to read a pull request, beside Pull
 * requests, which stays the place to browse, create and merge them.
 *
 * 1S is Merger (FEAT-100): any two branches, and what merging them would do
 * before anything is written. Conflicts (1D) stays for the operations git
 * stopped on by itself.
 */
export type ScreenCode =
	| '1A'
	| '1B'
	| '1C'
	| '1D'
	| '1E'
	| '1F'
	| '1G'
	| '1H'
	| '1I'
	| '1J'
	| '1K'
	| '1L'
	| '1M'
	| '1N'
	| '1O'
	| '1P'
	| '1Q'
	| '1R'
	| '1S';

/**
 * What kind of place a destination is (TASK-041).
 *
 * Fourteen rows, one divider, and identical treatment for all of them: that is
 * a list, not a structure, and it made three quite different things look like
 * one. Supervising a farm of agents, doing routine git work, and opening a tool
 * you reach for once a fortnight are not the same activity, and the eye had no
 * way to tell them apart without reading every label.
 *
 * Four groups, in the order a day goes:
 *
 * - `farm` — the gateway this application is for. One row, at the top, with
 *   nothing else in its group, which is what keeps it prominent without making
 *   it louder than everything else.
 * - `work` — the screens somebody is in and out of all day.
 * - `tools` — the occasional ones, still about the open repository. Still in
 *   the rail, still keyboard-reachable, still in the command palette; simply
 *   no longer competing with the work.
 * - `app` — the two places that are not about the open repository at all.
 *   These already had a divider above them, which is the only structure the
 *   rail had; the heading says what the divider meant.
 *
 * The heading is what says so, and it is only drawn when the rail is expanded —
 * a collapsed rail keeps the dividers, which is the same information at the
 * width available.
 */
export type NavGroup = 'farm' | 'work' | 'tools' | 'app';

/** What each group is called in the rail. `farm` has none: it is one row. */
export const GROUP_LABELS: Record<NavGroup, string | null> = {
	farm: null,
	work: 'Repository',
	tools: 'Tools',
	app: 'Spagitty'
};

export interface NavItem {
	code: ScreenCode;
	label: string;
	href: string;
	/** Which count to show right-aligned, if any. */
	count?: CountKey;
	group: NavGroup;
	/** Render a divider above this item. */
	dividerBefore?: boolean;
	/**
	 * The screen's icon: beside the label when the rail is open, and the whole
	 * item when it is collapsed.
	 *
	 * These were Unicode glyphs, chosen because the application shipped no icon
	 * set. It ships one now — `src/lib/ui/icons.ts` — so a screen names an icon
	 * and every rail, menu and toolbar draws the same shape at the same weight.
	 */
	icon: IconName;
	/** When the row is on the rail. Absent means `always`. */
	shows?: NavShows;
	/**
	 * Other routes this row stands for (TASK-045). Tags, Stash and Reflog are
	 * views of the refs the Branches screen lists, so the Branches row is where
	 * you are on any of them.
	 */
	also?: string[];
	/** Only offered while the delight layer is on (TASK-045). */
	delight?: boolean;
	/** The version a screen arrived in, while that is still news. Settings › Rail marks it. */
	since?: string;
}

/**
 * When a destination is on the rail (TASK-045).
 *
 * - `always` — where a day is spent, and Settings.
 * - `conflicts` — only while there is something to resolve. A Conflicts row
 *   with a zero beside it says nothing, all day.
 * - `open` — only while it is the screen you are on. These are still screens,
 *   reached from the toolbar, the tab strip, a shortcut or the palette; the
 *   rail shows one while it is open so "where am I" always has an answer, and
 *   lets it go when you leave.
 */
export type NavShows = 'always' | 'conflicts' | 'open';

/** The routes Branches stands for, in the order its tabs name them. */
export const REF_SCREENS = ['/branches', '/tags', '/stash', '/reflog'] as const;

/**
 * Six rows where there were fourteen (TASK-045).
 *
 * Grouping (TASK-041) made fourteen rows readable; it did not make them fewer,
 * and half of them were somewhere a person goes to look one thing up. The order
 * of what is left is unchanged, so nothing a hand has learned has moved further
 * than the rows that left from above it.
 */
export const NAV_ITEMS: NavItem[] = [
	{ code: '1Q', label: 'Farm', href: '/farm', group: 'farm', icon: 'farm' },
	{ code: '1A', label: 'Graph', href: '/', count: 'commits', group: 'work', icon: 'graph' },
	{
		code: '1C',
		label: 'Working copy',
		href: '/changes',
		count: 'working',
		group: 'work',
		icon: 'edit'
	},
	{
		code: '1D',
		label: 'Conflicts',
		href: '/conflicts',
		count: 'conflicts',
		group: 'work',
		icon: 'conflict',
		shows: 'conflicts'
	},
	// FEAT-100. Merging two branches, seen first. Always on the rail: it is
	// where a merge starts, not something that happens to you.
	{ code: '1S', label: 'Merger', href: '/merge', group: 'work', icon: 'merge' },
	{
		code: '1F',
		label: 'Branches',
		href: '/branches',
		count: 'branches',
		group: 'work',
		icon: 'branch'
	},
	{ code: '1H', label: 'Pull requests', href: '/requests', group: 'work', icon: 'request' },
	// FEAT-087. Reading a pull request, after browsing them.
	{ code: '1R', label: 'Review', href: '/review', group: 'work', icon: 'review' },
	// TASK-056: the author wanted these back where they can be seen, so the
	// tools are always on the rail again, and Stash, Tags and Reflog are rows
	// of their own rather than tabs Branches stands for (reversing TASK-045).
	{ code: '1E', label: 'Rebase', href: '/rebase', group: 'tools', icon: 'rebase' },
	{ code: '1I', label: 'Log', href: '/search', group: 'tools', icon: 'search' },
	{ code: '1G', label: 'Stash', href: '/stash', group: 'tools', icon: 'stash' },
	{ code: '1N', label: 'Tags', href: '/tags', group: 'tools', icon: 'tag' },
	{ code: '1M', label: 'Reflog', href: '/reflog', group: 'tools', icon: 'history' },
	{
		code: '1P',
		label: 'Badges',
		href: '/badges',
		group: 'tools',
		icon: 'badge',
		shows: 'open',
		delight: true
	},
	{
		code: '1J',
		label: 'All repositories',
		href: '/repos',
		group: 'tools',
		icon: 'folder'
	},
	{
		code: '1K',
		label: 'Settings',
		href: '/settings',
		group: 'app',
		dividerBefore: true,
		icon: 'settings'
	}
];

/**
 * What a person chose for the rail (FEAT-134).
 *
 * 3.0 adds four rows, and TASK-056 had already put every tool back on the rail
 * at the author's request. Only the person using it knows which rows they never
 * touch, so the choice is theirs: each screen is pinned or hidden, and the rows
 * of a group can be put in another order. Stored in Spagitty's settings, never
 * in the repository.
 *
 * - `hidden` — the screens taken off the rail.
 * - `order` — every screen's code in the order chosen. Empty means the order
 *   of `NAV_ITEMS`. A row only ever moves within its group.
 * - `where` — where the hidden ones go: under the rail's More button, or
 *   nowhere but the palette, which lists every screen whatever is chosen.
 */
export interface RailSettings {
	hidden: ScreenCode[];
	order: ScreenCode[];
	where: RailWhere;
}

export type RailWhere = 'more' | 'palette';

/** Everything pinned, the `NAV_ITEMS` order, More. TASK-056's decision stands. */
export const RAIL_DEFAULT: RailSettings = { hidden: [], order: [], where: 'more' };

/**
 * What can be chosen about a row.
 *
 * - `always` — Farm and Settings. Settings has to stay reachable to undo a
 *   hide, and the farm is what this application is for.
 * - `rule` — Conflicts and Badges, which already come and go by a rule of
 *   their own. The rule is shown in place of the choice.
 * - `free` — pinned or hidden, as the person likes.
 */
export type RailChoice = 'always' | 'rule' | 'free';

export function railChoice(item: NavItem): RailChoice {
	if (item.group === 'farm' || item.group === 'app') return 'always';
	if ((item.shows ?? 'always') !== 'always') return 'rule';
	return 'free';
}

/** How Settings › Rail words the rule a row keeps. */
export const RULE_LABELS: Record<Exclude<NavShows, 'always'>, string> = {
	conflicts: 'When there are any',
	open: 'While open'
};

/**
 * Whether the person hid `item`.
 *
 * Only a `free` row can be: a stored list that names Settings — hand-edited,
 * or written by a build whose rules differed — must not take away the one way
 * back.
 */
export function isHidden(item: NavItem, rail: RailSettings): boolean {
	return railChoice(item) === 'free' && rail.hidden.includes(item.code);
}

/**
 * `items` in the order the person chose.
 *
 * The groups keep their places and each stays one run: only the rows inside a
 * group are re-sequenced. A screen `order` does not name — one a later build
 * added — goes after the row it follows in `items`, so a new screen arrives
 * where it was designed to be rather than at the end of a list written before
 * it existed.
 */
export function inOrder(items: NavItem[], order: ScreenCode[]): NavItem[] {
	if (order.length === 0) return items;

	const groups: NavGroup[] = [];
	for (const item of items) if (!groups.includes(item.group)) groups.push(item.group);

	return groups.flatMap((group) => {
		const members = items.filter((item) => item.group === group);
		const placed = members
			.filter((item) => order.includes(item.code))
			.sort((a, b) => order.indexOf(a.code) - order.indexOf(b.code));

		members.forEach((item, index) => {
			if (placed.includes(item)) return;
			const before = members
				.slice(0, index)
				.reverse()
				.find((other) => placed.includes(other));
			placed.splice(before ? placed.indexOf(before) + 1 : 0, 0, item);
		});
		return placed;
	});
}

/** The order to store: nothing at all when it is the order of `items`. */
function orderOf(arranged: NavItem[], items: NavItem[]): ScreenCode[] {
	const codes = arranged.map((item) => item.code);
	return codes.every((code, index) => code === items[index].code) ? [] : codes;
}

/**
 * The order after moving `code` by `delta` rows, within its group.
 *
 * A move that would leave the group is no move: the first row of Tools does not
 * become the last row of Repository by pressing up once more.
 */
export function moveBy(
	code: ScreenCode,
	delta: number,
	order: ScreenCode[],
	items: NavItem[] = NAV_ITEMS
): ScreenCode[] {
	const arranged = [...inOrder(items, order)];
	const from = arranged.findIndex((item) => item.code === code);
	const to = from + delta;
	if (from === -1 || to < 0 || to >= arranged.length) return order;
	if (arranged[to].group !== arranged[from].group) return order;

	const [moving] = arranged.splice(from, 1);
	arranged.splice(to, 0, moving);
	return orderOf(arranged, items);
}

/** The order after dropping `code` on `target`'s row. Another group's row refuses it. */
export function moveOnto(
	code: ScreenCode,
	target: ScreenCode,
	order: ScreenCode[],
	items: NavItem[] = NAV_ITEMS
): ScreenCode[] {
	const arranged = inOrder(items, order);
	const from = arranged.findIndex((item) => item.code === code);
	const to = arranged.findIndex((item) => item.code === target);
	if (from === -1 || to === -1) return order;
	return moveBy(code, to - from, order, items);
}

/** What decides which rows are on the rail right now. */
export interface NavContext {
	pathname: string;
	/** Paths in conflict, or `null` before anything has been counted. */
	conflicts: number | null;
	/** Whether the delight layer is on. */
	delight: boolean;
	/** What the person pinned, hid and reordered. Absent means the default. */
	rail?: RailSettings;
}

/** True when `item` is where you are on `pathname`, counting what it stands for. */
export function isItemActive(item: NavItem, pathname: string): boolean {
	return [item.href, ...(item.also ?? [])].some((href) => isActive(href, pathname));
}

/** Whether `item` is on the rail in `context`. */
export function isShown(item: NavItem, context: NavContext): boolean {
	if (item.delight && !context.delight) return false;
	// The screen you are on is on the rail, hidden or not, for as long as you
	// are on it: "where am I" always has an answer.
	if (isItemActive(item, context.pathname)) return true;
	if (context.rail && isHidden(item, context.rail)) return false;
	switch (item.shows ?? 'always') {
		case 'always':
			return true;
		case 'conflicts':
			return (context.conflicts ?? 0) > 0;
		case 'open':
			return false;
	}
}

/**
 * The rows, with the group each one starts, in one pass.
 *
 * Computed here rather than in the component so the rail renders a list and
 * `nav.test.ts` can assert the shape of the grouping without mounting
 * anything. `startsGroup` is what carries the heading and the divider — a
 * component that worked it out with an index comparison would be doing the
 * same arithmetic in a place nothing can read.
 */
export interface NavRow {
	item: NavItem;
	/** True on the first item of each group. */
	startsGroup: boolean;
	/** The heading to draw above it, if the group has one. */
	heading: string | null;
}

export function navRows(items: NavItem[] = NAV_ITEMS, context?: NavContext): NavRow[] {
	let previous: NavGroup | null = null;
	const arranged = context?.rail ? inOrder(items, context.rail.order) : items;
	const shown = context ? arranged.filter((item) => isShown(item, context)) : arranged;

	return shown.map((item) => {
		const startsGroup = item.group !== previous;
		previous = item.group;
		return {
			item,
			startsGroup,
			heading: startsGroup ? GROUP_LABELS[item.group] : null
		};
	});
}

/**
 * The screens the person hid, in rail order (FEAT-134).
 *
 * All of them, the one that is open included: a menu whose contents change
 * with where you are is one nobody can learn.
 */
export function hiddenItems(items: NavItem[], context: NavContext): NavItem[] {
	const rail = context.rail;
	if (!rail) return [];
	return inOrder(items, rail.order).filter(
		(item) => isHidden(item, rail) && (!item.delight || context.delight)
	);
}

/** One row of the rail: a screen, or the More button with what is under it. */
export type RailRow =
	| ({ kind: 'screen' } & NavRow)
	| { kind: 'more'; items: NavItem[]; startsGroup: boolean };

/**
 * The rail as it is drawn: the rows that are on it, and the More button.
 *
 * More sits at the end of Tools, and is only there when something is hidden
 * and the person asked for hidden screens to go under it. With every tool
 * hidden it is the whole of that group, so it carries the divider itself.
 */
export function railRows(items: NavItem[], context: NavContext): RailRow[] {
	const rows: RailRow[] = navRows(items, context).map((row) => ({ kind: 'screen', ...row }));
	const under = context.rail?.where === 'more' ? hiddenItems(items, context) : [];
	if (under.length === 0) return rows;

	const group = (row: RailRow) => (row.kind === 'screen' ? row.item.group : 'tools');
	const lastTool = rows.findLastIndex((row) => group(row) === 'tools');
	const app = rows.findIndex((row) => group(row) === 'app');
	const at = lastTool !== -1 ? lastTool + 1 : app !== -1 ? app : rows.length;

	rows.splice(at, 0, { kind: 'more', items: under, startsGroup: lastTool === -1 });
	return rows;
}

/** Screens that exist but are not reachable from the rail. */
export const OFF_RAIL: Record<string, { code: ScreenCode; label: string }> = {
	'/diff': { code: '1B', label: 'Diff' },
	'/history': { code: '1O', label: 'File history' }
};

/** Routes that are screens but are not reachable from the rail. */
export const DIFF_ROUTE = '/diff';

/** True when `href` is the active screen for `pathname`. */
export function isActive(href: string, pathname: string): boolean {
	if (href === '/') return pathname === '/' || pathname === '';
	return pathname === href || pathname.startsWith(href + '/');
}
