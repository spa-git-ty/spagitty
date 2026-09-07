// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * Author pictures, asked for once and drawn from memory afterwards (FEAT-079).
 *
 * # Why the graph cannot simply `await` one
 *
 * The lane canvas repaints synchronously, for every visible node, on every
 * scroll frame. It cannot wait for anything, and a picture that arrives half a
 * second after a row was painted has to reach the screen without the caller
 * having asked twice. So this is a cache with a reactive counter beside it,
 * the same arrangement `store.svelte.ts` uses for rows and for the same reason:
 * [`lookup`] is a synchronous read that never blocks and never rejects, and
 * `version` is what tells the canvas that a repaint would now show something
 * new.
 *
 * # What "once" means here
 *
 * Once per address, per session, whatever the answer was — a hit, a miss, a
 * failed request. A repository with one author and forty thousand commits makes
 * exactly one call. The disk cache behind the command makes it once per author
 * per *month* across sessions; this layer exists so that a screen full of the
 * same four people does not make four calls per scroll.
 *
 * A pending address is held in the same map as a resolved one, so two rows by
 * the same person painted in one frame produce one request rather than two.
 *
 * # The queue
 *
 * Requests go out a few at a time. Not for the backend's sake — it answers most
 * of them off a disk cache — but because a repository whose first screen has
 * thirty distinct authors would otherwise open thirty sockets in the frame the
 * window appears in, on the same thread pool the graph walk is using.
 */

import * as api from '../api';
import { seedOf } from './portrait';

/** How many requests are allowed to be in flight together. */
const AT_ONCE = 4;

/** What is known about one address. */
interface Entry {
	/** The decoded picture, once it has arrived and loaded. */
	image: HTMLImageElement | null;
	/** The account name, when the address carries one. */
	handle: string | null;
	/** Whether the answer is final, so it is never asked for twice. */
	settled: boolean;
}

/**
 * Untracked storage, like the graph's row buffer and for the same reason: a
 * repository's whole committer list wrapped in reactive proxies would be paid
 * for on every paint, and nothing here needs per-entry reactivity. `version` is
 * the one signal.
 */
const held = new Map<string, Entry>();

let version = $state(0);
let queue: string[] = [];
let running = 0;

/**
 * Whether to ask at all.
 *
 * Set from the Settings store once it has read the preference. The default is
 * `false` rather than `true` — not because the preference is off, but because
 * nothing should leave the machine in the window between the graph's first
 * paint and the preference being known. The backend checks the preference too
 * and is the authority; this only decides whether to bother it.
 */
let enabled = $state(false);

/** Decode a `data:` URL into something the canvas can draw. */
function decode(seed: string, picture: string): void {
	const image = new Image();
	image.onload = () => {
		const entry = held.get(seed);
		if (entry) entry.image = image;
		// The picture is only now drawable. Bumping before `onload` would make
		// the canvas draw an image with no dimensions, which paints nothing and
		// leaves the node blank until the next unrelated repaint.
		version += 1;
	};
	// A `data:` URL that will not decode is a miss like any other: the entry is
	// already settled, so nothing asks again and the generated face stands.
	image.onerror = () => {};
	image.src = picture;
}

async function run(seed: string): Promise<void> {
	try {
		const answer = await api.avatar(seed);
		const entry = held.get(seed);
		if (!entry) return;

		entry.handle = answer.handle;
		// A handle with no picture is still worth a repaint: it is what the
		// hover says.
		if (answer.handle) version += 1;
		if (answer.picture) decode(seed, answer.picture);
	} catch {
		// Every ordinary absence already comes back as a null field, so a
		// rejection here is the command itself being unavailable — in a plain
		// browser, or during shutdown. The generated face is the answer.
	} finally {
		const entry = held.get(seed);
		if (entry) entry.settled = true;
		running -= 1;
		pump();
	}
}

function pump(): void {
	while (running < AT_ONCE && queue.length > 0) {
		const seed = queue.shift();
		if (seed === undefined) return;
		running += 1;
		void run(seed);
	}
}

export const avatars = {
	/**
	 * Bumped whenever something new could be drawn. Read by the lane canvas to
	 * make a repaint depend on it.
	 */
	get version(): number {
		return version;
	},

	/** Whether pictures are being fetched at all. */
	get enabled(): boolean {
		return enabled;
	},

	/**
	 * Turn fetching on or off, from the preference.
	 *
	 * Turning it off drops what is held as well as stopping the queue: a
	 * picture that goes on being drawn after somebody opted out is a preference
	 * that did not take effect. The backend empties its disk cache on the same
	 * switch.
	 */
	setEnabled(value: boolean): void {
		if (value === enabled) return;
		enabled = value;
		if (!value) {
			held.clear();
			queue = [];
		}
		version += 1;
	},

	/**
	 * What is known about an author, right now, without waiting.
	 *
	 * The first call for an address starts a request and returns nothing, which
	 * is the answer the graph draws a generated face for. Later calls return
	 * whatever has arrived since.
	 */
	lookup(email: string, name = ''): { image: HTMLImageElement | null; handle: string | null } {
		// Read so a change to the preference re-runs an effect that called this.
		void enabled;
		const seed = seedOf(email, name);
		if (!seed) return { image: null, handle: null };

		const entry = held.get(seed);
		if (entry) return { image: entry.image, handle: entry.handle };

		if (!enabled) return { image: null, handle: null };

		held.set(seed, { image: null, handle: null, settled: false });
		queue.push(seed);
		pump();
		return { image: null, handle: null };
	},

	/**
	 * The picture for an address if one has already arrived, asking for
	 * nothing.
	 *
	 * For the canvas, which paints inside a scroll frame and must not be the
	 * thing that starts a hundred requests while somebody flings the list. The
	 * rows ask through [`lookup`]; the canvas draws what asking produced.
	 */
	drawable(email: string, name = ''): HTMLImageElement | null {
		void version;
		return held.get(seedOf(email, name))?.image ?? null;
	},

	/** Drop everything. For tests, and for a repository being closed. */
	reset(): void {
		held.clear();
		queue = [];
		running = 0;
		version += 1;
	}
};
