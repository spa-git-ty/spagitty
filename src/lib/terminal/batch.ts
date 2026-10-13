// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * Output, gathered and written once a frame (FEAT-130).
 *
 * `yes | head -n 1000000` arrives as thousands of small pieces in a second.
 * Handing each to the emulator as it arrives makes it parse and schedule a
 * draw for each, and the window stops answering. Gathered, a frame's worth is
 * one write: the emulator parses it in one go and draws once.
 *
 * Nothing is dropped and nothing is reordered. A piece that ends in the
 * middle of a character or an escape sequence is joined to the next before
 * the emulator sees either, which is also the only correct way to hand them
 * over.
 */

export type Schedule = (work: () => void) => void;

const nextFrame: Schedule = (work) => {
	if (typeof requestAnimationFrame === 'function') requestAnimationFrame(() => work());
	else setTimeout(work, 16);
};

export class Batcher {
	private pieces: Uint8Array[] = [];
	private size = 0;
	private waiting = false;

	constructor(
		private readonly write: (bytes: Uint8Array) => void,
		private readonly schedule: Schedule = nextFrame
	) {}

	/** Take a piece. It is written with everything else that arrives this frame. */
	push(bytes: Uint8Array): void {
		if (bytes.length === 0) return;
		this.pieces.push(bytes);
		this.size += bytes.length;
		if (this.waiting) return;
		this.waiting = true;
		this.schedule(() => this.flush());
	}

	/** Write what has been gathered, now. Called by the frame, and before a terminal is closed. */
	flush(): void {
		this.waiting = false;
		if (this.pieces.length === 0) return;
		const all = this.pieces.length === 1 ? this.pieces[0] : join(this.pieces, this.size);
		this.pieces = [];
		this.size = 0;
		this.write(all);
	}

	/** How many bytes are waiting for the frame. */
	get pending(): number {
		return this.size;
	}
}

function join(pieces: Uint8Array[], size: number): Uint8Array {
	const all = new Uint8Array(size);
	let at = 0;
	for (const piece of pieces) {
		all.set(piece, at);
		at += piece.length;
	}
	return all;
}
