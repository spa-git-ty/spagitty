<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { isActive, REF_SCREENS } from '$lib/nav';
	import { repo } from '$lib/repo.svelte';
	import type { CountKey } from '$lib/nav';

	/**
	 * Branches, Tags, Stash and Reflog, as one place (TASK-045).
	 *
	 * They are four views of the same refs, and they were four rows of the
	 * rail. Now the rail has one row for all of them, and each of the four
	 * screens opens with this in place of its title: the screen you are on in
	 * full strength, the other three a click away, each with its count.
	 */
	const TABS: { href: (typeof REF_SCREENS)[number]; label: string; count?: CountKey }[] = [
		{ href: '/branches', label: 'Branches', count: 'branches' },
		{ href: '/tags', label: 'Tags', count: 'tags' },
		{ href: '/stash', label: 'Stash', count: 'stashes' },
		{ href: '/reflog', label: 'Reflog' }
	];

	function countOf(key: CountKey | undefined): string | null {
		if (!key) return null;
		const value = repo.counts[key];
		return value === null ? null : String(value);
	}
</script>

<div class="tabs" role="tablist" aria-label="Refs">
	{#each TABS as tab (tab.href)}
		{@const active = isActive(tab.href, page.url.pathname)}
		{@const count = countOf(tab.count)}
		<button
			class="tab"
			class:active
			role="tab"
			aria-selected={active}
			tabindex={active ? 0 : -1}
			onclick={() => {
				if (!active) goto(tab.href);
			}}
		>
			<span class="label">{tab.label}</span>
			{#if count !== null}<span class="count mono">{count}</span>{/if}
		</button>
	{/each}
</div>

<style>
	.tabs {
		display: flex;
		align-items: baseline;
		gap: 14px;
		min-width: 0;
	}

	.tab {
		display: inline-flex;
		align-items: baseline;
		gap: 5px;
		padding: 0;
		font-size: var(--fs-title);
		white-space: nowrap;
		color: var(--muted);
		transition: color var(--t-fast) var(--ease);
	}

	.tab:hover {
		color: var(--ink);
	}

	.tab.active {
		color: var(--ink);
		cursor: default;
	}

	.count {
		font-size: var(--fs-mono);
		color: var(--muted);
		font-variant-numeric: tabular-nums;
	}
</style>
