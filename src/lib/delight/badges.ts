// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * The badge catalogue (FEAT-072).
 *
 * Data, and nothing else. What earns a badge lives in `engine.ts`, what a badge
 * looks like lives in `BadgeChip.svelte`, and what noise it makes lives in
 * `sound.ts` — this file only says which badges exist and what they mean, so
 * adding one is a row here plus a rule there rather than an edit in five
 * places.
 *
 * # Two axes, not one
 *
 * `category` says what kind of accomplishment it is; `rarity` says how hard it
 * was. They are separate because a `chaos` badge can be common and a `git`
 * badge can be legendary, and folding them into one field means every new badge
 * has to lie about one of the two.
 *
 * `secret` is a flag rather than a rarity for the same reason: a secret badge
 * still has a difficulty, and the badge screen needs to know both — one decides
 * the treatment it is drawn with, the other decides whether it is drawn at all
 * before it is earned.
 *
 * # The rule the whole file is written against
 *
 * Nothing here rewards usage. There is no badge for opening Spagitty, for
 * committing often, or for having the window in front of you — every row below
 * is earned by doing something skilful, recovering from something bad, or doing
 * something funny enough that a developer would tell somebody about it.
 */

/** What kind of accomplishment a badge is for. Decides which section it sits in. */
export type Category = 'git' | 'engineering' | 'agent' | 'recovery' | 'chaos' | 'legendary';

/** How hard it was. Decides the treatment, the sound, and the sort order. */
export type Rarity = 'common' | 'uncommon' | 'rare' | 'epic' | 'legendary';

export interface Badge {
	id: string;
	/** The glyph. One emoji — the badge is recognised by it before it is read. */
	emoji: string;
	name: string;
	/** The line under the name in the reward moment. One sentence, present tense. */
	line: string;
	category: Category;
	rarity: Rarity;
	/**
	 * Hidden from the badge list until it is earned, shown as `???`.
	 *
	 * Only for badges whose discovery is the point — the ones you find out about
	 * by surviving something. A secret badge with a criteria anybody would guess
	 * is just a badge with its label missing.
	 */
	secret?: boolean;
	/**
	 * An anti-badge. Lives under Hall of Shame, is never celebrated with a
	 * reward moment, and never counts towards a title.
	 *
	 * These acknowledge what happened rather than approve of it, and the wording
	 * has one job: to be a developer laughing at a familiar developer thing, not
	 * Spagitty judging somebody who is still learning.
	 */
	shame?: boolean;
	/** The badge this one grows out of. Draws the evolution chains. */
	after?: string;
}

/**
 * Every badge, in the order the screen shows them.
 *
 * Two emoji collisions in here are deliberate. `🍝` is both Spagitty Chef and
 * Actual Spaghetti, because that *is* the joke — the same plate, cooked or raw —
 * and the two never appear in one grid: shame badges are drawn in their own
 * section. Everything else is unique, which is why Gatekeeper is `🚧` rather
 * than the shield the design document gave it: Regression Slayer already has
 * the shield, and two shields in one actor's row is a badge nobody can name at
 * a glance.
 */
export const BADGES: Badge[] = [
	// --- Git skill ---------------------------------------------------------
	{
		id: 'cherry-picker',
		emoji: '🍒',
		name: 'Cherry Picker',
		line: 'Taken from there. Placed here. Nothing spilled.',
		category: 'git',
		rarity: 'common'
	},
	{
		id: 'conflict-rookie',
		emoji: '🪡',
		name: 'Conflict Rookie',
		line: 'Your first one. It will not be your last.',
		category: 'git',
		rarity: 'common'
	},
	{
		id: 'conflict-tamer',
		emoji: '🪢',
		name: 'Conflict Tamer',
		line: 'Both sides wanted the same lines. You decided.',
		category: 'git',
		rarity: 'uncommon',
		after: 'conflict-rookie'
	},
	{
		id: 'conflict-samurai',
		emoji: '🗡',
		name: 'Conflict Samurai',
		line: 'Conflicts stopped being events and became weather.',
		category: 'git',
		rarity: 'rare',
		after: 'conflict-tamer'
	},
	{
		id: 'rebase-ronin',
		emoji: '⚔️',
		name: 'Rebase Ronin',
		line: 'Many rebases walked in. Every commit walked out.',
		category: 'git',
		rarity: 'rare'
	},
	{
		id: 'octopus',
		emoji: '🐙',
		name: 'Octopus',
		line: 'Fifteen branches folded in. Not one of them lost.',
		category: 'git',
		rarity: 'rare'
	},
	{
		id: 'git-sensei',
		emoji: '🥋',
		name: 'Git Sensei',
		line: 'History bends to your will.',
		category: 'git',
		rarity: 'epic',
		after: 'conflict-samurai'
	},

	// --- Engineering quality -----------------------------------------------
	{
		id: 'cook',
		emoji: '🥄',
		name: 'Cook',
		line: 'Something clean came out of the kitchen.',
		category: 'engineering',
		rarity: 'common'
	},
	{
		id: 'zero-noise',
		emoji: '🧊',
		name: 'Zero Noise',
		line: 'You changed what you meant to change. Nothing else.',
		category: 'engineering',
		rarity: 'common'
	},
	{
		id: 'test-goblin',
		emoji: '🧪',
		name: 'Test Goblin',
		line: 'The tests arrived with the code, not after it.',
		category: 'engineering',
		rarity: 'uncommon'
	},
	{
		id: 'clean-freak',
		emoji: '🧹',
		name: 'Clean Freak',
		line: 'Eight in a row, and not one of them needed tidying up.',
		category: 'engineering',
		rarity: 'uncommon'
	},
	{
		id: 'spagitty-chef',
		emoji: '🍝',
		name: 'Spagitty Chef',
		line: 'Tangle in. Plate out.',
		category: 'engineering',
		rarity: 'uncommon',
		after: 'cook'
	},
	{
		id: 'surgical-strike',
		emoji: '🎯',
		name: 'Surgical Strike',
		line: 'One file. Four lines. Bug gone.',
		category: 'engineering',
		rarity: 'rare'
	},
	{
		id: 'architect',
		emoji: '📐',
		name: 'Architect',
		line: 'Everything moved. Nothing broke.',
		category: 'engineering',
		rarity: 'rare'
	},
	{
		id: 'regression-slayer',
		emoji: '🛡',
		name: 'Regression Slayer',
		line: 'It would have shipped. You caught it first.',
		category: 'engineering',
		rarity: 'rare'
	},

	// --- Agent performance --------------------------------------------------
	{
		id: 'first-try',
		emoji: '⚡',
		name: 'First Try',
		line: 'Clean implementation. Zero corrections.',
		category: 'agent',
		rarity: 'rare'
	},
	{
		id: 'eagle-eye',
		emoji: '👁',
		name: 'Eagle Eye',
		line: 'Everyone else read that file. You read it properly.',
		category: 'agent',
		rarity: 'rare'
	},
	{
		id: 'gatekeeper',
		emoji: '🚧',
		name: 'Gatekeeper',
		line: 'Five bad changes stopped at your door.',
		category: 'agent',
		rarity: 'rare'
	},
	{
		id: 'big-brain',
		emoji: '🧠',
		name: 'Big Brain',
		line: 'A hard one, solved without being asked twice.',
		category: 'agent',
		rarity: 'epic'
	},
	{
		id: 'perfect-handoff',
		emoji: '🤝',
		name: 'Perfect Handoff',
		line: 'Planner to implementer to reviewer, and nobody had to ask a human.',
		category: 'agent',
		rarity: 'epic'
	},
	{
		id: 'certified-chef',
		emoji: '👨‍🍳',
		name: 'Certified Chef',
		line: 'Four kinds of good engineering, from one pair of hands.',
		category: 'agent',
		rarity: 'epic',
		after: 'spagitty-chef'
	},

	// --- Recovery -----------------------------------------------------------
	{
		id: 'rebase-survivor',
		emoji: '🔥',
		name: 'Rebase Survivor',
		line: 'It went wrong in the middle, and you still got everything back.',
		category: 'recovery',
		rarity: 'rare'
	},
	{
		id: 'detached-head-survivor',
		emoji: '💀',
		name: 'Detached HEAD Survivor',
		line: 'You were on no branch at all, and you got out with the work.',
		category: 'recovery',
		rarity: 'uncommon',
		secret: true
	},
	{
		id: 'reflog-wizard',
		emoji: '🧙',
		name: 'Reflog Wizard',
		line: 'It was gone. You went and got it.',
		category: 'recovery',
		rarity: 'epic',
		secret: true
	},

	// --- Hall of shame ------------------------------------------------------
	{
		id: 'main-character',
		emoji: '🧨',
		name: 'Main Character',
		line: "You committed straight to the default branch. We're not angry.",
		category: 'chaos',
		rarity: 'common',
		shame: true
	},
	{
		id: 'rip-branch',
		emoji: '🪦',
		name: 'RIP Branch',
		line: 'Three branches deleted. They knew things.',
		category: 'chaos',
		rarity: 'common',
		shame: true
	},
	{
		id: 'force-push-and-pray',
		emoji: '🙏',
		name: 'Force Push And Pray',
		line: 'We saw that.',
		category: 'chaos',
		rarity: 'uncommon',
		shame: true
	},
	{
		id: 'works-on-my-machine',
		emoji: '🗿',
		name: 'Works On My Machine',
		line: 'Green here. Red everywhere else.',
		category: 'chaos',
		rarity: 'uncommon',
		shame: true
	},
	{
		id: 'actual-spaghetti',
		emoji: '🍝',
		name: 'Actual Spaghetti',
		line: 'The pasta is getting out of control.',
		category: 'chaos',
		rarity: 'uncommon',
		shame: true
	},
	{
		id: 'this-is-fine',
		emoji: '🫠',
		name: 'This Is Fine',
		line: 'Ten files conflicted at once. You sat down anyway.',
		category: 'chaos',
		rarity: 'rare',
		shame: true
	},
	{
		id: 'what-branch-am-i-on',
		emoji: '🧭',
		name: 'What Branch Am I On?',
		line: 'Six checkouts in three minutes. Debugging is going great.',
		category: 'chaos',
		rarity: 'uncommon',
		secret: true,
		shame: true
	},

	// --- Legendary ----------------------------------------------------------
	{
		id: 'git-lord',
		emoji: '👑',
		name: 'Git Lord',
		line: 'There is nothing left in this repository that frightens you.',
		category: 'legendary',
		rarity: 'legendary',
		after: 'git-sensei'
	},
	{
		id: 'history-bender',
		emoji: '⏳',
		name: 'History Bender',
		line: 'Two hundred commits replayed, and every one of them survived.',
		category: 'legendary',
		rarity: 'legendary',
		secret: true
	},
	{
		id: 'michelin-commit',
		emoji: '⭐',
		name: 'Michelin Commit',
		line: 'Ten straight, first time, no notes.',
		category: 'legendary',
		rarity: 'legendary',
		after: 'certified-chef'
	},
	{
		id: 'pasta-master',
		emoji: '🍜',
		name: 'Pasta Master',
		line: 'Every badge worth having. In one repository.',
		category: 'legendary',
		rarity: 'legendary'
	}
];

const BY_ID = new Map(BADGES.map((badge) => [badge.id, badge]));

/** The badge, or null for an id from a newer build than this one. */
export function badge(id: string): Badge | null {
	return BY_ID.get(id) ?? null;
}

/** Badges that can be equipped as a title: earned, and not a shame badge. */
export function titleable(earned: string[]): Badge[] {
	return earned
		.map((id) => BY_ID.get(id))
		.filter((found): found is Badge => found !== undefined && !found.shame);
}

/** The order rarity is sorted and weighted in, low to high. */
export const RARITY_ORDER: Rarity[] = ['common', 'uncommon', 'rare', 'epic', 'legendary'];

/** How much a badge is worth when one has to be picked out of many. */
export function weight(rarity: Rarity): number {
	return RARITY_ORDER.indexOf(rarity);
}

/**
 * The badges counted by the `n / ??` on the badge screen.
 *
 * Secret badges are excluded from the denominator on purpose — the count is
 * meant to say "there are more of these than you have found", and a total that
 * gives away exactly how many are left removes the reason the secrets exist.
 */
export function knownTotal(): number {
	return BADGES.filter((found) => !found.secret).length;
}

/** The section headings, in the order the badge screen draws them. */
export const CATEGORY_LABELS: { id: Category; label: string }[] = [
	{ id: 'git', label: 'Git skill' },
	{ id: 'engineering', label: 'Engineering quality' },
	{ id: 'agent', label: 'Agent performance' },
	{ id: 'recovery', label: 'Recovery' },
	{ id: 'legendary', label: 'Legendary' },
	{ id: 'chaos', label: 'Hall of shame' }
];
