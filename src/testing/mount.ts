// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * Mounting helpers for component tests.
 *
 * Svelte 5 components are mounted for real rather than rendered to a string:
 * what these tests are checking is behaviour — a click reaching its handler, a
 * key moving a selection, a disabled control staying disabled — and none of
 * that survives being flattened to HTML.
 *
 * This file sits outside `src/lib` on purpose, so it is not counted as
 * first-party code by the coverage floor. It is scaffolding, not product.
 */

import { createRawSnippet, flushSync, mount, unmount, type Component } from 'svelte';

export interface Mounted {
	/** The element the component was mounted into. */
	target: HTMLElement;
	/** Query one element, or null. */
	find(selector: string): HTMLElement | null;
	/** Query one element, failing loudly when it is missing. */
	get(selector: string): HTMLElement;
	/** Query all matching elements. */
	all(selector: string): HTMLElement[];
	/** Trimmed text content of the whole mount. */
	text(): string;
	destroy(): void;
}

/** Mount a component into a detached container and return handles to it. */
export function render<Props extends Record<string, unknown>>(
	component: Component<Props>,
	props: Props
): Mounted {
	const target = document.createElement('div');
	document.body.appendChild(target);
	const instance = mount(component, { target, props });
	flushSync();

	return {
		target,
		find: (selector) => target.querySelector<HTMLElement>(selector),
		get(selector) {
			const found = target.querySelector<HTMLElement>(selector);
			if (!found) throw new Error(`no element matching ${selector}`);
			return found;
		},
		all: (selector) => [...target.querySelectorAll<HTMLElement>(selector)],
		text: () => target.textContent?.replace(/\s+/g, ' ').trim() ?? '',
		destroy() {
			unmount(instance);
			target.remove();
		}
	};
}

/** A snippet rendering fixed text, for components that take `children`. */
export function textSnippet(content: string) {
	return createRawSnippet(() => ({
		render: () => `<span data-slot>${content}</span>`
	}));
}

/** Click an element and let the resulting state changes reach the DOM. */
export function click(element: HTMLElement, init: MouseEventInit = {}): void {
	element.dispatchEvent(new MouseEvent('click', { bubbles: true, ...init }));
	flushSync();
}

/** Dispatch a keyboard event and flush. */
export function press(
	target: EventTarget,
	key: string,
	init: KeyboardEventInit = {}
): KeyboardEvent {
	const event = new KeyboardEvent('keydown', { key, bubbles: true, cancelable: true, ...init });
	target.dispatchEvent(event);
	flushSync();
	return event;
}

/** Dispatch a generic event and flush. */
export function fire(target: EventTarget, type: string, init: EventInit = {}): Event {
	const event = new Event(type, { bubbles: true, cancelable: true, ...init });
	target.dispatchEvent(event);
	flushSync();
	return event;
}

export { flushSync };
