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
	| '1Q';

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
}

/**
 * The order is unchanged, and that is deliberate: grouping the rows must not
 * move them. Whatever somebody's hand has learned about which position is which
 * screen still holds, and the only new thing on screen is a heading saying what
 * the run of rows below it has in common.
 *
 * Rebase moves into `tools` and it is the one judgement call here. It is git
 * work, but it is not *routine* git work — it is a thing done deliberately,
 * occasionally, with the screen open in front of you, which is exactly what the
 * group is for.
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
		icon: 'conflict'
	},
	{
		code: '1F',
		label: 'Branches',
		href: '/branches',
		count: 'branches',
		group: 'work',
		icon: 'branch'
	},
	{ code: '1N', label: 'Tags', href: '/tags', count: 'tags', group: 'work', icon: 'tag' },
	{ code: '1G', label: 'Stash', href: '/stash', count: 'stashes', group: 'work', icon: 'stash' },
	{ code: '1H', label: 'Pull requests', href: '/requests', group: 'work', icon: 'request' },
	{ code: '1E', label: 'Rebase', href: '/rebase', group: 'tools', icon: 'rebase' },
	{ code: '1I', label: 'Log', href: '/search', group: 'tools', icon: 'search' },
	{ code: '1M', label: 'Reflog', href: '/reflog', group: 'tools', icon: 'history' },
	{ code: '1P', label: 'Badges', href: '/badges', group: 'tools', icon: 'badge' },
	{
		code: '1J',
		label: 'All repositories',
		href: '/repos',
		group: 'app',
		// Kept, and now redundant: the group boundary draws the same rule. It
		// stays because `nav.test.ts` has asserted this divider since FEAT-040
		// and the assertion is about a *boundary existing here*, which is still
		// true and is now true for a better reason.
		dividerBefore: true,
		icon: 'folder'
	},
	{ code: '1K', label: 'Settings', href: '/settings', group: 'app', icon: 'settings' }
];

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

export function navRows(items: NavItem[] = NAV_ITEMS): NavRow[] {
	let previous: NavGroup | null = null;

	return items.map((item) => {
		const startsGroup = item.group !== previous;
		previous = item.group;
		return {
			item,
			startsGroup,
			heading: startsGroup ? GROUP_LABELS[item.group] : null
		};
	});
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
