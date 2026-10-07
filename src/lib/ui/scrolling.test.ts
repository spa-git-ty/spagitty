// SPDX-License-Identifier: GPL-3.0-or-later

import { afterEach, describe, expect, it, vi } from 'vitest';
import { readFileSync } from 'node:fs';
import { SCROLLING_CLASS, watchScrolling } from './scrolling';

afterEach(() => {
	vi.useRealTimers();
	document.body.innerHTML = '';
});

/** BUG-054: scrollbars at rest are gone; they show while their element scrolls. */
describe('watchScrolling', () => {
	it('marks the element that scrolls, and only for a moment after it stops', () => {
		vi.useFakeTimers();
		const stop = watchScrolling(document, 500);
		const pane = document.createElement('div');
		const other = document.createElement('div');
		document.body.append(pane, other);

		pane.dispatchEvent(new Event('scroll'));
		expect(pane.classList.contains(SCROLLING_CLASS)).toBe(true);
		expect(other.classList.contains(SCROLLING_CLASS)).toBe(false);

		vi.advanceTimersByTime(400);
		pane.dispatchEvent(new Event('scroll'));
		vi.advanceTimersByTime(400);
		expect(pane.classList.contains(SCROLLING_CLASS)).toBe(true);

		vi.advanceTimersByTime(200);
		expect(pane.classList.contains(SCROLLING_CLASS)).toBe(false);
		stop();
	});

	it('lets go of everything when stopped', () => {
		const stop = watchScrolling(document, 10_000);
		const pane = document.createElement('div');
		document.body.append(pane);
		pane.dispatchEvent(new Event('scroll'));
		stop();
		expect(pane.classList.contains(SCROLLING_CLASS)).toBe(false);
	});

	it('paints a thumb only on a scrolling element or under the pointer', () => {
		const css = readFileSync('src/app.css', 'utf8');
		expect(css).toMatch(/::-webkit-scrollbar-thumb\s*{[^}]*background:\s*transparent/s);
		expect(css).toMatch(/\.is-scrolling::-webkit-scrollbar-thumb/);
		expect(css).toMatch(/::-webkit-scrollbar-thumb:hover/);
	});
});
