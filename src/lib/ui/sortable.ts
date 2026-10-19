// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * Reordering rows with the pointer, without HTML drag and drop.
 *
 * The Rail section's rows were `draggable` and listened for `drop`, and on
 * Windows nothing was ever dropped. The window takes drags for itself so that
 * files dragged in from the desktop reach Releases with their paths, and while
 * it does WebView2 sends the page no `dragover` and no `drop`. Turning that off
 * would lose the file drops, so the rows are moved with pointer events instead,
 * which the window leaves alone.
 *
 * The list is the node this action is put on. Each row carries
 * `data-sort-key`; a row with a `[data-sort-grip]` inside moves only from its
 * grip, so its buttons still click. A part marked `data-sort-ignore` — a
 * resize handle — never starts a move. A move starts after a few pixels, so a
 * press that does not travel is still a click, and Escape gives it up.
 */

export interface SortState {
	/** The row being moved, or null. */
	dragging: string | null;
	/** The row it would be dropped on, when that row accepts it. */
	over: string | null;
}

export interface SortableOptions {
	/** `from` was dropped on `onto`. Never called with the same key twice. */
	onmove: (from: string, onto: string) => void;
	/** Whether `onto` takes `from`. Everything does when absent. */
	accepts?: (from: string, onto: string) => boolean;
	/** Called whenever what is moving, or where to, changes. */
	onstate?: (state: SortState) => void;
	/** No moving while true. */
	disabled?: boolean;
}

/** How far the pointer goes before a press becomes a move. */
const START_AFTER = 4;

export function sortable(node: HTMLElement, initial: SortableOptions) {
	let options = initial;
	let pressed: { key: string; x: number; y: number; pointer: number } | null = null;
	let dragging: string | null = null;
	let over: string | null = null;
	/** A move just ended, and the click that follows it is not a press. */
	let swallow = false;

	function tell() {
		options.onstate?.({ dragging, over });
	}

	/** Capture is a nicety: a pointer that cannot be captured still moves rows. */
	function capture(call: () => void) {
		try {
			call();
		} catch {
			// Not every pointer can be captured, and nothing depends on it.
		}
	}

	function rowAt(x: number, y: number): HTMLElement | null {
		const hit = node.ownerDocument.elementFromPoint?.(x, y) ?? null;
		const row = hit?.closest<HTMLElement>('[data-sort-key]') ?? null;
		return row && node.contains(row) ? row : null;
	}

	function reset() {
		const was = dragging !== null;
		pressed = null;
		dragging = null;
		over = null;
		node.ownerDocument.removeEventListener('keydown', onkey, true);
		if (was) tell();
	}

	function onpointerdown(event: PointerEvent) {
		// A press that never heard its release starts nothing now.
		if (pressed) reset();
		if (options.disabled || event.button !== 0) return;
		const target = event.target as HTMLElement | null;
		const row = target?.closest<HTMLElement>('[data-sort-key]');
		if (!row || !node.contains(row)) return;
		const grip = row.querySelector('[data-sort-grip]');
		if (grip && !grip.contains(target)) return;
		// A press on a control inside a row with no grip is that control's.
		if (target?.closest('[data-sort-ignore]')) return;
		if (!grip && target !== row && target?.closest('button, input, select, textarea, a')) return;
		pressed = { key: row.dataset.sortKey ?? '', x: event.clientX, y: event.clientY, pointer: event.pointerId };
	}

	function onpointermove(event: PointerEvent) {
		if (!pressed || event.pointerId !== pressed.pointer) return;
		let started = false;
		if (dragging === null) {
			if (Math.hypot(event.clientX - pressed.x, event.clientY - pressed.y) < START_AFTER) return;
			dragging = pressed.key;
			started = true;
			capture(() => node.setPointerCapture(event.pointerId));
			node.ownerDocument.addEventListener('keydown', onkey, true);
		}
		event.preventDefault();
		const row = rowAt(event.clientX, event.clientY);
		const key = row?.dataset.sortKey ?? null;
		const next = key !== null && key !== dragging && (options.accepts?.(dragging, key) ?? true) ? key : null;
		if (started || next !== over) {
			over = next;
			tell();
		}
	}

	function onpointerup(event: PointerEvent) {
		if (!pressed || event.pointerId !== pressed.pointer) return;
		const from = dragging;
		const onto = over;
		capture(() => node.releasePointerCapture(event.pointerId));
		reset();
		if (from !== null && onto !== null && from !== onto) options.onmove(from, onto);
	}

	function onkey(event: KeyboardEvent) {
		if (event.key !== 'Escape') return;
		event.preventDefault();
		event.stopPropagation();
		reset();
	}

	/** A move that ended in a click must not also press what it ended on. */
	function onclick(event: MouseEvent) {
		if (swallow) {
			event.preventDefault();
			event.stopPropagation();
			swallow = false;
		}
	}

	function onup() {
		swallow = dragging !== null;
	}

	node.addEventListener('pointerdown', onpointerdown);
	node.addEventListener('pointermove', onpointermove);
	node.addEventListener('pointerup', onup, true);
	node.addEventListener('pointerup', onpointerup);
	node.addEventListener('pointercancel', reset);
	node.addEventListener('click', onclick, true);

	return {
		update(next: SortableOptions) {
			options = next;
		},
		destroy() {
			node.removeEventListener('pointerdown', onpointerdown);
			node.removeEventListener('pointermove', onpointermove);
			node.removeEventListener('pointerup', onup, true);
			node.removeEventListener('pointerup', onpointerup);
			node.removeEventListener('pointercancel', reset);
			node.removeEventListener('click', onclick, true);
			reset();
		}
	};
}
