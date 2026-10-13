// SPDX-License-Identifier: GPL-3.0-or-later

import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';
import { CLASSES, linesOf, pieces, tokensOf } from './colour';

describe('the token classes', () => {
	/**
	 * A class crosses the wire as a number, so the two lists are one list
	 * written twice. A class added on one side only would colour every token
	 * after it as its neighbour.
	 */
	it('are the ones the Rust highlighter numbers, in the same order', () => {
		const rust = readFileSync('crates/spagitty-core/src/highlight.rs', 'utf8');
		const block = /pub const CLASSES: \[&str; (\d+)\] = \[([^\]]+)\];/.exec(rust);
		if (!block) throw new Error('CLASSES is not in highlight.rs');
		const names = [...block[2].matchAll(/"([a-z]+)"/g)].map((match) => match[1]);

		expect(Number(block[1])).toBe(CLASSES.length);
		expect(names).toEqual([...CLASSES]);
	});

	it('are all classes the stylesheet colours', () => {
		const css = readFileSync('src/app.css', 'utf8');
		for (const name of CLASSES) {
			if (name === 'plain') continue;
			expect(css, `.tok-${name}`).toContain(`.tok-${name}`);
		}
	});
});

describe('tokensOf', () => {
	it('cuts a line by its runs', () => {
		// `let x = 1;` — keyword, plain, operator, plain, number, punctuation.
		const runs = [1, 3, 0, 3, 7, 1, 0, 1, 3, 1, 8, 1];

		expect(tokensOf('let x = 1;', runs)).toEqual([
			{ type: 'keyword', text: 'let' },
			{ type: 'plain', text: ' x ' },
			{ type: 'operator', text: '=' },
			{ type: 'plain', text: ' ' },
			{ type: 'number', text: '1' },
			{ type: 'punctuation', text: ';' }
		]);
	});

	it('is the whole line, plain, until its colour has arrived', () => {
		expect(tokensOf('let x = 1;', undefined)).toEqual([{ type: 'plain', text: 'let x = 1;' }]);
		expect(tokensOf('let x = 1;', [])).toEqual([{ type: 'plain', text: 'let x = 1;' }]);
		expect(tokensOf('', undefined)).toEqual([]);
	});

	it('counts in UTF-16 units, as the runs do', () => {
		// The pasta is two units; the string run is 4: quote, pasta, quote.
		expect(tokensOf('"🍝";', [2, 4, 8, 1])).toEqual([
			{ type: 'string', text: '"🍝"' },
			{ type: 'punctuation', text: ';' }
		]);
	});

	/** A line and its colour can be a frame apart. A word must never go missing. */
	it('draws plain whatever the runs leave, and ignores what they claim past the end', () => {
		expect(tokensOf('abcdef', [1, 2])).toEqual([
			{ type: 'keyword', text: 'ab' },
			{ type: 'plain', text: 'cdef' }
		]);
		expect(tokensOf('ab', [1, 2, 2, 9])).toEqual([{ type: 'keyword', text: 'ab' }]);
		expect(tokensOf('ab', [99, 2])).toEqual([{ type: 'plain', text: 'ab' }]);
		expect(
			tokensOf('abc', [1, 9])
				.map((token) => token.text)
				.join('')
		).toBe('abc');
	});
});

describe('pieces', () => {
	const tokens = tokensOf('fn is_stale(path)', [1, 2, 0, 1, 5, 8, 8, 1, 0, 4, 8, 1]);

	it('leaves the tokens as they are with nothing found', () => {
		expect(pieces(tokens, null).every((piece) => !piece.hit)).toBe(true);
		expect(pieces(tokens, null).map((piece) => piece.text)).toEqual(tokens.map((t) => t.text));
		expect(pieces(tokens, [4, 4]).every((piece) => !piece.hit)).toBe(true);
	});

	it('marks the match and keeps every colour', () => {
		const cut = pieces(tokens, [3, 11]);

		expect(cut.filter((piece) => piece.hit).map((piece) => piece.text)).toEqual(['is_stale']);
		expect(cut.find((piece) => piece.hit)?.type).toBe('fn');
		expect(cut.map((piece) => piece.text).join('')).toBe('fn is_stale(path)');
	});

	it('cuts a token the match starts or ends inside, and spans several', () => {
		// From inside `is_stale` to inside `path`.
		const cut = pieces(tokens, [6, 14]);

		expect(cut.filter((piece) => piece.hit).map((piece) => piece.text)).toEqual(['stale', '(', 'pa']);
		expect(cut.map((piece) => piece.text).join('')).toBe('fn is_stale(path)');
		expect(cut.filter((piece) => piece.hit).map((piece) => piece.type)).toEqual([
			'fn',
			'punctuation',
			'plain'
		]);
	});
});

describe('linesOf', () => {
	it('ends the last line at a trailing newline and starts none after it', () => {
		expect(linesOf('a\nb\n')).toEqual(['a', 'b']);
		expect(linesOf('a\nb')).toEqual(['a', 'b']);
		expect(linesOf('a\n\n')).toEqual(['a', '']);
		expect(linesOf('')).toEqual([]);
		expect(linesOf('\n')).toEqual(['']);
	});

	it('takes a carriage return as part of the line ending', () => {
		expect(linesOf('a\r\nb\r\n')).toEqual(['a', 'b']);
		// One in the middle of a line is the line's own.
		expect(linesOf('a\rb\n')).toEqual(['a\rb']);
	});
});
