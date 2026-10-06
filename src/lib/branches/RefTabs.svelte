<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { page } from '$app/state';
	import { isActive, REF_SCREENS } from '$lib/nav';
	import { repo } from '$lib/repo.svelte';
	import type { CountKey } from '$lib/nav';

	/**
	 * The title of Branches, Tags, Stash or Reflog, with its count.
	 *
	 * TASK-045 made the four one place, each screen naming all four as tabs.
	 * TASK-056 gave each its own row on the rail again, and the author asked
	 * for the tabs to go with that (2026-10-06): the rail is where you move
	 * between them, so each screen says only what it is.
	 */
	const TITLES: { href: (typeof REF_SCREENS)[number]; label: string; count?: CountKey }[] = [
		{ href: '/branches', label: 'Branches', count: 'branches' },
		{ href: '/tags', label: 'Tags', count: 'tags' },
		{ href: '/stash', label: 'Stash', count: 'stashes' },
		{ href: '/reflog', label: 'Reflog' }
	];

	const title = $derived(TITLES.find((entry) => isActive(entry.href, page.url.pathname)) ?? TITLES[0]);
	const count = $derived.by(() => {
		if (!title.count) return null;
		const value = repo.counts[title.count];
		return value === null ? null : String(value);
	});
</script>

<h1 class="title">
	<span class="label">{title.label}</span>
	{#if count !== null}<span class="count mono">{count}</span>{/if}
</h1>

<style>
	.title {
		display: inline-flex;
		align-items: baseline;
		gap: 5px;
		margin: 0;
		font-size: var(--fs-title);
		font-weight: 400;
		white-space: nowrap;
		color: var(--ink);
	}

	.count {
		font-size: var(--fs-mono);
		color: var(--muted);
		font-variant-numeric: tabular-nums;
	}
</style>
