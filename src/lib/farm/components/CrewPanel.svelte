<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import Btn from '$lib/ui/Btn.svelte';
	import Chip from '$lib/ui/Chip.svelte';
	import Icon from '$lib/ui/Icon.svelte';
	import Loader from '$lib/ui/Loader.svelte';
	import AgentBadge from './AgentBadge.svelte';
	import AddAgentSheet from './AddAgentSheet.svelte';
	import { farmStore } from '../store.svelte';
	import * as api from '../api';
	import {
		FARM_COPY as C,
		PROVIDER_LABELS,
		agentColour,
		availabilityLabel,
		elapsed,
		freeLine,
		isQuiet,
		runLine
	} from '../describe';
	import { AGENT_ROLES, lines } from '../options';
	import type { AgentDefinition, AgentRun } from '../types';

	/**
	 * The agents on this machine (FEAT-112).
	 *
	 * One card per agent: what it is doing now, what it is good at, and its
	 * record on this farm. Turning an agent off while it runs lets the run
	 * finish, and the card says so.
	 */
	interface Props {
		now: number;
		busy: boolean;
		act: (message: string, run: () => Promise<unknown>) => Promise<boolean>;
	}

	let { now, busy, act }: Props = $props();

	let adding = $state(false);
	/** The agent whose arguments are open, if any. */
	let editingArgs = $state<string | null>(null);

	/** Capabilities a card shows; every farm agent codes, so that one goes unsaid. */
	const CAPABILITY_WORDS: Record<string, string> = {
		planning: 'planning',
		frontend: 'frontend',
		backend: 'backend',
		review: 'review',
		research: 'research',
		testing: 'testing',
		documentation: 'docs',
		longContext: 'long context',
		vision: 'vision',
		toolUse: 'tool use'
	};

	const found = $derived(farmStore.agents.filter((a) => a.availability.state !== 'missing'));
	/** Known agents that are not on this machine: one quiet line, not a card each. */
	const missing = $derived([
		...farmStore.agents
			.filter((a) => a.availability.state === 'missing')
			.map((a) => a.definition.displayName),
		...farmStore.undetected.map((provider) => PROVIDER_LABELS[provider])
	]);

	function runFor(agent: string): AgentRun | undefined {
		return farmStore.runs.find((r) => r.agent === agent && r.outcome.state === 'running');
	}

	function doing(run: AgentRun): string {
		if (run.phase === 'planning') return 'Planning the goal';
		const task = farmStore.byId.get(run.task);
		const verb = run.phase === 'review' ? 'Reviewing' : 'Working on';
		return task ? `${verb} ${task.id} · ${task.title}` : `${verb} ${run.task}`;
	}

	function traits(d: AgentDefinition): string {
		return [
			d.inputMode === 'stdin' ? 'Prompt on stdin' : 'Prompt as an argument',
			d.traits.streaming && 'streams output',
			d.traits.structuredOutput && 'structured output',
			d.traits.resumableSessions && 'resumable sessions'
		]
			.filter(Boolean)
			.join(' · ');
	}

	function setEnabled(d: AgentDefinition, enabled: boolean) {
		return act('Could not change the crew', () => api.saveAgent({ ...d, enabled }));
	}
</script>

<div class="farm-columns">
	<main class="farm-main crew">
		{#each found as agent (agent.definition.id)}
			{@const d = agent.definition}
			{@const run = runFor(d.id)}
			{@const score = farmStore.scoreboard.find((s) => s.agent === d.id)}
			{@const available = agent.availability.state === 'available'}
			<article class="card agent" style:--agent-colour={agentColour(d)}>
				<header>
					<AgentBadge agent={d} running={!!run} large />
					<div class="who">
						<div class="name">
							<strong>{d.displayName}</strong>
							<Chip>{AGENT_ROLES.find((r) => r.id === d.role)?.label ?? d.role}</Chip>
						</div>
						{#if agent.availability.state !== 'missing'}
							<span class="mono muted">
								{agent.availability.path}{available
									? ` · ${availabilityLabel(agent.availability)}`
									: ''}
							</span>
						{/if}
					</div>
					<label class="switch">
						<span class="muted">{C.onCrew}</span>
						<input
							type="checkbox"
							role="switch"
							checked={d.enabled}
							disabled={busy || !available}
							onchange={(e) => setEnabled(d, e.currentTarget.checked)}
						/>
					</label>
				</header>

				{#if agent.availability.state === 'broken'}
					<p class="danger">{availabilityLabel(agent.availability)}</p>
				{/if}

				<div class="now" class:busy={!!run}>
					{#if run}
						<Loader size="inline" label={doing(run)} />
						<span class="doing">{doing(run)}</span>
						<span class="muted" class:warn={isQuiet(run, now)}>{runLine(run, now)}</span>
					{:else}
						<span class="doing">{C.free}</span>
						<span class="muted">{freeLine(d.id, farmStore.tasks) ?? ''}</span>
					{/if}
				</div>
				{#if !d.enabled && run}
					<p class="muted">{C.busyDisabled}</p>
				{/if}

				<div class="inline">
					{#each d.capabilities.filter((c) => CAPABILITY_WORDS[c]) as capability (capability)}
						<Chip>{CAPABILITY_WORDS[capability]}</Chip>
					{/each}
				</div>

				<div class="record">
					<div><strong>{score?.completed ?? 0}</strong><small>{C.landedStat}</small></div>
					<div><strong>{score?.changesRequested ?? 0}</strong><small>{C.sentBack}</small></div>
					<div>
						<strong class:danger={!!score?.failed}>{score?.failed ?? 0}</strong>
						<small>{C.failedChecks}</small>
					</div>
					<div>
						<strong>{score?.averageMs ? elapsed(score.averageMs) : '—'}</strong>
						<small>{C.typical}</small>
					</div>
				</div>

				<footer class="spread">
					<span class="muted">{traits(d)}</span>
					<Btn onclick={() => (editingArgs = editingArgs === d.id ? null : d.id)}>
						{C.arguments}
					</Btn>
				</footer>
				{#if editingArgs === d.id}
					<label>
						One argument per line
						<textarea
							class="mono"
							value={d.extraArgs.join('\n')}
							disabled={busy}
							onblur={(e) =>
								act('Could not save the arguments', () =>
									api.saveAgent({ ...d, extraArgs: lines(e.currentTarget.value) })
								)}
						></textarea>
					</label>
				{/if}
			</article>
		{/each}
		{#if !found.length}
			<p class="empty">No command-line agent was found on PATH.</p>
		{/if}
		{#if missing.length}
			<p class="muted not-found">Not found: {missing.join(', ')}.</p>
		{/if}
	</main>

	<aside class="card farm-aside how">
		<section>
			<Btn disabled={busy} onclick={() => act('Could not look for agents', api.detectAgents)}>
				<Icon name="refresh" />
				{C.lookAgain}
			</Btn>
			<Btn onclick={() => (adding = true)}>
				<Icon name="plus" />
				{C.addCli}
			</Btn>
		</section>
	</aside>
</div>

{#if adding}
	<AddAgentSheet {busy} {act} onclose={() => (adding = false)} />
{/if}

<style>
	.crew {
		display: grid;
		grid-template-columns: repeat(2, minmax(0, 1fr));
		align-content: start;
		gap: 14px;
	}

	.agent {
		display: flex;
		flex-direction: column;
		gap: 14px;
		padding: 18px;
	}

	.agent header {
		display: flex;
		align-items: flex-start;
		gap: 12px;
	}

	.who {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
		gap: 2px;
	}

	.name {
		display: flex;
		align-items: center;
		gap: 8px;
	}

	.name strong {
		font-size: calc(var(--fs-ui) * 1.2);
	}

	/* A switch in the agent's own colour. */
	.switch {
		flex-direction: row;
		align-items: center;
		gap: 10px;
		cursor: pointer;
	}

	.switch input {
		appearance: none;
		width: 34px;
		height: 20px;
		margin: 0;
		border-radius: var(--r-pill);
		background: var(--soft);
		position: relative;
		cursor: pointer;
		transition: background var(--t-fast) var(--ease);
	}

	.switch input::after {
		content: '';
		position: absolute;
		top: 3px;
		left: 3px;
		width: 14px;
		height: 14px;
		border-radius: 50%;
		background: var(--ink);
		transition: transform var(--t-fast) var(--ease);
	}

	.switch input:checked {
		background: var(--agent-colour);
	}

	.switch input:checked::after {
		transform: translateX(14px);
		background: var(--bg);
	}

	.switch input:disabled {
		opacity: 0.5;
		cursor: default;
	}

	.now {
		display: flex;
		align-items: center;
		gap: 10px;
		padding: 10px 14px;
		border-radius: var(--r-button);
		border: 1px solid var(--soft);
		background: var(--surface-veil);
	}

	.now.busy {
		background: color-mix(in srgb, var(--agent-colour) 12%, transparent);
		border-color: color-mix(in srgb, var(--agent-colour) 35%, transparent);
	}

	.doing {
		flex: 1;
		min-width: 0;
	}

	.record {
		display: grid;
		grid-template-columns: repeat(4, 1fr);
		gap: 8px;
		padding-top: 14px;
		border-top: 1px solid var(--soft);
	}

	.record strong {
		display: block;
		font-size: var(--fs-title);
		font-weight: 500;
	}

	.record small {
		color: var(--muted);
		font-size: var(--fs-secondary);
	}

	.how {
		display: flex;
		flex-direction: column;
		gap: 4px;
	}

	.how section {
		display: flex;
		flex-direction: column;
		align-items: flex-start;
		gap: 8px;
	}

	@media (max-width: 1100px) {
		.crew {
			grid-template-columns: 1fr;
		}
	}
</style>
