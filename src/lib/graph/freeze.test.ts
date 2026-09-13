// SPDX-License-Identifier: GPL-3.0-or-later

import { describe, expect, it } from 'vitest';
import { freezeAt, frozenLeft } from './freeze';

describe('freezeAt', () => {
	it('starts at the commit message, which is the list pane', () => {
		expect(freezeAt([{ id: 'refs' }, { id: 'graph' }, { id: 'message' }])).toBe(2);
	});

	it('includes author, date and sha with the message', () => {
		expect(
			freezeAt([
				{ id: 'refs' },
				{ id: 'graph' },
				{ id: 'message' },
				{ id: 'author' },
				{ id: 'sha' }
			])
		).toBe(2);
	});

	it('treats a missing message as nothing frozen', () => {
		expect(freezeAt([{ id: 'refs' }, { id: 'graph' }])).toBe(2);
	});
});

describe('frozenLeft', () => {
	it('sits against the graph at rest', () => {
		expect(frozenLeft(200, 400, 0, 600)).toBe(200);
	});

	it('does not follow the graph once the pane would leave the window', () => {
		// 200 of graph, 400 of list, 500 of window: 100px of overflow.
		// After panning 150px the list would be at 50, which is off the right
		// pin (500 - 400 = 100), so it stays at 100 and the graph slides under.
		expect(frozenLeft(200, 400, 150, 500)).toBe(100);
	});

	it('does not move at all while the table still fits', () => {
		expect(frozenLeft(200, 400, 80, 600)).toBe(200);
	});
});
