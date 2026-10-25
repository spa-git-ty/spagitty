<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import Icon from './Icon.svelte';
	import { speed } from './speedsearch.svelte';

	/**
	 * What has been typed into a list's search (FEAT-158), floating over the
	 * top right of the list it narrows. Mounted once by the shell; it shows
	 * nothing until a list is being searched.
	 *
	 * It is not a field and never takes the focus: the letters are caught
	 * where they are typed, so the list keeps its focus and its selection.
	 */

	const open = $derived(speed.open);
	const query = $derived(open?.options.search?.query ?? '');
	const count = $derived(open?.options.count?.() ?? null);
	const none = $derived(count !== null && count[0] === 0);

	let place = $state({ top: 0, right: 0 });

	$effect(() => {
		if (open === null) return;
		void query;
		const measure = () => {
			const rect = open.node.getBoundingClientRect();
			place = {
				top: Math.max(8, rect.top + 8),
				right: Math.max(8, window.innerWidth - rect.right + 12)
			};
		};
		measure();
		window.addEventListener('resize', measure);
		return () => window.removeEventListener('resize', measure);
	});
</script>

{#if open !== null && query !== ''}
	<div
		class="speed-search floating"
		class:none
		style:top="{place.top}px"
		style:right="{place.right}px"
		role="status"
		aria-live="polite"
	>
		<Icon name="search" size={13} weight={2} />
		<span class="label">{open.options.label ? `Search ${open.options.label}` : 'Search'}</span>
		<span class="query mono">{query}</span>
		{#if count !== null}
			<span class="count">{none ? 'no match' : `${count[0]} of ${count[1]}`}</span>
		{/if}
		<span class="keys" aria-hidden="true">esc</span>
	</div>
{/if}

<style>
	.speed-search {
		position: fixed;
		z-index: 60;
		display: flex;
		align-items: center;
		gap: 8px;
		max-width: min(420px, 70vw);
		padding: 5px 10px;
		border-radius: 999px;
		font-size: var(--fs-secondary);
		color: var(--muted);
		pointer-events: none;
		animation: rise-in var(--t-enter-liquid) var(--spring-liquid);
	}

	.label {
		flex: none;
	}

	.query {
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: pre;
		color: var(--ink);
		font-size: var(--fs-mono);
		padding: 1px 6px;
		border-radius: 6px;
		background: color-mix(in srgb, var(--accent) 16%, transparent);
	}

	.none .query {
		background: color-mix(in srgb, var(--danger) 16%, transparent);
	}

	.none .count {
		color: var(--danger);
	}

	.count {
		flex: none;
		font-variant-numeric: tabular-nums;
	}

	.keys {
		flex: none;
		font-size: 0.8em;
		padding: 0 5px;
		border: 1px solid var(--line);
		border-radius: 4px;
	}
</style>
