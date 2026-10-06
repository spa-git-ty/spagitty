<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { untrack } from 'svelte';
	import MergerPlan from '$lib/merger/MergerPlan.svelte';
	import { merger } from '$lib/merger/store.svelte';
	import { repo } from '$lib/repo.svelte';

	/**
	 * Merger (1S, FEAT-100): any two branches, and what merging them would do,
	 * before anything is written.
	 *
	 * The forecast is asked for again whenever the refs move — a commit, a
	 * fetch, a checkout elsewhere — because a plan for commits that are no
	 * longer the branches' tips is a plan for some other merge.
	 */
	$effect(() => {
		const info = repo.info;
		void repo.token;
		if (!info) return;
		untrack(() => merger.prime(info.path, info.head.branch ?? null));
	});
</script>

<div class="screen">
	{#if repo.info}
		<MergerPlan />
	{:else}
		<p class="note empty">Open a repository to merge its branches.</p>
	{/if}
</div>

<style>
	.screen {
		flex: 1;
		min-width: 0;
		min-height: 0;
		display: flex;
		flex-direction: column;
		position: relative;
		overflow: hidden;
	}

	.empty {
		padding: 20px;
	}
</style>
