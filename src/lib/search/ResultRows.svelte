<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { relativeTime } from '$lib/format';
	import { search } from '$lib/search/store.svelte';
	import RefChip from '$lib/ui/RefChip.svelte';
	import AuthorAvatar from '$lib/graph/AuthorAvatar.svelte';

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
				<span class="face"><AuthorAvatar email={row.authorEmail} name={row.authorName} letters={row.initials} /></span>
				<span class="text">
					<span class="line">
						<span class="summary" title={row.summary}>{row.summary}</span>
						{#if row.refs.length > 0}
							<span class="refs">
								{#each row.refs as chip (chip.name)}
									<RefChip {chip} />
								{/each}
							</span>
						{/if}
					</span>
					<span class="meta note">
						<span class="who">{row.authorName}</span>
						<span aria-hidden="true">·</span>
						<span class="when">{relativeTime(row.time)}</span>
					</span>
				</span>
				<span class="sha mono">{row.short}</span>
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
		gap: 2px;
	}

	.row {
		width: 100%;
		display: flex;
		align-items: center;
		gap: 12px;
		padding: 8px 12px;
		border-radius: 12px;
		text-align: left;
		min-width: 0;
	}

	.row:hover {
		background: var(--hover);
	}

	.row.selected {
		background: color-mix(in srgb, var(--accent) 14%, transparent);
		box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--accent) 40%, transparent);
	}

	.face {
		flex: none;
		display: grid;
		place-items: center;
		width: 28px;
		height: 28px;
	}

	.text {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
		gap: 2px;
	}

	.line {
		display: flex;
		align-items: center;
		gap: 8px;
		min-width: 0;
	}

	.summary {
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		font-weight: 550;
	}

	.refs {
		flex: none;
		display: flex;
		gap: 4px;
	}

	.meta {
		display: flex;
		gap: 6px;
		font-size: var(--fs-mono);
		white-space: nowrap;
	}

	.sha {
		flex: none;
		font-size: var(--fs-mono);
		color: var(--muted);
		padding: 2px 8px;
		border-radius: var(--r-pill);
		background: color-mix(in srgb, var(--sunken) 70%, transparent);
	}
</style>
