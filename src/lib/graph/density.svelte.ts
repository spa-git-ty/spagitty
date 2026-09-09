// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * How much room the graph column asks for (TASK-041).
 *
 * A view preference, and it lives beside the theme and the zoom rather than
 * with the settings the backend owns — for the same reason those do: the Graph
 * screen lays itself out on the first frame, and the layout cannot wait on a
 * Tauri command to find out how wide its own column is.
 *
 * What the two settings actually mean is argued in `metrics.ts`, beside the
 * numbers. In short: at the design's own metrics a three-lane repository is
 * still given five lanes' worth of column, and on a 1280 window that column is
 * competing with the commit subject — which is what the screen is for.
 *
 * The preference is a name rather than the numbers. Storing `{pitch: 16, node:
 * 6}` would freeze one release's idea of compact into every installation that
 * ever chose it, and the numbers are exactly the sort of thing that gets retuned
 * once somebody looks at a screenshot.
 */

import { COMFORTABLE, COMPACT, type Density } from '$lib/metrics';

export type DensityId = 'comfortable' | 'compact';

const KEY = 'spagitty.graph.density';

const TABLE: Record<DensityId, Density> = {
	comfortable: COMFORTABLE,
	compact: COMPACT
};

export const DENSITIES: { id: DensityId; label: string; note: string }[] = [
	{ id: 'comfortable', label: 'Comfortable', note: 'author portraits on the nodes' },
	{ id: 'compact', label: 'Compact', note: 'narrower lanes, more room for messages' }
];

function isDensity(value: string | null): value is DensityId {
	return value === 'comfortable' || value === 'compact';
}

let chosen = $state<DensityId>('comfortable');

export const density = {
	get id(): DensityId {
		return chosen;
	},

	/** The three numbers the geometry is made of. */
	get current(): Density {
		return TABLE[chosen];
	},

	/**
	 * True while nodes are big enough to be a face.
	 *
	 * Read by the canvas rather than inferred from the radius there: "is a
	 * portrait worth drawing" is a question about the preference, and a painter
	 * comparing a radius against a magic number would be answering it by
	 * accident.
	 */
	get portraits(): boolean {
		return chosen === 'comfortable';
	},

	set(next: DensityId): void {
		chosen = next;
		try {
			localStorage.setItem(KEY, next);
		} catch {
			// It just won't persist. Not worth failing a paint over.
		}
	},

	init(): void {
		try {
			const stored = localStorage.getItem(KEY);
			if (isDensity(stored)) chosen = stored;
		} catch {
			// Private mode or a locked-down webview; the default stands.
		}
	}
};
