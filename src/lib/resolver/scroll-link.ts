// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * Columns that scroll together (FEAT-128).
 *
 * A conflict's three columns are three texts of three lengths: one side may be
 * forty lines where the other is four. Moving them by the same number of
 * pixels would leave the short one at its end while the long one is a third of
 * the way down. So they move by the same *share* of what each has to scroll:
 * all three are at their tops together and at their ends together.
 */

/** What is read from a column: how far it is scrolled, and how far it can be. */
export interface Scrolled {
	scrollTop: number;
	scrollHeight: number;
	clientHeight: number;
}

/**
 * Where `target` belongs when `source` is where it is.
 *
 * A column that has nothing to scroll stays at its top, and one that follows
 * a column with nothing to scroll stays where it was.
 */
export function follow(source: Scrolled, target: Scrolled): number {
	const span = source.scrollHeight - source.clientHeight;
	const room = target.scrollHeight - target.clientHeight;
	if (room <= 0) return 0;
	if (span <= 0) return target.scrollTop;
	const share = Math.min(1, Math.max(0, source.scrollTop / span));
	return Math.round(share * room);
}

/**
 * How far a column may be from where it was put and still be taken as put
 * there: a browser keeps a part of a pixel that the place asked for did not
 * have.
 */
const NEAR = 2;

/**
 * Keeps a set of columns in step.
 *
 * Moving a follower raises a scroll event on it, which would move the leader
 * back by a rounding error, and the two would creep. So each column remembers
 * where it was last put from here, and the event that finds it there is its
 * own echo and moves nothing. Any other event is the person's, whichever
 * column it is on and however soon after the last: that column leads.
 */
export function linker() {
	const put = new WeakMap<HTMLElement, number>();

	return {
		/** `from` was scrolled: put `others` where they belong. */
		scrolled(from: HTMLElement, others: Iterable<HTMLElement>): void {
			const expected = put.get(from);
			if (expected !== undefined) {
				put.delete(from);
				if (Math.abs(from.scrollTop - expected) <= NEAR) return;
			}
			for (const other of others) {
				if (other === from) continue;
				const top = follow(from, other);
				// Already there: nothing moves, so no event will come to answer for.
				if (other.scrollTop === top) continue;
				other.scrollTop = top;
				put.set(other, top);
			}
		}
	};
}
