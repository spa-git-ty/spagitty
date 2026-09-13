// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * The full commit message, shown while the pointer rests on a row (FEAT-081).
 *
 * The message column truncates to one line, and until this the only way to read
 * the rest was to select the commit — which replaces the detail panel, and is
 * the wrong price for a glance. A native `title` showed the subject again, which
 * was already on screen, and could not show the body at all because the row
 * does not have it: the walk streams subjects only, and bodies cost a lookup.
 *
 * So this asks for the body, and the design is mostly about asking politely:
 *
 * - **Only after the pointer settles.** A pointer crossing twenty rows on its
 *   way to the toolbar is not twenty questions. The delay is the one a native
 *   tooltip uses, give or take, so it feels like one.
 * - **Once per commit.** Answers are kept, most recent last and trimmed from
 *   the front, so moving back and forth over the same few rows costs nothing.
 *   The selected commit's detail is already loaded and is used as-is.
 * - **Never out of turn.** Every enter and leave takes a new token, and an
 *   answer carrying an old one is dropped. Without that, a slow lookup for the
 *   row the pointer *left* lands after it has arrived somewhere else, and the
 *   tooltip names the wrong commit — worse than no tooltip.
 *
 * It never selects anything and never touches the detail panel: hovering is
 * looking, and selection is a decision.
 */

import type { CommitDetail } from '../types';

export interface Peek {
	/** Row index the tooltip belongs to. */
	index: number;
	id: string;
	/** Subject, then the body — trailers included — with its paragraphs kept. */
	text: string;
	/** Viewport position to place it at. */
	x: number;
	y: number;
}

export interface PeekOptions {
	/** Look up a commit's detail. `api.commitDetail` in the application. */
	lookup: (id: string) => Promise<Pick<CommitDetail, 'id' | 'summary' | 'body'>>;
	/** Detail that is already loaded, when there is some. The selected commit's. */
	loaded?: () => Pick<CommitDetail, 'id' | 'summary' | 'body'> | null;
	/** Milliseconds the pointer has to rest before a lookup starts. */
	delay?: number;
	/** How many answers to keep. */
	keep?: number;
}

/** A commit's whole message, as one string with its paragraph breaks. */
export function fullMessage(detail: Pick<CommitDetail, 'summary' | 'body'>): string {
	const body = detail.body.trim();
	return body === '' ? detail.summary : `${detail.summary}\n\n${body}`;
}

export function createPeek(options: PeekOptions) {
	const { lookup, loaded, delay = 450, keep = 200 } = options;

	let current = $state<Peek | null>(null);
	let token = 0;
	let timer: ReturnType<typeof setTimeout> | null = null;
	const cache = new Map<string, string>();

	function remember(id: string, text: string): void {
		cache.delete(id);
		cache.set(id, text);
		while (cache.size > keep) {
			const oldest = cache.keys().next().value;
			if (oldest === undefined) break;
			cache.delete(oldest);
		}
	}

	function known(id: string): string | null {
		const cached = cache.get(id);
		if (cached !== undefined) return cached;
		const detail = loaded?.() ?? null;
		if (detail && detail.id === id) {
			const text = fullMessage(detail);
			remember(id, text);
			return text;
		}
		return null;
	}

	function stopTimer(): void {
		if (timer !== null) clearTimeout(timer);
		timer = null;
	}

	return {
		/** What is showing, or null. */
		get current(): Peek | null {
			return current;
		},

		/** The pointer came to rest over row `index`, commit `id`, at `x, y`. */
		enter(index: number, id: string, x: number, y: number): void {
			if (current?.id === id) return;
			stopTimer();
			current = null;
			const mine = ++token;

			timer = setTimeout(() => {
				timer = null;
				if (mine !== token) return;

				const text = known(id);
				if (text !== null) {
					current = { index, id, text, x, y };
					return;
				}

				lookup(id)
					.then((detail) => {
						const full = fullMessage(detail);
						remember(id, full);
						if (mine === token) current = { index, id, text: full, x, y };
					})
					.catch(() => {
						// No tooltip is the right answer to a failed lookup: the row
						// still shows its subject, and the detail panel will say what
						// went wrong if the commit is selected.
					});
			}, delay);
		},

		/** Where the pointer is now, while it stays on the same row. */
		move(x: number, y: number): void {
			if (current) current = { ...current, x, y };
		},

		/** The pointer left, scrolled away, or something else took over. */
		leave(): void {
			token++;
			stopTimer();
			current = null;
		}
	};
}

export type PeekStore = ReturnType<typeof createPeek>;
