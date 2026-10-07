// SPDX-License-Identifier: GPL-3.0-or-later

import type { GraphRow } from '../types';

/**
 * The Branch / Tag column, fitted to its names (BUG-053).
 *
 * Double-clicking the column's divider widens it to the widest row's labels,
 * the way a spreadsheet fits a column — the author asked for it after reading
 * branch names cut short. Every row walked so far counts, not only the ones on
 * screen, so scrolling does not reveal a longer name the fit ignored.
 */
export interface RefsFit {
	/** Width of a label's text, in CSS pixels. */
	measure: (text: string) => number;
	/** What a chip adds round its name: padding, border, marks. */
	chipExtra: number;
	/** How many chips a row shows before `+N`. */
	maxChips: number;
	/** Gap between two chips. */
	gap: number;
	/** The cell's own padding, plus the shortest lead to the graph. */
	frame: number;
	/** Never narrower than this. */
	min: number;
	/** Never wider than this: one absurd name must not take the screen. */
	max: number;
}

export function refsFitWidth(rows: Iterable<GraphRow>, fit: RefsFit): number {
	const widths = new Map<string, number>();
	const width = (text: string): number => {
		let known = widths.get(text);
		if (known === undefined) {
			known = fit.measure(text);
			widths.set(text, known);
		}
		return known;
	};

	let widest = 0;
	for (const row of rows) {
		if (row.refs.length === 0) continue;
		const shown = row.refs.slice(0, fit.maxChips);
		let total = fit.frame + fit.gap * (shown.length - 1);
		for (const chip of shown) total += width(chip.name) + fit.chipExtra;
		const extra = row.refs.length - fit.maxChips;
		if (extra > 0) total += fit.gap + width(`+${extra}`);
		widest = Math.max(widest, total);
	}

	if (widest === 0) return fit.min;
	return Math.round(Math.min(fit.max, Math.max(fit.min, widest)));
}
