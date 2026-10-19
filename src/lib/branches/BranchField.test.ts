// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * A dialog's branch field (TASK-069): it opens the searchable branch menu,
 * drawn from `<body>` so a dialog's glass cannot move it away from the field.
 */

import { afterEach, describe, expect, it, vi } from 'vitest';
import { click, fire, flushSync, press, render, type Mounted } from '../../testing/mount';
import BranchField from './BranchField.svelte';
import type { BranchOption } from './pick';

const OPTIONS: BranchOption[] = [
	{ name: 'main', group: 'local' },
	{ name: 'feat/tab-drag', group: 'local' },
	{ name: 'origin/main', group: 'remote' }
];

let view: Mounted;

function mount(props: Record<string, unknown> = {}) {
	const onpick = vi.fn();
	view = render(BranchField, {
		id: 'base-branch',
		label: 'Base branch',
		value: 'main',
		options: OPTIONS,
		onpick,
		...props
	});
	return { onpick };
}

const field = () => view.get('button.branch-field') as HTMLButtonElement;
const menu = () => document.querySelector<HTMLElement>('.branch-menu');
const query = () => menu()!.querySelector('input.query') as HTMLInputElement;
const names = () =>
	[...menu()!.querySelectorAll('.entry .name')].map((name) => name.textContent?.trim() ?? '');

afterEach(() => {
	view?.destroy();
});

describe('BranchField', () => {
	it('shows the branch chosen, or what to do while there is none', () => {
		mount();
		expect(field().textContent).toContain('main');
		view.destroy();
		mount({ value: '', placeholder: 'Select a branch…' });
		expect(field().textContent).toContain('Select a branch…');
		expect(field().classList.contains('empty')).toBe(true);
	});

	it('opens the branch menu outside the dialog, above it', () => {
		mount();
		click(field());
		const opened = menu();
		expect(opened).not.toBeNull();
		// Drawn from <body>, not from inside the field's dialog.
		expect(opened!.parentElement).toBe(document.body);
		expect(view.target.contains(opened)).toBe(false);
		expect(opened!.classList.contains('dialog')).toBe(true);
		expect(field().getAttribute('aria-expanded')).toBe('true');
	});

	it('searches, and a pick is handed over and closes the menu', () => {
		const { onpick } = mount();
		click(field());
		query().value = 'drag';
		fire(query(), 'input');
		expect(names()).toEqual(['feat/tab-drag']);
		const entry = menu()!.querySelector('.entry') as HTMLElement;
		click(entry);
		expect(onpick).toHaveBeenCalledWith('feat/tab-drag');
		expect(menu()).toBeNull();
	});

	it('picks with the keyboard', () => {
		const { onpick } = mount();
		click(field());
		query().value = 'origin';
		fire(query(), 'input');
		press(query(), 'Enter');
		flushSync();
		expect(onpick).toHaveBeenCalledWith('origin/main');
	});

	it('leaves nothing behind in <body> when the dialog closes', () => {
		mount();
		click(field());
		expect(menu()).not.toBeNull();
		view.destroy();
		expect(menu()).toBeNull();
		mount();
	});

	it('does not open while the form is busy', () => {
		mount({ disabled: true });
		expect(field().disabled).toBe(true);
	});
});
