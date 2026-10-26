// SPDX-License-Identifier: GPL-3.0-or-later

import { describe, expect, it } from 'vitest';
import { complete, isBuiltin, namesIn, segments, suggest, typingAt } from './vars';

describe('variables in a call', () => {
	it('cuts text into plain runs and variables, with where each starts', () => {
		expect(segments('{{baseUrl}}/auth/{{ id }}?x=1')).toEqual([
			{ text: '{{baseUrl}}', name: 'baseUrl', start: 0 },
			{ text: '/auth/', start: 11 },
			{ text: '{{ id }}', name: 'id', start: 17 },
			{ text: '?x=1', start: 25 }
		]);
		expect(segments('')).toEqual([]);
		expect(segments('plain')).toEqual([{ text: 'plain', start: 0 }]);
	});

	it('leaves braces that do not make a name as text', () => {
		const text = '{{}} {{a\nb}} {{ {{token}} {{open';
		const names = segments(text).filter((segment) => segment.name);
		expect(names.map((segment) => segment.name)).toEqual(['token']);
		expect(segments(text).map((segment) => segment.text).join('')).toBe(text);
	});

	it('lists each name once', () => {
		expect(namesIn('{{a}} {{b}} {{a}}')).toEqual(['a', 'b']);
		expect(isBuiltin('$uuid')).toBe(true);
		expect(isBuiltin('uuid')).toBe(false);
	});

	it('finds a `{{` being typed before the caret', () => {
		expect(typingAt('{{base', 6)).toEqual({ from: 0, query: 'base' });
		expect(typingAt('x {{ ', 5)).toEqual({ from: 2, query: '' });
		expect(typingAt('{{done}} ', 9)).toBeNull();
		expect(typingAt('{{base', 3)).toEqual({ from: 0, query: 'b' });
	});

	it('offers names that start with the query before names that contain it', () => {
		expect(suggest('id', ['userId', 'idToken', 'id'])).toEqual(['id', 'idToken', '$uuid', 'userId']);
		expect(suggest('', ['b', 'a']).slice(0, 2)).toEqual(['$isoNow', '$randomInt']);
		expect(suggest('UUID', [])).toEqual(['$uuid']);
	});

	it('completes the name and puts the caret after the braces', () => {
		const text = '{{ba/x';
		expect(complete(text, 4, { from: 0, query: 'ba' }, 'baseUrl')).toEqual({ text: '{{baseUrl}}/x', caret: 11 });
		// A closing pair already there is not doubled.
		expect(complete('{{ba}}/x', 4, { from: 0, query: 'ba' }, 'baseUrl')).toEqual({
			text: '{{baseUrl}}/x',
			caret: 11
		});
	});
});
