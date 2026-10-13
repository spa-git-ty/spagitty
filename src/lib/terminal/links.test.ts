// SPDX-License-Identifier: GPL-3.0-or-later

import { describe, expect, it } from 'vitest';
import { linksIn, pathsIn, wordsIn, type Known } from './links';

const words = (line: string) => wordsIn(line).map((word) => word.text);
const paths = (line: string) => pathsIn(line).map((place) => [place.path, place.line, place.col]);

function known(commits: Record<string, string> = {}, files: string[] = []): Known {
	return { commits: new Map(Object.entries(commits)), paths: new Set(files) };
}

describe('hex words that may be commits', () => {
	it('finds what `git log --oneline` prints, and where on the line', () => {
		const found = wordsIn('a3f9c1e Fix the lane order');

		expect(found).toEqual([{ text: 'a3f9c1e', start: 0, end: 7 }]);
	});

	it('takes seven to forty hex characters and nothing shorter or longer', () => {
		const full = 'a3f9c1e0'.repeat(5);

		expect(words('abc123 is six')).toEqual([]);
		expect(words(`commit ${full}`)).toEqual([full]);
		expect(words(`not ${full}a`)).toEqual([]);
	});

	it('takes a word of its own, not part of a longer word, a name or a path', () => {
		expect(words('xa3f9c1e a3f9c1ex a3f9c1e_ _a3f9c1e')).toEqual([]);
		expect(words('target/a3f9c1e obj-a3f9c1e a3f9c1e.txt.a3f9c1e')).toEqual(['a3f9c1e']);
		expect(words('(a3f9c1e) [9d02b7f], 4b1e0aa.')).toEqual(['a3f9c1e', '9d02b7f', '4b1e0aa']);
	});

	it('leaves a number alone, and capitals, which git does not print', () => {
		expect(words('1234567 took 20261013 ms')).toEqual([]);
		expect(words('A3F9C1E')).toEqual([]);
	});

	/** `decade1` is a word and a hex word both: only the repository can say. */
	it('offers a word that only looks like a commit, to be checked', () => {
		expect(words('the decade1 problem')).toEqual(['decade1']);
	});
});

describe('places in a file', () => {
	it('finds what a compiler prints, with and without a column', () => {
		expect(paths('  --> crates/spagitty-core/src/avatars.rs:214:9')).toEqual([
			['crates/spagitty-core/src/avatars.rs', 214, 9]
		]);
		expect(paths('src/lib/nav.ts:42 is where')).toEqual([['src/lib/nav.ts', 42, null]]);
	});

	it('says where on the line the whole place is', () => {
		const [place] = pathsIn('at src/a.ts:3:1!');

		expect(place.text).toBe('src/a.ts:3:1');
		expect([place.start, place.end]).toEqual([3, 15]);
	});

	it('reads `./` off the front, and keeps `../`', () => {
		expect(paths('./README.md:1')).toEqual([['README.md', 1, null]]);
		expect(paths('../other/file.rs:7')).toEqual([['../other/file.rs', 7, null]]);
	});

	it('takes a file in the root by its dot, and a path by its slash', () => {
		expect(paths('Cargo.toml:12')).toEqual([['Cargo.toml', 12, null]]);
		expect(paths('tools/run:3')).toEqual([['tools/run', 3, null]]);
	});

	it('is not fooled by a label, a clock or an address', () => {
		expect(paths('error:12 warning:3')).toEqual([]);
		expect(paths('at 10:41:07 it stopped')).toEqual([]);
		expect(paths('http://localhost:1420')).toEqual([]);
	});

	/** `@` is in the names of real folders, so this is offered, and the repository says no. */
	it('offers what only looks like a place, to be checked', () => {
		expect(paths('ada@example.com:22')).toEqual([['ada@example.com', 22, null]]);
		expect(linksIn('ada@example.com:22', known())).toEqual([]);
	});

	it('finds each of several on a line', () => {
		expect(paths('src/a.ts:1 and src/b.ts:2:3')).toEqual([
			['src/a.ts', 1, null],
			['src/b.ts', 2, 3]
		]);
	});
});

describe('the links on a line', () => {
	const sha = 'a3f9c1e' + '0'.repeat(33);

	it('are only what the repository confirmed', () => {
		const line = 'a3f9c1e decade1 src/a.ts:3 src/gone.ts:9';

		expect(linksIn(line, known())).toEqual([]);
		expect(linksIn(line, known({ a3f9c1e: sha }, ['src/a.ts']))).toEqual([
			{ kind: 'commit', start: 0, end: 7, text: 'a3f9c1e', commit: sha },
			{ kind: 'file', start: 16, end: 26, text: 'src/a.ts:3', path: 'src/a.ts', line: 3 }
		]);
	});

	it('opens a commit by its full id, whatever was printed', () => {
		const [link] = linksIn('commit a3f9c1e', known({ a3f9c1e: sha }));

		expect(link).toMatchObject({ kind: 'commit', text: 'a3f9c1e', commit: sha });
	});

	/** A folder named for a commit is part of the path, not a commit of its own. */
	it('gives a word inside a path to the path', () => {
		const line = 'deadbee/notes.md:4';
		const all = known({ deadbee: 'deadbee' + '0'.repeat(33) }, ['deadbee/notes.md']);

		expect(linksIn(line, all)).toEqual([
			{ kind: 'file', start: 0, end: 18, text: line, path: 'deadbee/notes.md', line: 4 }
		]);
	});

	it('are in the order they stand on the line', () => {
		const line = 'src/a.ts:1 a3f9c1e src/b.ts:2';
		const all = known({ a3f9c1e: sha }, ['src/a.ts', 'src/b.ts']);

		expect(linksIn(line, all).map((link) => link.text)).toEqual(['src/a.ts:1', 'a3f9c1e', 'src/b.ts:2']);
	});
});
