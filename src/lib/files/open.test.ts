// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * Right-click a file on any screen and open it in Files: the address, the
 * revision a change is read at, and the window's menu.
 */

import { afterEach, describe, expect, it, vi } from 'vitest';
import { click, flushSync, render, type Mounted } from '../../testing/mount';

const { goto } = vi.hoisted(() => ({ goto: vi.fn() }));
vi.mock('$app/navigation', () => ({ goto }));

import FileMenu from './FileMenu.svelte';
import { fileAttrs, filesHref, pullFileAttrs, revOfChange } from './open';

const SHA = 'c'.repeat(40);

describe('the address', () => {
	it('names the file, the revision and the line', () => {
		expect(filesHref({ path: 'src/a b.rs', rev: 'main', line: 12 })).toBe(
			'/files?path=src%2Fa+b.rs&rev=main&line=12'
		);
		expect(filesHref({ path: 'README.md' })).toBe('/files?path=README.md');
	});

	it('reads a file a commit deleted where its parent left it', () => {
		expect(revOfChange(SHA, 'modified')).toBe(SHA);
		expect(revOfChange(SHA, 'deleted')).toBe(`${SHA}^`);
	});

	it('says why a file has nothing to read', () => {
		expect(fileAttrs('a.rs', 'main')).toEqual({
			'data-file-path': 'a.rs',
			'data-file-rev': 'main',
			'data-file-absent': undefined
		});
		expect(fileAttrs('a.rs', null)['data-file-absent']).toBe('Not in a commit');
		expect(fileAttrs('a.rs', 'main', 'Not in a commit yet')['data-file-absent']).toBe(
			'Not in a commit yet'
		);
	});

	it('reads a pull request’s file at its head, and a deleted one where the head left the target', () => {
		const head = { head: 'h'.repeat(40), mergeBase: 'b'.repeat(40) };
		expect(pullFileAttrs('a.rs', 'modified', head)['data-file-rev']).toBe(head.head);
		expect(pullFileAttrs('a.rs', 'deleted', head)['data-file-rev']).toBe(head.mergeBase);
		expect(pullFileAttrs('a.rs', 'modified', null)['data-file-absent']).toBe('Not fetched');
	});
});

describe('the menu', () => {
	let view: Mounted | null = null;
	let rows: HTMLElement | null = null;

	afterEach(() => {
		view?.destroy();
		rows?.remove();
		view = rows = null;
		goto.mockClear();
	});

	function screen(html: string): HTMLElement {
		rows = document.createElement('div');
		rows.innerHTML = html;
		document.body.append(rows);
		view = render(FileMenu, {});
		return rows;
	}

	function rightClick(target: Element): Event {
		const event = new MouseEvent('contextmenu', { bubbles: true, cancelable: true, clientX: 20, clientY: 30 });
		target.dispatchEvent(event);
		flushSync();
		return event;
	}

	const entry = () =>
		[...document.querySelectorAll<HTMLElement>('[role="menuitem"]')].find((item) =>
			item.textContent?.includes('Open in Files')
		);

	it('opens a file at the revision its row shows', () => {
		const at = screen(`<button data-file-path="src/lib.rs" data-file-rev="${SHA}"><span>lib.rs</span></button>`);

		const event = rightClick(at.querySelector('span')!);
		expect(event.defaultPrevented).toBe(true);
		expect(entry()?.textContent).toContain(SHA.slice(0, 7));

		click(entry()!);
		expect(goto).toHaveBeenCalledWith(`/files?path=src%2Flib.rs&rev=${SHA}`);
	});

	it('shows why a file cannot be opened, and does not open it', () => {
		const at = screen('<div data-file-path="new.rs" data-file-absent="Not in a commit yet">new.rs</div>');

		rightClick(at.firstElementChild!);
		expect((entry() as HTMLButtonElement).disabled).toBe(true);
		expect(document.body.textContent).toContain('Not in a commit yet');
		click(entry()!);
		expect(goto).not.toHaveBeenCalled();
	});

	describe('without the menu (FEAT-150)', () => {
		const press = (target: Element, init: MouseEventInit = {}, type = 'click') => {
			const event = new MouseEvent(type, { bubbles: true, cancelable: true, button: 0, ...init });
			target.dispatchEvent(event);
			return event;
		};

		it('opens any file row on a double-click, at its revision and line', () => {
			const at = screen(`<button data-file-path="src/lib.rs" data-file-rev="${SHA}" data-file-line="12"><span>lib.rs</span></button>`);

			press(at.querySelector('span')!, {}, 'dblclick');
			expect(goto).toHaveBeenCalledWith(`/files?path=src%2Flib.rs&rev=${SHA}&line=12`);
		});

		it('opens a row on a plain click only when the row says it has no click of its own', () => {
			const at = screen(
				'<button class="own" data-file-path="a.rs" data-file-rev="main">a.rs</button>' +
					'<span class="name" data-file-path="b.rs" data-file-rev="main" data-file-open="click">b.rs<button class="inner">x</button></span>'
			);

			press(at.querySelector('.own')!);
			expect(goto).not.toHaveBeenCalled();

			press(at.querySelector('.inner')!);
			expect(goto).not.toHaveBeenCalled();

			press(at.querySelector('.name')!);
			expect(goto).toHaveBeenCalledWith('/files?path=b.rs&rev=main');
		});

		it('opens any row on a click with Ctrl or ⌘', () => {
			const at = screen('<button data-file-path="a.rs" data-file-rev="main">a.rs</button>');

			press(at.firstElementChild!, { ctrlKey: true });
			press(at.firstElementChild!, { metaKey: true });
			expect(goto).toHaveBeenCalledTimes(2);
		});

		it('does not open a file that is not there to read, or from a field', () => {
			const at = screen(
				'<div data-file-path="new.rs" data-file-absent="Not in a commit yet" data-file-open="click">new.rs</div>' +
					'<label data-file-path="a.rs" data-file-rev="main"><input /></label>'
			);

			press(at.firstElementChild!);
			press(at.firstElementChild!, {}, 'dblclick');
			press(at.querySelector('input')!, {}, 'dblclick');
			expect(goto).not.toHaveBeenCalled();
		});
	});

	it('leaves alone a right-click that is not on a file, or that a row has answered', () => {
		const at = screen('<p>nothing</p><div data-file-path="a.rs" data-file-rev="main">a.rs</div>');

		expect(rightClick(at.querySelector('p')!).defaultPrevented).toBe(false);
		expect(entry()).toBeUndefined();

		const answered = new MouseEvent('contextmenu', { bubbles: true, cancelable: true });
		answered.preventDefault();
		at.querySelector('div')!.dispatchEvent(answered);
		flushSync();
		expect(entry()).toBeUndefined();
	});
});
