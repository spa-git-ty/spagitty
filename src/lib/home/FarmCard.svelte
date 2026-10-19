<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { goto } from '$app/navigation';
	import AgentBadge from '$lib/farm/components/AgentBadge.svelte';
	import { FARM_STATUS_LABELS, SEGMENTS, segment } from '$lib/farm/describe';
	import { farmStore } from '$lib/farm/store.svelte';
	import type { Farm } from '$lib/farm/types';

	/**
	 * The Farm card (FEAT-140): the goal, how far it has got in the same
	 * segments Building draws, and the crew with a halo on the busy ones. With
	 * no farm, a quiet way to start one.
	 */
	let { farm, onstart }: { farm: Farm | null; onstart: () => void } = $props();

	const tasks = $derived(farmStore.tasks);
	const leaves = $derived(tasks.filter((t) => t.status !== 'cancelled' && !tasks.some((c) => c.parent === t.id)));
	const needIds = $derived(new Set(farmStore.needsYou.map((t) => t.id)));
	const segments = $derived(
		leaves
			.map((task) => segment(task, farmStore.details[task.id], needIds.has(task.id)))
			.sort((a, b) => SEGMENTS.findIndex((s) => s.id === a) - SEGMENTS.findIndex((s) => s.id === b))
	);
	const merged = $derived(leaves.filter((t) => t.status === 'done').length);
	const busy = $derived(new Set(farmStore.runs.filter((r) => r.outcome.state === 'running').map((r) => r.agent)));
	const crew = $derived(farmStore.usable.map((a) => a.definition));
	const label = $derived(
		segments
			.reduce<string[]>((parts, id) => {
				const name = SEGMENTS.find((s) => s.id === id)?.label ?? id;
				const n = segments.filter((s) => s === id).length;
				const part = `${n} ${name}`;
				return parts.includes(part) ? parts : [...parts, part];
			}, [])
			.join(', ')
	);
</script>

<section class="card home-card roomy" aria-labelledby="home-farm">
	<div class="home-card-head">
		<h2 id="home-farm">Farm</h2>
		{#if farm}<span class="note">{FARM_STATUS_LABELS[farm.status]}</span>{/if}
	</div>

	{#if farm}
		<div class="goal">{farm.goal.title}</div>
		<div class="progress">
			<div class="count">
				<span><strong>{merged}</strong> <span class="home-quiet">of {leaves.length} merged</span></span>
				<span class="home-quiet">{leaves.length - merged} to go</span>
			</div>
			<div class="segments" role="img" aria-label={label || 'No tasks yet'} style:--n={Math.max(1, segments.length)}>
				{#each segments as id, i (i)}
					<span style:background={SEGMENTS.find((s) => s.id === id)?.colour}></span>
				{/each}
			</div>
		</div>
		<div class="foot">
			<div class="crew">
				{#each crew as agent (agent.id)}
					<AgentBadge {agent} running={busy.has(agent.id)} />
				{/each}
			</div>
			<button class="home-link" onclick={() => goto('/farm')}>Open the farm →</button>
		</div>
	{:else}
		<p class="home-quiet">No farm in this repository yet.</p>
		<button class="home-link start" onclick={onstart}>Start a farm</button>
	{/if}
</section>

<style>
	.goal {
		font-size: calc(var(--fs-ui) * 1.1);
		font-weight: 500;
		line-height: 1.3;
	}

	.progress {
		display: flex;
		flex-direction: column;
		gap: 8px;
	}

	.count {
		display: flex;
		justify-content: space-between;
		align-items: baseline;
	}

	strong {
		font-size: var(--fs-title);
		font-weight: 600;
	}

	.segments {
		display: grid;
		grid-template-columns: repeat(var(--n), minmax(0, 1fr));
		gap: 4px;
	}

	.segments span {
		height: 6px;
		border-radius: 3px;
	}

	.foot {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 10px;
	}

	.crew {
		display: flex;
		gap: 4px;
	}

	.start {
		align-self: flex-start;
		font-size: var(--fs-secondary);
	}

	p {
		margin: 0;
	}
</style>
