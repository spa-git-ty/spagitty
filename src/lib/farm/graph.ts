// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * Showing a farm's branch in the Graph (FEAT-110, FEAT-113).
 *
 * The graph selects by commit, not by name, so the branch is resolved to its
 * tip first and the graph is asked to select that commit once its walk
 * reaches it — the same `want` a reopened tab uses.
 */

import { goto } from '$app/navigation';
import { branches } from '$lib/api';
import { graph } from '$lib/graph/store.svelte';

/** Open the Graph on a branch's tip. Throws when the branch no longer exists. */
export async function openInGraph(branch: string): Promise<void> {
	const found = (await branches()).find((row) => row.name === branch);
	if (!found) throw new Error(`${branch} is not a branch any more.`);
	graph.want(found.id);
	await goto('/');
}
