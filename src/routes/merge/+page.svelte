<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { untrack } from 'svelte';
	import { goto } from '$app/navigation';
	import LandDialog from '$lib/merger/LandDialog.svelte';
	import MergerPlan from '$lib/merger/MergerPlan.svelte';
	import MergerResolve from '$lib/merger/MergerResolve.svelte';
	import { resolving } from '$lib/merger/resolve.svelte';
	import { merger } from '$lib/merger/store.svelte';
	import { repo } from '$lib/repo.svelte';

	/**
	 * Merger (1S, FEAT-100): any two branches, and what merging them would do,
	 * before anything is written.
	 *
	 * The plan, resolving (FEAT-102), and the commit dialog over whichever it
	 * was opened from. The forecast is asked for again whenever the refs move
	 * — a commit, a fetch, a checkout elsewhere — because a plan for commits
	 * that are no longer the branches' tips is a plan for some other merge.
	 * Only on the plan: while resolving, committing or done, the screen is
	 * about the merge it already read, and landing refuses one whose branches
	 * moved.
	 */
	$effect(() => {
		const info = repo.info;
		void repo.token;
		if (!info) return;
		untrack(() => {
			if (merger.phase !== 'plan') return;
			void merger.prime(info.path, info.head.branch ?? null);
		});
	});

	const names = $derived({ a: merger.forecast?.a.name ?? 'A', b: merger.forecast?.b.name ?? 'B' });
	const fromResolve = $derived(merger.returnTo === 'resolve');

	function resolve(path: string | null) {
		if (merger.strategy === 'rebase') void resolving.openRebase();
		else if (repo.info) void resolving.open(repo.info.path, path);
	}

	/** Merge now: a rebase goes through its worktree, even with nothing in the way. */
	function mergeNow() {
		if (merger.strategy === 'rebase') void resolving.openRebase();
		else merger.openCommit('plan');
	}
</script>

<div class="screen">
	{#if repo.info}
		{#if merger.phase === 'resolve' || (merger.phase !== 'plan' && fromResolve)}
			<MergerResolve />
		{:else}
			<MergerPlan onresolve={resolve} onmerge={mergeNow} busy={merger.landing || resolving.loading} />
		{/if}
		{#if merger.phase === 'commit' || merger.phase === 'done'}
			<LandDialog
				rows={fromResolve && merger.phase === 'commit' ? resolving.summary(names) : []}
				oncommit={() => void (fromResolve && merger.strategy !== 'rebase' ? resolving.commit() : merger.land())}
				ongraph={() => void goto('/')}
			/>
		{/if}
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
