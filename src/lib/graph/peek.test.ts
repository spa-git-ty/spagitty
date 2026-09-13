// SPDX-License-Identifier: GPL-3.0-or-later

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { createPeek, fullMessage } from './peek.svelte';

function detail(id: string, summary: string, body = '') {
	return { id, summary, body };
}

/** A lookup the test answers by hand, in whatever order it likes. */
function deferredLookup() {
	const waiting = new Map<string, (value: ReturnType<typeof detail>) => void>();
	const asked: string[] = [];
	return {
		asked,
		lookup: (id: string) =>
			new Promise<ReturnType<typeof detail>>((resolve) => {
				asked.push(id);
				waiting.set(id, resolve);
			}),
		answer(id: string, summary: string, body = '') {
			waiting.get(id)?.(detail(id, summary, body));
			waiting.delete(id);
		}
	};
}

describe('fullMessage', () => {
	it('keeps paragraph breaks and trailers', () => {
		const text = fullMessage(
			detail('a', 'feat: one', 'Why it is.\n\nMore.\n\nCo-Authored-By: Someone <s@x>\n')
		);
		expect(text).toBe('feat: one\n\nWhy it is.\n\nMore.\n\nCo-Authored-By: Someone <s@x>');
	});

	it('is just the subject when there is no body', () => {
		expect(fullMessage(detail('a', 'fix', '  \n'))).toBe('fix');
	});
});

describe('createPeek', () => {
	beforeEach(() => vi.useFakeTimers());
	afterEach(() => vi.useRealTimers());

	it('waits for the pointer to settle before asking', async () => {
		const backend = deferredLookup();
		const peek = createPeek({ lookup: backend.lookup, delay: 400 });

		peek.enter(3, 'c3', 10, 20);
		vi.advanceTimersByTime(399);
		expect(backend.asked).toEqual([]);

		vi.advanceTimersByTime(1);
		expect(backend.asked).toEqual(['c3']);
		backend.answer('c3', 'subject', 'body');
		await vi.runAllTimersAsync();
		expect(peek.current).toEqual({ index: 3, id: 'c3', text: 'subject\n\nbody', x: 10, y: 20 });
	});

	it('asks nothing for rows the pointer only crossed', () => {
		const backend = deferredLookup();
		const peek = createPeek({ lookup: backend.lookup, delay: 400 });
		for (let i = 0; i < 20; i++) {
			peek.enter(i, `c${i}`, 0, i * 30);
			vi.advanceTimersByTime(50);
		}
		peek.leave();
		vi.advanceTimersByTime(1000);
		expect(backend.asked).toEqual([]);
	});

	it('drops a stale answer that lands after the pointer moved on', async () => {
		const backend = deferredLookup();
		const peek = createPeek({ lookup: backend.lookup, delay: 10 });

		peek.enter(1, 'old', 0, 0);
		vi.advanceTimersByTime(10);
		peek.enter(2, 'new', 0, 30);
		vi.advanceTimersByTime(10);
		expect(backend.asked).toEqual(['old', 'new']);

		backend.answer('new', 'the new one');
		await vi.runAllTimersAsync();
		backend.answer('old', 'the old one');
		await vi.runAllTimersAsync();

		expect(peek.current?.id).toBe('new');
		expect(peek.current?.text).toBe('the new one');
	});

	it('drops an answer that lands after the pointer left', async () => {
		const backend = deferredLookup();
		const peek = createPeek({ lookup: backend.lookup, delay: 10 });
		peek.enter(1, 'a', 0, 0);
		vi.advanceTimersByTime(10);
		peek.leave();
		backend.answer('a', 'late');
		await vi.runAllTimersAsync();
		expect(peek.current).toBeNull();
	});

	it('asks once per commit', async () => {
		const backend = deferredLookup();
		const peek = createPeek({ lookup: backend.lookup, delay: 10 });
		peek.enter(1, 'a', 0, 0);
		vi.advanceTimersByTime(10);
		backend.answer('a', 'A');
		await vi.runAllTimersAsync();
		peek.leave();

		peek.enter(1, 'a', 5, 5);
		vi.advanceTimersByTime(10);
		expect(backend.asked).toEqual(['a']);
		expect(peek.current?.text).toBe('A');
	});

	it('uses the selected commit detail without a lookup', () => {
		const backend = deferredLookup();
		const peek = createPeek({
			lookup: backend.lookup,
			loaded: () => detail('sel', 'picked', 'with a body'),
			delay: 10
		});
		peek.enter(0, 'sel', 0, 0);
		vi.advanceTimersByTime(10);
		expect(backend.asked).toEqual([]);
		expect(peek.current?.text).toBe('picked\n\nwith a body');
	});

	it('forgets the oldest answers past its limit', async () => {
		const backend = deferredLookup();
		const peek = createPeek({ lookup: backend.lookup, delay: 1, keep: 2 });
		for (const id of ['a', 'b', 'c']) {
			peek.enter(0, id, 0, 0);
			vi.advanceTimersByTime(1);
			backend.answer(id, id.toUpperCase());
			await vi.runAllTimersAsync();
			peek.leave();
		}
		peek.enter(0, 'a', 0, 0);
		vi.advanceTimersByTime(1);
		expect(backend.asked).toEqual(['a', 'b', 'c', 'a']);
	});

	it('shows nothing when the lookup fails', async () => {
		const peek = createPeek({ lookup: () => Promise.reject(new Error('gone')), delay: 1 });
		peek.enter(0, 'x', 0, 0);
		vi.advanceTimersByTime(1);
		await vi.runAllTimersAsync();
		expect(peek.current).toBeNull();
	});

	it('follows the pointer while it stays, and ignores re-entering the same commit', async () => {
		const backend = deferredLookup();
		const peek = createPeek({ lookup: backend.lookup, delay: 1 });
		peek.move(1, 1);
		expect(peek.current).toBeNull();
		peek.enter(0, 'a', 0, 0);
		vi.advanceTimersByTime(1);
		backend.answer('a', 'A');
		await vi.runAllTimersAsync();
		peek.enter(0, 'a', 99, 99);
		expect(peek.current?.x).toBe(0);
		peek.move(40, 50);
		expect(peek.current).toMatchObject({ x: 40, y: 50, id: 'a' });
	});
});
