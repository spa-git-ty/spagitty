// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * The life of one column-divider drag, apart from the element it happens on
 * (FEAT-081).
 *
 * A divider used to write the column's width on every `pointermove`. A mouse
 * reports faster than a display repaints — 125 to 1000 events a second against
 * 60 frames — so most of those writes laid the table out for a frame nobody
 * would see, and each one also serialised the layout to `localStorage`. The
 * graph column is the expensive one: a width change re-lays every visible row
 * and repaints the lane canvas. Doing that several times per frame is what
 * makes a drag feel heavy even when each individual update is cheap.
 *
 * So the pointer's position is *kept*, and applied at most once per animation
 * frame — the latest position, never a queue of stale ones. Nothing is eased:
 * the boundary is wherever the pointer was when the frame was drawn, which is
 * what "follows the pointer" means on a screen that updates at 60Hz. Easing was
 * rejected outright; a boundary that lags the hand and catches up reads as
 * rubber, and the reference does not do it.
 *
 * The rest is the part that is easy to get wrong and hard to see: a drag that
 * never ends. Releasing outside the window, the system taking the pointer away
 * (`pointercancel`), capture being lost, or the component unmounting mid-drag
 * all have to land in the same place — the last position applied, the pending
 * frame cancelled, the width saved once, and the drag over. That is `end`, and
 * every exit calls it.
 *
 * Kept as plain TypeScript rather than inside the header component so the
 * lifecycle can be tested by handing it a fake frame clock, which is the only
 * way to test "at most once per frame" without a browser.
 */

export interface ResizeDragOptions {
	/** Set the width the drag has reached. Called at most once per frame. */
	apply(width: number): void;
	/** Save the result. Called once when a drag that moved ends. */
	settle(): void;
	/** Schedule a frame. `requestAnimationFrame` unless a test supplies a clock. */
	frame?: (callback: () => void) => number;
	/** Cancel a scheduled frame. */
	cancelFrame?: (handle: number) => void;
}

export interface ResizeDrag {
	/** True between `start` and `end`. */
	readonly active: boolean;
	/**
	 * Begin a drag at `clientX` on a column that measures `startWidth`.
	 *
	 * Moves nothing: the width is only written once the pointer moves, so
	 * pressing a divider — or double-clicking it to reset — does not turn a
	 * filling column into a sized one.
	 */
	start(clientX: number, startWidth: number): void;
	/** The pointer is now at `clientX`. Applied on the next frame. */
	move(clientX: number): void;
	/**
	 * Finish: apply the final position now, drop any pending frame, save.
	 *
	 * Idempotent, because the browser sends `pointerup` and then
	 * `lostpointercapture` for the same release, and both are wired here. With
	 * no `clientX` — a cancellation — the last position seen is kept, which is
	 * where the boundary already is on screen.
	 */
	end(clientX?: number): void;
}

export function createResizeDrag(options: ResizeDragOptions): ResizeDrag {
	const frame =
		options.frame ?? ((callback: () => void) => requestAnimationFrame(() => callback()));
	const cancelFrame = options.cancelFrame ?? ((handle: number) => cancelAnimationFrame(handle));

	let active = false;
	let moved = false;
	let startX = 0;
	let startWidth = 0;
	let latestX = 0;
	let pending: number | null = null;

	function widthAt(clientX: number): number {
		return startWidth + (clientX - startX);
	}

	function flush(): void {
		pending = null;
		if (active) options.apply(widthAt(latestX));
	}

	function end(clientX?: number): void {
		if (!active) return;
		if (clientX !== undefined && clientX !== latestX) {
			latestX = clientX;
			moved = true;
		}
		if (pending !== null) {
			cancelFrame(pending);
			pending = null;
		}
		active = false;
		// A press that never moved changes nothing and saves nothing.
		if (!moved) return;
		options.apply(widthAt(latestX));
		options.settle();
	}

	return {
		get active() {
			return active;
		},

		start(clientX: number, width: number): void {
			// A second press while one drag is live — a second pointer, or an
			// `up` the window never delivered — finishes the first rather than
			// leaving its frame pending against the new start.
			end();
			active = true;
			moved = false;
			startX = clientX;
			latestX = clientX;
			startWidth = width;
		},

		move(clientX: number): void {
			if (!active) return;
			if (clientX === latestX) return;
			latestX = clientX;
			moved = true;
			if (pending === null) pending = frame(flush);
		},

		end
	};
}
