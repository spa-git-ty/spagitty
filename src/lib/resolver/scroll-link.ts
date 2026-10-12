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

/** How long a column that was moved here ignores its own scroll events. */
const SETTLE_MS = 120;

/**
 * Keeps a set of columns in step.
 *
 * Moving a follower raises a scroll event on it, which would move the leader
 * back by a rounding error, and the two would creep. So the column the person
 * is scrolling leads, and for a moment the others only follow.
 */
export function linker(now: () => number = () => performance.now()) {
	let leader: Element | null = null;
	let until = 0;

	return {
		/** `from` was scrolled: put `others` where they belong. */
		scrolled(from: HTMLElement, others: Iterable<HTMLElement>): void {
			const at = now();
			if (leader !== null && leader !== from && at < until) return;
			leader = from;
			until = at + SETTLE_MS;
			for (const other of others) {
				if (other === from) continue;
				const top = follow(from, other);
				if (other.scrollTop !== top) other.scrollTop = top;
			}
		}
	};
}
