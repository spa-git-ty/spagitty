<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import AgentBadge from '../AgentBadge.svelte';
	import type { AgentDefinition } from '../../types';
	import { WAY_NAMES, plannerGets, wayDetail, wayOf, ways, type WizardState } from '../../wizard';

	/**
	 * Step 4 (FEAT-141): how the tasks are made, and exactly what the planner
	 * is given. Nothing runs until the person accepts the tasks.
	 */
	interface Props {
		wizard: WizardState;
		planners: AgentDefinition[];
		planner: string | null;
		hasPolicy: boolean;
		busy: boolean;
	}

	let { wizard = $bindable(), planners, planner = $bindable(), hasPolicy, busy }: Props = $props();

	const offered = $derived(ways(wizard));
	const chosen = $derived(wayOf(wizard));
	const chips = $derived(plannerGets(wizard, hasPolicy));
	const current = $derived(planners.find((p) => p.id === planner));
</script>

<div class="ways" role="radiogroup" aria-label="How the tasks are made">
	{#each offered as way (way)}
		{@const on = chosen === way}
		<div class="way" class:on>
			<button class="pick" role="radio" aria-checked={on} disabled={busy} onclick={() => (wizard.way = way)}>
				<span class="ring"><span class="pip"></span></span>
				<span class="text">
					<strong>{WAY_NAMES[way]}</strong>
					<span class="detail">{wayDetail(way, wizard.issues)}</span>
				</span>
			</button>
			{#if way === 'plan' && on && planners.length}
				<label class="planner">
					{#if current}<AgentBadge agent={current} />{/if}
					<select aria-label="Planner" bind:value={planner} disabled={busy}>
						{#each planners as agent (agent.id)}<option value={agent.id}>{agent.displayName}</option>{/each}
					</select>
				</label>
			{/if}
		</div>
	{/each}
</div>

{#if chosen === 'plan'}
	<div class="gets">
		<span class="small">The planner gets</span>
		<div class="chips">
			{#each chips as chip, i (i)}
				<span class="chip-tone {chip.tone}" class:mono={chip.mono}>{chip.label}</span>
			{/each}
		</div>
	</div>
{/if}

<style>
	.ways {
		display: flex;
		flex-direction: column;
		gap: 10px;
	}

	.way {
		display: flex;
		align-items: center;
		gap: 16px;
		padding-right: 20px;
		border-radius: var(--r-floating);
		border: 1.5px solid var(--soft);
		background: var(--surface);
	}

	.way.on {
		border-color: var(--accent);
		background: color-mix(in srgb, var(--accent) 8%, transparent);
	}

	.pick {
		flex: 1;
		min-width: 0;
		display: flex;
		align-items: center;
		gap: 16px;
		padding: 18px 20px;
		border: 0;
		background: none;
		color: var(--ink);
		font: inherit;
		text-align: left;
		cursor: pointer;
	}

	.ring {
		flex: none;
		width: 20px;
		height: 20px;
		box-sizing: border-box;
		display: grid;
		place-items: center;
		border-radius: 50%;
		border: 2px solid color-mix(in srgb, var(--ink) 35%, transparent);
	}

	.pip {
		width: 10px;
		height: 10px;
		border-radius: 50%;
	}

	.on .ring {
		border-color: var(--accent);
	}

	.on .pip {
		background: var(--accent);
	}

	.text {
		display: flex;
		flex-direction: column;
		gap: 5px;
		min-width: 0;
	}

	.text strong {
		font-size: calc(var(--fs-ui) * 1.05);
	}

	.detail {
		color: var(--muted);
		font-size: var(--fs-secondary);
		line-height: 1.45;
	}

	.planner {
		flex: none;
		display: inline-flex;
		align-items: center;
		gap: 8px;
	}

	.gets {
		display: flex;
		flex-direction: column;
		gap: 10px;
		margin-top: 4px;
	}

	.small {
		color: var(--muted);
		font-size: var(--fs-secondary);
	}

	.chips {
		display: flex;
		flex-wrap: wrap;
		gap: 8px;
	}

	.chip-tone {
		padding: 6px 12px;
		border-radius: var(--r-pill);
		font-size: var(--fs-secondary);
		background: var(--soft);
		color: color-mix(in srgb, var(--ink) 80%, transparent);
	}

	.chip-tone.goal {
		background: color-mix(in srgb, var(--accent) 12%, transparent);
		color: var(--ink);
	}

	.chip-tone.issue {
		background: color-mix(in srgb, var(--lane-5) 12%, transparent);
		color: var(--lane-5);
	}

	.chip-tone.brief {
		background: color-mix(in srgb, var(--warn) 12%, transparent);
		color: var(--warn);
	}

	.chip-tone.mono {
		font-family: var(--font-mono);
		font-size: var(--fs-mono);
	}
</style>
