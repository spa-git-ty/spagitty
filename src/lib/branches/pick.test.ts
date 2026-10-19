// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * Which branches a typed word finds, and in what order (FEAT-122).
 */

import { describe, expect, it } from 'vitest';
import {
	drawn,
	marks,
	optionsOf,
	rank,
	ROWS_PER_GROUP,
	search,
	sections,
	split,
	TAGS_UNSEARCHED,
	words,
	type BranchOption
} from './pick';
import type { BranchRow } from '../types';

const local = (name: string): BranchOption => ({ name, group: 'local' });
const remote = (name: string): BranchOption => ({ name, group: 'remote' });
const tag = (name: string): BranchOption => ({ name, group: 'tag' });

const names = (options: BranchOption[]) => options.map((option) => option.name);

describe('words', () => {
	it('lowers and splits on spaces, and drops the empty ones', () => {
		expect(words('  Feat   TAB ')).toEqual(['feat', 'tab']);
		expect(words('   ')).toEqual([]);
	});
});

describe('rank', () => {
	it('puts the name itself first, then a prefix, then a part, then anywhere', () => {
		expect(rank('main', ['main'])).toBe(0);
		expect(rank('maintenance', ['main'])).toBe(1);
		expect(rank('feat/main-menu', ['main'])).toBe(2);
		expect(rank('domain', ['main'])).toBe(3);
	});

	it('is null when a word is missing', () => {
		expect(rank('feat/tab-drag', ['tab', 'zzz'])).toBeNull();
	});

	it('does not care about case', () => {
		expect(rank('feature/FEAT-121-say', ['feat-121'])).toBe(3);
		expect(rank('Main', words('MAIN'))).toBe(0);
	});

	it('treats every separator a branch name uses as the start of a part', () => {
		for (const name of ['fix/tab', 'fix-tab', 'fix_tab', 'fix.tab']) {
			expect(rank(name, ['tab']), name).toBe(2);
		}
	});
});

describe('search', () => {
	const options = [local('domain'), local('feat/main-menu'), local('maintenance'), local('main'), local('old')];

	it('keeps the order given when nothing is typed', () => {
		expect(names(search(options, ''))).toEqual(names(options));
		expect(names(search(options, '   '))).toEqual(names(options));
	});

	it('puts the best answer first and keeps the order given among equals', () => {
		expect(names(search(options, 'main'))).toEqual(['main', 'maintenance', 'feat/main-menu', 'domain']);
	});

	it('needs every word, in any order', () => {
		const found = search([local('feat/tab-drag'), local('feat/tab-close'), local('fix/drag')], 'drag feat');
		expect(names(found)).toEqual(['feat/tab-drag']);
	});

	it('finds a branch by the middle of its name', () => {
		expect(names(search([local('feature/FEAT-121-say-what-is-running')], '121'))).toHaveLength(1);
	});
});

describe('sections', () => {
	it('groups in a fixed order and leaves out a group with nothing in it', () => {
		const list = sections([tag('v1.0.0'), remote('origin/main'), local('main')], '');

		expect(list.map((section) => section.heading)).toEqual(['Branches', 'Remote branches', 'Tags']);
		expect(sections([local('main')], '').map((section) => section.group)).toEqual(['local']);
	});

	it('leaves out the groups a query finds nothing in', () => {
		const list = sections([local('main'), remote('origin/main'), tag('v1.0.0')], 'v1');

		expect(list.map((section) => section.group)).toEqual(['tag']);
	});

	it('draws only so many rows of a group, and says how many it found', () => {
		const many = Array.from({ length: ROWS_PER_GROUP + 40 }, (_, i) => local(`topic/${i}`));

		const [section] = sections(many, '');

		expect(section.rows).toHaveLength(ROWS_PER_GROUP);
		expect(section.found).toBe(ROWS_PER_GROUP + 40);
	});

	it('offers the newest few tags until something is typed, then every one that matches', () => {
		const tags = Array.from({ length: TAGS_UNSEARCHED + 20 }, (_, i) => tag(`v1.${i}.0`));

		expect(sections(tags, '')[0].rows).toHaveLength(TAGS_UNSEARCHED);
		// Past the unsearched cut: found by typing, which is what the cut promises.
		expect(names(sections(tags, `v1.${TAGS_UNSEARCHED + 5}.0`)[0].rows)).toEqual([
			`v1.${TAGS_UNSEARCHED + 5}.0`
		]);
	});
});

describe('marks', () => {
	it('is one unmarked stretch when nothing is typed', () => {
		expect(marks('main', '')).toEqual([{ text: 'main', hit: false }]);
	});

	it('cuts the name where the query matched', () => {
		expect(marks('feat/tab-drag', 'tab')).toEqual([
			{ text: 'feat/', hit: false },
			{ text: 'tab', hit: true },
			{ text: '-drag', hit: false }
		]);
	});

	it('marks every word, and every place a word is', () => {
		expect(marks('tab-tab-drag', 'tab drag').filter((mark) => mark.hit).map((mark) => mark.text)).toEqual([
			'tab',
			'tab',
			'drag'
		]);
	});

	it('keeps the case the name has', () => {
		expect(marks('FEAT-121', 'feat')[0]).toEqual({ text: 'FEAT', hit: true });
	});
});

describe('split and drawn', () => {
	it('separates the folder from the part that tells branches apart', () => {
		expect(split('feature/FEAT-121-say')).toEqual({ dir: 'feature/', leaf: 'FEAT-121-say' });
		expect(split('origin/feat/tabs')).toEqual({ dir: 'origin/feat/', leaf: 'tabs' });
		expect(split('main')).toEqual({ dir: '', leaf: 'main' });
	});

	it('marks a match that runs across the slash on both sides of it', () => {
		const { dir, leaf } = drawn('feat/tab-drag', 'at/ta');

		expect(dir).toEqual([
			{ text: 'fe', hit: false },
			{ text: 'at/', hit: true }
		]);
		expect(leaf).toEqual([
			{ text: 'ta', hit: true },
			{ text: 'b-drag', hit: false }
		]);
	});

	it('puts every character of the name in one part or the other, once', () => {
		for (const [name, query] of [
			['feature/FEAT-121-say', '121'],
			['main', 'ai'],
			['origin/main', 'origin'],
			['a/b/c', 'b']
		]) {
			const { dir, leaf } = drawn(name, query);
			expect([...dir, ...leaf].map((mark) => mark.text).join(''), name).toBe(name);
		}
	});
});

describe('optionsOf (TASK-069)', () => {
	it('lists each row under the group of its kind', () => {
		const row = (name: string, kind: BranchRow['kind']) => ({ name, kind }) as BranchRow;
		expect(optionsOf([row('main', 'branch'), row('origin/main', 'remote'), row('v1', 'tag')])).toEqual([
			local('main'),
			remote('origin/main'),
			tag('v1')
		]);
	});
});
