<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { untrack } from 'svelte';
	import Btn from '$lib/ui/Btn.svelte';
	import Chip from '$lib/ui/Chip.svelte';
	import Icon from '$lib/ui/Icon.svelte';
	import { agents } from './store.svelte';
	import { LEVELS, above, capped, leaves, sentenceFor } from './levels';
	import type { Assigned, Job, Level } from './types';

	/**
	 * Assign an agent (2.0): which agent, how far it goes alone, an optional
	 * note, and two plain lines — where it runs and, for a remote agent, what
	 * leaves the machine. Shared by Review and Merger.
	 *
	 * Refusals are said up front: an agent that wrote commits here is listed
	 * and disabled with the reason, and a level above the repository's highest
	 * is not offered.
	 */
	interface Props {
		job: Job;
		/** Agents that may not do this job here, by id, with why. */
		refused?: Record<string, string>;
		busy?: boolean;
		onassign: (assigned: Assigned) => void;
		oncancel: () => void;
	}

	let { job, refused = {}, busy = false, onassign, oncancel }: Props = $props();

	const usable = $derived(agents.usable(job));
	const highest = $derived<Level>(agents.rules?.highest ?? 'signOff');
	const defaults = untrack(() => agents.snapshot?.defaults);

	let chosen = $state<string | null>(untrack(() => agents.defaultFor(job)));
	let level = $state<Level>(
		untrack(() =>
			capped(
				(job === 'review' ? defaults?.reviewLevel : defaults?.mergeLevel) ?? 'stepByStep',
				agents.rules?.highest ?? 'signOff'
			)
		)
	);
	let note = $state('');
	let lands = $state(false);

	/** The chosen agent, unless it is refused here. */
	const agent = $derived(usable.find((u) => u.id === chosen && !refused[u.id]) ?? null);
	/** Landing means something only where the agent may act at the end. */
	const landsMeans = $derived(level === 'signOff' || level === 'unattended');

	const where = $derived(
		job === 'review'
			? 'A scratch copy of the pull request’s head — your branch is not touched.'
			: 'A scratch copy of the merge — your working copy is not touched.'
	);

	function assign() {
		if (!agent) return;
		onassign({ agent: agent.id, level, note: note.trim(), lands: job === 'merge' && lands && landsMeans });
	}
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div
	class="assign floating"
	role="dialog"
	aria-label="Assign an agent"
	tabindex="-1"
	onkeydown={(event) => {
		if (event.key === 'Escape') oncancel();
	}}
>
	<div class="agents" role="radiogroup" aria-label="Agent">
		{#each usable as option (option.id)}
			{@const why = refused[option.id]}
			<button
				class="agent"
				role="radio"
				aria-checked={chosen === option.id && !why}
				aria-disabled={!!why}
				disabled={!!why}
				title={why ?? option.name}
				onclick={() => (chosen = option.id)}
			>
				<Icon name={option.reach === 'remote' ? (option.remote?.local ? 'machine' : 'cloud') : 'terminal'} size="1.05em" />
				<span class="name">{option.name}</span>
				{#if why}
					<span class="note">{why}</span>
				{:else if option.detail}
					<span class="note mono">{option.detail}</span>
				{/if}
				{#if chosen === option.id && !why}<span class="tick"><Icon name="check" size="1em" /></span>{/if}
			</button>
		{/each}
	</div>

	<div class="levels" role="radiogroup" aria-label="Level">
		{#each LEVELS as option (option.id)}
			{@const over = above(option.id, highest)}
			<Chip
				active={level === option.id}
				disabled={over}
				title={over ? `Capped at ${LEVELS.find((l) => l.id === highest)?.label} here` : option.label}
				onclick={() => (level = option.id)}
			>
				{option.short}
			</Chip>
		{/each}
	</div>
	<p class="note">{sentenceFor(level, job)}</p>

	{#if job === 'merge'}
		<div class="levels" role="radiogroup" aria-label="What to hand over">
			<Chip active={!lands || !landsMeans} onclick={() => (lands = false)}>Resolve the conflicts</Chip>
			<Chip
				active={lands && landsMeans}
				disabled={!landsMeans}
				title={landsMeans ? 'The agent resolves, runs the checks and lands' : 'Needs Sign off or Unattended'}
				onclick={() => (lands = true)}
			>
				Resolve, check and land
			</Chip>
		</div>
	{/if}

	<input type="text" class="field" aria-label="Note for the agent" placeholder="Note (optional)" bind:value={note} />

	<p class="note line"><Icon name="branch" size="1em" />{where}</p>
	{#if agent?.remote}
		<p class="note line"><Icon name={agent.remote.local ? 'machine' : 'upload'} size="1em" />{leaves(agent.remote)}</p>
	{/if}

	<div class="actions">
		<Btn onclick={oncancel}>Cancel</Btn>
		<Btn primary quiet disabled={!agent} {busy} onclick={assign}>Assign</Btn>
	</div>
</div>

<style>
	.assign {
		position: absolute;
		bottom: calc(100% + 8px);
		right: 0;
		z-index: 6;
		width: 400px;
		max-width: 90vw;
		padding: 14px;
		border-radius: var(--r-floating);
		display: flex;
		flex-direction: column;
		gap: 10px;
		outline: none;
	}

	.agents {
		display: flex;
		flex-direction: column;
		gap: 2px;
	}

	.agent {
		display: flex;
		align-items: center;
		gap: 10px;
		padding: 8px 10px;
		border-radius: var(--r-row);
		border: 1px solid transparent;
		text-align: left;
	}

	.agent:hover:not(:disabled) {
		background: var(--hover);
	}

	.agent[aria-checked='true'] {
		border-color: var(--agent-edge);
		background: var(--agent-soft);
	}

	.agent:disabled {
		color: var(--muted);
		cursor: default;
	}

	.name {
		flex: 1;
	}

	.tick {
		display: inline-flex;
		color: var(--agent);
	}

	.levels {
		display: flex;
		flex-wrap: wrap;
		gap: 6px;
	}

	p {
		margin: 0;
	}

	.line {
		display: flex;
		align-items: flex-start;
		gap: 8px;
	}

	.field {
		font-size: var(--fs-ui);
	}

	.actions {
		display: flex;
		justify-content: flex-end;
		gap: 8px;
	}
</style>
