<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import * as api from '$lib/api';
	import BlameStrip from '$lib/search/BlameStrip.svelte';
	import QueryBar from '$lib/search/QueryBar.svelte';
	import ResultDetail from '$lib/search/ResultDetail.svelte';
	import ResultRows from '$lib/search/ResultRows.svelte';
	import { search } from '$lib/search/store.svelte';

	/**
	 * Find commits by author, path, message or date, and see who last touched
	 * each line.
	 *
	 * Results stream as the walk finds them, so the first match appears long
	 * before the walk reaches the end of history.
	 */

	/** `Ctrl+F` lands here with `?focus=1`, which is what focuses the first field. */
	const focused = $derived(page.url.searchParams.get('focus') === '1');

	onMount(() => {
		if (!api.inTauri()) return;
		let detach: (() => void) | null = null;
		search.attach().then((off) => (detach = off));

		// Leaving cancels the walk. A query nobody is watching is a thread
		// reading history for no reason, and coming back to half a result set
		// from a question asked five minutes ago is worse than coming back
		// empty.
		return () => {
			detach?.();
			search.stop();
		};
	});

	/** `↵` — read the commit, beside the results. */
	function openCommit(id: string) {
		search.select(id);
	}

	/** `Alt+Enter` — its hunks, which are a different question and a different screen. */
	function openDiff(id: string) {
		search.select(id);
		goto(`/diff?commit=${id}`);
	}
</script>

<div class="screen">
	<header class="head">
		<div class="left">
			<span class="title">Log</span>
			{#if search.ran}
				<span class="note">
					{search.count}
					{search.count === 1 ? 'result' : 'results'}{search.running ? '…' : ''}
				</span>
			{/if}
		</div>
	</header>

	<div class="bar">
		<QueryBar autofocus={focused} />
	</div>

	<div class="body">
		<div class="results">
			{#if search.error}
				<p class="note error">{search.error}</p>
			{:else if !search.ran}
				<p class="note">
					Search by author, message, path or date. Filters combine — every one you add
					narrows what is left.
				</p>
			{:else if search.count === 0 && !search.running}
				<p class="note">
					Nothing matched{#if search.narrowestApplied}, and
					<span class="mono">{search.narrowestApplied}</span> is the narrowest filter
					applied{/if}.
				</p>
			{:else}
				<ResultRows onopen={openCommit} ondiff={openDiff} />
				{#if search.running}
					<p class="note working">Still walking…</p>
				{/if}
			{/if}
		</div>

		<div class="side">
			<ResultDetail ondiff={openDiff} />
			<BlameStrip />
		</div>
	</div>

</div>

<style>
	.screen {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
		overflow: hidden;
	}

	.head {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 10px;
		padding: 10px 12px;
		background-color: var(--chrome-veil);
		border-bottom: 1px solid var(--band-rule, color-mix(in srgb, var(--line) 55%, transparent));
		box-shadow: none;
		position: relative;
		z-index: 1;
		flex: none;
	}

	.left {
		display: flex;
		align-items: baseline;
		gap: 8px;
		min-width: 0;
	}

	.title {
		font-size: var(--fs-title);
	}

	.bar {
		flex: none;
		padding: 8px 12px;
		background-color: var(--chrome-veil);
		border-bottom: 1px solid var(--band-rule, color-mix(in srgb, var(--line) 55%, transparent));
		box-shadow: none;
		position: relative;
		z-index: 1;
	}

	.body {
		flex: 1;
		min-height: 0;
		display: flex;
		min-width: 0;
	}

	.results {
		flex: 1;
		min-width: 0;
		overflow: auto;
		padding: 8px 12px;
	}

	/* The design's 280px column: what was opened above, blame below. */
	.side {
		width: var(--search-side-w);
		flex: none;
		display: flex;
		flex-direction: column;
		min-height: 0;
		border-left: 1px solid var(--soft);
	}

	.working {
		padding: 6px 8px;
	}
</style>
