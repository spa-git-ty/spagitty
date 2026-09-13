<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { goto } from '$app/navigation';
	import CommitDetail from '$lib/graph/CommitDetail.svelte';
	import CommitRows from '$lib/graph/CommitRows.svelte';
	import { graphOrder, ORDERS } from '$lib/graph/order.svelte';
	import { graph } from '$lib/graph/store.svelte';
	import { visibility, type Mode } from '$lib/graph/visibility.svelte';
	import { repo } from '$lib/repo.svelte';
	import Btn from '$lib/ui/Btn.svelte';
	import Chip from '$lib/ui/Chip.svelte';
	import Icon from '$lib/ui/Icon.svelte';
	import { panels } from '$lib/panels.svelte';
	import Splitter from '$lib/ui/Splitter.svelte';

	const branch = $derived(repo.info?.head.branch ?? null);

	/** Whether the commit detail panel is put away. */
	const detailHidden = $derived(panels.isHidden('detail'));

	/** What the chip says the walk is currently rooted at. */
	const SCOPE: Record<Mode, { label: string; title: string }> = {
		all: { label: 'all branches', title: 'Every local and remote branch is walked' },
		hide: { label: 'some branches hidden', title: 'Everything except the branches you hid' },
		solo: { label: 'soloed', title: 'Only the branches you soloed' },
		smart: {
			label: 'smart visibility',
			title: 'The checked-out branch, what it is based on, and their upstreams'
		}
	};

	const scope = $derived(SCOPE[visibility.mode]);

	function openDiff(id: string) {
		goto(`/diff?commit=${id}`);
	}
</script>

<div class="screen">
	<div class="column">
		<header class="head">
			<div class="left">
				<span class="title">Graph</span>
				{#if branch}<Chip active>{branch}</Chip>{/if}
				<Chip active={visibility.filtered} title={scope.title}>{scope.label}</Chip>
				<div class="orders" role="group" aria-label="Commit order">
					{#each ORDERS as option (option.id)}
						<Chip
							active={graphOrder.id === option.id}
							title={option.title}
							onclick={() => void graphOrder.set(option.id)}
						>
							{option.label}
						</Chip>
					{/each}
				</div>
			</div>
			<div class="right">
				<!--
					The detail panel had no way to go away (FEAT-054). It is a
					third of the window on a laptop, and reading a wide commit
					message meant dragging it shut and dragging it back.
				-->
				<button
					class="gear"
					aria-label={detailHidden ? 'Show the detail panel' : 'Hide the detail panel'}
					title={detailHidden ? 'Show the detail panel' : 'Hide the detail panel'}
					aria-pressed={!detailHidden}
					onclick={() => panels.toggleHidden('detail')}
				>
					<Icon name={detailHidden ? 'chevron-left' : 'chevron-right'} size="1.1em" />
				</button>
			</div>
		</header>

		{#if repo.info === null}
			<div class="empty">
				{#if repo.busy}
					<span class="note">Opening…</span>
				{:else}
					<p class="note">No repository open.</p>
					<Btn primary onclick={() => repo.choose()}>Open repository…</Btn>
					{#if repo.error}<p class="note error">{repo.error}</p>{/if}
				{/if}
			</div>
		{:else if graph.error && graph.count === 0}
			<div class="empty">
				<p class="note error">{graph.error}</p>
			</div>
		{:else if graph.count === 0}
			<div class="empty"><span class="note">Walking history…</span></div>
		{:else}
			<CommitRows onopen={openDiff} onwip={() => goto('/changes')} />
		{/if}

	</div>

	<!-- Both go together: a divider with nothing on one side of it resizes
	     nothing. -->
	{#if !detailHidden}
		<div class="detail-pane">
			<Splitter panel="detail" label="Resize the detail panel" />
			<CommitDetail onopen={openDiff} />
		</div>
	{/if}
</div>

<style>
	.screen {
		flex: 1;
		min-width: 0;
		display: flex;
		overflow: hidden;
	}

	/* Layout-neutral at normal sizes; it exists so responsive layout can treat
	   the inspector and its resize handle as one optional unit. */
	.detail-pane {
		display: contents;
	}

	.column {
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
		border-bottom: 1px solid color-mix(in srgb, var(--line) 55%, transparent);
		box-shadow: none;
		position: relative;
		z-index: 1;
		flex: none;
	}

	.left,
	.right {
		display: flex;
		align-items: center;
		gap: 6px;
	}

	.left {
		gap: 8px;
	}

	.orders {
		display: flex;
		align-items: center;
		gap: 6px;
	}

	.title {
		font-size: var(--fs-title);
	}

	/* `--r-chip` and `--dim` were never tokens — both silently resolved to
	   nothing, so these buttons had square corners and inherited colour. */
	.gear {
		display: grid;
		place-items: center;
		width: 28px;
		height: 28px;
		border: 1px solid transparent;
		border-radius: var(--r-button);
		background: none;
		color: var(--muted);
		cursor: pointer;
		transition:
			background var(--t-fast) var(--ease),
			color var(--t-fast) var(--ease),
			transform var(--t-fast) var(--spring);
	}

	.gear:hover {
		background: var(--hover);
		color: var(--accent);
	}

	.gear:active {
		transform: scale(0.92);
	}

	.empty {
		flex: 1;
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: 10px;
	}

	.error {
		color: var(--danger);
	}

	/* The commit remains reachable by opening it in Diff. At constrained
	   widths the graph itself is the primary surface and must not be crushed by
	   a permanent inspector. */
	@media (max-width: 900px) {
		.detail-pane {
			display: none;
		}
	}
</style>
