<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { untrack } from 'svelte';
	import { goto } from '$app/navigation';
	import Btn from '$lib/ui/Btn.svelte';
	import Icon from '$lib/ui/Icon.svelte';
	import AgentBadge from './AgentBadge.svelte';
	import AddAgentSheet from './AddAgentSheet.svelte';
	import RulesEditor from './RulesEditor.svelte';
	import {
		FARM_COPY as C,
		PROVIDER_LABELS,
		agentColour,
		agentName,
		availabilityLabel,
		strengths
	} from '../describe';
	import { farmStore } from '../store.svelte';
	import * as api from '../api';
	import type { FarmSettings } from '../types';

	/**
	 * No farm yet: crew, goal and rules on one page (FEAT-111).
	 *
	 * Replaces the old starter and the Settings pane. `Plan it` creates the
	 * farm, configures it with the rules on this page, then asks the planner —
	 * in that order, and stopping at the first step that fails.
	 */
	interface Props {
		busy: boolean;
		act: (message: string, run: () => Promise<unknown>) => Promise<boolean>;
		/** A goal carried over from Wrap up's `Start a farm with it`. */
		title?: string;
		description?: string;
		oncreated: () => void;
	}

	let {
		busy,
		act,
		title: initialTitle = '',
		description: initialDescription = '',
		oncreated
	}: Props = $props();

	let title = $state(untrack(() => initialTitle));
	let description = $state(untrack(() => initialDescription));
	let adding = $state(false);
	let settings = $state<FarmSettings>({
		autonomy: 'semiAuto',
		maxParallel: 2,
		maxAttempts: 3,
		verification: []
	});

	/** Everything detected, broken ones included; missing ones get one quiet line. */
	const found = $derived(farmStore.agents.filter((a) => a.availability.state !== 'missing'));
	const missing = $derived([
		...farmStore.agents
			.filter((a) => a.availability.state === 'missing')
			.map((a) => a.definition.displayName),
		...farmStore.undetected.map((provider) => PROVIDER_LABELS[provider])
	]);
	const planner = $derived(
		farmStore.usable.find((a) => a.definition.capabilities.includes('planning'))
	);

	async function create(plan: boolean) {
		if (!title.trim()) return;
		const created = await act('Could not start the farm', async () => {
			await api.create(title.trim(), description.trim());
			await api.configure({ ...settings, agents: farmStore.usable.map((a) => a.definition.id) });
			if (plan) await api.plan(planner?.definition.id ?? null);
		});
		if (created) oncreated();
	}
</script>

<div class="setup">
	<section class="intro">
		<h2>{C.setupTitle}</h2>
	</section>

	<section class="card part">
		<header>
			<span class="number">1</span>
			<div>
				<h3>{C.crew}</h3>
			</div>
			<Btn disabled={busy} onclick={() => act('Could not look for agents', api.detectAgents)}>
				<Icon name="refresh" />
				{C.lookAgain}
			</Btn>
			<Btn onclick={() => (adding = true)}>{C.addCli}</Btn>
			<!-- The machine's agents are set up once, in Settings › Agents (2.0);
			     the crew is chosen from them here. -->
			<Btn title="Every agent on this machine, and what each may do" onclick={() => goto('/settings#agents')}>
				Settings › Agents
			</Btn>
		</header>
		<div class="tiles">
			{#each found as agent (agent.definition.id)}
				{@const d = agent.definition}
				{@const available = agent.availability.state === 'available'}
				<label
					class="tile"
					class:off={!d.enabled || !available}
					style:--agent-colour={agentColour(d)}
				>
					<div class="tile-head">
						<AgentBadge agent={d} large />
						<div class="who">
							<strong>{d.displayName}</strong>
							{#if agent.availability.state !== 'missing'}
								<span class="mono muted path">{agent.availability.path}</span>
							{/if}
						</div>
						<input
							type="checkbox"
							aria-label="{d.displayName} {C.onCrew.toLowerCase()}"
							checked={d.enabled && available}
							disabled={busy || !available}
							onchange={(e) =>
								act('Could not change the crew', () =>
									api.saveAgent({ ...d, enabled: e.currentTarget.checked })
								)}
						/>
					</div>
					{#if available}
						<span class="state">
							<span class="dot ready"></span>
							{d.enabled ? C.onTheCrew : C.leftOut}
						</span>
					{:else}
						<span class="state danger">{availabilityLabel(agent.availability)}</span>
					{/if}
					<span class="muted">{strengths(d)}</span>
				</label>
			{/each}
		</div>
		{#if missing.length}
			<p class="muted">Not found: {missing.join(', ')}.</p>
		{/if}
	</section>

	<section class="card part">
		<header>
			<span class="number">2</span>
			<div>
				<h3>{C.goal}</h3>
			</div>
		</header>
		<label>
			{C.goalQuestion}
			<!-- One sentence, so Enter does what the bar's primary button does. -->
			<input
				class="goal"
				bind:value={title}
				onkeydown={(event) => {
					if (event.key === 'Enter' && title.trim() && !busy) void create(!!planner);
				}}
			/>
		</label>
		<label>{C.goalKnow}<textarea bind:value={description}></textarea></label>
	</section>

	<section class="card part">
		<header>
			<span class="number">3</span>
			<div>
				<h3>{C.rules}</h3>
			</div>
		</header>
		<RulesEditor bind:settings {busy} {act} />
	</section>

	<footer class="bar ornament">
		<Btn disabled={busy || !title.trim()} onclick={() => create(false)}>{C.manualTasks}</Btn>
		<Btn primary disabled={busy || !title.trim() || !planner} onclick={() => create(true)}>
			Plan it with {agentName(planner?.definition.id, farmStore.agents)}
		</Btn>
	</footer>
</div>

{#if adding}
	<AddAgentSheet {busy} {act} onclose={() => (adding = false)} />
{/if}

<style>
	.setup {
		max-width: 1020px;
		margin: 0 auto;
		padding-top: 16px;
		display: flex;
		flex-direction: column;
		gap: 18px;
	}

	.intro h2 {
		font-size: calc(var(--fs-title) * 1.6);
		margin-bottom: 8px;
	}

	.part {
		padding: 18px;
		display: flex;
		flex-direction: column;
		gap: 14px;
	}

	.part header {
		display: flex;
		align-items: center;
		gap: 12px;
	}

	.part header > div {
		flex: 1;
	}

	.part h3 {
		font-size: calc(var(--fs-ui) * 1.2);
		margin: 0;
	}

	.number {
		display: grid;
		place-items: center;
		width: 26px;
		height: 26px;
		flex-shrink: 0;
		border-radius: 50%;
		color: var(--accent);
		background: var(--accent-soft);
		font-size: var(--fs-secondary);
	}

	.tiles {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(210px, 1fr));
		gap: 10px;
	}

	.tile {
		display: flex;
		flex-direction: column;
		gap: 10px;
		padding: 14px;
		font-size: var(--fs-ui);
		border: 1px solid color-mix(in srgb, var(--agent-colour) 70%, transparent);
		border-radius: var(--r-panel);
		background: color-mix(in srgb, var(--agent-colour) 12%, transparent);
		cursor: pointer;
	}

	.tile.off {
		border-color: var(--soft);
		background: none;
		opacity: 0.7;
	}

	.tile-head {
		display: flex;
		align-items: center;
		gap: 10px;
		min-width: 0;
	}

	.who {
		display: flex;
		flex-direction: column;
		min-width: 0;
		flex: 1;
	}

	.path {
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.tile input[type='checkbox'] {
		accent-color: var(--agent-colour);
		align-self: flex-start;
	}

	.state {
		display: flex;
		align-items: center;
		gap: 6px;
	}

	.dot.ready {
		background: var(--ok);
	}

	.goal {
		font-size: calc(var(--fs-ui) * 1.15);
		padding: 10px 12px;
	}

	/* The second blurred surface this redesign allows; the farm pill is the other. */
	.bar {
		position: sticky;
		bottom: 4px;
		z-index: 3;
		display: flex;
		align-items: center;
		justify-content: flex-end;
		gap: 12px;
		padding: 12px 16px;
		border-radius: var(--r-ornament);
	}
</style>
