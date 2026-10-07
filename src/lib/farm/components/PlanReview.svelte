<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import Btn from '$lib/ui/Btn.svelte';
	import Chip from '$lib/ui/Chip.svelte';
	import AgentBadge from './AgentBadge.svelte';
	import {
		AUTONOMY_LEVELS,
		FARM_COPY as C,
		TASK_KIND_LABELS,
		agentColour,
		agentName,
		listIds,
		waves,
		whereYouComeIn,
		worthALook
	} from '../describe';
	import { farmStore } from '../store.svelte';
	import * as api from '../api';
	import type { Task } from '../types';

	/**
	 * The plan, before anything runs (FEAT-111).
	 *
	 * The drafts in dependency waves: wave one needs nothing, wave n waits on
	 * wave n − 1, and a cycle or a missing task lands in a last column that
	 * says it cannot be ordered. Unticking a task leaves it out; a kept task
	 * that needs a left-out one is marked, and building cannot start until
	 * that is settled.
	 *
	 * `Start building` is three calls in order — accept the kept, discard the
	 * rest, start — and stops at the first that fails.
	 */
	interface Props {
		busy: boolean;
		act: (message: string, run: () => Promise<unknown>) => Promise<boolean>;
		onrules: () => void;
		onedit: (task: Task) => void;
	}

	let { busy, act, onrules, onedit }: Props = $props();

	/** Tasks left out. Everything starts kept; a re-plan starts over. */
	let leftOut = $state<string[]>([]);
	let planned = '';

	const drafts = $derived(farmStore.drafts);
	const layers = $derived(waves(drafts));
	const kept = $derived(drafts.filter((t) => !leftOut.includes(t.id)));
	const planner = $derived(farmStore.runs.findLast((r) => r.phase === 'planning'));
	const farm = $derived(farmStore.farm);

	$effect(() => {
		const ids = drafts.map((t) => t.id).join(',');
		if (ids !== planned) {
			planned = ids;
			leftOut = [];
		}
	});

	/** What a kept task needs that is neither kept nor landed. */
	function missingFor(task: Task): string[] {
		return task.dependsOn.filter(
			(id) => !kept.some((t) => t.id === id) && farmStore.byId.get(id)?.status !== 'done'
		);
	}

	const broken = $derived(kept.some((t) => missingFor(t).length > 0));
	const unordered = $derived(layers.some((l) => l.unordered));
	const notes = $derived(worthALook(kept));

	function toggle(id: string) {
		leftOut = leftOut.includes(id) ? leftOut.filter((x) => x !== id) : [...leftOut, id];
	}

	function caption(index: number, unordered: boolean): string {
		if (unordered) return C.missingDependency;
		if (index === 0) return 'Starts at once';
		if (index === layers.length - 1 && index > 1) return 'Last; each waits for its own tasks';
		return `When what it needs from wave ${index} lands`;
	}

	async function start() {
		const keep = kept.map((t) => t.id);
		const discard = [...leftOut];
		await act('Could not start building', async () => {
			await api.readyTasks(keep);
			if (discard.length) await api.discardTasks(discard);
			await api.start();
		});
	}

	async function planAgain() {
		await act('Could not plan again', async () => {
			await api.discardTasks(drafts.map((t) => t.id));
			await api.plan(planner?.agent ?? null);
		});
	}
</script>

<div class="farm-columns">
	<main class="farm-main">
		<p class="intro">
			{agentName(planner?.agent, farmStore.agents)} proposes
			<strong>
				{drafts.length}
				{drafts.length === 1 ? 'task' : 'tasks'} in {layers.length}
				{layers.length === 1 ? 'wave' : 'waves'}</strong
			>. A wave starts when what it needs has landed; inside a wave, tasks run side by side. Untick
			anything you don't want; every brief stays editable until its task starts.
		</p>

		<div class="waves">
			{#each layers as layer, i (i)}
				<section>
					<h3>
						{layer.unordered ? C.cycle : `Wave ${i + 1}`}
						<span class="muted">{layer.tasks.length}</span>
					</h3>
					<p class="muted caption" class:warn={layer.unordered}>{caption(i, layer.unordered)}</p>
					<div class="stack">
						{#each layer.tasks as task (task.id)}
							{@const keep = !leftOut.includes(task.id)}
							{@const missing = keep ? missingFor(task) : []}
							{@const agent = farmStore.agents.find((a) => a.definition.id === task.assignedAgent)}
							<article class="card draft" class:omitted={!keep} class:missing={missing.length > 0}>
								<div class="spread">
									<label class="inline">
										<input
											type="checkbox"
											aria-label="Keep {task.id}"
											checked={keep}
											onchange={() => toggle(task.id)}
										/>
										<span class="mono muted">{task.id}</span>
									</label>
									<Chip>{TASK_KIND_LABELS[task.kind]}</Chip>
								</div>
								<button class="title" title="Edit the brief" onclick={() => onedit(task)}>
									{task.title}
								</button>
								{#if task.allowedPaths.length}
									<p class="mono muted paths">{task.allowedPaths.join(', ')}</p>
								{/if}
								<div class="spread who">
									<span class="inline">
										<AgentBadge agent={agent?.definition} />
										<span class="muted">{agentName(task.assignedAgent, farmStore.agents)}</span>
									</span>
									{#if task.dependsOn.length}
										<span class="muted" class:warn={missing.length > 0}>
											needs {listIds(task.dependsOn)}{missing.length ? ', left out' : ''}
										</span>
									{/if}
								</div>
							</article>
						{/each}
					</div>
				</section>
			{/each}
		</div>

		<footer class="farm-pill ornament">
			<span class="muted">
				{leftOut.length
					? `${kept.length} kept · ${leftOut.length} left out`
					: `All ${kept.length} kept`}
			</span>
			<Btn
				disabled={busy}
				onclick={() =>
					act('Could not discard the plan', () => api.discardTasks(drafts.map((t) => t.id)))}
			>
				{C.discard}
			</Btn>
			<Btn disabled={busy} onclick={planAgain}>{C.planAgain}</Btn>
			<Btn primary disabled={busy || !kept.length || broken || unordered} onclick={start}>
				{C.startBuilding} · {kept.length}
				{kept.length === 1 ? 'task' : 'tasks'}
			</Btn>
		</footer>
	</main>

	<aside class="card farm-aside notes">
		<section>
			<h3 class="muted">{C.whoDoesWhat}</h3>
			<div class="assignments">
				{#each farmStore.usable as agent (agent.definition.id)}
					{@const count = kept.filter((t) => t.assignedAgent === agent.definition.id).length}
					<div class="assignment">
						<AgentBadge agent={agent.definition} />
						<span class="name">{agent.definition.displayName}</span>
						<span class="track">
							<span
								style:width="{(count / Math.max(1, kept.length)) * 100}%"
								style:background={agentColour(agent.definition)}
							></span>
						</span>
						<span class="muted">{count}</span>
					</div>
				{/each}
			</div>
			<p class="muted">
				Suggestions. A task goes to the first free agent that suits it if its own is busy.
			</p>
		</section>

		<section class="rule">
			<h3 class="muted">{C.whenStarts}</h3>
			<ul class="when">
				{#if farm?.verification.length}
					<li><span class="dot ok-dot"></span>Every task must pass {listIds(farm.verification)}</li>
				{:else}
					<li><span class="dot warn-dot"></span>No checks: nothing proves a task works</li>
				{/if}
				<li>
					<span class="dot ok-dot"></span>Up to {farm?.maxParallel} agents at once, each in its own worktree
				</li>
				<li>
					<span class="dot"></span>{AUTONOMY_LEVELS.find((a) => a.id === farm?.autonomy)?.label}:
					you come in {whereYouComeIn(farm?.autonomy ?? 'semiAuto').replace(/\.$/, '')}
				</li>
			</ul>
			<button class="link" onclick={onrules}>{C.changeRules}</button>
		</section>

		{#if notes.length || broken}
			<section class="rule">
				<h3 class="muted">{C.worthLook}</h3>
				{#if broken}<p class="warn">A kept task needs a task that is left out.</p>{/if}
				{#each notes as note, i (i)}<p>{note}</p>{/each}
			</section>
		{/if}
	</aside>
</div>

<style>
	.intro {
		font-size: calc(var(--fs-ui) * 1.1);
		line-height: 1.6;
		max-width: 900px;
		margin: 0 0 18px;
	}

	.waves {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(230px, 1fr));
		gap: 16px;
	}

	.waves h3 {
		margin: 0;
	}

	.caption {
		margin: 2px 0 12px;
	}

	.draft {
		display: flex;
		flex-direction: column;
		gap: 8px;
	}

	.title {
		display: block;
		width: 100%;
		padding: 0;
		border: 0;
		background: none;
		color: var(--ink);
		font: inherit;
		font-weight: 550;
		text-align: left;
		cursor: pointer;
	}

	.title:hover {
		color: var(--accent);
	}

	.paths {
		margin: 0;
	}

	.notes {
		display: flex;
		flex-direction: column;
		gap: 4px;
	}

	.assignments {
		display: flex;
		flex-direction: column;
		gap: 10px;
		margin-bottom: 10px;
	}

	.assignment {
		display: grid;
		grid-template-columns: auto 1fr 1fr auto;
		align-items: center;
		gap: 10px;
	}

	.assignment .track {
		height: 5px;
	}

	.assignment .track span {
		display: block;
		height: 100%;
		border-radius: var(--r-pill);
	}

	.when {
		list-style: none;
		margin: 0 0 10px;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: 8px;
	}

	.when li {
		display: flex;
		align-items: baseline;
		gap: 8px;
	}

	.dot.ok-dot {
		background: var(--ok);
	}

	.dot.warn-dot {
		background: var(--warn);
	}

	.link {
		border: none;
		background: none;
		padding: 0;
		color: var(--accent);
		font: inherit;
		cursor: pointer;
	}
</style>
