<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { goto } from '$app/navigation';
	import { graph } from '$lib/graph/store.svelte';
	import { laneColorVar } from '$lib/metrics';
	import { age } from './needs';

	/**
	 * Latest commits (FEAT-140): seven rows from the graph's own walk, so each
	 * dot is its lane's colour in the Graph.
	 */
	let { now }: { now: number } = $props();

	const commits = $derived.by(() => {
		void graph.version;
		const rows = [];
		for (let i = 0; i < Math.min(7, graph.count); i++) {
			const row = graph.row(i);
			if (row) rows.push(row);
		}
		return rows;
	});
</script>

<section class="card home-card" aria-labelledby="home-commits">
	<div class="home-card-head">
		<h2 id="home-commits">Latest commits</h2>
		<button class="home-link" onclick={() => goto('/')}>Graph</button>
	</div>
	<div class="commits">
		{#if commits.length}<span class="spine" aria-hidden="true"></span>{/if}
		{#each commits as row (row.id)}
			<div class="commit">
				<span class="node" style:--lane="var({laneColorVar(row.color)})"></span>
				<span class="text">
					<span class="mono short">{row.short}</span>
					<span class="home-ellipsis">{row.summary}</span>
				</span>
				<span class="home-quiet">{age(row.time * 1000, now)}</span>
			</div>
		{:else}
			<p class="home-quiet">{graph.error ?? 'Walking history…'}</p>
		{/each}
	</div>
</section>

<style>
	.commits {
		position: relative;
	}

	.spine {
		position: absolute;
		left: 6px;
		top: 14px;
		bottom: 14px;
		width: 2px;
		background: var(--soft);
	}

	.commit {
		position: relative;
		display: grid;
		grid-template-columns: 14px minmax(0, 1fr) auto;
		align-items: center;
		gap: 12px;
		padding: 7px 0;
	}

	.node {
		width: 12px;
		height: 12px;
		margin-left: 1px;
		box-sizing: border-box;
		border-radius: 50%;
		background: var(--surface);
		border: 2.5px solid var(--lane);
	}

	.text {
		min-width: 0;
		display: flex;
		align-items: baseline;
		gap: 10px;
		font-size: var(--fs-secondary);
	}

	.short {
		flex: none;
		color: var(--muted);
		font-size: var(--fs-mono);
	}

	p {
		margin: 0;
	}
</style>
