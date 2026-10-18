<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import AgentBadge from './AgentBadge.svelte';
	import { FARM_COPY as C, PROVIDER_LABELS, agentColour, availabilityLabel, strengths } from '../describe';
	import { farmStore } from '../store.svelte';
	import * as api from '../api';

	/**
	 * One toggle tile per detected agent (FEAT-111, FEAT-141): badge, name and
	 * what it is good at. Broken agents are drawn and cannot be put on the
	 * crew; missing ones are one quiet line. Toggling saves the agent, so the
	 * crew is remembered with the repository's agent registry.
	 */
	interface Props {
		busy: boolean;
		act: (message: string, run: () => Promise<unknown>) => Promise<boolean>;
	}

	let { busy, act }: Props = $props();

	const found = $derived(farmStore.agents.filter((a) => a.availability.state !== 'missing'));
	const missing = $derived([
		...farmStore.agents.filter((a) => a.availability.state === 'missing').map((a) => a.definition.displayName),
		...farmStore.undetected.map((provider) => PROVIDER_LABELS[provider])
	]);
</script>

<div class="tiles">
	{#each found as agent (agent.definition.id)}
		{@const d = agent.definition}
		{@const available = agent.availability.state === 'available'}
		{@const on = d.enabled && available}
		<button
			class="tile"
			class:off={!on}
			style:--agent-colour={agentColour(d)}
			aria-pressed={on}
			aria-label="{d.displayName} {C.onCrew.toLowerCase()}"
			disabled={busy || !available}
			onclick={() => act('Could not change the crew', () => api.saveAgent({ ...d, enabled: !d.enabled }))}
		>
			<AgentBadge agent={d} large />
			<span class="who">
				<strong>{d.displayName}</strong>
				{#if available}
					<span class="skills">{strengths(d)}</span>
				{:else}
					<span class="skills danger">{availabilityLabel(agent.availability)}</span>
				{/if}
			</span>
		</button>
	{:else}
		<p class="muted">No agents found on this machine yet.</p>
	{/each}
</div>
{#if missing.length}
	<p class="muted">Not found: {missing.join(', ')}.</p>
{/if}

<style>
	.tiles {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(190px, 1fr));
		gap: 10px;
	}

	.tile {
		display: flex;
		align-items: center;
		gap: 10px;
		padding: 12px 14px;
		text-align: left;
		color: var(--ink);
		font: inherit;
		border: 1.5px solid color-mix(in srgb, var(--agent-colour) 55%, transparent);
		border-radius: var(--r-floating);
		background: color-mix(in srgb, var(--agent-colour) 10%, transparent);
		cursor: pointer;
	}

	.tile.off {
		border-color: var(--soft);
		background: none;
		opacity: 0.55;
	}

	.tile:disabled {
		cursor: default;
	}

	.who {
		display: flex;
		flex-direction: column;
		gap: 2px;
		min-width: 0;
	}

	.skills {
		color: var(--muted);
		font-size: var(--fs-mono);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.danger {
		color: var(--danger);
	}

	p {
		margin: 0;
	}
</style>
