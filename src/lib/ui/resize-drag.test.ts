// SPDX-License-Identifier: GPL-3.0-or-later

import { describe, expect, it } from 'vitest';
import { createResizeDrag } from './resize-drag';

/** A frame clock the test advances by hand. */
function clock() {
	let next = 1;
	const queued = new Map<number, () => void>();
	return {
		frame(callback: () => void): number {
			const handle = next++;
			queued.set(handle, callback);
			return handle;
		},
		cancelFrame(handle: number): void {
			queued.delete(handle);
		},
		get pending(): number {
			return queued.size;
		},
		tick(): void {
			const due = [...queued.values()];
			queued.clear();
			for (const callback of due) callback();
		}
	};
}

function harness() {
	const time = clock();
	const applied: number[] = [];
	let settled = 0;
	const drag = createResizeDrag({
		apply: (width) => applied.push(width),
		settle: () => settled++,
		frame: (callback) => time.frame(callback),
		cancelFrame: (handle) => time.cancelFrame(handle)
	});
	return {
		drag,
		time,
		applied,
		get settled() {
			return settled;
		}
	};
}

describe('createResizeDrag', () => {
	it('moves nothing when pressed', () => {
		const h = harness();
		h.drag.start(100, 200);
		expect(h.drag.active).toBe(true);
		expect(h.applied).toEqual([]);
		expect(h.time.pending).toBe(0);
	});

	it('applies only the latest position, once per frame', () => {
		const h = harness();
		h.drag.start(100, 200);
		for (let x = 101; x <= 140; x++) h.drag.move(x);
		expect(h.applied).toEqual([]);
		expect(h.time.pending).toBe(1);

		h.time.tick();
		expect(h.applied).toEqual([240]);

		h.drag.move(90);
		h.drag.move(80);
		h.time.tick();
		expect(h.applied).toEqual([240, 180]);
	});

	it('does not save during the drag', () => {
		const h = harness();
		h.drag.start(0, 150);
		for (let x = 1; x < 50; x++) {
			h.drag.move(x);
			h.time.tick();
		}
		expect(h.settled).toBe(0);
	});

	it('flushes the release position, cancels the pending frame, and saves once', () => {
		const h = harness();
		h.drag.start(100, 200);
		h.drag.move(150);
		h.drag.end(170);

		expect(h.applied).toEqual([270]);
		expect(h.settled).toBe(1);
		expect(h.time.pending).toBe(0);
		expect(h.drag.active).toBe(false);

		// The cancelled frame must not land later with a stale width.
		h.time.tick();
		expect(h.applied).toEqual([270]);
	});

	it('survives fast reversals without losing the final position', () => {
		const h = harness();
		h.drag.start(300, 300);
		const path = [250, 120, 60, 200, 380, 90, 310];
		for (const x of path) h.drag.move(x);
		h.time.tick();
		for (const x of [40, 400, 20]) h.drag.move(x);
		h.drag.end(20);
		expect(h.applied.at(-1)).toBe(20);
		expect(h.settled).toBe(1);
	});

	it('is idempotent across pointerup followed by lostpointercapture', () => {
		const h = harness();
		h.drag.start(0, 100);
		h.drag.move(30);
		h.drag.end(30);
		h.drag.end();
		expect(h.settled).toBe(1);
		expect(h.applied).toEqual([130]);
	});

	it('keeps the last seen position on cancellation', () => {
		const h = harness();
		h.drag.start(0, 100);
		h.drag.move(-40);
		h.drag.end();
		expect(h.applied).toEqual([60]);
		expect(h.settled).toBe(1);
		expect(h.drag.active).toBe(false);
		expect(h.time.pending).toBe(0);
	});

	it('neither applies nor saves a press that never moved', () => {
		const h = harness();
		h.drag.start(50, 100);
		h.drag.end(50);
		expect(h.applied).toEqual([]);
		expect(h.settled).toBe(0);
	});

	it('ignores moves once ended, and moves that went nowhere', () => {
		const h = harness();
		h.drag.move(10);
		expect(h.time.pending).toBe(0);
		h.drag.start(10, 100);
		h.drag.move(10);
		expect(h.time.pending).toBe(0);
		h.drag.end();
		h.drag.move(90);
		h.time.tick();
		expect(h.applied).toEqual([]);
	});

	it('finishes a live drag when a new one starts', () => {
		const h = harness();
		h.drag.start(0, 100);
		h.drag.move(25);
		h.drag.start(500, 300);
		expect(h.applied).toEqual([125]);
		expect(h.settled).toBe(1);
		expect(h.time.pending).toBe(0);
		expect(h.drag.active).toBe(true);
	});

	it('falls back to the browser frame clock', () => {
		const calls: string[] = [];
		const raf = globalThis.requestAnimationFrame;
		const caf = globalThis.cancelAnimationFrame;
		globalThis.requestAnimationFrame = ((cb: FrameRequestCallback) => {
			calls.push('raf');
			cb(0);
			return 7;
		}) as typeof requestAnimationFrame;
		globalThis.cancelAnimationFrame = (() => calls.push('caf')) as typeof cancelAnimationFrame;
		try {
			const applied: number[] = [];
			const drag = createResizeDrag({ apply: (w) => applied.push(w), settle: () => {} });
			drag.start(0, 10);
			drag.move(5);
			expect(applied).toEqual([15]);
			expect(calls).toEqual(['raf']);
		} finally {
			globalThis.requestAnimationFrame = raf;
			globalThis.cancelAnimationFrame = caf;
		}
	});
});
