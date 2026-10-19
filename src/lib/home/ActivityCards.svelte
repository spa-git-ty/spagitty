<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { goto } from '$app/navigation';
	import type { Release } from '$lib/releases/types';
	import type { Run } from '$lib/pipelines/types';
	import Btn from '$lib/ui/Btn.svelte';
	import { age } from './needs';
	import { heatmap, nextVersion, unreleasedLine, workflows } from './shape';
	import type { Overview } from './types';

	/**
	 * Home's last row (FEAT-140): the last sixteen weeks, the pipelines and
	 * the releases. One component, because
	 * each is small and they share the overview they are drawn from. `card`
	 * picks one, since Home places each where its settings say (FEAT-148).
	 */
	interface Props {
		card: 'heat' | 'pipelines' | 'releases';
		overview: Overview | null;
		runs: Run[];
		latest: Release | null;
		pipelinesError: string;
		hasHost: boolean;
		now: number;
	}

	let { card, overview, runs, latest, pipelinesError, hasHost, now }: Props = $props();

	const map = $derived(heatmap(overview?.commitTimes ?? [], now));
	const pipes = $derived(workflows(runs));
	const running = $derived(runs.filter((r) => r.state !== 'completed').length);
	const next = $derived(nextVersion(overview?.version ?? null, overview?.unreleased ?? null));
	const newest = $derived(
		[...runs].sort((a, b) => (b.ended || b.started) - (a.ended || a.started))[0] ?? null
	);
	const versionAge = $derived(overview?.versionTime ? age(overview.versionTime * 1000, now) : null);
</script>

{#if card === 'heat'}
	<section class="card home-card roomy" aria-labelledby="home-heat">
		<div class="home-card-head">
			<h2 id="home-heat">Commits, last 16 weeks</h2>
			<span class="note">{map.total} {map.total === 1 ? 'commit' : 'commits'}</span>
		</div>
		<div class="heat" role="img" aria-label="Commit activity, last 16 weeks: {map.total} commits">
			{#each map.weeks as week, w (w)}
				<div class="week">
					{#each week as day (day.day)}
						<span class="day l{day.level}" title="{new Date(day.day).toLocaleDateString()}: {day.count}"></span>
					{/each}
				</div>
			{/each}
		</div>
		<div class="legend home-quiet">
			<span>{map.busiest ? `Busiest on ${map.busiest}` : 'Nothing committed'}</span>
			<span class="steps">less <span class="day l0"></span><span class="day l1"></span><span class="day l2"></span><span class="day l3"></span><span class="day l4"></span> more</span>
		</div>
	</section>
{:else if card === 'pipelines'}
	<section class="card home-card roomy" aria-labelledby="home-pipes">
		<div class="home-card-head">
			<h2 id="home-pipes">Pipelines</h2>
			{#if running}<span class="note running">{running} running</span>{/if}
		</div>
		{#if pipelinesError}
			<p class="home-error">{pipelinesError}</p>
		{:else if !hasHost}
			<p class="home-quiet">No hosted remote with pipelines.</p>
		{:else if !pipes.length}
			<p class="home-quiet">No runs on {overview?.defaultBranch ?? 'the default branch'} yet.</p>
		{/if}
		{#each pipes as pipe (pipe.name)}
			<div class="pipe">
				<span class="mono home-ellipsis">{pipe.name}</span>
				<div class="bars" role="img" aria-label="{pipe.name}: last {pipe.runs.length} runs">
					{#each pipe.runs as run (run.id)}
						<span class="bar {run.state}" style:height="{run.height}%"></span>
					{/each}
				</div>
			</div>
		{/each}
		{#if newest}
			<div class="home-quiet">
				Newest on the right · <span class="mono">{newest.name}</span> on <span class="mono">{newest.branch}</span>, {age((newest.ended || newest.started) * 1000, now)}
			</div>
		{/if}
		<button class="home-link left" onclick={() => goto('/pipelines/live')}>Open Pipelines</button>
	</section>
{:else}
	<section class="card home-card roomy" aria-labelledby="home-releases">
		<div class="home-card-head">
			<h2 id="home-releases">Releases</h2>
			<button class="home-link" onclick={() => goto('/releases')}>All</button>
		</div>
		<div class="version">
			<span class="mono tag">{latest?.tag ?? overview?.version ?? 'No release yet'}</span>
			{#if latest}
				<span class="home-quiet">latest · {latest.assets.length} {latest.assets.length === 1 ? 'file' : 'files'}</span>
			{:else if versionAge}
				<span class="home-quiet">tagged {versionAge} ago</span>
			{/if}
		</div>
		{#if overview?.unreleased}
			<div class="unreleased">
				<span><strong>{overview.unreleased.total}</strong> {overview.unreleased.total === 1 ? 'entry' : 'entries'} under Unreleased</span>
				{#if unreleasedLine(overview.unreleased)}<span class="home-quiet">{unreleasedLine(overview.unreleased)}</span>{/if}
			</div>
		{:else}
			<p class="home-quiet">No <span class="mono">## [Unreleased]</span> in CHANGELOG.md.</p>
		{/if}
		{#if next}
			<div><Btn onclick={() => goto('/releases')}>Draft {next}</Btn></div>
		{/if}
	</section>
{/if}

<style>
	.heat {
		display: grid;
		grid-template-columns: repeat(16, minmax(0, 1fr));
		gap: 4px;
	}

	.week {
		display: grid;
		grid-template-rows: repeat(7, minmax(0, 1fr));
		gap: 4px;
	}

	.day {
		display: block;
		aspect-ratio: 1 / 1;
		border-radius: 3px;
	}

	/* Five steps of the accent, from the soft ground to the accent itself. */
	.l0 {
		background: var(--soft);
	}
	.l1 {
		background: color-mix(in srgb, var(--accent) 30%, transparent);
	}
	.l2 {
		background: color-mix(in srgb, var(--accent) 55%, transparent);
	}
	.l3 {
		background: color-mix(in srgb, var(--accent) 80%, transparent);
	}
	.l4 {
		background: var(--accent);
	}

	.legend {
		display: flex;
		justify-content: space-between;
		align-items: center;
	}

	.steps {
		display: inline-flex;
		align-items: center;
		gap: 4px;
	}

	.steps .day {
		width: 10px;
		height: 10px;
	}

	.running {
		color: var(--warn);
	}

	.pipe {
		display: grid;
		grid-template-columns: 96px minmax(0, 1fr);
		align-items: center;
		gap: 12px;
		font-size: var(--fs-secondary);
	}

	.bars {
		display: grid;
		grid-template-columns: repeat(14, minmax(0, 1fr));
		gap: 3px;
		align-items: end;
		height: 22px;
	}

	.bar {
		display: block;
		border-radius: 2px;
		background: var(--ok);
	}

	.bar.failed {
		background: var(--danger);
	}

	.bar.running {
		background: var(--warn);
		box-shadow: 0 0 8px color-mix(in srgb, var(--warn) 60%, transparent);
	}

	.bar.other {
		background: var(--soft);
	}

	.left {
		align-self: flex-start;
	}

	.version {
		display: flex;
		align-items: baseline;
		gap: 12px;
		flex-wrap: wrap;
	}

	.tag {
		font-size: calc(var(--fs-title) * 1.35);
		font-weight: 500;
	}

	.unreleased {
		display: flex;
		flex-direction: column;
		gap: 6px;
		padding: 12px 14px;
		border-radius: var(--r-panel);
		background: var(--soft);
		font-size: var(--fs-secondary);
	}

	strong {
		font-weight: 600;
	}

	p {
		margin: 0;
	}

	@media (prefers-reduced-motion: reduce) {
		.bar.running {
			box-shadow: none;
		}
	}
</style>
