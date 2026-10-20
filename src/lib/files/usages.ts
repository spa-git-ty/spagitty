// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * The name a Ctrl-click (⌘ on a Mac) lands on in Files (FEAT-154).
 *
 * The file is drawn as coloured runs, and a run is not a name: a plain run
 * holds spaces and punctuation as well. So the click is turned into a column
 * in the line, and the name is read out of the line's text around it.
 */

/** A run of name characters: letters, digits, `_` and `$`, in any script. */
const NAME = /[\p{L}\p{N}_$]+/gu;

/** The name at `column` of `text`, or null when the column is not on one. */
export function nameAt(text: string, column: number): string | null {
	for (const found of text.matchAll(NAME)) {
		const start = found.index;
		const end = start + found[0].length;
		if (column < start) return null;
		// The caret can land just past a name's last letter.
		if (column <= end) return /^\p{N}/u.test(found[0]) ? null : found[0];
	}
	return null;
}

interface CaretDocument {
	caretPositionFromPoint?: (x: number, y: number) => { offsetNode: Node; offset: number } | null;
	caretRangeFromPoint?: (x: number, y: number) => Range | null;
}

/**
 * Where in `element`'s text the point `x, y` falls, counted in UTF-16 units
 * as the line's text is; null when the browser cannot say, or the point is
 * not in the element.
 */
export function columnAt(element: HTMLElement, x: number, y: number): number | null {
	const doc = element.ownerDocument as Document & CaretDocument;
	let node: Node | null = null;
	let offset = 0;
	const position = doc.caretPositionFromPoint?.(x, y);
	if (position) {
		node = position.offsetNode;
		offset = position.offset;
	} else {
		const range = doc.caretRangeFromPoint?.(x, y);
		if (range) {
			node = range.startContainer;
			offset = range.startOffset;
		}
	}
	if (!node || node.nodeType !== Node.TEXT_NODE || !element.contains(node)) return null;
	const walker = doc.createTreeWalker(element, NodeFilter.SHOW_TEXT);
	let column = 0;
	for (let text = walker.nextNode(); text; text = walker.nextNode()) {
		if (text === node) return column + offset;
		column += text.textContent?.length ?? 0;
	}
	return null;
}

/**
 * The name a click on a line's text means: under the pointer when the
 * browser can place it, or else the coloured run clicked when that run is
 * a name and nothing else.
 */
export function nameClicked(event: MouseEvent, element: HTMLElement, text: string): string | null {
	const column = columnAt(element, event.clientX, event.clientY);
	if (column !== null) return nameAt(text, column);
	const run = event.target instanceof Element ? event.target.textContent?.trim() : undefined;
	return run && nameAt(run, 0) === run ? run : null;
}
