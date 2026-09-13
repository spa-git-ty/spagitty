<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { relativeTime } from '$lib/format';
	import { search } from '$lib/search/store.svelte';
	import RefChip from '$lib/ui/RefChip.svelte';

	/**
	 * The results.
	 *
	 * Deliberately not the graph's row component. That one is welded to the
	 * graph store's virtualisation and its lane canvas, and lanes are exactly
	 * what a filtered list must not have: they would draw edges between commits
	 * that are not parent and child. What is shared is the vocabulary — the
	 * initials glyph, the short id, the refs, the relative time — so a result
	 * still reads as a commit.
	 */
	interface Props {
		/** `↵` on the focused row. */
		onopen?: (id: string) => void;
		/** `Alt+Enter` on the focused row. */
		ondiff?: (id: string) => void;
	}

	let { onopen, ondiff }: Props = $props();

	const rows = $derived(search.rows());

	function activate(id: string, event: MouseEvent | KeyboardEvent) {
		search.select(id);
		if (event.altKey) ondiff?.(id);
		else onopen?.(id);
	}

	function keydown(id: string, event: KeyboardEvent) {
		if (event.key !== 'Enter') return;
		event.preventDefault();
		activate(id, event);
	}
</script>

<ol class="results">
	{#each rows as row (row.id)}
		<li>
			<button
				class="row"
				class:selected={row.id === search.selectedId}
				onclick={(event) => activate(row.id, event)}
				onkeydown={(event) => keydown(row.id, event)}
			>
				<span class="glyph" aria-hidden="true">{row.initials}</span>
				<span class="summary" title={row.summary}>{row.summary}</span>
				{#if row.refs.length > 0}
					<span class="refs">
						{#each row.refs as chip (chip.name)}
							<RefChip {chip} />
						{/each}
					</span>
				{/if}
				<span class="who note">{row.authorName}</span>
				<span class="when note">{relativeTime(row.time)}</span>
				<span class="sha mono note">{row.short}</span>
			</button>
		</li>
	{/each}
</ol>

<style>
	.results {
		margin: 0;
		padding: 0;
		list-style: none;
		display: flex;
		flex-direction: column;
	}

	.row {
		width: 100%;
		height: var(--row-pitch);
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 0 8px;
		border: none;
		border-radius: var(--r-field);
		background: transparent;
		color: inherit;
		font: inherit;
		text-align: left;
		min-width: 0;
	}

	.row:hover {
		background: var(--soft);
	}

	.row.selected {
		background: var(--selection);
	}

	.glyph {
		flex: none;
		width: 20px;
		height: 20px;
		border: 1px solid var(--line);
		border-radius: 50%;
		display: inline-flex;
		align-items: center;
		justify-content: center;
		font-size: var(--fs-secondary);
	}

	.summary {
		flex: 1;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.refs {
		flex: none;
		display: flex;
		gap: 4px;
	}

	.who,
	.when {
		flex: none;
		white-space: nowrap;
	}

	.sha {
		flex: none;
	}
</style>
