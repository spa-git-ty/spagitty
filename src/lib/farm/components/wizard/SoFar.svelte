<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import AgentBadge from '../AgentBadge.svelte';
	import type { AgentDefinition } from '../../types';
	import type { SoFarRow } from '../../wizard';

	/**
	 * **Your farm** (FEAT-141): a card that fills in as the person goes. The
	 * remembered crew and rules are dimmed and tagged `last time` until step 3
	 * has been seen.
	 */
	let { rows, crew }: { rows: SoFarRow[]; crew: AgentDefinition[] } = $props();
</script>

<aside class="card so-far" aria-labelledby="so-far">
	<h2 id="so-far">Your farm</h2>
	{#each rows as row (row.label)}
		<div class="row">
			<div class="head">
				<span class="label">{row.label}</span>
				{#if row.lastTime}<span class="last">last time</span>{/if}
			</div>
			<span class="value" class:dim={row.dim}>{row.value}</span>
			{#if row.badges && crew.length}
				<div class="badges" class:dim={row.dim}>
					{#each crew as agent (agent.id)}<AgentBadge {agent} />{/each}
				</div>
			{/if}
		</div>
	{/each}
	<p class="note">Crew and rules are remembered for this repository.</p>
</aside>

<style>
	.so-far {
		padding: 20px 22px;
		border-radius: var(--r-pane);
		display: flex;
		flex-direction: column;
		gap: 4px;
	}

	h2 {
		margin: 0 0 8px;
		font-size: var(--fs-ui);
		font-weight: 600;
	}

	.row {
		display: flex;
		flex-direction: column;
		gap: 5px;
		padding: 12px 0;
		border-top: 1px solid color-mix(in srgb, var(--line) 45%, transparent);
	}

	.head {
		display: flex;
		align-items: center;
		justify-content: space-between;
	}

	.label,
	.note {
		color: var(--muted);
		font-size: var(--fs-mono);
	}

	.last {
		padding: 2px 8px;
		border-radius: var(--r-pill);
		background: color-mix(in srgb, var(--lane-5) 14%, transparent);
		color: var(--lane-5);
		font-size: calc(var(--fs-mono) * 0.92);
	}

	.value {
		line-height: 1.4;
	}

	.dim {
		opacity: 0.55;
	}

	.badges {
		display: flex;
		gap: 3px;
		margin-top: 2px;
	}

	.note {
		margin: 8px 0 0;
		line-height: 1.5;
	}
</style>
