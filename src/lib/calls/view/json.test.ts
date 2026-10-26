// SPDX-License-Identifier: GPL-3.0-or-later

import { describe, expect, it } from 'vitest';
import { formatJson, step, type Line } from './json';

const text = (lines: Line[]) => lines.map((line) => line.map((piece) => piece.text).join(''));
const values = (lines: Line[]) =>
	lines.flatMap((line) => line.flatMap((piece) => (piece.path ? [[piece.path, piece.value]] : [])));

describe('a JSON response', () => {
	it('is drawn with two spaces, each member on its own line', () => {
		const lines = formatJson('{"a":1,"b":[true,null],"c":{},"d":[]}');
		expect(text(lines!)).toEqual([
			'{',
			'  "a": 1,',
			'  "b": [',
			'    true,',
			'    null',
			'  ],',
			'  "c": {},',
			'  "d": []',
			'}'
		]);
	});

	it('gives each value the JSONPath that finds it', () => {
		const lines = formatJson('{"data":{"accessToken":"ey.J","user":{"id":"usr_1"}},"list":[3,"x"],"a-b":false}');
		expect(values(lines!)).toEqual([
			['$.data.accessToken', 'ey.J'],
			['$.data.user.id', 'usr_1'],
			['$.list[0]', '3'],
			['$.list[1]', 'x'],
			["$['a-b']", 'false']
		]);
	});

	it('keeps what the server sent: key order, digits and escapes', () => {
		const lines = formatJson('{"b":1,"10":2,"n":12345678901234567890,"s":"a\\"b\\u00e9"}');
		expect(text(lines!)).toEqual(['{', '  "b": 1,', '  "10": 2,', '  "n": 12345678901234567890,', '  "s": "a\\"b\\u00e9"', '}']);
		expect(values(lines!)).toContainEqual(['$.s', 'a"bé']);
		expect(values(lines!)).toContainEqual(["$['10']", '2']);
	});

	it('is null for text that is not JSON', () => {
		for (const bad of ['', '{', '{"a":}', '[1,]', 'nope', '{"a":1} x', '"open']) {
			expect(formatJson(bad)).toBeNull();
		}
		expect(text(formatJson(' 42 ')!)).toEqual(['42']);
	});

	it('quotes a key that is not a plain name', () => {
		expect(step('$', 'ok')).toBe('$.ok');
		expect(step('$', 0)).toBe('$[0]');
		expect(step('$', "it's")).toBe("$['it\\'s']");
	});
});
