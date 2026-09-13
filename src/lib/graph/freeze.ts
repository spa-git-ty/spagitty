// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * The commit-list pane: everything from the message column onward.
 *
 * GitKraken keeps that pane fixed while the graph slides under it. The freeze
 * index is the first column that belongs to the pane; columns before it — the
 * refs gutter and the graph — are what pan.
 */

export function freezeAt(shown: { id: string }[]): number {
	const index = shown.findIndex((column) => column.id === 'message');
	return index < 0 ? shown.length : index;
}

/**
 * Left edge of the frozen pane, in CSS pixels from the viewport's left.
 *
 * At rest it sits where the message column naturally starts. Once the graph
 * has been panned far enough that the pane would leave through the right
 * edge, it pins there so the commit list stays on screen.
 */
export function frozenLeft(
	scrollingWidth: number,
	frozenPackWidth: number,
	scrollLeft: number,
	viewportWidth: number
): number {
	return Math.max(scrollingWidth - scrollLeft, viewportWidth - frozenPackWidth);
}
