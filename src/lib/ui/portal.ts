// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * Move an element to the end of `<body>` while it is mounted (TASK-069).
 *
 * A `position: fixed` element is placed against the window, unless an
 * ancestor has a `backdrop-filter`, a `filter` or a `transform`: then it is
 * placed against that ancestor. Every dialog here is glass, with a
 * `backdrop-filter`, so a menu opened inside one was drawn as far from its
 * field as the dialog is from the window's corner. Out of the dialog, it is
 * placed where its field is.
 *
 * `on` false leaves the element where it is. It is read once, at mount.
 */
export function portal(node: HTMLElement, on: boolean = true) {
	if (on) document.body.appendChild(node);
	return {
		destroy() {
			if (on) node.remove();
		}
	};
}
