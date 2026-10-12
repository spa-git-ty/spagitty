// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * What the branch menu lists, and how a typed word narrows it (FEAT-122).
 *
 * Kept apart from the component because the rules are the part worth
 * testing: which names a query finds, which comes first, where the match is
 * drawn, and how many rows are put on screen for a repository with a
 * thousand branches.
 */

export type BranchGroup = 'local' | 'remote' | 'tag';

/** One name the menu offers. */
export interface BranchOption {
	name: string;
	group: BranchGroup;
	/**
	 * Why it cannot be picked, in a few words. Listed anyway, with the reason:
	 * a list whose contents change with state is one nobody can learn.
	 */
	reason?: string | null;
}

export const GROUP_HEADINGS: Record<BranchGroup, string> = {
	local: 'Branches',
	remote: 'Remote branches',
	tag: 'Tags'
};

const GROUP_ORDER: readonly BranchGroup[] = ['local', 'remote', 'tag'];

/**
 * Rows drawn per group. A menu is not a table: past this many, the rest are
 * found by typing, and the group says how many are waiting.
 */
export const ROWS_PER_GROUP = 100;
/** With nothing typed, tags are the newest few: Tags lists them all. */
export const TAGS_UNSEARCHED = 30;

/** The words of a query, lowered. Every one has to be in the name. */
export function words(query: string): string[] {
	return query.toLowerCase().split(/\s+/).filter(Boolean);
}

/**
 * How well a name answers the first word: the name itself, then a name that
 * starts with it, then a part of the name that does (`tab` in
 * `feat/tab-drag`), then anywhere. Null when a word is missing.
 */
export function rank(name: string, query: readonly string[]): number | null {
	if (query.length === 0) return 0;
	const lower = name.toLowerCase();
	if (!query.every((word) => lower.includes(word))) return null;

	const first = query[0];
	if (lower === first) return 0;
	if (lower.startsWith(first)) return 1;
	if (lower.split(/[/\-_.]/).some((part) => part.startsWith(first))) return 2;
	return 3;
}

/** The options a query finds, best first, otherwise in the order given. */
export function search(options: readonly BranchOption[], query: string): BranchOption[] {
	const asked = words(query);
	if (asked.length === 0) return [...options];

	return options
		.map((option, index) => ({ option, index, rank: rank(option.name, asked) }))
		.filter((row): row is { option: BranchOption; index: number; rank: number } => row.rank !== null)
		.sort((x, y) => x.rank - y.rank || x.index - y.index)
		.map((row) => row.option);
}

export interface BranchSection {
	group: BranchGroup;
	heading: string;
	/** The rows drawn. */
	rows: BranchOption[];
	/** How many the query found in this group, drawn or not. */
	found: number;
}

/**
 * The menu's contents: the groups that have something, in a fixed order,
 * each cut to what a menu can hold.
 */
export function sections(options: readonly BranchOption[], query: string): BranchSection[] {
	const found = search(options, query);
	const searching = words(query).length > 0;
	const list: BranchSection[] = [];

	for (const group of GROUP_ORDER) {
		const rows = found.filter((option) => option.group === group);
		if (rows.length === 0) continue;
		const cap = group === 'tag' && !searching ? TAGS_UNSEARCHED : ROWS_PER_GROUP;
		list.push({
			group,
			heading: GROUP_HEADINGS[group],
			rows: rows.slice(0, cap),
			found: rows.length
		});
	}
	return list;
}

export interface Mark {
	text: string;
	/** Part of what was typed. */
	hit: boolean;
}

/** `text` cut into the stretches a query matched and the ones it did not. */
export function marks(text: string, query: string): Mark[] {
	const asked = words(query);
	if (asked.length === 0 || text === '') return text === '' ? [] : [{ text, hit: false }];

	const lower = text.toLowerCase();
	const hit = new Array<boolean>(text.length).fill(false);
	for (const word of asked) {
		let at = lower.indexOf(word);
		while (at !== -1) {
			for (let i = at; i < at + word.length; i += 1) hit[i] = true;
			at = lower.indexOf(word, at + word.length);
		}
	}

	const list: Mark[] = [];
	for (let i = 0; i < text.length; i += 1) {
		const last = list[list.length - 1];
		if (last && last.hit === hit[i]) last.text += text[i];
		else list.push({ text: text[i], hit: hit[i] });
	}
	return list;
}

/**
 * A name as the folder it is filed under and the part that tells it from its
 * neighbours: `feature/FEAT-121-` reads the same down a whole column, and
 * what follows is what the eye is looking for.
 */
export function split(name: string): { dir: string; leaf: string } {
	const at = name.lastIndexOf('/');
	if (at === -1 || at === name.length - 1) return { dir: '', leaf: name };
	return { dir: name.slice(0, at + 1), leaf: name.slice(at + 1) };
}

/**
 * A name ready to draw: its folder and its own part, each cut where the
 * query matched. Matched over the whole name first, so `feat/tab` is marked
 * across the slash.
 */
export function drawn(name: string, query: string): { dir: Mark[]; leaf: Mark[] } {
	const boundary = split(name).dir.length;
	const dir: Mark[] = [];
	const leaf: Mark[] = [];
	let at = 0;
	for (const mark of marks(name, query)) {
		const end = at + mark.text.length;
		if (end <= boundary) dir.push(mark);
		else if (at >= boundary) leaf.push(mark);
		else {
			dir.push({ text: mark.text.slice(0, boundary - at), hit: mark.hit });
			leaf.push({ text: mark.text.slice(boundary - at), hit: mark.hit });
		}
		at = end;
	}
	return { dir, leaf };
}
