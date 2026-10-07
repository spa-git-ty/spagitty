// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * Scrollbars that show while something scrolls, then fade away (BUG-054).
 *
 * The author asked for them gone at rest and back the moment anything moves,
 * the way macOS and a phone do it. Scroll events do not bubble, but they can
 * be caught on the way down, so one listener on the document marks whichever
 * element is scrolling with `is-scrolling` and clears it after a quiet spell.
 * `app.css` paints the thumb only on a marked element, or under the pointer.
 */

/** How long a scrollbar stays after the last scroll, in milliseconds. */
export const LINGER_MS = 900;

export const SCROLLING_CLASS = 'is-scrolling';

export function watchScrolling(root: Document = document, linger = LINGER_MS): () => void {
	const timers = new Map<Element, ReturnType<typeof setTimeout>>();

	function onscroll(event: Event) {
		const target =
			event.target instanceof Document
				? event.target.scrollingElement
				: event.target instanceof Element
					? event.target
					: null;
		if (!target) return;
		target.classList.add(SCROLLING_CLASS);
		const pending = timers.get(target);
		if (pending) clearTimeout(pending);
		timers.set(
			target,
			setTimeout(() => {
				target.classList.remove(SCROLLING_CLASS);
				timers.delete(target);
			}, linger)
		);
	}

	root.addEventListener('scroll', onscroll, { capture: true, passive: true });
	return () => {
		root.removeEventListener('scroll', onscroll, { capture: true });
		for (const [element, timer] of timers) {
			clearTimeout(timer);
			element.classList.remove(SCROLLING_CLASS);
		}
		timers.clear();
	};
}
