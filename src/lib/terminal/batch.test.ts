// SPDX-License-Identifier: GPL-3.0-or-later

import { describe, expect, it } from 'vitest';
import { Batcher } from './batch';

const bytes = (text: string) => new TextEncoder().encode(text);
const text = (all: Uint8Array) => new TextDecoder().decode(all);

/** A frame that comes when the test says so. */
function frames() {
	const waiting: (() => void)[] = [];
	return {
		schedule: (work: () => void) => void waiting.push(work),
		get asked() {
			return waiting.length;
		},
		next() {
			waiting.shift()?.();
		}
	};
}

describe('output gathered for a frame', () => {
	it('writes everything that arrived in a frame once, in the order it arrived', () => {
		const frame = frames();
		const written: string[] = [];
		const batcher = new Batcher((all) => written.push(text(all)), frame.schedule);

		batcher.push(bytes('one '));
		batcher.push(bytes('two '));
		batcher.push(bytes('three'));

		expect(written).toEqual([]);
		expect(frame.asked).toBe(1);
		expect(batcher.pending).toBe(13);

		frame.next();

		expect(written).toEqual(['one two three']);
		expect(batcher.pending).toBe(0);
	});

	/** The item: `yes | head -n 1000000` arrives in frames, not in a million writes. */
	it('turns a million lines into a write for each frame', () => {
		const frame = frames();
		let writes = 0;
		let total = 0;
		const batcher = new Batcher((all) => {
			writes += 1;
			total += all.length;
		}, frame.schedule);
		const line = bytes('y\n');

		for (let i = 0; i < 1_000_000; i++) {
			batcher.push(line);
			// A frame every ten thousand lines.
			if (i % 10_000 === 9_999) frame.next();
		}

		expect(writes).toBe(100);
		expect(total).toBe(2_000_000);
		expect(frame.asked).toBe(0);
	});

	it('joins a character cut in two before the emulator sees either half', () => {
		const frame = frames();
		const written: Uint8Array[] = [];
		const batcher = new Batcher((all) => written.push(all), frame.schedule);
		const snowman = bytes('☃');

		batcher.push(snowman.slice(0, 1));
		batcher.push(snowman.slice(1));
		frame.next();

		expect(written).toHaveLength(1);
		expect(text(written[0])).toBe('☃');
	});

	it('hands a single piece over as it is, without copying it', () => {
		const frame = frames();
		const written: Uint8Array[] = [];
		const batcher = new Batcher((all) => written.push(all), frame.schedule);
		const piece = bytes('alone');

		batcher.push(piece);
		frame.next();

		expect(written[0]).toBe(piece);
	});

	it('asks for one frame at a time, and for another only when more arrives', () => {
		const frame = frames();
		const written: string[] = [];
		const batcher = new Batcher((all) => written.push(text(all)), frame.schedule);

		batcher.push(bytes('a'));
		batcher.push(bytes('b'));
		expect(frame.asked).toBe(1);
		frame.next();
		expect(frame.asked).toBe(0);

		batcher.push(bytes('c'));
		expect(frame.asked).toBe(1);
		frame.next();

		expect(written).toEqual(['ab', 'c']);
	});

	it('takes nothing from an empty piece', () => {
		const frame = frames();
		const written: string[] = [];
		const batcher = new Batcher((all) => written.push(text(all)), frame.schedule);

		batcher.push(new Uint8Array(0));

		expect(frame.asked).toBe(0);
		batcher.flush();
		expect(written).toEqual([]);
	});

	/** A terminal that is closed writes what it had before it goes. */
	it('writes what is waiting when flushed, and the frame that comes after writes nothing twice', () => {
		const frame = frames();
		const written: string[] = [];
		const batcher = new Batcher((all) => written.push(text(all)), frame.schedule);

		batcher.push(bytes('last words'));
		batcher.flush();
		frame.next();

		expect(written).toEqual(['last words']);
	});

	it('uses the window’s own frame when it is given none', async () => {
		const written: string[] = [];
		const batcher = new Batcher((all) => written.push(text(all)));

		batcher.push(bytes('drawn'));
		expect(written).toEqual([]);
		await new Promise((done) => setTimeout(done, 60));

		expect(written).toEqual(['drawn']);
	});
});
