// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * A pointer drag, as `sortable` hears one: pressed on `from`, moved over
 * `onto`, released there.
 *
 * The test DOM lays nothing out, so "what is under the pointer" is answered
 * by stubbing `elementFromPoint` with the element the drag is over. Every
 * event goes to `from`, as a captured pointer's do in a browser.
 */

import { flushSync } from 'svelte';

function pointer(target: EventTarget, type: string, init: PointerEventInit = {}): PointerEvent {
	const event = new PointerEvent(type, { bubbles: true, cancelable: true, button: 0, pointerId: 1, ...init });
	target.dispatchEvent(event);
	return event;
}

export function drag(from: HTMLElement, onto: HTMLElement, options: { release?: boolean } = {}): void {
	const doc = from.ownerDocument;
	const original = doc.elementFromPoint;
	doc.elementFromPoint = () => onto;
	try {
		pointer(from, 'pointerdown', { clientX: 0, clientY: 0 });
		pointer(from, 'pointermove', { clientX: 0, clientY: 20 });
		pointer(from, 'pointermove', { clientX: 0, clientY: 40 });
		flushSync();
		if (options.release !== false) {
			pointer(from, 'pointerup', { clientX: 0, clientY: 40 });
			flushSync();
		}
	} finally {
		doc.elementFromPoint = original;
	}
}
