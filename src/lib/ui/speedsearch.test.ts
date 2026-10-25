// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * Type to search (FEAT-158).
 *
 * Two things have to be right. The matcher, because a list that hides the row
 * being looked for is worse than no search. And which keys are taken: a
 * letter typed into a field, or with Ctrl held, or into an open menu, is
 * somebody else's, and taking it breaks that somebody.
 */

import { afterEach, describe, expect, it, vi } from 'vitest';
import { click, flushSync, press, render, type Mounted } from '../../testing/mount';
import Harness from '../../testing/SpeedSearchHarness.svelte';
import { createSearch, match, resetSpeedSearch, speed } from './speedsearch.svelte';

describe('match', () => {
	it('matches everything, marking nothing, for an empty query', () => {
		expect(match('anything', '')).toEqual([]);
		expect(match('anything', '   ')).toEqual([]);
	});

	it('finds a substring whatever its case', () => {
		expect(match('feature/FEAT-158', 'feat-1')).toEqual([[8, 14]]);
		expect(match('Main', 'AI')).toEqual([[1, 3]]);
	});

	it('finds letters that start words, in order', () => {
		expect(match('feature/BUG-098', 'fb')).toEqual([
			[0, 1],
			[8, 9]
		]);
		expect(match('CommitRows', 'cr')).toEqual([
			[0, 1],
			[6, 7]
		]);
		expect(match('CommitRows', 'comro')).toEqual([
			[0, 3],
			[6, 8]
		]);
		expect(match('release-3.9.0', 'r39')).not.toBeNull();
	});

	it('does not match letters from the middle of words', () => {
		expect(match('feature/BUG-098', 'eu')).toBeNull();
		expect(match('CommitRows', 'ows x')).toBeNull();
	});

	it('needs every word of the query', () => {
		expect(match('origin/main', 'main origin')).not.toBeNull();
		expect(match('origin/main', 'main upstream')).toBeNull();
	});

	it('backs off a wrong first guess', () => {
		// `ab` then a `c` that neither follows it nor starts a word: the `b`
		// that starts a word, with its `c` after it, is the one meant.
		expect(match('ab-x a-bc', 'abc')).toEqual([
			[0, 1],
			[7, 9]
		]);
	});
});

describe('createSearch', () => {
	it('narrows a list by any of the texts each row gives', () => {
		const search = createSearch();
		const rows = [
			{ name: 'main', upstream: 'origin/main' },
			{ name: 'feature/x', upstream: null }
		];
		const by = (row: (typeof rows)[number]) => [row.name, row.upstream];
		expect(search.filter(rows, by)).toBe(rows);
		search.set('origin');
		expect(search.filter(rows, by).map((row) => row.name)).toEqual(['main']);
		search.clear();
		expect(search.active).toBe(false);
	});
});

describe('typing on a screen', () => {
	let view: Mounted | null = null;

	function mount(props: Parameters<typeof render<{ names: string[] }>>[1] & Record<string, unknown>) {
		view = render(Harness, props as { names: string[] });
		return view;
	}

	afterEach(() => {
		view?.destroy();
		view = null;
		resetSpeedSearch();
		document.body.innerHTML = '';
	});

	const names = ['main', 'feature/FEAT-158', 'fix/BUG-097'];
	const rows = (v: Mounted) => v.all('.first .row').map((row) => row.textContent);

	it('opens on the first letter and narrows the list', () => {
		const v = mount({ names });
		const event = press(document.body, 'f');
		expect(event.defaultPrevented).toBe(true);
		expect(rows(v)).toEqual(['feature/FEAT-158', 'fix/BUG-097']);
		press(document.body, 'b');
		expect(rows(v)).toEqual(['fix/BUG-097']);
		expect(document.querySelector('.speed-search')?.textContent).toContain('fb');
		expect(document.querySelector('.speed-search')?.textContent).toContain('1 of 3');
	});

	it('marks what matched', () => {
		const v = mount({ names });
		press(document.body, 'm');
		press(document.body, 'a');
		expect(v.get('.drawn mark').textContent).toBe('ma');
	});

	it('takes a letter back with Backspace and closes when none are left', () => {
		const v = mount({ names });
		press(document.body, 'f');
		press(document.body, 'b');
		press(document.body, 'Backspace');
		expect(rows(v)).toHaveLength(2);
		press(document.body, 'Backspace');
		expect(rows(v)).toHaveLength(3);
		expect(speed.open).toBeNull();
		expect(document.querySelector('.speed-search')).toBeNull();
	});

	it('puts the whole list back on Escape', () => {
		const v = mount({ names });
		press(document.body, 'q');
		expect(document.querySelector('.speed-search')?.textContent).toContain('no match');
		const escape = press(document.body, 'Escape');
		expect(escape.defaultPrevented).toBe(true);
		expect(rows(v)).toHaveLength(3);
	});

	it('opens the first match on Enter', () => {
		const onenter = vi.fn();
		mount({ names, onenter });
		press(document.body, 'f');
		press(document.body, 'i');
		press(document.body, 'Enter');
		expect(onenter).toHaveBeenCalledWith('fix/BUG-097');
		expect(speed.open).toBeNull();
	});

	it('leaves letters typed into a field to the field', () => {
		const v = mount({ names });
		const event = press(v.get('.elsewhere'), 'f');
		expect(event.defaultPrevented).toBe(false);
		expect(speed.open).toBeNull();
	});

	it('leaves shortcuts alone', () => {
		mount({ names });
		expect(press(document.body, 'f', { ctrlKey: true }).defaultPrevented).toBe(false);
		expect(press(document.body, 'f', { metaKey: true }).defaultPrevented).toBe(false);
		expect(speed.open).toBeNull();
	});

	it('never starts on a space, which presses the focused button', () => {
		mount({ names });
		expect(press(document.body, ' ').defaultPrevented).toBe(false);
		expect(speed.open).toBeNull();
	});

	it('leaves the letters a screen already answers to', () => {
		mount({ names, reserve: 'jk' });
		expect(press(document.body, 'j').defaultPrevented).toBe(false);
		expect(speed.open).toBeNull();
		press(document.body, 'm');
		press(document.body, 'j');
		expect(speed.open?.options.search?.query).toBe('mj');
	});

	it('leaves an open menu its keys', () => {
		mount({ names });
		const menu = document.createElement('div');
		menu.setAttribute('role', 'menu');
		document.body.appendChild(menu);
		expect(press(document.body, 'f').defaultPrevented).toBe(false);
	});

	it('reaches only a list inside an open modal', () => {
		mount({ names });
		const modal = document.createElement('div');
		modal.setAttribute('aria-modal', 'true');
		document.body.appendChild(modal);
		expect(press(document.body, 'f').defaultPrevented).toBe(false);
	});

	it('types into the list’s own field when it was clicked in last', () => {
		const v = mount({ names, others: ['alpha', 'beta'] });
		v.get('.other').dispatchEvent(new PointerEvent('pointerdown', { bubbles: true }));
		press(v.get('.other'), 'b');
		flushSync();
		const field = v.get('.filter') as HTMLInputElement;
		expect(field.value).toBe('b');
		expect(document.activeElement).toBe(field);
		expect(v.all('.other-row').map((row) => row.textContent)).toEqual(['beta']);
		expect(speed.open).toBeNull();
	});

	it('closes when the click lands outside the list', () => {
		const v = mount({ names });
		press(document.body, 'm');
		expect(rows(v)).toEqual(['main']);
		v.get('.other').dispatchEvent(new PointerEvent('pointerdown', { bubbles: true }));
		flushSync();
		expect(rows(v)).toHaveLength(3);
	});

	it('keeps the search when a match is clicked', () => {
		const v = mount({ names });
		press(document.body, 'm');
		const row = v.get('.first .row');
		row.dispatchEvent(new PointerEvent('pointerdown', { bubbles: true }));
		click(row);
		expect(rows(v)).toEqual(['main']);
	});
});
