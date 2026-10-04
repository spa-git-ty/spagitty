// SPDX-License-Identifier: GPL-3.0-or-later
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { createRawSnippet, flushSync, mount, unmount } from 'svelte';
import VirtualRows from './VirtualRows.svelte';

/** Observers created, newest last, so a test can report a size to one. */
let observers: { callback: ResizeObserverCallback; observed: Set<Element> }[] = [];

class FakeObserver {
	record: { callback: ResizeObserverCallback; observed: Set<Element> };
	constructor(callback: ResizeObserverCallback) {
		this.record = { callback, observed: new Set() };
		observers.push(this.record);
	}
	observe(element: Element) {
		this.record.observed.add(element);
	}
	unobserve(element: Element) {
		this.record.observed.delete(element);
	}
	disconnect() {
		this.record.observed.clear();
	}
}

const ITEMS = Array.from({ length: 1000 }, (_, i) => `r${i}`);
let target: HTMLElement;
let instance: ReturnType<typeof mount>;

function viewport(): HTMLElement {
	return target.querySelector<HTMLElement>('.viewport')!;
}

function drawn(): string[] {
	return [...target.querySelectorAll<HTMLElement>('.row')].map((row) => row.dataset.key!);
}

beforeEach(() => {
	observers = [];
	vi.stubGlobal('ResizeObserver', FakeObserver);
	vi.spyOn(HTMLElement.prototype, 'clientHeight', 'get').mockReturnValue(300);
	vi.stubGlobal('requestAnimationFrame', () => 0);
	target = document.createElement('div');
	document.body.appendChild(target);
	instance = mount(VirtualRows<string>, {
		target,
		props: {
			items: ITEMS,
			key: (item: string) => item,
			estimate: () => 20,
			row: createRawSnippet((item: () => string) => ({ render: () => `<span>${item()}</span>` })),
			overscan: 600
		}
	});
	flushSync();
});

afterEach(() => {
	unmount(instance);
	target.remove();
	vi.unstubAllGlobals();
	vi.restoreAllMocks();
});

it('draws the rows near the viewport, and stands the rest in for by their height', () => {
	// 300px in view and 600 below it, at 20px a row.
	expect(drawn()).toHaveLength(46);
	expect(drawn()[0]).toBe('r0');
	const spacers = [...viewport().children].filter((child) => !child.classList.contains('row'));
	expect((spacers[1] as HTMLElement).style.height).toBe(`${(1000 - 46) * 20}px`);
});

it('follows the scroll', () => {
	viewport().scrollTop = 10_000;
	viewport().dispatchEvent(new Event('scroll'));
	flushSync();
	expect(drawn()[0]).toBe('r470');
	expect(drawn().at(-1)).toBe('r545');
});

it('keeps the line being read in place when a row above it grows', () => {
	viewport().scrollTop = 10_000;
	viewport().dispatchEvent(new Event('scroll'));
	flushSync();

	const rows = observers.at(-1)!;
	const above = target.querySelector<HTMLElement>('[data-key="r470"]')!;
	const below = target.querySelector<HTMLElement>('[data-key="r520"]')!;
	rows.callback(
		[
			{ target: above, borderBoxSize: [{ blockSize: 50, inlineSize: 0 }] },
			{ target: below, borderBoxSize: [{ blockSize: 80, inlineSize: 0 }] }
		] as unknown as ResizeObserverEntry[],
		rows as unknown as ResizeObserver
	);
	flushSync();
	// Only the row above the top edge moves what is in view.
	expect(viewport().scrollTop).toBe(10_030);
});

it('brings a row into view on asking', () => {
	(instance as unknown as { scrollToIndex(index: number, align: string): void }).scrollToIndex(800, 'start');
	flushSync();
	expect(viewport().scrollTop).toBe(16_000);
	expect(drawn()).toContain('r800');
});
