// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * The list a branch is chosen from (FEAT-122).
 *
 * Three complaints made it: it could not be searched, the branch checked out
 * was the faintest row in it, and its rows read as two ragged columns. The
 * first two are behaviour and are pinned here; the third is the one column
 * every row is drawn in, which the markup tests below hold in place.
 */

import { afterEach, describe, expect, it, vi } from 'vitest';
import { click, fire, flushSync, press, render, type Mounted } from '../../testing/mount';
import BranchMenu from './BranchMenu.svelte';
import { ROWS_PER_GROUP, type BranchOption } from './pick';

const OPTIONS: BranchOption[] = [
	{ name: 'main', group: 'local' },
	{ name: 'feat/tab-drag', group: 'local' },
	{ name: 'feat/tab-close', group: 'local' },
	{ name: 'old', group: 'local', reason: 'already B' },
	{ name: 'origin/main', group: 'remote' },
	{ name: 'v1.0.0', group: 'tag' }
];

let view: Mounted;
let anchor: HTMLButtonElement | null = null;

function open(props: Partial<Parameters<typeof render<Record<string, unknown>>>[1]> = {}) {
	const onpick = vi.fn();
	const onclose = vi.fn();
	view = render(BranchMenu, {
		x: 40,
		y: 60,
		label: 'Switch branch',
		options: OPTIONS,
		onpick,
		onclose,
		...props
	});
	return { onpick, onclose };
}

const field = () => view.get('input.query') as HTMLInputElement;
const entries = () => view.all('.entry');
const names = () => entries().map((entry) => entry.querySelector('.name')?.textContent?.trim() ?? '');
const entry = (name: string) => entries()[names().indexOf(name)];
const at = () => view.find('.entry.at')?.querySelector('.name')?.textContent?.trim() ?? null;

function type(text: string) {
	field().value = text;
	fire(field(), 'input');
}

afterEach(() => {
	view?.destroy();
	anchor?.remove();
	anchor = null;
});

describe('what it lists', () => {
	it('groups the names under headings that say how many each holds', () => {
		open();

		expect(view.all('.heading').map((heading) => heading.textContent?.replace(/\s+/g, ' ').trim())).toEqual([
			'Branches 4',
			'Remote branches 1',
			'Tags 1'
		]);
		expect(names()).toEqual(['main', 'feat/tab-drag', 'feat/tab-close', 'old', 'origin/main', 'v1.0.0']);
	});

	it('is a listbox named for what it chooses, driven from a field', () => {
		open();

		expect(view.get('[role="listbox"]').getAttribute('aria-label')).toBe('Switch branch');
		expect(field().getAttribute('role')).toBe('combobox');
		expect(field().getAttribute('aria-controls')).toBe(view.get('[role="listbox"]').id);
		expect(entries().every((row) => row.getAttribute('role') === 'option')).toBe(true);
	});

	it('draws every row in one column: a tick slot, then the name', () => {
		// The names line up whether or not a row is the one held, because the
		// tick has its own slot in every row rather than only in one.
		open({ value: 'main' });

		for (const row of entries()) {
			expect(row.children[0].classList.contains('tick'), row.textContent ?? '').toBe(true);
			expect(row.children[1].classList.contains('name'), row.textContent ?? '').toBe(true);
		}
	});

	it('makes the folder part of a name quieter than the part that tells it apart', () => {
		open();

		const row = entry('feat/tab-drag');
		expect(row.querySelector('.dir')?.textContent).toBe('feat/');
		expect(entry('main').querySelector('.dir')).toBeNull();
	});

	it('says so while the names are being read, and when there are none', () => {
		open({ options: [], loading: true });
		expect(view.get('.empty').textContent?.trim()).toBe('Reading branches…');
		view.destroy();

		open({ options: [] });
		expect(view.get('.empty').textContent?.trim()).toBe('No branches');
	});

	it('draws only so many rows of a long group, and says how to reach the rest', () => {
		const many = Array.from({ length: ROWS_PER_GROUP + 12 }, (_, i) => ({
			name: `topic/${i}`,
			group: 'local' as const
		}));
		open({ options: many });

		expect(entries()).toHaveLength(ROWS_PER_GROUP);
		expect(view.get('.more').textContent?.trim()).toBe('+ 12 more. Type to find them.');
	});
});

describe('where you are', () => {
	it('ticks the branch held now and keeps it at full strength', () => {
		open({ value: 'main', current: 'main' });

		const row = entry('main');
		expect(row.classList.contains('chosen')).toBe(true);
		expect(row.getAttribute('aria-selected')).toBe('true');
		expect(row.querySelector('.tick svg')).not.toBeNull();
		// It was the one disabled row, which is the one drawn faintest.
		expect(row.classList.contains('off')).toBe(false);
		expect(row.hasAttribute('disabled')).toBe(false);
		expect(row.getAttribute('aria-disabled')).toBeNull();
	});

	it('marks the branch checked out, in words, wherever it is listed', () => {
		// Merger's card B holds another branch; main is still where you are.
		open({ value: 'feat/tab-drag', current: 'main' });

		expect(entry('main').querySelector('.here')?.textContent).toBe('current');
		expect(view.all('.here')).toHaveLength(1);
		expect(entry('feat/tab-drag').classList.contains('chosen')).toBe(true);
	});

	it('ticks no row but the one held', () => {
		open({ value: 'main' });

		expect(view.all('.tick svg')).toHaveLength(1);
		expect(view.all('[aria-selected="true"]')).toHaveLength(1);
	});

	it('closes without picking when the branch held is chosen again', () => {
		const { onpick, onclose } = open({ value: 'main' });

		click(entry('main'));

		expect(onclose).toHaveBeenCalled();
		expect(onpick).not.toHaveBeenCalled();
	});

	it('lists a branch that cannot be picked, with its reason, and does not pick it', () => {
		const { onpick, onclose } = open();

		const row = entry('old');
		expect(row.getAttribute('aria-disabled')).toBe('true');
		expect(row.querySelector('.why')?.textContent).toBe('already B');
		expect(row.title).toBe('old — already B');

		click(row);
		expect(onpick).not.toHaveBeenCalled();
		expect(onclose).not.toHaveBeenCalled();
	});
});

describe('searching', () => {
	it('puts the focus in the field as it opens', async () => {
		open();
		await vi.waitFor(() => expect(document.activeElement).toBe(field()));
	});

	it('narrows to what was typed and says how many of how many', () => {
		open();

		type('tab');

		expect(names()).toEqual(['feat/tab-drag', 'feat/tab-close']);
		expect(view.get('.count').textContent).toBe('2 of 6');
		expect(view.all('.heading').map((heading) => heading.textContent?.replace(/\s+/g, ' ').trim())).toEqual([
			'Branches 2'
		]);
	});

	it('shows no count until something is typed', () => {
		open();
		expect(view.find('.count')).toBeNull();
	});

	it('finds a name by any part of it, in any case', () => {
		open();

		type('DRAG');

		expect(names()).toEqual(['feat/tab-drag']);
	});

	it('searches every group, remote branches and tags included', () => {
		open();

		type('main');
		expect(names()).toEqual(['main', 'origin/main']);

		type('v1');
		expect(names()).toEqual(['v1.0.0']);
	});

	it('marks where the match is', () => {
		open();

		type('tab');

		expect(entry('feat/tab-drag').querySelector('mark')?.textContent).toBe('tab');
	});

	it('says that nothing matches, in the words typed', () => {
		open();

		type('zzz');

		expect(entries()).toHaveLength(0);
		expect(view.get('.empty').textContent?.trim()).toBe('Nothing matches “zzz”');
	});

	it('brings the whole list back when the field is emptied', () => {
		open();
		type('tab');

		type('');

		expect(entries()).toHaveLength(OPTIONS.length);
	});
});

describe('the keys', () => {
	it('starts on the branch held, so the list opens where you are', () => {
		open({ value: 'feat/tab-close' });
		expect(at()).toBe('feat/tab-close');
	});

	it('puts Enter on the best match as soon as something is typed', () => {
		const { onpick, onclose } = open({ value: 'main' });

		type('close');
		expect(at()).toBe('feat/tab-close');
		press(field(), 'Enter');

		expect(onpick).toHaveBeenCalledWith('feat/tab-close');
		expect(onclose).toHaveBeenCalled();
	});

	it('names the row the keys are on for a screen reader', () => {
		open();
		type('close');

		expect(field().getAttribute('aria-activedescendant')).toBe(view.get('.entry.at').id);
	});

	it('moves down and up over the rows that can be picked, and wraps', () => {
		open({ value: 'main' });

		press(field(), 'ArrowDown');
		expect(at()).toBe('feat/tab-drag');
		press(field(), 'ArrowDown');
		expect(at()).toBe('feat/tab-close');
		// `old` cannot be picked: the cursor does not stop on it.
		press(field(), 'ArrowDown');
		expect(at()).toBe('origin/main');
		press(field(), 'ArrowDown');
		expect(at()).toBe('v1.0.0');
		press(field(), 'ArrowDown');
		expect(at()).toBe('feat/tab-drag');

		press(field(), 'ArrowUp');
		expect(at()).toBe('v1.0.0');
	});

	it('goes to the last row on ArrowUp from the top', () => {
		open();

		press(field(), 'ArrowUp');

		expect(at()).toBe('v1.0.0');
	});

	it('does nothing on Enter with no row under the keys', () => {
		const { onpick, onclose } = open();

		press(field(), 'Enter');

		expect(onpick).not.toHaveBeenCalled();
		expect(onclose).not.toHaveBeenCalled();
	});

	it('does not pick the row held when Enter is pressed on opening', () => {
		const { onpick } = open({ value: 'main' });

		press(field(), 'Enter');

		expect(onpick).not.toHaveBeenCalled();
	});

	it('closes on Escape, picks nothing, and gives the focus back to its control', () => {
		anchor = document.createElement('button');
		document.body.appendChild(anchor);
		const { onpick, onclose } = open({ anchor });

		const event = press(field(), 'Escape');

		expect(onclose).toHaveBeenCalled();
		expect(onpick).not.toHaveBeenCalled();
		expect(event.defaultPrevented).toBe(true);
		expect(document.activeElement).toBe(anchor);
	});

	it('keeps the focus in the field when a row is pressed', () => {
		// Or the next letter typed would go nowhere.
		open();

		const down = new MouseEvent('mousedown', { bubbles: true, cancelable: true });
		entry('feat/tab-drag').dispatchEvent(down);

		expect(down.defaultPrevented).toBe(true);
	});

	it('follows the pointer onto a row that can be picked, and not onto one that cannot', () => {
		open();

		entry('feat/tab-drag').dispatchEvent(new MouseEvent('mouseenter'));
		flushSync();
		expect(at()).toBe('feat/tab-drag');

		entry('old').dispatchEvent(new MouseEvent('mouseenter'));
		flushSync();
		expect(at()).toBe('feat/tab-drag');
	});
});

describe('picking and closing', () => {
	it('closes and then picks, so a dialog the pick opens is not behind it', () => {
		const order: string[] = [];
		open({
			onclose: () => order.push('close'),
			onpick: (name: string) => order.push(`pick ${name}`)
		});

		click(entry('origin/main'));

		expect(order).toEqual(['close', 'pick origin/main']);
	});

	it('closes on a press anywhere else', () => {
		const { onclose } = open();

		document.body.dispatchEvent(new MouseEvent('mousedown', { bubbles: true }));

		expect(onclose).toHaveBeenCalled();
	});

	it('leaves a press on its own control to that control (BUG-018)', () => {
		anchor = document.createElement('button');
		document.body.appendChild(anchor);
		const { onclose } = open({ anchor });

		anchor.dispatchEvent(new MouseEvent('mousedown', { bubbles: true }));

		expect(onclose).not.toHaveBeenCalled();
	});

	it('stays open for a press inside it', () => {
		const { onclose } = open();

		view.get('.find').dispatchEvent(new MouseEvent('mousedown', { bubbles: true }));

		expect(onclose).not.toHaveBeenCalled();
	});

	it('closes when the focus goes somewhere else', () => {
		const { onclose } = open();

		field().dispatchEvent(new FocusEvent('focusout', { bubbles: true, relatedTarget: null }));

		expect(onclose).toHaveBeenCalled();
	});

	it('closes when the window is resized, since it was placed for the old size', () => {
		const { onclose } = open();

		window.dispatchEvent(new Event('resize'));

		expect(onclose).toHaveBeenCalled();
	});
});
