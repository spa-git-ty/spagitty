// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * How the graph sequences commits.
 *
 * A view preference, stored globally like density rather than per repository:
 * people who want GitKraken-style grouping want it on every checkout, and the
 * Graph screen has to know the answer before the walk starts.
 *
 * Changing it restarts the walk. Lanes are assigned as commits arrive, so a
 * drawing produced in one order cannot be rearranged into the other.
 */

import * as api from '../api';
import { repo } from '../repo.svelte';
import type { GraphOrder } from '../types';
import { graph } from './store.svelte';

export type { GraphOrder };

const KEY = 'spagitty.graph.order';

export const ORDERS: { id: GraphOrder; label: string; title: string }[] = [
	{
		id: 'date',
		label: 'by date',
		title: 'Newest first, like git log. Parallel histories mix by time.'
	},
	{
		id: 'branch',
		label: 'by branch',
		title: "Keep a branch's commits together, like git log --topo-order."
	}
];

function isOrder(value: string | null): value is GraphOrder {
	return value === 'date' || value === 'branch';
}

let chosen = $state<GraphOrder>('date');

export const graphOrder = {
	get id(): GraphOrder {
		return chosen;
	},

	async set(next: GraphOrder): Promise<void> {
		const changed = next !== chosen;
		chosen = next;
		try {
			localStorage.setItem(KEY, next);
		} catch {
			// It just won't persist. Not worth failing a paint over.
		}
		if (!changed || repo.info === null) return;
		await graph.adopt(await api.graphOrder(next));
	},

	init(): void {
		try {
			const stored = localStorage.getItem(KEY);
			if (isOrder(stored)) chosen = stored;
		} catch {
			// Private mode or a locked-down webview; the default stands.
		}
	}
};
